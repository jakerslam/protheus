                "enabled": true,
                "proposal_type": "directive_decomposition",
                "type_in_quarantine_set": true,
                "allow_directive": true,
                "allow_tier1": true,
                "objective_id": "T1_demo",
                "tier1_objective": false
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale unknown_type_quarantine_decision");
        assert!(out.contains("\"mode\":\"unknown_type_quarantine_decision\""));
    }

    #[test]
    fn infer_optimization_delta_prefers_direct_meta_field() {
        let out = compute_infer_optimization_delta(&InferOptimizationDeltaInput {
            optimization_delta_percent: None,
            expected_optimization_percent: Some(12.75),
            expected_delta_percent: Some(9.0),
            estimated_improvement_percent: None,
            target_improvement_percent: None,
            performance_gain_percent: None,
            text_blob: Some("fallback 30%".to_string()),
        });
        assert_eq!(out.delta_percent, Some(12.75));
        assert_eq!(
            out.delta_source.as_deref(),
            Some("meta:expected_optimization_percent")
        );
    }

    #[test]
    fn autoscale_json_infer_optimization_delta_path_works() {
        let payload = serde_json::json!({
            "mode": "infer_optimization_delta",
            "infer_optimization_delta_input": {
                "optimization_delta_percent": null,
                "expected_optimization_percent": null,
                "expected_delta_percent": null,
                "estimated_improvement_percent": null,
                "target_improvement_percent": null,
                "performance_gain_percent": null,
                "text_blob": "target +18% reduction"
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale infer_optimization_delta");
        assert!(out.contains("\"mode\":\"infer_optimization_delta\""));
    }

    #[test]
    fn optimization_intent_proposal_detects_expected_terms() {
        let out = compute_optimization_intent_proposal(&OptimizationIntentProposalInput {
            proposal_type: Some("automation".to_string()),
            blob: Some("optimize latency and throughput".to_string()),
            has_actuation_meta: false,
        });
        assert!(out.intent);
    }

    #[test]
    fn autoscale_json_optimization_intent_proposal_path_works() {
        let payload = serde_json::json!({
            "mode": "optimization_intent_proposal",
            "optimization_intent_proposal_input": {
                "proposal_type": "actuation",
                "blob": "canary smoke test rollout",
                "has_actuation_meta": true
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale optimization_intent_proposal");
        assert!(out.contains("\"mode\":\"optimization_intent_proposal\""));
    }

    #[test]
    fn unlinked_optimization_admission_blocks_high_risk_when_unlinked() {
        let out = compute_unlinked_optimization_admission(&UnlinkedOptimizationAdmissionInput {
            optimization_intent: true,
            proposal_type: Some("optimization".to_string()),
            exempt_types: vec!["directive_clarification".to_string()],
            linked: false,
            normalized_risk: Some("high".to_string()),
            hard_block_high_risk: true,
            penalty: 8.0,
        });
        assert!(out.applies);
        assert!(!out.linked);
        assert!(out.block);
        assert_eq!(
            out.reason.as_deref(),
            Some("optimization_unlinked_objective_high_risk_block")
        );
    }

    #[test]
    fn autoscale_json_unlinked_optimization_admission_path_works() {
        let payload = serde_json::json!({
            "mode": "unlinked_optimization_admission",
            "unlinked_optimization_admission_input": {
                "optimization_intent": true,
                "proposal_type": "optimization",
                "exempt_types": ["directive_clarification"],
                "linked": false,
                "normalized_risk": "low",
                "hard_block_high_risk": true,
                "penalty": 12
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale unlinked_optimization_admission");
        assert!(out.contains("\"mode\":\"unlinked_optimization_admission\""));
    }

    #[test]
    fn optimization_good_enough_fails_when_delta_below_min() {
        let out = compute_optimization_good_enough(&OptimizationGoodEnoughInput {
            applies: true,
            min_delta_percent: 10.0,
            require_delta: true,
            high_accuracy_mode: false,
            normalized_risk: Some("medium".to_string()),
            delta_percent: Some(4.0),
            delta_source: Some("text:%".to_string()),
        });
        assert!(out.applies);
        assert!(!out.pass);
        assert_eq!(out.reason.as_deref(), Some("optimization_good_enough"));
    }

    #[test]
    fn autoscale_json_optimization_good_enough_path_works() {
        let payload = serde_json::json!({
            "mode": "optimization_good_enough",
            "optimization_good_enough_input": {
                "applies": true,
                "min_delta_percent": 8,
                "require_delta": true,
                "high_accuracy_mode": false,
                "normalized_risk": "low",
                "delta_percent": 12,
                "delta_source": "meta:expected_delta_percent"
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale optimization_good_enough");
        assert!(out.contains("\"mode\":\"optimization_good_enough\""));
    }

    #[test]
    fn proposal_dependency_summary_builds_chain_and_edges() {
        let out = compute_proposal_dependency_summary(&ProposalDependencySummaryInput {
            proposal_id: Some("p-1".to_string()),
            decision: Some("accept".to_string()),
            source: Some("directive_decomposition".to_string()),
            parent_objective_id: Some("T1_parent".to_string()),
            created_ids: vec!["T1_child_a".to_string(), "T1_child_b".to_string()],
            dry_run: false,
            created_count: Some(2.0),
            quality_ok: true,
            reason: None,
        });
        assert_eq!(out.decision, "ACCEPT");
        assert_eq!(out.edge_count, 2);
        assert_eq!(out.chain.len(), 3);
        assert_eq!(out.child_objective_ids.len(), 2);
    }

    #[test]
    fn autoscale_json_proposal_dependency_summary_path_works() {
        let payload = serde_json::json!({
            "mode": "proposal_dependency_summary",
            "proposal_dependency_summary_input": {
                "proposal_id": "p-2",
                "decision": "accept",
                "source": "directive_decomposition",
                "parent_objective_id": "T1_parent",
                "created_ids": ["T1_child_a"],
                "dry_run": false,
                "created_count": 1,
                "quality_ok": true,
                "reason": null
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale proposal_dependency_summary");
        assert!(out.contains("\"mode\":\"proposal_dependency_summary\""));
    }

    #[test]
    fn choose_selection_mode_switches_to_explore_on_cadence() {
        let out = compute_choose_selection_mode(&ChooseSelectionModeInput {
            eligible_len: 6,
            executed_count: 4,
            explore_used: 1,
            exploit_used: 3,
            explore_quota: 5,
            every_n: 2,
            min_eligible: 2,
        });
        assert_eq!(out.mode, "explore");
        assert!(out.index >= 1);
    }

    #[test]
    fn autoscale_json_choose_selection_mode_path_works() {
        let payload = serde_json::json!({
            "mode": "choose_selection_mode",
            "choose_selection_mode_input": {
                "eligible_len": 3,
                "executed_count": 1,
                "explore_used": 0,
                "exploit_used": 1,
                "explore_quota": 2,
                "every_n": 1,
                "min_eligible": 2
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale choose_selection_mode");
        assert!(out.contains("\"mode\":\"choose_selection_mode\""));
    }

    #[test]
    fn explore_quota_for_day_clamps_fraction_and_floor() {
        let out = compute_explore_quota_for_day(&ExploreQuotaForDayInput {
            daily_runs_cap: Some(12.0),
            explore_fraction: Some(0.25),
            default_max_runs: 8.0,
        });
        assert_eq!(out.quota, 3);
    }

    #[test]
    fn autoscale_json_explore_quota_for_day_path_works() {
        let payload = serde_json::json!({
            "mode": "explore_quota_for_day",
            "explore_quota_for_day_input": {
                "daily_runs_cap": 10,
                "explore_fraction": 0.2,
                "default_max_runs": 8
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale explore_quota_for_day");
        assert!(out.contains("\"mode\":\"explore_quota_for_day\""));
    }

    #[test]
    fn medium_risk_thresholds_derives_bounds() {
        let out = compute_medium_risk_thresholds(&MediumRiskThresholdsInput {
            base_min_directive_fit: 40.0,
            base_min_actionability_score: 45.0,
            medium_risk_min_composite_eligibility: 70.0,
            min_composite_eligibility: 68.0,
            medium_risk_min_directive_fit: 50.0,
            default_min_directive_fit: 45.0,
            medium_risk_min_actionability: 52.0,
            default_min_actionability: 46.0,
        });
        assert_eq!(out.composite_min, 74.0);
        assert_eq!(out.directive_fit_min, 50.0);
        assert_eq!(out.actionability_min, 52.0);
    }

    #[test]
    fn autoscale_json_medium_risk_thresholds_path_works() {
        let payload = serde_json::json!({
            "mode": "medium_risk_thresholds",
            "medium_risk_thresholds_input": {
                "base_min_directive_fit": 40,
                "base_min_actionability_score": 45,
                "medium_risk_min_composite_eligibility": 70,
                "min_composite_eligibility": 68,
                "medium_risk_min_directive_fit": 50,
                "default_min_directive_fit": 45,
                "medium_risk_min_actionability": 52,
                "default_min_actionability": 46
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale medium_risk_thresholds");
        assert!(out.contains("\"mode\":\"medium_risk_thresholds\""));
    }

    #[test]
    fn medium_risk_gate_decision_flags_low_scores() {
        let out = compute_medium_risk_gate_decision(&MediumRiskGateDecisionInput {
            risk: Some("medium".to_string()),
            composite_score: 60.0,
            directive_fit_score: 55.0,
            actionability_score: 54.0,
            composite_min: 70.0,
            directive_fit_min: 60.0,
            actionability_min: 62.0,
        });
        assert!(!out.pass);
        assert_eq!(out.risk, "medium");
        assert!(out.reasons.contains(&"medium_composite_low".to_string()));
        assert!(out.required.is_some());
    }

    #[test]
    fn autoscale_json_medium_risk_gate_decision_path_works() {
        let payload = serde_json::json!({
            "mode": "medium_risk_gate_decision",
            "medium_risk_gate_decision_input": {
                "risk": "medium",
                "composite_score": 72,
                "directive_fit_score": 68,
                "actionability_score": 66,
                "composite_min": 70,
                "directive_fit_min": 60,
                "actionability_min": 62
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale medium_risk_gate_decision");
        assert!(out.contains("\"mode\":\"medium_risk_gate_decision\""));
    }

    #[test]
    fn route_block_prefilter_blocks_when_rate_exceeded() {
        let out = compute_route_block_prefilter(&RouteBlockPrefilterInput {
            enabled: true,
            capability_key: Some("deploy".to_string()),
            window_hours: 24.0,
            min_observations: 3.0,
            max_block_rate: 0.5,
            row_present: true,
            attempts: 10.0,
            route_blocked: 6.0,
            route_block_rate: 0.6,
        });
        assert!(out.applicable);
        assert!(!out.pass);
        assert_eq!(out.reason, "route_block_rate_exceeded");
    }

    #[test]
    fn autoscale_json_route_block_prefilter_path_works() {
        let payload = serde_json::json!({
            "mode": "route_block_prefilter",
            "route_block_prefilter_input": {
                "enabled": true,
                "capability_key": "deploy",
                "window_hours": 24,
                "min_observations": 3,
                "max_block_rate": 0.5,
                "row_present": true,
                "attempts": 4,
                "route_blocked": 1,
                "route_block_rate": 0.25
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale route_block_prefilter");
        assert!(out.contains("\"mode\":\"route_block_prefilter\""));
    }

    #[test]
    fn route_execution_sample_event_matches_route_logic() {
        let blocked = compute_route_execution_sample_event(&RouteExecutionSampleEventInput {
            event_type: Some("autonomy_run".to_string()),
            result: Some("score_only_fallback_route_block".to_string()),
            execution_target: Some("cell".to_string()),
            route_summary_present: false,
        });
        assert!(blocked.is_sample_event);

        let route_exec = compute_route_execution_sample_event(&RouteExecutionSampleEventInput {
            event_type: Some("autonomy_run".to_string()),
            result: Some("executed".to_string()),
            execution_target: Some("route".to_string()),
            route_summary_present: false,
        });
        assert!(route_exec.is_sample_event);

        let non_sample = compute_route_execution_sample_event(&RouteExecutionSampleEventInput {
            event_type: Some("autonomy_run".to_string()),
            result: Some("no_change".to_string()),
            execution_target: Some("route".to_string()),
            route_summary_present: true,
        });
        assert!(!non_sample.is_sample_event);
    }

    #[test]
    fn autoscale_json_route_execution_sample_event_path_works() {
        let payload = serde_json::json!({
            "mode": "route_execution_sample_event",
            "route_execution_sample_event_input": {
                "event_type": "autonomy_run",
                "result": "executed",
                "execution_target": "route",
                "route_summary_present": false
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale route_execution_sample_event");
        assert!(out.contains("\"mode\":\"route_execution_sample_event\""));
        assert!(out.contains("\"is_sample_event\":true"));
    }

    #[test]
    fn route_block_telemetry_summary_aggregates_by_capability() {
        let out = compute_route_block_telemetry_summary(&RouteBlockTelemetrySummaryInput {
            events: vec![
                RouteBlockTelemetryEventInput {
                    event_type: Some("autonomy_run".to_string()),
                    result: Some("executed".to_string()),
                    execution_target: Some("route".to_string()),
                    route_summary_present: false,
                    capability_key: Some("deploy".to_string()),
                },
                RouteBlockTelemetryEventInput {
                    event_type: Some("autonomy_run".to_string()),
                    result: Some("score_only_fallback_route_block".to_string()),
                    execution_target: Some("cell".to_string()),
                    route_summary_present: false,
                    capability_key: Some("deploy".to_string()),
                },
            ],
            window_hours: 12.0,
        });
        assert_eq!(out.sample_events, 2.0);
        assert_eq!(out.by_capability.len(), 1);
        assert_eq!(out.by_capability[0].key, "deploy");
        assert_eq!(out.by_capability[0].attempts, 2.0);
        assert_eq!(out.by_capability[0].route_blocked, 1.0);
        assert!((out.by_capability[0].route_block_rate - 0.5).abs() < 1e-6);
    }

    #[test]
    fn autoscale_json_route_block_telemetry_summary_path_works() {
        let payload = serde_json::json!({
            "mode": "route_block_telemetry_summary",
            "route_block_telemetry_summary_input": {
                "events": [
                    {
                        "event_type": "autonomy_run",
                        "result": "executed",
                        "execution_target": "route",
                        "route_summary_present": false,
                        "capability_key": "deploy"
                    }
                ],
                "window_hours": 6
            }
