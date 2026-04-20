    #[test]
    fn compile_directive_pulse_objectives_filters_and_derives_rows() {
        let out = compute_compile_directive_pulse_objectives(
            &CompileDirectivePulseObjectivesInput {
                directives: vec![
                    serde_json::json!({
                        "id": "T0_FOUNDATION",
                        "data": {
                            "metadata": { "id": "T0_FOUNDATION", "description": "ignore me", "tier": 1 }
                        }
                    }),
                    serde_json::json!({
                        "id": "T1_MEMORY",
                        "tier": 1,
                        "data": {
                            "metadata": {
                                "id": "T1_MEMORY",
                                "description": "Improve memory durability and recall quality",
                                "value_currency": "quality"
                            },
                            "intent": {
                                "primary": "Improve memory durability",
                                "value_currency": "time_savings"
                            },
                            "scope": {
                                "included": ["durability guardrails", "recall quality"]
                            },
                            "success_metrics": {
                                "leading": ["reduced regressions"],
                                "lagging": ["higher recall score"]
                            }
                        }
                    }),
                ],
                stopwords: vec!["the".to_string(), "and".to_string()],
                allowed_value_keys: vec![
                    "revenue".to_string(),
                    "delivery".to_string(),
                    "user_value".to_string(),
                    "quality".to_string(),
                    "time_savings".to_string(),
                    "learning".to_string(),
                ],
                t1_min_share: Some(0.5),
                t2_min_share: Some(0.25),
            },
        );
        assert_eq!(out.objectives.len(), 1);
        let row = &out.objectives[0];
        assert_eq!(row.id, "T1_MEMORY");
        assert_eq!(row.tier, 1);
        assert!(!row.phrases.is_empty());
        assert!(!row.tokens.is_empty());
        assert_eq!(row.primary_currency.as_deref(), Some("quality"));
    }

    #[test]
    fn autoscale_json_compile_directive_pulse_objectives_path_works() {
        let payload = serde_json::json!({
            "mode": "compile_directive_pulse_objectives",
            "compile_directive_pulse_objectives_input": {
                "directives": [
                    {
                        "id": "T1_MEMORY",
                        "tier": 1,
                        "data": {
                            "metadata": { "id": "T1_MEMORY", "description": "memory durability" },
                            "intent": { "primary": "Improve memory durability" }
                        }
                    }
                ],
                "stopwords": ["the", "and"],
                "allowed_value_keys": ["quality", "time_savings", "learning", "user_value", "delivery", "revenue"],
                "t1_min_share": 0.5,
                "t2_min_share": 0.25
            }
        })
        .to_string();
        let out =
            run_autoscale_json(&payload).expect("autoscale compile_directive_pulse_objectives");
        assert!(out.contains("\"mode\":\"compile_directive_pulse_objectives\""));
    }

    #[test]
    fn directive_pulse_objectives_profile_handles_disabled_and_error() {
        let disabled =
            compute_directive_pulse_objectives_profile(&DirectivePulseObjectivesProfileInput {
                enabled: false,
                load_error: None,
                objectives: vec![],
            });
        assert!(!disabled.enabled);
        assert!(!disabled.available);
        assert_eq!(disabled.error.as_deref(), Some("directive_pulse_disabled"));

        let errored =
            compute_directive_pulse_objectives_profile(&DirectivePulseObjectivesProfileInput {
                enabled: true,
                load_error: Some(" boom ".to_string()),
                objectives: vec![serde_json::json!({"id":"T1"})],
            });
        assert!(errored.enabled);
        assert!(!errored.available);
        assert_eq!(errored.objectives.len(), 0);
        assert_eq!(errored.error.as_deref(), Some("boom"));
    }

    #[test]
    fn autoscale_json_directive_pulse_objectives_profile_path_works() {
        let payload = serde_json::json!({
            "mode": "directive_pulse_objectives_profile",
            "directive_pulse_objectives_profile_input": {
                "enabled": true,
                "load_error": null,
                "objectives": [{"id":"T1_MEMORY"}]
            }
        })
        .to_string();
        let out =
            run_autoscale_json(&payload).expect("autoscale directive_pulse_objectives_profile");
        assert!(out.contains("\"mode\":\"directive_pulse_objectives_profile\""));
    }

    #[test]
    fn recent_directive_pulse_cooldown_count_matches_objective_and_window() {
        let now_ms = chrono::DateTime::parse_from_rfc3339("2026-03-04T12:00:00.000Z")
            .unwrap()
            .with_timezone(&Utc)
            .timestamp_millis() as f64;
        let out = compute_recent_directive_pulse_cooldown_count(
            &RecentDirectivePulseCooldownCountInput {
                objective_id: Some("OBJ-1".to_string()),
                hours: Some(24.0),
                now_ms: Some(now_ms),
                events: vec![
                    RecentDirectivePulseCooldownEventInput {
                        event_type: Some("autonomy_run".to_string()),
                        result: Some("stop_repeat_gate_directive_pulse_cooldown".to_string()),
                        ts: Some("2026-03-04T10:00:00.000Z".to_string()),
                        objective_id: Some("OBJ-1".to_string()),
                        sample_objective_id: None,
                    },
                    RecentDirectivePulseCooldownEventInput {
                        event_type: Some("autonomy_run".to_string()),
                        result: Some("stop_repeat_gate_directive_pulse_cooldown".to_string()),
                        ts: Some("2026-03-03T08:00:00.000Z".to_string()),
                        objective_id: Some("OBJ-1".to_string()),
                        sample_objective_id: None,
                    },
                    RecentDirectivePulseCooldownEventInput {
                        event_type: Some("autonomy_run".to_string()),
                        result: Some("stop_repeat_gate_directive_pulse_cooldown".to_string()),
                        ts: Some("2026-03-04T11:00:00.000Z".to_string()),
                        objective_id: Some("OBJ-2".to_string()),
                        sample_objective_id: None,
                    },
                ],
            },
        );
        assert_eq!(out.count, 1);
    }

    #[test]
    fn autoscale_json_recent_directive_pulse_cooldown_count_path_works() {
        let payload = serde_json::json!({
            "mode": "recent_directive_pulse_cooldown_count",
            "recent_directive_pulse_cooldown_count_input": {
                "objective_id": "OBJ-1",
                "hours": 24,
                "now_ms": 1772625600000.0,
                "events": [
                    {
                        "event_type": "autonomy_run",
                        "result": "stop_repeat_gate_directive_pulse_cooldown",
                        "ts": "2026-03-04T10:00:00.000Z",
                        "objective_id": "OBJ-1"
                    }
                ]
            }
        })
        .to_string();
        let out =
            run_autoscale_json(&payload).expect("autoscale recent_directive_pulse_cooldown_count");
        assert!(out.contains("\"mode\":\"recent_directive_pulse_cooldown_count\""));
    }

    #[test]
    fn proposal_directive_text_matches_expected_normalization() {
        let out = compute_proposal_directive_text(&ProposalDirectiveTextInput {
            proposal: Some(serde_json::json!({
                "title": "Directive fit improve",
                "type": "directive_clarification",
                "summary": "Improve objective focus",
                "meta": {
                    "normalized_hint_tokens": ["memory", "durability"],
                    "topics": ["alignment", "metrics"]
                },
                "validation": ["one metric"],
                "evidence": [{"match":"directive", "evidence_ref":"eye:directive/1"}]
            })),
        });
        assert!(out.text.contains("directive"));
        assert!(out.text.contains("memory"));
        assert!(out.text.contains("alignment"));
    }

    #[test]
    fn autoscale_json_proposal_directive_text_path_works() {
        let payload = serde_json::json!({
            "mode": "proposal_directive_text",
            "proposal_directive_text_input": {
                "proposal": {
                    "title": "Directive fit improve",
                    "type": "directive_clarification",
                    "summary": "Improve objective focus"
                }
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale proposal_directive_text");
        assert!(out.contains("\"mode\":\"proposal_directive_text\""));
    }

    #[test]
    fn objective_ids_from_pulse_context_prefers_objectives_then_fallback() {
        let out = compute_objective_ids_from_pulse_context(&ObjectiveIdsFromPulseContextInput {
            objectives: vec![
                serde_json::json!({"id":"OBJ-A"}),
                serde_json::json!({"id":"OBJ-B"}),
                serde_json::json!({"id":"OBJ-A"}),
            ],
            fallback_enabled: true,
            fallback_ids: vec!["OBJ-C".to_string()],
        });
        assert_eq!(out.ids, vec!["OBJ-A".to_string(), "OBJ-B".to_string()]);

        let fallback =
            compute_objective_ids_from_pulse_context(&ObjectiveIdsFromPulseContextInput {
                objectives: vec![],
                fallback_enabled: true,
                fallback_ids: vec![
                    "OBJ-C".to_string(),
                    "OBJ-C".to_string(),
                    "OBJ-D".to_string(),
                ],
            });
        assert_eq!(fallback.ids, vec!["OBJ-C".to_string(), "OBJ-D".to_string()]);
    }

    #[test]
    fn autoscale_json_objective_ids_from_pulse_context_path_works() {
        let payload = serde_json::json!({
            "mode": "objective_ids_from_pulse_context",
            "objective_ids_from_pulse_context_input": {
                "objectives": [{"id":"OBJ-A"}, {"id":"OBJ-B"}],
                "fallback_enabled": true,
                "fallback_ids": ["OBJ-C"]
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale objective_ids_from_pulse_context");
        assert!(out.contains("\"mode\":\"objective_ids_from_pulse_context\""));
    }

    #[test]
    fn policy_hold_objective_context_prefers_candidate_then_dominant() {
        let out = compute_policy_hold_objective_context(&PolicyHoldObjectiveContextInput {
            candidate_objective_ids: vec![
                "T1_OBJ_A".to_string(),
                " T1_OBJ_A ".to_string(),
                "T2_OBJ_B".to_string(),
            ],
            pool_objective_ids: vec!["T3_OBJ_C".to_string()],
            dominant_objective_id: Some("T4_OBJ_Z".to_string()),
        });
        assert_eq!(out.objective_id.as_deref(), Some("T4_OBJ_Z"));
        assert_eq!(
            out.objective_source.as_deref(),
            Some("directive_pulse_dominant")
        );
        assert_eq!(
            out.objective_ids.unwrap_or_default(),
            vec!["T1_OBJ_A".to_string(), "T2_OBJ_B".to_string()]
        );
    }

    #[test]
    fn autoscale_json_policy_hold_objective_context_path_works() {
        let payload = serde_json::json!({
            "mode": "policy_hold_objective_context",
            "policy_hold_objective_context_input": {
                "candidate_objective_ids": ["OBJ_A"],
                "pool_objective_ids": ["OBJ_B"],
                "dominant_objective_id": "OBJ_A"
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale policy_hold_objective_context");
        assert!(out.contains("\"mode\":\"policy_hold_objective_context\""));
    }

    #[test]
    fn proposal_semantic_objective_id_prefers_meta_then_command() {
        let out = compute_proposal_semantic_objective_id(&ProposalSemanticObjectiveIdInput {
            proposal: Some(serde_json::json!({
                "meta": {
                    "objective_id": "",
                    "directive_objective_id": "T1_PRIMARY",
                    "linked_objective_id": "T2_SECONDARY"
                },
                "suggested_next_command": "node x --id=T3_CMD"
            })),
        });
        assert_eq!(out.objective_id, "T1_PRIMARY");
    }

    #[test]
    fn autoscale_json_proposal_semantic_objective_id_path_works() {
        let payload = serde_json::json!({
            "mode": "proposal_semantic_objective_id",
            "proposal_semantic_objective_id_input": {
                "proposal": {
                    "meta": { "objective_id": "T1_OBJ" }
                }
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale proposal_semantic_objective_id");
        assert!(out.contains("\"mode\":\"proposal_semantic_objective_id\""));
    }

