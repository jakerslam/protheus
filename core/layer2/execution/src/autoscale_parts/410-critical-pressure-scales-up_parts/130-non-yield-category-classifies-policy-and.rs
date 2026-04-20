    #[test]
    fn non_yield_category_classifies_policy_and_safety_and_progress() {
        let budget_hold = compute_non_yield_category(&NonYieldCategoryInput {
            event_type: Some("autonomy_run".to_string()),
            result: Some("no_candidates_policy_daily_cap".to_string()),
            outcome: None,
            policy_hold: Some(true),
            hold_reason: Some("budget guard blocked".to_string()),
            route_block_reason: None,
        });
        assert_eq!(budget_hold.category, Some("budget_hold".to_string()));

        let safety = compute_non_yield_category(&NonYieldCategoryInput {
            event_type: Some("autonomy_run".to_string()),
            result: Some("stop_repeat_gate_human_escalation_pending".to_string()),
            outcome: None,
            policy_hold: Some(false),
            hold_reason: None,
            route_block_reason: None,
        });
        assert_eq!(safety.category, Some("safety_stop".to_string()));

        let no_progress = compute_non_yield_category(&NonYieldCategoryInput {
            event_type: Some("autonomy_run".to_string()),
            result: Some("executed".to_string()),
            outcome: Some("no_change".to_string()),
            policy_hold: Some(false),
            hold_reason: None,
            route_block_reason: None,
        });
        assert_eq!(no_progress.category, Some("no_progress".to_string()));
    }

    #[test]
    fn autoscale_json_non_yield_category_path_works() {
        let payload = serde_json::json!({
            "mode": "non_yield_category",
            "non_yield_category_input": {
                "event_type": "autonomy_run",
                "result": "executed",
                "outcome": "no_change",
                "policy_hold": false,
                "hold_reason": "",
                "route_block_reason": ""
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale non_yield_category");
        assert!(out.contains("\"mode\":\"non_yield_category\""));
    }

    #[test]
    fn non_yield_reason_prefers_explicit_then_falls_back() {
        let explicit = compute_non_yield_reason(&NonYieldReasonInput {
            category: Some("policy_hold".to_string()),
            hold_reason: Some("Gate Manual".to_string()),
            route_block_reason: None,
            reason: None,
            result: Some("stop_init_gate_readiness".to_string()),
            outcome: None,
        });
        assert_eq!(explicit.reason, "gate_manual");

        let no_progress_executed = compute_non_yield_reason(&NonYieldReasonInput {
            category: Some("no_progress".to_string()),
            hold_reason: None,
            route_block_reason: None,
            reason: None,
            result: Some("executed".to_string()),
            outcome: Some("no_change".to_string()),
        });
        assert_eq!(no_progress_executed.reason, "executed_no_change");

        let fallback = compute_non_yield_reason(&NonYieldReasonInput {
            category: Some("safety_stop".to_string()),
            hold_reason: None,
            route_block_reason: None,
            reason: None,
            result: None,
            outcome: None,
        });
        assert_eq!(fallback.reason, "safety_stop_unknown");
    }

    #[test]
    fn autoscale_json_non_yield_reason_path_works() {
        let payload = serde_json::json!({
            "mode": "non_yield_reason",
            "non_yield_reason_input": {
                "category": "no_progress",
                "result": "executed",
                "outcome": "no_change"
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale non_yield_reason");
        assert!(out.contains("\"mode\":\"non_yield_reason\""));
    }

    #[test]
    fn proposal_type_from_run_event_prefers_direct_then_capability_key() {
        let direct = compute_proposal_type_from_run_event(&ProposalTypeFromRunEventInput {
            event_type: Some("autonomy_run".to_string()),
            proposal_type: Some("Unknown".to_string()),
            capability_key: Some("proposal:directive".to_string()),
        });
        assert_eq!(direct.proposal_type, "unknown".to_string());

        let derived = compute_proposal_type_from_run_event(&ProposalTypeFromRunEventInput {
            event_type: Some("autonomy_run".to_string()),
            proposal_type: Some(String::new()),
            capability_key: Some("proposal:directive".to_string()),
        });
        assert_eq!(derived.proposal_type, "directive".to_string());
    }

    #[test]
    fn autoscale_json_proposal_type_from_run_event_path_works() {
        let payload = serde_json::json!({
            "mode": "proposal_type_from_run_event",
            "proposal_type_from_run_event_input": {
                "event_type": "autonomy_run",
                "proposal_type": "",
                "capability_key": "proposal:unknown"
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale proposal_type_from_run_event");
        assert!(out.contains("\"mode\":\"proposal_type_from_run_event\""));
    }

    #[test]
    fn run_event_objective_id_uses_truthy_priority_then_sanitizes() {
        let from_objective = compute_run_event_objective_id(&RunEventObjectiveIdInput {
            directive_pulse_present: Some(false),
            directive_pulse_objective_id: Some(String::new()),
            objective_id_present: Some(true),
            objective_id: Some("T1_alpha".to_string()),
            objective_binding_present: Some(true),
            objective_binding_objective_id: Some("T1_beta".to_string()),
            top_escalation_present: Some(true),
            top_escalation_objective_id: Some("T1_gamma".to_string()),
        });
        assert_eq!(from_objective.objective_id, "T1_alpha".to_string());

        let blocked_by_truthy_invalid = compute_run_event_objective_id(&RunEventObjectiveIdInput {
            directive_pulse_present: Some(true),
            directive_pulse_objective_id: Some("   ".to_string()),
            objective_id_present: Some(true),
            objective_id: Some("T1_valid".to_string()),
            objective_binding_present: Some(false),
            objective_binding_objective_id: Some(String::new()),
            top_escalation_present: Some(false),
            top_escalation_objective_id: Some(String::new()),
        });
        assert_eq!(blocked_by_truthy_invalid.objective_id, String::new());
    }

    #[test]
    fn autoscale_json_run_event_objective_id_path_works() {
        let payload = serde_json::json!({
            "mode": "run_event_objective_id",
            "run_event_objective_id_input": {
                "directive_pulse_present": false,
                "directive_pulse_objective_id": "",
                "objective_id_present": true,
                "objective_id": "T1_alpha",
                "objective_binding_present": false,
                "objective_binding_objective_id": "",
                "top_escalation_present": false,
                "top_escalation_objective_id": ""
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale run_event_objective_id");
        assert!(out.contains("\"mode\":\"run_event_objective_id\""));
    }

    #[test]
    fn run_event_proposal_id_uses_truthy_priority_then_normalizes_spaces() {
        let from_direct = compute_run_event_proposal_id(&RunEventProposalIdInput {
            proposal_id_present: Some(true),
            proposal_id: Some("  p-001  ".to_string()),
            selected_proposal_id_present: Some(true),
            selected_proposal_id: Some("p-002".to_string()),
            top_escalation_present: Some(true),
            top_escalation_proposal_id: Some("p-003".to_string()),
        });
        assert_eq!(from_direct.proposal_id, "p-001".to_string());

        let from_selected = compute_run_event_proposal_id(&RunEventProposalIdInput {
            proposal_id_present: Some(false),
            proposal_id: Some(String::new()),
            selected_proposal_id_present: Some(true),
            selected_proposal_id: Some(" selected   proposal ".to_string()),
            top_escalation_present: Some(true),
            top_escalation_proposal_id: Some("p-003".to_string()),
        });
        assert_eq!(from_selected.proposal_id, "selected proposal".to_string());
    }

    #[test]
    fn autoscale_json_run_event_proposal_id_path_works() {
        let payload = serde_json::json!({
            "mode": "run_event_proposal_id",
            "run_event_proposal_id_input": {
                "proposal_id_present": false,
                "proposal_id": "",
                "selected_proposal_id_present": true,
                "selected_proposal_id": "p-009",
                "top_escalation_present": false,
                "top_escalation_proposal_id": ""
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale run_event_proposal_id");
        assert!(out.contains("\"mode\":\"run_event_proposal_id\""));
    }

    #[test]
    fn capacity_counted_attempt_event_classifies_expected_cases() {
        let executed = compute_capacity_counted_attempt_event(&CapacityCountedAttemptEventInput {
            event_type: Some("autonomy_run".to_string()),
            result: Some("executed".to_string()),
            policy_hold: Some(false),
            proposal_id: Some(String::new()),
        });
        assert!(executed.capacity_counted);

        let policy_hold =
            compute_capacity_counted_attempt_event(&CapacityCountedAttemptEventInput {
                event_type: Some("autonomy_run".to_string()),
                result: Some("stop_init_gate_readiness".to_string()),
                policy_hold: Some(false),
                proposal_id: Some("p-001".to_string()),
            });
        assert!(!policy_hold.capacity_counted);

        let attempt_with_proposal =
            compute_capacity_counted_attempt_event(&CapacityCountedAttemptEventInput {
                event_type: Some("autonomy_run".to_string()),
                result: Some("stop_repeat_gate_candidate_exhausted".to_string()),
                policy_hold: Some(false),
                proposal_id: Some("p-001".to_string()),
            });
        assert!(attempt_with_proposal.capacity_counted);

        let attempt_without_proposal =
            compute_capacity_counted_attempt_event(&CapacityCountedAttemptEventInput {
                event_type: Some("autonomy_run".to_string()),
                result: Some("stop_repeat_gate_candidate_exhausted".to_string()),
                policy_hold: Some(false),
                proposal_id: Some(String::new()),
            });
        assert!(!attempt_without_proposal.capacity_counted);
    }

    #[test]
    fn autoscale_json_capacity_counted_attempt_event_path_works() {
        let payload = serde_json::json!({
            "mode": "capacity_counted_attempt_event",
            "capacity_counted_attempt_event_input": {
                "event_type": "autonomy_run",
                "result": "stop_repeat_gate_candidate_exhausted",
                "policy_hold": false,
                "proposal_id": "p-001"
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale capacity_counted_attempt_event");
        assert!(out.contains("\"mode\":\"capacity_counted_attempt_event\""));
    }

    #[test]
    fn repeat_gate_anchor_builds_binding_only_with_objective_id() {
        let out = compute_repeat_gate_anchor(&RepeatGateAnchorInput {
            proposal_id: Some(" p-001 ".to_string()),
            objective_id: Some("T1_alpha".to_string()),
            objective_binding_present: Some(true),
            objective_binding_pass: Some(false),
            objective_binding_required: Some(true),
            objective_binding_source: Some("".to_string()),
            objective_binding_valid: Some(false),
        });
        assert_eq!(out.proposal_id, Some("p-001".to_string()));
        assert_eq!(out.objective_id, Some("T1_alpha".to_string()));
        assert!(out.objective_binding.is_some());
        let binding = out.objective_binding.expect("binding");
        assert!(!binding.pass);
        assert!(binding.required);
        assert_eq!(binding.source, "repeat_gate_anchor".to_string());
        assert!(!binding.valid);
    }

    #[test]
    fn autoscale_json_repeat_gate_anchor_path_works() {
        let payload = serde_json::json!({
            "mode": "repeat_gate_anchor",
            "repeat_gate_anchor_input": {
                "proposal_id": "p-002",
                "objective_id": "T1_alpha",
                "objective_binding_present": true,
                "objective_binding_pass": true,
                "objective_binding_required": false,
                "objective_binding_source": "repeat_gate_anchor",
                "objective_binding_valid": true
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale repeat_gate_anchor");
        assert!(out.contains("\"mode\":\"repeat_gate_anchor\""));
    }

    #[test]
    fn route_execution_policy_hold_maps_summary_fields() {
        let out = compute_route_execution_policy_hold(&RouteExecutionPolicyHoldInput {
            target: Some("route".to_string()),
            gate_decision: Some("allow".to_string()),
            route_decision_raw: None,
            decision: Some("manual".to_string()),
            needs_manual_review: Some(false),
            executable: Some(false),
            budget_block_reason: None,
            budget_enforcement_reason: None,
            budget_global_reason: None,
            summary_reason: Some("manual route".to_string()),
            route_reason: None,
            budget_blocked: Some(false),
            budget_global_blocked: Some(false),
            budget_enforcement_blocked: Some(false),
        });
        assert!(out.hold);
        assert_eq!(out.hold_scope, Some("proposal".to_string()));
        assert_eq!(out.hold_reason, Some("gate_manual".to_string()));
    }

