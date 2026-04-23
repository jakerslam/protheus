// SPDX-License-Identifier: Apache-2.0
// SRS coverage: V6-WORKFLOW-011.1, V6-WORKFLOW-011.2, V6-WORKFLOW-011.3,
// V6-WORKFLOW-011.4, V6-WORKFLOW-011.5, V6-WORKFLOW-011.6, V6-WORKFLOW-011.7,
// V6-WORKFLOW-011.8, V6-WORKFLOW-011.9

use infring_ops_core::mastra_bridge;
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

fn run_bridge(root: &Path, args: &[String]) -> i32 {
    mastra_bridge::run(root, args)
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

fn assert_tooling_receipt_payload(receipt: &Value) {
    assert!(
        receipt.get("payload").and_then(Value::as_object).is_some(),
        "receipt payload must be an object"
    );
}

#[test]
fn workflow_011_graph_agent_memory_hitl_mcp_eval_route_shell_and_intake_emit_receipts() {
    let root = tempfile::tempdir().expect("tempdir");
    let state_path = root.path().join("state/mastra/latest.json");
    let history_path = root.path().join("state/mastra/history.jsonl");
    let swarm_state_path = root.path().join("state/mastra/swarm.json");
    let approval_queue_path = root.path().join("state/mastra/approvals.yaml");

    let runtime_bridge_payload = json!({
        "name": "mastra-python-gateway",
        "language": "python",
        "provider": "openai-compatible",
        "model_family": "gpt-5",
        "models": ["gpt-5-mini"],
        "bridge_path": "adapters/polyglot/mastra_runtime_bridge.ts",
        "supported_profiles": ["rich", "pure"]
    });
    assert_eq!(
        run_bridge(
            root.path(),
            &[
                "register-runtime-bridge".to_string(),
                format!("--payload={}", runtime_bridge_payload),
                format!("--state-path={}", state_path.display()),
                format!("--history-path={}", history_path.display()),
            ],
        ),
        0
    );
    let runtime_bridge_receipt = latest_receipt(&state_path);
    assert_tooling_receipt_payload(&runtime_bridge_receipt);
    assert_eq!(
        runtime_bridge_receipt["payload"]["claim_evidence"][0]["id"].as_str(),
        Some("V6-WORKFLOW-011.7")
    );
    let runtime_bridge_id = runtime_bridge_receipt["payload"]["runtime_bridge"]["bridge_id"]
        .as_str()
        .expect("runtime bridge id")
        .to_string();

    assert_eq!(
        run_bridge(
            root.path(),
            &[
                "route-model".to_string(),
                format!(
                    "--payload={}",
                    json!({
                        "bridge_id": runtime_bridge_id,
                        "language": "python",
                        "provider": "openai-compatible",
                        "model": "gpt-5-mini",
                        "profile": "pure"
                    })
                ),
                format!("--state-path={}", state_path.display()),
                format!("--history-path={}", history_path.display()),
            ],
        ),
        0
    );
    let route_receipt = latest_receipt(&state_path);
    assert_eq!(
        route_receipt["payload"]["route"]["reason_code"].as_str(),
        Some("polyglot_runtime_requires_rich_profile")
    );

    let graph_payload = json!({
        "name": "incident-graph",
        "entrypoint": "intake",
        "nodes": [
            {"id": "intake", "spawn": false},
            {"id": "research", "parallel": true, "budget": 128},
            {"id": "draft", "parallel": true, "budget": 128}
        ],
        "edges": [
            {"from": "intake", "to": "research"},
            {"from": "intake", "to": "draft"}
        ]
    });
    assert_eq!(
        run_bridge(
            root.path(),
            &[
                "register-graph".to_string(),
                format!("--payload={}", graph_payload),
                format!("--state-path={}", state_path.display()),
                format!("--history-path={}", history_path.display()),
            ],
        ),
        0
    );
    let graph_receipt = latest_receipt(&state_path);
    assert_eq!(
        graph_receipt["payload"]["claim_evidence"][0]["id"].as_str(),
        Some("V6-WORKFLOW-011.1")
    );
    let graph_id = graph_receipt["payload"]["graph"]["graph_id"]
        .as_str()
        .expect("graph id")
        .to_string();

    assert_eq!(
        run_bridge(
            root.path(),
            &[
                "execute-graph".to_string(),
                format!(
                    "--payload={}",
                    json!({"graph_id": graph_id, "profile": "pure"})
                ),
                format!("--state-path={}", state_path.display()),
                format!("--history-path={}", history_path.display()),
                format!("--swarm-state-path={}", swarm_state_path.display()),
            ],
        ),
        0
    );
    let graph_run_receipt = latest_receipt(&state_path);
    assert_eq!(
        graph_run_receipt["payload"]["run"]["degraded"].as_bool(),
        Some(true)
    );

    assert_eq!(
        run_bridge(
            root.path(),
            &[
                "register-mcp-bridge".to_string(),
                format!(
                    "--payload={}",
                    json!({
                        "name": "incident-resource-bridge",
                        "entrypoint": "invoke",
                        "bridge_path": "adapters/protocol/mastra_mcp_bridge.ts",
                        "supported_profiles": ["rich", "pure"],
                        "requires_approval": false,
                        "capabilities": ["tools", "resources"]
                    })
                ),
                format!("--state-path={}", state_path.display()),
                format!("--history-path={}", history_path.display()),
            ],
        ),
        0
    );
    let mcp_receipt = latest_receipt(&state_path);
    assert_tooling_receipt_payload(&mcp_receipt);
    let bridge_id = mcp_receipt["payload"]["mcp_bridge"]["tool_id"]
        .as_str()
        .expect("mcp bridge id")
        .to_string();
    assert_eq!(
        mcp_receipt["payload"]["claim_evidence"][0]["id"].as_str(),
        Some("V6-WORKFLOW-011.5")
    );

    assert_eq!(
        run_bridge(
            root.path(),
            &[
                "run-agent-loop".to_string(),
                format!(
                    "--payload={}",
                    json!({
                        "name": "incident-agent",
                        "instruction": "triage the incident and choose the right tool",
                        "runtime_bridge_id": runtime_bridge_receipt["payload"]["runtime_bridge"]["bridge_id"],
                        "language": "python",
                        "provider": "openai-compatible",
                        "model": "gpt-5-mini",
                        "profile": "rich",
                        "tools": [{"tool_id": bridge_id, "budget": 96}],
                        "max_iterations": 2
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
    let run_id = agent_receipt["payload"]["agent"]["agent_id"]
        .as_str()
        .expect("agent id")
        .to_string();
    assert_eq!(
        agent_receipt["payload"]["claim_evidence"][0]["id"].as_str(),
        Some("V6-WORKFLOW-011.2")
    );

    assert_eq!(
        run_bridge(
            root.path(),
            &[
                "memory-recall".to_string(),
                format!(
                    "--payload={}",
                    json!({"query": "incident policy", "top": 3, "profile": "pure"})
                ),
                format!("--state-path={}", state_path.display()),
                format!("--history-path={}", history_path.display()),
            ],
        ),
        0
    );
    let recall_receipt = latest_receipt(&state_path);
    assert_eq!(
        recall_receipt["payload"]["claim_evidence"][0]["id"].as_str(),
        Some("V6-WORKFLOW-011.3")
    );

    assert_eq!(
        run_bridge(
            root.path(),
            &[
                "suspend-run".to_string(),
                format!(
                    "--payload={}",
                    json!({
                        "run_id": run_id,
                        "summary": "operator review needed",
                        "reason": "escalate before execute",
                        "require_approval": true
                    })
                ),
                format!("--state-path={}", state_path.display()),
                format!("--history-path={}", history_path.display()),
                format!("--approval-queue-path={}", approval_queue_path.display()),
            ],
        ),
        0
    );
    let suspend_receipt = latest_receipt(&state_path);
    let action_id = suspend_receipt["payload"]["suspension"]["approval"]["action_id"]
        .as_str()
        .expect("approval action id")
        .to_string();

    assert_eq!(
        run_bridge(
            root.path(),
            &[
                "approval-checkpoint".to_string(),
                format!(
                    "--payload={}",
                    json!({"action_id": action_id, "decision": "approve"})
                ),
                format!("--state-path={}", state_path.display()),
                format!("--history-path={}", history_path.display()),
                format!("--approval-queue-path={}", approval_queue_path.display()),
            ],
        ),
        0
    );
    assert_eq!(
        run_bridge(
            root.path(),
            &[
                "resume-run".to_string(),
                format!("--payload={}", json!({"run_id": run_id})),
                format!("--state-path={}", state_path.display()),
                format!("--history-path={}", history_path.display()),
                format!("--swarm-state-path={}", swarm_state_path.display()),
                format!("--approval-queue-path={}", approval_queue_path.display()),
            ],
        ),
        0
    );
    let resume_receipt = latest_receipt(&state_path);
    assert_eq!(
        resume_receipt["payload"]["resume"]["status"].as_str(),
        Some("resumed")
    );

    assert_eq!(
        run_bridge(
            root.path(),
            &[
                "invoke-mcp-bridge".to_string(),
                format!(
                    "--payload={}",
                    json!({
                        "bridge_id": bridge_id,
                        "profile": "rich",
                        "args": {"resource": "incident-playbook"}
                    })
                ),
                format!("--state-path={}", state_path.display()),
                format!("--history-path={}", history_path.display()),
                format!("--approval-queue-path={}", approval_queue_path.display()),
            ],
        ),
        0
    );
    let invoke_receipt = latest_receipt(&state_path);
    assert_tooling_receipt_payload(&invoke_receipt);
    assert_eq!(
        invoke_receipt["payload"]["claim_evidence"][0]["id"].as_str(),
        Some("V6-WORKFLOW-011.5")
    );

    assert_eq!(
        run_bridge(
            root.path(),
            &[
                "record-eval-trace".to_string(),
                format!(
                    "--payload={}",
                    json!({
                        "session_id": agent_receipt["payload"]["agent"]["primary_session_id"],
                        "profile": "rich",
                        "score": 0.91,
                        "metrics": {"tool_success": 1},
                        "trace": [{"span": "tool-call", "ms": 22}],
                        "token_telemetry": {"prompt_tokens": 120, "completion_tokens": 44},
                        "log_summary": "agent loop traced cleanly"
                    })
                ),
                format!("--state-path={}", state_path.display()),
                format!("--history-path={}", history_path.display()),
            ],
        ),
        0
    );
    let eval_receipt = latest_receipt(&state_path);
    assert_eq!(
        eval_receipt["payload"]["claim_evidence"][0]["id"].as_str(),
        Some("V6-WORKFLOW-011.6")
    );

    assert_eq!(
        run_bridge(
            root.path(),
            &[
                "deploy-shell".to_string(),
                format!(
                    "--payload={}",
                    json!({
                        "shell_name": "mastra-studio",
                        "shell_path": "client/runtime/systems/workflow/mastra_bridge.ts",
                        "target": "local",
                        "artifact_path": "apps/mastra-studio"
                    })
                ),
                format!("--state-path={}", state_path.display()),
                format!("--history-path={}", history_path.display()),
            ],
        ),
        0
    );
    let deploy_receipt = latest_receipt(&state_path);
    assert_eq!(
        deploy_receipt["payload"]["deployment"]["authority_delegate"].as_str(),
        Some("core://mastra-bridge")
    );

    assert_eq!(
        run_bridge(
            root.path(),
            &[
                "scaffold-intake".to_string(),
                format!(
                    "--payload={}",
                    json!({"output_dir": "apps/mastra-shell", "package_name": "mastra-shell"})
                ),
                format!("--state-path={}", state_path.display()),
                format!("--history-path={}", history_path.display()),
            ],
        ),
        0
    );
    let intake_receipt = latest_receipt(&state_path);
    assert_eq!(
        intake_receipt["payload"]["claim_evidence"][0]["id"].as_str(),
        Some("V6-WORKFLOW-011.9")
    );

    assert_eq!(
        run_bridge(
            root.path(),
            &[
                "status".to_string(),
                format!("--state-path={}", state_path.display()),
                format!("--history-path={}", history_path.display()),
            ],
        ),
        0
    );
    let status_receipt = latest_receipt(&state_path);
    assert_eq!(status_receipt["payload"]["graphs"].as_u64(), Some(1));
    assert_eq!(status_receipt["payload"]["graph_runs"].as_u64(), Some(1));
    assert_eq!(status_receipt["payload"]["agent_loops"].as_u64(), Some(1));
    assert_eq!(
        status_receipt["payload"]["memory_recalls"].as_u64(),
        Some(1)
    );
    assert_eq!(
        status_receipt["payload"]["suspended_runs"].as_u64(),
        Some(1)
    );
    assert_eq!(status_receipt["payload"]["mcp_bridges"].as_u64(), Some(1));
    assert_eq!(status_receipt["payload"]["eval_traces"].as_u64(), Some(1));
    assert_eq!(status_receipt["payload"]["deployments"].as_u64(), Some(1));
    assert_eq!(
        status_receipt["payload"]["runtime_bridges"].as_u64(),
        Some(1)
    );
    assert_eq!(status_receipt["payload"]["intakes"].as_u64(), Some(1));
}
