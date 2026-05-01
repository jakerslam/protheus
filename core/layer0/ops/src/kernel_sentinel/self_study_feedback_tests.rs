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
    assert!(rows[0]["feedback_quality_score"].as_u64().unwrap() > 0);
    assert_eq!(rows[0]["feedback_quality_rank"], 1);
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
    assert_eq!(rows[0]["todo_actionability_state"], "todo_ready");
    assert!(
        rows[0]["feedback_quality_score"].as_u64().unwrap()
            > rows[1]["feedback_quality_score"].as_u64().unwrap()
    );
}

#[test]
fn generic_recommendations_require_root_cause_synthesis_before_auto_todo() {
    let report = json!({
        "findings": [
            {
                "status": "open",
                "severity": "critical",
                "category": "receipt_integrity",
                "fingerprint": "verity_receipts:drift_events",
                "summary": "kernel receipt drift exceeded policy",
                "recommended_action": "inspect deterministic kernel evidence and restore fail-closed behavior",
                "evidence": ["runtime://verity/drift_events"]
            }
        ]
    });

    let rows = build_feedback_inbox(&report, "2026-05-01T00:00:00Z");

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["todo_actionability_state"], "needs_root_cause_synthesis");
    assert_eq!(
        rows[0]["feedback_to_todo_contract"]["generic_recommendation_blocks_auto_todo"],
        true
    );
    assert_eq!(
        rows[0]["quality_signals"]["todo_actionability_state"],
        "needs_root_cause_synthesis"
    );
}

#[test]
fn authority_ghost_findings_become_structural_todo_ready_feedback() {
    let report = json!({
        "findings": [
            {
                "status": "open",
                "severity": "critical",
                "category": "security_boundary",
                "fingerprint": "nexus_boundary:shell_truth_leak:unknown_layer:shell_truth_leak_guard_current",
                "summary": "shell_truth_leak_guard_current violates shell_truth_leak across unknown_layer",
                "recommended_action": "move truth inference back to Kernel authority and expose only backend state",
                "evidence": [
                    "field://shell/projection_only=false",
                    "check://shell_truth_leak_guard/pass=false"
                ]
            }
        ]
    });

    let rows = build_feedback_inbox(&report, "2026-05-01T00:00:00Z");

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["todo_actionability_state"], "todo_ready");
    assert_eq!(rows[0]["root_cause_profile"]["kind"], "authority_ghost");
    assert_eq!(rows[0]["root_cause_profile"]["structural_signal"], true);
    assert_eq!(rows[0]["root_cause_profile"]["symptom_patching_risk"], true);
    assert_eq!(
        rows[0]["root_cause_profile"]["next_step"],
        "promote_to_actionable_todo"
    );
    assert_eq!(rows[0]["quality_signals"]["root_cause_kind"], "authority_ghost");
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
