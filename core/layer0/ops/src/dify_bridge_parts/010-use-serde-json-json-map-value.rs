// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use serde_json::{json, Map, Value};
use std::fs;
use std::path::{Path, PathBuf};

use crate::contract_lane_utils::{
    self as lane_utils, clean_text, clean_token, cli_error, cli_receipt,
    normalize_bridge_path_clean, path_flag, payload_obj, print_json_line, rel_path as rel,
};
use crate::now_iso;

const DEFAULT_STATE_REL: &str = "local/state/ops/dify_bridge/latest.json";
const DEFAULT_HISTORY_REL: &str = "local/state/ops/dify_bridge/history.jsonl";
const DEFAULT_SWARM_STATE_REL: &str = "local/state/ops/dify_bridge/swarm_state.json";
const DEFAULT_TRACE_REL: &str = "local/state/ops/dify_bridge/audit_trace.jsonl";
const DEFAULT_DASHBOARD_REL: &str = "client/runtime/local/state/dify_dashboard_shell";

fn usage() {
    println!("dify-bridge commands:");
    println!("  protheus-ops dify-bridge status [--state-path=<path>]");
    println!("  protheus-ops dify-bridge register-canvas [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops dify-bridge sync-knowledge-base [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops dify-bridge register-agent-app [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops dify-bridge publish-dashboard [--payload-base64=<json>] [--state-path=<path>] [--dashboard-dir=<path>]");
    println!(
        "  protheus-ops dify-bridge route-provider [--payload-base64=<json>] [--state-path=<path>]"
    );
    println!("  protheus-ops dify-bridge run-conditional-flow [--payload-base64=<json>] [--state-path=<path>] [--swarm-state-path=<path>]");
    println!("  protheus-ops dify-bridge record-audit-trace [--payload-base64=<json>] [--state-path=<path>] [--trace-path=<path>]");
}

fn payload_json(argv: &[String]) -> Result<Value, String> {
    lane_utils::payload_json(argv, "dify_bridge")
}

fn bridge_path_flag(
    root: &Path,
    argv: &[String],
    payload: &Map<String, Value>,
    flag_name: &str,
    payload_key: &str,
    default_rel: &str,
) -> PathBuf {
    path_flag(root, argv, payload, flag_name, payload_key, default_rel)
}

fn state_path(root: &Path, argv: &[String], payload: &Map<String, Value>) -> PathBuf {
    bridge_path_flag(
        root,
        argv,
        payload,
        "state-path",
        "state_path",
        DEFAULT_STATE_REL,
    )
}

fn history_path(root: &Path, argv: &[String], payload: &Map<String, Value>) -> PathBuf {
    bridge_path_flag(
        root,
        argv,
        payload,
        "history-path",
        "history_path",
        DEFAULT_HISTORY_REL,
    )
}

fn swarm_state_path(root: &Path, argv: &[String], payload: &Map<String, Value>) -> PathBuf {
    bridge_path_flag(
        root,
        argv,
        payload,
        "swarm-state-path",
        "swarm_state_path",
        DEFAULT_SWARM_STATE_REL,
    )
}

fn trace_path(root: &Path, argv: &[String], payload: &Map<String, Value>) -> PathBuf {
    bridge_path_flag(
        root,
        argv,
        payload,
        "trace-path",
        "trace_path",
        DEFAULT_TRACE_REL,
    )
}

fn dashboard_dir(root: &Path, argv: &[String], payload: &Map<String, Value>) -> PathBuf {
    bridge_path_flag(
        root,
        argv,
        payload,
        "dashboard-dir",
        "dashboard_dir",
        DEFAULT_DASHBOARD_REL,
    )
}

fn default_state() -> Value {
    json!({
        "schema_version": "dify_bridge_state_v1",
        "canvases": {},
        "knowledge_bases": {},
        "agent_apps": {},
        "dashboards": {},
        "provider_routes": {},
        "flow_runs": {},
        "audit_traces": [],
        "last_receipt": null,
    })
}

fn ensure_state_shape(value: &mut Value) {
    if !value.is_object() {
        *value = default_state();
        return;
    }
    for key in [
        "canvases",
        "knowledge_bases",
        "agent_apps",
        "dashboards",
        "provider_routes",
        "flow_runs",
    ] {
        if !value.get(key).map(Value::is_object).unwrap_or(false) {
            value[key] = json!({});
        }
    }
    if !value
        .get("audit_traces")
        .map(Value::is_array)
        .unwrap_or(false)
    {
        value["audit_traces"] = json!([]);
    }
    if value
        .get("schema_version")
        .and_then(Value::as_str)
        .is_none()
    {
        value["schema_version"] = json!("dify_bridge_state_v1");
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

fn normalize_bridge_path(root: &Path, raw: &str) -> Result<String, String> {
    normalize_bridge_path_clean(root, raw, "dify_bridge_path_outside_allowed_surface")
}

fn claim(id: &str, claim: &str) -> Value {
    json!([{"id": id, "claim": claim}])
}

fn profile(raw: Option<&Value>) -> String {
    clean_token(raw.and_then(Value::as_str), "rich")
}

fn register_canvas(state: &mut Value, payload: &Map<String, Value>) -> Result<Value, String> {
    let name = clean_text(payload.get("name").and_then(Value::as_str), 120);
    if name.is_empty() {
        return Err("dify_canvas_name_required".to_string());
    }
    let nodes = payload
        .get("nodes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if nodes.is_empty() {
        return Err("dify_canvas_nodes_required".to_string());
    }
    let edges = payload
        .get("edges")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let canvas = json!({
        "canvas_id": stable_id("difycanvas", &json!({"name": name, "nodes": nodes, "edges": edges})),
        "name": name,
        "drag_and_drop": payload.get("drag_and_drop").and_then(Value::as_bool).unwrap_or(true),
        "node_count": nodes.len(),
        "edge_count": edges.len(),
        "conditional_edge_count": edges.iter().filter(|row| row.get("condition").is_some()).count(),
        "nodes": nodes,
        "edges": edges,
        "created_at": now_iso(),
    });
    let id = canvas
        .get("canvas_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "canvases").insert(id, canvas.clone());
    Ok(json!({
        "ok": true,
        "canvas": canvas,
        "claim_evidence": claim("V6-WORKFLOW-005.1", "dify_visual_canvas_nodes_edges_and_drag_drop_are_receipted_on_authoritative_workflow_surface"),
    }))
}
