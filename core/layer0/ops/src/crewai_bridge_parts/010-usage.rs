// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use serde_json::{json, Map, Value};
use std::fs;
use std::path::{Path, PathBuf};

use crate::contract_lane_utils::{
    self as lane_utils, clean_text, clean_token, cli_error, cli_receipt, normalize_bridge_path,
    payload_obj, print_json_line, rel_path as rel, repo_path,
};
use crate::{deterministic_receipt_hash, now_iso};

const DEFAULT_STATE_REL: &str = "local/state/ops/crewai_bridge/latest.json";
const DEFAULT_HISTORY_REL: &str = "local/state/ops/crewai_bridge/history.jsonl";
const DEFAULT_SWARM_STATE_REL: &str = "local/state/ops/crewai_bridge/swarm_state.json";
const DEFAULT_APPROVAL_QUEUE_REL: &str = "client/runtime/local/state/crewai_approvals.yaml";
const DEFAULT_TRACE_REL: &str = "local/state/ops/crewai_bridge/amp_trace.jsonl";

fn usage() {
    println!("crewai-bridge commands:");
    println!("  protheus-ops crewai-bridge status [--state-path=<path>]");
    println!("  protheus-ops crewai-bridge register-crew [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops crewai-bridge run-process [--payload-base64=<json>] [--state-path=<path>] [--swarm-state-path=<path>]");
    println!(
        "  protheus-ops crewai-bridge run-flow [--payload-base64=<json>] [--state-path=<path>]"
    );
    println!("  protheus-ops crewai-bridge memory-bridge [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops crewai-bridge ingest-config [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops crewai-bridge route-delegation [--payload-base64=<json>] [--state-path=<path>] [--swarm-state-path=<path>]");
    println!("  protheus-ops crewai-bridge review-crew [--payload-base64=<json>] [--state-path=<path>] [--approval-queue-path=<path>]");
    println!("  protheus-ops crewai-bridge record-amp-trace [--payload-base64=<json>] [--state-path=<path>] [--trace-path=<path>]");
    println!("  protheus-ops crewai-bridge benchmark-parity [--payload-base64=<json>] [--state-path=<path>]");
    println!(
        "  protheus-ops crewai-bridge route-model [--payload-base64=<json>] [--state-path=<path>]"
    );
    println!("  protheus-ops crewai-bridge run-governed-workflow [--payload-base64=<json>] [--state-path=<path>]");
}

fn payload_json(argv: &[String]) -> Result<Value, String> {
    lane_utils::payload_json(argv, "crewai_bridge")
}

fn state_path(root: &Path, argv: &[String], payload: &Map<String, Value>) -> PathBuf {
    lane_utils::parse_flag(argv, "state-path", false)
        .or_else(|| {
            payload
                .get("state_path")
                .and_then(Value::as_str)
                .map(ToString::to_string)
        })
        .map(|raw| repo_path(root, &raw))
        .unwrap_or_else(|| root.join(DEFAULT_STATE_REL))
}

fn history_path(root: &Path, argv: &[String], payload: &Map<String, Value>) -> PathBuf {
    lane_utils::parse_flag(argv, "history-path", false)
        .or_else(|| {
            payload
                .get("history_path")
                .and_then(Value::as_str)
                .map(ToString::to_string)
        })
        .map(|raw| repo_path(root, &raw))
        .unwrap_or_else(|| root.join(DEFAULT_HISTORY_REL))
}

fn swarm_state_path(root: &Path, argv: &[String], payload: &Map<String, Value>) -> PathBuf {
    lane_utils::parse_flag(argv, "swarm-state-path", false)
        .or_else(|| {
            payload
                .get("swarm_state_path")
                .and_then(Value::as_str)
                .map(ToString::to_string)
        })
        .map(|raw| repo_path(root, &raw))
        .unwrap_or_else(|| root.join(DEFAULT_SWARM_STATE_REL))
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

fn trace_path(root: &Path, argv: &[String], payload: &Map<String, Value>) -> PathBuf {
    lane_utils::parse_flag(argv, "trace-path", false)
        .or_else(|| {
            payload
                .get("trace_path")
                .and_then(Value::as_str)
                .map(ToString::to_string)
        })
        .map(|raw| repo_path(root, &raw))
        .unwrap_or_else(|| root.join(DEFAULT_TRACE_REL))
}

fn default_state() -> Value {
    json!({
        "schema_version": "crewai_bridge_state_v1",
        "crews": {},
        "process_runs": {},
        "flow_runs": {},
        "memory_records": {},
        "configs": {},
        "delegations": {},
        "reviews": {},
        "traces": [],
        "benchmarks": {},
        "model_routes": {},
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
        "crews",
        "process_runs",
        "flow_runs",
        "memory_records",
        "configs",
        "delegations",
        "reviews",
        "benchmarks",
        "model_routes",
        "governed_workflows",
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
        value["schema_version"] = json!("crewai_bridge_state_v1");
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

fn now_millis() -> u128 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|row| row.as_millis())
        .unwrap_or(0)
}

fn to_base36(mut value: u128) -> String {
    if value == 0 {
        return "0".to_string();
    }
    let mut out = Vec::new();
    while value > 0 {
        let digit = (value % 36) as u8;
        out.push(if digit < 10 {
            (b'0' + digit) as char
        } else {
            (b'a' + digit - 10) as char
        });
        value /= 36;
    }
    out.iter().rev().collect()
}

fn stable_id(prefix: &str, basis: &Value) -> String {
    let digest = deterministic_receipt_hash(basis);
    format!("{prefix}_{}_{}", to_base36(now_millis()), &digest[..12])
}

fn clean_tools(value: Option<&Value>) -> Vec<Value> {
    value
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| match row {
            Value::String(s) => Some(Value::String(clean_token(Some(&s), "tool"))),
            Value::Object(obj) => obj
                .get("name")
                .and_then(Value::as_str)
                .map(|name| Value::String(clean_token(Some(name), "tool"))),
            _ => None,
        })
        .collect()
}

fn default_claim_evidence(id: &str, claim: &str) -> Value {
    json!([{ "id": id, "claim": claim }])
}

fn semantic_claim(id: &str) -> &'static str {
    match id {
        "V6-WORKFLOW-004.1" => "crewai_roles_goals_and_crews_register_as_governed_receipted_execution_units",
        "V6-WORKFLOW-004.2" => "crewai_sequential_and_hierarchical_processes_reuse_authoritative_workflow_and_swarm_primitives",
        "V6-WORKFLOW-004.3" => "crewai_event_driven_flows_and_decorators_route_through_fail_closed_workflow_paths",
        "V6-WORKFLOW-004.4" => "crewai_unified_memory_routes_through_canonical_receipted_memory_authority",
        "V6-WORKFLOW-004.5" => "crewai_yaml_and_declarative_config_assets_normalize_through_governed_intake_bridges",
        "V6-WORKFLOW-004.6" => "crewai_dynamic_delegation_and_tool_routing_stay_receipted_and_profile_aware",
        "V6-WORKFLOW-004.7" => "crewai_human_review_and_intervention_reuse_existing_approval_boundaries",
        "V6-WORKFLOW-004.8" => "crewai_amp_style_tracing_and_control_plane_events_fold_into_native_observability",
        "V6-WORKFLOW-004.9" => "crewai_runtime_parity_claims_route_through_governed_benchmark_receipts",
        "V6-WORKFLOW-004.10" => "crewai_multimodal_and_local_model_routing_remains_adapter_owned_and_fail_closed",
        "V6-WORKFLOW-004.11" => "crewai_frontend_adapter_execution_routes_through_tooling_claims_and_unified_memory_authority",
        _ => "crewai_bridge_claim",
    }
}

fn read_swarm_state(path: &Path) -> Value {
    lane_utils::read_json(path).unwrap_or_else(|| json!({"sessions": {}, "handoff_registry": {}}))
}

fn save_swarm_state(path: &Path, state: &Value) -> Result<(), String> {
    lane_utils::write_json(path, state)
}

fn load_review_queue(path: &Path) -> Value {
    match fs::read_to_string(path) {
        Ok(raw) => serde_yaml::from_str::<Value>(&raw).unwrap_or_else(|_| json!({"entries": []})),
        Err(_) => json!({"entries": []}),
    }
}

fn save_review_queue(path: &Path, queue: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("crewai_review_queue_parent_create_failed:{err}"))?;
    }
    let encoded = serde_yaml::to_string(queue)
        .map_err(|err| format!("crewai_review_queue_encode_failed:{err}"))?;
    fs::write(path, encoded).map_err(|err| format!("crewai_review_queue_write_failed:{err}"))
}

fn emit_amp_trace(trace_path: &Path, row: &Value) -> Result<(), String> {
    lane_utils::append_jsonl(trace_path, row)
}

fn normalize_agent(agent: &Value) -> Value {
    let obj = agent.as_object().cloned().unwrap_or_default();
    json!({
        "agent_id": clean_token(obj.get("agent_id").and_then(Value::as_str).or_else(|| obj.get("role").and_then(Value::as_str)), "agent"),
        "role": clean_token(obj.get("role").and_then(Value::as_str), "specialist"),
        "goal": clean_text(obj.get("goal").and_then(Value::as_str), 180),
        "backstory": clean_text(obj.get("backstory").and_then(Value::as_str), 200),
        "tools": clean_tools(obj.get("tools")),
        "multimodal": obj.get("multimodal").and_then(Value::as_bool).unwrap_or(false),
        "local_model_only": obj.get("local_model_only").and_then(Value::as_bool).unwrap_or(false),
    })
}

fn normalize_task(task: &Value, idx: usize) -> Value {
    let obj = task.as_object().cloned().unwrap_or_default();
    json!({
        "task_id": clean_token(obj.get("task_id").and_then(Value::as_str), &format!("task{}", idx + 1)),
        "name": clean_token(obj.get("name").and_then(Value::as_str), &format!("task{}", idx + 1)),
        "description": clean_text(obj.get("description").and_then(Value::as_str), 180),
        "role_hint": clean_token(obj.get("role_hint").and_then(Value::as_str), ""),
        "required_tool": clean_token(obj.get("required_tool").and_then(Value::as_str), ""),
    })
}

fn select_agent_for_task(agents: &[Value], task: &Value) -> Option<Value> {
    let role_hint = task
        .get("role_hint")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let required_tool = task
        .get("required_tool")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if !required_tool.is_empty() {
        if let Some(agent) = agents.iter().find(|agent| {
            agent
                .get("tools")
                .and_then(Value::as_array)
                .map(|rows| rows.iter().any(|row| row.as_str() == Some(required_tool)))
                .unwrap_or(false)
        }) {
            return Some(agent.clone());
        }
    }
    if !role_hint.is_empty() {
        if let Some(agent) = agents
            .iter()
            .find(|agent| agent.get("role").and_then(Value::as_str) == Some(role_hint))
        {
            return Some(agent.clone());
        }
    }
    agents.first().cloned()
}

fn allowed_route(route: &Value, trigger_event: &str, context: &Map<String, Value>) -> bool {
    let obj = route.as_object().cloned().unwrap_or_default();
    let event = obj.get("event").and_then(Value::as_str).unwrap_or_default();
    let default_route = obj.get("default").and_then(Value::as_bool).unwrap_or(false);
    if !event.is_empty() && event != trigger_event {
        return false;
    }
    if let Some(condition) = obj.get("condition").and_then(Value::as_object) {
        let field = condition
            .get("field")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if field.is_empty() {
            return default_route;
        }
        let actual = context.get(field);
        if let Some(expected) = condition.get("equals") {
            return actual == Some(expected);
        }
        return default_route;
    }
    default_route || event == trigger_event
}

fn top_level_unsupported_keys(obj: &Map<String, Value>) -> Vec<String> {
    const ALLOWED: &[&str] = &[
        "crew", "agents", "tasks", "flows", "process", "tools", "models", "memory", "config",
    ];
    let mut unsupported = Vec::new();
    for key in obj.keys() {
        if !ALLOWED.iter().any(|allowed| allowed == key) {
            unsupported.push(key.to_string());
        }
    }
    unsupported.sort();
    unsupported
}

fn register_crew(state: &mut Value, payload: &Map<String, Value>) -> Result<Value, String> {
    let agents = payload
        .get("agents")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if agents.is_empty() {
        return Err("crewai_agents_required".to_string());
    }
    let normalized_agents: Vec<Value> = agents.iter().map(normalize_agent).collect();
    let crew = json!({
        "crew_id": stable_id("crew", &json!({"name": payload.get("crew_name"), "agents": normalized_agents})),
        "crew_name": clean_token(payload.get("crew_name").and_then(Value::as_str), "crew"),
        "process_type": clean_token(payload.get("process_type").and_then(Value::as_str), "sequential"),
        "manager_role": clean_token(payload.get("manager_role").and_then(Value::as_str), normalized_agents.first().and_then(|row| row.get("role")).and_then(Value::as_str).unwrap_or("manager")),
        "agents": normalized_agents,
        "goal": clean_text(payload.get("goal").and_then(Value::as_str), 180),
        "registered_at": now_iso(),
    });
    let crew_id = crew
        .get("crew_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "crews").insert(crew_id, crew.clone());
    Ok(json!({
        "ok": true,
        "crew": crew,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-004.1", semantic_claim("V6-WORKFLOW-004.1")),
    }))
}
