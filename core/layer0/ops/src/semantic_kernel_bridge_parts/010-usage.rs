// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use crate::contract_lane_utils as lane_utils;
use crate::{deterministic_receipt_hash, now_iso};

const DEFAULT_STATE_REL: &str = "local/state/ops/semantic_kernel_bridge/latest.json";
const DEFAULT_HISTORY_REL: &str = "local/state/ops/semantic_kernel_bridge/history.jsonl";
const DEFAULT_SWARM_STATE_REL: &str = "local/state/ops/semantic_kernel_bridge/swarm_state.json";

fn usage() {
    println!("semantic-kernel-bridge commands:");
    println!("  protheus-ops semantic-kernel-bridge status [--state-path=<path>]");
    println!("  protheus-ops semantic-kernel-bridge register-service [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops semantic-kernel-bridge register-plugin [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops semantic-kernel-bridge invoke-plugin [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops semantic-kernel-bridge collaborate [--payload-base64=<json>] [--state-path=<path>] [--swarm-state-path=<path>]");
    println!("  protheus-ops semantic-kernel-bridge plan [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops semantic-kernel-bridge register-vector-connector [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops semantic-kernel-bridge retrieve [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops semantic-kernel-bridge register-llm-connector [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops semantic-kernel-bridge route-llm [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops semantic-kernel-bridge validate-structured-output [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops semantic-kernel-bridge emit-enterprise-event [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops semantic-kernel-bridge register-dotnet-bridge [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops semantic-kernel-bridge invoke-dotnet-bridge [--payload-base64=<json>] [--state-path=<path>]");
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
            .map_err(|err| format!("semantic_kernel_bridge_payload_decode_failed:{err}"));
    }
    if let Some(raw_b64) = lane_utils::parse_flag(argv, "payload-base64", false) {
        let bytes = BASE64_STANDARD
            .decode(raw_b64.as_bytes())
            .map_err(|err| format!("semantic_kernel_bridge_payload_base64_decode_failed:{err}"))?;
        let text = String::from_utf8(bytes)
            .map_err(|err| format!("semantic_kernel_bridge_payload_utf8_decode_failed:{err}"))?;
        return serde_json::from_str::<Value>(&text)
            .map_err(|err| format!("semantic_kernel_bridge_payload_decode_failed:{err}"));
    }
    Ok(json!({}))
}

fn payload_obj<'a>(value: &'a Value) -> &'a Map<String, Value> {
    value.as_object().unwrap_or_else(|| {
        static EMPTY: OnceLock<Map<String, Value>> = OnceLock::new();
        EMPTY.get_or_init(Map::new)
    })
}

fn is_plain_object(value: &Value) -> bool {
    value.is_object()
}

fn repo_path(root: &Path, rel: &str) -> PathBuf {
    let trimmed = rel.trim();
    let candidate = PathBuf::from(trimmed);
    if candidate.is_absolute() {
        candidate
    } else {
        root.join(candidate)
    }
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

fn semantic_swarm_state_path(
    root: &Path,
    argv: &[String],
    payload: &Map<String, Value>,
) -> PathBuf {
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
        "schema_version": "semantic_kernel_bridge_state_v1",
        "services": {},
        "plugins": {},
        "collaborations": {},
        "plans": {},
        "vector_connectors": {},
        "llm_connectors": {},
        "structured_processes": {},
        "enterprise_events": [],
        "dotnet_bridges": {},
        "last_receipt": null,
    })
}

fn ensure_state_shape(value: &mut Value) {
    if !value.is_object() {
        *value = default_state();
        return;
    }
    for key in [
        "services",
        "plugins",
        "collaborations",
        "plans",
        "vector_connectors",
        "llm_connectors",
        "structured_processes",
        "dotnet_bridges",
    ] {
        if !value.get(key).map(is_plain_object).unwrap_or(false) {
            value[key] = json!({});
        }
    }
    if !value
        .get("enterprise_events")
        .map(Value::is_array)
        .unwrap_or(false)
    {
        value["enterprise_events"] = json!([]);
    }
    if value
        .get("schema_version")
        .and_then(Value::as_str)
        .is_none()
    {
        value["schema_version"] = json!("semantic_kernel_bridge_state_v1");
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
            (b'a' + digit - 10) as char
        });
        value /= 36;
    }
    out.iter().rev().collect()
}

fn stable_id(prefix: &str, basis: &Value) -> String {
    let basis_hash = deterministic_receipt_hash(basis);
    format!(
        "{}_{}_{}",
        prefix,
        to_base36(now_millis()),
        &basis_hash[..12]
    )
}

fn clean_text(raw: Option<&str>, max_len: usize) -> String {
    lane_utils::clean_text(raw, max_len)
}

fn clean_token(raw: Option<&str>, fallback: &str) -> String {
    lane_utils::clean_token(raw, fallback)
}

fn rel(root: &Path, path: &Path) -> String {
    lane_utils::rel_path(root, path)
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

fn normalized_profile(raw: &str) -> &'static str {
    match raw.trim().to_ascii_lowercase().as_str() {
        "tiny" | "tiny-max" | "embedded" => "tiny-max",
        "pure" => "pure",
        _ => "rich",
    }
}

fn approx_token_count(text: &str) -> u64 {
    let words = text.split_whitespace().count() as u64;
    let chars = text.chars().count() as u64;
    words.max(chars / 4).max(1)
}

fn has_token(input: &str, token: &str) -> bool {
    input
        .to_ascii_lowercase()
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .any(|part| !part.is_empty() && part == token)
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

fn normalize_bridge_path(root: &Path, raw: &str) -> Result<String, String> {
    let candidate = raw.trim();
    if candidate.is_empty() {
        return Err("bridge_path_required".to_string());
    }
    if candidate.contains("..") {
        return Err("unsafe_bridge_path_parent_reference".to_string());
    }
    let abs = repo_path(root, candidate);
    let rel_path = rel(root, &abs);
    if !safe_prefix_for_bridge(&rel_path) {
        return Err("unsupported_bridge_path".to_string());
    }
    Ok(rel_path)
}

fn default_claim_evidence(id: &str, claim: &str) -> Value {
    json!([{ "id": id, "claim": claim }])
}

fn read_swarm_state(path: &Path) -> Value {
    lane_utils::read_json(path).unwrap_or_else(
        || json!({ "sessions": {}, "handoff_registry": {}, "network_registry": {} }),
    )
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

fn find_swarm_network_id_by_name(state: &Value, name: &str) -> Option<String> {
    state
        .get("network_registry")
        .and_then(Value::as_object)
        .and_then(|rows| {
            rows.iter().find_map(|(network_id, row)| {
                (row.get("name").and_then(Value::as_str) == Some(name)).then(|| network_id.clone())
            })
        })
}

fn find_swarm_network_by_name(state: &Value, name: &str) -> Option<Value> {
    state
        .get("network_registry")
        .and_then(Value::as_object)
        .and_then(|rows| {
            rows.values()
                .find(|row| row.get("name").and_then(Value::as_str) == Some(name))
                .cloned()
        })
}

fn encode_json_arg(value: &Value) -> Result<String, String> {
    serde_json::to_string(value).map_err(|err| format!("json_encode_failed:{err}"))
}

fn semantic_claim(id: &str) -> &'static str {
    match id {
        "V6-WORKFLOW-008.1" => "kernel_service_registration_is_receipted_over_one_governed_orchestration_surface",
        "V6-WORKFLOW-008.2" => "plugin_assets_normalize_into_governed_manifests_with_fail_closed_invocation",
        "V6-WORKFLOW-008.3" => "semantic_kernel_style_agent_collaboration_reuses_authoritative_swarm_sessions_and_handoffs",
        "V6-WORKFLOW-008.4" => "planner_semantics_compile_into_deterministic_function_selection_receipts",
        "V6-WORKFLOW-008.5" => "vector_connector_retrieval_enforces_context_budget_and_explicit_profile_degradation",
        "V6-WORKFLOW-008.6" => "llm_connector_routes_and_multimodal_paths_are_policy_gated_and_receipted",
        "V6-WORKFLOW-008.7" => "structured_output_and_process_graphs_are_schema_validated_and_receipted",
        "V6-WORKFLOW-008.8" => "enterprise_observability_and_azure_events_emit_native_receipts_without_side_telemetry_stack",
        "V6-WORKFLOW-008.9" => "dotnet_parity_flows_route_through_governed_bridge_receipts",
        _ => "semantic_kernel_bridge_action_emits_deterministic_receipt",
    }
}

fn allowed_service_surface(surface: &str) -> bool {
    matches!(
        surface,
        "workflow-executor"
            | "workflow-controller"
            | "swarm-runtime"
            | "mcp-plane"
            | "policy-runtime-kernel"
    )
}

