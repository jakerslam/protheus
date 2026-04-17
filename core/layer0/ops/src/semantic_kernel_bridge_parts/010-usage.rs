// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

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
    println!("  protheus-ops semantic-kernel-bridge run-governed-workflow [--payload-base64=<json>] [--state-path=<path>]");
}

fn cli_receipt(kind: &str, payload: Value) -> Value {
    crate::contract_lane_utils::cli_receipt(kind, payload)
}

fn cli_error(kind: &str, error: &str) -> Value {
    crate::contract_lane_utils::cli_error(kind, error)
}

fn print_json_line(value: &Value) {
    crate::contract_lane_utils::print_json_line(value);
}

fn payload_json(argv: &[String]) -> Result<Value, String> {
    lane_utils::payload_json(argv, "semantic_kernel_bridge")
}

fn payload_obj<'a>(value: &'a Value) -> &'a Map<String, Value> {
    lane_utils::payload_obj(value)
}

fn is_plain_object(value: &Value) -> bool {
    value.is_object()
}

fn repo_path(root: &Path, rel: &str) -> PathBuf {
    lane_utils::repo_path(root, rel)
}

fn state_path(root: &Path, argv: &[String], payload: &Map<String, Value>) -> PathBuf {
    lane_utils::path_flag(root, argv, payload, "state-path", "state_path", DEFAULT_STATE_REL)
}

fn history_path(root: &Path, argv: &[String], payload: &Map<String, Value>) -> PathBuf {
    lane_utils::path_flag(
        root,
        argv,
        payload,
        "history-path",
        "history_path",
        DEFAULT_HISTORY_REL,
    )
}

fn semantic_swarm_state_path(
    root: &Path,
    argv: &[String],
    payload: &Map<String, Value>,
) -> PathBuf {
    lane_utils::path_flag(
        root,
        argv,
        payload,
        "swarm-state-path",
        "swarm_state_path",
        DEFAULT_SWARM_STATE_REL,
    )
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
        "governed_workflows": {},
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
        "governed_workflows",
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

fn stable_id(prefix: &str, basis: &Value) -> String {
    lane_utils::stable_id(prefix, basis)
}

fn clean_text(raw: Option<&str>, max_len: usize) -> String {
    crate::contract_lane_utils::clean_text(raw, max_len)
}

fn clean_token(raw: Option<&str>, fallback: &str) -> String {
    lane_utils::clean_token(raw, fallback)
}

fn rel(root: &Path, path: &Path) -> String {
    lane_utils::rel_path(root, path)
}

fn parse_u64_value(value: Option<&Value>, fallback: u64, min: u64, max: u64) -> u64 {
    lane_utils::json_u64_coerce(value, fallback, min, max)
}

fn parse_f64_value(value: Option<&Value>, fallback: f64, min: f64, max: f64) -> f64 {
    lane_utils::json_f64_coerce(value, fallback, min, max)
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

fn normalize_bridge_path(root: &Path, raw: &str) -> Result<String, String> {
    lane_utils::normalize_prefixed_path(
        root,
        raw,
        "bridge_path_required",
        "unsafe_bridge_path_parent_reference",
        "unsupported_bridge_path",
        &[
            "adapters/",
            "client/runtime/systems/",
            "client/runtime/lib/",
            "client/lib/",
            "planes/contracts/",
        ],
    )
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
    lane_utils::find_swarm_session_id_by_task(state, task)
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
