// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::contract_lane_utils as lane_utils;
use crate::now_iso;

const DEFAULT_STATE_REL: &str = "local/state/ops/llamaindex_bridge/latest.json";
const DEFAULT_HISTORY_REL: &str = "local/state/ops/llamaindex_bridge/history.jsonl";
const DEFAULT_SWARM_STATE_REL: &str = "local/state/ops/llamaindex_bridge/swarm_state.json";

fn usage() {
    println!("llamaindex-bridge commands:");
    println!("  protheus-ops llamaindex-bridge status [--state-path=<path>]");
    println!("  protheus-ops llamaindex-bridge register-index [--payload-base64=<json>] [--state-path=<path>]");
    println!(
        "  protheus-ops llamaindex-bridge query [--payload-base64=<json>] [--state-path=<path>]"
    );
    println!("  protheus-ops llamaindex-bridge run-agent-workflow [--payload-base64=<json>] [--state-path=<path>] [--swarm-state-path=<path>]");
    println!("  protheus-ops llamaindex-bridge ingest-multimodal [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops llamaindex-bridge record-memory-eval [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops llamaindex-bridge run-conditional-workflow [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops llamaindex-bridge emit-trace [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops llamaindex-bridge register-connector [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops llamaindex-bridge connector-query [--payload-base64=<json>] [--state-path=<path>]");
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
    lane_utils::payload_json(argv, "llamaindex_bridge")
}

fn payload_obj<'a>(value: &'a Value) -> &'a Map<String, Value> {
    lane_utils::payload_obj(value)
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
        "schema_version": "llamaindex_bridge_state_v1",
        "indexes": {},
        "agent_workflows": {},
        "ingestions": {},
        "memory_store": {},
        "evaluations": {},
        "conditional_workflows": {},
        "traces": [],
        "connectors": {},
        "last_receipt": null,
    })
}

fn ensure_state_shape(value: &mut Value) {
    if !value.is_object() {
        *value = default_state();
        return;
    }
    for key in [
        "indexes",
        "agent_workflows",
        "ingestions",
        "memory_store",
        "evaluations",
        "conditional_workflows",
        "connectors",
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
        value["schema_version"] = json!("llamaindex_bridge_state_v1");
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
    let cleaned = raw
        .unwrap_or_default()
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | ':'))
        .collect::<String>();
    if cleaned.is_empty() {
        fallback.to_string()
    } else {
        cleaned
    }
}

fn parse_u64_value(value: Option<&Value>, fallback: u64, min: u64, max: u64) -> u64 {
    lane_utils::json_u64(value, fallback, min, max)
}

fn parse_f64_value(value: Option<&Value>, fallback: f64, min: f64, max: f64) -> f64 {
    value.and_then(Value::as_f64).unwrap_or(fallback).clamp(min, max)
}

fn rel(root: &Path, path: &Path) -> String {
    lane_utils::rel_path(root, path)
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
    lane_utils::read_json(path).unwrap_or_else(|| json!({ "sessions": {}, "handoff_registry": {} }))
}

fn find_swarm_session_id_by_task(state: &Value, task: &str) -> Option<String> {
    lane_utils::find_swarm_session_id_by_task(state, task)
}

fn semantic_claim(id: &str) -> &'static str {
    match id {
        "V6-WORKFLOW-009.1" => "llamaindex_indexes_retrievers_and_query_engines_are_governed_and_receipted",
        "V6-WORKFLOW-009.2" => "llamaindex_agentic_workflows_reuse_authoritative_swarm_handoffs_and_receipted_tool_calls",
        "V6-WORKFLOW-009.3" => "llamaindex_multimodal_ingestion_and_loader_paths_enforce_profile_degradation_and_receipts",
        "V6-WORKFLOW-009.4" => "llamaindex_memory_store_and_eval_outputs_persist_as_governed_observability_artifacts",
        "V6-WORKFLOW-009.5" => "llamaindex_conditional_workflows_route_deterministically_with_checkpoint_receipts",
        "V6-WORKFLOW-009.6" => "llamaindex_traces_fold_into_native_observability_without_duplicate_telemetry_stacks",
        "V6-WORKFLOW-009.7" => "llamaindex_connectors_normalize_into_governed_manifests_with_fail_closed_query_paths",
        _ => "llamaindex_bridge_claim",
    }
}

fn doc_token_set(doc: &Value) -> BTreeSet<String> {
    clean_text(doc.get("text").and_then(Value::as_str), 4096)
        .to_ascii_lowercase()
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|row| !row.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn query_terms(query: &str) -> Vec<String> {
    clean_text(Some(query), 240)
        .to_ascii_lowercase()
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|row| !row.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn retrieval_score(doc: &Value, terms: &[String], mode: &str) -> i64 {
    let tokens = doc_token_set(doc);
    let mut score = 0i64;
    for term in terms {
        if tokens.contains(term) {
            score += match mode {
                "graph" => 4,
                "vector" => 3,
                _ => 2,
            };
        }
    }
    if mode == "hybrid"
        && doc
            .get("metadata")
            .and_then(|row| row.get("kind"))
            .and_then(Value::as_str)
            == Some("graph")
    {
        score += 2;
    }
    score
}

fn register_index(state: &mut Value, payload: &Map<String, Value>) -> Result<Value, String> {
    let name = clean_token(
        payload.get("name").and_then(Value::as_str),
        "llamaindex-index",
    );
    let documents = payload
        .get("documents")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if documents.is_empty() {
        return Err("llamaindex_index_documents_required".to_string());
    }
    let retrieval_modes = payload
        .get("retrieval_modes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_else(|| vec![json!("hybrid"), json!("vector"), json!("graph")]);
    let query_engine = clean_token(
        payload.get("query_engine").and_then(Value::as_str),
        "router",
    );
    let index = json!({
        "index_id": stable_id("llxidx", &json!({"name": name, "engine": query_engine})),
        "name": name,
        "retrieval_modes": retrieval_modes,
        "query_engine": query_engine,
        "documents": documents,
        "registered_at": now_iso(),
    });
    let index_id = index
        .get("index_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "indexes").insert(index_id, index.clone());
    Ok(json!({
        "ok": true,
        "index": index,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-009.1", semantic_claim("V6-WORKFLOW-009.1")),
    }))
}
