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

    #[test]
    fn criteria_pattern_keys_normalizes_and_sorts_unique() {
        let out = compute_criteria_pattern_keys(&CriteriaPatternKeysInput {
            capability_key_hint: Some("".to_string()),
            capability_descriptor_key: Some("actuation:run".to_string()),
            rows: vec![
                CriteriaPatternKeysRowInput {
                    metric: Some("latency_ms".to_string()),
                },
                CriteriaPatternKeysRowInput {
                    metric: Some("Latency Ms".to_string()),
                },
                CriteriaPatternKeysRowInput { metric: None },
            ],
        });
        assert_eq!(out.keys, vec!["actuation:run|latency_ms".to_string()]);
    }

    #[test]
    fn autoscale_json_criteria_pattern_keys_path_works() {
        let payload = serde_json::json!({
            "mode": "criteria_pattern_keys",
            "criteria_pattern_keys_input": {
                "capability_key_hint": "actuation:run",
                "rows": [{"metric":"latency_ms"}]
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale criteria_pattern_keys");
        assert!(out.contains("\"mode\":\"criteria_pattern_keys\""));
    }

    #[test]
    fn success_criteria_requirement_merges_exempt_types() {
        let out = compute_success_criteria_requirement(&SuccessCriteriaRequirementInput {
            require_success_criteria: Some(true),
            min_success_criteria_count: Some(2.0),
            policy_exempt_types: vec!["directive_clarification".to_string()],
            env_exempt_types: vec![
                "directive_clarification".to_string(),
                "remediation".to_string(),
            ],
        });
        assert!(out.required);
        assert_eq!(out.min_count, 2.0);
        assert_eq!(
            out.exempt_types,
            vec![
                "directive_clarification".to_string(),
                "remediation".to_string()
            ]
        );
    }

    #[test]
    fn autoscale_json_success_criteria_requirement_path_works() {
        let payload = serde_json::json!({
            "mode": "success_criteria_requirement",
            "success_criteria_requirement_input": {
                "require_success_criteria": true,
                "min_success_criteria_count": 1,
                "policy_exempt_types": ["directive_clarification"],
                "env_exempt_types": ["remediation"]
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale success_criteria_requirement");
        assert!(out.contains("\"mode\":\"success_criteria_requirement\""));
    }

    #[test]
    fn success_criteria_policy_for_proposal_applies_exemptions() {
        let out =
            compute_success_criteria_policy_for_proposal(&SuccessCriteriaPolicyForProposalInput {
                base_required: true,
                base_min_count: 1.0,
                base_exempt_types: vec!["directive_clarification".to_string()],
                proposal_type: Some("directive_clarification".to_string()),
            });
        assert!(!out.required);
        assert!(out.exempt);
        assert_eq!(out.min_count, 1.0);
    }

    #[test]
    fn autoscale_json_success_criteria_policy_for_proposal_path_works() {
        let payload = serde_json::json!({
            "mode": "success_criteria_policy_for_proposal",
            "success_criteria_policy_for_proposal_input": {
                "base_required": true,
                "base_min_count": 1,
                "base_exempt_types": ["directive_clarification"],
                "proposal_type": "directive_clarification"
            }
        })
        .to_string();
        let out =
            run_autoscale_json(&payload).expect("autoscale success_criteria_policy_for_proposal");
        assert!(out.contains("\"mode\":\"success_criteria_policy_for_proposal\""));
    }

    #[test]
    fn capability_descriptor_prefers_actuation_kind() {
        let out = compute_capability_descriptor(&CapabilityDescriptorInput {
            actuation_kind: Some("route_execute".to_string()),
            proposal_type: Some("optimization".to_string()),
        });
        assert_eq!(out.key, "actuation:route_execute");
        assert_eq!(out.aliases, vec!["actuation".to_string()]);
    }

    #[test]
    fn autoscale_json_capability_descriptor_path_works() {
        let payload = serde_json::json!({
            "mode": "capability_descriptor",
            "capability_descriptor_input": {
                "actuation_kind": null,
                "proposal_type": "optimization"
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale capability_descriptor");
        assert!(out.contains("\"mode\":\"capability_descriptor\""));
    }

    #[test]
    fn normalize_token_usage_shape_uses_fallback_fields() {
        let out = compute_normalize_token_usage_shape(&NormalizeTokenUsageShapeInput {
            prompt_tokens: None,
            input_tokens: Some(11.0),
            completion_tokens: None,
            output_tokens: Some(5.0),
            total_tokens: None,
            tokens_used: None,
            source: Some("route_execute_metrics".to_string()),
        });
        assert!(out.has_value);
        let usage = out.usage.expect("usage");
        assert_eq!(usage.prompt_tokens, Some(11.0));
        assert_eq!(usage.completion_tokens, Some(5.0));
        assert_eq!(usage.total_tokens, Some(16.0));
    }

    #[test]
    fn autoscale_json_normalize_token_usage_shape_path_works() {
        let payload = serde_json::json!({
            "mode": "normalize_token_usage_shape",
            "normalize_token_usage_shape_input": {
                "prompt_tokens": 10,
                "completion_tokens": 4,
                "source": "route_execute_metrics"
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale normalize_token_usage_shape");
        assert!(out.contains("\"mode\":\"normalize_token_usage_shape\""));
    }

    #[test]
    fn default_backlog_autoscale_state_uses_input_module() {
        let out = compute_default_backlog_autoscale_state(&DefaultBacklogAutoscaleStateInput {
            module: "autonomy_spawn".to_string(),
        });
        assert_eq!(out.schema_id, "autonomy_backlog_autoscale");
        assert_eq!(out.schema_version, "1.0.0");
        assert_eq!(out.module, "autonomy_spawn");
        assert_eq!(out.current_cells, 0.0);
        assert_eq!(out.target_cells, 0.0);
        assert_eq!(out.last_run_ts, None);
    }

    #[test]
    fn autoscale_json_default_backlog_autoscale_state_path_works() {
        let payload = serde_json::json!({
            "mode": "default_backlog_autoscale_state",
            "default_backlog_autoscale_state_input": {
                "module": "autonomy_backlog_autoscale"
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale default_backlog_autoscale_state");
        assert!(out.contains("\"mode\":\"default_backlog_autoscale_state\""));
    }

    #[test]
    fn normalize_backlog_autoscale_state_normalizes_cells_and_strings() {
        let out = compute_normalize_backlog_autoscale_state(&NormalizeBacklogAutoscaleStateInput {
            module: "autonomy_backlog_autoscale".to_string(),
            src: Some(serde_json::json!({
