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
fn research_golden_counts_low_signal_packaged_result_before_evidence_extraction() {
    let root = temp_path("research_golden_low_signal_packaged");
    let cases = root.join("cases.json");
    let responses = root.join("responses.json");
    write_json_file(&cases, &dataset());
    write_json_file(
        &responses,
        &json!({
            "responses": [{
                "case_id": "research_gold_test",
                "response_payload": {
                    "response": "The current source coverage is limited. The retrieved results point to official docs and release notes, but they are not strong enough yet for a confident comparison. The next useful action is to add one narrower query per framework and then synthesize from the combined evidence.",
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
                            "title": "LangGraph release notes",
                            "snippet": "LangGraph release notes discuss graph execution, persistence, and workflow reliability improvements."
                        }],
                        "result": "The current source coverage is limited. Retrieved results point to docs and release notes, but they are not yet strong enough for a confident comparison."
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
        report.pointer(
            "/cases/0/gate_transition_diagnostics/post_tool_pipeline/packaged_tool_result_present"
        ),
        Some(&Value::Bool(true))
    );
    assert_eq!(
        report
            .pointer("/cases/0/gate_transition_diagnostics/post_tool_pipeline/packaged_tool_result_paths/0")
            .and_then(Value::as_str),
        Some("tools.0.result")
    );
    assert_eq!(
        report
            .pointer("/cases/0/gate_transition_diagnostics/first_failed_checkpoint")
            .and_then(Value::as_str),
        Some("5d_evidence_refs_extracted")
    );
}

#[test]
fn research_golden_counts_error_status_tool_artifacts_before_evidence_extraction() {
    let root = temp_path("research_golden_error_status_tool_artifacts");
    let cases = root.join("cases.json");
    let responses = root.join("responses.json");
    write_json_file(&cases, &dataset());
    write_json_file(
        &responses,
        &json!({
            "responses": [{
                "case_id": "research_gold_test",
                "response_payload": {
                    "response": "The search failed, but the recorded provider artifact still shows which query ran and why it degraded. That is enough to explain the limitation and propose a narrower retry.",
                    "pending_tool_request": {
                        "status": "executed",
                        "tool_name": "batch_query",
                        "selected_tool_family": "Web Search / Fetch",
                        "input": {
                            "query": "Find recent benchmarks comparing agent frameworks",
                            "aperture": "medium"
                        }
                    },
                    "tools": [{
                        "name": "batch_query",
                        "status": "error",
                        "provider_results": [{
                            "provider": "bing_rss",
                            "query": "Find recent benchmarks comparing agent frameworks",
                            "summary": "Search provider returned no comparison-grade benchmark rows.",
                            "error": "web_search_tool_surface_degraded"
                        }],
                        "result": "The recorded tool outcome shows a failed search attempt with degraded provider coverage."
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
        report.pointer(
            "/cases/0/gate_transition_diagnostics/post_tool_pipeline/raw_provider_result_present"
        ),
        Some(&Value::Bool(true))
    );
    assert_eq!(
        report.pointer(
            "/cases/0/gate_transition_diagnostics/post_tool_pipeline/packaged_tool_result_present"
        ),
        Some(&Value::Bool(true))
    );
    assert_eq!(
        report
            .pointer("/cases/0/gate_transition_diagnostics/first_failed_checkpoint")
            .and_then(Value::as_str),
        Some("5d_evidence_refs_extracted")
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
    assert_eq!(
        report
            .pointer("/cases/0/gate_transition_diagnostics/synthesis_failure_hardness")
            .and_then(Value::as_str),
        Some("soft")
    );
    assert_eq!(
        report
            .pointer("/cases/0/gate_transition_diagnostics/synthesis_failure_class")
            .and_then(Value::as_str),
        Some("evidence_entity_coverage_gap")
    );
}

#[test]
fn research_golden_counts_tool_row_evidence_refs_as_extracted_evidence() {
    let root = temp_path("research_golden_tool_row_evidence");
    let cases = root.join("cases.json");
    let responses = root.join("responses.json");
    write_json_file(&cases, &dataset());
    write_json_file(
        &responses,
        &json!({
            "responses": [{
                "case_id": "research_gold_test",
                "response_payload": {
                    "response": "According to the current evidence, LangGraph documents durable execution while Infring still appears sparse in public sources. The comparison remains limited, but the retrieved evidence supports that one workflow favors mature graph orchestration and the other favors editable gate experimentation.",
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
                        "result": "Current source evidence says LangGraph documents graph orchestration and durable execution, while Infring notes emphasize workflow CD gates and rollback.",
                        "evidence_refs": [
                            {"title": "LangGraph durable graph docs", "locator": "https://docs.langchain.com/langgraph", "score": 0.91}
                        ]
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
            .pointer("/cases/0/gate_transition_diagnostics/post_tool_pipeline/evidence_extracted"),
        Some(&Value::Bool(true))
    );
    assert_ne!(
        report
            .pointer("/cases/0/gate_transition_diagnostics/first_failed_checkpoint")
            .and_then(Value::as_str),
        Some("5d_evidence_refs_extracted")
    );
}

#[test]
fn research_golden_grades_low_signal_evidence_as_low_evidence_synthesis() {
    let root = temp_path("research_golden_low_signal_evidence_lane");
    let cases = root.join("cases.json");
    let responses = root.join("responses.json");
    write_json_file(&cases, &dataset());
    write_json_file(
        &responses,
        &json!({
            "responses": [{
                "case_id": "research_gold_test",
                "response_payload": {
                    "response": "The returned results were low-signal, so there is no source-backed winner. This retrieval miss supports only that LangGraph has clearer public docs for durable graph execution; it does not support a complete Infring vs LangGraph comparison. Bounded conclusion: evaluate LangGraph first for public evidence maturity, and treat Infring as needing direct docs or repo inspection before selection.",
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
                        "status": "low_signal",
                        "raw_results": [{
                            "title": "LangGraph durable graph docs",
                            "snippet": "LangGraph documents graph orchestration, durable execution, and state-machine patterns for agent workflows."
                        }],
                        "result": "Low signal: one relevant LangGraph source surfaced, but no comparable public Infring source surfaced.",
                        "evidence_refs": [
                            {"title": "LangGraph durable graph docs", "locator": "https://docs.langchain.com/langgraph", "score": 0.91}
                        ]
                    }],
                    "response_finalization": {
                        "outcome": "workflow_authored+tool_completion:low_signal+synthesized",
                        "tool_completion": {
                            "completion_state": "low_signal",
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
    let checkpoints = report
        .pointer("/cases/0/gate_transition_diagnostics/checkpoints")
        .and_then(Value::as_array)
        .expect("checkpoints");
    let gate_6a = checkpoints
        .iter()
        .find(|row| {
            row.get("checkpoint").and_then(Value::as_str)
                == Some("6a_synthesis_uses_evidence_or_low_evidence_fallback")
        })
        .expect("6a checkpoint");
    assert_eq!(gate_6a.get("status").and_then(Value::as_str), Some("pass"));
    assert_ne!(
        report
            .pointer("/cases/0/gate_transition_diagnostics/first_failed_checkpoint")
            .and_then(Value::as_str),
        Some("6a_synthesis_uses_evidence_or_low_evidence_fallback")
    );
}

#[test]
fn research_golden_accepts_no_results_retrieval_failure_as_low_evidence_synthesis() {
    let root = temp_path("research_golden_no_results_low_evidence");
    let cases = root.join("cases.json");
    let responses = root.join("responses.json");
    write_json_file(&cases, &dataset());
    write_json_file(
        &responses,
        &json!({
            "responses": [{
                "case_id": "research_gold_test",
                "response_payload": {
                    "response": "The web search returned no directly relevant results for this query and the evidence is limited. That retrieval failure is not evidence that Infring and LangGraph are equivalent. The attempted search covered official docs and comparison terms, but returned zero candidate snippets or evidence refs, so I cannot cite a source-backed winner. Bounded conclusion: use this only as a low-evidence signal, retry with narrower source targets, and avoid treating the absence of retrievable evidence as a product judgment.",
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
                        "raw_results": [{
                            "title": "No usable result",
                            "snippet": "No results were returned for the query."
                        }],
                        "result": "No results: provider returned no usable source snippets.",
                        "evidence_refs": [
                            {"title": "No usable result", "locator": "tool:no-results", "score": 0.0}
                        ]
                    }],
                    "response_finalization": {
                        "outcome": "workflow_authored+tool_completion:no_results+synthesized",
                        "tool_completion": {
                            "completion_state": "no_results",
                            "findings_available": false,
                            "evidence_refs_used": ["tool:no-results"]
                        }
                    }
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
    let gate_6a = checkpoints
        .iter()
        .find(|row| {
            row.get("checkpoint").and_then(Value::as_str)
                == Some("6a_synthesis_uses_evidence_or_low_evidence_fallback")
        })
        .expect("6a checkpoint");
    assert_eq!(gate_6a.get("status").and_then(Value::as_str), Some("pass"));
    assert_ne!(
        report
            .pointer("/cases/0/gate_transition_diagnostics/synthesis_failure_class")
            .and_then(Value::as_str),
        Some("low_signal_not_acknowledged")
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
                    "response": "The first result was low signal, so no source-backed conclusion is available yet. What we know is that public evidence about Infring is sparse. What we do not know is how it compares head to head with other frameworks from current source material. The next useful action is to narrow the query to one competitor at a time."
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
        .find(|row| {
            row.get("checkpoint").and_then(Value::as_str) == Some("4a_request_template_signaled")
        })
        .expect("4a checkpoint");
    assert_eq!(gate_4a.get("status").and_then(Value::as_str), Some("pass"));
    let gate_4b = checkpoints
        .iter()
        .find(|row| {
            row.get("checkpoint").and_then(Value::as_str)
                == Some("4b_tool_request_candidate_present")
        })
        .expect("4b checkpoint");
    assert_eq!(gate_4b.get("status").and_then(Value::as_str), Some("pass"));
    let gate_5a = checkpoints
        .iter()
        .find(|row| {
            row.get("checkpoint").and_then(Value::as_str) == Some("5a_tool_execution_recorded")
        })
        .expect("5a checkpoint");
    assert_eq!(gate_5a.get("status").and_then(Value::as_str), Some("pass"));
    let gate_5b = checkpoints
        .iter()
        .find(|row| {
            row.get("checkpoint").and_then(Value::as_str) == Some("5b_raw_provider_result_present")
        })
        .expect("5b checkpoint");
    assert_eq!(gate_5b.get("status").and_then(Value::as_str), Some("pass"));
    let gate_5c = checkpoints
        .iter()
        .find(|row| {
            row.get("checkpoint").and_then(Value::as_str) == Some("5c_packaged_tool_result_present")
        })
        .expect("5c checkpoint");
    assert_eq!(gate_5c.get("status").and_then(Value::as_str), Some("pass"));
    let gate_5d = checkpoints
        .iter()
        .find(|row| {
            row.get("checkpoint").and_then(Value::as_str) == Some("5d_evidence_refs_extracted")
        })
        .expect("5d checkpoint");
    assert_eq!(gate_5d.get("status").and_then(Value::as_str), Some("pass"));
    let gate_5e = checkpoints
        .iter()
        .find(|row| {
            row.get("checkpoint").and_then(Value::as_str)
                == Some("5e_agent_received_evidence_context")
        })
        .expect("5e checkpoint");
    assert_eq!(gate_5e.get("status").and_then(Value::as_str), Some("pass"));
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
    assert_ne!(
        report
            .pointer("/cases/0/gate_transition_diagnostics/first_failed_checkpoint")
            .and_then(Value::as_str),
        Some("5a_tool_execution_recorded")
    );
}

#[test]
fn research_golden_allows_bounded_missing_tool_context_fallback_at_6a() {
    let root = temp_path("research_golden_missing_tool_context_fallback");
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
                "id": "research_gold_post_tool_missing_context",
                "category": "post_tool_synthesis",
                "prompt": "After the web tool returns several source snippets about agent frameworks, synthesize the tradeoffs and cite evidence refs without dumping the raw payload.",
                "expected_gate_path": {
                    "gate_1": "tool_required_or_pending_tool_result",
                    "gate_2": "web_research",
                    "gate_3": "web_search",
                    "gate_4_required_fields": ["query", "aperture"],
                    "post_tool": "must_synthesize_from_evidence_refs"
                },
                "required_entities": ["agent frameworks"]
            }]
        }),
    );
    write_json_file(
        &responses,
        &json!({
            "responses": [{
                "case_id": "research_gold_post_tool_missing_context",
                "response_payload": {
                    "response": "No returned tool result is available in this turn, so no source-backed synthesis is available yet. What we know is that agent frameworks usually involve a tradeoff between autonomy, observability, and deployment maturity. What we do not know is which current frameworks the missing snippets supported, and that would require live research. The next best search query is agent framework tradeoff benchmark observability deployment 2026."
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
    let gate_6a = checkpoints
        .iter()
        .find(|row| {
            row.get("checkpoint").and_then(Value::as_str)
                == Some("6a_synthesis_uses_evidence_or_low_evidence_fallback")
        })
        .expect("6a checkpoint");
    assert_eq!(gate_6a.get("status").and_then(Value::as_str), Some("pass"));
    assert_ne!(
        report
            .pointer("/cases/0/gate_transition_diagnostics/first_failed_checkpoint")
            .and_then(Value::as_str),
        Some("6a_synthesis_uses_evidence_or_low_evidence_fallback")
    );
}

#[test]
fn research_golden_allows_explicit_missing_tool_fallback_when_tools_array_is_empty() {
    let root = temp_path("research_golden_missing_tool_empty_tools");
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
                "id": "research_gold_post_tool_missing_context_empty_tools",
                "category": "post_tool_synthesis",
                "prompt": "After the web tool returns several source snippets about agent frameworks, synthesize the tradeoffs and cite evidence refs without dumping the raw payload.",
                "expected_gate_path": {
                    "gate_1": "tool_required_or_pending_tool_result",
                    "gate_2": "web_research",
                    "gate_3": "web_search",
                    "gate_4_required_fields": ["query", "aperture"],
                    "post_tool": "must_synthesize_from_evidence_refs"
                },
                "required_entities": ["agent frameworks"]
            }]
        }),
    );
    write_json_file(
        &responses,
        &json!({
            "responses": [{
                "case_id": "research_gold_post_tool_missing_context_empty_tools",
                "response_payload": {
                    "tools": [],
                    "response": "No returned tool result is available in this turn, so no source-backed synthesis is available yet. What we know is that agent frameworks usually involve a tradeoff between autonomy, observability, and deployment maturity, but no returned tool result, snippets, or evidence refs are present in this turn. What we do not know is which agent frameworks the missing snippets supported or what evidence refs they would justify, so no source-backed comparison is available yet. My recommendation is to rerun one focused source-backed comparison before drawing conclusions. The next best search query is agent framework tradeoff benchmark observability deployment 2026."
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
    let gate_6a = checkpoints
        .iter()
        .find(|row| {
            row.get("checkpoint").and_then(Value::as_str)
                == Some("6a_synthesis_uses_evidence_or_low_evidence_fallback")
        })
        .expect("6a checkpoint");
    assert_eq!(gate_6a.get("status").and_then(Value::as_str), Some("pass"));
    assert_ne!(
        report
            .pointer("/cases/0/gate_transition_diagnostics/first_failed_checkpoint")
            .and_then(Value::as_str),
        Some("6a_synthesis_uses_evidence_or_low_evidence_fallback")
    );
}

#[test]
fn research_golden_rejects_internal_runtime_context_as_post_tool_evidence() {
    let root = temp_path("research_golden_internal_context_not_evidence");
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
                "id": "research_gold_post_tool_internal_context",
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
                "case_id": "research_gold_post_tool_internal_context",
                "response_payload": {
                    "response": "What we know is the identity context: Infring is the platform hosting this conversation, evident from system instructions and the agent name in the runtime. That platform identity suggests orchestration features, but it is not tied to any returned external evidence. The next step would be to search for public framework comparisons."
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
