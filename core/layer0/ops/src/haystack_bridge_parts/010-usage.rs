// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use serde_json::{json, Map, Value};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::contract_lane_utils as lane_utils;
use crate::{deterministic_receipt_hash, now_iso};

const DEFAULT_STATE_REL: &str = "local/state/ops/haystack_bridge/latest.json";
const DEFAULT_HISTORY_REL: &str = "local/state/ops/haystack_bridge/history.jsonl";
const DEFAULT_SWARM_STATE_REL: &str = "local/state/ops/haystack_bridge/swarm_state.json";

fn usage() {
    println!("haystack-bridge commands:");
    println!("  protheus-ops haystack-bridge status [--state-path=<path>]");
    println!("  protheus-ops haystack-bridge register-pipeline [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops haystack-bridge run-pipeline [--payload-base64=<json>] [--state-path=<path>] [--swarm-state-path=<path>]");
    println!("  protheus-ops haystack-bridge run-agent-toolset [--payload-base64=<json>] [--state-path=<path>] [--swarm-state-path=<path>]");
    println!("  protheus-ops haystack-bridge register-template [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops haystack-bridge render-template [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops haystack-bridge register-document-store [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops haystack-bridge retrieve-documents [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops haystack-bridge route-and-rank [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops haystack-bridge record-multimodal-eval [--payload-base64=<json>] [--state-path=<path>]");
    println!(
        "  protheus-ops haystack-bridge trace-run [--payload-base64=<json>] [--state-path=<path>]"
    );
    println!("  protheus-ops haystack-bridge import-connector [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops haystack-bridge assimilate-intake [--payload-base64=<json>] [--state-path=<path>]");
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
    lane_utils::payload_json(argv, "haystack_bridge")
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
        "schema_version": "haystack_bridge_state_v1",
        "pipelines": {},
        "pipeline_runs": {},
        "agent_runs": {},
        "templates": {},
        "template_renders": {},
        "document_stores": {},
        "retrieval_runs": {},
        "routes": {},
        "evaluations": {},
        "traces": [],
        "connectors": {},
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
        "pipelines",
        "pipeline_runs",
        "agent_runs",
        "templates",
        "template_renders",
        "document_stores",
        "retrieval_runs",
        "routes",
        "evaluations",
        "connectors",
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
        value["schema_version"] = json!("haystack_bridge_state_v1");
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
    let value = clean_text(raw, 96);
    if value.is_empty() {
        fallback.to_string()
    } else {
        value
    }
}

fn parse_u64_value(value: Option<&Value>, fallback: u64, min: u64, max: u64) -> u64 {
    lane_utils::json_u64(value, fallback, min, max)
}

fn parse_bool_value(value: Option<&Value>, fallback: bool) -> bool {
    lane_utils::json_bool(value, fallback)
}

fn normalize_bridge_path(root: &Path, raw: &str) -> Result<String, String> {
    let cleaned = clean_text(Some(raw), 240);
    lane_utils::normalize_prefixed_path(
        root,
        &cleaned,
        "haystack_bridge_path_required",
        "haystack_bridge_path_escapes_adapters",
        "haystack_bridge_path_must_be_adapter_owned",
        &["adapters/"],
    )
}

fn normalize_shell_path(root: &Path, raw: &str) -> Result<String, String> {
    let cleaned = clean_text(Some(raw), 240);
    lane_utils::normalize_prefixed_path(
        root,
        &cleaned,
        "haystack_shell_path_required",
        "haystack_shell_path_escapes_workspace",
        "haystack_shell_path_must_live_under_client_or_apps",
        &["client/", "apps/"],
    )
}

fn default_claim_evidence(id: &str, claim: &str) -> Value {
    json!([{ "id": id, "claim": claim }])
}

fn haystack_claim(id: &str) -> &'static str {
    match id {
        "V6-WORKFLOW-012.1" => {
            "haystack_pipelines_register_and_execute_as_governed_component_graphs"
        }
        "V6-WORKFLOW-012.2" => {
            "haystack_searchable_tool_agents_reduce_tool_fanout_and_execute_through_swarm_authority"
        }
        "V6-WORKFLOW-012.3" => {
            "haystack_templates_and_rendered_prompts_are_versioned_provenanced_and_receipted"
        }
        "V6-WORKFLOW-012.4" => {
            "haystack_document_stores_and_rag_queries_normalize_to_governed_retrieval_runtime"
        }
        "V6-WORKFLOW-012.5" => {
            "haystack_routes_and_rankers_are_deterministic_replayable_and_fail_closed"
        }
        "V6-WORKFLOW-012.6" => {
            "haystack_multimodal_evals_emit_typed_artifacts_and_governed_metrics"
        }
        "V6-WORKFLOW-012.7" => {
            "haystack_step_traces_and_branch_decisions_fold_into_native_observability"
        }
        "V6-WORKFLOW-012.8" => {
            "haystack_connectors_and_pipeline_assets_ingest_through_one_governed_gateway"
        }
        _ => "haystack_bridge_claim",
    }
}

fn read_swarm_state(path: &Path) -> Value {
    lane_utils::read_json(path).unwrap_or_else(|| json!({ "sessions": {}, "handoff_registry": {} }))
}

fn find_swarm_session_id_by_task(state: &Value, task: &str) -> Option<String> {
    lane_utils::find_swarm_session_id_by_task(state, task)
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
        return Err(format!("haystack_swarm_spawn_failed:{label}"));
    }
    let swarm_state = read_swarm_state(swarm_state_path);
    find_swarm_session_id_by_task(&swarm_state, task)
        .ok_or_else(|| format!("haystack_swarm_session_missing:{label}"))
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
        return Err("haystack_observability_enable_failed".to_string());
    }
    let exit = crate::observability_plane::run(
        root,
        &[
            "acp-provenance".to_string(),
            "--op=trace".to_string(),
            "--source-agent=haystack-bridge".to_string(),
            format!("--target-agent={}", clean_token(Some(intent), "workflow")),
            format!("--intent={}", clean_text(Some(intent), 80)),
            format!("--message={}", clean_text(Some(message), 160)),
            format!("--trace-id={trace_id}"),
            "--visibility-mode=meta".to_string(),
            "--strict=1".to_string(),
        ],
    );
    if exit != 0 {
        return Err("haystack_observability_trace_failed".to_string());
    }
    Ok(())
}
