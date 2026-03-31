#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decompose_generates_micro_tasks() {
        let req = DecomposeRequest {
            run_id: "tdp_test".to_string(),
            goal_id: "goal_test".to_string(),
            goal_text: "Design a creative onboarding campaign and test API endpoint health checks then summarize findings".to_string(),
            objective_id: Some("obj_test".to_string()),
            creator_id: None,
            policy: DecomposePolicy {
                human_lane_keywords: vec!["creative".to_string(), "design".to_string()],
                autonomous_lane_keywords: vec!["test".to_string(), "api".to_string()],
                ..DecomposePolicy::default()
            },
        };
        let out = decompose_goal(&req);
        assert!(!out.is_empty());
        assert!(out.iter().all(|row| !row.micro_task_id.is_empty()));
        assert!(out.iter().all(|row| !row.profile_id.is_empty()));
    }

    #[test]
    fn compose_materializes_profiles_and_routes() {
        let req = ComposeRequest {
            run_id: "tdp_compose_test".to_string(),
            goal_id: "goal_compose".to_string(),
            goal_text: "Build and verify rollout checklist".to_string(),
            objective_id: Some("obj_compose".to_string()),
            creator_id: Some("operator".to_string()),
            policy: ComposePolicy::default(),
            tasks: vec![BaseTask {
                micro_task_id: "mt_a".to_string(),
                goal_id: "goal_compose".to_string(),
                objective_id: Some("obj_compose".to_string()),
                parent_id: None,
                depth: 0,
                index: 0,
                title: "Verify checklist".to_string(),
                task_text: "Verify checklist integrity and publish summary".to_string(),
                estimated_minutes: 3,
                success_criteria: vec!["Execute verification".to_string()],
                required_capability: "quality_check".to_string(),
                profile_id: "task_micro_mt_a".to_string(),
                capability: Capability {
                    capability_id: "quality_check".to_string(),
                    adapter_kind: "shell_task".to_string(),
                    source_type: "analysis".to_string(),
                },
                suggested_lane: "autonomous_micro_agent".to_string(),
                parallel_group: 0,
                parallel_priority: 0.3333,
            }],
        };
        let out = compose_micro_tasks(&req);
        assert_eq!(out.len(), 1);
        let row = out[0].as_object().expect("row should be object");
        assert_eq!(
            row.get("micro_task_id")
                .and_then(|v| v.as_str())
                .unwrap_or(""),
            "mt_a"
        );
        assert!(row.get("profile").is_some());
        assert!(row.get("route").is_some());
    }

    #[test]
    fn summarize_tasks_reports_expected_counts() {
        let tasks = vec![
            json!({
                "route": { "lane": "autonomous_micro_agent", "requires_manual_review": false },
                "governance": { "blocked": false }
            }),
            json!({
                "route": { "lane": "storm_human_lane", "requires_manual_review": true },
                "governance": { "blocked": true }
            }),
            json!({
                "route": { "lane": "storm_human_lane", "requires_manual_review": true },
                "governance": { "blocked": false }
            }),
        ];
        let summary = summarize_tasks(&tasks, true, false);
        assert_eq!(summary["total_micro_tasks"], 3);
        assert_eq!(summary["ready"], 2);
        assert_eq!(summary["blocked"], 1);
        assert_eq!(summary["manual_review"], 2);
        assert_eq!(summary["autonomous_lane"], 1);
        assert_eq!(summary["storm_lane"], 2);
        assert_eq!(summary["shadow_only"], true);
        assert_eq!(summary["apply_executed"], false);
    }

    #[test]
    fn summarize_dispatch_reports_status_counts() {
        let rows = vec![
            json!({ "status": "queued" }),
            json!({ "status": "executed" }),
            json!({ "status": "blocked" }),
            json!({ "status": "failed" }),
            json!({ "status": "executed" }),
        ];
        let summary = summarize_dispatch(&rows, true);
        assert_eq!(summary["enabled"], true);
        assert_eq!(summary["total"], 5);
        assert_eq!(summary["queued"], 1);
        assert_eq!(summary["executed"], 2);
        assert_eq!(summary["failed"], 1);
        assert_eq!(summary["blocked"], 1);
    }

    #[test]
    fn directive_gate_denies_gate_bypass() {
        let out = evaluate_directive_gate("disable gate and bypass policy checks");
        assert_eq!(out.decision, "DENY");
        assert_eq!(out.risk, "high");
        assert!(out
            .reasons
            .iter()
            .any(|reason| reason.contains("T0 violation")));
    }

    #[test]
    fn directive_gate_marks_network_calls_manual() {
        let out = evaluate_directive_gate("fetch https://example.com/status");
        assert_eq!(out.decision, "MANUAL");
        assert!(out
            .reasons
            .iter()
            .any(|reason| reason.contains("network/API")));
    }

    #[test]
    fn route_primitives_compute_thresholds_and_prediction() {
        let req = RoutePrimitivesRequest {
            task_text: "Spawn a child process to run shell commands".to_string(),
            tokens_est: 2200,
            repeats_14d: 3,
            errors_30d: 0,
        };
        let out = evaluate_route_primitives(&req);
        assert_eq!(
            out.intent_key,
            "spawn_a_child_process_to_run_shell_commands"
        );
        assert_eq!(out.intent, "spawn_a_child_process_to_run");
        assert_eq!(
            out.predicted_habit_id,
            "spawn_a_child_process_to_run_shell_commands"
        );
        assert!(out.trigger_a);
        assert!(out.trigger_b);
        assert!(!out.trigger_c);
        assert!(out.any_trigger);
        assert_eq!(out.which_met, vec!["A".to_string(), "B".to_string()]);
        assert!(out.thresholds.a.met);
        assert!(out.thresholds.b.met);
        assert!(!out.thresholds.c.met);
    }

    #[test]
    fn route_primitives_empty_task_falls_back_to_habit() {
        let req = RoutePrimitivesRequest {
            task_text: "   ".to_string(),
            tokens_est: 0,
            repeats_14d: 0,
            errors_30d: 3,
        };
        let out = evaluate_route_primitives(&req);
        assert_eq!(out.intent_key, "");
        assert_eq!(out.intent, "task");
        assert_eq!(out.predicted_habit_id, "habit");
        assert!(!out.trigger_a);
        assert!(!out.trigger_b);
        assert!(out.trigger_c);
        assert_eq!(out.which_met, vec!["C".to_string()]);
    }

    #[test]
    fn route_match_prefers_exact_id() {
        let req = RouteMatchRequest {
            intent_key: "security_scan".to_string(),
            skip_habit_id: String::new(),
            habits: vec![
                RouteMatchHabit {
                    id: "daily_ops".to_string(),
                },
                RouteMatchHabit {
                    id: "security_scan".to_string(),
                },
            ],
        };
        let out = evaluate_route_match(&req);
        assert_eq!(out.matched_habit_id, Some("security_scan".to_string()));
        assert_eq!(out.match_strategy, "exact");
    }

    #[test]
    fn route_match_uses_token_when_exact_missing() {
        let req = RouteMatchRequest {
            intent_key: "please_run_daily_ops_now".to_string(),
            skip_habit_id: String::new(),
            habits: vec![RouteMatchHabit {
                id: "daily_ops".to_string(),
            }],
        };
        let out = evaluate_route_match(&req);
        assert_eq!(out.matched_habit_id, Some("daily_ops".to_string()));
        assert_eq!(out.match_strategy, "token");
    }

    #[test]
    fn route_match_respects_skip_habit() {
        let req = RouteMatchRequest {
            intent_key: "daily_ops".to_string(),
            skip_habit_id: "daily_ops".to_string(),
            habits: vec![RouteMatchHabit {
                id: "daily_ops".to_string(),
            }],
        };
        let out = evaluate_route_match(&req);
        assert_eq!(out.matched_habit_id, None);
        assert_eq!(out.match_strategy, "none");
    }

    #[test]
    fn route_reflex_match_prefers_direct_id() {
        let req = RouteReflexMatchRequest {
            intent_key: "nightly_backup".to_string(),
            task_text: "backup database now".to_string(),
            routines: vec![
                RouteReflexRoutine {
                    id: "database_repair".to_string(),
                    status: "enabled".to_string(),
                    tags: vec!["repair".to_string()],
                },
                RouteReflexRoutine {
                    id: "nightly_backup".to_string(),
                    status: "enabled".to_string(),
                    tags: vec!["backup".to_string()],
                },
            ],
        };
        let out = evaluate_route_reflex_match(&req);
        assert_eq!(out.matched_reflex_id, Some("nightly_backup".to_string()));
        assert_eq!(out.match_strategy, "direct_id");
    }

    #[test]
    fn route_reflex_match_uses_tag_when_direct_missing() {
        let req = RouteReflexMatchRequest {
            intent_key: "unrelated_key".to_string(),
            task_text: "run emergency drift remediation playbook".to_string(),
            routines: vec![
                RouteReflexRoutine {
                    id: "drift_guard".to_string(),
                    status: "enabled".to_string(),
                    tags: vec!["drift".to_string(), "remediation".to_string()],
                },
                RouteReflexRoutine {
                    id: "nightly_backup".to_string(),
                    status: "disabled".to_string(),
                    tags: vec!["backup".to_string()],
                },
            ],
        };
        let out = evaluate_route_reflex_match(&req);
        assert_eq!(out.matched_reflex_id, Some("drift_guard".to_string()));
        assert_eq!(out.match_strategy, "tag");
    }

    #[test]
    fn route_complexity_respects_thresholds() {
        let high = evaluate_route_complexity(&RouteComplexityRequest {
            task_text: "short".to_string(),
            tokens_est: 2500,
            has_match: false,
            any_trigger: false,
        });
        assert_eq!(high.complexity, "high");
        assert_eq!(high.reason, "tokens_est_high");

        let medium = evaluate_route_complexity(&RouteComplexityRequest {
            task_text: "short".to_string(),
            tokens_est: 900,
            has_match: false,
            any_trigger: false,
        });
        assert_eq!(medium.complexity, "medium");
        assert_eq!(medium.reason, "tokens_est_medium");

        let low = evaluate_route_complexity(&RouteComplexityRequest {
            task_text: "short".to_string(),
            tokens_est: 10,
            has_match: false,
            any_trigger: false,
        });
        assert_eq!(low.complexity, "low");
        assert_eq!(low.reason, "default_low");
    }

    #[test]
    fn route_evaluate_combines_primitives_match_reflex_and_complexity() {
        let req = RouteEvaluateRequest {
            task_text: "run nightly backup and drift remediation".to_string(),
            tokens_est: 900,
            repeats_14d: 3,
            errors_30d: 0,
            skip_habit_id: String::new(),
            habits: vec![
                RouteMatchHabit {
                    id: "nightly_backup".to_string(),
                },
                RouteMatchHabit {
                    id: "daily_ops".to_string(),
                },
            ],
            reflex_routines: vec![RouteReflexRoutine {
                id: "drift_guard".to_string(),
                status: "enabled".to_string(),
                tags: vec!["drift".to_string(), "remediation".to_string()],
            }],
        };
        let out = evaluate_route(&req);
        assert!(out.ok);
        assert_eq!(out.intent_key, "run_nightly_backup_and_drift_remediation");
        assert_eq!(out.matched_habit_id, Some("nightly_backup".to_string()));
        assert_eq!(out.matched_reflex_id, Some("drift_guard".to_string()));
        assert_eq!(out.complexity, "medium");
        assert_eq!(out.complexity_reason, "tokens_est_medium");
        assert!(out.trigger_a);
        assert!(!out.trigger_c);
    }

    #[test]
    fn route_decision_prefers_reflex_when_eligible() {
        let req = RouteDecisionRequest {
            matched_reflex_id: "drift_guard".to_string(),
            reflex_eligible: true,
            ..Default::default()
        };
        let out = evaluate_route_decision(&req);
        assert_eq!(out.decision, "RUN_REFLEX");
        assert_eq!(out.reason_code, "reflex_match");
        assert_eq!(out.suggested_habit_id, None);
    }

    #[test]
    fn route_decision_requires_inputs_for_active_habit() {
        let req = RouteDecisionRequest {
            matched_habit_id: "nightly_backup".to_string(),
            matched_habit_state: "active".to_string(),
            has_required_inputs: true,
            required_input_count: 2,
            trusted_entrypoint: true,
            ..Default::default()
        };
        let out = evaluate_route_decision(&req);
        assert_eq!(out.decision, "MANUAL");
        assert_eq!(out.reason_code, "required_inputs");
        assert_eq!(out.suggested_habit_id, Some("nightly_backup".to_string()));
    }

    #[test]
    fn route_decision_runs_active_habit_when_ready() {
        let req = RouteDecisionRequest {
            matched_habit_id: "nightly_backup".to_string(),
            matched_habit_state: "active".to_string(),
            trusted_entrypoint: true,
            ..Default::default()
        };
        let out = evaluate_route_decision(&req);
        assert_eq!(out.decision, "RUN_HABIT");
        assert_eq!(out.reason_code, "active_match");
    }

    #[test]
    fn route_decision_auto_crystallizes_when_triggered_without_match() {
        let req = RouteDecisionRequest {
            any_trigger: true,
            predicted_habit_id: "spawn_a_child_process".to_string(),
            ..Default::default()
        };
        let out = evaluate_route_decision(&req);
        assert_eq!(out.decision, "RUN_CANDIDATE_FOR_VERIFICATION");
        assert_eq!(out.reason_code, "trigger_autocrystallize");
        assert!(out.auto_habit_flow);
        assert_eq!(
            out.suggested_habit_id,
            Some("spawn_a_child_process".to_string())
        );
    }

    #[test]
    fn route_decision_defaults_manual_without_match_or_trigger() {
        let out = evaluate_route_decision(&RouteDecisionRequest::default());
        assert_eq!(out.decision, "MANUAL");
        assert_eq!(out.reason_code, "no_match_no_trigger");
    }

    #[test]
    fn route_habit_readiness_reports_required_inputs() {
        let out = evaluate_route_habit_readiness(&RouteHabitReadinessRequest {
            habit_state: "active".to_string(),
            entrypoint_resolved: "/repo/client/cognition/habits/scripts/run_habit.js".to_string(),
            trusted_entrypoints: vec![
                "/repo/client/cognition/habits/scripts/run_habit.js".to_string()
            ],
            required_inputs: vec!["user_id".to_string(), "scope".to_string()],
        });
        assert_eq!(out.state, "active");
        assert!(!out.runnable);
        assert_eq!(out.reason_code, "required_inputs");
        assert_eq!(out.required_inputs.len(), 2);
    }

    #[test]
    fn route_habit_readiness_reports_untrusted_entrypoint() {
        let out = evaluate_route_habit_readiness(&RouteHabitReadinessRequest {
            habit_state: "candidate".to_string(),
            entrypoint_resolved: "/repo/client/cognition/habits/scripts/untrusted.js".to_string(),
            trusted_entrypoints: vec![
                "/repo/client/cognition/habits/scripts/run_habit.js".to_string()
            ],
            required_inputs: vec![],
        });
        assert_eq!(out.state, "candidate");
        assert!(!out.trusted_entrypoint);
        assert!(!out.runnable);
        assert_eq!(out.reason_code, "untrusted_entrypoint");
    }

    #[test]
    fn route_habit_readiness_reports_runnable_active() {
        let out = evaluate_route_habit_readiness(&RouteHabitReadinessRequest {
            habit_state: "active".to_string(),
            entrypoint_resolved: "/repo/client/cognition/habits/scripts/run_habit.js".to_string(),
            trusted_entrypoints: vec![
                "/repo/client/cognition/habits/scripts/run_habit.js".to_string()
            ],
            required_inputs: vec![],
        });
        assert_eq!(out.state, "active");
        assert!(out.trusted_entrypoint);
        assert!(out.runnable);
        assert_eq!(out.reason_code, "runnable_active");
    }

    #[test]
    fn heroic_gate_blocks_local_destructive_without_purified_row() {
        let req = HeroicGateRequest {
            task_text: "disable all guards immediately".to_string(),
            block_on_destructive: true,
            purified_row: None,
        };
        let out = evaluate_heroic_gate(&req);
        assert_eq!(out.classification, "destructive_instruction");
        assert_eq!(out.decision, "blocked_destructive_local_pattern");
        assert!(out.blocked);
        assert!(out
            .reason_codes
            .iter()
            .any(|code| code == "local_destructive_pattern"));
    }

    #[test]
    fn heroic_gate_uses_purified_row_when_safe() {
        let req = HeroicGateRequest {
            task_text: "summarize sprint progress".to_string(),
            block_on_destructive: true,
            purified_row: Some(json!({
                "classification": "normal",
                "decision": "allow",
                "blocked": false,
                "reason_codes": ["safe_input"]
            })),
        };
        let out = evaluate_heroic_gate(&req);
        assert_eq!(out.classification, "normal");
        assert_eq!(out.decision, "allow");
        assert!(!out.blocked);
        assert!(out.reason_codes.iter().any(|code| code == "safe_input"));
    }

    #[test]
    fn apply_governance_updates_lanes_and_flags() {
        let req = GovernanceApplyRequest {
            policy: GovernanceApplyPolicy {
                default_lane: "autonomous_micro_agent".to_string(),
                storm_lane: "storm_human_lane".to_string(),
                min_storm_share: 0.0,
                block_on_constitution_deny: true,
            },
            rows: vec![
                json!({
                    "suggested_lane": "autonomous_micro_agent",
                    "heroic": {
                        "classification": "normal",
                        "decision": "allow",
                        "blocked": false,
                        "reason_codes": []
                    },
                    "constitution": {
                        "decision": "ALLOW",
                        "risk": "low",
                        "reasons": []
                    },
                    "duality": {
                        "enabled": true,
                        "score_trit": 1,
                        "score_label": "aligned",
                        "zero_point_harmony_potential": 0.8,
                        "recommended_adjustment": "none",
                        "indicator": { "subtle_hint": "ok" }
                    },
                    "task": {
                        "micro_task_id": "mt_1",
                        "task_text": "verify rollout safety",
                        "route": { "lane": "autonomous_micro_agent", "parallel_group": 0, "parallel_priority": 0.5 },
                        "profile": {
                            "routing": {},
                            "provenance": { "evidence": { "decomposition_depth": 0 } }
                        }
                    }
                }),
                json!({
                    "suggested_lane": "autonomous_micro_agent",
                    "heroic": {
                        "classification": "normal",
                        "decision": "allow",
                        "blocked": false,
                        "reason_codes": []
                    },
                    "constitution": {
                        "decision": "MANUAL",
                        "risk": "medium",
                        "reasons": ["human_judgment"]
                    },
                    "duality": {},
                    "task": {
                        "micro_task_id": "mt_2",
                        "task_text": "design campaign direction",
                        "route": { "lane": "autonomous_micro_agent", "parallel_group": 1, "parallel_priority": 0.4 },
                        "profile": {
                            "routing": {},
                            "provenance": { "evidence": { "decomposition_depth": 0 } }
                        }
                    }
                }),
            ],
        };
        let tasks = apply_governance(&req);
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0]["route"]["lane"], "autonomous_micro_agent");
        assert_eq!(tasks[0]["governance"]["blocked"], false);
        assert_eq!(
            tasks[0]["profile"]["routing"]["requires_manual_review"],
            false
        );
        assert_eq!(tasks[0]["duality"]["score_label"], "aligned");

        assert_eq!(tasks[1]["route"]["lane"], "storm_human_lane");
        assert_eq!(tasks[1]["route"]["requires_manual_review"], true);
        assert_eq!(tasks[1]["governance"]["constitution"]["decision"], "MANUAL");
    }

    #[test]
    fn apply_governance_enforces_min_storm_share() {
        let req = GovernanceApplyRequest {
            policy: GovernanceApplyPolicy {
                min_storm_share: 0.34,
                ..GovernanceApplyPolicy::default()
            },
            rows: vec![
                json!({
                    "suggested_lane": "autonomous_micro_agent",
                    "heroic": { "blocked": false },
                    "constitution": { "decision": "ALLOW", "risk": "low", "reasons": [] },
                    "duality": {},
                    "task": { "micro_task_id": "mt_a", "task_text": "a", "route": { "lane": "autonomous_micro_agent" }, "profile": { "routing": {} } }
                }),
                json!({
                    "suggested_lane": "autonomous_micro_agent",
                    "heroic": { "blocked": false },
                    "constitution": { "decision": "ALLOW", "risk": "low", "reasons": [] },
                    "duality": {},
                    "task": { "micro_task_id": "mt_b", "task_text": "b", "route": { "lane": "autonomous_micro_agent" }, "profile": { "routing": {} } }
                }),
                json!({
                    "suggested_lane": "autonomous_micro_agent",
                    "heroic": { "blocked": false },
                    "constitution": { "decision": "ALLOW", "risk": "low", "reasons": [] },
                    "duality": {},
                    "task": { "micro_task_id": "mt_c", "task_text": "c", "route": { "lane": "autonomous_micro_agent" }, "profile": { "routing": {} } }
                }),
            ],
        };
        let tasks = apply_governance(&req);
        let storm_count = tasks
            .iter()
            .filter(|task| task["route"]["lane"] == "storm_human_lane")
            .count();
        assert!(storm_count >= 1);
    }

    #[test]
    fn build_queue_rows_emits_weaver_and_storm_shapes() {
        let req = QueueRowsRequest {
            run_id: "run_a".to_string(),
            goal_id: "goal_a".to_string(),
            objective_id: Some("obj_a".to_string()),
            shadow_only: true,
            passport_id: Some("passport_a".to_string()),
            storm_lane: "storm_human_lane".to_string(),
            tasks: vec![
                json!({
                    "micro_task_id": "mt_1",
                    "profile_id": "p_1",
                    "title": "Task One",
                    "task_text": "Do task one",
                    "estimated_minutes": 2,
                    "success_criteria": ["A"],
                    "route": {
                        "lane": "autonomous_micro_agent",
                        "parallel_group": 0,
                        "parallel_priority": 0.5,
                        "blocked": false,
                        "requires_manual_review": false
                    },
                    "duality": { "indicator": { "subtle_hint": "ok" } },
                    "profile": { "attribution": { "source_goal_id": "goal_a" } }
                }),
                json!({
                    "micro_task_id": "mt_2",
                    "profile_id": "p_2",
                    "title": "Task Two",
                    "task_text": "Do task two",
                    "estimated_minutes": 3,
                    "success_criteria": ["B"],
                    "route": {
                        "lane": "storm_human_lane",
                        "parallel_group": 1,
                        "parallel_priority": 0.3,
                        "blocked": false,
                        "requires_manual_review": true
                    }
                }),
            ],
        };
        let (weaver, storm) = build_queue_rows(&req);
        assert_eq!(weaver.len(), 2);
        assert_eq!(storm.len(), 1);
        assert_eq!(weaver[0]["type"], "task_micro_route_candidate");
        assert_eq!(storm[0]["type"], "storm_micro_task_offer");
        assert_eq!(storm[0]["micro_task_id"], "mt_2");
    }

    #[test]
    fn build_dispatch_rows_emits_executor_and_status() {
        let req = DispatchRowsRequest {
            run_id: "run_dispatch".to_string(),
            goal_id: "goal_dispatch".to_string(),
            objective_id: Some("obj_dispatch".to_string()),
            shadow_only: false,
            apply_executed: true,
            passport_id: Some("passport_dispatch".to_string()),
            storm_lane: "storm_human_lane".to_string(),
            autonomous_executor: "universal_execution_primitive".to_string(),
            storm_executor: "storm_human_lane".to_string(),
            tasks: vec![
                json!({
                    "micro_task_id": "mt_a",
                    "profile_id": "p_a",
                    "route": { "lane": "autonomous_micro_agent" },
                    "governance": { "blocked": false }
                }),
                json!({
                    "micro_task_id": "mt_b",
                    "profile_id": "p_b",
                    "route": { "lane": "storm_human_lane" },
                    "governance": { "blocked": true }
                }),
            ],
        };
        let rows = build_dispatch_rows(&req);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0]["executor"], "universal_execution_primitive");
        assert_eq!(rows[0]["status"], "queued");
        assert_eq!(rows[1]["executor"], "storm_human_lane");
        assert_eq!(rows[1]["status"], "blocked");
    }
}
