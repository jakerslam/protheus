// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use serde_json::{json, Map, Value};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use crate::contract_lane_utils as lane_utils;
use crate::{deterministic_receipt_hash, now_iso};

const DEFAULT_STATE_REL: &str = "local/state/ops/google_adk_bridge/latest.json";
const DEFAULT_HISTORY_REL: &str = "local/state/ops/google_adk_bridge/history.jsonl";
const DEFAULT_SWARM_STATE_REL: &str = "local/state/ops/google_adk_bridge/swarm_state.json";
const DEFAULT_APPROVAL_QUEUE_REL: &str = "client/runtime/local/state/google_adk_approvals.yaml";

fn usage() {
    println!("google-adk-bridge commands:");
    println!("  protheus-ops google-adk-bridge status [--state-path=<path>]");
    println!("  protheus-ops google-adk-bridge register-a2a-agent [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops google-adk-bridge send-a2a-message [--payload-base64=<json>] [--state-path=<path>] [--swarm-state-path=<path>]");
    println!("  protheus-ops google-adk-bridge run-llm-agent [--payload-base64=<json>] [--state-path=<path>] [--swarm-state-path=<path>]");
    println!("  protheus-ops google-adk-bridge register-tool-manifest [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops google-adk-bridge invoke-tool-manifest [--payload-base64=<json>] [--state-path=<path>] [--approval-queue-path=<path>]");
    println!("  protheus-ops google-adk-bridge coordinate-hierarchy [--payload-base64=<json>] [--state-path=<path>] [--swarm-state-path=<path>]");
    println!("  protheus-ops google-adk-bridge approval-checkpoint [--payload-base64=<json>] [--state-path=<path>] [--approval-queue-path=<path>]");
    println!("  protheus-ops google-adk-bridge rewind-session [--payload-base64=<json>] [--state-path=<path>] [--swarm-state-path=<path>]");
    println!("  protheus-ops google-adk-bridge record-evaluation [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops google-adk-bridge sandbox-execute [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops google-adk-bridge deploy-shell [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops google-adk-bridge register-runtime-bridge [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops google-adk-bridge route-model [--payload-base64=<json>] [--state-path=<path>]");
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
            .map_err(|err| format!("google_adk_bridge_payload_decode_failed:{err}"));
    }
    if let Some(raw_b64) = lane_utils::parse_flag(argv, "payload-base64", false) {
        let bytes = BASE64_STANDARD
            .decode(raw_b64.as_bytes())
            .map_err(|err| format!("google_adk_bridge_payload_base64_decode_failed:{err}"))?;
        let text = String::from_utf8(bytes)
            .map_err(|err| format!("google_adk_bridge_payload_utf8_decode_failed:{err}"))?;
        return serde_json::from_str::<Value>(&text)
            .map_err(|err| format!("google_adk_bridge_payload_decode_failed:{err}"));
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
    lane_utils::rel_path(root, path)
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

fn approval_queue_path(root: &Path, argv: &[String], payload: &Map<String, Value>) -> PathBuf {
    lane_utils::parse_flag(argv, "approval-queue-path", false)
        .or_else(|| {
            payload
                .get("approval_queue_path")
                .and_then(Value::as_str)
                .map(ToString::to_string)
        })
        .map(|raw| repo_path(root, &raw))
        .unwrap_or_else(|| root.join(DEFAULT_APPROVAL_QUEUE_REL))
}

fn default_state() -> Value {
    json!({
        "schema_version": "google_adk_bridge_state_v1",
        "a2a_agents": {},
        "llm_agents": {},
        "tool_manifests": {},
        "hierarchies": {},
        "approval_records": {},
        "session_snapshots": {},
        "evaluations": {},
        "sandbox_runs": {},
        "deployments": {},
        "runtime_bridges": {},
        "last_receipt": null,
    })
}

fn ensure_state_shape(value: &mut Value) {
    if !value.is_object() {
        *value = default_state();
        return;
    }
    for key in [
        "a2a_agents",
        "llm_agents",
        "tool_manifests",
        "hierarchies",
        "approval_records",
        "session_snapshots",
        "evaluations",
        "sandbox_runs",
        "deployments",
        "runtime_bridges",
    ] {
        if !value.get(key).map(Value::is_object).unwrap_or(false) {
            value[key] = json!({});
        }
    }
    if value
        .get("schema_version")
        .and_then(Value::as_str)
        .is_none()
    {
        value["schema_version"] = json!("google_adk_bridge_state_v1");
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
            (b'a' + digit - 10) as char
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
    lane_utils::clean_text(raw, max_len)
}

fn clean_token(raw: Option<&str>, fallback: &str) -> String {
    lane_utils::clean_token(raw, fallback)
}

fn parse_u64_value(value: Option<&Value>, fallback: u64, min: u64, max: u64) -> u64 {
    value
        .and_then(|row| match row {
            Value::Number(n) => n.as_u64(),
            Value::String(s) => s.trim().parse::<u64>().ok(),
            _ => None,
        })
        .unwrap_or(fallback)
        .clamp(min, max)
}

fn parse_f64_value(value: Option<&Value>, fallback: f64, min: f64, max: f64) -> f64 {
    value
        .and_then(|row| match row {
            Value::Number(n) => n.as_f64(),
            Value::String(s) => s.trim().parse::<f64>().ok(),
            _ => None,
        })
        .unwrap_or(fallback)
        .clamp(min, max)
}

fn parse_bool_value(value: Option<&Value>, fallback: bool) -> bool {
    value
        .and_then(|row| match row {
            Value::Bool(v) => Some(*v),
            Value::String(s) => {
                let lower = s.trim().to_ascii_lowercase();
                match lower.as_str() {
                    "1" | "true" | "yes" | "on" => Some(true),
                    "0" | "false" | "no" | "off" => Some(false),
                    _ => None,
                }
            }
            _ => None,
        })
        .unwrap_or(fallback)
}

fn safe_prefix_for_bridge(path: &str) -> bool {
    [
        "adapters/",
        "client/runtime/systems/",
        "client/runtime/lib/",
        "client/lib/",
        "planes/contracts/",
    ]
    .iter()
    .any(|prefix| path.starts_with(prefix))
}

fn safe_shell_prefix(path: &str) -> bool {
    ["client/", "apps/"]
        .iter()
        .any(|prefix| path.starts_with(prefix))
}

fn normalize_bridge_path(root: &Path, raw: &str) -> Result<String, String> {
    let candidate = raw.trim();
    if candidate.is_empty() {
        return Err("google_adk_bridge_path_required".to_string());
    }
    if candidate.contains("..") {
        return Err("google_adk_unsafe_bridge_path_parent_reference".to_string());
    }
    let abs = repo_path(root, candidate);
    let rel_path = rel(root, &abs);
    if !safe_prefix_for_bridge(&rel_path) {
        return Err("google_adk_unsupported_bridge_path".to_string());
    }
    Ok(rel_path)
}

fn normalize_shell_path(root: &Path, raw: &str) -> Result<String, String> {
    let candidate = raw.trim();
    if candidate.is_empty() {
        return Err("google_adk_shell_path_required".to_string());
    }
    if candidate.contains("..") {
        return Err("google_adk_shell_path_parent_reference".to_string());
    }
    let abs = repo_path(root, candidate);
    let rel_path = rel(root, &abs);
    if !safe_shell_prefix(&rel_path) {
        return Err("google_adk_shell_path_outside_client_or_apps".to_string());
    }
    Ok(rel_path)
}

fn encode_json_arg(value: &Value) -> Result<String, String> {
    serde_json::to_string(value).map_err(|err| format!("google_adk_json_encode_failed:{err}"))
}

fn default_claim_evidence(id: &str, claim: &str) -> Value {
    json!([{ "id": id, "claim": claim }])
}

fn adk_claim(id: &str) -> &'static str {
    match id {
        "V6-WORKFLOW-010.1" => "google_adk_a2a_registry_routes_remote_agent_interop_through_governed_swarm_sessions_and_adapter_paths",
        "V6-WORKFLOW-010.2" => "google_adk_llmagent_and_workflow_semantics_execute_over_authoritative_workflow_and_budget_lanes",
        "V6-WORKFLOW-010.3" => "google_adk_tool_imports_and_invocations_normalize_into_governed_mcp_openapi_and_custom_tool_manifests",
        "V6-WORKFLOW-010.4" => "google_adk_hierarchical_coordination_reuses_authoritative_swarm_lineage_budgets_and_context_controls",
        "V6-WORKFLOW-010.5" => "google_adk_hitl_tool_approvals_reuse_existing_approval_gate_with_deterministic_decision_receipts",
        "V6-WORKFLOW-010.6" => "google_adk_session_rewind_and_evaluation_artifacts_restore_bounded_state_and_emit_native_observability_receipts",
        "V6-WORKFLOW-010.7" => "google_adk_sandbox_and_cloud_paths_stay_adapter_owned_policy_gated_and_fail_closed",
        "V6-WORKFLOW-010.8" => "google_adk_dev_shells_and_deployment_artifacts_remain_non_authoritative_and_delegate_to_core_bridge_receipts",
        "V6-WORKFLOW-010.9" => "google_adk_model_agnostic_polyglot_routing_remains_adapter_owned_and_profile_safe",
        _ => "google_adk_bridge_claim",
    }
}

fn read_swarm_state(path: &Path) -> Value {
    lane_utils::read_json(path)
        .unwrap_or_else(|| json!({ "sessions": {}, "handoff_registry": {}, "message_queues": {} }))
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

fn parse_string_list(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| row.as_str().map(|s| clean_token(Some(s), "")))
        .filter(|row| !row.is_empty())
        .collect()
}

fn profile_supported(supported_profiles: &[String], profile: &str) -> bool {
    supported_profiles.is_empty() || supported_profiles.iter().any(|row| row == profile)
}

fn read_yaml_value(path: &Path) -> Value {
    let raw = std::fs::read_to_string(path).unwrap_or_default();
    if raw.trim().is_empty() {
        return json!({});
    }
    serde_yaml::from_str::<Value>(&raw).unwrap_or_else(|_| json!({}))
}

fn approval_status_from_queue(queue_path: &Path, action_id: &str) -> String {
    let queue = read_yaml_value(queue_path);
    for (status, key) in [
        ("pending", "pending"),
        ("approved", "approved"),
        ("denied", "denied"),
    ] {
        if queue
            .get(key)
            .and_then(Value::as_array)
            .map(|rows| {
                rows.iter()
                    .any(|row| row.get("action_id").and_then(Value::as_str) == Some(action_id))
            })
            .unwrap_or(false)
        {
            return status.to_string();
        }
    }
    "unknown".to_string()
}

fn approval_is_approved(queue_path: &Path, action_id: &str) -> bool {
    approval_status_from_queue(queue_path, action_id) == "approved"
}

fn allowed_language(language: &str) -> bool {
    matches!(language, "python" | "ts" | "go" | "java" | "rust")
}

fn allowed_tool_kind(kind: &str) -> bool {
    matches!(kind, "native" | "mcp" | "openapi" | "custom")
}

fn allowed_workflow_mode(mode: &str) -> bool {
    matches!(mode, "sequential" | "parallel" | "loop")
}

