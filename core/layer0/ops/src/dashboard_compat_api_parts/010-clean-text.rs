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

#[path = "../dashboard_compat_api_agent_identity.rs"]
mod dashboard_compat_api_agent_identity;
#[path = "../dashboard_compat_api_channels.rs"]
mod dashboard_compat_api_channels;
#[path = "../dashboard_compat_api_comms.rs"]
mod dashboard_compat_api_comms;
#[path = "../dashboard_compat_api_hands.rs"]
mod dashboard_compat_api_hands;
#[path = "../dashboard_compat_api_reference_gap_closure.rs"]
mod dashboard_compat_api_reference_gap_closure;
#[path = "../dashboard_compat_api_reference_parity.rs"]
mod dashboard_compat_api_reference_parity;
#[path = "../dashboard_compat_api_settings_ops.rs"]
mod dashboard_compat_api_settings_ops;
#[path = "../dashboard_compat_api_sidebar_ops.rs"]
mod dashboard_compat_api_sidebar_ops;
#[path = "../dashboard_skills_marketplace.rs"]
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

fn runtime_access_denied_phrase(text: &str) -> bool {
    let lowered = text.to_ascii_lowercase();
    let normalized = lowered
        .replace('’', "'")
        .replace('`', "'")
        .replace('\u{201c}', "\"")
        .replace('\u{201d}', "\"");
    let internal_meta_dump = normalized.contains("internal memory metadata")
        || normalized.contains("instead of actually answering your question")
        || normalized.contains("bug in my response generation")
        || normalized.contains("my response generation")
        || normalized.contains("which of the suggestions did you implement")
        || normalized.contains("if you can tell me which lever you pulled")
        || normalized.contains("what should i be looking for");
    let workspace_only_capability_dump = normalized
        .contains("i can only read what's in your workspace files")
        || normalized.contains("i can only read what is in your workspace files")
        || normalized.contains("i don't have inherent introspection")
        || normalized.contains("i do not have inherent introspection")
        || normalized.contains("beyond what i can infer from runtime behavior")
        || normalized.contains("this particular instance appears under-provisioned")
        || normalized.contains("heavily sandboxed")
        || normalized.contains("missing basic fetch capabilities");
    normalized.contains("don't have access")
        || normalized.contains("do not have access")
        || normalized.contains("cannot access")
        || normalized.contains("no web access")
        || normalized.contains("no internet access")
        || normalized.contains("text-based ai assistant without system monitoring capabilities")
        || normalized.contains("without system monitoring")
        || normalized.contains("text-based ai assistant")
        || normalized.contains("cannot directly interface")
        || normalized.contains("cannot execute the protheus-ops commands")
        || normalized.contains("check your system monitoring tools")
        || normalized.contains("no access to")
        || workspace_only_capability_dump
        || internal_meta_dump
}

fn internal_context_metadata_phrase(text: &str) -> bool {
    let lowered = text.to_ascii_lowercase();
    let has_recalled_context = lowered.contains("recalled context:");
    let has_persistent_memory = lowered.contains("persistent memory");
    let has_stored_messages = lowered.contains("stored messages");
    let has_session_count = lowered.contains("session(s)") || lowered.contains(" sessions");
    (has_recalled_context && (has_persistent_memory || has_stored_messages || has_session_count))
        || (has_persistent_memory && has_stored_messages)
}

fn strip_internal_context_metadata_prefix(text: &str) -> String {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let lowered = trimmed.to_ascii_lowercase();
    let Some(marker_idx) = lowered.find("recalled context:") else {
        return trimmed.to_string();
    };
    let prefix = &lowered[..marker_idx];
    let internal_prefix = prefix.contains("persistent memory")
        || prefix.contains("stored messages")
        || prefix.contains("session(s)")
        || prefix.contains(" sessions");
    if !internal_prefix {
        return trimmed.to_string();
    }
    let suffix = trimmed
        .split_once("Recalled context:")
        .map(|(_, tail)| tail)
        .or_else(|| {
            trimmed
                .split_once("recalled context:")
                .map(|(_, tail)| tail)
        })
        .or_else(|| {
            trimmed
                .split_once("RECALLED CONTEXT:")
                .map(|(_, tail)| tail)
        })
        .unwrap_or("")
        .trim();
    if suffix.is_empty() {
        return String::new();
    }
    if let Some((_, tail)) = suffix.split_once("\n\n") {
        let cleaned = tail.trim();
        if !cleaned.is_empty() {
            return cleaned.to_string();
        }
    }
    if let Some((_, tail)) = suffix.split_once("Final answer:") {
        let cleaned = tail.trim();
        if !cleaned.is_empty() {
            return cleaned.to_string();
        }
    }
    if let Some((_, tail)) = suffix.split_once("Answer:") {
        let cleaned = tail.trim();
        if !cleaned.is_empty() {
            return cleaned.to_string();
        }
    }
    String::new()
}

fn strip_internal_cache_control_markup(text: &str) -> String {
    let mut cleaned = clean_chat_text(text, 64_000);
    loop {
        let lowered = cleaned.to_ascii_lowercase();
        let Some(start) = lowered.find("<cache_control") else {
            break;
        };
        let tail = &lowered[start..];
        let end_rel = tail
            .find("/>")
            .map(|idx| idx + 2)
            .or_else(|| {
                tail.find("</cache_control>")
                    .map(|idx| idx + "</cache_control>".len())
            })
            .or_else(|| tail.find('>').map(|idx| idx + 1))
            .unwrap_or(tail.len());
        let end = start.saturating_add(end_rel).min(cleaned.len());
        if end <= start {
            break;
        }
        cleaned.replace_range(start..end, "");
    }
    cleaned
        .lines()
        .filter(|line| {
            let lowered = line.to_ascii_lowercase();
            !(lowered.contains("stable_hash=")
                && (lowered.contains("cache_control") || lowered.contains("cache control")))
        })
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
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
        || lowered.contains("unless you explicitly use a memory conduit")
        || lowered.contains("persistent memory is enabled for this agent across")
        || lowered.contains("recalled context:")
        || internal_context_metadata_phrase(text)
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
        || lowered.contains("active workers")
        || lowered.contains("live signals")
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

fn swarm_intent_requested(text: &str) -> bool {
    let lowered = text.to_ascii_lowercase();
    lowered.contains("swarm")
        || lowered.contains("summon swarm")
        || lowered.contains("summon a swarm")
        || lowered.contains("subagent")
        || lowered.contains("sub-agent")
        || lowered.contains("descendant agent")
        || lowered.contains("parallel")
        || lowered.contains("split into")
        || lowered.contains("spawn agent")
        || lowered.contains("spawn workers")
        || lowered.contains("spin up agents")
}

fn spawn_surface_denied_phrase(text: &str) -> bool {
    let lowered = text.to_ascii_lowercase();
    (lowered.contains("command surface") && lowered.contains("spawn"))
        || (lowered.contains("don't currently see") && lowered.contains("spawn"))
        || (lowered.contains("do not currently see") && lowered.contains("spawn"))
        || (lowered.contains("requires") && lowered.contains("activation path"))
        || (lowered.contains("might require") && lowered.contains("runtime instances"))
        || (lowered.contains("don't have") && lowered.contains("swarm"))
        || (lowered.contains("do not have") && lowered.contains("swarm"))
}

fn infer_subagent_count_from_message(text: &str) -> usize {
    let lowered = text.to_ascii_lowercase();
    for token in lowered
        .split(|ch: char| !ch.is_ascii_digit())
        .filter(|token| !token.is_empty())
    {
        if let Ok(value) = token.parse::<usize>() {
            if value > 0 {
                return value.clamp(1, 8);
            }
        }
    }
    if lowered.contains("dozen") || lowered.contains("many") || lowered.contains("all") {
        return 5;
    }
    if lowered.contains("comprehensive") || lowered.contains("across") || lowered.contains("stress")
    {
        return 4;
    }
    if lowered.contains("parallel") || lowered.contains("swarm") || lowered.contains("subagent") {
        return 3;
    }
    2
}

fn user_requested_internal_runtime_details(text: &str) -> bool {
    let lowered = text.to_ascii_lowercase();
    lowered.contains("conduit")
        || lowered.contains("cockpit")
        || lowered.contains("attention queue")
        || lowered.contains("memory lane")
        || lowered.contains("runtime lane")
        || lowered.contains("internal mechanics")
        || lowered.contains("system internals")
}

fn abstract_runtime_mechanics_terms(text: &str) -> String {
    let mut rewritten = text.to_string();
    let replacements = [
        ("conduit signals", "live signals"),
        ("cockpit blocks", "active workers"),
        ("attention queue", "priority queue"),
        ("memory context", "memory state"),
        ("runtime lane", "runtime path"),
    ];
    for (from, to) in replacements {
        rewritten = rewritten.replace(from, to);
        rewritten = rewritten.replace(&from.to_ascii_uppercase(), &to.to_ascii_uppercase());
        let capitalized_from = from
            .split(' ')
            .map(|segment| {
                let mut chars = segment.chars();
                match chars.next() {
                    Some(first) => format!(
                        "{}{}",
                        first.to_ascii_uppercase(),
                        chars.collect::<String>()
                    ),
                    None => String::new(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ");
        let capitalized_to = to
            .split(' ')
            .map(|segment| {
                let mut chars = segment.chars();
                match chars.next() {
                    Some(first) => format!(
                        "{}{}",
                        first.to_ascii_uppercase(),
                        chars.collect::<String>()
                    ),
                    None => String::new(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ");
        rewritten = rewritten.replace(&capitalized_from, &capitalized_to);
    }
    rewritten
}

fn runtime_access_summary_text(runtime_summary: &Value) -> String {
    let queue_depth = parse_non_negative_i64(runtime_summary.get("queue_depth"), 0);
    let cockpit_blocks = parse_non_negative_i64(runtime_summary.get("cockpit_blocks"), 0);
    let cockpit_total_blocks =
        parse_non_negative_i64(runtime_summary.get("cockpit_total_blocks"), 0);
    let conduit_signals = parse_non_negative_i64(runtime_summary.get("conduit_signals"), 0);
    format!(
        "Current queue depth: {queue_depth}, active workers: {cockpit_blocks} ({cockpit_total_blocks} total), live signals: {conduit_signals}. Runtime status, persistent memory, and command surfaces are available."
    )
}

#[cfg(test)]
include!("010-clean-text-tests.rs");
