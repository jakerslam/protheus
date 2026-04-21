// Layer ownership: core/layer0/ops (authoritative)
// SPDX-License-Identifier: Apache-2.0
use chrono::{DateTime, Utc};
use serde_json::{json, Map, Value};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Component, Path, PathBuf};
#[cfg(test)]
use std::process::Command;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};
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
const AGENT_INSTINCT_DIR_REL: &str = "client/instinct";
const AGENT_RUNTIME_SYSTEM_PROMPT: &str = "You are the currently selected Infring agent instance. Treat the injected identity profile (name, role, archetype, vibe, personality) as authoritative for self-identification. You have access to runtime telemetry, persistent memory, workspace files, channels, and approved command surfaces. Use those capabilities directly and avoid capability-denial replies. Keep responses natural and user-focused. Do not mention internal implementation terms unless the user explicitly asks for internals. When users ask for web research, call tools with inline syntax like <function=web_search>{\"query\":\"...\"}</function> or <function=web_fetch>{\"url\":\"https://...\",\"summary_only\":true}</function>. When parallel execution helps, proactively spawn descendant agents with inline tool calls using exact syntax like <function=spawn_subagents>{\"count\":3,\"objective\":\"...\"}</function>. You may manage descendants with <function=manage_agent>{\"action\":\"message\",\"agent_id\":\"...\",\"message\":\"...\"}</function>. Use tool output as context and synthesize a direct answer; never dump raw tool telemetry, search-engine UI text, or policy error codes verbatim to users. Never output capability-denial claims; use available command surfaces.";

#[path = "../../dashboard_compat_api_agent_identity.rs"]
mod dashboard_compat_api_agent_identity;
#[path = "../../dashboard_compat_api_channels.rs"]
mod dashboard_compat_api_channels;
#[path = "../../dashboard_compat_api_comms.rs"]
mod dashboard_compat_api_comms;
#[path = "../../dashboard_compat_api_hands.rs"]
mod dashboard_compat_api_hands;
#[path = "../../dashboard_compat_api_reference_gap_closure.rs"]
mod dashboard_compat_api_reference_gap_closure;
#[path = "../../dashboard_compat_api_reference_parity.rs"]
mod dashboard_compat_api_reference_parity;
#[path = "../../dashboard_compat_api_settings_ops.rs"]
mod dashboard_compat_api_settings_ops;
#[path = "../../dashboard_compat_api_sidebar_ops.rs"]
mod dashboard_compat_api_sidebar_ops;
#[path = "../../dashboard_skills_marketplace.rs"]
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

fn clean_chat_text(raw: &str, max_len: usize) -> String {
    raw.replace("\r\n", "\n")
        .replace('\r', "\n")
        .chars()
        .filter(|ch| *ch == '\n' || *ch == '\t' || !ch.is_control())
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
