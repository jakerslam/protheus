// Layer ownership: surface/orchestration (non-canonical orchestration coordination only).
use super::workflow_contracts::{
    registered_workflow_graphs, registered_workflow_validations, tool_contracts_cover_required,
    tool_family_contracts, NormalizedWorkflowGraph, ToolFamilyContract, WorkflowValidation,
    REQUIRED_TELEMETRY_STREAMS, REQUIRED_TERMINAL_STATES, REQUIRED_TOOL_FAMILIES,
};
use super::workflow_runtime::{run_registered_replay_fixtures, workflow_runtime_contract_ok};
use super::workflow_runtime_types::WorkflowReplayReport;
use serde_json::{json, Value};
use std::fs;
use std::io;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

const DEFAULT_OUT_PATH: &str =
    "core/local/artifacts/orchestration_workflow_contract_guard_current.json";
const DEFAULT_GRAPH_OUT_PATH: &str =
    "local/state/ops/orchestration/workflow_contract_graphs_current.json";
const DEFAULT_REPORT_PATH: &str =
    "local/workspace/reports/ORCHESTRATION_WORKFLOW_CONTRACT_GUARD_CURRENT.md";
const FORMAT_POLICY_PATH: &str = "docs/workspace/workflow_json_format_policy.md";
const ENFORCER_PATH: &str = "docs/workspace/codex_enforcer.md";
const PARITY_MAP_PATH: &str = "docs/workspace/orchestration_control_plane_parity_map.md";

pub fn run_workflow_contract_guard(args: &[String]) -> i32 {
    let strict = flag_value(args, "--strict").unwrap_or_else(|| "0".to_string()) == "1";
    let out_path = flag_value(args, "--out").unwrap_or_else(|| DEFAULT_OUT_PATH.to_string());
    let graph_out =
        flag_value(args, "--graph-out").unwrap_or_else(|| DEFAULT_GRAPH_OUT_PATH.to_string());
    let report_path =
        flag_value(args, "--report").unwrap_or_else(|| DEFAULT_REPORT_PATH.to_string());

    let validations = registered_workflow_validations();
    let graphs = registered_workflow_graphs();
    let tool_contracts = tool_family_contracts();
    let replay_reports = run_registered_replay_fixtures();
    let checks = build_checks(&validations, &graphs, &tool_contracts, &replay_reports);
    let ok = checks
        .iter()
        .all(|row| row.get("ok").and_then(Value::as_bool).unwrap_or(false));
    let graph_artifact = json!({
        "type": "orchestration_workflow_contract_graphs",
        "schema_version": 1,
        "generated_unix_seconds": now_unix_seconds(),
        "graphs": graphs,
        "tool_family_contracts": tool_contracts,
        "runtime_replay_reports": replay_reports,
    });
    let report = json!({
        "type": "orchestration_workflow_contract_guard",
        "schema_version": 1,
        "generated_unix_seconds": now_unix_seconds(),
        "ok": ok,
        "checks": checks,
        "summary": {
            "workflow_count": validations.len(),
            "valid_workflows": validations.iter().filter(|row| row.ok).count(),
            "tool_family_contracts": REQUIRED_TOOL_FAMILIES.len(),
            "runtime_replay_fixtures": replay_reports.len(),
            "system_chat_injection_allowed": false,
            "graph_artifact_path": graph_out,
        },
        "validations": validations,
        "artifact_paths": {
            "graphs": graph_out,
            "format_policy": FORMAT_POLICY_PATH,
            "enforcer": ENFORCER_PATH,
            "parity_map": PARITY_MAP_PATH
        }
    });
    let markdown = format!(
        "# Orchestration Workflow Contract Guard\n\n- ok: {ok}\n- workflows: {}\n- graph_artifact: {graph_out}\n",
        validations.len()
    );
    let wrote = write_json(&graph_out, &graph_artifact)
        .and_then(|_| write_json(&out_path, &report))
        .and_then(|_| write_text(&report_path, &markdown))
        .is_ok();
    println!(
        "{}",
        serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string())
    );
    if strict && (!ok || !wrote) {
        return 1;
    }
    0
}

fn build_checks(
    validations: &[WorkflowValidation],
    graphs: &[NormalizedWorkflowGraph],
    tool_contracts: &[ToolFamilyContract],
    replay_reports: &[WorkflowReplayReport],
) -> Vec<Value> {
    let format_policy = read_text(FORMAT_POLICY_PATH);
    let enforcer = read_text(ENFORCER_PATH);
    let parity_map = read_text(PARITY_MAP_PATH);
    vec![
        json!({"id": "workflow_json_compiles_to_typed_graphs", "ok": !graphs.is_empty() && validations.iter().all(|row| row.ok), "detail": format!("graphs={};workflows={}", graphs.len(), validations.len())}),
        json!({"id": "structured_gate_contract", "ok": graphs.iter().all(|row| row.gate_contract.allowed_input_shapes == ["multiple_choice", "text_input"] && row.gate_contract.resume_token_required), "detail": "gates expose only multiple_choice or text_input with resume tokens"}),
        json!({"id": "tool_family_contracts_complete", "ok": tool_contracts_cover_required(tool_contracts), "detail": format!("families={}", tool_contracts.len())}),
        json!({"id": "run_budget_and_terminal_contract", "ok": graphs.iter().all(run_budget_ok), "detail": "terminal states and bounded run budgets required"}),
        json!({"id": "telemetry_stream_separation_contract", "ok": graphs.iter().all(telemetry_ok), "detail": "workflow_state, agent_internal_notes, tool_trace, eval_trace, and final_answer streams required"}),
        json!({"id": "no_system_chat_injection_contract", "ok": graphs.iter().all(|row| row.visible_chat_policy == "llm_final_only_no_system_injection") && enforcer.contains("System-authored fallback text is prohibited in visible chat"), "detail": "visible chat source is llm final output only"}),
        json!({"id": "workflow_runtime_replay_contract", "ok": workflow_runtime_contract_ok(replay_reports), "detail": format!("fixtures={}", replay_reports.len())}),
        json!({"id": "workflow_runtime_budget_contract", "ok": replay_reports.iter().all(runtime_budget_ok), "detail": "runtime replays stay under stage/model/tool/token budgets and keep loop guard active"}),
        json!({"id": "workflow_runtime_inspector_contract", "ok": replay_reports.iter().all(runtime_inspector_ok), "detail": "workflow_state, agent_internal_notes, tool_trace, eval_trace, and final_answer are separated from visible chat"}),
        json!({"id": "workflow_runtime_graph_binding_contract", "ok": replay_reports.iter().all(|row| !row.graph_hash.is_empty() && row.inspector.selected_graph_source == "orchestration_typed_graph_v1"), "detail": "runtime selection consumes typed orchestration graph bindings"}),
        json!({"id": "workflow_format_policy_contract", "ok": format_policy.contains("typed_execution_contract") && format_policy.contains("llm_final_only_no_system_injection"), "detail": FORMAT_POLICY_PATH}),
        json!({"id": "control_plane_parity_map_contract", "ok": all_present(&parity_map, &["OpenHands", "OpenFang", "Infring", "surface/orchestration/src", "event-sourced action/observation"]), "detail": PARITY_MAP_PATH}),
    ]
}

fn runtime_budget_ok(report: &WorkflowReplayReport) -> bool {
    report.budget.loop_guard_active
        && !report.budget.budget_exceeded
        && !report.budget.loop_signature_repeated
        && report.budget.stages_seen <= report.budget.max_stages
        && report.budget.model_turns_seen <= report.budget.max_model_turns
        && report.budget.tool_calls_seen <= report.budget.max_tool_calls
        && report.budget.estimated_tokens_seen <= report.budget.token_budget
}

fn runtime_inspector_ok(report: &WorkflowReplayReport) -> bool {
    REQUIRED_TELEMETRY_STREAMS
        .iter()
        .all(|stream| report.inspector.trace_streams.contains_key(*stream))
        && !report.inspector.system_chat_injection_allowed
        && report.inspector.visible_chat_source == "final_answer_stream_only"
        && report
            .events
            .iter()
            .all(|event| event.stream == "final_answer" || !event.visible_chat_eligible)
}

fn run_budget_ok(graph: &NormalizedWorkflowGraph) -> bool {
    REQUIRED_TERMINAL_STATES
        .iter()
        .all(|state| graph.terminal_states.iter().any(|v| v == state))
        && graph.run_budgets.max_stages > 0
        && graph.run_budgets.max_model_turns > 0
        && graph.run_budgets.max_tool_calls > 0
        && graph.run_budgets.token_budget > 0
}

fn telemetry_ok(graph: &NormalizedWorkflowGraph) -> bool {
    REQUIRED_TELEMETRY_STREAMS
        .iter()
        .all(|stream| graph.telemetry_streams.iter().any(|v| v == stream))
}

fn all_present(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().all(|needle| haystack.contains(needle))
}

fn flag_value(args: &[String], key: &str) -> Option<String> {
    let inline = format!("{key}=");
    for (idx, arg) in args.iter().enumerate() {
        if let Some(value) = arg.strip_prefix(&inline) {
            return Some(value.to_string());
        }
        if arg == key {
            return args.get(idx + 1).cloned();
        }
    }
    None
}

fn now_unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn read_text(path: &str) -> String {
    fs::read_to_string(path).unwrap_or_default()
}

fn write_json(path: &str, value: &Value) -> io::Result<()> {
    if let Some(parent) = Path::new(path).parent() {
        fs::create_dir_all(parent)?;
    }
    let payload = serde_json::to_string_pretty(value).unwrap_or_else(|_| "{}".to_string());
    fs::write(path, format!("{payload}\n"))
}

fn write_text(path: &str, body: &str) -> io::Result<()> {
    if let Some(parent) = Path::new(path).parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, body)
}
