// SPDX-License-Identifier: Apache-2.0
// SRS coverage: V6-WORKFLOW-012.1, V6-WORKFLOW-012.2, V6-WORKFLOW-012.3,
// V6-WORKFLOW-012.4, V6-WORKFLOW-012.5, V6-WORKFLOW-012.6, V6-WORKFLOW-012.7,
// V6-WORKFLOW-012.8

use protheus_ops_core::haystack_bridge;
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

fn run_bridge(root: &Path, args: &[String]) -> i32 {
    haystack_bridge::run(root, args)
}

fn read_json(path: &Path) -> Value {
    serde_json::from_str(&fs::read_to_string(path).expect("read json")).expect("parse json")
}

fn latest_receipt(state_path: &Path) -> Value {
    read_json(state_path)
        .get("last_receipt")
        .cloned()
        .expect("last receipt")
}

fn extend_claim_ids(claim_ids: &mut BTreeSet<String>, receipt: &Value) {
    let rows = receipt
        .get("payload")
        .and_then(|v| v.get("claim_evidence"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for row in rows {
        if let Some(id) = row.get("id").and_then(Value::as_str) {
            claim_ids.insert(id.to_string());
        }
    }
}

#[test]
fn workflow_012_pipeline_agent_template_rag_route_eval_trace_and_connector_intake_emit_receipts() {
    let root = tempfile::tempdir().expect("tempdir");
    let state_path = root.path().join("state/haystack/latest.json");
    let history_path = root.path().join("state/haystack/history.jsonl");
    let swarm_state_path = root.path().join("state/haystack/swarm.json");
    let mut claim_ids = BTreeSet::<String>::new();

    let pipeline_payload = json!({
        "name": "incident-pipeline",
        "components": [
            {"id": "retrieve", "stage_type": "retriever", "parallel": true, "budget": 192},
            {"id": "rank", "stage_type": "ranker", "parallel": true, "budget": 160},
            {"id": "answer", "stage_type": "generator", "spawn": true, "budget": 256}
        ]
    });
    assert_eq!(
        run_bridge(
            root.path(),
            &[
                "register-pipeline".to_string(),
                format!("--payload={pipeline_payload}"),
                format!("--state-path={}", state_path.display()),
                format!("--history-path={}", history_path.display()),
            ],
        ),
        0
    );
    let pipeline_receipt = latest_receipt(&state_path);
    extend_claim_ids(&mut claim_ids, &pipeline_receipt);
    assert_eq!(
        pipeline_receipt["payload"]["claim_evidence"][0]["id"].as_str(),
        Some("V6-WORKFLOW-012.1")
    );
    let pipeline_id = pipeline_receipt["payload"]["pipeline"]["pipeline_id"]
        .as_str()
        .expect("pipeline id")
        .to_string();

    assert_eq!(
        run_bridge(
            root.path(),
            &[
                "run-pipeline".to_string(),
                format!(
                    "--payload={}",
                    json!({"pipeline_id": pipeline_id, "profile": "pure"})
                ),
                format!("--state-path={}", state_path.display()),
                format!("--history-path={}", history_path.display()),
                format!("--swarm-state-path={}", swarm_state_path.display()),
            ],
        ),
        0
    );
    let run_receipt = latest_receipt(&state_path);
    extend_claim_ids(&mut claim_ids, &run_receipt);
    assert_eq!(
        run_receipt["payload"]["run"]["degraded"].as_bool(),
        Some(true)
    );

    assert_eq!(
        run_bridge(
            root.path(),
            &[
                "run-agent-toolset".to_string(),
                format!(
                    "--payload={}",
                    json!({
                        "name": "incident-agent",
                        "goal": "triage billing incident",
                        "search_limit": 2,
                        "tools": [
                            {"name": "billing_lookup", "description": "billing incident ledger lookup", "tags": ["billing", "incident"]},
                            {"name": "general_faq", "description": "general frequently asked questions", "tags": ["faq"]},
                            {"name": "ops_console", "description": "operational incident tool", "tags": ["incident"]}
                        ]
                    })
                ),
                format!("--state-path={}", state_path.display()),
                format!("--history-path={}", history_path.display()),
                format!("--swarm-state-path={}", swarm_state_path.display()),
            ],
        ),
        0
    );
    let agent_receipt = latest_receipt(&state_path);
    extend_claim_ids(&mut claim_ids, &agent_receipt);
    assert_eq!(
        agent_receipt["payload"]["claim_evidence"][0]["id"].as_str(),
        Some("V6-WORKFLOW-012.2")
    );
    assert_eq!(
        agent_receipt["payload"]["agent"]["selected_tools"]
            .as_array()
            .map(|v| !v.is_empty()),
        Some(true)
    );

    assert_eq!(
        run_bridge(
            root.path(),
            &[
                "register-template".to_string(),
                format!(
                    "--payload={}",
                    json!({"name": "incident-template", "template": "Answer {{question}} with {{context}}"})
                ),
                format!("--state-path={}", state_path.display()),
                format!("--history-path={}", history_path.display()),
            ],
        ),
        0
    );
    let template_receipt = latest_receipt(&state_path);
    extend_claim_ids(&mut claim_ids, &template_receipt);
    let template_id = template_receipt["payload"]["template"]["template_id"]
        .as_str()
        .expect("template id")
        .to_string();
    assert_eq!(
        run_bridge(
            root.path(),
            &[
                "render-template".to_string(),
                format!(
                    "--payload={}",
                    json!({"template_id": template_id, "variables": {"question": "What happened?", "context": "billing service degraded"}})
                ),
                format!("--state-path={}", state_path.display()),
                format!("--history-path={}", history_path.display()),
            ],
        ),
        0
    );
    let render_receipt = latest_receipt(&state_path);
    extend_claim_ids(&mut claim_ids, &render_receipt);
    assert_eq!(
        render_receipt["payload"]["render"]["output"].as_str(),
        Some("Answer What happened? with billing service degraded")
    );

    assert_eq!(
        run_bridge(
            root.path(),
            &[
                "register-document-store".to_string(),
                format!(
                    "--payload={}",
                    json!({
                        "name": "incident-docs",
                        "documents": [
                            {"text": "billing incident playbook", "metadata": {"kind": "graph", "source": "playbook"}},
                            {"text": "general faq on accounts", "metadata": {"kind": "faq", "source": "faq"}}
                        ]
                    })
                ),
                format!("--state-path={}", state_path.display()),
                format!("--history-path={}", history_path.display()),
            ],
        ),
        0
    );
    let store_receipt = latest_receipt(&state_path);
    extend_claim_ids(&mut claim_ids, &store_receipt);
    let store_id = store_receipt["payload"]["document_store"]["store_id"]
        .as_str()
        .expect("store id")
        .to_string();
    assert_eq!(
        run_bridge(
            root.path(),
            &[
                "retrieve-documents".to_string(),
                format!(
                    "--payload={}",
                    json!({"store_id": store_id, "query": "billing incident", "mode": "hybrid", "profile": "pure", "top_k": 4})
                ),
                format!("--state-path={}", state_path.display()),
                format!("--history-path={}", history_path.display()),
            ],
        ),
        0
    );
    let retrieval_receipt = latest_receipt(&state_path);
    extend_claim_ids(&mut claim_ids, &retrieval_receipt);
    assert_eq!(
        retrieval_receipt["payload"]["claim_evidence"][0]["id"].as_str(),
        Some("V6-WORKFLOW-012.4")
    );

    assert_eq!(
        run_bridge(
            root.path(),
            &[
                "route-and-rank".to_string(),
                format!(
                    "--payload={}",
                    json!({
                        "name": "incident-router",
                        "query": "billing escalation",
                        "context": {"intent": "billing"},
                        "routes": [
                            {"id": "billing", "field": "intent", "equals": "billing", "reason": "billing route"},
                            {"id": "general", "field": "intent", "equals": "general", "reason": "general route"}
                        ],
                        "candidates": [
                            {"text": "billing policy doc", "metadata": {"kind": "policy"}},
                            {"text": "generic faq", "metadata": {"kind": "faq"}}
                        ]
                    })
                ),
                format!("--state-path={}", state_path.display()),
                format!("--history-path={}", history_path.display()),
            ],
        ),
        0
    );
    let route_receipt = latest_receipt(&state_path);
    extend_claim_ids(&mut claim_ids, &route_receipt);
    assert_eq!(
        route_receipt["payload"]["route"]["selected_route"]["id"].as_str(),
        Some("billing")
    );

    assert_eq!(
        run_bridge(
            root.path(),
            &[
                "record-multimodal-eval".to_string(),
                format!(
                    "--payload={}",
                    json!({
                        "name": "incident-eval",
                        "profile": "pure",
                        "artifacts": [
                            {"media_type": "image/png", "path": "adapters/assets/incident.png"},
                            {"media_type": "text/plain", "path": "adapters/assets/incident.txt"}
                        ],
                        "metrics": {"faithfulness": 0.93}
                    })
                ),
                format!("--state-path={}", state_path.display()),
                format!("--history-path={}", history_path.display()),
            ],
        ),
        0
    );
    let eval_receipt = latest_receipt(&state_path);
    extend_claim_ids(&mut claim_ids, &eval_receipt);
    assert_eq!(
        eval_receipt["payload"]["evaluation"]["degraded"].as_bool(),
        Some(true)
    );

    assert_eq!(
        run_bridge(
            root.path(),
            &[
                "trace-run".to_string(),
                format!(
                    "--payload={}",
                    json!({
                        "trace_id": "incident-trace",
                        "steps": [
                            {"stage": "retrieve", "message": "retrieved evidence"},
                            {"stage": "answer", "message": "drafted response"}
                        ]
                    })
                ),
                format!("--state-path={}", state_path.display()),
                format!("--history-path={}", history_path.display()),
            ],
        ),
        0
    );
    let trace_receipt = latest_receipt(&state_path);
    extend_claim_ids(&mut claim_ids, &trace_receipt);
    assert_eq!(
        trace_receipt["payload"]["claim_evidence"][0]["id"].as_str(),
        Some("V6-WORKFLOW-012.7")
    );

    assert_eq!(
        run_bridge(
            root.path(),
            &[
                "import-connector".to_string(),
                format!(
                    "--payload={}",
                    json!({"name": "haystack-qdrant", "connector_type": "qdrant"})
                ),
                format!("--state-path={}", state_path.display()),
                format!("--history-path={}", history_path.display()),
            ],
        ),
        0
    );
    let connector_receipt = latest_receipt(&state_path);
    extend_claim_ids(&mut claim_ids, &connector_receipt);
    assert_eq!(
        connector_receipt["payload"]["claim_evidence"][0]["id"].as_str(),
        Some("V6-WORKFLOW-012.8")
    );

    assert_eq!(
        run_bridge(
            root.path(),
            &[
                "assimilate-intake".to_string(),
                format!(
                    "--payload={}",
                    json!({"output_dir": "client/runtime/local/state/haystack-shell"})
                ),
                format!("--state-path={}", state_path.display()),
                format!("--history-path={}", history_path.display()),
            ],
        ),
        0
    );
    let intake_receipt = latest_receipt(&state_path);
    extend_claim_ids(&mut claim_ids, &intake_receipt);
    assert_eq!(
        intake_receipt["payload"]["intake"]["files"]
            .as_array()
            .map(|v| v.len()),
        Some(4)
    );

    for claim in [
        "V6-WORKFLOW-012.1",
        "V6-WORKFLOW-012.2",
        "V6-WORKFLOW-012.3",
        "V6-WORKFLOW-012.4",
        "V6-WORKFLOW-012.5",
        "V6-WORKFLOW-012.6",
        "V6-WORKFLOW-012.7",
        "V6-WORKFLOW-012.8",
    ] {
        assert!(claim_ids.contains(claim), "missing workflow claim id={claim}");
    }
}
