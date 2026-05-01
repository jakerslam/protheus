use super::*;
use serde_json::json;

#[test]
fn feedback_operator_value_ranking_prefers_correctness_security_release_before_optimization() {
    let report = json!({
        "findings": [
            {
                "status": "open",
                "severity": "critical",
                "category": "performance_optimization",
                "fingerprint": "optimize_latency",
                "summary": "latency can be improved",
                "recommended_action": "tune latency",
                "evidence": ["metric://latency/p95"]
            },
            {
                "status": "open",
                "severity": "medium",
                "category": "release_gate",
                "fingerprint": "release_blocked",
                "summary": "release proof is blocked",
                "recommended_action": "repair release proof",
                "evidence": ["check://release/proof=blocked"]
            },
            {
                "status": "open",
                "severity": "medium",
                "category": "security_boundary",
                "fingerprint": "capability_leak",
                "summary": "capability boundary leaked",
                "recommended_action": "repair capability gate",
                "evidence": ["field://security/capability=false"]
            },
            {
                "status": "open",
                "severity": "low",
                "category": "runtime_correctness",
                "fingerprint": "receipt_truth_gap",
                "summary": "receipt truth diverged",
                "recommended_action": "repair receipt invariant",
                "evidence": ["check://runtime/receipt_truth=false"]
            }
        ]
    });

    let rows = build_feedback_inbox(&report, "2026-04-29T00:00:00Z");

    assert_eq!(rows.len(), 4);
    assert_eq!(rows[0]["operator_value_tier"], "correctness");
    assert_eq!(rows[0]["fingerprint"], "receipt_truth_gap");
    assert_eq!(rows[1]["operator_value_tier"], "security");
    assert_eq!(rows[1]["fingerprint"], "capability_leak");
    assert_eq!(rows[2]["operator_value_tier"], "release_blocking");
    assert_eq!(rows[2]["fingerprint"], "release_blocked");
    assert_eq!(rows[3]["operator_value_tier"], "optimization");
    assert_eq!(rows[3]["fingerprint"], "optimize_latency");
    assert_eq!(rows[0]["feedback_quality_rank"], 1);
}

#[test]
fn synthetic_round_failures_collapse_to_one_feedback_item() {
    let report = json!({
        "findings": [
            {
                "status": "open",
                "severity": "medium",
                "category": "runtime_correctness",
                "fingerprint": "synthetic_user_chat_harness:misty_simulated_round01_failures",
                "summary": "round 01 synthetic chat failure",
                "recommended_action": "inspect synthetic chat harness output",
                "evidence": ["synthetic://misty/round01"]
            },
            {
                "status": "open",
                "severity": "high",
                "category": "runtime_correctness",
                "fingerprint": "synthetic_user_chat_harness:misty_simulated_round02_failures",
                "summary": "round 02 synthetic chat failure",
                "recommended_action": "inspect synthetic chat harness output",
                "evidence": ["synthetic://misty/round02"]
            }
        ]
    });

    let rows = build_feedback_inbox(&report, "2026-04-28T00:00:00Z");

    assert_eq!(rows.len(), 1);
    assert_eq!(
        rows[0]["feedback_family_fingerprint"],
        "synthetic_user_chat_harness:misty_simulated_failures"
    );
    assert_eq!(
        rows[0]["dedupe_key"],
        "runtime_correctness:synthetic_user_chat_harness:misty_simulated_failures"
    );
    assert_eq!(rows[0]["severity"], "high");
    assert_eq!(rows[0]["failure_level"], "L2_boundary_contract_breach");
    assert_eq!(rows[0]["root_frame"], "cross_boundary_contract");
    assert_eq!(rows[0]["remediation_level"], "boundary_repair");
    assert_eq!(
        rows[0]["fingerprint"],
        "synthetic_user_chat_harness:misty_simulated_round02_failures"
    );
    assert_eq!(
        rows[0]["evidence"],
        json!(["synthetic://misty/round02", "synthetic://misty/round01"])
    );
    assert_eq!(rows[0]["per_run_evidence"].as_array().unwrap().len(), 2);
    assert_eq!(rows[0]["recurrence_count"], 2);
    assert_eq!(rows[0]["recurrence_threshold"], 2);
    assert_eq!(rows[0]["issue_candidate_ready"], true);
    assert_eq!(rows[0]["todo_actionability_state"], "todo_ready");
    assert_eq!(rows[0]["todo_ready"], true);
    assert_eq!(
        rows[0]["todo_actionability"]["requirements"]["evidence_present"],
        true
    );
    assert_eq!(
        rows[0]["todo_actionability"]["requirements"]["semantic_frame_present"],
        true
    );
    assert!(rows[0]["feedback_quality_score"].as_u64().unwrap() > 0);
    assert_eq!(rows[0]["feedback_quality_rank"], 1);
}

#[test]
fn repeated_symptoms_are_clustered_by_structural_root_cause_family() {
    let report = json!({
        "findings": [
            {
                "status": "open",
                "severity": "high",
                "category": "runtime_correctness",
                "fingerprint": "shell_chat_tool_boxes_outside_bubble",
                "summary": "shell chat rendered tool boxes outside the chat bubble",
                "recommended_action": "repair the shell projection boundary and rerun the chat projection guard",
                "evidence": ["field://shell/chat/tool_box_parent=false"]
            },
            {
                "status": "open",
                "severity": "medium",
                "category": "runtime_correctness",
                "fingerprint": "shell_taskbar_connectivity_offline",
                "summary": "dashboard taskbar connectivity indicator stayed offline while runtime was active",
                "recommended_action": "repair the shell projection boundary and rerun the connectivity projection guard",
                "evidence": ["field://shell/taskbar/connectivity_projected=false"]
            },
            {
                "status": "open",
                "severity": "medium",
                "category": "runtime_correctness",
                "fingerprint": "gateway_health_probe_stale",
                "summary": "gateway health projection was stale",
                "recommended_action": "repair the gateway health projection and rerun the status guard",
                "evidence": ["field://gateway/health/stale=true"]
            }
        ]
    });

    let rows = build_feedback_inbox(&report, "2026-05-01T00:00:00Z");
    let shell_rows = rows
        .iter()
        .filter(|row| row["symptom_surface_family"] == "shell_projection_runtime")
        .collect::<Vec<_>>();
    let gateway_rows = rows
        .iter()
        .filter(|row| row["symptom_surface_family"] == "gateway_boundary_runtime")
        .collect::<Vec<_>>();

    assert_eq!(shell_rows.len(), 2);
    assert_eq!(gateway_rows.len(), 1);
    for row in shell_rows {
        assert_eq!(row["root_cause_cluster_repeated"], true);
        assert_eq!(row["root_cause_cluster_member_count"], 2);
        assert_eq!(
            row["root_cause_cluster"]["policy"],
            "repeated_symptoms_must_be_triaged_as_one_structural_failure_family_before_opening_separate_local_tickets"
        );
        assert_eq!(row["root_cause_cluster"]["members"].as_array().unwrap().len(), 2);
        assert_eq!(
            row["todo_actionability"]["root_cause_cluster_ready"],
            true
        );
    }
    assert_eq!(gateway_rows[0]["root_cause_cluster_repeated"], false);
    assert_eq!(gateway_rows[0]["root_cause_cluster_member_count"], 1);
}

#[test]
fn feedback_quality_ranking_prefers_specific_actionable_evidence_with_same_severity() {
    let report = json!({
        "findings": [
            {
                "status": "open",
                "severity": "high",
                "category": "runtime_correctness",
                "fingerprint": "vague_failure",
                "summary": "runtime failed",
                "recommended_action": "unknown",
                "evidence": ["runtime://vague"]
            },
            {
                "status": "open",
                "severity": "high",
                "category": "runtime_correctness",
                "fingerprint": "specific_failure",
                "summary": "runtime failed with exact field evidence",
                "recommended_action": "repair the failing runtime guard and rerun the exact receipt check",
                "evidence": [
                    "runtime://specific",
                    "field://runtime_observation/specific/ok=false",
                    "check://runtime_observation/specific/failing_check=listener_ready"
                ]
            }
        ]
    });

    let rows = build_feedback_inbox(&report, "2026-04-28T00:00:00Z");

    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["fingerprint"], "specific_failure");
    assert_eq!(rows[0]["feedback_quality_rank"], 1);
    assert_eq!(rows[0]["quality_signals"]["field_citation_count"], 1);
    assert_eq!(rows[0]["quality_signals"]["check_citation_count"], 1);
    assert!(
        rows[0]["feedback_quality_score"].as_u64().unwrap()
            > rows[1]["feedback_quality_score"].as_u64().unwrap()
    );
    assert_eq!(rows[0]["todo_actionability_state"], "triage_to_todo");
    assert_eq!(rows[1]["todo_actionability_state"], "needs_root_cause_synthesis");
    assert_eq!(
        rows[1]["todo_actionability"]["missing_requirements"],
        json!(["concrete_next_action"])
    );
}

#[test]
fn every_feedback_item_declares_todo_actionability_contract() {
    let report = json!({
        "findings": [
            {
                "status": "open",
                "severity": "medium",
                "category": "runtime_correctness",
                "fingerprint": "single_actionable_failure",
                "summary": "runtime failed with a bounded evidence trail",
                "recommended_action": "repair the runtime route and rerun the evidence check",
                "evidence": ["field://runtime/ok=false"]
            },
            {
                "status": "open",
                "severity": "medium",
                "category": "runtime_correctness",
                "fingerprint": "weak_failure",
                "summary": "runtime failed",
                "recommended_action": "unknown",
                "evidence": []
            }
        ]
    });

    let rows = build_feedback_inbox(&report, "2026-05-01T00:00:00Z");

    assert_eq!(rows.len(), 2);
    for row in rows {
        let state = row["todo_actionability_state"].as_str().unwrap();
        assert!(matches!(
            state,
            "todo_ready" | "triage_to_todo" | "needs_root_cause_synthesis"
        ));
        assert_eq!(row["todo_actionability"]["human_review_required"], true);
        assert_eq!(row["todo_actionability"]["safe_to_mutate_todo"], false);
        assert!(row["todo_actionability"]["requirements"].is_object());
        assert!(row["todo_actionability"]["missing_requirements"].is_array());
    }
}

#[test]
fn empty_response_child_variants_are_downranked_once_parent_issue_exists() {
    let report = json!({
        "findings": [
            {
                "status": "open",
                "severity": "high",
                "category": "runtime_correctness",
                "fingerprint": "chat_empty_response_parent",
                "summary": "assistant produced an empty response",
                "recommended_action": "repair empty response finalization",
                "evidence": ["check://chat/final_response=empty"]
            },
            {
                "status": "open",
                "severity": "high",
                "category": "runtime_correctness",
                "fingerprint": "chat_empty_response_parent",
                "summary": "assistant produced a blank response",
                "recommended_action": "repair empty response finalization",
                "evidence": ["field://chat/final_response_length=0"]
            },
            {
                "status": "open",
                "severity": "critical",
                "category": "runtime_correctness",
                "fingerprint": "chat_empty_response_child_variant",
                "summary": "another no response variant appeared in chat",
                "recommended_action": "attach this to the parent issue instead of opening a duplicate",
                "evidence": ["runtime://chat/no_response_variant"]
            },
            {
                "status": "open",
                "severity": "low",
                "category": "runtime_correctness",
                "fingerprint": "distinct_receipt_truth_gap",
                "summary": "receipt truth diverged",
                "recommended_action": "repair receipt truth",
                "evidence": ["check://runtime/receipt_truth=false"]
            }
        ]
    });

    let rows = build_feedback_inbox(&report, "2026-04-29T00:00:00Z");

    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0]["fingerprint"], "chat_empty_response_parent");
    assert_eq!(rows[0]["issue_candidate_ready"], true);
    assert_eq!(rows[0]["downranked_by_parent_issue"], false);
    assert_eq!(rows[1]["fingerprint"], "distinct_receipt_truth_gap");
    assert_eq!(rows[2]["fingerprint"], "chat_empty_response_child_variant");
    assert_eq!(rows[2]["empty_response_variant"], true);
    assert_eq!(rows[2]["empty_response_parent_issue_exists"], true);
    assert_eq!(rows[2]["downranked_by_parent_issue"], true);
    assert_eq!(rows[2]["feedback_quality_rank"], 3);
}
