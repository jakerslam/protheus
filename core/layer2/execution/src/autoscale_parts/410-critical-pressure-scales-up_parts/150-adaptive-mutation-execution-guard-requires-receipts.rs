    #[test]
    fn adaptive_mutation_execution_guard_requires_receipts() {
        let out = compute_adaptive_mutation_execution_guard(&AdaptiveMutationExecutionGuardInput {
            guard_required: true,
            applies: true,
            metadata_applies: true,
            guard_pass: true,
            guard_reason: None,
            safety_attestation: None,
            rollback_receipt: None,
            guard_receipt_id: None,
            mutation_kernel_applies: false,
            mutation_kernel_pass: true,
        });
        assert!(!out.pass);
        assert!(out
            .reasons
            .contains(&"adaptive_mutation_missing_safety_attestation".to_string()));
    }

    #[test]
    fn autoscale_json_adaptive_mutation_guard_path_works() {
        let payload = serde_json::json!({
            "mode": "adaptive_mutation_execution_guard",
            "adaptive_mutation_execution_guard_input": {
                "guard_required": true,
                "applies": true,
                "metadata_applies": false,
                "guard_pass": false,
                "guard_reason": "failed",
                "safety_attestation": "safe-1",
                "rollback_receipt": "roll-1",
                "guard_receipt_id": "guard-1",
                "mutation_kernel_applies": true,
                "mutation_kernel_pass": false
            }
        })
        .to_string();
        let out =
            run_autoscale_json(&payload).expect("autoscale adaptive_mutation_execution_guard");
        assert!(out.contains("\"mode\":\"adaptive_mutation_execution_guard\""));
    }

    #[test]
    fn strategy_selection_chooses_primary_when_canary_not_due() {
        let out = compute_strategy_selection(&StrategySelectionInput {
            date_str: Some("2026-03-04".to_string()),
            attempt_index: 1.0,
            canary_enabled: true,
            canary_allow_execute: false,
            canary_fraction: 0.25,
            max_active: 3.0,
            fallback_strategy_id: Some("fallback".to_string()),
            variants: vec![
                StrategySelectionVariantInput {
                    strategy_id: Some("s-main".to_string()),
                    score: 0.9,
                    confidence: 0.8,
                    stage: Some("stable".to_string()),
                    execution_mode: Some("execute".to_string()),
                },
                StrategySelectionVariantInput {
                    strategy_id: Some("s-canary".to_string()),
                    score: 0.8,
                    confidence: 0.7,
                    stage: Some("trial".to_string()),
                    execution_mode: Some("score_only".to_string()),
                },
            ],
        });
        assert_eq!(out.mode, "primary_best");
        assert_eq!(out.selected_strategy_id.as_deref(), Some("s-main"));
        assert_eq!(out.active_count, 2);
    }

    #[test]
    fn autoscale_json_strategy_selection_path_works() {
        let payload = serde_json::json!({
            "mode": "strategy_selection",
            "strategy_selection_input": {
                "date_str": "2026-03-04",
                "attempt_index": 4,
                "canary_enabled": true,
                "canary_allow_execute": false,
                "canary_fraction": 0.25,
                "max_active": 3,
                "fallback_strategy_id": "fallback",
                "variants": [
                    {
                        "strategy_id": "s-main",
                        "score": 0.9,
                        "confidence": 0.8,
                        "stage": "stable",
                        "execution_mode": "execute"
                    },
                    {
                        "strategy_id": "s-canary",
                        "score": 0.8,
                        "confidence": 0.7,
                        "stage": "trial",
                        "execution_mode": "score_only"
                    }
                ]
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale strategy_selection");
        assert!(out.contains("\"mode\":\"strategy_selection\""));
    }

    #[test]
    fn calibration_deltas_loosen_when_exhausted_and_low_ship_rate() {
        let out = compute_calibration_deltas(&CalibrationDeltasInput {
            executed_count: 8.0,
            shipped_rate: 0.05,
            no_change_rate: 0.7,
            reverted_rate: 0.1,
            exhausted: 4.0,
            min_executed: 6.0,
            tighten_min_executed: 10.0,
            loosen_low_shipped_rate: 0.2,
            loosen_exhausted_threshold: 3.0,
            tighten_min_shipped_rate: 0.2,
            max_delta: 6.0,
        });
        assert_eq!(out.min_signal_quality, -3.0);
        assert_eq!(out.min_directive_fit, -3.0);
        assert_eq!(out.min_actionability_score, -2.0);
        assert_eq!(out.min_sensory_relevance_score, -1.0);
    }

    #[test]
    fn autoscale_json_calibration_deltas_path_works() {
        let payload = serde_json::json!({
            "mode": "calibration_deltas",
            "calibration_deltas_input": {
                "executed_count": 12,
                "shipped_rate": 0.5,
                "no_change_rate": 0.65,
                "reverted_rate": 0.2,
                "exhausted": 3,
                "min_executed": 6,
                "tighten_min_executed": 10,
                "loosen_low_shipped_rate": 0.2,
                "loosen_exhausted_threshold": 3,
                "tighten_min_shipped_rate": 0.2,
                "max_delta": 6
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale calibration_deltas");
        assert!(out.contains("\"mode\":\"calibration_deltas\""));
    }

    #[test]
    fn strategy_admission_decision_blocks_duplicate_window() {
        let out = compute_strategy_admission_decision(&StrategyAdmissionDecisionInput {
            require_admission_preview: false,
            preview_eligible: true,
            preview_blocked_by: vec![],
            mutation_guard: None,
            strategy_type_allowed: true,
            max_risk_per_action: Some(0.8),
            strategy_max_risk_per_action: Some(0.8),
            hard_max_risk_per_action: None,
            risk_score: Some(0.2),
            remediation_check_required: false,
            remediation_depth: None,
            remediation_max_depth: None,
            dedup_key: Some("proposal:key".to_string()),
            duplicate_window_hours: Some(24.0),
            recent_count: Some(2.0),
        });
        assert!(!out.allow);
        assert_eq!(out.reason.as_deref(), Some("strategy_duplicate_window"));
        assert_eq!(out.recent_count, Some(2.0));
    }

    #[test]
    fn autoscale_json_strategy_admission_decision_path_works() {
        let payload = serde_json::json!({
            "mode": "strategy_admission_decision",
            "strategy_admission_decision_input": {
                "require_admission_preview": true,
                "preview_eligible": false,
                "preview_blocked_by": ["preview_gate"],
                "mutation_guard": {
                    "applies": false,
                    "pass": true,
                    "reason": null,
                    "reasons": [],
                    "controls": {}
                },
                "strategy_type_allowed": true
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale strategy_admission_decision");
        assert!(out.contains("\"mode\":\"strategy_admission_decision\""));
    }

    #[test]
    fn expected_value_score_returns_input_score() {
        let out = compute_expected_value_score(&ExpectedValueScoreInput { score: 42.5 });
        assert_eq!(out.score, 42.5);
    }

    #[test]
    fn autoscale_json_expected_value_score_path_works() {
        let payload = serde_json::json!({
            "mode": "expected_value_score",
            "expected_value_score_input": {
                "score": 77.0
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale expected_value_score");
        assert!(out.contains("\"mode\":\"expected_value_score\""));
    }

    #[test]
    fn suggest_run_batch_max_normalizes_values() {
        let out = compute_suggest_run_batch_max(&SuggestRunBatchMaxInput {
            enabled: true,
            batch_max: 3.8,
            batch_reason: Some(" backlog_autoscale ".to_string()),
            daily_remaining: 4.2,
            autoscale_hint: serde_json::json!({"current_cells": 2}),
        });
        assert!(out.enabled);
        assert_eq!(out.max, 3.0);
        assert_eq!(out.reason, "backlog_autoscale");
        assert_eq!(out.daily_remaining, 4.0);
    }

    #[test]
    fn autoscale_json_suggest_run_batch_max_path_works() {
        let payload = serde_json::json!({
            "mode": "suggest_run_batch_max",
            "suggest_run_batch_max_input": {
                "enabled": true,
                "batch_max": 2,
                "batch_reason": "no_pressure",
                "daily_remaining": 6,
                "autoscale_hint": {"state": {"current_cells": 1}}
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale suggest_run_batch_max");
        assert!(out.contains("\"mode\":\"suggest_run_batch_max\""));
    }

    #[test]
    fn backlog_autoscale_snapshot_normalizes_payload() {
        let out = compute_backlog_autoscale_snapshot(&BacklogAutoscaleSnapshotInput {
            enabled: true,
            module: Some(" autonomy_backlog_autoscale ".to_string()),
            state: serde_json::json!({"current_cells": 2}),
            queue: serde_json::json!({"pressure": "warning"}),
            current_cells: 3.8,
            plan: serde_json::json!({"action": "scale_up"}),
            trit_productivity: serde_json::json!({"hold": false}),
        });
        assert!(out.enabled);
        assert_eq!(out.module, "autonomy_backlog_autoscale");
        assert_eq!(out.current_cells, 3.8);
    }

    #[test]
    fn autoscale_json_backlog_autoscale_snapshot_path_works() {
        let payload = serde_json::json!({
            "mode": "backlog_autoscale_snapshot",
            "backlog_autoscale_snapshot_input": {
                "enabled": true,
                "module": "autonomy_backlog_autoscale",
                "state": {"current_cells": 1},
                "queue": {"pressure": "normal"},
                "current_cells": 1,
                "plan": {"action": "hold"},
                "trit_productivity": {"hold": false}
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale backlog_autoscale_snapshot");
        assert!(out.contains("\"mode\":\"backlog_autoscale_snapshot\""));
    }

    #[test]
    fn admission_summary_tallies_blocked_reasons() {
        let out = compute_admission_summary(&AdmissionSummaryInput {
            proposals: vec![
                AdmissionSummaryProposalInput {
                    preview_eligible: Some(true),
                    blocked_by: vec![],
                },
                AdmissionSummaryProposalInput {
                    preview_eligible: Some(false),
                    blocked_by: vec!["policy_hold".to_string(), "risk".to_string()],
                },
                AdmissionSummaryProposalInput {
                    preview_eligible: Some(false),
                    blocked_by: vec![],
                },
            ],
        });
        assert_eq!(out.total, 3);
        assert_eq!(out.eligible, 1);
        assert_eq!(out.blocked, 2);
        assert_eq!(out.blocked_by_reason.get("policy_hold"), Some(&1));
        assert_eq!(out.blocked_by_reason.get("risk"), Some(&1));
        assert_eq!(out.blocked_by_reason.get("unknown"), Some(&1));
    }

    #[test]
    fn autoscale_json_admission_summary_path_works() {
        let payload = serde_json::json!({
            "mode": "admission_summary",
            "admission_summary_input": {
                "proposals": [
                    {"preview_eligible": true, "blocked_by": []},
                    {"preview_eligible": false, "blocked_by": ["manual_gate"]}
                ]
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale admission_summary");
        assert!(out.contains("\"mode\":\"admission_summary\""));
    }

