// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use serde_json::{json, Map, Value};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use crate::contract_lane_utils as lane_utils;
use crate::{deterministic_receipt_hash, now_iso};

const DEFAULT_STATE_REL: &str = "local/state/ops/langchain_bridge/latest.json";
const DEFAULT_HISTORY_REL: &str = "local/state/ops/langchain_bridge/history.jsonl";
const DEFAULT_SWARM_STATE_REL: &str = "local/state/ops/langchain_bridge/swarm_state.json";

fn usage() {
    println!("langchain-bridge commands:");
    println!("  protheus-ops langchain-bridge status [--state-path=<path>]");
    println!("  protheus-ops langchain-bridge register-chain [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops langchain-bridge execute-chain [--payload-base64=<json>] [--state-path=<path>] [--swarm-state-path=<path>]");
    println!("  protheus-ops langchain-bridge register-middleware [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops langchain-bridge run-deep-agent [--payload-base64=<json>] [--state-path=<path>] [--swarm-state-path=<path>]");
    println!("  protheus-ops langchain-bridge register-memory-bridge [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops langchain-bridge recall-memory [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops langchain-bridge import-integration [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops langchain-bridge route-prompt [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops langchain-bridge parse-structured-output [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops langchain-bridge record-trace [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops langchain-bridge checkpoint-run [--payload-base64=<json>] [--state-path=<path>] [--swarm-state-path=<path>]");
    println!("  protheus-ops langchain-bridge assimilate-intake [--payload-base64=<json>] [--state-path=<path>]");
}

fn cli_receipt(kind: &str, payload: Value) -> Value {
    let ts = now_iso();
    let ok = payload.get("ok").and_then(Value::as_bool).unwrap_or(true);
    let mut out = json!({
        "ok": ok,
        "type": kind,
        "ts": ts,
        "date": ts[..10].to_string(),
        "payload": payload,
    });
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    out
}

fn cli_error(kind: &str, error: &str) -> Value {
    let ts = now_iso();
    let mut out = json!({
        "ok": false,
        "type": kind,
        "ts": ts,
        "date": ts[..10].to_string(),
        "error": error,
        "fail_closed": true,
    });
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    out
}

fn print_json_line(value: &Value) {
    println!(
        "{}",
        serde_json::to_string(value)
            .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode_failed\"}".to_string())
    );
}

fn payload_json(argv: &[String]) -> Result<Value, String> {
    if let Some(raw) = lane_utils::parse_flag(argv, "payload", false) {
        return serde_json::from_str::<Value>(&raw)
            .map_err(|err| format!("langchain_bridge_payload_decode_failed:{err}"));
    }
    if let Some(raw_b64) = lane_utils::parse_flag(argv, "payload-base64", false) {
        let bytes = BASE64_STANDARD
            .decode(raw_b64.as_bytes())
            .map_err(|err| format!("langchain_bridge_payload_base64_decode_failed:{err}"))?;
        let text = String::from_utf8(bytes)
            .map_err(|err| format!("langchain_bridge_payload_utf8_decode_failed:{err}"))?;
        return serde_json::from_str::<Value>(&text)
            .map_err(|err| format!("langchain_bridge_payload_decode_failed:{err}"));
    }
    Ok(json!({}))
}

fn payload_obj<'a>(value: &'a Value) -> &'a Map<String, Value> {
    value.as_object().unwrap_or_else(|| {
        static EMPTY: OnceLock<Map<String, Value>> = OnceLock::new();
        EMPTY.get_or_init(Map::new)
    })
}

fn repo_path(root: &Path, rel: &str) -> PathBuf {
    let candidate = PathBuf::from(rel.trim());
    if candidate.is_absolute() {
        candidate
    } else {
        root.join(candidate)
    }
}

fn rel(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .map(|value| value.display().to_string())
        .unwrap_or_else(|_| path.display().to_string())
}

fn state_path(root: &Path, argv: &[String], payload: &Map<String, Value>) -> PathBuf {
    lane_utils::parse_flag(argv, "state-path", false)
        .or_else(|| {
            payload
                .get("state_path")
                .and_then(Value::as_str)
                .map(ToString::to_string)
        })
        .map(|raw| repo_path(root, &raw))
        .unwrap_or_else(|| root.join(DEFAULT_STATE_REL))
}

fn history_path(root: &Path, argv: &[String], payload: &Map<String, Value>) -> PathBuf {
    lane_utils::parse_flag(argv, "history-path", false)
        .or_else(|| {
            payload
                .get("history_path")
                .and_then(Value::as_str)
                .map(ToString::to_string)
        })
        .map(|raw| repo_path(root, &raw))
        .unwrap_or_else(|| root.join(DEFAULT_HISTORY_REL))
}

fn swarm_state_path(root: &Path, argv: &[String], payload: &Map<String, Value>) -> PathBuf {
    lane_utils::parse_flag(argv, "swarm-state-path", false)
        .or_else(|| {
            payload
                .get("swarm_state_path")
                .and_then(Value::as_str)
                .map(ToString::to_string)
        })
        .map(|raw| repo_path(root, &raw))
        .unwrap_or_else(|| root.join(DEFAULT_SWARM_STATE_REL))
}

fn default_state() -> Value {
    json!({
        "schema_version": "langchain_bridge_state_v1",
        "chains": {},
        "chain_runs": {},
        "middleware_hooks": {},
        "agent_runs": {},
        "memory_bridges": {},
        "memory_queries": {},
        "integrations": {},
        "prompt_routes": {},
        "structured_outputs": {},
        "traces": [],
        "checkpoints": {},
        "intakes": {},
        "last_receipt": null,
    })
}

fn ensure_state_shape(value: &mut Value) {
    if !value.is_object() {
        *value = default_state();
        return;
    }
    for key in [
        "chains",
        "chain_runs",
        "middleware_hooks",
        "agent_runs",
        "memory_bridges",
        "memory_queries",
        "integrations",
        "prompt_routes",
        "structured_outputs",
        "checkpoints",
        "intakes",
    ] {
        if !value.get(key).map(Value::is_object).unwrap_or(false) {
            value[key] = json!({});
        }
    }
    if !value.get("traces").map(Value::is_array).unwrap_or(false) {
        value["traces"] = json!([]);
    }
    if value
        .get("schema_version")
        .and_then(Value::as_str)
        .is_none()
    {
        value["schema_version"] = json!("langchain_bridge_state_v1");
    }
}

fn load_state(path: &Path) -> Value {
    let mut state = lane_utils::read_json(path).unwrap_or_else(default_state);
    ensure_state_shape(&mut state);
    state
}

fn save_state(path: &Path, state: &Value) -> Result<(), String> {
    lane_utils::write_json(path, state)
}

fn append_history(path: &Path, row: &Value) -> Result<(), String> {
    lane_utils::append_jsonl(path, row)
}

fn as_object_mut<'a>(value: &'a mut Value, key: &str) -> &'a mut Map<String, Value> {
    if !value.get(key).map(Value::is_object).unwrap_or(false) {
        value[key] = json!({});
    }
    value
        .get_mut(key)
        .and_then(Value::as_object_mut)
        .expect("object")
}

fn as_array_mut<'a>(value: &'a mut Value, key: &str) -> &'a mut Vec<Value> {
    if !value.get(key).map(Value::is_array).unwrap_or(false) {
        value[key] = json!([]);
    }
    value
        .get_mut(key)
        .and_then(Value::as_array_mut)
        .expect("array")
}

fn now_millis() -> u128 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|row| row.as_millis())
        .unwrap_or(0)
}

fn to_base36(mut value: u128) -> String {
    if value == 0 {
        return "0".to_string();
    }
    let mut out = Vec::new();
    while value > 0 {
        let digit = (value % 36) as u8;
        out.push(if digit < 10 {
            (b'0' + digit) as char
        } else {
            (b'a' + (digit - 10)) as char
        });
        value /= 36;
    }
    out.iter().rev().collect()
}

fn stable_id(prefix: &str, basis: &Value) -> String {
    let digest = deterministic_receipt_hash(basis);
    format!("{prefix}_{}_{}", to_base36(now_millis()), &digest[..12])
}

fn clean_text(raw: Option<&str>, max_len: usize) -> String {
    raw.unwrap_or_default()
        .chars()
        .map(|ch| if ch.is_control() { ' ' } else { ch })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(max_len)
        .collect()
}

fn clean_token(raw: Option<&str>, fallback: &str) -> String {
    let value = clean_text(raw, 96);
    if value.is_empty() {
        fallback.to_string()
    } else {
        value
    }
}

fn parse_u64_value(value: Option<&Value>, fallback: u64, min: u64, max: u64) -> u64 {
    value
        .and_then(|row| row.as_u64())
        .unwrap_or(fallback)
        .clamp(min, max)
}

fn parse_bool_value(value: Option<&Value>, fallback: bool) -> bool {
    value.and_then(Value::as_bool).unwrap_or(fallback)
}

fn safe_prefix_for_bridge(path: &str) -> bool {
    path.starts_with("adapters/")
}

fn safe_shell_prefix(path: &str) -> bool {
    path.starts_with("client/") || path.starts_with("apps/")
}

fn normalize_bridge_path(root: &Path, raw: &str) -> Result<String, String> {
    let cleaned = clean_text(Some(raw), 240);
    if cleaned.is_empty() {
        return Err("langchain_bridge_path_required".to_string());
    }
    if !safe_prefix_for_bridge(&cleaned) {
        return Err("langchain_bridge_path_must_be_adapter_owned".to_string());
    }
    let full = repo_path(root, &cleaned);
    if !full.starts_with(root.join("adapters")) {
        return Err("langchain_bridge_path_escapes_adapters".to_string());
    }
    Ok(cleaned)
}

fn normalize_shell_path(root: &Path, raw: &str) -> Result<String, String> {
    let cleaned = clean_text(Some(raw), 240);
    if cleaned.is_empty() {
        return Err("langchain_shell_path_required".to_string());
    }
    if !safe_shell_prefix(&cleaned) {
        return Err("langchain_shell_path_must_live_under_client_or_apps".to_string());
    }
    let full = repo_path(root, &cleaned);
    if !(full.starts_with(root.join("client")) || full.starts_with(root.join("apps"))) {
        return Err("langchain_shell_path_escapes_workspace".to_string());
    }
    Ok(cleaned)
}

fn default_claim_evidence(id: &str, claim: &str) -> Value {
    json!([{ "id": id, "claim": claim }])
}

fn langchain_claim(id: &str) -> &'static str {
    match id {
        "V6-WORKFLOW-014.1" => {
            "langchain_lcel_and_runnable_chains_register_and_execute_as_governed_workflows"
        }
        "V6-WORKFLOW-014.2" => "langchain_legacy_and_deep_agents_execute_through_swarm_authority",
        "V6-WORKFLOW-014.3" => {
            "langchain_retrieval_and_memory_abstractions_normalize_to_governed_memory_runtime"
        }
        "V6-WORKFLOW-014.4" => "langchain_integrations_ingest_through_one_governed_gateway",
        "V6-WORKFLOW-014.5" => {
            "langchain_model_routing_and_prompt_templates_are_deterministic_and_fail_closed"
        }
        "V6-WORKFLOW-014.6" => "langchain_traces_and_eval_events_fold_into_native_observability",
        "V6-WORKFLOW-014.7" => {
            "langchain_stateful_runs_checkpoint_and_replay_through_authoritative_workflow_lanes"
        }
        "V6-WORKFLOW-014.8" => {
            "langchain_structured_output_parsing_and_schema_validation_remain_fail_closed"
        }
        "V6-WORKFLOW-014.9" => {
            "langchain_middleware_hooks_register_and_apply_with_receipted_workflow_visibility"
        }
        _ => "langchain_bridge_claim",
    }
}

fn read_swarm_state(path: &Path) -> Value {
    lane_utils::read_json(path).unwrap_or_else(|| json!({ "sessions": {}, "handoff_registry": {} }))
}

fn find_swarm_session_id_by_task(state: &Value, task: &str) -> Option<String> {
    state
        .get("sessions")
        .and_then(Value::as_object)
        .and_then(|rows| {
            rows.iter().find_map(|(session_id, row)| {
                let row_task = row.get("task").and_then(Value::as_str);
                let report_task = row
                    .get("report")
                    .and_then(|value| value.get("task"))
                    .and_then(Value::as_str);
                (row_task == Some(task) || report_task == Some(task)).then(|| session_id.clone())
            })
        })
}

fn ensure_session_for_task(
    root: &Path,
    swarm_state_path: &Path,
    task: &str,
    label: &str,
    role: Option<&str>,
    parent_session_id: Option<&str>,
    max_tokens: u64,
) -> Result<String, String> {
    let mut args = vec![
        "spawn".to_string(),
        format!("--task={task}"),
        format!("--agent-label={label}"),
        format!("--max-tokens={max_tokens}"),
        format!("--state-path={}", swarm_state_path.display()),
    ];
    if let Some(role) = role {
        args.push(format!("--role={role}"));
    }
    if let Some(parent) = parent_session_id {
        args.push(format!("--session-id={parent}"));
    }
    let exit = crate::swarm_runtime::run(root, &args);
    if exit != 0 {
        return Err(format!("langchain_swarm_spawn_failed:{label}"));
    }
    let swarm_state = read_swarm_state(swarm_state_path);
    find_swarm_session_id_by_task(&swarm_state, task)
        .ok_or_else(|| format!("langchain_swarm_session_missing:{label}"))
}

fn emit_native_trace(
    root: &Path,
    trace_id: &str,
    intent: &str,
    message: &str,
) -> Result<(), String> {
    let enable_exit = crate::observability_plane::run(
        root,
        &[
            "acp-provenance".to_string(),
            "--op=enable".to_string(),
            "--enabled=1".to_string(),
            "--visibility-mode=meta".to_string(),
            "--strict=1".to_string(),
        ],
    );
    if enable_exit != 0 {
        return Err("langchain_observability_enable_failed".to_string());
    }
    let exit = crate::observability_plane::run(
        root,
        &[
            "acp-provenance".to_string(),
            "--op=trace".to_string(),
            "--source-agent=langchain-bridge".to_string(),
            format!("--target-agent={}", clean_token(Some(intent), "workflow")),
            format!("--intent={}", clean_text(Some(intent), 80)),
            format!("--message={}", clean_text(Some(message), 160)),
            format!("--trace-id={trace_id}"),
            "--visibility-mode=meta".to_string(),
            "--strict=1".to_string(),
        ],
    );
    if exit != 0 {
        return Err("langchain_observability_trace_failed".to_string());
    }
    Ok(())
}

fn doc_token_set(doc: &Value) -> BTreeSet<String> {
    clean_text(doc.get("text").and_then(Value::as_str), 4096)
        .to_ascii_lowercase()
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|row| !row.is_empty())
        .map(ToString::to_string)
        .collect()
}
