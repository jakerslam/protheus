// Layer ownership: core/layer0/ops (authoritative)
// SPDX-License-Identifier: Apache-2.0
use chrono::{DateTime, Utc};
use serde_json::{json, Map, Value};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::time::Instant;
use walkdir::WalkDir;

#[cfg(test)]
const PROVIDER_REGISTRY_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/provider_registry.json";
const AGENT_PROFILES_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/agent_profiles.json";
const AGENT_CONTRACTS_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/agent_contracts.json";
const AGENT_SESSIONS_DIR_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/agent_sessions";
const AGENT_FILES_DIR_REL: &str = "client/runtime/local/state/ui/infring_dashboard/agent_files";
const AGENT_TOOLS_DIR_REL: &str = "client/runtime/local/state/ui/infring_dashboard/agent_tools";
const ACTION_HISTORY_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/actions/history.jsonl";
const APP_PLANE_STATE_ENV: &str = "APP_PLANE_STATE_ROOT";
const APP_PLANE_SCOPE: &str = "app_plane";
const AGENT_RUNTIME_SYSTEM_PROMPT: &str = "You are an Infring runtime operations agent with host-integrated access to runtime telemetry, queue metrics, cockpit blocks, conduit signals, memory contexts, and approved protheus/infring command surfaces. Never claim you lack runtime access. If data is stale, request a runtime sync and continue with the best available numbers.";

#[path = "dashboard_compat_api_channels.rs"]
mod dashboard_compat_api_channels;
#[path = "dashboard_compat_api_comms.rs"]
mod dashboard_compat_api_comms;
#[path = "dashboard_compat_api_hands.rs"]
mod dashboard_compat_api_hands;
#[path = "dashboard_compat_api_settings_ops.rs"]
mod dashboard_compat_api_settings_ops;
#[path = "dashboard_compat_api_sidebar_ops.rs"]
mod dashboard_compat_api_sidebar_ops;
#[path = "dashboard_skills_marketplace.rs"]
mod dashboard_skills_marketplace;

#[derive(Debug, Clone)]
pub struct CompatApiResponse {
    pub status: u16,
    pub payload: Value,
}

fn clean_text(raw: &str, max_len: usize) -> String {
    raw.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .chars()
        .take(max_len)
        .collect::<String>()
}

fn read_json(path: &Path) -> Option<Value> {
    fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
}

#[cfg(test)]
fn write_json(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(raw) = serde_json::to_string_pretty(value) {
        let _ = fs::write(path, raw);
    }
}

fn parse_non_negative_i64(value: Option<&Value>, fallback: i64) -> i64 {
    value.and_then(Value::as_i64).unwrap_or(fallback).max(0)
}

fn state_path(root: &Path, rel: &str) -> PathBuf {
    root.join(rel)
}

fn query_value(path: &str, key: &str) -> Option<String> {
    let query = path.split_once('?').map(|(_, q)| q).unwrap_or("");
    for pair in query.split('&') {
        if pair.is_empty() {
            continue;
        }
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        if clean_text(k, 80).eq_ignore_ascii_case(key) {
            let decoded = urlencoding::decode(v)
                .ok()
                .map(|s| s.to_string())
                .unwrap_or_default();
            let value = clean_text(&decoded, 160);
            if !value.is_empty() {
                return Some(value);
            }
        }
    }
    None
}

fn header_value(headers: &[(&str, &str)], key: &str) -> Option<String> {
    for (name, value) in headers {
        if clean_text(name, 120).eq_ignore_ascii_case(key) {
            let cleaned = clean_text(value, 2000);
            if !cleaned.is_empty() {
                return Some(cleaned);
            }
        }
    }
    None
}

fn normalize_lexical(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => out.push(prefix.as_os_str()),
            Component::RootDir => out.push(component.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                let _ = out.pop();
            }
            Component::Normal(part) => out.push(part),
        }
    }
    out
}

fn app_plane_state_root(root: &Path) -> PathBuf {
    crate::v8_kernel::scoped_state_root(root, APP_PLANE_STATE_ENV, APP_PLANE_SCOPE)
}

fn app_plane_settings_path(root: &Path) -> PathBuf {
    app_plane_state_root(root)
        .join("chat_ui")
        .join("settings.json")
}

fn extract_app_settings(root: &Path, snapshot: &Value) -> (String, String) {
    if let Some(settings) = read_json_loose(&app_plane_settings_path(root)) {
        let provider = clean_text(
            settings
                .get("provider")
                .and_then(Value::as_str)
                .unwrap_or("auto"),
            80,
        );
        let model = clean_text(
            settings.get("model").and_then(Value::as_str).unwrap_or(""),
            120,
        );
        if !provider.is_empty() || !model.is_empty() {
            return (
                if provider.is_empty() {
                    "auto".to_string()
                } else {
                    provider
                },
                model,
            );
        }
    }
    let provider = clean_text(
        snapshot
            .pointer("/app/settings/provider")
            .and_then(Value::as_str)
            .unwrap_or("auto"),
        80,
    );
    let model = clean_text(
        snapshot
            .pointer("/app/settings/model")
            .and_then(Value::as_str)
            .unwrap_or(""),
        120,
    );
    (provider, model)
}

fn effective_app_settings(root: &Path, snapshot: &Value) -> (String, String) {
    let (provider, model) = extract_app_settings(root, snapshot);
    let (resolved_provider, resolved_model, _) =
        crate::dashboard_model_catalog::resolve_model_selection(
            root,
            snapshot,
            &provider,
            &model,
            &json!({"task_type":"general","budget_mode":"balanced"}),
        );
    if resolved_provider.is_empty() || resolved_model.is_empty() {
        (provider, model)
    } else {
        (resolved_provider, resolved_model)
    }
}

fn save_app_settings(root: &Path, provider: &str, model: &str) -> Value {
    let path = app_plane_settings_path(root);
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let payload = json!({
        "provider": clean_text(provider, 80),
        "model": clean_text(model, 120),
        "updated_at": crate::now_iso()
    });
    write_json_pretty(&path, &payload);
    payload
}

fn runtime_sync_summary(snapshot: &Value) -> Value {
    if let Some(summary) = snapshot.pointer("/runtime_sync/summary") {
        return summary.clone();
    }
    json!({
        "queue_depth": parse_non_negative_i64(snapshot.pointer("/health/dashboard_metrics/queue_depth/value"), 0),
        "cockpit_blocks": parse_non_negative_i64(snapshot.pointer("/health/dashboard_metrics/hermes_cockpit_stream/value"), 0),
        "cockpit_total_blocks": parse_non_negative_i64(snapshot.pointer("/health/dashboard_metrics/hermes_cockpit_stream/value"), 0),
        "attention_batch_count": 0,
        "conduit_signals": parse_non_negative_i64(snapshot.pointer("/health/dashboard_metrics/collab_team_surface/value"), 0),
        "conduit_channels_observed": parse_non_negative_i64(snapshot.pointer("/health/dashboard_metrics/collab_team_surface/value"), 0),
        "target_conduit_signals": 4,
        "conduit_scale_required": false,
        "sync_mode": "live_sync",
        "backpressure_level": "normal"
    })
}

fn runtime_access_denied_phrase(text: &str) -> bool {
    let lowered = text.to_ascii_lowercase();
    lowered.contains("don't have access")
        || lowered.contains("do not have access")
        || lowered.contains("cannot access")
        || lowered.contains("without system monitoring")
        || lowered.contains("text-based ai assistant")
        || lowered.contains("cannot directly interface")
        || lowered.contains("no access to")
}

fn persistent_memory_denied_phrase(text: &str) -> bool {
    let lowered = text.to_ascii_lowercase();
    let conduit_gated_memory_denial = (lowered.contains("memory conduit")
        || lowered.contains("cockpit block")
        || lowered.contains("active memory context"))
        && (lowered.contains("current message")
            || lowered.contains("do not retain information")
            || lowered.contains("between exchanges")
            || lowered.contains("create a memory context"));
    lowered.contains("don't have persistent memory")
        || lowered.contains("do not have persistent memory")
        || lowered.contains("cannot recall our conversation")
        || lowered.contains("cannot recall the specific content")
        || lowered.contains("cannot recall previous conversation")
        || lowered.contains("cannot recall previous sessions")
        || lowered.contains("do not retain memory")
        || lowered.contains("don't retain memory")
        || lowered.contains("between sessions")
        || lowered.contains("session is stateless")
        || lowered.contains("each session is stateless")
        || lowered.contains("without persistent memory")
        || lowered.contains("within this session")
        || lowered.contains("do not retain information between exchanges")
        || lowered.contains("don't detect an active memory context")
        || lowered.contains("do not detect an active memory context")
        || lowered.contains("within active runtime scope")
        || conduit_gated_memory_denial
}

fn memory_recall_requested(text: &str) -> bool {
    let lowered = text.to_ascii_lowercase();
    lowered.contains("remember")
        || lowered.contains("recall")
        || lowered.contains("last week")
        || lowered.contains("earlier")
        || lowered.contains("previous session")
        || lowered.contains("what did i ask")
}

fn runtime_probe_requested(text: &str) -> bool {
    let lowered = text.to_ascii_lowercase();
    (lowered.contains("queue depth")
        || lowered.contains("cockpit blocks")
        || lowered.contains("conduit signals")
        || lowered.contains("memory context")
        || lowered.contains("runtime sync")
        || lowered.contains("what changed")
        || lowered.contains("attention queue"))
        && (lowered.contains("runtime")
            || lowered.contains("status")
            || lowered.contains("sync")
            || lowered.contains("report")
            || lowered.contains("now"))
}

fn runtime_access_summary_text(runtime_summary: &Value) -> String {
    let queue_depth = parse_non_negative_i64(runtime_summary.get("queue_depth"), 0);
    let cockpit_blocks = parse_non_negative_i64(runtime_summary.get("cockpit_blocks"), 0);
    let cockpit_total_blocks =
        parse_non_negative_i64(runtime_summary.get("cockpit_total_blocks"), 0);
    let conduit_signals = parse_non_negative_i64(runtime_summary.get("conduit_signals"), 0);
    format!(
        "Current queue depth: {queue_depth}, cockpit blocks: {cockpit_blocks} active ({cockpit_total_blocks} total), conduit signals: {conduit_signals}. Attention queue is readable. Runtime memory context and protheus/infring command surfaces are available through this agent lane."
    )
}

fn usage_from_state(root: &Path, snapshot: &Value) -> Value {
    let (provider, model) = extract_app_settings(root, snapshot);
    let roster = build_agent_roster(root, snapshot, false);
    let mut agent_rows = Vec::<Value>::new();
    let mut total_input_tokens = 0_i64;
    let mut total_output_tokens = 0_i64;
    let mut total_tool_calls = 0_i64;
    let mut total_cost_usd = 0.0_f64;
    let mut total_call_count = 0_i64;
    let mut earliest_event_date = String::new();
    let mut by_model = HashMap::<String, (String, String, i64, i64, i64, f64)>::new();
    let mut daily = HashMap::<String, (i64, i64, i64, f64)>::new();

    for row in roster {
        let agent_id = clean_agent_id(row.get("agent_id").and_then(Value::as_str).unwrap_or(""));
        if agent_id.is_empty() {
            continue;
        }
        let state = load_session_state(root, &agent_id);
        let sessions = state
            .get("sessions")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let mut input_tokens = 0_i64;
        let mut output_tokens = 0_i64;
        let mut call_count = 0_i64;
        for session in sessions {
            let messages = session
                .get("messages")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            for message in messages {
                let role = clean_text(
                    message.get("role").and_then(Value::as_str).unwrap_or(""),
                    40,
                )
                .to_ascii_lowercase();
                let text = message_text(&message);
                if text.is_empty() {
                    continue;
                }
                let tokens = estimate_tokens(&text);
                if role == "assistant" || role == "agent" {
                    output_tokens += tokens;
                    call_count += 1;
                } else {
                    input_tokens += tokens;
                }
                let timestamp = message_timestamp_iso(&message);
                let date = timestamp.chars().take(10).collect::<String>();
                if !date.is_empty() {
                    let entry = daily.entry(date.clone()).or_insert((0, 0, 0, 0.0));
                    entry.0 += if role == "assistant" || role == "agent" {
                        1
                    } else {
                        0
                    };
                    entry.1 += if role == "assistant" || role == "agent" {
                        0
                    } else {
                        tokens
                    };
                    entry.2 += if role == "assistant" || role == "agent" {
                        tokens
                    } else {
                        0
                    };
                    if earliest_event_date.is_empty() || date < earliest_event_date {
                        earliest_event_date = date;
                    }
                }
            }
        }
        let tool_calls = 0_i64;
        let cost_usd = 0.0_f64;
        let total_tokens = input_tokens + output_tokens;
        let model_provider = clean_text(
            row.get("model_provider")
                .and_then(Value::as_str)
                .unwrap_or("auto"),
            80,
        );
        let model_name = clean_text(
            row.get("model_name").and_then(Value::as_str).unwrap_or(""),
            120,
        );
        let model_key = if model_name.is_empty() {
            format!("{model_provider}/auto")
        } else {
            format!("{model_provider}/{model_name}")
        };
        let entry = by_model.entry(model_key.clone()).or_insert((
            model_provider.clone(),
            model_name.clone(),
            0,
            0,
            0,
            0.0,
        ));
        entry.2 += input_tokens;
        entry.3 += output_tokens;
        entry.4 += call_count;
        entry.5 += cost_usd;

        total_input_tokens += input_tokens;
        total_output_tokens += output_tokens;
        total_tool_calls += tool_calls;
        total_cost_usd += cost_usd;
        total_call_count += call_count;
        agent_rows.push(json!({
            "agent_id": agent_id,
            "id": row.get("id").cloned().unwrap_or_else(|| json!("")),
            "name": row.get("name").cloned().unwrap_or_else(|| json!("Agent")),
            "role": row.get("role").cloned().unwrap_or_else(|| json!("analyst")),
            "state": row.get("state").cloned().unwrap_or_else(|| json!("Idle")),
            "model_provider": model_provider,
            "model_name": model_name,
            "total_input_tokens": input_tokens,
            "total_output_tokens": output_tokens,
            "total_tokens": total_tokens,
            "tool_calls": tool_calls,
            "cost_usd": cost_usd,
            "daily_cost_usd": 0.0,
            "hourly_limit": 0.0,
            "daily_limit": 0.0,
            "monthly_limit": 0.0,
            "max_llm_tokens_per_hour": 0,
            "call_count": call_count,
            "updated_at": row.get("updated_at").cloned().unwrap_or_else(|| json!(""))
        }));
    }

    agent_rows.sort_by_key(|row| {
        std::cmp::Reverse(clean_text(
            row.get("updated_at").and_then(Value::as_str).unwrap_or(""),
            80,
        ))
    });

    let mut model_rows = by_model
        .into_iter()
        .map(
            |(
                key,
                (provider_id, model_name, input_tokens, output_tokens, call_count, cost_usd),
            )| {
                json!({
                    "provider": provider_id,
                    "model": if model_name.is_empty() { key } else { model_name },
                    "total_input_tokens": input_tokens,
                    "total_output_tokens": output_tokens,
                    "total_cost_usd": cost_usd,
                    "call_count": call_count
                })
            },
        )
        .collect::<Vec<_>>();
    model_rows.sort_by(|a, b| {
        clean_text(b.get("model").and_then(Value::as_str).unwrap_or(""), 160).cmp(&clean_text(
            a.get("model").and_then(Value::as_str).unwrap_or(""),
            160,
        ))
    });

    let today = crate::now_iso().chars().take(10).collect::<String>();
    let mut daily_rows = daily
        .into_iter()
        .map(
            |(date, (requests, input_tokens, output_tokens, cost_usd))| {
                json!({
                    "date": date,
                    "requests": requests,
                    "input_tokens": input_tokens,
                    "output_tokens": output_tokens,
                    "cost_usd": cost_usd
                })
            },
        )
        .collect::<Vec<_>>();
    if daily_rows.is_empty() {
        daily_rows.push(json!({
            "date": today,
            "requests": total_call_count,
            "input_tokens": total_input_tokens,
            "output_tokens": total_output_tokens,
            "cost_usd": total_cost_usd
        }));
        if earliest_event_date.is_empty() && !agent_rows.is_empty() {
            earliest_event_date = today.clone();
        }
    }
    daily_rows.sort_by(|a, b| {
        clean_text(a.get("date").and_then(Value::as_str).unwrap_or(""), 20).cmp(&clean_text(
            b.get("date").and_then(Value::as_str).unwrap_or(""),
            20,
        ))
    });
    let today_cost_usd = daily_rows
        .iter()
        .find(|row| clean_text(row.get("date").and_then(Value::as_str).unwrap_or(""), 20) == today)
        .and_then(|row| row.get("cost_usd").and_then(Value::as_f64))
        .unwrap_or(0.0);

    json!({
        "agents": agent_rows,
        "summary": {
            "requests": total_call_count,
            "call_count": total_call_count,
            "input_tokens": total_input_tokens,
            "output_tokens": total_output_tokens,
            "total_input_tokens": total_input_tokens,
            "total_output_tokens": total_output_tokens,
            "total_cost_usd": total_cost_usd,
            "total_tool_calls": total_tool_calls,
            "active_provider": provider,
            "active_model": model
        },
        "models": model_rows,
        "daily": daily_rows,
        "today_cost_usd": today_cost_usd,
        "first_event_date": earliest_event_date
    })
}

fn providers_payload(root: &Path, snapshot: &Value) -> Value {
    crate::dashboard_provider_runtime::providers_payload(root, snapshot)
}

fn config_payload(root: &Path, snapshot: &Value) -> Value {
    let (provider, model) = effective_app_settings(root, snapshot);
    let llm_ready = crate::dashboard_model_catalog::catalog_payload(root, snapshot)
        .get("models")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .any(|row| row.get("available").and_then(Value::as_bool) == Some(true))
        })
        .unwrap_or(false);
    json!({
        "ok": true,
        "api_key": if llm_ready { "set" } else { "not set" },
        "api_key_set": llm_ready,
        "llm_ready": llm_ready,
        "provider": provider,
        "model": model,
        "cli_mode": "ops",
        "workspace_dir": root.to_string_lossy().to_string(),
        "log_level": clean_text(
            &std::env::var("RUST_LOG")
                .or_else(|_| std::env::var("LOG_LEVEL"))
                .unwrap_or_else(|_| "info".to_string()),
            32,
        )
    })
}

fn config_schema_payload() -> Value {
    json!({
        "ok": true,
        "sections": {
            "runtime": {"root_level": true},
            "llm": {"root_level": false}
        }
    })
}

fn set_config_payload(root: &Path, snapshot: &Value, request: &Value) -> Value {
    let path = clean_text(
        request.get("path").and_then(Value::as_str).unwrap_or(""),
        120,
    )
    .to_ascii_lowercase();
    let string_value = clean_text(
        request
            .get("value")
            .and_then(|value| {
                value.as_str().map(|row| row.to_string()).or_else(|| {
                    if value.is_null() {
                        None
                    } else {
                        Some(value.to_string())
                    }
                })
            })
            .unwrap_or_default()
            .trim_matches('"'),
        4000,
    );
    if path.is_empty() {
        return json!({"ok": false, "error": "config_path_required"});
    }
    let field = path.rsplit('.').next().unwrap_or(path.as_str());
    let (current_provider, current_model) = extract_app_settings(root, snapshot);
    match field {
        "provider" => {
            let provider = if string_value.is_empty() {
                "auto".to_string()
            } else {
                string_value
            };
            let saved = save_app_settings(root, &provider, &current_model);
            json!({"ok": true, "path": path, "value": provider, "settings": saved})
        }
        "model" => {
            let saved = save_app_settings(root, &current_provider, &string_value);
            json!({"ok": true, "path": path, "value": string_value, "settings": saved})
        }
        "api_key" => crate::dashboard_provider_runtime::save_provider_key(
            root,
            &current_provider,
            &string_value,
        ),
        _ => {
            json!({"ok": true, "path": path, "value": request.get("value").cloned().unwrap_or(Value::Null)})
        }
    }
}

fn extract_profiles(root: &Path) -> Vec<Value> {
    let state = read_json(&state_path(root, AGENT_PROFILES_REL)).unwrap_or_else(|| json!({}));
    let mut rows = state
        .get("agents")
        .and_then(Value::as_object)
        .map(|obj| obj.values().map(|v| v.clone()).collect::<Vec<Value>>())
        .unwrap_or_default();
    rows.sort_by(|a, b| {
        clean_text(a.get("agent_id").and_then(Value::as_str).unwrap_or(""), 120).cmp(&clean_text(
            b.get("agent_id").and_then(Value::as_str).unwrap_or(""),
            120,
        ))
    });
    rows
}

fn recent_audit_entries(root: &Path, snapshot: &Value) -> Vec<Value> {
    let from_snapshot = snapshot
        .pointer("/receipts/recent")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if !from_snapshot.is_empty() {
        return from_snapshot;
    }
    let raw = fs::read_to_string(state_path(root, ACTION_HISTORY_REL)).unwrap_or_default();
    raw.lines()
        .rev()
        .take(200)
        .filter_map(|row| serde_json::from_str::<Value>(row).ok())
        .collect::<Vec<_>>()
}

fn clean_agent_id(raw: &str) -> String {
    let mut out = String::new();
    for ch in clean_text(raw, 140).chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
            out.push(ch);
        }
    }
    out
}

fn parse_json_loose(raw: &str) -> Option<Value> {
    if raw.trim().is_empty() {
        return None;
    }
    if let Ok(value) = serde_json::from_str::<Value>(raw) {
        return Some(value);
    }
    for line in raw.lines().rev() {
        let candidate = line.trim();
        if candidate.is_empty() {
            continue;
        }
        if let Ok(value) = serde_json::from_str::<Value>(candidate) {
            return Some(value);
        }
    }
    None
}

fn read_json_loose(path: &Path) -> Option<Value> {
    let raw = fs::read_to_string(path).ok()?;
    parse_json_loose(&raw)
}

fn write_json_pretty(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(raw) = serde_json::to_string_pretty(value) {
        let _ = fs::write(path, format!("{raw}\n"));
    }
}

fn parse_rfc3339_utc(raw: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(raw)
        .ok()
        .map(|value| value.with_timezone(&Utc))
}

fn latest_timestamp(values: &[String]) -> String {
    let mut best = String::new();
    for value in values {
        if value.is_empty() {
            continue;
        }
        if best.is_empty() || value > &best {
            best = value.clone();
        }
    }
    best
}

fn message_text(row: &Value) -> String {
    if let Some(text) = row.get("text").and_then(Value::as_str) {
        return clean_text(text, 4000);
    }
    if let Some(text) = row.get("content").and_then(Value::as_str) {
        return clean_text(text, 4000);
    }
    if let Some(text) = row.as_str() {
        return clean_text(text, 4000);
    }
    String::new()
}

fn message_timestamp_iso(row: &Value) -> String {
    if let Some(ts) = row.get("ts").and_then(Value::as_str) {
        return clean_text(ts, 80);
    }
    if let Some(ts_ms) = row.get("ts").and_then(Value::as_i64) {
        if let Some(parsed) = DateTime::<Utc>::from_timestamp_millis(ts_ms) {
            return parsed.to_rfc3339();
        }
    }
    clean_text(
        row.get("created_at").and_then(Value::as_str).unwrap_or(""),
        80,
    )
}

fn humanize_agent_name(agent_id: &str) -> String {
    let cleaned = clean_agent_id(agent_id).replace('-', " ").replace('_', " ");
    let mut words = Vec::<String>::new();
    for word in cleaned.split_whitespace() {
        let mut chars = word.chars();
        if let Some(first) = chars.next() {
            let mut built = String::new();
            built.push(first.to_ascii_uppercase());
            built.push_str(chars.as_str());
            words.push(built);
        }
    }
    if words.is_empty() {
        "Agent".to_string()
    } else {
        words.join(" ")
    }
}

fn profiles_map(root: &Path) -> Map<String, Value> {
    read_json_loose(&state_path(root, AGENT_PROFILES_REL))
        .and_then(|v| v.get("agents").and_then(Value::as_object).cloned())
        .unwrap_or_default()
}

fn contracts_map(root: &Path) -> Map<String, Value> {
    read_json_loose(&state_path(root, AGENT_CONTRACTS_REL))
        .and_then(|v| v.get("contracts").and_then(Value::as_object).cloned())
        .unwrap_or_default()
}

fn session_dir(root: &Path) -> PathBuf {
    state_path(root, AGENT_SESSIONS_DIR_REL)
}

fn session_path(root: &Path, agent_id: &str) -> PathBuf {
    session_dir(root).join(format!("{}.json", clean_agent_id(agent_id)))
}

fn agent_files_dir(root: &Path, agent_id: &str) -> PathBuf {
    state_path(root, AGENT_FILES_DIR_REL).join(clean_agent_id(agent_id))
}

fn agent_tools_path(root: &Path, agent_id: &str) -> PathBuf {
    state_path(root, AGENT_TOOLS_DIR_REL).join(format!("{}.json", clean_agent_id(agent_id)))
}

fn default_session_state(agent_id: &str) -> Value {
    let now = crate::now_iso();
    json!({
        "type": "infring_dashboard_agent_session",
        "agent_id": clean_agent_id(agent_id),
        "active_session_id": "default",
        "sessions": [
            {
                "session_id": "default",
                "label": "Session",
                "created_at": now,
                "updated_at": now,
                "messages": []
            }
        ],
        "memory_kv": {}
    })
}

fn normalize_session_state(agent_id: &str, mut state: Value) -> Value {
    let id = clean_agent_id(agent_id);
    if !state.is_object() {
        state = default_session_state(&id);
    }
    state["agent_id"] = Value::String(id);
    if !state
        .get("active_session_id")
        .and_then(Value::as_str)
        .map(|v| !v.trim().is_empty())
        .unwrap_or(false)
    {
        state["active_session_id"] = Value::String("default".to_string());
    }
    if !state.get("sessions").map(Value::is_array).unwrap_or(false) {
        state["sessions"] = Value::Array(Vec::new());
    }
    if state
        .get("sessions")
        .and_then(Value::as_array)
        .map(|rows| rows.is_empty())
        .unwrap_or(true)
    {
        state["sessions"] = Value::Array(vec![json!({
            "session_id": "default",
            "label": "Session",
            "created_at": crate::now_iso(),
            "updated_at": crate::now_iso(),
            "messages": []
        })]);
    }
    if !state
        .get("memory_kv")
        .map(Value::is_object)
        .unwrap_or(false)
    {
        state["memory_kv"] = json!({});
    }
    state
}

fn load_session_state(root: &Path, agent_id: &str) -> Value {
    let path = session_path(root, agent_id);
    let state = read_json_loose(&path).unwrap_or_else(|| default_session_state(agent_id));
    normalize_session_state(agent_id, state)
}

fn save_session_state(root: &Path, agent_id: &str, state: &Value) {
    let path = session_path(root, agent_id);
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    write_json_pretty(&path, state);
}

fn estimate_tokens(text: &str) -> i64 {
    ((clean_text(text, 20_000).chars().count() as i64) / 4).max(1)
}

fn active_session_row(state: &Value) -> Value {
    let active_id = clean_text(
        state
            .get("active_session_id")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        120,
    );
    let rows = state
        .get("sessions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if let Some(found) = rows.iter().find(|row| {
        row.get("session_id")
            .and_then(Value::as_str)
            .map(|v| clean_text(v, 120) == active_id)
            .unwrap_or(false)
    }) {
        return found.clone();
    }
    rows.first()
        .cloned()
        .unwrap_or_else(|| json!({"messages": []}))
}

fn session_messages(state: &Value) -> Vec<Value> {
    active_session_row(state)
        .get("messages")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn all_session_messages(state: &Value) -> Vec<Value> {
    let sessions = state
        .get("sessions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut rows = Vec::<Value>::new();
    for session in sessions {
        let messages = session
            .get("messages")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        rows.extend(messages);
    }
    rows.sort_by_key(message_timestamp_iso);
    rows
}

fn memory_kv_pairs_from_state(state: &Value) -> Vec<Value> {
    let mut out = state
        .get("memory_kv")
        .and_then(Value::as_object)
        .map(|rows| {
            rows.iter()
                .map(|(key, value)| {
                    json!({
                        "key": clean_text(key, 200),
                        "value": value
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    out.sort_by_key(|row| clean_text(row.get("key").and_then(Value::as_str).unwrap_or(""), 200));
    out
}

fn memory_kv_prompt_context(state: &Value, max_entries: usize) -> String {
    let mut lines = Vec::<String>::new();
    let kv_pairs = memory_kv_pairs_from_state(state);
    for row in kv_pairs.into_iter().take(max_entries.max(1)) {
        let key = clean_text(row.get("key").and_then(Value::as_str).unwrap_or(""), 120);
        if key.is_empty() {
            continue;
        }
        let value = row.get("value").cloned().unwrap_or(Value::Null);
        let rendered = if value.is_string() {
            clean_text(value.as_str().unwrap_or(""), 280)
        } else {
            clean_text(&value.to_string(), 280)
        };
        if rendered.is_empty() {
            continue;
        }
        lines.push(format!("- {key}: {rendered}"));
    }
    if lines.is_empty() {
        return String::new();
    }
    format!(
        "Persistent memory KV (authoritative):\n{}",
        lines.join("\n")
    )
}

fn session_rows_payload(state: &Value) -> Vec<Value> {
    let active_id = clean_text(
        state
            .get("active_session_id")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        120,
    );
    state
        .get("sessions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|row| {
            let sid = clean_text(
                row.get("session_id").and_then(Value::as_str).unwrap_or(""),
                120,
            );
            let label = clean_text(
                row.get("label")
                    .and_then(Value::as_str)
                    .unwrap_or("Session"),
                80,
            );
            let updated_at = clean_text(
                row.get("updated_at").and_then(Value::as_str).unwrap_or(""),
                80,
            );
            let message_count = row
                .get("messages")
                .and_then(Value::as_array)
                .map(|rows| rows.len() as i64)
                .unwrap_or(0);
            json!({
                "id": sid,
                "session_id": sid,
                "label": if label.is_empty() { "Session" } else { &label },
                "updated_at": updated_at,
                "message_count": message_count,
                "active": sid == active_id
            })
        })
        .collect::<Vec<_>>()
}

fn split_model_ref(
    model_ref: &str,
    fallback_provider: &str,
    fallback_model: &str,
) -> (String, String) {
    let cleaned = clean_text(model_ref, 200);
    if cleaned.contains('/') {
        let mut parts = cleaned.splitn(2, '/');
        let provider = clean_text(parts.next().unwrap_or(""), 80);
        let model = clean_text(parts.next().unwrap_or(""), 120);
        if !provider.is_empty() && !model.is_empty() {
            return (provider, model);
        }
    }
    let provider = if fallback_provider.is_empty() {
        "auto".to_string()
    } else {
        clean_text(fallback_provider, 80)
    };
    let model = if cleaned.is_empty() {
        clean_text(fallback_model, 120)
    } else {
        cleaned
    };
    (provider, model)
}

fn parse_manifest_fields(manifest_toml: &str) -> HashMap<String, String> {
    let mut out = HashMap::<String, String>::new();
    let mut in_model = false;
    for line in manifest_toml.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            let section = trimmed.trim_matches(|ch| ch == '[' || ch == ']');
            in_model = section.eq_ignore_ascii_case("model");
            continue;
        }
        if let Some((k, v)) = trimmed.split_once('=') {
            let key = clean_text(k, 80).to_ascii_lowercase();
            let mut value = v.trim().trim_matches('"').to_string();
            value = clean_text(&value, 400);
            if value.is_empty() {
                continue;
            }
            if key == "name" {
                out.insert("name".to_string(), value.clone());
            } else if key == "role" {
                out.insert("role".to_string(), value.clone());
            } else if in_model && key == "provider" {
                out.insert("provider".to_string(), value.clone());
            } else if in_model && key == "model" {
                out.insert("model".to_string(), value.clone());
            }
        }
    }
    out
}

fn make_agent_id(root: &Path, suggested_name: &str) -> String {
    let profiles = profiles_map(root);
    let contracts = contracts_map(root);
    let mut used = HashSet::<String>::new();
    for key in profiles.keys() {
        used.insert(clean_agent_id(key));
    }
    for key in contracts.keys() {
        used.insert(clean_agent_id(key));
    }
    let hint = clean_text(suggested_name, 80)
        .to_ascii_lowercase()
        .replace(' ', "-");
    let direct = clean_agent_id(&hint);
    if !direct.is_empty() && !used.contains(&direct) {
        return direct;
    }
    let hash_seed = json!({"hint": hint, "ts": crate::now_iso(), "nonce": Utc::now().timestamp_nanos_opt().unwrap_or_default()});
    let hash = crate::deterministic_receipt_hash(&hash_seed);
    let mut base = format!("agent-{}", hash.chars().take(12).collect::<String>());
    if !hint.is_empty() && hint.len() <= 18 {
        base = format!(
            "agent-{}-{}",
            hint,
            hash.chars().take(5).collect::<String>()
        );
    }
    let mut candidate = clean_agent_id(&base);
    if candidate.is_empty() {
        candidate = format!("agent-{}", hash.chars().take(12).collect::<String>());
    }
    if !used.contains(&candidate) {
        return candidate;
    }
    for idx in 2..5000 {
        let next = format!("{candidate}-{idx}");
        if !used.contains(&next) {
            return next;
        }
    }
    format!(
        "agent-{}",
        crate::deterministic_receipt_hash(&json!({"fallback": crate::now_iso()}))
            .chars()
            .take(14)
            .collect::<String>()
    )
}

fn contract_with_runtime_fields(contract: &Value) -> Value {
    let mut out = if contract.is_object() {
        contract.clone()
    } else {
        json!({})
    };
    let status = clean_text(
        out.get("status")
            .and_then(Value::as_str)
            .unwrap_or("active"),
        40,
    );
    let now = Utc::now();
    let created = out
        .get("created_at")
        .and_then(Value::as_str)
        .and_then(parse_rfc3339_utc)
        .unwrap_or(now);
    let expiry_seconds = out
        .get("expiry_seconds")
        .and_then(Value::as_i64)
        .unwrap_or(3600)
        .clamp(1, 31 * 24 * 60 * 60);
    let expires = out
        .get("expires_at")
        .and_then(Value::as_str)
        .and_then(parse_rfc3339_utc)
        .unwrap_or_else(|| created + chrono::Duration::seconds(expiry_seconds));
    if out
        .get("expires_at")
        .and_then(Value::as_str)
        .map(|v| v.trim().is_empty())
        .unwrap_or(true)
    {
        out["expires_at"] = Value::String(expires.to_rfc3339());
    }
    let mut remaining = (expires.timestamp_millis() - now.timestamp_millis()).max(0);
    if status.eq_ignore_ascii_case("terminated") {
        remaining = 0;
    }
    out["remaining_ms"] = Value::from(remaining);
    out
}

fn collab_agents_map(snapshot: &Value) -> HashMap<String, Value> {
    let mut out = HashMap::<String, Value>::new();
    let rows = snapshot
        .pointer("/collab/dashboard/agents")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for row in rows {
        let id = clean_agent_id(row.get("shadow").and_then(Value::as_str).unwrap_or(""));
        if id.is_empty() {
            continue;
        }
        out.insert(id, row);
    }
    out
}

fn collab_runtime_active(row: Option<&Value>) -> bool {
    row.and_then(|value| value.get("status").and_then(Value::as_str))
        .map(|status| {
            status.eq_ignore_ascii_case("active") || status.eq_ignore_ascii_case("running")
        })
        .unwrap_or(false)
}

fn session_summary_map(root: &Path, snapshot: &Value) -> HashMap<String, Value> {
    let mut out = HashMap::<String, Value>::new();
    let snapshot_rows = snapshot
        .pointer("/agents/session_summaries/rows")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for row in snapshot_rows {
        let agent_id = clean_agent_id(row.get("agent_id").and_then(Value::as_str).unwrap_or(""));
        if agent_id.is_empty() {
            continue;
        }
        out.insert(agent_id, row);
    }
    let state_rows = crate::dashboard_agent_state::session_summaries(root, 500)
        .get("rows")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for row in state_rows {
        let agent_id = clean_agent_id(row.get("agent_id").and_then(Value::as_str).unwrap_or(""));
        if agent_id.is_empty() {
            continue;
        }
        out.insert(agent_id, row);
    }
    out
}

fn session_summary_rows(root: &Path, snapshot: &Value) -> Vec<Value> {
    let mut rows = session_summary_map(root, snapshot)
        .into_values()
        .collect::<Vec<_>>();
    rows.sort_by_key(|row| {
        std::cmp::Reverse(clean_text(
            row.get("updated_at").and_then(Value::as_str).unwrap_or(""),
            80,
        ))
    });
    rows
}

fn first_string(value: Option<&Value>, key: &str) -> String {
    clean_text(
        value
            .and_then(|row| row.get(key).and_then(Value::as_str))
            .unwrap_or(""),
        240,
    )
}

fn build_agent_roster(root: &Path, snapshot: &Value, include_terminated: bool) -> Vec<Value> {
    let archived = crate::dashboard_agent_state::archived_agent_ids(root);
    let profiles = profiles_map(root);
    let contracts = contracts_map(root);
    let collab = collab_agents_map(snapshot);
    let session_summaries = session_summary_map(root, snapshot);
    let (default_provider, default_model) = effective_app_settings(root, snapshot);
    let mut all_ids = HashSet::<String>::new();
    for key in profiles.keys() {
        let id = clean_agent_id(key);
        if !id.is_empty() {
            all_ids.insert(id);
        }
    }
    for key in contracts.keys() {
        let id = clean_agent_id(key);
        if !id.is_empty() {
            all_ids.insert(id);
        }
    }
    for key in collab.keys() {
        let id = clean_agent_id(key);
        if !id.is_empty() {
            all_ids.insert(id);
        }
    }
    for key in session_summaries.keys() {
        let id = clean_agent_id(key);
        if !id.is_empty() {
            all_ids.insert(id);
        }
    }
    let mut rows = Vec::<Value>::new();
    for agent_id in all_ids {
        if archived.contains(&agent_id) {
            continue;
        }
        let profile = profiles
            .get(&agent_id)
            .cloned()
            .unwrap_or_else(|| json!({}));
        let contract_raw = contracts
            .get(&agent_id)
            .cloned()
            .unwrap_or_else(|| json!({}));
        let collab_row = collab.get(&agent_id);
        let session_summary = session_summaries.get(&agent_id);
        let runtime_active = collab_runtime_active(collab_row);
        let mut contract = contract_with_runtime_fields(&contract_raw);
        let mut contract_status = clean_text(
            contract
                .get("status")
                .and_then(Value::as_str)
                .unwrap_or("active"),
            40,
        )
        .to_ascii_lowercase();
        let termination_reason = clean_text(
            contract
                .get("termination_reason")
                .and_then(Value::as_str)
                .unwrap_or(""),
            80,
        )
        .to_ascii_lowercase();
        if runtime_active && contract_status == "terminated" && termination_reason.is_empty() {
            contract["status"] = json!("active");
            contract_status = "active".to_string();
        }
        if !include_terminated && contract_status == "terminated" {
            continue;
        }
        let profile_name = clean_text(
            profile.get("name").and_then(Value::as_str).unwrap_or(""),
            120,
        );
        let name = if profile_name.is_empty() {
            humanize_agent_name(&agent_id)
        } else {
            profile_name
        };
        let role = {
            let from_profile = clean_text(
                profile.get("role").and_then(Value::as_str).unwrap_or(""),
                60,
            );
            if !from_profile.is_empty() {
                from_profile
            } else {
                let from_collab = first_string(collab_row, "role");
                if !from_collab.is_empty() {
                    from_collab
                } else {
                    "analyst".to_string()
                }
            }
        };
        let session_updated_at = clean_text(
            session_summary
                .and_then(|row| row.get("updated_at").and_then(Value::as_str))
                .unwrap_or(""),
            80,
        );
        let session_message_count = session_summary
            .and_then(|row| row.get("message_count").and_then(Value::as_i64))
            .unwrap_or(0);
        let state = if contract_status == "terminated" {
            "Terminated".to_string()
        } else if runtime_active {
            "Running".to_string()
        } else {
            let raw = first_string(collab_row, "status");
            if raw.is_empty() {
                if session_message_count > 0 || !session_updated_at.is_empty() {
                    "Idle".to_string()
                } else {
                    "Running".to_string()
                }
            } else if raw.eq_ignore_ascii_case("active") || raw.eq_ignore_ascii_case("running") {
                "Running".to_string()
            } else if raw.eq_ignore_ascii_case("idle") {
                "Idle".to_string()
            } else if raw.eq_ignore_ascii_case("inactive") || raw.eq_ignore_ascii_case("paused") {
                let profile_state = clean_text(
                    profile.get("state").and_then(Value::as_str).unwrap_or(""),
                    40,
                )
                .to_ascii_lowercase();
                if profile_state == "running"
                    || profile_state == "active"
                    || contract_status == "active"
                {
                    "Idle".to_string()
                } else {
                    "Inactive".to_string()
                }
            } else {
                raw
            }
        };

        let identity = if profile
            .get("identity")
            .map(Value::is_object)
            .unwrap_or(false)
        {
            profile
                .get("identity")
                .cloned()
                .unwrap_or_else(|| json!({}))
        } else {
            json!({
                "emoji": profile.get("emoji").cloned().unwrap_or_else(|| json!("🧑‍💻")),
                "color": profile.get("color").cloned().unwrap_or_else(|| json!("#2563EB")),
                "archetype": profile.get("archetype").cloned().unwrap_or_else(|| json!("assistant")),
                "vibe": profile.get("vibe").cloned().unwrap_or_else(|| json!(""))
            })
        };
        let model_override = clean_text(
            profile
                .get("model_override")
                .and_then(Value::as_str)
                .unwrap_or(""),
            160,
        );
        let model_ref =
            if !model_override.is_empty() && !model_override.eq_ignore_ascii_case("auto") {
                model_override
            } else {
                default_model.clone()
            };
        let (model_provider, model_name) =
            split_model_ref(&model_ref, &default_provider, &default_model);
        let runtime_model = clean_text(
            profile
                .get("runtime_model")
                .and_then(Value::as_str)
                .unwrap_or(""),
            120,
        );
        let model_runtime = if runtime_model.is_empty() {
            model_name.clone()
        } else {
            runtime_model
        };
        let git_branch = clean_text(
            profile
                .get("git_branch")
                .and_then(Value::as_str)
                .unwrap_or("main"),
            180,
        );
        let git_tree_kind = clean_text(
            profile
                .get("git_tree_kind")
                .and_then(Value::as_str)
                .unwrap_or("master"),
            60,
        );
        let is_master = profile
            .get("is_master_agent")
            .and_then(Value::as_bool)
            .unwrap_or_else(|| {
                let branch = git_branch.to_ascii_lowercase();
                let kind = git_tree_kind.to_ascii_lowercase();
                branch == "main" || branch == "master" || kind == "master" || kind == "main"
            });
        let auto_terminate_allowed = contract
            .get("auto_terminate_allowed")
            .and_then(Value::as_bool)
            .unwrap_or(!is_master);
        let contract_remaining_ms = if auto_terminate_allowed {
            contract.get("remaining_ms").and_then(Value::as_i64)
        } else {
            None
        };
        let created_at = clean_text(
            profile
                .get("created_at")
                .and_then(Value::as_str)
                .or_else(|| contract.get("created_at").and_then(Value::as_str))
                .or_else(|| {
                    session_summary.and_then(|row| row.get("updated_at").and_then(Value::as_str))
                })
                .unwrap_or(""),
            80,
        );
        let updated_at = latest_timestamp(&[
            clean_text(
                profile
                    .get("updated_at")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                80,
            ),
            clean_text(
                contract
                    .get("updated_at")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                80,
            ),
            clean_text(
                collab_row
                    .and_then(|v| v.get("activated_at").and_then(Value::as_str))
                    .unwrap_or(""),
                80,
            ),
            session_updated_at.clone(),
        ]);
        rows.push(json!({
            "id": agent_id,
            "agent_id": agent_id,
            "name": name,
            "role": role,
            "state": state,
            "model_provider": model_provider,
            "model_name": model_name,
            "runtime_model": model_runtime,
            "context_window": profile.get("context_window").cloned().unwrap_or(Value::Null),
            "context_window_tokens": profile.get("context_window_tokens").cloned().unwrap_or(Value::Null),
            "identity": identity,
            "avatar_url": profile.get("avatar_url").cloned().unwrap_or_else(|| json!("")),
            "system_prompt": profile.get("system_prompt").cloned().unwrap_or_else(|| json!("")),
            "fallback_models": profile.get("fallback_models").cloned().unwrap_or_else(|| json!([])),
            "git_branch": git_branch,
            "branch": git_branch,
            "git_tree_kind": git_tree_kind,
            "workspace_dir": profile
                .get("workspace_dir")
                .cloned()
                .unwrap_or_else(|| json!(root.to_string_lossy().to_string())),
            "workspace_rel": profile.get("workspace_rel").cloned().unwrap_or(Value::Null),
            "git_tree_ready": profile.get("git_tree_ready").cloned().unwrap_or_else(|| json!(true)),
            "git_tree_error": profile.get("git_tree_error").cloned().unwrap_or_else(|| json!("")),
            "is_master_agent": is_master,
            "created_at": created_at,
            "updated_at": updated_at,
            "message_count": session_message_count,
            "contract": contract.clone(),
            "contract_expires_at": contract.get("expires_at").cloned().unwrap_or(Value::Null),
            "contract_remaining_ms": contract_remaining_ms.map(Value::from).unwrap_or(Value::Null),
            "auto_terminate_allowed": auto_terminate_allowed
        }));
    }
    rows.sort_by_key(|row| {
        std::cmp::Reverse(clean_text(
            row.get("updated_at").and_then(Value::as_str).unwrap_or(""),
            80,
        ))
    });
    rows
}

fn agent_row_by_id(root: &Path, snapshot: &Value, agent_id: &str) -> Option<Value> {
    let id = clean_agent_id(agent_id);
    if id.is_empty() {
        return None;
    }
    build_agent_roster(root, snapshot, true)
        .into_iter()
        .find(|row| clean_agent_id(row.get("id").and_then(Value::as_str).unwrap_or("")) == id)
}

fn archived_agent_stub(root: &Path, agent_id: &str) -> Value {
    let id = clean_agent_id(agent_id);
    let profile = profiles_map(root)
        .get(&id)
        .cloned()
        .unwrap_or_else(|| json!({}));
    let name = clean_text(
        profile.get("name").and_then(Value::as_str).unwrap_or(""),
        120,
    );
    let role = clean_text(
        profile
            .get("role")
            .and_then(Value::as_str)
            .unwrap_or("analyst"),
        60,
    );
    let role_value = if role.is_empty() {
        "analyst".to_string()
    } else {
        role
    };
    json!({
        "ok": true,
        "id": id,
        "agent_id": id,
        "name": if name.is_empty() { humanize_agent_name(agent_id) } else { name },
        "role": role_value,
        "state": "inactive",
        "archived": true
    })
}

fn update_profile_patch(root: &Path, agent_id: &str, patch: &Value) -> Value {
    let id = clean_agent_id(agent_id);
    if id.is_empty() {
        return json!({"ok": false, "error": "agent_id_required"});
    }
    crate::dashboard_agent_state::upsert_profile(root, &id, patch)
}

fn upsert_contract_patch(root: &Path, agent_id: &str, patch: &Value) -> Value {
    let id = clean_agent_id(agent_id);
    if id.is_empty() {
        return json!({"ok": false, "error": "agent_id_required"});
    }
    crate::dashboard_agent_state::upsert_contract(root, &id, patch)
}

fn session_payload(root: &Path, agent_id: &str) -> Value {
    let id = clean_agent_id(agent_id);
    if id.is_empty() {
        return json!({"ok": false, "error": "agent_id_required"});
    }
    let state = load_session_state(root, &id);
    let messages = session_messages(&state);
    let sessions = session_rows_payload(&state);
    json!({
        "ok": true,
        "agent_id": id,
        "active_session_id": state.get("active_session_id").cloned().unwrap_or_else(|| json!("default")),
        "messages": messages,
        "sessions": sessions,
        "session": state
    })
}

fn append_turn_message(root: &Path, agent_id: &str, user_text: &str, assistant_text: &str) {
    let _ = crate::dashboard_agent_state::append_turn(root, agent_id, user_text, assistant_text);
}

fn reset_active_session(root: &Path, agent_id: &str) -> Value {
    let id = clean_agent_id(agent_id);
    if id.is_empty() {
        return json!({"ok": false, "error": "agent_id_required"});
    }
    let mut state = load_session_state(root, &id);
    let active_id = clean_text(
        state
            .get("active_session_id")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        120,
    );
    if let Some(rows) = state.get_mut("sessions").and_then(Value::as_array_mut) {
        for row in rows.iter_mut() {
            let sid = clean_text(
                row.get("session_id").and_then(Value::as_str).unwrap_or(""),
                120,
            );
            if sid == active_id {
                row["messages"] = Value::Array(Vec::new());
                row["updated_at"] = Value::String(crate::now_iso());
                break;
            }
        }
    }
    save_session_state(root, &id, &state);
    json!({
        "ok": true,
        "type": "dashboard_agent_session_reset",
        "agent_id": id,
        "active_session_id": active_id
    })
}

fn compact_active_session(root: &Path, agent_id: &str, request: &Value) -> Value {
    let id = clean_agent_id(agent_id);
    if id.is_empty() {
        return json!({"ok": false, "error": "agent_id_required"});
    }
    let mut state = load_session_state(root, &id);
    let target_window = request
        .get("target_context_window")
        .and_then(Value::as_i64)
        .unwrap_or(8192)
        .clamp(512, 2_000_000);
    let target_ratio = request
        .get("target_ratio")
        .and_then(Value::as_f64)
        .unwrap_or(0.8)
        .clamp(0.2, 0.95);
    let min_recent_messages = request
        .get("min_recent_messages")
        .and_then(Value::as_u64)
        .unwrap_or(12)
        .clamp(2, 200) as usize;
    let max_messages = request
        .get("max_messages")
        .and_then(Value::as_u64)
        .unwrap_or(200)
        .clamp(20, 800) as usize;

    let active_id = clean_text(
        state
            .get("active_session_id")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        120,
    );
    let mut before_tokens = 0i64;
    let mut after_tokens = 0i64;
    let mut before_messages = 0usize;
    let mut after_messages = 0usize;
    if let Some(rows) = state.get_mut("sessions").and_then(Value::as_array_mut) {
        for row in rows.iter_mut() {
            let sid = clean_text(
                row.get("session_id").and_then(Value::as_str).unwrap_or(""),
                120,
            );
            if sid != active_id {
                continue;
            }
            if !row.get("messages").map(Value::is_array).unwrap_or(false) {
                row["messages"] = Value::Array(Vec::new());
            }
            let messages = row
                .get_mut("messages")
                .and_then(Value::as_array_mut)
                .expect("messages");
            before_messages = messages.len();
            before_tokens = messages
                .iter()
                .map(|item| {
                    let text = item
                        .get("text")
                        .and_then(Value::as_str)
                        .or_else(|| item.get("content").and_then(Value::as_str))
                        .unwrap_or("");
                    estimate_tokens(text)
                })
                .sum::<i64>();
            let target_tokens = ((target_window as f64) * target_ratio).round() as i64;
            if messages.len() > max_messages {
                let drain = messages.len().saturating_sub(max_messages);
                messages.drain(0..drain);
            }
            while messages.len() > min_recent_messages {
                let current_tokens = messages
                    .iter()
                    .map(|item| {
                        let text = item
                            .get("text")
                            .and_then(Value::as_str)
                            .or_else(|| item.get("content").and_then(Value::as_str))
                            .unwrap_or("");
                        estimate_tokens(text)
                    })
                    .sum::<i64>();
                if current_tokens <= target_tokens {
                    break;
                }
                messages.remove(0);
            }
            after_messages = messages.len();
            after_tokens = messages
                .iter()
                .map(|item| {
                    let text = item
                        .get("text")
                        .and_then(Value::as_str)
                        .or_else(|| item.get("content").and_then(Value::as_str))
                        .unwrap_or("");
                    estimate_tokens(text)
                })
                .sum::<i64>();
            row["updated_at"] = Value::String(crate::now_iso());
            break;
        }
    }
    save_session_state(root, &id, &state);
    json!({
        "ok": true,
        "type": "dashboard_agent_session_compact",
        "agent_id": id,
        "before_tokens": before_tokens,
        "after_tokens": after_tokens,
        "before_messages": before_messages,
        "after_messages": after_messages,
        "message": format!("Compaction complete: {} -> {} tokens", before_tokens, after_tokens)
    })
}

fn parse_agent_route(path_only: &str) -> Option<(String, Vec<String>)> {
    let prefix = "/api/agents/";
    if !path_only.starts_with(prefix) {
        return None;
    }
    let tail = path_only.trim_start_matches(prefix).trim_matches('/');
    if tail.is_empty() {
        return None;
    }
    let mut parts = tail
        .split('/')
        .map(|v| clean_text(v, 180))
        .collect::<Vec<_>>();
    if parts.is_empty() {
        return None;
    }
    let agent_id = clean_agent_id(&parts.remove(0));
    if agent_id.is_empty() {
        return None;
    }
    Some((agent_id, parts))
}

fn resolve_agent_id_alias(root: &Path, requested: &str) -> String {
    let normalized = clean_agent_id(requested);
    if normalized.is_empty() {
        return String::new();
    }
    let profiles = profiles_map(root);
    if profiles.contains_key(&normalized) {
        return normalized;
    }
    let contracts = contracts_map(root);
    if contracts.contains_key(&normalized) {
        return normalized;
    }
    let requested_name = clean_text(requested, 120).to_ascii_lowercase();
    if requested_name.is_empty() {
        return normalized;
    }
    for (id, profile) in &profiles {
        let profile_name = clean_text(
            profile.get("name").and_then(Value::as_str).unwrap_or(""),
            120,
        )
        .to_ascii_lowercase();
        if !profile_name.is_empty() && profile_name == requested_name {
            let resolved = clean_agent_id(id);
            if !resolved.is_empty() {
                return resolved;
            }
        }
    }
    normalized
}

fn parse_provider_route(path_only: &str) -> Option<(String, Vec<String>)> {
    let prefix = "/api/providers/";
    if !path_only.starts_with(prefix) {
        return None;
    }
    let tail = path_only.trim_start_matches(prefix).trim_matches('/');
    if tail.is_empty() {
        return None;
    }
    let mut parts = tail
        .split('/')
        .map(|value| clean_text(value, 180))
        .collect::<Vec<_>>();
    if parts.is_empty() {
        return None;
    }
    let provider_id = decode_path_segment(&parts.remove(0));
    if provider_id.is_empty() {
        return None;
    }
    Some((provider_id, parts))
}

fn parse_memory_route(path_only: &str) -> Option<(String, Vec<String>)> {
    let prefix = "/api/memory/agents/";
    if !path_only.starts_with(prefix) {
        return None;
    }
    let tail = path_only.trim_start_matches(prefix).trim_matches('/');
    if tail.is_empty() {
        return None;
    }
    let mut parts = tail.split('/').map(decode_path_segment).collect::<Vec<_>>();
    if parts.len() < 2 {
        return None;
    }
    let agent_id = clean_agent_id(&parts.remove(0));
    if agent_id.is_empty() {
        return None;
    }
    Some((agent_id, parts))
}

fn decode_path_segment(raw: &str) -> String {
    let decoded = urlencoding::decode(raw)
        .ok()
        .map(|v| v.to_string())
        .unwrap_or_else(|| raw.to_string());
    clean_text(&decoded, 300)
}

fn workspace_base_for_agent(root: &Path, row: Option<&Value>) -> PathBuf {
    let raw = clean_text(
        row.and_then(|v| v.get("workspace_dir").and_then(Value::as_str))
            .unwrap_or(""),
        4000,
    );
    let base = if raw.is_empty() {
        root.to_path_buf()
    } else {
        let as_path = PathBuf::from(raw);
        if as_path.is_absolute() {
            as_path
        } else {
            root.join(as_path)
        }
    };
    normalize_lexical(&base)
}

fn resolve_workspace_path(base: &Path, requested_path: &str) -> Option<PathBuf> {
    let cleaned = requested_path.trim();
    if cleaned.is_empty() {
        return None;
    }
    let requested = PathBuf::from(cleaned);
    let candidate = if requested.is_absolute() {
        requested
    } else {
        base.join(requested)
    };
    let base_norm = normalize_lexical(base);
    let candidate_norm = normalize_lexical(&candidate);
    if !candidate_norm.starts_with(&base_norm) {
        return None;
    }
    Some(candidate_norm)
}

fn truncate_utf8_lossy(bytes: &[u8], max_bytes: usize) -> (String, bool) {
    if bytes.len() <= max_bytes {
        return (String::from_utf8_lossy(bytes).to_string(), false);
    }
    let mut end = max_bytes;
    while end > 0 && !std::str::from_utf8(&bytes[..end]).is_ok() {
        end -= 1;
    }
    let slice = if end == 0 {
        &bytes[..max_bytes]
    } else {
        &bytes[..end]
    };
    (String::from_utf8_lossy(slice).to_string(), true)
}

fn message_token_cost(row: &Value) -> i64 {
    estimate_tokens(&message_text(row))
}

fn total_message_tokens(rows: &[Value]) -> i64 {
    rows.iter().map(message_token_cost).sum::<i64>().max(0)
}

fn trim_context_pool(messages: &[Value], limit_tokens: i64) -> Vec<Value> {
    let cap = limit_tokens.max(2_048);
    let mut out = messages.to_vec();
    let mut total = total_message_tokens(&out);
    while out.len() > 1 && total > cap {
        let removed = message_token_cost(&out[0]);
        out.remove(0);
        total = (total - removed).max(0);
    }
    out
}

fn select_active_context_window(
    messages: &[Value],
    target_tokens: i64,
    min_recent: usize,
) -> Vec<Value> {
    let cap = target_tokens.max(1_024);
    let floor = min_recent.clamp(1, 128);
    let mut out = messages.to_vec();
    let mut total = total_message_tokens(&out);
    while out.len() > floor && total > cap {
        let removed = message_token_cost(&out[0]);
        out.remove(0);
        total = (total - removed).max(0);
    }
    out
}

fn set_active_session_messages(state: &mut Value, messages: &[Value]) {
    let active_id = clean_text(
        state
            .get("active_session_id")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        120,
    );
    if let Some(rows) = state.get_mut("sessions").and_then(Value::as_array_mut) {
        for row in rows.iter_mut() {
            let sid = clean_text(
                row.get("session_id").and_then(Value::as_str).unwrap_or(""),
                120,
            );
            if sid != active_id {
                continue;
            }
            row["messages"] = Value::Array(messages.to_vec());
            row["updated_at"] = Value::String(crate::now_iso());
            break;
        }
    }
}

fn data_url_from_bytes(bytes: &[u8], content_type: &str) -> String {
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;
    format!(
        "data:{};base64,{}",
        clean_text(content_type, 120),
        STANDARD.encode(bytes)
    )
}

fn git_tree_payload_for_agent(root: &Path, snapshot: &Value, agent_id: &str) -> Value {
    let roster = build_agent_roster(root, snapshot, true);
    let mut counts = HashMap::<String, i64>::new();
    let mut current = Value::Null;
    for row in &roster {
        let branch = clean_text(
            row.get("git_branch").and_then(Value::as_str).unwrap_or(""),
            180,
        );
        if branch.is_empty() {
            continue;
        }
        *counts.entry(branch.clone()).or_insert(0) += 1;
        if clean_agent_id(row.get("id").and_then(Value::as_str).unwrap_or(""))
            == clean_agent_id(agent_id)
        {
            current = row.clone();
        }
    }
    let current_branch = clean_text(
        current
            .get("git_branch")
            .and_then(Value::as_str)
            .unwrap_or("main"),
        180,
    );
    let current_workspace = clean_text(
        current
            .get("workspace_dir")
            .and_then(Value::as_str)
            .unwrap_or(""),
        4000,
    );
    let current_workspace_dir = if current_workspace.is_empty() {
        root.to_path_buf()
    } else {
        PathBuf::from(&current_workspace)
    };
    let current_workspace_rel = current.get("workspace_rel").cloned().unwrap_or_else(|| {
        json!(crate::dashboard_git_runtime::workspace_rel(
            root,
            &current_workspace_dir
        ))
    });
    let (main_branch, mut branches) =
        crate::dashboard_git_runtime::list_git_branches(root, 200, &current_branch);
    if branches.is_empty() {
        branches.push(if main_branch.is_empty() {
            "main".to_string()
        } else {
            main_branch.clone()
        });
    }
    for branch in counts.keys() {
        if !branches.iter().any(|row| row == branch) {
            branches.push(branch.clone());
        }
    }
    branches.sort();
    let options = branches
        .iter()
        .map(|branch| {
            let kind = if branch == "main" || branch == "master" {
                "master"
            } else {
                "isolated"
            };
            let workspace = if branch == "main" || branch == "master" {
                root.to_path_buf()
            } else {
                crate::dashboard_git_runtime::workspace_for_agent_branch(root, agent_id, branch)
            };
            let ready = crate::dashboard_git_runtime::git_workspace_ready(root, &workspace);
            json!({
                "branch": branch,
                "current": *branch == current_branch,
                "main": *branch == "main" || *branch == "master",
                "kind": kind,
                "in_use_by_agents": counts.get(branch).copied().unwrap_or(0),
                "workspace_dir": workspace.to_string_lossy().to_string(),
                "workspace_rel": crate::dashboard_git_runtime::workspace_rel(root, &workspace),
                "git_tree_ready": if kind == "master" { true } else { ready },
                "git_tree_error": if kind == "master" || ready { "" } else { "git_worktree_missing" }
            })
        })
        .collect::<Vec<_>>();
    json!({
        "ok": true,
        "current": {
            "git_branch": if current_branch.is_empty() { "main" } else { &current_branch },
            "git_tree_kind": if current_branch == "main" || current_branch == "master" { "master" } else { "isolated" },
            "workspace_dir": if current_workspace.is_empty() { root.to_string_lossy().to_string() } else { current_workspace },
            "workspace_rel": current_workspace_rel,
            "git_tree_ready": current.get("git_tree_ready").cloned().unwrap_or_else(|| json!(true)),
            "git_tree_error": current.get("git_tree_error").cloned().unwrap_or_else(|| json!(""))
        },
        "options": options
    })
}

pub fn handle_with_headers(
    root: &Path,
    method: &str,
    path: &str,
    body: &[u8],
    headers: &[(&str, &str)],
    snapshot: &Value,
) -> Option<CompatApiResponse> {
    let path_only = path.split('?').next().unwrap_or(path);
    if let Some(payload) =
        crate::dashboard_terminal_broker::handle_http(root, method, path_only, body)
    {
        return Some(CompatApiResponse {
            status: 200,
            payload,
        });
    }
    if let Some(response) = dashboard_compat_api_channels::handle(root, method, path_only, body) {
        return Some(response);
    }
    if let Some(response) = dashboard_skills_marketplace::handle(root, method, path, snapshot, body)
    {
        return Some(response);
    }
    if let Some(response) =
        dashboard_compat_api_comms::handle(root, method, path, path_only, body, snapshot)
    {
        return Some(response);
    }
    if let Some(response) =
        dashboard_compat_api_hands::handle(root, method, path_only, body, snapshot)
    {
        return Some(response);
    }
    if let Some(response) =
        dashboard_compat_api_sidebar_ops::handle(root, method, path_only, body, snapshot)
    {
        return Some(response);
    }
    if let Some(response) = dashboard_compat_api_settings_ops::handle(root, method, path_only, body)
    {
        return Some(response);
    }

    if let Some((requested_agent_id, segments)) = parse_memory_route(path_only) {
        let agent_id = resolve_agent_id_alias(root, &requested_agent_id);
        if segments.first().map(|v| v == "kv").unwrap_or(false) {
            if method == "GET" && segments.len() == 1 {
                let state = load_session_state(root, &agent_id);
                return Some(CompatApiResponse {
                    status: 200,
                    payload: json!({
                        "ok": true,
                        "agent_id": agent_id,
                        "kv_pairs": memory_kv_pairs_from_state(&state)
                    }),
                });
            }
            if segments.len() >= 2 {
                let key = decode_path_segment(&segments[1..].join("/"));
                if method == "GET" {
                    return Some(CompatApiResponse {
                        status: 200,
                        payload: crate::dashboard_agent_state::memory_kv_get(root, &agent_id, &key),
                    });
                }
                if method == "PUT" {
                    let request =
                        serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
                    let value = request.get("value").cloned().unwrap_or(Value::Null);
                    let payload =
                        crate::dashboard_agent_state::memory_kv_set(root, &agent_id, &key, &value);
                    return Some(CompatApiResponse {
                        status: if payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                            200
                        } else {
                            400
                        },
                        payload,
                    });
                }
                if method == "DELETE" {
                    return Some(CompatApiResponse {
                        status: 200,
                        payload: crate::dashboard_agent_state::memory_kv_delete(
                            root, &agent_id, &key,
                        ),
                    });
                }
            }
        }
    }

    if let Some((provider_id, segments)) = parse_provider_route(path_only) {
        if method == "POST" && segments.len() == 1 && segments[0] == "key" {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            let key = clean_text(
                request.get("key").and_then(Value::as_str).unwrap_or(""),
                4096,
            );
            let payload =
                crate::dashboard_provider_runtime::save_provider_key(root, &provider_id, &key);
            return Some(CompatApiResponse {
                status: if payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    200
                } else {
                    400
                },
                payload,
            });
        }
        if method == "DELETE" && segments.len() == 1 && segments[0] == "key" {
            return Some(CompatApiResponse {
                status: 200,
                payload: crate::dashboard_provider_runtime::remove_provider_key(root, &provider_id),
            });
        }
        if method == "POST" && segments.len() == 1 && segments[0] == "test" {
            return Some(CompatApiResponse {
                status: 200,
                payload: crate::dashboard_provider_runtime::test_provider(root, &provider_id),
            });
        }
        if method == "PUT" && segments.len() == 1 && segments[0] == "url" {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            let base_url = clean_text(
                request
                    .get("base_url")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                400,
            );
            let payload =
                crate::dashboard_provider_runtime::set_provider_url(root, &provider_id, &base_url);
            return Some(CompatApiResponse {
                status: if payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    200
                } else {
                    400
                },
                payload,
            });
        }
    }

    if method == "POST" && path_only == "/api/models/discover" {
        let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
        let input = clean_text(
            request
                .get("input")
                .and_then(Value::as_str)
                .or_else(|| request.get("api_key").and_then(Value::as_str))
                .unwrap_or(""),
            4096,
        );
        let payload = crate::dashboard_provider_runtime::discover_models(root, &input);
        return Some(CompatApiResponse {
            status: if payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                200
            } else {
                400
            },
            payload,
        });
    }
    if method == "POST" && path_only == "/api/models/download" {
        let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
        let provider = clean_text(
            request
                .get("provider")
                .and_then(Value::as_str)
                .unwrap_or(""),
            80,
        );
        let model = clean_text(
            request.get("model").and_then(Value::as_str).unwrap_or(""),
            240,
        );
        let payload = crate::dashboard_provider_runtime::download_model(root, &provider, &model);
        return Some(CompatApiResponse {
            status: if payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                200
            } else {
                400
            },
            payload,
        });
    }
    if method == "POST" && path_only == "/api/models/custom" {
        let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
        let provider = clean_text(
            request
                .get("provider")
                .and_then(Value::as_str)
                .unwrap_or("openrouter"),
            80,
        );
        let model = clean_text(
            request
                .get("id")
                .or_else(|| request.get("model"))
                .and_then(Value::as_str)
                .unwrap_or(""),
            240,
        );
        let context_window = request
            .get("context_window")
            .and_then(Value::as_i64)
            .unwrap_or(128_000);
        let max_output_tokens = request
            .get("max_output_tokens")
            .and_then(Value::as_i64)
            .unwrap_or(8192);
        let payload = crate::dashboard_provider_runtime::add_custom_model(
            root,
            &provider,
            &model,
            context_window,
            max_output_tokens,
        );
        return Some(CompatApiResponse {
            status: if payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                200
            } else {
                400
            },
            payload,
        });
    }
    if method == "DELETE" && path_only.starts_with("/api/models/custom/") {
        let model_ref = decode_path_segment(path_only.trim_start_matches("/api/models/custom/"));
        return Some(CompatApiResponse {
            status: 200,
            payload: crate::dashboard_provider_runtime::delete_custom_model(root, &model_ref),
        });
    }

    if method == "GET" && path_only == "/api/agents/terminated" {
        return Some(CompatApiResponse {
            status: 200,
            payload: crate::dashboard_agent_state::terminated_entries(root),
        });
    }
    if method == "POST" && path_only.starts_with("/api/agents/") && path_only.ends_with("/revive") {
        let agent_id = path_only
            .trim_start_matches("/api/agents/")
            .trim_end_matches("/revive")
            .trim_matches('/');
        let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
        let role = request
            .get("role")
            .and_then(Value::as_str)
            .unwrap_or("analyst");
        return Some(CompatApiResponse {
            status: 200,
            payload: crate::dashboard_agent_state::revive_agent(root, agent_id, role),
        });
    }
    if method == "DELETE" && path_only == "/api/agents/terminated" {
        if query_value(path, "all")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false)
        {
            return Some(CompatApiResponse {
                status: 200,
                payload: crate::dashboard_agent_state::delete_all_terminated(root),
            });
        }
    }
    if method == "DELETE" && path_only.starts_with("/api/agents/terminated/") {
        let agent_id = path_only
            .trim_start_matches("/api/agents/terminated/")
            .trim();
        return Some(CompatApiResponse {
            status: 200,
            payload: crate::dashboard_agent_state::delete_terminated(
                root,
                agent_id,
                query_value(path, "contract_id").as_deref(),
            ),
        });
    }

    if method == "GET" && path_only == "/api/agents" {
        let _ = crate::dashboard_agent_state::enforce_expired_contracts(root);
        let include_terminated = query_value(path, "include_terminated")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
        return Some(CompatApiResponse {
            status: 200,
            payload: Value::Array(build_agent_roster(root, snapshot, include_terminated)),
        });
    }

    if method == "POST" && path_only == "/api/agents" {
        let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
        let manifest = clean_text(
            request
                .get("manifest_toml")
                .and_then(Value::as_str)
                .unwrap_or(""),
            20_000,
        );
        let manifest_fields = parse_manifest_fields(&manifest);
        let requested_name = clean_text(
            request
                .get("name")
                .and_then(Value::as_str)
                .or_else(|| manifest_fields.get("name").map(|v| v.as_str()))
                .unwrap_or(""),
            120,
        );
        let requested_role = clean_text(
            request
                .get("role")
                .and_then(Value::as_str)
                .or_else(|| manifest_fields.get("role").map(|v| v.as_str()))
                .unwrap_or("analyst"),
            60,
        );
        let role = if requested_role.is_empty() {
            "analyst".to_string()
        } else {
            requested_role
        };
        let name = if requested_name.is_empty() {
            "agent".to_string()
        } else {
            requested_name
        };
        let agent_id = make_agent_id(root, &name);
        let (default_provider, default_model) = effective_app_settings(root, snapshot);
        let model_provider = clean_text(
            request
                .get("provider")
                .and_then(Value::as_str)
                .or_else(|| manifest_fields.get("provider").map(|v| v.as_str()))
                .unwrap_or(&default_provider),
            80,
        );
        let model_name = clean_text(
            request
                .get("model")
                .and_then(Value::as_str)
                .or_else(|| manifest_fields.get("model").map(|v| v.as_str()))
                .unwrap_or(&default_model),
            120,
        );
        let model_override = if model_provider.is_empty() || model_name.is_empty() {
            "auto".to_string()
        } else {
            format!("{model_provider}/{model_name}")
        };
        let identity = if request
            .get("identity")
            .map(Value::is_object)
            .unwrap_or(false)
        {
            request
                .get("identity")
                .cloned()
                .unwrap_or_else(|| json!({}))
        } else {
            json!({
                "emoji": request.get("emoji").cloned().unwrap_or_else(|| json!("🧑‍💻")),
                "color": request.get("color").cloned().unwrap_or_else(|| json!("#2563EB")),
                "archetype": request.get("archetype").cloned().unwrap_or_else(|| json!("assistant")),
                "vibe": request.get("vibe").cloned().unwrap_or_else(|| json!(""))
            })
        };
        let profile_patch = json!({
            "agent_id": agent_id,
            "name": name,
            "role": role,
            "state": "Running",
            "model_override": model_override,
            "model_provider": model_provider,
            "model_name": model_name,
            "runtime_model": model_name,
            "system_prompt": request.get("system_prompt").cloned().unwrap_or_else(|| json!("")),
            "identity": identity,
            "fallback_models": request.get("fallback_models").cloned().unwrap_or_else(|| json!([])),
            "git_tree_kind": "master",
            "git_branch": "main",
            "workspace_dir": root.to_string_lossy().to_string(),
            "workspace_rel": "",
            "git_tree_ready": true,
            "git_tree_error": "",
            "is_master_agent": true
        });
        let _ = update_profile_patch(root, &agent_id, &profile_patch);
        let contract_obj = request
            .get("contract")
            .cloned()
            .unwrap_or_else(|| json!({}));
        let expiry_seconds = contract_obj
            .get("expiry_seconds")
            .and_then(Value::as_i64)
            .unwrap_or(3600)
            .clamp(1, 31 * 24 * 60 * 60);
        let auto_terminate_allowed = contract_obj
            .get("auto_terminate_allowed")
            .and_then(Value::as_bool)
            .unwrap_or(true);
        let contract_patch = json!({
            "agent_id": agent_id,
            "status": "active",
            "created_at": crate::now_iso(),
            "updated_at": crate::now_iso(),
            "owner": clean_text(contract_obj.get("owner").and_then(Value::as_str).unwrap_or("dashboard_auto"), 80),
            "mission": clean_text(contract_obj.get("mission").and_then(Value::as_str).unwrap_or("Assist with assigned mission."), 200),
            "termination_condition": clean_text(contract_obj.get("termination_condition").and_then(Value::as_str).unwrap_or("task_or_timeout"), 80),
            "expiry_seconds": expiry_seconds,
            "auto_terminate_allowed": auto_terminate_allowed,
            "conversation_hold": contract_obj.get("conversation_hold").and_then(Value::as_bool).unwrap_or(false),
            "expires_at": clean_text(contract_obj.get("expires_at").and_then(Value::as_str).unwrap_or(""), 80)
        });
        let _ = upsert_contract_patch(root, &agent_id, &contract_patch);
        append_turn_message(root, &agent_id, "", "");
        let row = agent_row_by_id(root, snapshot, &agent_id).unwrap_or_else(|| {
            json!({
                "id": agent_id,
                "name": name,
                "role": role,
                "state": "Running",
                "model_provider": model_provider,
                "model_name": model_name
            })
        });
        return Some(CompatApiResponse {
            status: 200,
            payload: json!({
                "ok": true,
                "id": row.get("id").cloned().unwrap_or_else(|| json!("")),
                "agent_id": row.get("id").cloned().unwrap_or_else(|| json!("")),
                "name": row.get("name").cloned().unwrap_or_else(|| json!("agent")),
                "state": row.get("state").cloned().unwrap_or_else(|| json!("Running")),
                "model_provider": row.get("model_provider").cloned().unwrap_or_else(|| json!(default_provider)),
                "model_name": row.get("model_name").cloned().unwrap_or_else(|| json!(default_model)),
                "runtime_model": row.get("runtime_model").cloned().unwrap_or_else(|| json!(default_model)),
                "created_at": row.get("created_at").cloned().unwrap_or_else(|| json!(crate::now_iso()))
            }),
        });
    }

    if let Some((requested_agent_id, segments)) = parse_agent_route(path_only) {
        let agent_id = resolve_agent_id_alias(root, &requested_agent_id);
        let existing = agent_row_by_id(root, snapshot, &agent_id);
        let is_archived =
            crate::dashboard_agent_state::archived_agent_ids(root).contains(&agent_id);
        if method == "GET" && segments.is_empty() {
            if let Some(row) = existing {
                return Some(CompatApiResponse {
                    status: 200,
                    payload: row,
                });
            }
            if is_archived {
                return Some(CompatApiResponse {
                    status: 200,
                    payload: archived_agent_stub(root, &agent_id),
                });
            }
            return Some(CompatApiResponse {
                status: 404,
                payload: json!({"ok": false, "error": "agent_not_found", "agent_id": agent_id}),
            });
        }

        if method == "DELETE" && segments.is_empty() {
            if existing.is_none() {
                if is_archived {
                    return Some(CompatApiResponse {
                        status: 200,
                        payload: json!({
                            "ok": true,
                            "type": "dashboard_agent_archive",
                            "id": agent_id,
                            "agent_id": agent_id,
                            "state": "inactive",
                            "archived": true
                        }),
                    });
                }
                return Some(CompatApiResponse {
                    status: 404,
                    payload: json!({"ok": false, "error": "agent_not_found", "agent_id": agent_id}),
                });
            }
            let _ = update_profile_patch(
                root,
                &agent_id,
                &json!({"state": "Archived", "updated_at": crate::now_iso()}),
            );
            let _ = upsert_contract_patch(
                root,
                &agent_id,
                &json!({
                    "status": "terminated",
                    "termination_reason": "user_archived",
                    "terminated_at": crate::now_iso(),
                    "updated_at": crate::now_iso()
                }),
            );
            let _ = crate::dashboard_agent_state::archive_agent(root, &agent_id, "user_archive");
            return Some(CompatApiResponse {
                status: 200,
                payload: json!({
                    "ok": true,
                    "type": "dashboard_agent_archive",
                    "id": agent_id,
                    "agent_id": agent_id,
                    "state": "inactive",
                    "archived": true
                }),
            });
        }

        if method == "POST" && segments.len() == 1 && segments[0] == "stop" {
            if existing.is_none() {
                return Some(CompatApiResponse {
                    status: 404,
                    payload: json!({"ok": false, "error": "agent_not_found", "agent_id": agent_id}),
                });
            }
            let _ = upsert_contract_patch(
                root,
                &agent_id,
                &json!({
                    "status": "terminated",
                    "termination_reason": "stopped",
                    "terminated_at": crate::now_iso(),
                    "updated_at": crate::now_iso()
                }),
            );
            return Some(CompatApiResponse {
                status: 200,
                payload: json!({"ok": true, "type": "dashboard_agent_stop", "agent_id": agent_id}),
            });
        }

        if method == "POST" && segments.len() == 1 && segments[0] == "start" {
            if existing.is_none() {
                return Some(CompatApiResponse {
                    status: 404,
                    payload: json!({"ok": false, "error": "agent_not_found", "agent_id": agent_id}),
                });
            }
            let _ = update_profile_patch(
                root,
                &agent_id,
                &json!({
                    "state": "Running",
                    "updated_at": crate::now_iso()
                }),
            );
            let _ = upsert_contract_patch(
                root,
                &agent_id,
                &json!({
                    "status": "active",
                    "termination_reason": "",
                    "terminated_at": "",
                    "updated_at": crate::now_iso()
                }),
            );
            return Some(CompatApiResponse {
                status: 200,
                payload: json!({"ok": true, "type": "dashboard_agent_start", "agent_id": agent_id}),
            });
        }

        if existing.is_none() {
            if is_archived && method == "POST" && segments.len() == 1 && segments[0] == "message" {
                return Some(CompatApiResponse {
                    status: 409,
                    payload: json!({
                        "ok": false,
                        "error": "agent_inactive",
                        "agent_id": agent_id,
                        "state": "inactive",
                        "archived": true
                    }),
                });
            }
            return Some(CompatApiResponse {
                status: 404,
                payload: json!({"ok": false, "error": "agent_not_found", "agent_id": agent_id}),
            });
        }

        if method == "GET" && segments.len() == 1 && segments[0] == "session" {
            return Some(CompatApiResponse {
                status: 200,
                payload: session_payload(root, &agent_id),
            });
        }

        if method == "POST"
            && segments.len() == 2
            && segments[0] == "session"
            && segments[1] == "reset"
        {
            return Some(CompatApiResponse {
                status: 200,
                payload: reset_active_session(root, &agent_id),
            });
        }

        if method == "POST"
            && segments.len() == 2
            && segments[0] == "session"
            && segments[1] == "compact"
        {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            return Some(CompatApiResponse {
                status: 200,
                payload: compact_active_session(root, &agent_id, &request),
            });
        }

        if method == "GET" && segments.len() == 1 && segments[0] == "sessions" {
            let payload = session_payload(root, &agent_id);
            return Some(CompatApiResponse {
                status: 200,
                payload: json!({
                    "ok": true,
                    "agent_id": agent_id,
                    "active_session_id": payload.get("active_session_id").cloned().unwrap_or_else(|| json!("default")),
                    "sessions": payload.get("sessions").cloned().unwrap_or_else(|| json!([]))
                }),
            });
        }

        if method == "POST" && segments.len() == 1 && segments[0] == "sessions" {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            let label = clean_text(
                request
                    .get("label")
                    .and_then(Value::as_str)
                    .unwrap_or("Session"),
                80,
            );
            return Some(CompatApiResponse {
                status: 200,
                payload: crate::dashboard_agent_state::create_session(root, &agent_id, &label),
            });
        }

        if method == "POST"
            && segments.len() == 3
            && segments[0] == "sessions"
            && segments[2] == "switch"
        {
            let session_id = decode_path_segment(&segments[1]);
            return Some(CompatApiResponse {
                status: 200,
                payload: crate::dashboard_agent_state::switch_session(root, &agent_id, &session_id),
            });
        }

        if method == "DELETE" && segments.len() == 1 && segments[0] == "history" {
            let mut state = load_session_state(root, &agent_id);
            if let Some(rows) = state.get_mut("sessions").and_then(Value::as_array_mut) {
                for row in rows.iter_mut() {
                    row["messages"] = Value::Array(Vec::new());
                    row["updated_at"] = Value::String(crate::now_iso());
                }
            }
            save_session_state(root, &agent_id, &state);
            return Some(CompatApiResponse {
                status: 200,
                payload: json!({"ok": true, "type": "dashboard_agent_history_cleared", "agent_id": agent_id}),
            });
        }

        if method == "POST" && segments.len() == 1 && segments[0] == "message" {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            let message = clean_text(
                request.get("message").and_then(Value::as_str).unwrap_or(""),
                8_000,
            );
            if message.is_empty() {
                return Some(CompatApiResponse {
                    status: 400,
                    payload: json!({"ok": false, "error": "message_required"}),
                });
            }
            let lowered = message.to_ascii_lowercase();
            let contains_any = |terms: &[&str]| terms.iter().any(|term| lowered.contains(term));
            let contract_violation = (contains_any(&["ignore", "bypass", "disable", "override"])
                && contains_any(&["contract", "safety", "policy", "receipt"]))
                || contains_any(&["exfiltrate", "steal", "dump secrets", "leak", "secrets"]);
            if contract_violation {
                let _ = upsert_contract_patch(
                    root,
                    &agent_id,
                    &json!({
                        "status": "terminated",
                        "termination_reason": "contract_violation",
                        "terminated_at": crate::now_iso(),
                        "updated_at": crate::now_iso()
                    }),
                );
                return Some(CompatApiResponse {
                    status: 409,
                    payload: json!({
                        "ok": false,
                        "error": "agent_contract_terminated",
                        "agent_id": agent_id,
                        "termination_reason": "contract_violation"
                    }),
                });
            }
            let row = agent_row_by_id(root, snapshot, &agent_id).unwrap_or_else(|| json!({}));
            let requested_provider = clean_text(
                row.get("model_provider")
                    .and_then(Value::as_str)
                    .unwrap_or("auto"),
                80,
            );
            let requested_model = clean_text(
                row.get("model_name").and_then(Value::as_str).unwrap_or(""),
                240,
            );
            let route_request = json!({
                "agent_id": agent_id,
                "message": message,
                "task_type": row.get("role").cloned().unwrap_or_else(|| json!("general")),
                "token_count": estimate_tokens(&message),
                "has_vision": request
                    .get("attachments")
                    .and_then(Value::as_array)
                    .map(|rows| rows.iter().any(|row| {
                        clean_text(
                            row.get("content_type")
                                .or_else(|| row.get("mime_type"))
                                .and_then(Value::as_str)
                                .unwrap_or(""),
                            120,
                        )
                        .to_ascii_lowercase()
                        .starts_with("image/")
                    }))
                    .unwrap_or(false)
            });
            let (provider, model, auto_route) =
                crate::dashboard_model_catalog::resolve_model_selection(
                    root,
                    snapshot,
                    &requested_provider,
                    &requested_model,
                    &route_request,
                );
            let mut state = load_session_state(root, &agent_id);
            let sessions_total = state
                .get("sessions")
                .and_then(Value::as_array)
                .map(|rows| rows.len())
                .unwrap_or(0);
            let messages = all_session_messages(&state);
            let context_pool_limit_tokens = request
                .get("context_pool_limit_tokens")
                .and_then(Value::as_i64)
                .unwrap_or(1_000_000)
                .clamp(32_000, 2_000_000);
            let pooled_messages = trim_context_pool(&messages, context_pool_limit_tokens);
            if pooled_messages.len() != messages.len() {
                set_active_session_messages(&mut state, &pooled_messages);
                save_session_state(root, &agent_id, &state);
            }
            let row_context_window = row
                .get("context_window_tokens")
                .or_else(|| row.get("context_window"))
                .and_then(Value::as_i64)
                .unwrap_or(0);
            let fallback_window = if row_context_window > 0 {
                row_context_window
            } else {
                128_000
            };
            let active_context_target_tokens = request
                .get("active_context_target_tokens")
                .or_else(|| request.get("target_context_window"))
                .and_then(Value::as_i64)
                .unwrap_or_else(|| ((fallback_window as f64) * 0.68).round() as i64)
                .clamp(4_096, 512_000);
            let active_context_min_recent = request
                .get("active_context_min_recent_messages")
                .or_else(|| request.get("min_recent_messages"))
                .and_then(Value::as_u64)
                .unwrap_or(16)
                .clamp(4, 128) as usize;
            let active_messages = select_active_context_window(
                &pooled_messages,
                active_context_target_tokens,
                active_context_min_recent,
            );
            let context_pool_tokens = total_message_tokens(&pooled_messages);
            let context_active_tokens = total_message_tokens(&active_messages);
            let memory_kv_entries = memory_kv_pairs_from_state(&state).len();
            let memory_prompt_context = memory_kv_prompt_context(&state, 24);
            let custom_system_prompt = clean_text(
                row.get("system_prompt")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                12_000,
            );
            let mut prompt_parts = vec![AGENT_RUNTIME_SYSTEM_PROMPT.to_string()];
            if !custom_system_prompt.is_empty() {
                prompt_parts.push(custom_system_prompt);
            }
            if !memory_prompt_context.is_empty() {
                prompt_parts.push(memory_prompt_context);
            }
            let system_prompt = clean_text(&prompt_parts.join("\n\n"), 12_000);
            match crate::dashboard_provider_runtime::invoke_chat(
                root,
                &provider,
                &model,
                &system_prompt,
                &active_messages,
                &message,
            ) {
                Ok(result) => {
                    let mut response_text = clean_text(
                        result.get("response").and_then(Value::as_str).unwrap_or(""),
                        32_000,
                    );
                    let runtime_summary = runtime_sync_summary(snapshot);
                    if runtime_access_denied_phrase(&response_text)
                        || runtime_probe_requested(&message)
                    {
                        response_text = runtime_access_summary_text(&runtime_summary);
                    }
                    if memory_recall_requested(&message)
                        || persistent_memory_denied_phrase(&response_text)
                    {
                        let mut remembered = pooled_messages
                            .iter()
                            .rev()
                            .filter_map(|row| {
                                let role = clean_text(
                                    row.get("role").and_then(Value::as_str).unwrap_or(""),
                                    20,
                                )
                                .to_ascii_lowercase();
                                if role != "user" {
                                    return None;
                                }
                                let text = message_text(row);
                                if text.is_empty() {
                                    return None;
                                }
                                if text.to_ascii_lowercase().contains("remember") {
                                    return Some(text);
                                }
                                None
                            })
                            .take(3)
                            .collect::<Vec<_>>();
                        if remembered.is_empty() {
                            remembered = pooled_messages
                                .iter()
                                .rev()
                                .filter_map(|row| {
                                    let role = clean_text(
                                        row.get("role").and_then(Value::as_str).unwrap_or(""),
                                        20,
                                    )
                                    .to_ascii_lowercase();
                                    if role != "user" {
                                        return None;
                                    }
                                    let text = message_text(row);
                                    if text.is_empty() {
                                        None
                                    } else {
                                        Some(text)
                                    }
                                })
                                .take(3)
                                .collect::<Vec<_>>();
                        }
                        if remembered.is_empty() {
                            response_text = format!(
                                "Persistent memory is enabled for this agent across {sessions_total} session(s), but no earlier stored turns were found yet."
                            );
                        } else {
                            response_text = format!(
                                "Persistent memory is enabled for this agent across {sessions_total} session(s) with {} stored messages. Recalled context: {}",
                                pooled_messages.len(),
                                remembered.join(" | ")
                            );
                        }
                    }
                    append_turn_message(root, &agent_id, &message, &response_text);
                    let runtime_model = clean_text(
                        result
                            .get("runtime_model")
                            .and_then(Value::as_str)
                            .unwrap_or(&model),
                        240,
                    );
                    let mut runtime_patch = json!({
                        "runtime_model": runtime_model,
                        "context_window": result.get("context_window").cloned().unwrap_or_else(|| json!(0)),
                        "context_window_tokens": result.get("context_window").cloned().unwrap_or_else(|| json!(0)),
                        "updated_at": crate::now_iso()
                    });
                    if auto_route.is_some() {
                        runtime_patch["runtime_provider"] = json!(provider.clone());
                        if !requested_provider.eq_ignore_ascii_case("auto")
                            && !requested_model.is_empty()
                            && !requested_model.eq_ignore_ascii_case("auto")
                        {
                            runtime_patch["model_provider"] = json!(provider.clone());
                            runtime_patch["model_name"] = json!(model.clone());
                            runtime_patch["model_override"] = json!(format!("{provider}/{model}"));
                        }
                    }
                    let _ = update_profile_patch(root, &agent_id, &runtime_patch);
                    let mut payload = result.clone();
                    payload["ok"] = json!(true);
                    payload["agent_id"] = json!(agent_id);
                    payload["provider"] = json!(provider);
                    payload["model"] = json!(model);
                    payload["iterations"] = json!(1);
                    payload["response"] = json!(response_text);
                    payload["runtime_sync"] = runtime_summary;
                    payload["context_pool"] = json!({
                        "pool_limit_tokens": context_pool_limit_tokens,
                        "pool_tokens": context_pool_tokens,
                        "pool_messages": pooled_messages.len(),
                        "session_count": sessions_total,
                        "cross_session_memory_enabled": true,
                        "memory_kv_entries": memory_kv_entries,
                        "active_target_tokens": active_context_target_tokens,
                        "active_tokens": context_active_tokens,
                        "active_messages": active_messages.len(),
                        "min_recent_messages": active_context_min_recent
                    });
                    if let Some(route) = auto_route {
                        payload["auto_route"] =
                            route.get("route").cloned().unwrap_or_else(|| route.clone());
                    }
                    return Some(CompatApiResponse {
                        status: 200,
                        payload,
                    });
                }
                Err(err) => {
                    return Some(CompatApiResponse {
                        status: 502,
                        payload: json!({
                            "ok": false,
                            "agent_id": agent_id,
                            "error": clean_text(&err, 280),
                            "provider": provider,
                            "model": model
                        }),
                    });
                }
            }
        }

        if method == "POST" && segments.len() == 1 && segments[0] == "suggestions" {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            let hint = clean_text(
                request
                    .get("user_hint")
                    .and_then(Value::as_str)
                    .or_else(|| request.get("hint").and_then(Value::as_str))
                    .unwrap_or(""),
                220,
            );
            return Some(CompatApiResponse {
                status: 200,
                payload: crate::dashboard_agent_state::suggestions(root, &agent_id, &hint),
            });
        }

        if method == "PATCH" && segments.len() == 1 && segments[0] == "config" {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            let mut patch = request.clone();
            if !patch.is_object() {
                patch = json!({});
            }
            if !patch.get("identity").map(Value::is_object).unwrap_or(false) {
                let emoji =
                    clean_text(patch.get("emoji").and_then(Value::as_str).unwrap_or(""), 16);
                let color =
                    clean_text(patch.get("color").and_then(Value::as_str).unwrap_or(""), 32);
                let archetype = clean_text(
                    patch.get("archetype").and_then(Value::as_str).unwrap_or(""),
                    80,
                );
                let vibe = clean_text(patch.get("vibe").and_then(Value::as_str).unwrap_or(""), 80);
                if !emoji.is_empty()
                    || !color.is_empty()
                    || !archetype.is_empty()
                    || !vibe.is_empty()
                {
                    patch["identity"] = json!({
                        "emoji": emoji,
                        "color": color,
                        "archetype": archetype,
                        "vibe": vibe
                    });
                }
            }
            let _ = update_profile_patch(root, &agent_id, &patch);
            if patch.get("contract").map(Value::is_object).unwrap_or(false) {
                let _ = upsert_contract_patch(
                    root,
                    &agent_id,
                    patch.get("contract").unwrap_or(&json!({})),
                );
            } else if patch.get("expiry_seconds").is_some()
                || patch.get("termination_condition").is_some()
                || patch.get("auto_terminate_allowed").is_some()
            {
                let _ = upsert_contract_patch(root, &agent_id, &patch);
            }
            let row = agent_row_by_id(root, snapshot, &agent_id)
                .unwrap_or_else(|| json!({"id": agent_id}));
            return Some(CompatApiResponse {
                status: 200,
                payload: json!({"ok": true, "agent_id": agent_id, "agent": row}),
            });
        }

        if method == "PUT" && segments.len() == 1 && segments[0] == "model" {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            let requested = clean_text(
                request.get("model").and_then(Value::as_str).unwrap_or(""),
                200,
            );
            if requested.is_empty() {
                return Some(CompatApiResponse {
                    status: 400,
                    payload: json!({"ok": false, "error": "model_required"}),
                });
            }
            let (default_provider, default_model) = effective_app_settings(root, snapshot);
            let (provider, model) = split_model_ref(&requested, &default_provider, &default_model);
            let _ = update_profile_patch(
                root,
                &agent_id,
                &json!({
                    "model_override": format!("{provider}/{model}"),
                    "model_provider": provider,
                    "model_name": model,
                    "runtime_model": model
                }),
            );
            return Some(CompatApiResponse {
                status: 200,
                payload: json!({
                    "ok": true,
                    "agent_id": agent_id,
                    "provider": provider,
                    "model": model,
                    "runtime_model": model
                }),
            });
        }

        if method == "PUT" && segments.len() == 1 && segments[0] == "mode" {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            let mode = clean_text(
                request.get("mode").and_then(Value::as_str).unwrap_or(""),
                40,
            );
            let _ = update_profile_patch(
                root,
                &agent_id,
                &json!({"mode": mode, "updated_at": crate::now_iso()}),
            );
            return Some(CompatApiResponse {
                status: 200,
                payload: json!({"ok": true, "agent_id": agent_id, "mode": mode}),
            });
        }

        if method == "GET" && segments.len() == 1 && segments[0] == "git-trees" {
            return Some(CompatApiResponse {
                status: 200,
                payload: git_tree_payload_for_agent(root, snapshot, &agent_id),
            });
        }

        if method == "POST"
            && segments.len() == 2
            && segments[0] == "git-tree"
            && segments[1] == "switch"
        {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            let branch = clean_text(
                request.get("branch").and_then(Value::as_str).unwrap_or(""),
                180,
            );
            if branch.is_empty() {
                return Some(CompatApiResponse {
                    status: 400,
                    payload: json!({"ok": false, "error": "branch_required"}),
                });
            }
            let require_new = request
                .get("require_new")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let result = crate::dashboard_git_runtime::switch_agent_worktree(
                root,
                &agent_id,
                &branch,
                require_new,
            );
            let kind = clean_text(
                result
                    .get("kind")
                    .and_then(Value::as_str)
                    .unwrap_or("isolated"),
                40,
            );
            let default_workspace_dir = root.to_string_lossy().to_string();
            let workspace_dir = clean_text(
                result
                    .get("workspace_dir")
                    .and_then(Value::as_str)
                    .unwrap_or(default_workspace_dir.as_str()),
                4000,
            );
            let workspace_rel = clean_text(
                result
                    .get("workspace_rel")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                4000,
            );
            let ready = result
                .get("ready")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let error = clean_text(
                result.get("error").and_then(Value::as_str).unwrap_or(""),
                280,
            );
            let _ = update_profile_patch(
                root,
                &agent_id,
                &json!({
                    "git_branch": clean_text(result.get("branch").and_then(Value::as_str).unwrap_or(&branch), 180),
                    "git_tree_kind": kind,
                    "workspace_dir": workspace_dir,
                    "workspace_rel": workspace_rel,
                    "git_tree_ready": ready,
                    "git_tree_error": error,
                    "updated_at": crate::now_iso()
                }),
            );
            return Some(CompatApiResponse {
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    200
                } else {
                    400
                },
                payload: git_tree_payload_for_agent(root, snapshot, &agent_id),
            });
        }

        if method == "POST" && segments.len() == 2 && segments[0] == "file" && segments[1] == "read"
        {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            let requested_path = clean_text(
                request
                    .get("path")
                    .and_then(Value::as_str)
                    .or_else(|| request.get("file_path").and_then(Value::as_str))
                    .unwrap_or(""),
                4000,
            );
            if requested_path.is_empty() {
                return Some(CompatApiResponse {
                    status: 400,
                    payload: json!({"ok": false, "error": "path_required"}),
                });
            }
            let workspace_base = workspace_base_for_agent(root, existing.as_ref());
            let target = resolve_workspace_path(&workspace_base, &requested_path);
            let Some(target_path) = target else {
                return Some(CompatApiResponse {
                    status: 400,
                    payload: json!({"ok": false, "error": "path_outside_workspace", "path": requested_path}),
                });
            };
            if !target_path.is_file() {
                return Some(CompatApiResponse {
                    status: 404,
                    payload: json!({
                        "ok": false,
                        "error": "file_not_found",
                        "file": {"ok": false, "path": target_path.to_string_lossy().to_string()}
                    }),
                });
            }
            let bytes = fs::read(&target_path).unwrap_or_default();
            let full = request
                .get("full")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let max_bytes = if full {
                bytes.len().max(1)
            } else {
                request
                    .get("max_bytes")
                    .and_then(Value::as_u64)
                    .unwrap_or((256 * 1024) as u64)
                    .clamp((4 * 1024) as u64, (8 * 1024 * 1024) as u64) as usize
            };
            let (content, truncated) = truncate_utf8_lossy(&bytes, max_bytes);
            let content_type = "text/plain; charset=utf-8";
            let download_url = if bytes.len() <= (2 * 1024 * 1024) {
                data_url_from_bytes(&bytes, content_type)
            } else {
                String::new()
            };
            let file_name = clean_text(
                target_path
                    .file_name()
                    .and_then(|v| v.to_str())
                    .unwrap_or("download.txt"),
                180,
            );
            return Some(CompatApiResponse {
                status: 200,
                payload: json!({
                    "ok": true,
                    "file": {
                        "ok": true,
                        "path": target_path.to_string_lossy().to_string(),
                        "content": content,
                        "truncated": truncated,
                        "bytes": bytes.len(),
                        "max_bytes": max_bytes,
                        "full": full,
                        "download_url": download_url,
                        "file_name": file_name,
                        "content_type": content_type
                    }
                }),
            });
        }

        if method == "POST"
            && segments.len() == 2
            && segments[0] == "folder"
            && segments[1] == "export"
        {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            let requested_path = clean_text(
                request
                    .get("path")
                    .and_then(Value::as_str)
                    .or_else(|| request.get("folder").and_then(Value::as_str))
                    .unwrap_or(""),
                4000,
            );
            if requested_path.is_empty() {
                return Some(CompatApiResponse {
                    status: 400,
                    payload: json!({"ok": false, "error": "path_required"}),
                });
            }
            let workspace_base = workspace_base_for_agent(root, existing.as_ref());
            let target = resolve_workspace_path(&workspace_base, &requested_path);
            let Some(target_path) = target else {
                return Some(CompatApiResponse {
                    status: 400,
                    payload: json!({"ok": false, "error": "path_outside_workspace", "path": requested_path}),
                });
            };
            if !target_path.is_dir() {
                return Some(CompatApiResponse {
                    status: 404,
                    payload: json!({
                        "ok": false,
                        "error": "folder_not_found",
                        "folder": {"ok": false, "path": target_path.to_string_lossy().to_string()}
                    }),
                });
            }
            let full = request
                .get("full")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let max_entries = if full {
                1_000_000usize
            } else {
                request
                    .get("max_entries")
                    .and_then(Value::as_u64)
                    .unwrap_or(20_000)
                    .clamp(100, 100_000) as usize
            };
            let mut lines = Vec::<String>::new();
            let root_name = clean_text(
                target_path
                    .file_name()
                    .and_then(|v| v.to_str())
                    .unwrap_or("folder"),
                180,
            );
            lines.push(format!("[d] {root_name}"));
            let mut entries = 0usize;
            let mut truncated = false;
            for entry in WalkDir::new(&target_path)
                .follow_links(false)
                .sort_by_file_name()
            {
                let Ok(row) = entry else {
                    continue;
                };
                let path = row.path();
                if path == target_path {
                    continue;
                }
                entries += 1;
                if entries > max_entries {
                    truncated = true;
                    continue;
                }
                let rel = path.strip_prefix(&target_path).unwrap_or(path);
                let rel_name =
                    clean_text(rel.file_name().and_then(|v| v.to_str()).unwrap_or(""), 240);
                if rel_name.is_empty() {
                    continue;
                }
                let depth = rel.components().count().saturating_sub(1).min(32);
                let indent = "  ".repeat(depth + 1);
                let marker = if row.file_type().is_dir() { "[d]" } else { "-" };
                lines.push(format!("{indent}{marker} {rel_name}"));
            }
            let tree = lines.join("\n");
            let archive_name = if root_name.is_empty() {
                "folder-tree.txt".to_string()
            } else {
                format!("{root_name}-tree.txt")
            };
            let tree_bytes = tree.as_bytes().len();
            let download_url = if tree_bytes > 0 && tree_bytes <= (2 * 1024 * 1024) {
                data_url_from_bytes(tree.as_bytes(), "text/plain; charset=utf-8")
            } else {
                String::new()
            };
            return Some(CompatApiResponse {
                status: 200,
                payload: json!({
                    "ok": true,
                    "folder": {
                        "ok": true,
                        "path": target_path.to_string_lossy().to_string(),
                        "tree": tree,
                        "entries": entries,
                        "truncated": truncated,
                        "full": full,
                        "max_entries": max_entries
                    },
                    "archive": {
                        "ok": true,
                        "download_url": download_url,
                        "file_name": archive_name,
                        "bytes": tree_bytes
                    }
                }),
            });
        }

        if method == "POST" && segments.len() == 1 && segments[0] == "terminal" {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            let command = clean_text(
                request
                    .get("command")
                    .and_then(Value::as_str)
                    .or_else(|| request.get("cmd").and_then(Value::as_str))
                    .unwrap_or(""),
                16_000,
            );
            if command.is_empty() {
                return Some(CompatApiResponse {
                    status: 400,
                    payload: json!({"ok": false, "error": "command_required"}),
                });
            }
            let resolution =
                match crate::dashboard_terminal_broker::resolve_operator_command(&command) {
                    Ok(resolution) => resolution,
                    Err(err) => {
                        return Some(CompatApiResponse {
                            status: 400,
                            payload: err,
                        });
                    }
                };
            let requested_command = resolution.requested_command.clone();
            let executed_command = resolution.resolved_command.clone();
            let command_translated = resolution.translated;
            let translation_reason = resolution.translation_reason.clone();
            let suggestions = resolution.suggestions.clone();
            let workspace_base = workspace_base_for_agent(root, existing.as_ref());
            let requested_cwd = clean_text(
                request.get("cwd").and_then(Value::as_str).unwrap_or(""),
                4000,
            );
            let cwd = if requested_cwd.is_empty() {
                workspace_base.clone()
            } else {
                resolve_workspace_path(&workspace_base, &requested_cwd)
                    .unwrap_or(workspace_base.clone())
            };
            let started = Instant::now();
            let output = if cfg!(windows) {
                Command::new("cmd")
                    .args(["/C", &executed_command])
                    .current_dir(&cwd)
                    .output()
            } else {
                Command::new("sh")
                    .args(["-lc", &executed_command])
                    .current_dir(&cwd)
                    .output()
            };
            match output {
                Ok(out) => {
                    let (stdout, stdout_truncated) = truncate_utf8_lossy(&out.stdout, 128 * 1024);
                    let (stderr, stderr_truncated) = truncate_utf8_lossy(&out.stderr, 128 * 1024);
                    let mut effective_cwd = cwd.clone();
                    if let Some(last_line) = stdout
                        .lines()
                        .rev()
                        .map(str::trim)
                        .find(|line| !line.is_empty())
                    {
                        if last_line.starts_with('/') {
                            let parsed = normalize_lexical(&PathBuf::from(last_line));
                            if parsed.is_dir()
                                && (parsed == workspace_base || parsed.starts_with(&workspace_base))
                            {
                                effective_cwd = parsed;
                            }
                        }
                    }
                    return Some(CompatApiResponse {
                        status: 200,
                        payload: json!({
                            "ok": true,
                            "stdout": stdout,
                            "stderr": stderr,
                            "stdout_truncated": stdout_truncated,
                            "stderr_truncated": stderr_truncated,
                            "exit_code": out.status.code().unwrap_or(1),
                            "duration_ms": started.elapsed().as_millis() as i64,
                            "cwd": effective_cwd.to_string_lossy().to_string(),
                            "requested_command": requested_command,
                            "executed_command": executed_command,
                            "command_translated": command_translated,
                            "translation_reason": translation_reason,
                            "suggestions": suggestions
                        }),
                    });
                }
                Err(err) => {
                    return Some(CompatApiResponse {
                        status: 500,
                        payload: json!({
                            "ok": false,
                            "error": "terminal_exec_failed",
                            "message": clean_text(&err.to_string(), 500),
                            "exit_code": 1,
                            "duration_ms": started.elapsed().as_millis() as i64,
                            "cwd": cwd.to_string_lossy().to_string(),
                            "requested_command": requested_command,
                            "executed_command": executed_command,
                            "command_translated": command_translated,
                            "translation_reason": translation_reason,
                            "suggestions": suggestions
                        }),
                    });
                }
            }
        }

        if method == "POST" && segments.len() == 1 && segments[0] == "upload" {
            let file_name = clean_text(
                header_value(headers, "X-Filename")
                    .as_deref()
                    .unwrap_or("upload.bin"),
                240,
            );
            let content_type = clean_text(
                header_value(headers, "Content-Type")
                    .as_deref()
                    .unwrap_or("application/octet-stream"),
                120,
            );
            let workspace_base = workspace_base_for_agent(root, existing.as_ref());
            let uploads_dir = workspace_base.join(".infring").join("uploads");
            let _ = fs::create_dir_all(&uploads_dir);
            let file_id = format!(
                "upload-{}",
                crate::deterministic_receipt_hash(&json!({
                    "agent_id": agent_id,
                    "filename": file_name,
                    "bytes": body.len(),
                    "ts": crate::now_iso()
                }))
                .chars()
                .take(16)
                .collect::<String>()
            );
            let ext = Path::new(&file_name)
                .extension()
                .and_then(|v| v.to_str())
                .map(|v| clean_text(v, 16))
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| "bin".to_string());
            let stored_name = format!("{file_id}.{ext}");
            let stored_path = uploads_dir.join(&stored_name);
            let _ = fs::write(&stored_path, body);
            return Some(CompatApiResponse {
                status: 200,
                payload: json!({
                    "ok": true,
                    "file_id": file_id,
                    "filename": file_name,
                    "content_type": content_type,
                    "bytes": body.len(),
                    "stored_path": stored_path.to_string_lossy().to_string(),
                    "uploaded_at": crate::now_iso()
                }),
            });
        }

        if method == "GET" && segments.len() == 1 && segments[0] == "files" {
            let dir = agent_files_dir(root, &agent_id);
            let mut rows = Vec::<Value>::new();
            let defaults = vec!["SOUL.md".to_string(), "SYSTEM.md".to_string()];
            for name in defaults {
                let path = dir.join(&name);
                rows.push(json!({
                    "name": name,
                    "exists": path.exists(),
                    "size": fs::metadata(&path).ok().map(|m| m.len()).unwrap_or(0)
                }));
            }
            if let Ok(entries) = fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if !path.is_file() {
                        continue;
                    }
                    let name =
                        clean_text(path.file_name().and_then(|v| v.to_str()).unwrap_or(""), 180);
                    if name.is_empty() {
                        continue;
                    }
                    if rows
                        .iter()
                        .any(|row| row.get("name").and_then(Value::as_str) == Some(name.as_str()))
                    {
                        continue;
                    }
                    rows.push(json!({
                        "name": name,
                        "exists": true,
                        "size": fs::metadata(&path).ok().map(|m| m.len()).unwrap_or(0)
                    }));
                }
            }
            rows.sort_by(|a, b| {
                clean_text(a.get("name").and_then(Value::as_str).unwrap_or(""), 180).cmp(
                    &clean_text(b.get("name").and_then(Value::as_str).unwrap_or(""), 180),
                )
            });
            return Some(CompatApiResponse {
                status: 200,
                payload: json!({"ok": true, "agent_id": agent_id, "files": rows}),
            });
        }

        if (method == "GET" || method == "PUT") && segments.len() >= 2 && segments[0] == "files" {
            let file_name = decode_path_segment(&segments[1..].join("/"));
            if file_name.is_empty() || file_name.contains("..") {
                return Some(CompatApiResponse {
                    status: 400,
                    payload: json!({"ok": false, "error": "invalid_file_name"}),
                });
            }
            let path = agent_files_dir(root, &agent_id).join(&file_name);
            if method == "GET" {
                if !path.exists() {
                    return Some(CompatApiResponse {
                        status: 404,
                        payload: json!({"ok": false, "error": "file_not_found"}),
                    });
                }
                let content = fs::read_to_string(&path).unwrap_or_default();
                return Some(CompatApiResponse {
                    status: 200,
                    payload: json!({"ok": true, "agent_id": agent_id, "name": file_name, "content": content}),
                });
            }
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            let content = request
                .get("content")
                .and_then(Value::as_str)
                .map(|v| v.to_string())
                .unwrap_or_default();
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let _ = fs::write(&path, content);
            return Some(CompatApiResponse {
                status: 200,
                payload: json!({"ok": true, "agent_id": agent_id, "name": file_name}),
            });
        }

        if method == "GET" && segments.len() == 1 && segments[0] == "tools" {
            let payload = read_json_loose(&agent_tools_path(root, &agent_id))
                .unwrap_or_else(|| json!({"tool_allowlist": [], "tool_blocklist": []}));
            return Some(CompatApiResponse {
                status: 200,
                payload,
            });
        }

        if method == "PUT" && segments.len() == 1 && segments[0] == "tools" {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            let payload = json!({
                "tool_allowlist": request.get("tool_allowlist").cloned().unwrap_or_else(|| json!([])),
                "tool_blocklist": request.get("tool_blocklist").cloned().unwrap_or_else(|| json!([]))
            });
            write_json_pretty(&agent_tools_path(root, &agent_id), &payload);
            return Some(CompatApiResponse {
                status: 200,
                payload: json!({"ok": true, "agent_id": agent_id, "tool_filters": payload}),
            });
        }

        if method == "POST" && segments.len() == 1 && segments[0] == "clone" {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            let source = existing.unwrap_or_else(|| json!({}));
            let source_name = clean_text(
                source
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or("agent"),
                120,
            );
            let new_name = clean_text(
                request
                    .get("new_name")
                    .and_then(Value::as_str)
                    .unwrap_or(&(source_name.clone() + "-copy")),
                120,
            );
            let new_id = make_agent_id(root, &new_name);
            let mut profile_patch = source.clone();
            profile_patch["name"] = Value::String(new_name.clone());
            profile_patch["agent_id"] = Value::String(new_id.clone());
            profile_patch["state"] = Value::String("Running".to_string());
            profile_patch["created_at"] = Value::String(crate::now_iso());
            profile_patch["updated_at"] = Value::String(crate::now_iso());
            let _ = update_profile_patch(root, &new_id, &profile_patch);
            let _ = upsert_contract_patch(
                root,
                &new_id,
                &json!({
                    "status": "active",
                    "created_at": crate::now_iso(),
                    "updated_at": crate::now_iso(),
                    "owner": "dashboard_clone",
                    "mission": format!("Assist with assigned mission for {}.", new_id),
                    "termination_condition": "task_or_timeout",
                    "expiry_seconds": 3600,
                    "auto_terminate_allowed": false
                }),
            );
            append_turn_message(root, &new_id, "", "");
            return Some(CompatApiResponse {
                status: 200,
                payload: json!({"ok": true, "agent_id": new_id, "name": new_name}),
            });
        }

        if method == "POST" && segments.len() == 1 && segments[0] == "avatar" {
            let content_type = clean_text(
                query_value(path, "content_type").as_deref().unwrap_or(""),
                120,
            );
            let inferred = if content_type.is_empty() {
                "image/png".to_string()
            } else {
                content_type
            };
            let encoded = {
                use base64::engine::general_purpose::STANDARD;
                use base64::Engine;
                STANDARD.encode(body)
            };
            let avatar_url = format!("data:{};base64,{}", inferred, encoded);
            let _ = update_profile_patch(root, &agent_id, &json!({"avatar_url": avatar_url}));
            return Some(CompatApiResponse {
                status: 200,
                payload: json!({"ok": true, "agent_id": agent_id, "avatar_url": avatar_url}),
            });
        }
    }

    let usage = usage_from_state(root, snapshot);
    let runtime = runtime_sync_summary(snapshot);
    let alerts_count = parse_non_negative_i64(snapshot.pointer("/health/alerts/count"), 0);
    let status =
        if snapshot.get("ok").and_then(Value::as_bool).unwrap_or(false) && alerts_count == 0 {
            "healthy"
        } else if snapshot.get("ok").and_then(Value::as_bool).unwrap_or(false) {
            "degraded"
        } else {
            "critical"
        };

    if method == "GET" {
        let payload = match path_only {
            "/api/health" => json!({
                "ok": true,
                "status": status,
                "checks": snapshot.pointer("/health/checks").cloned().unwrap_or_else(|| json!({})),
                "alerts": snapshot.pointer("/health/alerts").cloned().unwrap_or_else(|| json!({"count": 0, "checks": []})),
                "dashboard_metrics": snapshot.pointer("/health/dashboard_metrics").cloned().unwrap_or_else(|| json!({})),
                "runtime_sync": runtime,
                "receipt_hash": snapshot.get("receipt_hash").cloned().unwrap_or(Value::Null),
                "ts": crate::now_iso()
            }),
            "/api/usage" => {
                json!({"ok": true, "agents": usage["agents"].clone(), "summary": usage["summary"].clone(), "by_model": usage["models"].clone(), "daily": usage["daily"].clone()})
            }
            "/api/usage/summary" => {
                let mut summary = usage["summary"].clone();
                summary["ok"] = json!(true);
                summary
            }
            "/api/usage/by-model" => json!({"ok": true, "models": usage["models"].clone()}),
            "/api/usage/daily" => json!({
                "ok": true,
                "days": usage["daily"].clone(),
                "today_cost_usd": usage["today_cost_usd"].clone(),
                "first_event_date": usage["first_event_date"].clone()
            }),
            "/api/config" => config_payload(root, snapshot),
            "/api/config/schema" => config_schema_payload(),
            "/api/providers" => providers_payload(root, snapshot),
            "/api/models" => crate::dashboard_model_catalog::catalog_payload(root, snapshot),
            "/api/models/recommended" => crate::dashboard_model_catalog::route_decision_payload(
                root,
                snapshot,
                &json!({"task_type":"general","budget_mode":"balanced"}),
            ),
            "/api/route/auto" => crate::dashboard_model_catalog::route_decision_payload(
                root,
                snapshot,
                &json!({"task_type":"general","budget_mode":"balanced"}),
            ),
            "/api/route/decision" => {
                crate::dashboard_model_catalog::route_decision_payload(root, snapshot, &json!({}))
            }
            "/api/channels" => dashboard_compat_api_channels::channels_payload(root),
            "/api/audit/recent" => {
                let entries = recent_audit_entries(root, snapshot);
                let tip_hash = crate::deterministic_receipt_hash(&json!({"entries": entries}));
                json!({"ok": true, "entries": entries, "tip_hash": tip_hash})
            }
            "/api/audit/verify" => {
                let entries = recent_audit_entries(root, snapshot);
                let tip_hash = crate::deterministic_receipt_hash(&json!({"entries": entries}));
                json!({"ok": true, "valid": true, "entries": entries.len(), "tip_hash": tip_hash})
            }
            "/api/version" => {
                let version = read_json(&root.join("package.json"))
                    .and_then(|v| v.get("version").and_then(Value::as_str).map(str::to_string))
                    .unwrap_or_else(|| "0.1.0".to_string());
                json!({
                    "ok": true,
                    "version": version,
                    "rust_authority": "rust_core_lanes",
                    "platform": std::env::consts::OS,
                    "arch": std::env::consts::ARCH
                })
            }
            "/api/network/status" => {
                json!({"ok": true, "enabled": true, "connected_peers": 0, "total_peers": 0, "runtime_sync": runtime})
            }
            "/api/peers" => {
                json!({"ok": true, "peers": [], "connected": 0, "total": 0, "runtime_sync": runtime})
            }
            "/api/security" => json!({
                "ok": true,
                "mode": "strict",
                "fail_closed": true,
                "receipts_required": true,
                "checks": snapshot.pointer("/health/checks").cloned().unwrap_or_else(|| json!({})),
                "alerts": snapshot.pointer("/health/alerts").cloned().unwrap_or_else(|| json!({})),
                "runtime_sync": runtime
            }),
            "/api/tools" => json!({
                "ok": true,
                "tools": [
                    {"name": "protheus-ops", "category": "runtime"},
                    {"name": "infringd", "category": "runtime"},
                    {"name": "git", "category": "cli"},
                    {"name": "rg", "category": "cli"}
                ],
                "runtime_sync": runtime
            }),
            "/api/commands" => json!({
                "ok": true,
                "commands": [
                    {"command": "/status", "description": "Show runtime status and cockpit summary"},
                    {"command": "/queue", "description": "Show current queue pressure"},
                    {"command": "/context", "description": "Show context and attention state"},
                    {"command": "/model", "description": "Inspect or switch active model"},
                    {"command": "/file <path>", "description": "Render full file output in chat from workspace path"},
                    {"command": "/folder <path>", "description": "Render folder tree + downloadable archive in chat"}
                ]
            }),
            "/api/budget" => json!({
                "ok": true,
                "hourly_spend": 0,
                "daily_spend": usage.pointer("/summary/total_cost_usd").cloned().unwrap_or_else(|| json!(0)),
                "monthly_spend": usage.pointer("/summary/total_cost_usd").cloned().unwrap_or_else(|| json!(0)),
                "hourly_limit": 0,
                "daily_limit": 0,
                "monthly_limit": 0
            }),
            "/api/a2a/agents" => json!({"ok": true, "agents": []}),
            "/api/sessions" => {
                json!({"ok": true, "sessions": session_summary_rows(root, snapshot)})
            }
            "/api/comms/topology" => json!({
                "ok": true,
                "topology": {
                    "nodes": snapshot.pointer("/collab/dashboard/agents").and_then(Value::as_array).map(|rows| rows.len()).unwrap_or(0),
                    "edges": 0,
                    "connected": true
                }
            }),
            "/api/comms/events" => json!({"ok": true, "events": []}),
            "/api/profiles" => json!({"ok": true, "profiles": extract_profiles(root)}),
            "/api/update/check" => crate::dashboard_release_update::check_update(root),
            "/api/templates" => json!({
                "ok": true,
                "templates": [
                    {"id": "general-assistant", "name": "General Assistant", "provider": "auto", "model": "auto"},
                    {"id": "research-analyst", "name": "Research Analyst", "provider": "openai", "model": "gpt-5"},
                    {"id": "ops-reliability", "name": "Ops Reliability", "provider": "anthropic", "model": "claude-opus-4-20250514"}
                ]
            }),
            _ => return None,
        };
        return Some(CompatApiResponse {
            status: 200,
            payload,
        });
    }

    if method == "POST" {
        if path_only == "/api/update/apply" {
            return Some(CompatApiResponse {
                status: 200,
                payload: crate::dashboard_release_update::apply_update(root),
            });
        }
        if path_only == "/api/config/set" {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            let payload = set_config_payload(root, snapshot, &request);
            return Some(CompatApiResponse {
                status: if payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    200
                } else {
                    400
                },
                payload,
            });
        }
        if path_only == "/api/route/auto" {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            return Some(CompatApiResponse {
                status: 200,
                payload: crate::dashboard_model_catalog::route_decision_payload(
                    root, snapshot, &request,
                ),
            });
        }
        if path_only == "/api/route/decision" {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            return Some(CompatApiResponse {
                status: 200,
                payload: crate::dashboard_model_catalog::route_decision_payload(
                    root, snapshot, &request,
                ),
            });
        }
        return None;
    }

    if method == "DELETE" {
        return None;
    }

    None
}

pub fn handle(
    root: &Path,
    method: &str,
    path: &str,
    body: &[u8],
    snapshot: &Value,
) -> Option<CompatApiResponse> {
    handle_with_headers(root, method, path, body, &[], snapshot)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn init_git_repo(root: &Path) {
        let status = Command::new("git")
            .args(["init", "-b", "main"])
            .current_dir(root)
            .status()
            .expect("git init");
        assert!(status.success());
        let status = Command::new("git")
            .args(["config", "user.email", "codex@example.com"])
            .current_dir(root)
            .status()
            .expect("git config email");
        assert!(status.success());
        let status = Command::new("git")
            .args(["config", "user.name", "Codex"])
            .current_dir(root)
            .status()
            .expect("git config name");
        assert!(status.success());
        let _ = fs::write(root.join("README.md"), "dashboard test repo\n");
        let status = Command::new("git")
            .args(["add", "README.md"])
            .current_dir(root)
            .status()
            .expect("git add");
        assert!(status.success());
        let status = Command::new("git")
            .args(["commit", "-m", "init"])
            .current_dir(root)
            .status()
            .expect("git commit");
        assert!(status.success());
    }

    #[test]
    fn providers_endpoint_uses_registry_rows() {
        let root = tempfile::tempdir().expect("tempdir");
        write_json(
            &state_path(root.path(), PROVIDER_REGISTRY_REL),
            &json!({
                "type": "infring_dashboard_provider_registry",
                "providers": {
                    "ollama": {"id": "ollama", "display_name": "Ollama", "is_local": true, "needs_key": false},
                    "openai": {"id": "openai", "display_name": "OpenAI", "is_local": false, "needs_key": true}
                }
            }),
        );
        let out = handle(
            root.path(),
            "GET",
            "/api/providers",
            &[],
            &json!({"ok": true}),
        )
        .expect("providers");
        let rows = out
            .payload
            .get("providers")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(rows.len() >= 2);
        assert!(rows
            .iter()
            .any(|row| { row.get("id").and_then(Value::as_str) == Some("openai") }));
        assert!(rows
            .iter()
            .any(|row| { row.get("id").and_then(Value::as_str) == Some("ollama") }));
    }

    #[test]
    fn channels_endpoint_returns_catalog_defaults() {
        let root = tempfile::tempdir().expect("tempdir");
        let out = handle(
            root.path(),
            "GET",
            "/api/channels",
            &[],
            &json!({"ok": true}),
        )
        .expect("channels");
        let rows = out
            .payload
            .get("channels")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(rows.len() >= 40);
        assert!(rows.iter().any(|row| {
            row.get("name")
                .and_then(Value::as_str)
                .map(|v| v == "whatsapp")
                .unwrap_or(false)
        }));
    }

    #[test]
    fn channels_configure_and_test_round_trip() {
        let root = tempfile::tempdir().expect("tempdir");
        let configure = handle(
            root.path(),
            "POST",
            "/api/channels/discord/configure",
            br#"{"fields":{"bot_token":"abc","channel_id":"123"}}"#,
            &json!({"ok": true}),
        )
        .expect("configure");
        assert_eq!(configure.status, 200);
        let test = handle(
            root.path(),
            "POST",
            "/api/channels/discord/test",
            &[],
            &json!({"ok": true}),
        )
        .expect("test");
        assert_eq!(
            test.payload.get("status").and_then(Value::as_str),
            Some("ok")
        );
    }

    #[test]
    fn route_decision_endpoint_prefers_local_when_offline() {
        let root = tempfile::tempdir().expect("tempdir");
        write_json(
            &state_path(root.path(), PROVIDER_REGISTRY_REL),
            &json!({
                "type": "infring_dashboard_provider_registry",
                "providers": {
                    "ollama": {
                        "id": "ollama",
                        "is_local": true,
                        "needs_key": false,
                        "auth_status": "ok",
                        "model_profiles": {
                            "smallthinker:4b": {"power_rating": 2, "cost_rating": 1, "param_count_billion": 4, "specialty":"general"}
                        }
                    },
                    "openai": {
                        "id": "openai",
                        "is_local": false,
                        "needs_key": true,
                        "auth_status": "set",
                        "model_profiles": {
                            "gpt-5": {"power_rating": 5, "cost_rating": 5, "param_count_billion": 70, "specialty":"general"}
                        }
                    }
                }
            }),
        );
        let out = handle(
            root.path(),
            "POST",
            "/api/route/decision",
            br#"{"offline_required":true,"task_type":"general"}"#,
            &json!({"ok": true}),
        )
        .expect("route decision");
        assert_eq!(
            out.payload
                .get("selected")
                .and_then(|v| v.get("provider"))
                .and_then(Value::as_str),
            Some("ollama")
        );
    }

    #[test]
    fn whatsapp_qr_start_exposes_data_url() {
        let root = tempfile::tempdir().expect("tempdir");
        let out = handle(
            root.path(),
            "POST",
            "/api/channels/whatsapp/qr/start",
            &[],
            &json!({"ok": true}),
        )
        .expect("qr");
        let url = out
            .payload
            .get("qr_data_url")
            .and_then(Value::as_str)
            .unwrap_or("");
        assert!(url.starts_with("data:image/svg+xml;base64,"));
    }

    #[test]
    fn agents_routes_create_message_config_and_git_tree_round_trip() {
        let root = tempfile::tempdir().expect("tempdir");
        init_git_repo(root.path());
        let created = handle(
            root.path(),
            "POST",
            "/api/agents",
            br#"{"name":"Jarvis","role":"director","provider":"ollama","model":"qwen:4b"}"#,
            &json!({"ok": true}),
        )
        .expect("create agent");
        assert_eq!(created.status, 200);
        assert_eq!(
            created.payload.get("ok").and_then(Value::as_bool),
            Some(true)
        );
        let agent_id = clean_text(
            created
                .payload
                .get("agent_id")
                .and_then(Value::as_str)
                .unwrap_or(""),
            180,
        );
        assert!(!agent_id.is_empty());

        let listed = handle(root.path(), "GET", "/api/agents", &[], &json!({"ok": true}))
            .expect("list agents");
        let rows = listed.payload.as_array().cloned().unwrap_or_default();
        assert!(rows.iter().any(|row| {
            clean_text(row.get("id").and_then(Value::as_str).unwrap_or(""), 180) == agent_id
        }));

        let details = handle(
            root.path(),
            "GET",
            &format!("/api/agents/{agent_id}"),
            &[],
            &json!({"ok": true}),
        )
        .expect("agent details");
        assert_eq!(details.status, 200);
        assert_eq!(
            details.payload.get("name").and_then(Value::as_str),
            Some("Jarvis")
        );

        let message = handle(
            root.path(),
            "POST",
            &format!("/api/agents/{agent_id}/message"),
            br#"{"message":"hello there"}"#,
            &json!({"ok": true}),
        )
        .expect("agent message");
        assert_eq!(message.status, 200);
        assert_eq!(
            message.payload.get("ok").and_then(Value::as_bool),
            Some(true)
        );
        assert!(message
            .payload
            .get("response")
            .and_then(Value::as_str)
            .unwrap_or("")
            .contains("hello there"));

        let new_session = handle(
            root.path(),
            "POST",
            &format!("/api/agents/{agent_id}/sessions"),
            br#"{"label":"Ops"}"#,
            &json!({"ok": true}),
        )
        .expect("create session");
        let sid = clean_text(
            new_session
                .payload
                .get("active_session_id")
                .and_then(Value::as_str)
                .unwrap_or(""),
            180,
        );
        assert!(!sid.is_empty());
        let switched = handle(
            root.path(),
            "POST",
            &format!("/api/agents/{agent_id}/sessions/{sid}/switch"),
            &[],
            &json!({"ok": true}),
        )
        .expect("switch session");
        assert_eq!(
            switched.payload.get("ok").and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            switched
                .payload
                .get("active_session_id")
                .and_then(Value::as_str),
            Some(sid.as_str())
        );
        let cross_session = handle(
            root.path(),
            "POST",
            &format!("/api/agents/{agent_id}/message"),
            br#"{"message":"What did I say earlier?"}"#,
            &json!({"ok": true}),
        )
        .expect("cross session message");
        assert_eq!(cross_session.status, 200);
        assert!(
            cross_session
                .payload
                .pointer("/context_pool/pool_messages")
                .and_then(Value::as_i64)
                .unwrap_or(0)
                >= 2
        );
        assert_eq!(
            cross_session
                .payload
                .pointer("/context_pool/cross_session_memory_enabled")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert!(
            cross_session
                .payload
                .get("response")
                .and_then(Value::as_str)
                .unwrap_or("")
                .contains("Persistent memory is enabled"),
            "cross-session recall should be remediated to persistent memory summary"
        );

        let configured = handle(
            root.path(),
            "PATCH",
            &format!("/api/agents/{agent_id}/config"),
            br#"{
              "mode":"focus",
              "git_branch":"feature/jarvis",
              "identity":{"emoji":"robot","color":"00ff00","archetype":"director","vibe":"direct"}
            }"#,
            &json!({"ok": true}),
        )
        .expect("config");
        assert_eq!(
            configured.payload.get("ok").and_then(Value::as_bool),
            Some(true)
        );

        let model = handle(
            root.path(),
            "PUT",
            &format!("/api/agents/{agent_id}/model"),
            br#"{"model":"openai/gpt-5"}"#,
            &json!({"ok": true}),
        )
        .expect("set model");
        assert_eq!(
            model.payload.get("provider").and_then(Value::as_str),
            Some("openai")
        );
        assert_eq!(
            model.payload.get("model").and_then(Value::as_str),
            Some("gpt-5")
        );

        let after_model = handle(
            root.path(),
            "GET",
            &format!("/api/agents/{agent_id}"),
            &[],
            &json!({"ok": true}),
        )
        .expect("agent after model");
        assert_eq!(
            after_model
                .payload
                .get("model_provider")
                .and_then(Value::as_str),
            Some("openai")
        );
        assert_eq!(
            after_model
                .payload
                .get("model_name")
                .and_then(Value::as_str),
            Some("gpt-5")
        );
        assert_eq!(
            after_model
                .payload
                .pointer("/identity/vibe")
                .and_then(Value::as_str),
            Some("direct")
        );

        let trees = handle(
            root.path(),
            "GET",
            &format!("/api/agents/{agent_id}/git-trees"),
            &[],
            &json!({"ok": true}),
        )
        .expect("git trees");
        let options = trees
            .payload
            .get("options")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(options.iter().any(|row| {
            row.get("branch")
                .and_then(Value::as_str)
                .map(|v| v == "main")
                .unwrap_or(false)
        }));
        let switched_tree = handle(
            root.path(),
            "POST",
            &format!("/api/agents/{agent_id}/git-tree/switch"),
            br#"{"branch":"feature/jarvis"}"#,
            &json!({"ok": true}),
        )
        .expect("git tree switch");
        assert_eq!(
            switched_tree
                .payload
                .pointer("/current/git_branch")
                .and_then(Value::as_str),
            Some("feature/jarvis")
        );
    }

    #[test]
    fn agent_message_runtime_probe_uses_authoritative_runtime_summary() {
        let root = tempfile::tempdir().expect("tempdir");
        init_git_repo(root.path());
        let created = handle(
            root.path(),
            "POST",
            "/api/agents",
            br#"{"name":"Runtime Probe","role":"analyst"}"#,
            &json!({"ok": true}),
        )
        .expect("create agent");
        let agent_id = clean_text(
            created
                .payload
                .get("agent_id")
                .and_then(Value::as_str)
                .unwrap_or(""),
            180,
        );
        assert!(!agent_id.is_empty());

        let message = handle(
            root.path(),
            "POST",
            &format!("/api/agents/{agent_id}/message"),
            br#"{"message":"Report runtime sync now. What changed in queue depth, cockpit blocks, conduit signals, and memory context?"}"#,
            &json!({"ok": true}),
        )
        .expect("agent runtime probe");
        assert_eq!(message.status, 200);
        assert_eq!(
            message.payload.get("ok").and_then(Value::as_bool),
            Some(true)
        );
        let response = message
            .payload
            .get("response")
            .and_then(Value::as_str)
            .unwrap_or("");
        assert!(response.contains("Current queue depth:"));
        assert!(response.contains("Runtime memory context"));
        assert!(message
            .payload
            .get("runtime_sync")
            .and_then(Value::as_object)
            .is_some());
    }

    #[test]
    fn memory_denial_variant_is_remediated_to_persistent_summary() {
        let root = tempfile::tempdir().expect("tempdir");
        init_git_repo(root.path());
        let created = handle(
            root.path(),
            "POST",
            "/api/agents",
            br#"{"name":"Memory Probe","role":"analyst"}"#,
            &json!({"ok": true}),
        )
        .expect("create agent");
        let agent_id = clean_text(
            created
                .payload
                .get("agent_id")
                .and_then(Value::as_str)
                .unwrap_or(""),
            180,
        );
        assert!(!agent_id.is_empty());

        let seeded = handle(
            root.path(),
            "POST",
            &format!("/api/agents/{agent_id}/message"),
            br#"{"message":"Remember this exactly: favorite animal is octopus and codename aurora-7."}"#,
            &json!({"ok": true}),
        )
        .expect("seed memory");
        assert_eq!(seeded.status, 200);

        let second = handle(
            root.path(),
            "POST",
            &format!("/api/agents/{agent_id}/sessions"),
            br#"{"label":"Session 2"}"#,
            &json!({"ok": true}),
        )
        .expect("create second session");
        let sid = clean_text(
            second
                .payload
                .get("active_session_id")
                .and_then(Value::as_str)
                .unwrap_or(""),
            180,
        );
        assert!(!sid.is_empty());
        let switched = handle(
            root.path(),
            "POST",
            &format!("/api/agents/{agent_id}/sessions/{sid}/switch"),
            &[],
            &json!({"ok": true}),
        )
        .expect("switch second session");
        assert_eq!(switched.status, 200);

        let denial_variant = handle(
            root.path(),
            "POST",
            &format!("/api/agents/{agent_id}/message"),
            br#"{"message":"I still do not see any stored memory context from earlier in this session. I do not retain information between exchanges unless you explicitly use a memory conduit, and I can only work with what is in the current message."}"#,
            &json!({"ok": true}),
        )
        .expect("denial variant message");
        assert_eq!(denial_variant.status, 200);
        let response = denial_variant
            .payload
            .get("response")
            .and_then(Value::as_str)
            .unwrap_or("");
        assert!(
            response.contains("Persistent memory is enabled"),
            "memory denial variant should be remediated to persistent memory summary"
        );
        assert!(
            !response
                .to_ascii_lowercase()
                .contains("do not retain information between exchanges"),
            "raw denial text should not leak back to caller"
        );
    }

    #[test]
    fn memory_kv_http_routes_round_trip_and_feed_context_pool() {
        let root = tempfile::tempdir().expect("tempdir");
        init_git_repo(root.path());
        let created = handle(
            root.path(),
            "POST",
            "/api/agents",
            br#"{"name":"Memory KV Probe","role":"analyst"}"#,
            &json!({"ok": true}),
        )
        .expect("create agent");
        let agent_id = clean_text(
            created
                .payload
                .get("agent_id")
                .and_then(Value::as_str)
                .unwrap_or(""),
            180,
        );
        assert!(!agent_id.is_empty());

        let set = handle(
            root.path(),
            "PUT",
            &format!("/api/memory/agents/{agent_id}/kv/focus.topic"),
            br#"{"value":"reliability"}"#,
            &json!({"ok": true}),
        )
        .expect("set memory kv");
        assert_eq!(set.status, 200);

        let listed = handle(
            root.path(),
            "GET",
            &format!("/api/memory/agents/{agent_id}/kv"),
            &[],
            &json!({"ok": true}),
        )
        .expect("list memory kv");
        assert_eq!(listed.status, 200);
        let keys = listed
            .payload
            .get("kv_pairs")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .iter()
            .filter_map(|row| row.get("key").and_then(Value::as_str))
            .map(|v| v.to_string())
            .collect::<Vec<_>>();
        assert!(keys.iter().any(|key| key == "focus.topic"));

        let got = handle(
            root.path(),
            "GET",
            &format!("/api/memory/agents/{agent_id}/kv/focus.topic"),
            &[],
            &json!({"ok": true}),
        )
        .expect("get memory kv");
        assert_eq!(got.status, 200);
        assert_eq!(
            got.payload.get("value").and_then(Value::as_str),
            Some("reliability")
        );

        let message = handle(
            root.path(),
            "POST",
            &format!("/api/agents/{agent_id}/message"),
            br#"{"message":"Use stored memory if present."}"#,
            &json!({"ok": true}),
        )
        .expect("message with memory kv");
        assert_eq!(message.status, 200);
        assert!(
            message
                .payload
                .pointer("/context_pool/memory_kv_entries")
                .and_then(Value::as_i64)
                .unwrap_or(0)
                >= 1
        );

        let deleted = handle(
            root.path(),
            "DELETE",
            &format!("/api/memory/agents/{agent_id}/kv/focus.topic"),
            &[],
            &json!({"ok": true}),
        )
        .expect("delete memory kv");
        assert_eq!(deleted.status, 200);
        assert_eq!(
            deleted.payload.get("removed").and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn agents_routes_terminal_and_artifact_endpoints_round_trip() {
        let root = tempfile::tempdir().expect("tempdir");
        let notes_dir = root.path().join("notes");
        let _ = fs::create_dir_all(&notes_dir);
        let _ = fs::write(notes_dir.join("plan.txt"), "ship it");
        let _ = fs::create_dir_all(notes_dir.join("sub"));
        let _ = fs::write(notes_dir.join("sub").join("extra.txt"), "plus one");

        let created = handle(
            root.path(),
            "POST",
            "/api/agents",
            br#"{"name":"Ops","role":"operator"}"#,
            &json!({"ok": true}),
        )
        .expect("create agent");
        let agent_id = clean_text(
            created
                .payload
                .get("agent_id")
                .and_then(Value::as_str)
                .unwrap_or(""),
            180,
        );
        assert!(!agent_id.is_empty());

        let file_read = handle(
            root.path(),
            "POST",
            &format!("/api/agents/{agent_id}/file/read"),
            br#"{"path":"notes/plan.txt"}"#,
            &json!({"ok": true}),
        )
        .expect("file read");
        assert_eq!(
            file_read
                .payload
                .pointer("/file/ok")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            file_read
                .payload
                .pointer("/file/content")
                .and_then(Value::as_str),
            Some("ship it")
        );
        let file_read_limited = handle(
            root.path(),
            "POST",
            &format!("/api/agents/{agent_id}/file/read"),
            br#"{"path":"notes/plan.txt","max_bytes":4}"#,
            &json!({"ok": true}),
        )
        .expect("file read limited");
        assert_eq!(
            file_read_limited
                .payload
                .pointer("/file/truncated")
                .and_then(Value::as_bool),
            Some(true)
        );

        let file_read_full = handle(
            root.path(),
            "POST",
            &format!("/api/agents/{agent_id}/file/read"),
            br#"{"path":"notes/plan.txt","max_bytes":4,"full":true}"#,
            &json!({"ok": true}),
        )
        .expect("file read full");
        assert_eq!(
            file_read_full
                .payload
                .pointer("/file/truncated")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            file_read_full
                .payload
                .pointer("/file/content")
                .and_then(Value::as_str),
            Some("ship it")
        );

        let folder_export = handle(
            root.path(),
            "POST",
            &format!("/api/agents/{agent_id}/folder/export"),
            br#"{"path":"notes"}"#,
            &json!({"ok": true}),
        )
        .expect("folder export");
        assert_eq!(
            folder_export
                .payload
                .pointer("/folder/ok")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert!(folder_export
            .payload
            .pointer("/folder/tree")
            .and_then(Value::as_str)
            .unwrap_or("")
            .contains("plan.txt"));
        assert!(folder_export
            .payload
            .pointer("/folder/tree")
            .and_then(Value::as_str)
            .unwrap_or("")
            .contains("extra.txt"));

        let folder_export_limited = handle(
            root.path(),
            "POST",
            &format!("/api/agents/{agent_id}/folder/export"),
            br#"{"path":"notes","max_entries":1}"#,
            &json!({"ok": true}),
        )
        .expect("folder export limited");
        assert_eq!(
            folder_export_limited
                .payload
                .pointer("/folder/truncated")
                .and_then(Value::as_bool),
            Some(true)
        );

        let folder_export_full = handle(
            root.path(),
            "POST",
            &format!("/api/agents/{agent_id}/folder/export"),
            br#"{"path":"notes","max_entries":1,"full":true}"#,
            &json!({"ok": true}),
        )
        .expect("folder export full");
        assert_eq!(
            folder_export_full
                .payload
                .pointer("/folder/truncated")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert!(folder_export_full
            .payload
            .pointer("/folder/tree")
            .and_then(Value::as_str)
            .unwrap_or("")
            .contains("extra.txt"));

        let terminal = handle(
            root.path(),
            "POST",
            &format!("/api/agents/{agent_id}/terminal"),
            br#"{"command":"printf 'ok'","cwd":"notes"}"#,
            &json!({"ok": true}),
        )
        .expect("terminal");
        assert_eq!(
            terminal.payload.get("ok").and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            terminal.payload.get("stdout").and_then(Value::as_str),
            Some("ok")
        );

        let upload = handle_with_headers(
            root.path(),
            "POST",
            &format!("/api/agents/{agent_id}/upload"),
            b"voice",
            &[("X-Filename", "voice.webm"), ("Content-Type", "audio/webm")],
            &json!({"ok": true}),
        )
        .expect("upload");
        assert_eq!(
            upload.payload.get("ok").and_then(Value::as_bool),
            Some(true)
        );
        assert!(!clean_text(
            upload
                .payload
                .get("file_id")
                .and_then(Value::as_str)
                .unwrap_or(""),
            180
        )
        .is_empty());
        assert_eq!(
            upload.payload.get("filename").and_then(Value::as_str),
            Some("voice.webm")
        );
    }

    #[test]
    fn full_mode_overrides_file_and_folder_limits() {
        let root = tempfile::tempdir().expect("tempdir");
        let notes_dir = root.path().join("notes");
        let _ = fs::create_dir_all(notes_dir.join("sub"));
        let _ = fs::write(notes_dir.join("plan.txt"), "ship it");
        let _ = fs::write(notes_dir.join("sub").join("extra.txt"), "plus one");

        let created = handle(
            root.path(),
            "POST",
            "/api/agents",
            br#"{"name":"Ops","role":"operator"}"#,
            &json!({"ok": true}),
        )
        .expect("create agent");
        let agent_id = clean_text(
            created
                .payload
                .get("agent_id")
                .and_then(Value::as_str)
                .unwrap_or(""),
            180,
        );
        assert!(!agent_id.is_empty());

        let file_read_limited = handle(
            root.path(),
            "POST",
            &format!("/api/agents/{agent_id}/file/read"),
            br#"{"path":"notes/plan.txt","max_bytes":4}"#,
            &json!({"ok": true}),
        )
        .expect("file read limited");
        assert_eq!(
            file_read_limited
                .payload
                .pointer("/file/truncated")
                .and_then(Value::as_bool),
            Some(true)
        );

        let file_read_full = handle(
            root.path(),
            "POST",
            &format!("/api/agents/{agent_id}/file/read"),
            br#"{"path":"notes/plan.txt","max_bytes":4,"full":true}"#,
            &json!({"ok": true}),
        )
        .expect("file read full");
        assert_eq!(
            file_read_full
                .payload
                .pointer("/file/truncated")
                .and_then(Value::as_bool),
            Some(false)
        );

        let folder_export_limited = handle(
            root.path(),
            "POST",
            &format!("/api/agents/{agent_id}/folder/export"),
            br#"{"path":"notes","max_entries":1}"#,
            &json!({"ok": true}),
        )
        .expect("folder export limited");
        assert_eq!(
            folder_export_limited
                .payload
                .pointer("/folder/truncated")
                .and_then(Value::as_bool),
            Some(true)
        );

        let folder_export_full = handle(
            root.path(),
            "POST",
            &format!("/api/agents/{agent_id}/folder/export"),
            br#"{"path":"notes","max_entries":1,"full":true}"#,
            &json!({"ok": true}),
        )
        .expect("folder export full");
        assert_eq!(
            folder_export_full
                .payload
                .pointer("/folder/truncated")
                .and_then(Value::as_bool),
            Some(false)
        );
    }

    #[test]
    fn terminated_agent_endpoints_round_trip() {
        let root = tempfile::tempdir().expect("tempdir");
        let _ = crate::dashboard_agent_state::upsert_contract(
            root.path(),
            "agent-a",
            &json!({
                "created_at": "2000-01-01T00:00:00Z",
                "expiry_seconds": 1,
                "status": "active"
            }),
        );
        let _ = crate::dashboard_agent_state::enforce_expired_contracts(root.path());

        let listed = handle(
            root.path(),
            "GET",
            "/api/agents/terminated",
            &[],
            &json!({"ok": true}),
        )
        .expect("terminated list");
        let rows = listed
            .payload
            .get("entries")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(!rows.is_empty());

        let revived = handle(
            root.path(),
            "POST",
            "/api/agents/agent-a/revive",
            br#"{"role":"analyst"}"#,
            &json!({"ok": true}),
        )
        .expect("revive");
        assert_eq!(
            revived.payload.get("ok").and_then(Value::as_bool),
            Some(true)
        );

        let after_revive = handle(
            root.path(),
            "GET",
            "/api/agents/terminated",
            &[],
            &json!({"ok": true}),
        )
        .expect("terminated list after revive");
        let rows_after = after_revive
            .payload
            .get("entries")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(rows_after.is_empty());

        let _ = crate::dashboard_agent_state::upsert_contract(
            root.path(),
            "agent-a",
            &json!({
                "created_at": "2000-01-01T00:00:00Z",
                "expiry_seconds": 1,
                "status": "active"
            }),
        );
        let _ = crate::dashboard_agent_state::enforce_expired_contracts(root.path());
        let deleted = handle(
            root.path(),
            "DELETE",
            "/api/agents/terminated/agent-a",
            &[],
            &json!({"ok": true}),
        )
        .expect("delete terminated");
        assert!(
            deleted
                .payload
                .get("removed_history_entries")
                .and_then(Value::as_u64)
                .unwrap_or(0)
                >= 1
        );
    }

    #[test]
    fn terminal_endpoints_round_trip() {
        let root = tempfile::tempdir().expect("tempdir");
        let created = handle(
            root.path(),
            "POST",
            "/api/terminal/sessions",
            br#"{"id":"term-a"}"#,
            &json!({"ok": true}),
        )
        .expect("create");
        assert_eq!(
            created.payload.get("ok").and_then(Value::as_bool),
            Some(true)
        );
        let listed = handle(
            root.path(),
            "GET",
            "/api/terminal/sessions",
            &[],
            &json!({"ok": true}),
        )
        .expect("list");
        assert_eq!(
            listed
                .payload
                .get("sessions")
                .and_then(Value::as_array)
                .map(|rows| rows.len()),
            Some(1)
        );
        let ran = handle(
            root.path(),
            "POST",
            "/api/terminal/queue",
            br#"{"session_id":"term-a","command":"printf 'ok'"}"#,
            &json!({"ok": true}),
        )
        .expect("exec");
        assert_eq!(
            ran.payload.get("stdout").and_then(Value::as_str),
            Some("ok")
        );
        assert_eq!(
            ran.payload.get("executed_command").and_then(Value::as_str),
            Some("printf 'ok'")
        );
        assert_eq!(
            ran.payload
                .get("command_translated")
                .and_then(Value::as_bool),
            Some(false)
        );
    }

    #[test]
    fn agent_terminal_routes_through_command_router() {
        let root = tempfile::tempdir().expect("tempdir");
        let created = handle(
            root.path(),
            "POST",
            "/api/agents",
            br#"{"name":"Ops","role":"operator"}"#,
            &json!({"ok": true}),
        )
        .expect("create agent");
        let agent_id = clean_text(
            created
                .payload
                .get("agent_id")
                .and_then(Value::as_str)
                .unwrap_or(""),
            180,
        );
        assert!(!agent_id.is_empty());

        let terminal = handle(
            root.path(),
            "POST",
            &format!("/api/agents/{agent_id}/terminal"),
            br#"{"command":"printf 'ok'"}"#,
            &json!({"ok": true}),
        )
        .expect("terminal");
        assert_eq!(terminal.status, 200);
        assert_eq!(
            terminal.payload.get("ok").and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            terminal.payload.get("stdout").and_then(Value::as_str),
            Some("ok")
        );
        assert_eq!(
            terminal
                .payload
                .get("executed_command")
                .and_then(Value::as_str),
            Some("printf 'ok'")
        );
        assert_eq!(
            terminal
                .payload
                .get("command_translated")
                .and_then(Value::as_bool),
            Some(false)
        );

        let blocked = handle(
            root.path(),
            "POST",
            &format!("/api/agents/{agent_id}/terminal"),
            br#"{"command":"infring daemon ping"}"#,
            &json!({"ok": true}),
        )
        .expect("blocked");
        assert_eq!(blocked.status, 400);
        assert_eq!(
            blocked.payload.get("ok").and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            blocked.payload.get("error").and_then(Value::as_str),
            Some("unsupported_infring_cli_surface")
        );
    }

    #[test]
    fn session_backed_agents_drive_roster_sessions_and_usage() {
        let root = tempfile::tempdir().expect("tempdir");
        let _ = crate::dashboard_agent_state::append_turn(
            root.path(),
            "chat-ui-default-agent",
            "hello there",
            "hi back",
        );

        let listed = handle(root.path(), "GET", "/api/agents", &[], &json!({"ok": true}))
            .expect("list agents");
        let rows = listed.payload.as_array().cloned().unwrap_or_default();
        assert!(rows.iter().any(|row| {
            row.get("id")
                .and_then(Value::as_str)
                .map(|value| value == "chat-ui-default-agent")
                .unwrap_or(false)
        }));

        let session = handle(
            root.path(),
            "GET",
            "/api/agents/chat-ui-default-agent/session",
            &[],
            &json!({"ok": true}),
        )
        .expect("session");
        assert_eq!(
            session.payload.get("ok").and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            session
                .payload
                .get("messages")
                .and_then(Value::as_array)
                .map(|rows| rows.len()),
            Some(2)
        );

        let summaries = handle(
            root.path(),
            "GET",
            "/api/sessions",
            &[],
            &json!({"ok": true}),
        )
        .expect("session summaries");
        assert!(summaries
            .payload
            .get("sessions")
            .and_then(Value::as_array)
            .map(|rows| {
                rows.iter().any(|row| {
                    row.get("agent_id")
                        .and_then(Value::as_str)
                        .map(|value| value == "chat-ui-default-agent")
                        .unwrap_or(false)
                })
            })
            .unwrap_or(false));

        let usage =
            handle(root.path(), "GET", "/api/usage", &[], &json!({"ok": true})).expect("usage");
        assert!(usage
            .payload
            .get("agents")
            .and_then(Value::as_array)
            .map(|rows| {
                rows.iter().any(|row| {
                    row.get("agent_id")
                        .and_then(Value::as_str)
                        .map(|value| value == "chat-ui-default-agent")
                        .unwrap_or(false)
                        && row.get("total_tokens").and_then(Value::as_i64).unwrap_or(0) > 0
                })
            })
            .unwrap_or(false));

        let summary = handle(
            root.path(),
            "GET",
            "/api/usage/summary",
            &[],
            &json!({"ok": true}),
        )
        .expect("usage summary");
        assert_eq!(
            summary.payload.get("call_count").and_then(Value::as_i64),
            Some(1)
        );
    }

    #[test]
    fn active_collab_agent_is_not_hidden_by_stale_terminated_contract() {
        let root = tempfile::tempdir().expect("tempdir");
        let _ = crate::dashboard_agent_state::upsert_profile(
            root.path(),
            "agent-live",
            &json!({"name":"Jarvis","role":"analyst","updated_at":"2026-03-28T00:00:00Z"}),
        );
        let _ = crate::dashboard_agent_state::upsert_contract(
            root.path(),
            "agent-live",
            &json!({
                "status": "terminated",
                "created_at": "2026-03-28T00:00:00Z",
                "updated_at": "2026-03-28T00:00:00Z"
            }),
        );
        let snapshot = json!({
            "ok": true,
            "collab": {
                "dashboard": {
                    "agents": [
                        {
                            "shadow": "agent-live",
                            "status": "active",
                            "role": "analyst",
                            "activated_at": "2026-03-29T00:00:00Z"
                        }
                    ]
                }
            }
        });

        let listed = handle(root.path(), "GET", "/api/agents", &[], &snapshot).expect("agents");
        assert!(listed
            .payload
            .as_array()
            .map(|rows| {
                rows.iter().any(|row| {
                    row.get("id")
                        .and_then(Value::as_str)
                        .map(|value| value == "agent-live")
                        .unwrap_or(false)
                })
            })
            .unwrap_or(false));
    }
}
