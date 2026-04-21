// SPDX-License-Identifier: Apache-2.0
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use execution_core::{
    apply_governance_json, compose_micro_tasks_json, decompose_goal_json, dispatch_rows_json,
    evaluate_directive_gate_json, evaluate_heroic_gate_json, evaluate_importance_json,
    evaluate_initiative_json, evaluate_route_complexity_json, evaluate_route_decision_json,
    evaluate_route_habit_readiness_json, evaluate_route_json, evaluate_route_match_json,
    evaluate_route_primitives_json, evaluate_route_reflex_match_json, prioritize_attention_json,
    queue_rows_json, run_autoscale_json, run_importer_generic_json_json,
    run_importer_generic_yaml_json, run_importer_infring_json,
    run_importer_web_tooling_signal_json, run_importer_workflow_graph_json, run_inversion_json,
    run_sprint_contract_json, run_workflow, run_workflow_json,
    summarize_dispatch_json, summarize_tasks_json,
};
use std::env;
use std::fs;

fn usage() {
    eprintln!("Usage:");
    eprintln!("  execution_core run --yaml=<payload>");
    eprintln!("  execution_core run --yaml-base64=<base64_payload>");
    eprintln!("  execution_core run --yaml-file=<path>");
    eprintln!("  execution_core decompose --payload=<json_payload>");
    eprintln!("  execution_core decompose --payload-base64=<base64_json_payload>");
    eprintln!("  execution_core decompose --payload-file=<path>");
    eprintln!("  execution_core compose --payload=<json_payload>");
    eprintln!("  execution_core compose --payload-base64=<base64_json_payload>");
    eprintln!("  execution_core compose --payload-file=<path>");
    eprintln!("  execution_core task-summary --payload=<json_payload>");
    eprintln!("  execution_core task-summary --payload-base64=<base64_json_payload>");
    eprintln!("  execution_core task-summary --payload-file=<path>");
    eprintln!("  execution_core dispatch-summary --payload=<json_payload>");
    eprintln!("  execution_core dispatch-summary --payload-base64=<base64_json_payload>");
    eprintln!("  execution_core dispatch-summary --payload-file=<path>");
    eprintln!("  execution_core queue-rows --payload=<json_payload>");
    eprintln!("  execution_core queue-rows --payload-base64=<base64_json_payload>");
    eprintln!("  execution_core queue-rows --payload-file=<path>");
    eprintln!("  execution_core dispatch-rows --payload=<json_payload>");
    eprintln!("  execution_core dispatch-rows --payload-base64=<base64_json_payload>");
    eprintln!("  execution_core dispatch-rows --payload-file=<path>");
    eprintln!("  execution_core directive-gate --payload=<json_payload>");
    eprintln!("  execution_core directive-gate --payload-base64=<base64_json_payload>");
    eprintln!("  execution_core directive-gate --payload-file=<path>");
    eprintln!("  execution_core route-primitives --payload=<json_payload>");
    eprintln!("  execution_core route-primitives --payload-base64=<base64_json_payload>");
    eprintln!("  execution_core route-primitives --payload-file=<path>");
    eprintln!("  execution_core route-match --payload=<json_payload>");
    eprintln!("  execution_core route-match --payload-base64=<base64_json_payload>");
    eprintln!("  execution_core route-match --payload-file=<path>");
    eprintln!("  execution_core route-reflex-match --payload=<json_payload>");
    eprintln!("  execution_core route-reflex-match --payload-base64=<base64_json_payload>");
    eprintln!("  execution_core route-reflex-match --payload-file=<path>");
    eprintln!("  execution_core route-complexity --payload=<json_payload>");
    eprintln!("  execution_core route-complexity --payload-base64=<base64_json_payload>");
    eprintln!("  execution_core route-complexity --payload-file=<path>");
    eprintln!("  execution_core route-evaluate --payload=<json_payload>");
    eprintln!("  execution_core route-evaluate --payload-base64=<base64_json_payload>");
    eprintln!("  execution_core route-evaluate --payload-file=<path>");
    eprintln!("  execution_core route-decision --payload=<json_payload>");
    eprintln!("  execution_core route-decision --payload-base64=<base64_json_payload>");
    eprintln!("  execution_core route-decision --payload-file=<path>");
    eprintln!("  execution_core route-habit-readiness --payload=<json_payload>");
    eprintln!("  execution_core route-habit-readiness --payload-base64=<base64_json_payload>");
    eprintln!("  execution_core route-habit-readiness --payload-file=<path>");
    eprintln!("  execution_core initiative-score --payload=<json_payload>");
    eprintln!("  execution_core initiative-score --payload-base64=<base64_json_payload>");
    eprintln!("  execution_core initiative-score --payload-file=<path>");
    eprintln!("  execution_core initiative-action --payload=<json_payload>");
    eprintln!("  execution_core initiative-action --payload-base64=<base64_json_payload>");
    eprintln!("  execution_core initiative-action --payload-file=<path>");
    eprintln!("  execution_core attention-priority --payload=<json_payload>");
    eprintln!("  execution_core attention-priority --payload-base64=<base64_json_payload>");
    eprintln!("  execution_core attention-priority --payload-file=<path>");
    eprintln!("  execution_core heroic-gate --payload=<json_payload>");
    eprintln!("  execution_core heroic-gate --payload-base64=<base64_json_payload>");
    eprintln!("  execution_core heroic-gate --payload-file=<path>");
    eprintln!("  execution_core apply-governance --payload=<json_payload>");
    eprintln!("  execution_core apply-governance --payload-base64=<base64_json_payload>");
    eprintln!("  execution_core apply-governance --payload-file=<path>");
    eprintln!("  execution_core sprint-contract --payload=<json_payload>");
    eprintln!("  execution_core sprint-contract --payload-base64=<base64_json_payload>");
    eprintln!("  execution_core sprint-contract --payload-file=<path>");
    eprintln!("  execution_core autoscale --payload=<json_payload>");
    eprintln!("  execution_core autoscale --payload-base64=<base64_json_payload>");
    eprintln!("  execution_core autoscale --payload-file=<path>");
    eprintln!("  execution_core inversion --payload=<json_payload>");
    eprintln!("  execution_core inversion --payload-base64=<base64_json_payload>");
    eprintln!("  execution_core inversion --payload-file=<path>");
    eprintln!("  execution_core importer-generic-json --payload=<json_payload>");
    eprintln!("  execution_core importer-generic-json --payload-base64=<base64_json_payload>");
    eprintln!("  execution_core importer-generic-json --payload-file=<path>");
    eprintln!("  execution_core importer-generic-yaml --payload=<json_payload>");
    eprintln!("  execution_core importer-generic-yaml --payload-base64=<base64_json_payload>");
    eprintln!("  execution_core importer-generic-yaml --payload-file=<path>");
    eprintln!("  execution_core importer-infring --payload=<json_payload>");
    eprintln!("  execution_core importer-infring --payload-base64=<base64_json_payload>");
    eprintln!("  execution_core importer-infring --payload-file=<path>");
    eprintln!("  execution_core importer-workflow-graph --payload=<json_payload>");
    eprintln!("  execution_core importer-workflow-graph --payload-base64=<base64_json_payload>");
    eprintln!("  execution_core importer-workflow-graph --payload-file=<path>");
    eprintln!("  execution_core importer-web-tooling-signal --payload=<json_payload>");
    eprintln!("  execution_core importer-web-tooling-signal --payload-base64=<base64_json_payload>");
    eprintln!("  execution_core importer-web-tooling-signal --payload-file=<path>");
    eprintln!("  execution_core demo");
}

fn parse_arg(args: &[String], key: &str) -> Option<String> {
    for arg in args {
        if let Some((k, v)) = arg.split_once('=') {
            if k == key {
                return Some(v.to_string());
            }
        }
    }
    None
}

fn load_yaml(args: &[String]) -> Result<String, String> {
    if let Some(v) = parse_arg(args, "--yaml") {
        return Ok(v);
    }
    if let Some(v) = parse_arg(args, "--yaml-base64") {
        let bytes = BASE64_STANDARD
            .decode(v.as_bytes())
            .map_err(|err| format!("base64_decode_failed:{}", err))?;
        let text = String::from_utf8(bytes).map_err(|err| format!("utf8_decode_failed:{}", err))?;
        return Ok(text);
    }
    if let Some(v) = parse_arg(args, "--yaml-file") {
        let content = fs::read_to_string(v.as_str())
            .map_err(|err| format!("yaml_file_read_failed:{}", err))?;
        return Ok(content);
    }
    Err("missing_yaml_payload".to_string())
}

fn load_payload(args: &[String]) -> Result<String, String> {
    if let Some(v) = parse_arg(args, "--payload") {
        return Ok(v);
    }
    if let Some(v) = parse_arg(args, "--payload-base64") {
        let bytes = BASE64_STANDARD
            .decode(v.as_bytes())
            .map_err(|err| format!("base64_decode_failed:{}", err))?;
        let text = String::from_utf8(bytes).map_err(|err| format!("utf8_decode_failed:{}", err))?;
        return Ok(text);
    }
    if let Some(v) = parse_arg(args, "--payload-file") {
        let content = fs::read_to_string(v.as_str())
            .map_err(|err| format!("payload_file_read_failed:{}", err))?;
        return Ok(content);
    }
    Err("missing_json_payload".to_string())
}
