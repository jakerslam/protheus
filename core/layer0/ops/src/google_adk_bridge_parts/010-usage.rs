// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use serde_json::{json, Map, Value};
use std::path::{Path, PathBuf};

use crate::contract_lane_utils as lane_utils;
use crate::now_iso;

const DEFAULT_STATE_REL: &str = "local/state/ops/google_adk_bridge/latest.json";
const DEFAULT_HISTORY_REL: &str = "local/state/ops/google_adk_bridge/history.jsonl";
const DEFAULT_SWARM_STATE_REL: &str = "local/state/ops/google_adk_bridge/swarm_state.json";
const DEFAULT_APPROVAL_QUEUE_REL: &str = "client/runtime/local/state/google_adk_approvals.yaml";

fn usage() {
    println!("google-adk-bridge commands:");
    println!("  infring-ops google-adk-bridge status [--state-path=<path>]");
    println!("  infring-ops google-adk-bridge register-a2a-agent [--payload-base64=<json>] [--state-path=<path>]");
    println!("  infring-ops google-adk-bridge send-a2a-message [--payload-base64=<json>] [--state-path=<path>] [--swarm-state-path=<path>]");
    println!("  infring-ops google-adk-bridge run-llm-agent [--payload-base64=<json>] [--state-path=<path>] [--swarm-state-path=<path>]");
    println!("  infring-ops google-adk-bridge register-tool-manifest [--payload-base64=<json>] [--state-path=<path>]");
    println!("  infring-ops google-adk-bridge invoke-tool-manifest [--payload-base64=<json>] [--state-path=<path>] [--approval-queue-path=<path>]");
    println!("  infring-ops google-adk-bridge coordinate-hierarchy [--payload-base64=<json>] [--state-path=<path>] [--swarm-state-path=<path>]");
    println!("  infring-ops google-adk-bridge approval-checkpoint [--payload-base64=<json>] [--state-path=<path>] [--approval-queue-path=<path>]");
    println!("  infring-ops google-adk-bridge rewind-session [--payload-base64=<json>] [--state-path=<path>] [--swarm-state-path=<path>]");
    println!("  infring-ops google-adk-bridge record-evaluation [--payload-base64=<json>] [--state-path=<path>]");
    println!("  infring-ops google-adk-bridge sandbox-execute [--payload-base64=<json>] [--state-path=<path>]");
    println!("  infring-ops google-adk-bridge deploy-shell [--payload-base64=<json>] [--state-path=<path>]");
    println!("  infring-ops google-adk-bridge register-runtime-bridge [--payload-base64=<json>] [--state-path=<path>]");
    println!("  infring-ops google-adk-bridge route-model [--payload-base64=<json>] [--state-path=<path>]");
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
    lane_utils::payload_json(argv, "google_adk_bridge")
}

fn payload_obj<'a>(value: &'a Value) -> &'a Map<String, Value> {
    lane_utils::payload_obj(value)
}

fn repo_path(root: &Path, rel: &str) -> PathBuf {
    lane_utils::repo_path(root, rel)
}

fn rel(root: &Path, path: &Path) -> String {
    lane_utils::rel_path(root, path)
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

fn swarm_state_path(root: &Path, argv: &[String], payload: &Map<String, Value>) -> PathBuf {
    lane_utils::path_flag(
        root,
        argv,
        payload,
        "swarm-state-path",
        "swarm_state_path",
        DEFAULT_SWARM_STATE_REL,
    )
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

fn stable_id(prefix: &str, basis: &Value) -> String {
    lane_utils::stable_id(prefix, basis)
}

fn clean_text(raw: Option<&str>, max_len: usize) -> String {
    crate::contract_lane_utils::clean_text(raw, max_len)
}

fn clean_token(raw: Option<&str>, fallback: &str) -> String {
    lane_utils::clean_token(raw, fallback)
}

fn parse_u64_value(value: Option<&Value>, fallback: u64, min: u64, max: u64) -> u64 {
    lane_utils::json_u64_coerce(value, fallback, min, max)
}

fn parse_f64_value(value: Option<&Value>, fallback: f64, min: f64, max: f64) -> f64 {
    lane_utils::json_f64_coerce(value, fallback, min, max)
}

fn parse_bool_value(value: Option<&Value>, fallback: bool) -> bool {
    lane_utils::json_bool_coerce(value, fallback)
}

fn normalize_bridge_path(root: &Path, raw: &str) -> Result<String, String> {
    lane_utils::normalize_prefixed_path(
        root,
        raw,
        "google_adk_bridge_path_required",
        "google_adk_unsafe_bridge_path_parent_reference",
        "google_adk_unsupported_bridge_path",
        &[
            "adapters/",
            "client/runtime/systems/",
            "client/runtime/lib/",
            "client/lib/",
            "planes/contracts/",
        ],
    )
}

fn normalize_shell_path(root: &Path, raw: &str) -> Result<String, String> {
    lane_utils::normalize_prefixed_path(
        root,
        raw,
        "google_adk_shell_path_required",
        "google_adk_shell_path_parent_reference",
        "google_adk_shell_path_outside_client_or_apps",
        &["client/", "apps/"],
    )
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
    lane_utils::find_swarm_session_id_by_task(state, task)
}

fn parse_string_list(value: Option<&Value>) -> Vec<String> {
    lane_utils::json_string_list(value)
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
