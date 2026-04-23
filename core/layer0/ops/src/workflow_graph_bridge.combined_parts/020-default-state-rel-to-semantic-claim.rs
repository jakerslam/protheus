
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
