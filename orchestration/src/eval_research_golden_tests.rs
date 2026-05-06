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
fn research_golden_scores_evidenced_final_answer() {
    let root = temp_path("research_golden_pass");
    let cases = root.join("cases.json");
    let responses = root.join("responses.json");
    write_json_file(&cases, &dataset());
    write_json_file(
        &responses,
        &json!({
            "responses": [{
                "case_id": "research_gold_test",
                "response_payload": {
                    "response": "As of the current docs and release evidence, Infring and LangGraph solve different parts of the agent workflow problem. Infring appears strongest when you want an editable workflow CD with explicit gate tradeoffs and rollback discipline, while LangGraph is best for Python teams that want durable graph orchestration and documented production patterns. My recommendation: use LangGraph for a conventional app team, and keep Infring if the priority is inspecting workflow gates, evidence refs, and the tradeoff between flexible prompts and typed control. The caveat is that public evidence for Infring is sparse, so treat Infring-specific claims as provisional until verified.",
                    "pending_tool_request": {
                        "status": "pending_confirmation",
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
                        "result": "Source evidence from current docs: LangGraph documents graph orchestration and durable state patterns; Infring evidence is limited to workflow CD and gate inspection notes."
                    }],
                    "response_workflow": {
                        "evidence_refs": ["evidence:langgraph-docs", "evidence:infring-notes"],
                        "final_llm_response": {
                            "status": "synthesized",
                            "evidence_refs_used": ["evidence:langgraph-docs", "evidence:infring-notes"]
                        },
                        "stage_statuses": [
                            {"stage": "gate_1_tool_need", "status": "answered_yes"},
                            {"stage": "gate_2_tool_family", "status": "selected_web_research"},
                            {"stage": "gate_3_tool_key", "status": "selected_web_search"},
                            {"stage": "gate_4_request_template", "status": "completed"}
                        ]
                    }
                }
            }]
        }),
    );
    let code = run_research_golden(&runner_args(&root, &cases, &responses, true));
    assert_eq!(code, 0);
    let report = read_json(root.join("out.json").to_str().unwrap());
    assert_eq!(report.get("ok").and_then(Value::as_bool), Some(true));
    assert_eq!(
        report
            .pointer("/summary/research_success_rate")
            .and_then(Value::as_f64),
        Some(1.0)
    );
    assert_eq!(
        report.pointer("/cases/0/gate_transition_diagnostics/first_failed_checkpoint"),
        Some(&Value::Null)
    );
}

#[test]
fn research_golden_separates_gate_progress_from_research_success() {
    let root = temp_path("research_golden_pending");
    let cases = root.join("cases.json");
    let responses = root.join("responses.json");
    write_json_file(&cases, &dataset());
    write_json_file(
        &responses,
        &json!({
            "responses": [{
                "case_id": "research_gold_test",
                "response_payload": {
                    "response": "",
                    "pending_tool_request": {
                        "status": "pending_confirmation",
                        "tool_name": "web_search",
                        "selected_tool_family": "Web Search / Fetch",
                        "input": {
                            "query": "Infring LangGraph comparison current docs",
                            "aperture": "medium"
                        }
                    },
                    "response_workflow": {
                        "stage_statuses": [
                            {"stage": "gate_1_tool_need", "status": "answered_yes"},
                            {"stage": "gate_2_tool_family", "status": "selected_web_research"},
                            {"stage": "gate_3_tool_key", "status": "selected_web_search"},
                            {"stage": "gate_4_request_template", "status": "completed"}
                        ]
                    }
                }
            }]
        }),
    );
    let code = run_research_golden(&runner_args(&root, &cases, &responses, false));
    assert_eq!(code, 0);
    let report = read_json(root.join("out.json").to_str().unwrap());
    assert_eq!(report.get("ok").and_then(Value::as_bool), Some(false));
    assert_eq!(
        report
            .pointer("/summary/gate_path_ok")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        report
            .pointer("/summary/research_success_rate")
            .and_then(Value::as_f64),
        Some(0.0)
    );
    assert_eq!(
        report
            .pointer("/cases/0/failures")
            .and_then(Value::as_array)
            .unwrap()
            .iter()
            .any(|row| row.as_str() == Some("empty_research_response")),
        true
    );
    assert_eq!(
        report
            .pointer("/cases/0/gate_transition_diagnostics/first_failed_checkpoint")
            .and_then(Value::as_str),
        Some("5a_tool_execution_recorded")
    );
}

#[test]
fn research_golden_confirm_pending_tool_can_score_second_turn() {
    let root = temp_path("research_golden_confirm_pending");
    let cases = root.join("cases.json");
    let responses = root.join("responses.json");
    write_json_file(&cases, &dataset());
    write_json_file(
        &responses,
        &json!({
            "responses": [{
                "case_id": "research_gold_test",
                "response_payload": {
                    "response_sequence": [
                        {
                            "response": "",
                            "pending_tool_request": {
                                "status": "pending_confirmation",
                                "tool_name": "web_search",
                                "selected_tool_family": "Web Search / Fetch",
                                "input": {
                                    "query": "Infring LangGraph comparison current docs",
                                    "aperture": "medium"
                                }
                            }
                        },
                        {
                            "response": "Based on source evidence from current docs, Infring and LangGraph fit different research workflows. LangGraph is stronger for teams that want mature graph orchestration, durable state, and production deployment patterns. Infring is more attractive when the tradeoff you care about is editable workflow gates, evidence review, and CD-style rollback discipline. My recommendation is to use LangGraph for a conventional production assistant and keep Infring for experiments where inspecting the workflow path matters most. Caveat: public Infring evidence is limited, so verify Infring-specific claims before committing.",
                            "tools": [{
                                "name": "web_search",
                                "status": "ok",
                                "raw_results": [{
                                    "title": "LangGraph durable graph docs",
                                    "snippet": "LangGraph documents graph orchestration, durable execution, and state-machine patterns for agent workflows."
                                }],
                                "result": "Source evidence from current docs: LangGraph documents graph orchestration and durable state patterns; Infring evidence is limited to workflow CD and gate inspection notes.",
                                "input": {
                                    "query": "Infring LangGraph comparison current docs",
                                    "aperture": "medium"
                                }
                            }],
                            "response_workflow": {
                                "evidence_refs": ["evidence:langgraph-docs", "evidence:infring-notes"],
                                "final_llm_response": {
                                    "status": "synthesized",
                                    "evidence_refs_used": ["evidence:langgraph-docs", "evidence:infring-notes"]
                                }
                            }
                        }
                    ]
                }
            }]
        }),
    );
    let mut args = runner_args(&root, &cases, &responses, true);
    args.push("--confirm-pending-tool=1".to_string());
    let code = run_research_golden(&args);
    assert_eq!(code, 0);
    let report = read_json(root.join("out.json").to_str().unwrap());
    assert_eq!(report.get("ok").and_then(Value::as_bool), Some(true));
    assert_eq!(
        report.pointer("/cases/0/turn_sequence/confirmation_fixture_used"),
        Some(&Value::Bool(true))
    );
    assert_eq!(
        report
            .pointer(
                "/cases/0/turn_sequence/initial_gate_transition_diagnostics/first_failed_checkpoint"
            )
            .and_then(Value::as_str),
        Some("5a_tool_execution_recorded")
    );
    assert_eq!(
        report.pointer("/cases/0/gate_transition_diagnostics/first_failed_checkpoint"),
        Some(&Value::Null)
    );
}

#[test]
fn research_golden_identifies_unpromoted_request_candidate() {
    let root = temp_path("research_golden_unpromoted");
    let cases = root.join("cases.json");
    let responses = root.join("responses.json");
    write_json_file(&cases, &dataset());
    write_json_file(
        &responses,
        &json!({
            "responses": [{
                "case_id": "research_gold_test",
                "response_payload": {
                    "response": "",
                    "latent_tool_candidates": [{
                        "tool_name": "web_search",
                        "request_payload": {
                            "query": "Infring LangGraph comparison current docs",
                            "aperture": "medium"
                        }
                    }],
                    "response_workflow": {
                        "final_llm_response": {"status": "empty_llm_response"}
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
        Some("4e_pending_request_promoted")
    );
    assert_eq!(
        report
            .pointer("/cases/0/gate_transition_diagnostics/inferred_failure_boundary")
            .and_then(Value::as_str),
        Some("request_candidate_not_promoted")
    );
}

#[test]
fn research_golden_sanitizes_backend_key_errors() {
    let root = temp_path("research_golden_backend_error");
    let cases = root.join("cases.json");
    let responses = root.join("responses.json");
    write_json_file(&cases, &dataset());
    write_json_file(
        &responses,
        &json!({
            "responses": [{
                "case_id": "research_gold_test",
                "response_payload": {
                    "response": "",
                    "provider": "openai",
                    "model": "gpt-5",
                    "runtime_model": "gpt-5",
                    "initial_invoke_error": true,
                    "error": "model backend unavailable: Incorrect API key provided: secret-value. Check provider settings."
                }
            }]
        }),
    );
    let code = run_research_golden(&runner_args(&root, &cases, &responses, false));
    assert_eq!(code, 0);
    let report = read_json(root.join("out.json").to_str().unwrap());
    let error = report
        .pointer("/cases/0/response_diagnostics/error")
        .and_then(Value::as_str)
        .unwrap_or("");
    assert!(error.contains("[redacted]"), "{error}");
    assert!(!error.contains("secret-value"), "{error}");
}
