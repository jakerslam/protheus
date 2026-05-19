use serde_json::{json, Value};

use super::super::eval_research_golden_scoring::grade_case;
use super::super::eval_research_golden_utils::str_at;
use super::request_packs::load_request_pack_index;
use super::synthetic::synthesize_tooling_eval_payload;

#[test]
fn extracts_request_pack_from_research_report_case() {
    let path = std::env::temp_dir().join("web_tooling_request_pack_extract.json");
    let report = json!({
        "cases": [
            {
                "case_id": "case_a",
                "response_diagnostics": {
                    "pending_tool_request": {
                        "tool_name": "batch_query",
                        "input": {
                            "query": "hello",
                            "queries": ["hello"]
                        }
                    }
                }
            }
        ]
    });
    std::fs::write(&path, serde_json::to_vec(&report).unwrap()).expect("write report");
    let index = load_request_pack_index(path.to_str().expect("utf8"));
    assert_eq!(
        str_at(index.get("case_a").expect("case"), &["tool_name"], ""),
        "batch_query"
    );
    assert_eq!(
        str_at(index.get("case_a").expect("case"), &["input", "query"], ""),
        "hello"
    );
}

#[test]
fn ignores_null_pending_tool_request_in_research_report() {
    let path = std::env::temp_dir().join("web_tooling_request_pack_ignore_null.json");
    let report = json!({
        "cases": [
            {
                "case_id": "case_a",
                "response_diagnostics": {
                    "pending_tool_request": null
                }
            }
        ]
    });
    std::fs::write(&path, serde_json::to_vec(&report).unwrap()).expect("write report");
    let index = load_request_pack_index(path.to_str().expect("utf8"));
    assert!(index.get("case_a").is_none());
}

#[test]
fn synthetic_payload_exposes_direct_tool_artifacts_to_retrieval_grader() {
    let case = json!({
        "id": "case_a",
        "prompt": "Compare LangGraph and CrewAI",
        "expected_gate_path": {
            "gate_1": "tool_required",
            "gate_2": "web_research",
            "gate_3": "batch_query",
            "gate_4_required_fields": ["query", "aperture"]
        },
        "required_entities": ["LangGraph", "CrewAI"]
    });
    let request = json!({
        "query": "Compare LangGraph and CrewAI",
        "queries": ["Compare LangGraph and CrewAI"],
        "keywords": ["LangGraph", "CrewAI"],
        "required_coverage": {
            "entities": ["LangGraph", "CrewAI"],
            "facets": ["comparison"]
        },
        "aperture": "medium",
        "source": "web"
    });
    let direct_payload = json!({
        "status": "ok",
        "provider_results": [
            {"title": "LangGraph vs CrewAI docs", "snippet": "LangGraph and CrewAI are both agent frameworks used for production AI agents, with LangGraph emphasizing stateful orchestration and CrewAI emphasizing role-based coordination."}
        ],
        "evidence_refs": [
            {"title": "LangGraph vs CrewAI docs", "snippet": "LangGraph and CrewAI are both agent frameworks used for production AI agents, with LangGraph emphasizing stateful orchestration and CrewAI emphasizing role-based coordination.", "claim_hints": ["stateful orchestration", "role-based coordination"], "source_domain": "langchain.com"}
        ],
        "tool_result_quality": {
            "claim_hint_count": 2,
            "content_rich_candidate_count": 1
        }
    });
    let payload = synthesize_tooling_eval_payload("batch_query", &request, &direct_payload);
    let grade = grade_case(&case, &payload, 85, 95);
    assert_eq!(str_at(&grade.retrieval_quality, &["status"], ""), "usable");
    assert!(grade
        .retrieval_quality
        .get("usable_evidence")
        .and_then(Value::as_bool)
        .unwrap_or(false));
}
