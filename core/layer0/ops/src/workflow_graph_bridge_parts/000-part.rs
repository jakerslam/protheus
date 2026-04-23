// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use serde_json::{json, Map, Value};
use std::path::{Path, PathBuf};

use crate::contract_lane_utils::{
    self as lane_utils, clean_text, clean_token, cli_error, cli_receipt, normalize_bridge_path,
    path_flag, payload_obj, print_json_line, rel_path as rel,
};
use crate::now_iso;

const DEFAULT_STATE_REL: &str = "local/state/ops/workflow_graph_bridge/latest.json";
const DEFAULT_HISTORY_REL: &str = "local/state/ops/workflow_graph_bridge/history.jsonl";
const DEFAULT_SWARM_STATE_REL: &str = "local/state/ops/workflow_graph_bridge/swarm_state.json";
const DEFAULT_TRACE_REL: &str = "local/state/ops/workflow_graph_bridge/native_trace.jsonl";

fn usage() {
    println!("workflow_graph-bridge commands:");
    println!("  infring-ops workflow_graph-bridge status [--state-path=<path>]");
    println!("  infring-ops workflow_graph-bridge register-graph [--payload-base64=<json>] [--state-path=<path>]");
    println!("  infring-ops workflow_graph-bridge checkpoint-run [--payload-base64=<json>] [--state-path=<path>]");
    println!("  infring-ops workflow_graph-bridge inspect-state [--payload-base64=<json>] [--state-path=<path>]");
    println!("  infring-ops workflow_graph-bridge interrupt-run [--payload-base64=<json>] [--state-path=<path>]");
    println!("  infring-ops workflow_graph-bridge resume-run [--payload-base64=<json>] [--state-path=<path>]");
    println!("  infring-ops workflow_graph-bridge coordinate-subgraph [--payload-base64=<json>] [--state-path=<path>] [--swarm-state-path=<path>]");
    println!("  infring-ops workflow_graph-bridge record-trace [--payload-base64=<json>] [--state-path=<path>]");
    println!("  infring-ops workflow_graph-bridge stream-graph [--payload-base64=<json>] [--state-path=<path>]");
    println!("  infring-ops workflow_graph-bridge run-governed-workflow [--payload-base64=<json>] [--state-path=<path>]");
}

fn payload_json(argv: &[String]) -> Result<Value, String> {
    lane_utils::payload_json(argv, "workflow_graph_bridge")
}

fn bridge_path(
    root: &Path,
    argv: &[String],
    payload: &Map<String, Value>,
    cli_flag: &str,
    payload_key: &str,
    fallback: &str,
) -> PathBuf {
    path_flag(
        root,
        argv,
        payload,
        cli_flag,
        payload_key,
        fallback,
    )
}

fn state_path(root: &Path, argv: &[String], payload: &Map<String, Value>) -> PathBuf {
    bridge_path(
        root,
        argv,
        payload,
        "state-path",
        "state_path",
        DEFAULT_STATE_REL,
    )
}

fn history_path(root: &Path, argv: &[String], payload: &Map<String, Value>) -> PathBuf {
    bridge_path(
        root,
        argv,
        payload,
        "history-path",
        "history_path",
        DEFAULT_HISTORY_REL,
    )
}

fn swarm_state_path(root: &Path, argv: &[String], payload: &Map<String, Value>) -> PathBuf {
    bridge_path(
        root,
        argv,
        payload,
        "swarm-state-path",
        "swarm_state_path",
        DEFAULT_SWARM_STATE_REL,
    )
}

fn trace_path(root: &Path, argv: &[String], payload: &Map<String, Value>) -> PathBuf {
    bridge_path(
        root,
        argv,
        payload,
        "trace-path",
        "trace_path",
        DEFAULT_TRACE_REL,
    )
}

fn default_state() -> Value {
    json!({
        "schema_version": "workflow_graph_bridge_state_v1",
        "graphs": {},
        "checkpoints": {},
        "inspections": {},
        "interrupts": {},
        "subgraphs": {},
        "governed_workflows": {},
        "traces": [],
        "streams": [],
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
        "checkpoints",
        "inspections",
        "interrupts",
        "subgraphs",
        "governed_workflows",
    ] {
        if !value.get(key).map(Value::is_object).unwrap_or(false) {
            value[key] = json!({});
        }
    }
    for key in ["traces", "streams"] {
        if !value.get(key).map(Value::is_array).unwrap_or(false) {
            value[key] = json!([]);
        }
    }
    if value
        .get("schema_version")
        .and_then(Value::as_str)
        .is_none()
    {
        value["schema_version"] = json!("workflow_graph_bridge_state_v1");
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

fn default_claim_evidence(id: &str, claim: &str) -> Value {
    json!([{ "id": id, "claim": claim }])
}

fn semantic_claim(id: &str) -> &'static str {
    match id {
        "V6-WORKFLOW-002.1" => {
            "workflow_graph_nodes_edges_and_cycles_register_as_governed_receipted_graphs"
        }
        "V6-WORKFLOW-002.2" => {
            "workflow_graph_checkpoints_and_time_travel_replay_route_through_receipted_persistence"
        }
        "V6-WORKFLOW-002.3" => {
            "workflow_graph_hitl_state_inspection_and_intervention_remain_governed_and_receipted"
        }
        "V6-WORKFLOW-002.4" => {
            "workflow_graph_subgraphs_and_nested_agents_reuse_authoritative_swarm_lineage"
        }
        "V6-WORKFLOW-002.5" => {
            "workflow_graph_traces_fold_into_native_observability_without_duplicate_telemetry_stacks"
        }
        "V6-WORKFLOW-002.6" => {
            "workflow_graph_streaming_and_conditional_edges_remain_receipted_and_fail_closed"
        }
        "V6-WORKFLOW-002.7" => {
            "workflow_graph_interrupt_and_resume_lifecycle_stays_receipted_and_fail_closed"
        }
        "V6-WORKFLOW-002.8" => {
            "workflow_graph_frontend_adapter_execution_routes_through_tooling_claims_and_unified_memory_authority"
        }
        _ => "workflow_graph_bridge_claim",
    }
}

fn emit_native_trace(
    root: &Path,
    trace_path: &Path,
    trace_id: &str,
    stage: &str,
    message: &str,
) -> Result<(), String> {
    lane_utils::append_jsonl(
        trace_path,
        &json!({
            "trace_id": clean_token(Some(trace_id), "workflow_graph-trace"),
            "stage": clean_token(Some(stage), "graph"),
            "message": clean_text(Some(message), 200),
            "recorded_at": now_iso(),
            "root": rel(root, trace_path),
        }),
    )
}

fn read_swarm_state(path: &Path) -> Value {
    lane_utils::read_json(path).unwrap_or_else(|| json!({"sessions": {}, "handoff_registry": {}}))
}

fn save_swarm_state(path: &Path, state: &Value) -> Result<(), String> {
    lane_utils::write_json(path, state)
}

fn normalize_node(node: &Value) -> Value {
    let obj = node.as_object().cloned().unwrap_or_default();
    let node_id = clean_token(obj.get("id").and_then(Value::as_str), "node");
    json!({
        "id": node_id,
        "kind": clean_token(obj.get("kind").and_then(Value::as_str), "step"),
        "tool": clean_token(obj.get("tool").and_then(Value::as_str), ""),
        "checkpoint_key": clean_token(obj.get("checkpoint_key").and_then(Value::as_str), ""),
        "prompt": clean_text(obj.get("prompt").and_then(Value::as_str), 240),
    })
}

fn normalize_edge(edge: &Value) -> Value {
    let obj = edge.as_object().cloned().unwrap_or_default();
    json!({
        "from": clean_token(obj.get("from").and_then(Value::as_str), ""),
        "to": clean_token(obj.get("to").and_then(Value::as_str), ""),
        "label": clean_token(obj.get("label").and_then(Value::as_str), "edge"),
        "default": obj.get("default").and_then(Value::as_bool).unwrap_or(false),
        "condition": obj.get("condition").cloned().unwrap_or_else(|| json!(null)),
    })
}

fn condition_matches(condition: &Value, context: &Map<String, Value>) -> bool {
    let Some(obj) = condition.as_object() else {
        return false;
    };
    let field = obj.get("field").and_then(Value::as_str).unwrap_or_default();
    if field.is_empty() {
        return false;
    }
    let equals = obj.get("equals");
    let contains = obj.get("contains").and_then(Value::as_str);
    match (context.get(field), equals, contains) {
        (Some(actual), Some(expected), _) => actual == expected,
        (Some(actual), _, Some(needle)) => actual
            .as_str()
            .map(|row| row.contains(needle))
            .unwrap_or(false),
        _ => false,
    }
}

fn register_graph(state: &mut Value, payload: &Map<String, Value>) -> Result<Value, String> {
    let name = clean_token(
        payload.get("name").and_then(Value::as_str),
        "workflow_graph-graph",
    );
    let nodes = payload
        .get("nodes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if nodes.is_empty() {
        return Err("workflow_graph_nodes_required".to_string());
    }
    let edges = payload
        .get("edges")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let normalized_nodes: Vec<Value> = nodes.iter().map(normalize_node).collect();
    let normalized_edges: Vec<Value> = edges.iter().map(normalize_edge).collect();
    let entry_node = clean_token(
        payload
            .get("entry_node")
            .and_then(Value::as_str)
            .or_else(|| {
                normalized_nodes
                    .first()
                    .and_then(|row| row.get("id"))
                    .and_then(Value::as_str)
            }),
        "start",
    );
    let graph = json!({
        "graph_id": stable_id("lggraph", &json!({"name": name, "entry": entry_node})),
        "name": name,
        "entry_node": entry_node,
        "checkpoint_mode": clean_token(payload.get("checkpoint_mode").and_then(Value::as_str), "per_node"),
        "nodes": normalized_nodes,
        "edges": normalized_edges,
        "node_count": nodes.len(),
        "edge_count": edges.len(),
        "conditional_edge_count": normalized_edges.iter().filter(|row| row.get("condition").map(|v| !v.is_null()).unwrap_or(false)).count(),
        "registered_at": now_iso(),
    });
    let graph_id = graph
        .get("graph_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "graphs").insert(graph_id, graph.clone());
    Ok(json!({
        "ok": true,
        "graph": graph,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-002.1", semantic_claim("V6-WORKFLOW-002.1")),
    }))
}

fn checkpoint_run(state: &mut Value, payload: &Map<String, Value>) -> Result<Value, String> {
    let graph_id = clean_token(payload.get("graph_id").and_then(Value::as_str), "");
    if graph_id.is_empty() {
        return Err("workflow_graph_checkpoint_graph_id_required".to_string());
    }
    let graph = state
        .get("graphs")
        .and_then(Value::as_object)
        .and_then(|rows| rows.get(&graph_id))
        .cloned()
        .ok_or_else(|| format!("unknown_workflow_graph_graph:{graph_id}"))?;
    let snapshot = payload
        .get("state_snapshot")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let replay_enabled = payload
        .get("replay_enabled")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let checkpoint = json!({
        "checkpoint_id": stable_id("lgcp", &json!({"graph_id": graph_id, "snapshot": snapshot})),
        "graph_id": graph_id,
        "graph_name": graph.get("name").cloned().unwrap_or_else(|| json!(null)),
        "thread_id": clean_token(payload.get("thread_id").and_then(Value::as_str), "thread"),
        "checkpoint_label": clean_token(payload.get("checkpoint_label").and_then(Value::as_str), "graph_step"),
        "snapshot": snapshot,
        "snapshot_hash": crate::deterministic_receipt_hash(&json!({"snapshot": payload.get("state_snapshot").cloned().unwrap_or_else(|| json!({}))})),
        "replay_enabled": replay_enabled,
        "replay_token": if replay_enabled { json!(stable_id("lgreplay", &json!({"graph_id": graph_id}))) } else { json!(null) },
        "rewind_from": clean_token(payload.get("rewind_from").and_then(Value::as_str), ""),
        "captured_at": now_iso(),
    });
    let checkpoint_id = checkpoint
        .get("checkpoint_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "checkpoints").insert(checkpoint_id, checkpoint.clone());
    Ok(json!({
        "ok": true,
        "checkpoint": checkpoint,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-002.2", semantic_claim("V6-WORKFLOW-002.2")),
    }))
}

fn inspect_state(state: &mut Value, payload: &Map<String, Value>) -> Result<Value, String> {
    let checkpoint_id = clean_token(payload.get("checkpoint_id").and_then(Value::as_str), "");
    let checkpoint = if checkpoint_id.is_empty() {
        None
    } else {
        state
            .get("checkpoints")
            .and_then(Value::as_object)
            .and_then(|rows| rows.get(&checkpoint_id))
            .cloned()
    };
    let graph_id = clean_token(
        payload.get("graph_id").and_then(Value::as_str).or_else(|| {
            checkpoint
                .as_ref()
                .and_then(|row| row.get("graph_id"))
                .and_then(Value::as_str)
        }),
        "",
    );
    if graph_id.is_empty() {
        return Err("workflow_graph_inspection_graph_or_checkpoint_required".to_string());
    }
    let state_view = checkpoint
        .as_ref()
        .and_then(|row| row.get("snapshot"))
        .cloned()
        .unwrap_or_else(|| json!({}));
    let intervention = payload
        .get("intervention_patch")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let inspection = json!({
