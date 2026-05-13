// Layer ownership: orchestration (non-canonical orchestration coordination only).
use super::workflow_composition_contracts::workflow_composition_contract_report;
use super::workflow_contracts::{
    registered_workflow_graphs, registered_workflow_validations, tool_contracts_cover_required,
    tool_family_contracts, workflow_registry_contract_ok, NormalizedWorkflowGraph,
    ToolFamilyContract, WorkflowValidation, REQUIRED_TELEMETRY_STREAMS, REQUIRED_TERMINAL_STATES,
    REQUIRED_TOOL_FAMILIES, WORKFLOW_CONTRACT_SCHEMA_VERSION,
    WORKFLOW_SOURCE_OF_TRUTH_SCHEMA_VERSION,
};
use super::workflow_runtime::{
    run_registered_replay_fixtures, workflow_runtime_contract_ok,
    workflow_runtime_terminal_outcome_ok,
};
use super::workflow_runtime_types::WorkflowReplayReport;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

const DEFAULT_OUT_PATH: &str =
    "core/local/artifacts/orchestration_workflow_contract_guard_current.json";
const DEFAULT_GRAPH_OUT_PATH: &str =
    "local/state/ops/orchestration/workflow_contract_graphs_current.json";
const DEFAULT_COMPOSITION_LEDGER_OUT_PATH: &str =
    "local/state/ops/orchestration/workflow_composition_ledger_current.json";
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
    let composition_ledger_out = flag_value(args, "--composition-ledger-out")
        .unwrap_or_else(|| DEFAULT_COMPOSITION_LEDGER_OUT_PATH.to_string());
    let report_path =
        flag_value(args, "--report").unwrap_or_else(|| DEFAULT_REPORT_PATH.to_string());

    let validations = registered_workflow_validations();
    let graphs = registered_workflow_graphs();
    let tool_contracts = tool_family_contracts();
    let replay_reports = run_registered_replay_fixtures();
    let composition_ledger = workflow_composition_ledger_artifact(&graphs, &validations);
    let composition_report = workflow_composition_contract_report();
    let checks = build_checks(
        &validations,
        &graphs,
        &tool_contracts,
        &replay_reports,
        &composition_report,
    );
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
        "workflow_composition_contracts": composition_report,
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
            "assistant_response_workflows": graphs.iter().filter(|row| row.workflow_role == "assistant_response_workflow").count(),
            "assimilation_workflow_templates": graphs.iter().filter(|row| row.workflow_role == "assimilation_workflow_template").count(),
            "assimilation_subtemplates": graphs.iter().map(|row| row.subtemplate_count).sum::<usize>(),
            "max_workflow_level": graphs.iter().map(|row| row.primitive_level).max().unwrap_or(0),
            "composite_workflows": graphs.iter().filter(|row| row.primitive_level > 0).count(),
            "workflow_levels": graphs
                .iter()
                .map(|row| format!("{}:{}", row.workflow_id, row.primitive_level))
                .collect::<Vec<_>>(),
            "tool_family_contracts": REQUIRED_TOOL_FAMILIES.len(),
            "runtime_replay_fixtures": replay_reports.len(),
            "official_workflows": graphs.iter().filter(|row| row.workflow_tier == "official").count(),
            "lab_workflows": graphs.iter().filter(|row| row.workflow_tier == "lab").count(),
            "system_chat_injection_allowed": false,
            "graph_artifact_path": graph_out,
            "composition_ledger_path": composition_ledger_out,
        },
        "validations": validations,
        "composition_contracts": composition_report,
        "artifact_paths": {
            "graphs": graph_out,
            "composition_ledger": composition_ledger_out,
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
        .and_then(|_| write_json(&composition_ledger_out, &composition_ledger))
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
    composition_report: &Value,
) -> Vec<Value> {
    let format_policy = read_text(FORMAT_POLICY_PATH);
    let enforcer = read_text(ENFORCER_PATH);
    let parity_map = read_text(PARITY_MAP_PATH);
    vec![
        json!({"id": "workflow_json_compiles_to_typed_graphs", "ok": !graphs.is_empty() && validations.iter().all(|row| row.ok), "detail": format!("graphs={};workflows={}", graphs.len(), validations.len())}),
        json!({"id": "workflow_registry_tier_contract", "ok": workflow_registry_contract_ok() && graphs.iter().all(workflow_registry_graph_ok), "detail": "official workflows are runtime-selectable; lab/framework workflows are parseable comparison profiles only"}),
        json!({"id": "workflow_role_typing_contract", "ok": graphs.iter().all(workflow_role_ok), "detail": "workflow_role must be assistant_response_workflow or assimilation_workflow_template"}),
        json!({"id": "assistant_workflow_presence_contract", "ok": graphs.iter().any(|row| row.workflow_role == "assistant_response_workflow"), "detail": "at least one assistant-response workflow must remain available"}),
        json!({"id": "assimilation_template_role_contract", "ok": graphs.iter().filter(|row| assimilation_template_id(row)).all(|row| row.workflow_role == "assimilation_workflow_template"), "detail": "assimilation template specs must not masquerade as normal assistant response workflows; non-assimilation ForgeCode coding workflows may use assistant_response_workflow"}),
        json!({"id": "assimilation_template_structure_contract", "ok": graphs.iter().all(assimilation_template_structure_ok), "detail": "assimilation templates must declare at least one subtemplate and assistant-response workflows must not carry assimilation subtemplates"}),
        json!({"id": "structured_gate_contract", "ok": graphs.iter().all(|row| row.gate_contract.allowed_input_shapes == ["multiple_choice", "text_input"] && row.gate_contract.resume_token_required), "detail": "gates expose only multiple_choice or text_input with resume tokens"}),
        json!({"id": "tool_family_contracts_complete", "ok": tool_contracts_cover_required(tool_contracts), "detail": format!("families={}", tool_contracts.len())}),
        json!({"id": "run_budget_and_terminal_contract", "ok": graphs.iter().all(run_budget_ok), "detail": "terminal states and bounded run budgets required"}),
        json!({"id": "telemetry_stream_separation_contract", "ok": graphs.iter().all(telemetry_ok), "detail": "workflow_state, agent_internal_notes, tool_trace, eval_trace, and final_answer streams required"}),
        json!({"id": "no_system_chat_injection_contract", "ok": graphs.iter().all(|row| row.visible_chat_policy == "llm_final_only_no_system_injection") && enforcer.contains("System-authored fallback text is prohibited in visible chat"), "detail": "visible chat source is llm final output only"}),
        json!({"id": "final_output_contract_binding", "ok": graphs.iter().all(final_output_contract_ok), "detail": "typed workflow graphs carry a CD-derived final-output contract for synthesis handoff"}),
        json!({"id": "workflow_runtime_replay_contract", "ok": workflow_runtime_contract_ok(replay_reports), "detail": format!("fixtures={}", replay_reports.len())}),
        json!({"id": "workflow_runtime_budget_contract", "ok": replay_reports.iter().all(runtime_budget_ok), "detail": "runtime replays stay under stage/model/tool/token budgets and keep loop guard active"}),
        json!({"id": "workflow_runtime_inspector_contract", "ok": replay_reports.iter().all(runtime_inspector_ok), "detail": "workflow_state, agent_internal_notes, tool_trace, eval_trace, and final_answer are separated from visible chat"}),
        json!({"id": "workflow_runtime_graph_binding_contract", "ok": replay_reports.iter().all(|row| !row.graph_hash.is_empty() && row.inspector.selected_graph_source == "json_workflow_source_of_truth_v1"), "detail": "runtime selection consumes JSON source-of-truth orchestration graph bindings"}),
        json!({"id": "workflow_json_source_metadata_contract", "ok": graphs.iter().all(graph_json_source_metadata_ok), "detail": "typed graphs carry workflow id, source JSON path, contract schema version, and graph hash metadata"}),
        json!({"id": "workflow_composition_metadata_contract", "ok": workflow_composition_metadata_ok(graphs), "detail": "primitive workflow levels and composition references are present and valid"}),
        json!({"id": "workflow_runtime_registered_json_source_contract", "ok": replay_reports.iter().all(runtime_registered_json_source_ok), "detail": "runtime telemetry exposes selected workflow id, source JSON path, contract schema version, and graph hash from a registered JSON workflow"}),
        json!({"id": "workflow_cd_composition_contract", "ok": composition_report.get("ok").and_then(Value::as_bool).unwrap_or(false), "detail": "workflow CDs declare primitive/composite boundaries, typed child workflow calls, and exactly one terminal artifact return"}),
        json!({"id": "workflow_format_policy_contract", "ok": all_present(&format_policy, &["workflow_source_of_truth_contract", "typed_execution_contract", "burnable CD", "json_workflow_spec", "llm_final_only_no_system_injection"]), "detail": FORMAT_POLICY_PATH}),
        json!({"id": "control_plane_parity_map_contract", "ok": all_present(&parity_map, &["OpenHands", "OpenFang", "Infring", "orchestration/src", "event-sourced action/observation"]), "detail": PARITY_MAP_PATH}),
    ]
}

fn workflow_composition_ledger_artifact(
    graphs: &[NormalizedWorkflowGraph],
    validations: &[WorkflowValidation],
) -> Value {
    let mut parent_map: HashMap<String, Vec<String>> = HashMap::new();
    let mut validation_map: HashMap<String, &WorkflowValidation> = HashMap::new();
    for validation in validations {
        validation_map.insert(validation.workflow_id.clone(), validation);
    }
    for row in graphs {
        for child_id in &row.composed_of_workflow_ids {
            parent_map
                .entry(child_id.clone())
                .or_default()
                .push(row.workflow_id.clone());
        }
    }

    let mut rows: Vec<(String, Value)> = graphs
        .iter()
        .map(|row| {
            let validation = validation_map.get(&row.workflow_id);
            let composition_errors: Vec<String> = validation
                .map(|validation| {
                    validation
                        .errors
                        .iter()
                        .filter(|error| is_composition_error(error))
                        .cloned()
                        .collect()
                })
                .unwrap_or_default();
            let mut parents = parent_map
                .get(&row.workflow_id)
                .cloned()
                .unwrap_or_default();
            parents.sort_unstable();
            let mut children = row.composed_of_workflow_ids.clone();
            children.sort_unstable();
            (
                row.workflow_id.clone(),
                json!({
                    "workflow_id": row.workflow_id,
                    "workflow_tier": row.workflow_tier,
                    "runtime_selectable": row.runtime_selectable,
                    "promotion_status": row.promotion_status,
                    "primitive_level": row.primitive_level,
                    "is_composite_workflow": row.primitive_level > 0,
                    "composed_of_workflow_ids": children,
                    "parent_workflow_ids": parents,
                    "subtemplate_count": row.subtemplate_count,
                    "validation_ok": validation.map(|row| row.ok).unwrap_or(false),
                    "validation_error_count": validation.map(|row| row.errors.len()).unwrap_or(0),
                    "composition_errors": composition_errors,
                }),
            )
        })
        .collect();
    rows.sort_by(|left, right| left.0.cmp(&right.0));

    let workflow_rows = rows.into_iter().map(|(_, row)| row).collect::<Vec<_>>();

    json!({
        "type": "orchestration_workflow_composition_ledger",
        "schema_version": 1,
        "generated_unix_seconds": now_unix_seconds(),
        "workflows": workflow_rows,
        "workflow_count": graphs.len(),
        "primitive_workflow_count": graphs.iter().filter(|row| row.primitive_level == 0).count(),
        "composite_workflow_count": graphs.iter().filter(|row| row.primitive_level > 0).count(),
        "max_primitive_level": graphs.iter().map(|row| row.primitive_level).max().unwrap_or(0),
        "composition_graph_summary": {
            "nodes": graphs.len(),
            "edges": graphs
                .iter()
                .map(|row| row.composed_of_workflow_ids.len())
                .sum::<usize>(),
        }
    })
}

fn is_composition_error(error: &str) -> bool {
    error.starts_with("workflow_composition_")
        || error.starts_with("workflow_primitive_level_")
        || error.starts_with("empty_composed_workflow_id")
        || error.starts_with("duplicate_composed_workflow_id")
        || error.starts_with("unknown_composed_workflow_id")
        || error.starts_with("workflow_composition_self_reference")
}

fn workflow_role_ok(graph: &NormalizedWorkflowGraph) -> bool {
    graph.workflow_type == "control_plane_orchestration_workflow"
        && matches!(
            graph.workflow_role.as_str(),
            "assistant_response_workflow" | "assimilation_workflow_template"
        )
        && if graph.workflow_role == "assimilation_workflow_template" {
            graph.workflow_id.contains("assimilation")
                || graph.workflow_id.contains("synthesis")
                || graph.workflow_id.contains("codex")
                || graph.workflow_id.contains("forgecode")
                || graph.workflow_id.contains("openhands")
        } else {
            !graph.workflow_id.contains("assimilation")
        }
}

fn workflow_registry_graph_ok(graph: &NormalizedWorkflowGraph) -> bool {
    match graph.workflow_tier.as_str() {
        "official" => {
            graph.runtime_selectable
                && graph.promotion_status == "official"
                && graph
                    .source_json_path
                    .starts_with("orchestration/src/control_plane/workflows/official/")
        }
        "lab" => {
            !graph.runtime_selectable
                && matches!(graph.promotion_status.as_str(), "lab" | "candidate")
                && graph
                    .source_json_path
                    .starts_with("orchestration/src/control_plane/workflows/lab/")
        }
        _ => false,
    }
}

fn assimilation_template_structure_ok(graph: &NormalizedWorkflowGraph) -> bool {
    if graph.workflow_role == "assimilation_workflow_template" {
        graph.subtemplate_count > 0
    } else {
        graph.subtemplate_count == 0
    }
}

fn assimilation_template_id(graph: &NormalizedWorkflowGraph) -> bool {
    graph.workflow_id.contains("assimilation")
        || graph.workflow_id == "codex_tooling_synthesis"
        || graph.workflow_id == "forgecode_agent_composition"
        || graph.workflow_id == "openhands_control_plane_assimilation"
}

fn workflow_composition_metadata_ok(graphs: &[NormalizedWorkflowGraph]) -> bool {
    let level_by_id = graphs
        .iter()
        .map(|graph| (graph.workflow_id.as_str(), graph.primitive_level))
        .collect::<HashMap<_, _>>();
    graphs.iter().all(|graph| {
        if graph.primitive_level == 0 {
            return graph.composed_of_workflow_ids.is_empty();
        }
        if graph.composed_of_workflow_ids.is_empty() {
            return true;
        }
        let child_levels = graph
            .composed_of_workflow_ids
            .iter()
            .map(|child_id| level_by_id.get(child_id.as_str()).copied())
            .collect::<Option<Vec<_>>>();
        let Some(child_levels) = child_levels else {
            return false;
        };
        graph.primitive_level
            == child_levels
                .into_iter()
                .max()
                .map(|level| level + 1)
                .unwrap_or(0)
    })
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

fn final_output_contract_ok(graph: &NormalizedWorkflowGraph) -> bool {
    !graph.final_response_policy.trim().is_empty()
        && graph
            .final_output_contract
            .get("schema_version")
            .and_then(Value::as_str)
            .is_some()
        && graph
            .final_output_contract
            .get("source")
            .and_then(Value::as_str)
            .is_some()
}

fn runtime_inspector_ok(report: &WorkflowReplayReport) -> bool {
    REQUIRED_TELEMETRY_STREAMS
        .iter()
        .all(|stream| report.inspector.trace_streams.contains_key(*stream))
        && !report.inspector.system_chat_injection_allowed
        && report.inspector.visible_chat_source == "final_answer_stream_only"
        && workflow_runtime_terminal_outcome_ok(report)
        && report
            .events
            .iter()
            .all(|event| event.stream == "final_answer" || !event.visible_chat_eligible)
}

fn graph_json_source_metadata_ok(graph: &NormalizedWorkflowGraph) -> bool {
    !graph.workflow_id.trim().is_empty()
        && graph
            .source_json_path
            .starts_with("orchestration/src/control_plane/workflows/")
        && graph.source_json_path.ends_with(".workflow.json")
        && graph.contract_schema_version == WORKFLOW_CONTRACT_SCHEMA_VERSION
        && graph.source_of_truth_schema_version == WORKFLOW_SOURCE_OF_TRUTH_SCHEMA_VERSION
        && graph.interaction_source == "json_workflow_spec"
        && graph.rust_reader_role == "validate_execute_trace_only"
        && !graph.hardcoded_interaction_behavior_allowed
}

fn runtime_registered_json_source_ok(report: &WorkflowReplayReport) -> bool {
    !report.workflow_id.trim().is_empty()
        && !report.graph_hash.trim().is_empty()
        && report
            .source_json_path
            .starts_with("orchestration/src/control_plane/workflows/")
        && report.source_json_path.ends_with(".workflow.json")
        && report.contract_schema_version == WORKFLOW_CONTRACT_SCHEMA_VERSION
        && report.inspector.workflow_id == report.workflow_id
        && report.inspector.graph_hash == report.graph_hash
        && report.inspector.source_json_path == report.source_json_path
        && report.inspector.contract_schema_version == report.contract_schema_version
        && report.inspector.source_of_truth_schema_version
            == WORKFLOW_SOURCE_OF_TRUTH_SCHEMA_VERSION
        && report.inspector.interaction_source == "json_workflow_spec"
        && report.inspector.rust_reader_role == "validate_execute_trace_only"
        && !report.inspector.hardcoded_interaction_behavior_allowed
        && report.inspector.selected_graph_source == "json_workflow_source_of_truth_v1"
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
