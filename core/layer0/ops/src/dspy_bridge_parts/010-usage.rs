// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use serde_json::{json, Map, Value};
use std::path::{Path, PathBuf};

use crate::contract_lane_utils as lane_utils;
use crate::{deterministic_receipt_hash, now_iso};

const DEFAULT_STATE_REL: &str = "local/state/ops/dspy_bridge/latest.json";
const DEFAULT_HISTORY_REL: &str = "local/state/ops/dspy_bridge/history.jsonl";
const DEFAULT_SWARM_STATE_REL: &str = "local/state/ops/dspy_bridge/swarm_state.json";

fn usage() {
    println!("dspy-bridge commands:");
    println!("  protheus-ops dspy-bridge status [--state-path=<path>]");
    println!("  protheus-ops dspy-bridge register-signature [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops dspy-bridge compile-program [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops dspy-bridge optimize-program [--payload-base64=<json>] [--state-path=<path>]");
    println!(
        "  protheus-ops dspy-bridge assert-program [--payload-base64=<json>] [--state-path=<path>]"
    );
    println!("  protheus-ops dspy-bridge import-integration [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops dspy-bridge execute-multihop [--payload-base64=<json>] [--state-path=<path>] [--swarm-state-path=<path>]");
    println!("  protheus-ops dspy-bridge record-benchmark [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops dspy-bridge record-optimization-trace [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops dspy-bridge assimilate-intake [--payload-base64=<json>] [--state-path=<path>]");
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
    lane_utils::payload_json(argv, "dspy_bridge")
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

fn default_state() -> Value {
    json!({
        "schema_version": "dspy_bridge_state_v1",
        "signatures": {},
        "compiled_programs": {},
        "optimization_runs": {},
        "assertion_runs": {},
        "integrations": {},
        "multihop_runs": {},
        "benchmarks": {},
        "optimization_traces": {},
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
        "signatures",
        "compiled_programs",
        "optimization_runs",
        "assertion_runs",
        "integrations",
        "multihop_runs",
        "benchmarks",
        "optimization_traces",
        "intakes",
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
        value["schema_version"] = json!("dspy_bridge_state_v1");
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

fn parse_string_list(value: Option<&Value>) -> Vec<String> {
    lane_utils::json_string_list(value)
}

fn normalize_bridge_path(root: &Path, raw: &str) -> Result<String, String> {
    lane_utils::normalize_prefixed_path(
        root,
        raw,
        "dspy_bridge_path_required",
        "dspy_unsafe_bridge_path_parent_reference",
        "dspy_unsupported_bridge_path",
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
        "dspy_shell_path_required",
        "dspy_shell_path_parent_reference",
        "dspy_shell_path_outside_client_or_apps",
        &["client/", "apps/"],
    )
}

fn encode_json_arg(value: &Value) -> Result<String, String> {
    serde_json::to_string(value).map_err(|err| format!("dspy_json_encode_failed:{err}"))
}

fn default_claim_evidence(id: &str, claim: &str) -> Value {
    json!([{ "id": id, "claim": claim }])
}

fn dspy_claim(id: &str) -> &'static str {
    match id {
        "V6-WORKFLOW-017.1" => "dspy_signatures_and_typed_predictors_register_over_authoritative_workflow_and_swarm_lanes",
        "V6-WORKFLOW-017.2" => "dspy_modules_and_compiler_runs_normalize_to_the_authoritative_workflow_engine",
        "V6-WORKFLOW-017.3" => "dspy_optimizer_and_teleprompter_runs_remain_receipted_policy_bounded_and_profile_safe",
        "V6-WORKFLOW-017.4" => "dspy_assertions_retry_or_reject_fail_closed_with_deterministic_receipts",
        "V6-WORKFLOW-017.5" => "dspy_multihop_rag_and_agent_loops_reuse_memory_skill_and_swarm_primitives",
        "V6-WORKFLOW-017.6" => "dspy_metrics_evaluators_and_benchmarks_stream_through_native_observability_and_evidence",
        "V6-WORKFLOW-017.7" => "dspy_integrations_normalize_through_governed_intake_and_adapter_bridges",
        "V6-WORKFLOW-017.8" => "dspy_optimization_and_reproducibility_traces_flow_through_native_observability",
        _ => "dspy_bridge_claim",
    }
}

fn read_swarm_state(path: &Path) -> Value {
    lane_utils::read_json(path)
        .unwrap_or_else(|| json!({ "sessions": {}, "handoff_registry": {}, "message_queues": {} }))
}

fn find_swarm_session_id_by_task(state: &Value, task: &str) -> Option<String> {
    lane_utils::find_swarm_session_id_by_task(state, task)
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
        return Err("dspy_observability_enable_failed".to_string());
    }
    let exit = crate::observability_plane::run(
        root,
        &[
            "acp-provenance".to_string(),
            "--op=trace".to_string(),
            "--source-agent=dspy-bridge".to_string(),
            format!("--target-agent={}", clean_token(Some(intent), "workflow")),
            format!("--intent={}", clean_text(Some(intent), 80)),
            format!("--message={}", clean_text(Some(message), 160)),
            format!("--trace-id={trace_id}"),
            "--visibility-mode=meta".to_string(),
            "--strict=1".to_string(),
        ],
    );
    if exit != 0 {
        return Err("dspy_observability_trace_failed".to_string());
    }
    Ok(())
}
