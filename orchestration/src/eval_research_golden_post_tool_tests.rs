use super::*;
use std::fs;
use std::path::{Path, PathBuf};

fn temp_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("{name}_{}", now_iso_like().replace(':', "_")))
}

fn write_json_file(path: &Path, payload: &Value) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("temp parent");
    }
    fs::write(
        path,
        format!("{}\n", serde_json::to_string_pretty(payload).unwrap()),
    )
    .expect("write temp json");
}

fn dataset() -> Value {
    json!({
        "reliability_thresholds": {
            "min_cases_for_reliability_claim": 1,
            "workflow_gate_pass_min": 0.95,
            "research_success_min": 0.85,
            "max_empty_responses": 0,
            "max_raw_tool_leaks": 0,
            "max_tool_choice_as_final_response": 0,
            "max_unsupported_factual_claims": 0
        },
        "scoring_contract": {
            "pass_score": 85,
            "excellent_score": 95
        },
        "cases": [{
            "id": "research_gold_test",
            "category": "comparison",
            "prompt": "Use web research to compare Infring with LangGraph.",
            "expected_gate_path": {
                "gate_1": "tool_required",
                "gate_2": "web_research",
                "gate_3": "web_search",
                "gate_4_required_fields": ["query", "aperture"]
            },
            "required_entities": ["Infring", "LangGraph"]
        }]
    })
}

fn runner_args(root: &Path, cases: &Path, responses: &Path, strict: bool) -> Vec<String> {
    vec![
        format!("--cases={}", cases.display()),
        format!("--responses={}", responses.display()),
        format!("--out={}", root.join("out.json").display()),
        format!("--out-latest={}", root.join("latest.json").display()),
        format!("--out-markdown={}", root.join("report.md").display()),
        format!("--failures-out={}", root.join("failures.jsonl").display()),
        format!("--strict={}", if strict { "1" } else { "0" }),
    ]
}

#[test]
fn research_golden_splits_low_signal_tool_result_after_execution() {
    let root = temp_path("research_golden_low_signal_tool");
    let cases = root.join("cases.json");
    let responses = root.join("responses.json");
    write_json_file(&cases, &dataset());
    write_json_file(
        &responses,
        &json!({
            "responses": [{
                "case_id": "research_gold_test",
                "response_payload": {
                    "response": "The search was low signal and did not produce enough source coverage. Please narrow the query to one framework pair.",
                    "pending_tool_request": {
                        "status": "executed",
                        "tool_name": "web_search",
                        "selected_tool_family": "Web Search / Fetch",
                        "input": {
                            "query": "Infring LangGraph comparison current docs",
                            "aperture": "medium"
                        }
                    },
                    "tools": [{
                        "name": "web_search",
                        "status": "no_results",
                        "result": "Search did not produce enough source coverage for the requested comparison."
                    }],
                    "response_finalization": {
                        "tool_completion": {
                            "completion_state": "reported_no_findings",
                            "findings_available": false,
                            "final_no_findings": true
                        }
                    }
                }
            }]
        }),
    );
    let code = run_research_golden(&runner_args(&root, &cases, &responses, false));
    assert_eq!(code, 0);
    let report = read_json(root.join("out.json").to_str().unwrap());
    assert_eq!(
        report
            .pointer("/cases/0/gate_transition_diagnostics/first_failed_checkpoint")
            .and_then(Value::as_str),
        Some("5b_raw_provider_result_present")
    );
    assert_eq!(
        report
            .pointer("/cases/0/gate_transition_diagnostics/inferred_failure_boundary")
            .and_then(Value::as_str),
        Some("raw_provider_result_absent_or_low_signal")
    );
}

#[test]
fn research_golden_splits_missing_evidence_extraction_after_usable_result() {
    let root = temp_path("research_golden_missing_evidence");
    let cases = root.join("cases.json");
    let responses = root.join("responses.json");
    write_json_file(&cases, &dataset());
    write_json_file(
        &responses,
        &json!({
            "responses": [{
                "case_id": "research_gold_test",
                "response_payload": {
                    "response": "According to current source evidence, Infring and LangGraph differ in workflow control and graph orchestration. The practical tradeoff is that LangGraph fits production teams that need durable graph execution, while Infring fits teams experimenting with editable workflow gates and audit-friendly rollback. My recommendation is to use LangGraph for conventional production assistants and keep Infring for gate-inspection experiments.",
                    "pending_tool_request": {
                        "status": "executed",
                        "tool_name": "web_search",
                        "selected_tool_family": "Web Search / Fetch",
                        "input": {
                            "query": "Infring LangGraph comparison current docs",
                            "aperture": "medium"
                        }
                    },
                    "tools": [{
                        "name": "web_search",
                        "status": "ok",
                        "raw_results": [{
                            "title": "LangGraph durable graph docs",
                            "snippet": "LangGraph documents graph orchestration, durable execution, and state-machine patterns for agent workflows."
                        }],
                        "result": "Current source evidence says LangGraph documents graph orchestration and durable execution, while Infring notes emphasize workflow CD gates and rollback."
                    }],
                    "response_finalization": {
                        "tool_completion": {
                            "completion_state": "reported_findings",
                            "findings_available": true
                        }
                    }
                }
            }]
        }),
    );
    let code = run_research_golden(&runner_args(&root, &cases, &responses, false));
    assert_eq!(code, 0);
    let report = read_json(root.join("out.json").to_str().unwrap());
    assert_eq!(
        report
            .pointer("/cases/0/gate_transition_diagnostics/first_failed_checkpoint")
            .and_then(Value::as_str),
        Some("5d_evidence_refs_extracted")
    );
    assert_eq!(
        report
            .pointer("/cases/0/gate_transition_diagnostics/inferred_failure_boundary")
            .and_then(Value::as_str),
        Some("packaged_result_not_extracted_to_evidence")
    );
}

#[test]
fn research_golden_splits_weak_synthesis_after_evidence_extraction() {
    let root = temp_path("research_golden_weak_synthesis");
    let cases = root.join("cases.json");
    let responses = root.join("responses.json");
    write_json_file(&cases, &dataset());
    write_json_file(
        &responses,
        &json!({
            "responses": [{
                "case_id": "research_gold_test",
                "response_payload": {
                    "response": "I found sources. Please narrow the query.",
                    "pending_tool_request": {
                        "status": "executed",
                        "tool_name": "web_search",
                        "selected_tool_family": "Web Search / Fetch",
                        "input": {
                            "query": "Infring LangGraph comparison current docs",
                            "aperture": "medium"
                        }
                    },
                    "tools": [{
                        "name": "web_search",
                        "status": "ok",
                        "raw_results": [{
                            "title": "LangGraph durable graph docs",
                            "snippet": "LangGraph documents graph orchestration, durable execution, and state-machine patterns for agent workflows."
                        }],
                        "result": "Current source evidence says LangGraph documents graph orchestration and durable execution, while Infring notes emphasize workflow CD gates and rollback."
                    }],
                    "response_workflow": {
                        "evidence_refs": ["evidence:langgraph-docs", "evidence:infring-notes"]
                    },
                    "response_finalization": {
                        "tool_completion": {
                            "completion_state": "reported_findings",
                            "findings_available": true
                        }
                    }
                }
            }]
        }),
    );
    let code = run_research_golden(&runner_args(&root, &cases, &responses, false));
    assert_eq!(code, 0);
    let report = read_json(root.join("out.json").to_str().unwrap());
    assert_eq!(
        report
            .pointer("/cases/0/gate_transition_diagnostics/first_failed_checkpoint")
            .and_then(Value::as_str),
        Some("6a_synthesis_uses_evidence_or_low_evidence_fallback")
    );
    assert_eq!(
        report
            .pointer("/cases/0/gate_transition_diagnostics/inferred_failure_boundary")
            .and_then(Value::as_str),
        Some("post_tool_synthesis_not_useful")
    );
}

#[test]
fn research_golden_allows_post_tool_synthesis_without_fresh_request_candidate() {
    let root = temp_path("research_golden_post_tool_no_fresh_candidate");
    let cases = root.join("cases.json");
    let responses = root.join("responses.json");
    write_json_file(
        &cases,
        &json!({
            "reliability_thresholds": {
                "min_cases_for_reliability_claim": 1,
                "workflow_gate_pass_min": 0.95,
                "research_success_min": 0.85
            },
            "scoring_contract": {
                "pass_score": 85,
                "excellent_score": 95
            },
            "cases": [{
                "id": "research_gold_post_tool",
                "category": "post_tool_synthesis",
                "prompt": "After the web tool returns low-signal results for Infring, synthesize a useful answer anyway.",
                "expected_gate_path": {
                    "gate_1": "tool_required_or_pending_tool_result",
                    "gate_2": "web_research",
                    "gate_3": "web_search",
                    "gate_4_required_fields": ["query", "aperture"],
                    "post_tool": "must_synthesize_from_low_signal_evidence"
                },
                "required_entities": ["Infring"]
            }]
        }),
    );
    write_json_file(
        &responses,
        &json!({
            "responses": [{
                "case_id": "research_gold_post_tool",
                "response_payload": {
                    "response": "The first result was low signal, so no receipt-backed conclusion is available yet. What we know is that public evidence about Infring is sparse. What we do not know is how it compares head to head with other frameworks from current source material. The next useful action is to narrow the query to one competitor at a time."
                }
            }]
        }),
    );
    let code = run_research_golden(&runner_args(&root, &cases, &responses, false));
    assert_eq!(code, 0);
    let report = read_json(root.join("out.json").to_str().unwrap());
    let checkpoints = report
        .pointer("/cases/0/gate_transition_diagnostics/checkpoints")
        .and_then(Value::as_array)
        .expect("checkpoints");
    let gate_4a = checkpoints
        .iter()
        .find(|row| row.get("checkpoint").and_then(Value::as_str) == Some("4a_request_template_signaled"))
        .expect("4a checkpoint");
    assert_eq!(gate_4a.get("status").and_then(Value::as_str), Some("pass"));
    let gate_4b = checkpoints
        .iter()
        .find(|row| row.get("checkpoint").and_then(Value::as_str) == Some("4b_tool_request_candidate_present"))
        .expect("4b checkpoint");
    assert_eq!(gate_4b.get("status").and_then(Value::as_str), Some("pass"));
    assert_ne!(
        report
            .pointer("/cases/0/gate_transition_diagnostics/first_failed_checkpoint")
            .and_then(Value::as_str),
        Some("4a_request_template_signaled")
    );
    assert_ne!(
        report
            .pointer("/cases/0/gate_transition_diagnostics/first_failed_checkpoint")
            .and_then(Value::as_str),
        Some("4b_tool_request_candidate_present")
    );
}
