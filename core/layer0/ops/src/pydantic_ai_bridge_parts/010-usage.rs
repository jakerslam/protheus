// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use serde_json::{json, Map, Value};
use std::path::{Path, PathBuf};

use crate::contract_lane_utils as lane_utils;
use crate::{deterministic_receipt_hash, now_iso};

const DEFAULT_STATE_REL: &str = "local/state/ops/pydantic_ai_bridge/latest.json";
const DEFAULT_HISTORY_REL: &str = "local/state/ops/pydantic_ai_bridge/history.jsonl";
const DEFAULT_SWARM_STATE_REL: &str = "local/state/ops/pydantic_ai_bridge/swarm_state.json";
const DEFAULT_APPROVAL_QUEUE_REL: &str = "client/runtime/local/state/pydantic_ai_approvals.json";

fn usage() {
    println!("pydantic-ai-bridge commands:");
    println!("  protheus-ops pydantic-ai-bridge status [--state-path=<path>]");
    println!("  protheus-ops pydantic-ai-bridge register-agent [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops pydantic-ai-bridge validate-output [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops pydantic-ai-bridge register-tool-context [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops pydantic-ai-bridge invoke-tool-context [--payload-base64=<json>] [--state-path=<path>] [--approval-queue-path=<path>]");
    println!("  protheus-ops pydantic-ai-bridge bridge-protocol [--payload-base64=<json>] [--state-path=<path>] [--swarm-state-path=<path>]");
    println!("  protheus-ops pydantic-ai-bridge durable-run [--payload-base64=<json>] [--state-path=<path>] [--swarm-state-path=<path>]");
    println!("  protheus-ops pydantic-ai-bridge approval-checkpoint [--payload-base64=<json>] [--state-path=<path>] [--approval-queue-path=<path>]");
    println!("  protheus-ops pydantic-ai-bridge record-logfire [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops pydantic-ai-bridge execute-graph [--payload-base64=<json>] [--state-path=<path>] [--swarm-state-path=<path>]");
    println!("  protheus-ops pydantic-ai-bridge stream-model [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops pydantic-ai-bridge record-eval [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops pydantic-ai-bridge assimilate-intake [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops pydantic-ai-bridge run-governed-workflow [--payload-base64=<json>] [--state-path=<path>]");
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
    lane_utils::payload_json(argv, "pydantic_ai_bridge")
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
        "schema_version": "pydantic_ai_bridge_state_v1",
        "a2a_agents": {},
        "typed_agents": {},
        "structured_validations": {},
        "llm_agents": {},
        "tool_manifests": {},
        "protocol_events": {},
        "hierarchies": {},
        "approval_records": {},
        "session_snapshots": {},
        "evaluations": {},
        "durable_runs": {},
        "logfire_events": {},
        "graph_runs": {},
        "model_streams": {},
        "sandbox_runs": {},
        "deployments": {},
        "runtime_bridges": {},
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
        "a2a_agents",
        "typed_agents",
        "structured_validations",
        "llm_agents",
        "tool_manifests",
        "protocol_events",
        "hierarchies",
        "approval_records",
        "session_snapshots",
        "evaluations",
        "durable_runs",
        "logfire_events",
        "graph_runs",
        "model_streams",
        "sandbox_runs",
        "deployments",
        "runtime_bridges",
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
        value["schema_version"] = json!("pydantic_ai_bridge_state_v1");
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
        "pydantic_ai_bridge_path_required",
        "pydantic_ai_unsafe_bridge_path_parent_reference",
        "pydantic_ai_unsupported_bridge_path",
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
        "pydantic_ai_shell_path_required",
        "pydantic_ai_shell_path_parent_reference",
        "pydantic_ai_shell_path_outside_client_or_apps",
        &["client/", "apps/"],
    )
}

fn encode_json_arg(value: &Value) -> Result<String, String> {
    serde_json::to_string(value).map_err(|err| format!("pydantic_ai_json_encode_failed:{err}"))
}

fn default_claim_evidence(id: &str, claim: &str) -> Value {
    json!([{ "id": id, "claim": claim }])
}

fn pydantic_claim(id: &str) -> &'static str {
    match id {
        "V6-WORKFLOW-015.1" => {
            "pydantic_ai_typed_agents_register_over_authoritative_workflow_and_swarm_lanes"
        }
        "V6-WORKFLOW-015.2" => {
            "pydantic_ai_structured_outputs_validate_retry_and_reject_with_deterministic_receipts"
        }
        "V6-WORKFLOW-015.3" => {
            "pydantic_ai_tool_contexts_and_dependency_injection_stay_governed_and_fail_closed"
        }
        "V6-WORKFLOW-015.4" => {
            "pydantic_ai_protocol_flows_normalize_onto_existing_swarm_session_and_adapter_lanes"
        }
        "V6-WORKFLOW-015.5" => {
            "pydantic_ai_durable_runs_resume_and_retry_through_authoritative_checkpoint_lanes"
        }
        "V6-WORKFLOW-015.6" => {
            "pydantic_ai_hitl_approvals_reuse_existing_approval_gate_with_deterministic_receipts"
        }
        "V6-WORKFLOW-015.7" => "pydantic_ai_logfire_and_otel_events_fold_into_native_observability",
        "V6-WORKFLOW-015.8" => {
            "pydantic_ai_graph_execution_normalizes_to_authoritative_workflow_lineage"
        }
        "V6-WORKFLOW-015.9" => {
            "pydantic_ai_model_agnostic_routing_and_structured_streaming_remain_profile_safe"
        }
        "V6-WORKFLOW-015.10" => {
            "pydantic_ai_eval_artifacts_remain_replayable_provenance_linked_and_native"
        }
        "V6-WORKFLOW-015.11" => {
            "pydantic_ai_frontend_adapter_execution_routes_through_tooling_claims_and_unified_memory_authority"
        }
        _ => "pydantic_ai_bridge_claim",
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
