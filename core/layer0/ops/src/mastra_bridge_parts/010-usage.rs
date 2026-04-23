// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use serde_json::{json, Map, Value};
use std::path::{Path, PathBuf};

use crate::contract_lane_utils as lane_utils;
use crate::now_iso;

const DEFAULT_STATE_REL: &str = "local/state/ops/mastra_bridge/latest.json";
const DEFAULT_HISTORY_REL: &str = "local/state/ops/mastra_bridge/history.jsonl";
const DEFAULT_SWARM_STATE_REL: &str = "local/state/ops/mastra_bridge/swarm_state.json";
const DEFAULT_APPROVAL_QUEUE_REL: &str = "client/runtime/local/state/mastra_approvals.yaml";

fn usage() {
    println!("mastra-bridge commands:");
    println!("  infring-ops mastra-bridge status [--state-path=<path>]");
    println!("  infring-ops mastra-bridge register-graph [--payload-base64=<json>] [--state-path=<path>]");
    println!("  infring-ops mastra-bridge execute-graph [--payload-base64=<json>] [--state-path=<path>] [--swarm-state-path=<path>]");
    println!("  infring-ops mastra-bridge run-agent-loop [--payload-base64=<json>] [--state-path=<path>] [--swarm-state-path=<path>]");
    println!("  infring-ops mastra-bridge memory-recall [--payload-base64=<json>] [--state-path=<path>]");
    println!("  infring-ops mastra-bridge suspend-run [--payload-base64=<json>] [--state-path=<path>] [--approval-queue-path=<path>]");
    println!("  infring-ops mastra-bridge resume-run [--payload-base64=<json>] [--state-path=<path>] [--swarm-state-path=<path>] [--approval-queue-path=<path>]");
    println!("  infring-ops mastra-bridge register-mcp-bridge [--payload-base64=<json>] [--state-path=<path>]");
    println!("  infring-ops mastra-bridge invoke-mcp-bridge [--payload-base64=<json>] [--state-path=<path>] [--approval-queue-path=<path>]");
    println!("  infring-ops mastra-bridge record-eval-trace [--payload-base64=<json>] [--state-path=<path>]");
    println!(
        "  infring-ops mastra-bridge deploy-shell [--payload-base64=<json>] [--state-path=<path>]"
    );
    println!("  infring-ops mastra-bridge register-runtime-bridge [--payload-base64=<json>] [--state-path=<path>]");
    println!(
        "  infring-ops mastra-bridge route-model [--payload-base64=<json>] [--state-path=<path>]"
    );
    println!("  infring-ops mastra-bridge scaffold-intake [--payload-base64=<json>] [--state-path=<path>]");
    println!("  infring-ops mastra-bridge run-governed-workflow [--payload-base64=<json>] [--state-path=<path>]");
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
    lane_utils::payload_json(argv, "mastra_bridge")
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
        "schema_version": "mastra_bridge_state_v1",
        "graphs": {},
        "graph_runs": {},
        "agent_loops": {},
        "memory_recalls": {},
        "suspended_runs": {},
        "mcp_bridges": {},
        "run_snapshots": {},
        "eval_traces": {},
        "deployments": {},
        "runtime_bridges": {},
        "intakes": {},
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
        "graphs",
        "graph_runs",
        "agent_loops",
        "memory_recalls",
        "suspended_runs",
        "mcp_bridges",
        "run_snapshots",
        "eval_traces",
        "deployments",
        "runtime_bridges",
        "intakes",
        "governed_workflows",
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
        value["schema_version"] = json!("mastra_bridge_state_v1");
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
        "mastra_bridge_path_required",
        "mastra_unsafe_bridge_path_parent_reference",
        "mastra_unsupported_bridge_path",
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
        "mastra_shell_path_required",
        "mastra_shell_path_parent_reference",
        "mastra_shell_path_outside_client_or_apps",
        &["client/", "apps/"],
    )
}

fn encode_json_arg(value: &Value) -> Result<String, String> {
    serde_json::to_string(value).map_err(|err| format!("mastra_json_encode_failed:{err}"))
}

fn default_claim_evidence(id: &str, claim: &str) -> Value {
    json!([{ "id": id, "claim": claim }])
}

fn mastra_claim(id: &str) -> &'static str {
    match id {
        "V6-WORKFLOW-011.1" => "mastra_graph_workflows_register_and_execute_as_receipted_chain_branch_and_parallel_runs_over_authoritative_workflow_and_swarm_lanes",
        "V6-WORKFLOW-011.2" => "mastra_agent_tool_reasoning_reuses_authoritative_swarm_budgets_sessions_and_route_receipts",
        "V6-WORKFLOW-011.3" => "mastra_memory_recall_routes_through_existing_memory_runtime_and_budget_enforcement_with_profile_safe_degradation",
        "V6-WORKFLOW-011.4" => "mastra_suspend_resume_reuses_existing_receipt_backed_state_and_approval_gate_semantics",
        "V6-WORKFLOW-011.5" => "mastra_mcp_interoperability_is_adapter_owned_fail_closed_and_deterministically_receipted",
        "V6-WORKFLOW-011.6" => "mastra_evals_and_traces_emit_native_observability_receipts_without_a_parallel_telemetry_stack",
        "V6-WORKFLOW-011.7" => "mastra_multi_provider_model_routing_remains_adapter_owned_receipted_and_profile_safe",
        "V6-WORKFLOW-011.8" => "mastra_studio_and_full_stack_shells_remain_non_authoritative_and_delegate_back_to_core_receipts",
        "V6-WORKFLOW-011.9" => "mastra_ts_first_intake_scaffolds_thin_templates_without_forcing_node_dependency_into_sovereign_profiles",
        "V6-WORKFLOW-011.10" => "mastra_frontend_adapter_execution_routes_through_tooling_claims_and_unified_memory_authority",
        _ => "mastra_bridge_claim",
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
