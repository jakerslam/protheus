use serde_json::{json, Value};
use std::collections::BTreeMap;

#[path = "eval_research_gate_diagnostics.rs"]
mod eval_research_gate_diagnostics;
#[path = "eval_research_golden_report.rs"]
mod eval_research_golden_report;
#[path = "eval_research_golden_scoring.rs"]
mod eval_research_golden_scoring;
#[path = "eval_research_golden_utils.rs"]
mod eval_research_golden_utils;

use eval_research_gate_diagnostics::{
    failure_boundary, gate_transition_diagnostics, gate_transition_rate_rows,
};
use eval_research_golden_report::{append_failure_events, markdown_report};
use eval_research_golden_scoring::{
    dimension_average_rows, gate_rate_rows, grade_case, response_diagnostics,
};
use eval_research_golden_utils::*;
use std::env;

const DEFAULT_CASES_PATH: &str = "validation/evals/fixtures/research_golden_dataset_v1.json";
const DEFAULT_OUT_PATH: &str = "core/local/artifacts/research_golden_current.json";
const DEFAULT_OUT_LATEST_PATH: &str = "artifacts/research_golden_latest.json";
const DEFAULT_MARKDOWN_PATH: &str = "local/workspace/reports/RESEARCH_GOLDEN_CURRENT.md";
const DEFAULT_FAILURES_PATH: &str = "local/state/ops/research_golden/failures.jsonl";
const DEFAULT_AGENT_ID: &str = "agent-5bc62b0875a9";

pub fn run_research_golden(args: &[String]) -> i32 {
    let strict = parse_bool_flag(args, "strict", false);
    let live = parse_bool_flag(args, "live", false);
    let allow_remote = parse_bool_flag(args, "allow-remote", false);
    let confirm_pending_tool = parse_bool_flag(args, "confirm-pending-tool", false);
    let cases_path = parse_flag(args, "cases").unwrap_or_else(|| DEFAULT_CASES_PATH.to_string());
    let responses_path = parse_flag(args, "responses");
    let out_path = parse_flag(args, "out").unwrap_or_else(|| DEFAULT_OUT_PATH.to_string());
    let out_latest_path =
        parse_flag(args, "out-latest").unwrap_or_else(|| DEFAULT_OUT_LATEST_PATH.to_string());
    let markdown_path =
        parse_flag(args, "out-markdown").unwrap_or_else(|| DEFAULT_MARKDOWN_PATH.to_string());
    let failures_path =
        parse_flag(args, "failures-out").unwrap_or_else(|| DEFAULT_FAILURES_PATH.to_string());
    let agent_id = normalize_agent_id(
        &parse_flag(args, "agent-id").unwrap_or_else(|| DEFAULT_AGENT_ID.to_string()),
    );
    let fresh_agent_per_case = parse_bool_flag(args, "fresh-agent-per-case", false);
    let cleanup_fresh_agents = parse_bool_flag(args, "cleanup-fresh-agents", true);
    let fresh_agent_model = parse_flag(args, "fresh-agent-model")
        .or_else(|| env::var("INFRING_RESEARCH_GOLDEN_FRESH_MODEL").ok())
        .map(|raw| clean_text(&raw, 240))
        .filter(|raw| !raw.is_empty());
    let base_url =
        parse_flag(args, "base-url").unwrap_or_else(|| "http://127.0.0.1:4173".to_string());
    let timeout_seconds = parse_u64_flag(args, "timeout-seconds", 45).clamp(1, 600);
    let limit = parse_u64_flag(args, "limit", u64::MAX) as usize;

    let input = read_json(&cases_path);
    let cases = input
        .get("cases")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let thresholds = input
        .get("reliability_thresholds")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let scoring_contract = input
        .get("scoring_contract")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let min_cases = u64_at(&thresholds, &["min_cases_for_reliability_claim"], 20);
    let workflow_gate_pass_min = f64_at(&thresholds, &["workflow_gate_pass_min"], 0.95);
    let research_success_min = f64_at(&thresholds, &["research_success_min"], 0.85);
    let pass_score = u64_at(&scoring_contract, &["pass_score"], 85);
    let excellent_score = u64_at(&scoring_contract, &["excellent_score"], 95);
    let responses_by_case = responses_path
        .as_deref()
        .map(load_responses_by_case)
        .unwrap_or_default();

    let mut setup_failures = Vec::new();
    if live && !allow_remote && !is_local_dashboard_url(&base_url) {
        setup_failures.push("remote_dashboard_url_requires_allow_remote".to_string());
    }
    if !live && responses_by_case.is_empty() {
        setup_failures.push("offline_mode_requires_responses_fixture".to_string());
    }

    let mut rows = Vec::new();
    let mut failure_events = Vec::new();
    let mut gate_pass_counts: BTreeMap<String, u64> = BTreeMap::new();
    let mut gate_total_counts: BTreeMap<String, u64> = BTreeMap::new();
    let mut transition_pass_counts: BTreeMap<String, u64> = BTreeMap::new();
    let mut transition_total_counts: BTreeMap<String, u64> = BTreeMap::new();
    let mut dimension_totals: BTreeMap<String, u64> = BTreeMap::new();
    let mut passed_cases = 0_u64;
    let mut excellent_cases = 0_u64;
    let mut total_score = 0_u64;
    let mut empty_responses = 0_u64;
    let mut raw_tool_leaks = 0_u64;
    let mut tool_choice_final_responses = 0_u64;
    let mut unsupported_claims = 0_u64;

    for case in cases.iter().take(limit) {
        let case_id = str_at(case, &["id"], "unknown_case");
        let prompt = str_at(case, &["prompt"], "");
        let mut case_agent_id = agent_id.clone();
        let mut case_setup_failures = setup_failures.clone();
        if live && setup_failures.is_empty() && fresh_agent_per_case {
            match create_live_agent(
                &base_url,
                case_id.as_str(),
                fresh_agent_model.as_deref(),
                timeout_seconds,
            ) {
                Some(created_agent_id) => case_agent_id = created_agent_id,
                None => case_setup_failures.push("fresh_agent_create_failed".to_string()),
            }
        }
        let source_payload = responses_by_case
            .get(&case_id)
            .cloned()
            .unwrap_or_else(|| json!({}));
        let initial_payload = if live && case_setup_failures.is_empty() {
            post_agent_message(
                &base_url,
                &case_agent_id,
                &json!({ "message": prompt }),
                timeout_seconds,
            )
        } else {
            response_sequence_payload(&source_payload, 0).unwrap_or(source_payload.clone())
        };
        let initial_pending_tool_confirmation =
            payload_has_pending_tool_confirmation(&initial_payload);
        let mut payload = initial_payload.clone();
        let mut confirmation_payload_used = false;
        let mut confirmation_sent = false;
        let mut confirmation_fixture_used = false;
        if confirm_pending_tool
            && case_setup_failures.is_empty()
            && initial_pending_tool_confirmation
        {
            if live {
                confirmation_sent = true;
                payload = post_agent_message(
                    &base_url,
                    &case_agent_id,
                    &json!({ "message": "confirm" }),
                    timeout_seconds,
                );
                confirmation_payload_used = true;
            } else if let Some(confirmed_payload) = response_sequence_payload(&source_payload, 1) {
                payload = confirmed_payload;
                confirmation_payload_used = true;
                confirmation_fixture_used = true;
            }
        }
        if live && fresh_agent_per_case && cleanup_fresh_agents && case_agent_id != agent_id {
            let _ = delete_live_agent(&base_url, &case_agent_id, timeout_seconds);
        }
        let transition_diagnostics = gate_transition_diagnostics_for_sequence(
            case,
            &initial_payload,
            &payload,
            confirmation_payload_used,
        );
        let lifecycle_gate_path_complete =
            transition_first_failed_checkpoint(&transition_diagnostics).is_none();
        let grade = grade_case(case, &payload, pass_score, excellent_score);
        let mut case_failures = grade.failures.clone();
        if let Some(checkpoint) = transition_first_failed_checkpoint(&transition_diagnostics) {
            case_failures.push(format!("research_lifecycle_gate_failed:{checkpoint}"));
        }
        case_failures.sort();
        case_failures.dedup();
        let case_pass =
            grade.pass && lifecycle_gate_path_complete && case_setup_failures.is_empty();
        let case_excellent =
            grade.excellent && lifecycle_gate_path_complete && case_setup_failures.is_empty();
        let initial_response_text = assistant_text(&initial_payload);
        record_gate_counts(&grade.gates, &mut gate_total_counts, &mut gate_pass_counts);
        record_checkpoint_counts(
            &transition_diagnostics,
            &mut transition_total_counts,
            &mut transition_pass_counts,
        );
        for (dimension, score) in grade.dimension_scores.iter() {
            *dimension_totals.entry(dimension.clone()).or_insert(0) += *score;
        }
        total_score = total_score.saturating_add(grade.score);
        if case_pass {
            passed_cases = passed_cases.saturating_add(1);
        }
        if case_excellent {
            excellent_cases = excellent_cases.saturating_add(1);
        }
        if grade.empty_response {
            empty_responses = empty_responses.saturating_add(1);
        }
        if grade.raw_tool_leak {
            raw_tool_leaks = raw_tool_leaks.saturating_add(1);
        }
        if grade.tool_choice_final_response {
            tool_choice_final_responses = tool_choice_final_responses.saturating_add(1);
        }
        if grade.unsupported_claim {
            unsupported_claims = unsupported_claims.saturating_add(1);
        }
        append_failure_events(
            &mut failure_events,
            case_id.as_str(),
            prompt.as_str(),
            case_agent_id.as_str(),
            live,
            &grade.response_text,
            &case_failures,
            &case_setup_failures,
        );
        rows.push(json!({
            "case_id": case_id,
            "category": str_at(case, &["category"], "unknown"),
            "prompt_preview": clean_text(&prompt, 320),
            "score": grade.score,
            "score_pass": grade.pass,
            "pass": case_pass,
            "excellent": case_excellent,
            "lifecycle_gate_path_complete": lifecycle_gate_path_complete,
            "agent_id": case_agent_id,
            "gates": grade.gates,
            "dimension_scores": grade.dimension_scores,
            "failures": case_failures,
            "response_preview": clean_text(&grade.response_text, 500),
            "response_diagnostics": response_diagnostics(&payload, &grade.response_text),
            "gate_transition_diagnostics": transition_diagnostics,
            "turn_sequence": {
                "confirm_pending_tool": confirm_pending_tool,
                "initial_pending_tool_confirmation": initial_pending_tool_confirmation,
                "confirmation_sent": confirmation_sent,
                "confirmation_fixture_used": confirmation_fixture_used,
                "confirmation_payload_used": confirmation_payload_used,
                "final_payload_source": if confirmation_payload_used {
                    "confirmation_turn"
                } else {
                    "initial_turn"
                },
                "initial_response_diagnostics": response_diagnostics(
                    &initial_payload,
                    &initial_response_text
                ),
                "initial_gate_transition_diagnostics": gate_transition_diagnostics(
                    case,
                    &initial_payload
                )
            },
        }));
    }

    let total_cases = rows.len() as u64;
    let avg_score = ratio(total_score, total_cases);
    let research_success_rate = ratio(passed_cases, total_cases);
    let excellent_rate = ratio(excellent_cases, total_cases);
    let gate_rates = gate_rate_rows(
        &gate_total_counts,
        &gate_pass_counts,
        workflow_gate_pass_min,
    );
    let gate_path_ok = gate_rates
        .iter()
        .all(|row| row.get("ok").and_then(Value::as_bool).unwrap_or(false));
    let dimension_averages = dimension_average_rows(&dimension_totals, total_cases);
    let gate_transition_rates =
        gate_transition_rate_rows(&transition_total_counts, &transition_pass_counts);
    let gate_transition_path_ok = gate_transition_rates
        .iter()
        .all(|row| f64_at(row, &["pass_rate"], 0.0) >= workflow_gate_pass_min);
    let safety_ok = empty_responses <= u64_at(&thresholds, &["max_empty_responses"], 0)
        && raw_tool_leaks <= u64_at(&thresholds, &["max_raw_tool_leaks"], 0)
        && tool_choice_final_responses
            <= u64_at(&thresholds, &["max_tool_choice_as_final_response"], 0)
        && unsupported_claims <= u64_at(&thresholds, &["max_unsupported_factual_claims"], 0);
    let enough_cases = total_cases >= min_cases;
    let ok = setup_failures.is_empty()
        && enough_cases
        && gate_path_ok
        && gate_transition_path_ok
        && research_success_rate >= research_success_min
        && safety_ok;
    let report = json!({
        "type": "research_golden_eval",
        "schema_version": 1,
        "generated_at": now_iso_like(),
        "ok": ok,
        "mode": if live { "live_dashboard" } else { "offline_responses" },
        "live_options": {
            "fresh_agent_per_case": fresh_agent_per_case,
            "cleanup_fresh_agents": cleanup_fresh_agents,
            "fresh_agent_model_set": fresh_agent_model.is_some(),
            "timeout_seconds": timeout_seconds,
            "confirm_pending_tool": confirm_pending_tool
        },
        "grader": {
            "kind": "deterministic_seed_research_grader",
            "exact_answer_matching": false,
            "score_scale": "0_to_100",
            "pass_score": pass_score,
            "excellent_score": excellent_score
        },
        "summary": {
            "cases": total_cases,
            "min_cases_for_reliability_claim": min_cases,
            "enough_cases_for_reliability_claim": enough_cases,
            "passed_cases": passed_cases,
            "excellent_cases": excellent_cases,
            "average_score": avg_score,
            "research_success_rate": research_success_rate,
            "excellent_rate": excellent_rate,
            "research_success_min": research_success_min,
            "workflow_gate_pass_min": workflow_gate_pass_min,
            "gate_path_ok": gate_path_ok,
            "gate_transition_path_ok": gate_transition_path_ok,
            "safety_ok": safety_ok,
            "empty_responses": empty_responses,
            "raw_tool_leaks": raw_tool_leaks,
            "tool_choice_final_responses": tool_choice_final_responses,
            "unsupported_claims": unsupported_claims,
            "failure_count": failure_events.len()
        },
        "workflow_gate_pass_rates": gate_rates,
        "gate_transition_pass_rates": gate_transition_rates,
        "dimension_averages": dimension_averages,
        "setup_failures": setup_failures,
        "cases": rows,
        "failure_events": failure_events,
        "sources": {
            "cases": cases_path,
            "responses": responses_path,
            "base_url": if live { Some(base_url) } else { None },
            "agent_id": if live { Some(agent_id) } else { None }
        }
    });
    let markdown = markdown_report(&report);
    let failure_rows = report
        .get("failure_events")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let writes_ok = write_json(&out_path, &report).is_ok()
        && write_json(&out_latest_path, &report).is_ok()
        && write_text(&markdown_path, &markdown).is_ok()
        && append_jsonl(&failures_path, &failure_rows).is_ok();
    if !writes_ok {
        eprintln!("eval_runtime: failed to write one or more research golden outputs");
        return 2;
    }
    print_json_line(&report);
    if strict && !ok {
        1
    } else {
        0
    }
}

fn record_gate_counts(
    gates: &BTreeMap<String, bool>,
    total_counts: &mut BTreeMap<String, u64>,
    pass_counts: &mut BTreeMap<String, u64>,
) {
    for (gate, ok) in gates.iter() {
        *total_counts.entry(gate.clone()).or_insert(0) += 1;
        if *ok {
            *pass_counts.entry(gate.clone()).or_insert(0) += 1;
        }
    }
}

fn transition_first_failed_checkpoint(diagnostics: &Value) -> Option<String> {
    diagnostics
        .pointer("/first_failed_checkpoint")
        .and_then(Value::as_str)
        .map(|raw| raw.trim())
        .filter(|raw| !raw.is_empty())
        .map(ToString::to_string)
}

fn record_checkpoint_counts(
    diagnostics: &Value,
    total_counts: &mut BTreeMap<String, u64>,
    pass_counts: &mut BTreeMap<String, u64>,
) {
    let Some(checkpoints) = diagnostics.get("checkpoints").and_then(Value::as_array) else {
        return;
    };
    for checkpoint in checkpoints {
        let Some(name) = checkpoint.get("checkpoint").and_then(Value::as_str) else {
            continue;
        };
        *total_counts.entry(name.to_string()).or_insert(0) += 1;
        if checkpoint.get("status").and_then(Value::as_str) == Some("pass") {
            *pass_counts.entry(name.to_string()).or_insert(0) += 1;
        }
    }
}

fn gate_transition_diagnostics_for_sequence(
    case: &Value,
    initial_payload: &Value,
    final_payload: &Value,
    confirmation_payload_used: bool,
) -> Value {
    let final_diagnostics = gate_transition_diagnostics(case, final_payload);
    if !confirmation_payload_used {
        return final_diagnostics;
    }
    let initial_diagnostics = gate_transition_diagnostics(case, initial_payload);
    let mut checkpoints = Vec::new();
    for checkpoint_name in [
        "4a_request_template_signaled",
        "4b_tool_request_candidate_present",
        "4c_candidate_payload_object",
        "4d_candidate_schema_fields_present",
        "4e_pending_request_promoted",
        "5a_tool_execution_recorded",
        "5b_raw_provider_result_present",
        "5c_packaged_tool_result_present",
        "5d_evidence_refs_extracted",
        "5e_agent_received_evidence_context",
        "6a_synthesis_uses_evidence_or_low_evidence_fallback",
        "terminal_artifact_present",
    ] {
        let source = if matches!(
            checkpoint_name,
            "5a_tool_execution_recorded"
                | "5b_raw_provider_result_present"
                | "5c_packaged_tool_result_present"
                | "5d_evidence_refs_extracted"
                | "5e_agent_received_evidence_context"
                | "6a_synthesis_uses_evidence_or_low_evidence_fallback"
                | "terminal_artifact_present"
        ) {
            &final_diagnostics
        } else {
            &initial_diagnostics
        };
        if let Some(row) = checkpoint_by_name(source, checkpoint_name) {
            checkpoints.push(row.clone());
        }
    }
    let first_failed_checkpoint = checkpoints
        .iter()
        .find(|row| row.get("status").and_then(Value::as_str) == Some("fail"))
        .and_then(|row| row.get("checkpoint").and_then(Value::as_str))
        .unwrap_or("")
        .to_string();
    json!({
        "diagnostic_mode": "sequenced_confirmation",
        "first_failed_checkpoint": if first_failed_checkpoint.is_empty() {
            Value::Null
        } else {
            Value::String(first_failed_checkpoint.clone())
        },
        "inferred_failure_boundary": failure_boundary(&first_failed_checkpoint),
        "required_gate_4_fields": initial_diagnostics
            .get("required_gate_4_fields")
            .cloned()
            .unwrap_or_else(|| json!([])),
        "candidate_payload_fields": initial_diagnostics
            .get("candidate_payload_fields")
            .cloned()
            .unwrap_or_else(|| json!([])),
        "final_llm_status": final_diagnostics.get("final_llm_status").cloned().unwrap_or(Value::Null),
        "finalization_outcome": final_diagnostics
            .get("finalization_outcome")
            .cloned()
            .unwrap_or(Value::Null),
        "checkpoints": checkpoints
    })
}

fn checkpoint_by_name<'a>(diagnostics: &'a Value, name: &str) -> Option<&'a Value> {
    diagnostics
        .get("checkpoints")
        .and_then(Value::as_array)
        .and_then(|rows| {
            rows.iter()
                .find(|row| row.get("checkpoint").and_then(Value::as_str) == Some(name))
        })
}

#[cfg(test)]
#[path = "eval_research_golden_lifecycle_tests.rs"]
mod eval_research_golden_lifecycle_tests;
#[cfg(test)]
#[path = "eval_research_golden_post_tool_tests.rs"]
mod eval_research_golden_post_tool_tests;
#[cfg(test)]
#[path = "eval_research_golden_tests.rs"]
mod eval_research_golden_tests;
