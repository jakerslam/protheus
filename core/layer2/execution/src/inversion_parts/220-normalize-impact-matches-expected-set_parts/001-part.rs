            target: Some("identity".to_string()),
            fallback: Some(90),
        });
        assert_eq!(window.days, 30);

        let parsed = compute_parse_candidate_list_from_llm_payload(
            &ParseCandidateListFromLlmPayloadInput {
                payload: Some(json!({
                    "candidates": [
                        { "id": "c1", "filters": ["risk_guard_compaction", "fallback_pathing"], "probability": 0.8, "rationale": "ok" },
                        { "id": "c2", "filters": [], "probability": 0.4, "rationale": "skip" }
                    ]
                })),
            },
        );
        assert_eq!(parsed.candidates.len(), 1);
        assert_eq!(parsed.candidates[0].id, "c1");

        let heuristic = compute_heuristic_filter_candidates(&HeuristicFilterCandidatesInput {
            objective: Some("reduce budget drift".to_string()),
        });
        assert!(heuristic.candidates.len() >= 7);

        let score = compute_score_trial(&ScoreTrialInput {
            decision: Some(json!({
                "allowed": true,
                "attractor": { "score": 0.7 },
                "input": { "effective_certainty": 0.9 },
                "gating": { "required_certainty": 0.5 }
            })),
            candidate: Some(json!({ "score_hint": 0.8 })),
            trial_cfg: Some(json!({
                "score_weights": {
                    "decision_allowed": 0.35,
                    "attractor": 0.2,
                    "certainty_margin": 0.15,
                    "library_similarity": 0.1,
                    "runtime_probe": 0.2
                }
            })),
            runtime_probe_pass: Some(true),
        });
        assert!(score.score > 0.8);

        let mutated = compute_mutate_trial_candidates(&MutateTrialCandidatesInput {
            rows: vec![
                json!({"id":"n1","filters":["constraint_reframe"],"source":"heuristic","probability":0.5,"score_hint":0.4}),
            ],
        });
        assert_eq!(mutated.rows.len(), 1);
        let row = mutated.rows[0].as_object().expect("mutated row object");
        assert!(row
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .contains("_m1"));
        assert!(row
            .get("source")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .contains("_mutated"));
    }

    #[test]
    fn tier_scope_helpers_match_contract() {
        let iso = compute_normalize_iso_events(&NormalizeIsoEventsInput {
            src: vec![
                json!("2026-03-04T00:00:00.000Z"),
                json!("bad"),
                json!("2026-03-03T00:00:00.000Z"),
            ],
            max_rows: Some(10000),
        });
        assert_eq!(iso.events.len(), 2);

        let legacy = compute_expand_legacy_count_to_events(&ExpandLegacyCountToEventsInput {
            count: Some(json!(3)),
            ts: Some("2026-03-04T00:00:00.000Z".to_string()),
        });
        assert_eq!(legacy.events.len(), 3);

        let map = compute_normalize_tier_event_map(&NormalizeTierEventMapInput {
            src: Some(json!({"tactical":["2026-03-01T00:00:00.000Z"]})),
            fallback: Some(default_tier_event_map_value()),
            legacy_counts: Some(json!({"belief": 2})),
            legacy_ts: Some("2026-03-04T00:00:00.000Z".to_string()),
        });
        assert!(map.map["tactical"].is_array());
        assert!(map.map["belief"].is_array());

        let scope = compute_default_tier_scope(&DefaultTierScopeInput {
            legacy: Some(json!({"live_apply_counts": {"tactical": 1}})),
            legacy_ts: Some("2026-03-04T00:00:00.000Z".to_string()),
        });
        assert!(scope.scope["live_apply_attempts"]["tactical"].is_array());

        let norm_scope = compute_normalize_tier_scope(&NormalizeTierScopeInput {
            scope: Some(json!({"shadow_passes": {"identity": ["2026-03-04T00:00:00.000Z"]}})),
            legacy: Some(json!({})),
            legacy_ts: Some("2026-03-04T00:00:00.000Z".to_string()),
        });
        assert!(norm_scope.scope["shadow_passes"]["identity"].is_array());

        let state = compute_default_tier_governance_state(&DefaultTierGovernanceStateInput {
            policy_version: Some("1.2".to_string()),
        });
        assert_eq!(
            state.state["schema_id"],
            json!("inversion_tier_governance_state")
        );

        let cloned = compute_clone_tier_scope(&CloneTierScopeInput {
            scope: Some(norm_scope.scope.clone()),
        });
        assert!(cloned.scope["shadow_passes"]["identity"].is_array());

        let pruned = compute_prune_tier_scope_events(&PruneTierScopeEventsInput {
            scope: Some(json!({
                "live_apply_attempts": {"tactical":["2000-01-01T00:00:00.000Z","2026-03-04T00:00:00.000Z"]},
                "live_apply_successes": default_tier_event_map_value(),
                "live_apply_safe_aborts": default_tier_event_map_value(),
                "shadow_passes": default_tier_event_map_value(),
                "shadow_critical_failures": default_tier_event_map_value()
            })),
            retention_days: Some(365),
        });
        assert!(pruned.scope["live_apply_attempts"]["tactical"].is_array());

        let count = compute_count_tier_events(&CountTierEventsInput {
            scope: Some(pruned.scope.clone()),
            metric: Some("live_apply_attempts".to_string()),
            target: Some("tactical".to_string()),
            window_days: Some(3650),
        });
        assert!(count.count >= 0);

        let effective =
            compute_effective_window_days_for_target(&EffectiveWindowDaysForTargetInput {
                window_map: Some(json!({"identity": 30})),
                minimum_window_map: Some(json!({"identity": 45})),
                target: Some("identity".to_string()),
                fallback: Some(90),
            });
        assert_eq!(effective.days, 45);
    }

    #[test]
    fn foundational_scalar_helpers_match_contract() {
        let date = compute_to_date(&ToDateInput {
            value: Some("2026-03-04".to_string()),
        });
        assert_eq!(date.value, "2026-03-04".to_string());

        let ts = compute_parse_ts_ms(&ParseTsMsInput {
            value: Some("2026-03-04T00:00:00.000Z".to_string()),
        });
        assert!(ts.ts_ms > 0);

        let plus = compute_add_minutes(&AddMinutesInput {
            iso_ts: Some("2026-03-04T00:00:00.000Z".to_string()),
            minutes: Some(15.0),
        });
        assert!(plus
            .iso_ts
            .as_deref()
            .unwrap_or("")
            .contains("2026-03-04T00:15:00"));

        let ci = compute_clamp_int(&ClampIntInput {
            value: Some(json!(12)),
            lo: Some(0),
            hi: Some(10),
            fallback: Some(3),
        });
        assert_eq!(ci.value, 10);

        let cn = compute_clamp_number(&ClampNumberInput {
            value: Some(json!(1.7)),
            lo: Some(0.0),
            hi: Some(1.0),
            fallback: Some(0.5),
        });
        assert!((cn.value - 1.0).abs() < 1e-9);

        let b = compute_to_bool(&ToBoolInput {
            value: Some(json!("yes")),
            fallback: Some(false),
        });
        assert!(b.value);

        let clean = compute_clean_text(&CleanTextInput {
            value: Some("  a   b  ".to_string()),
            max_len: Some(16),
        });
        assert_eq!(clean.value, "a b".to_string());

        let token = compute_normalize_token(&NormalizeTokenInput {
            value: Some("A B+C".to_string()),
            max_len: Some(80),
        });
        assert_eq!(token.value, "a_b_c".to_string());

        let word = compute_normalize_word_token(&NormalizeWordTokenInput {
            value: Some("A B+C".to_string()),
            max_len: Some(80),
        });
        assert_eq!(word.value, "a_b_c".to_string());

        let band = compute_band_to_index(&BandToIndexInput {
            band: Some("seasoned".to_string()),
        });
        assert_eq!(band.index, 3);
    }

    #[test]
    fn helper_primitives_batch6_match_contract() {
        let escaped = compute_escape_regex(&EscapeRegexInput {
            value: Some("a+b?c".to_string()),
        });
        assert_eq!(escaped.value, "a\\+b\\?c".to_string());

        let pattern = compute_pattern_to_word_regex(&PatternToWordRegexInput {
            pattern: Some("risk guard".to_string()),
            max_len: Some(200),
        });
        assert_eq!(pattern.source, Some("\\brisk\\s+guard\\b".to_string()));

        let stable = compute_stable_id(&StableIdInput {
            seed: Some("seed".to_string()),
            prefix: Some("inv".to_string()),
        });
        assert!(stable.id.starts_with("inv_"));

        let rel = compute_rel_path(&RelPathInput {
            root: Some("/tmp/root".to_string()),
            file_path: Some("/tmp/root/state/a.json".to_string()),
        });
        assert_eq!(rel.value, "state/a.json".to_string());

        let axiom = compute_normalize_axiom_pattern(&NormalizeAxiomPatternInput {
            value: Some("  Risk   Guard  ".to_string()),
        });
        assert_eq!(axiom.value, "risk guard".to_string());

        let terms = compute_normalize_axiom_signal_terms(&NormalizeAxiomSignalTermsInput {
            terms: vec![json!(" Risk "), json!("Guard"), json!("")],
        });
        assert_eq!(terms.terms, vec!["risk".to_string(), "guard".to_string()]);

        let observer = compute_normalize_observer_id(&NormalizeObserverIdInput {
            value: Some("Observer 01".to_string()),
        });
        assert_eq!(observer.value, "observer_01".to_string());

        let num = compute_extract_numeric(&ExtractNumericInput {
            value: json!("2.5"),
        });
        assert_eq!(num.value, Some(2.5));

        let first = compute_pick_first_numeric(&PickFirstNumericInput {
            candidates: vec![json!(""), json!("x"), json!(7.0)],
        });
        assert_eq!(first.value, Some(0.0));

        let safe = compute_safe_rel_path(&SafeRelPathInput {
            root: Some("/tmp/root".to_string()),
            file_path: Some("/tmp/other/a.json".to_string()),
        });
        assert_eq!(safe.value, "/tmp/other/a.json".to_string());
    }

    #[test]
    fn helper_primitives_batch7_match_contract() {
        let now = compute_now_iso(&NowIsoInput::default());
        assert!(now.value.contains('T'));

        let default_map = compute_default_tier_event_map(&DefaultTierEventMapInput::default());
        assert!(default_map.map["tactical"].is_array());

        let coerced = compute_coerce_tier_event_map(&CoerceTierEventMapInput {
            map: Some(json!({"tactical":[1, "x"], "belief":["y"]})),
        });
        assert_eq!(coerced.map["tactical"][0], json!("1"));

        let got = compute_get_tier_scope(&GetTierScopeInput {
            state: Some(json!({"scopes": {}})),
            policy_version: Some("2.0".to_string()),
        });
        assert!(got.state["scopes"]["2.0"].is_object());
        assert!(got.scope.is_object());
        for metric in [
            "live_apply_attempts",
            "live_apply_successes",
            "live_apply_safe_aborts",
            "shadow_passes",
            "shadow_critical_failures",
        ] {
            assert!(
                got.scope
                    .get(metric)
                    .and_then(|v| v.get("constitution"))
                    .and_then(Value::as_array)
                    .is_some(),
                "missing constitution bucket for metric {metric}"
            );
        }

        let harness = compute_default_harness_state(&DefaultHarnessStateInput::default());
        assert_eq!(
            harness.state["schema_id"],
            json!("inversion_maturity_harness_state")
        );

        let lock = compute_default_first_principle_lock_state(
            &DefaultFirstPrincipleLockStateInput::default(),
        );
        assert_eq!(
            lock.state["schema_id"],
            json!("inversion_first_principle_lock_state")
        );

        let maturity = compute_default_maturity_state(&DefaultMaturityStateInput::default());
        assert_eq!(maturity.state["band"], json!("novice"));

        let key = compute_principle_key_for_session(&PrincipleKeyForSessionInput {
            objective_id: Some("BL-209".to_string()),
            objective: Some("fallback".to_string()),
            target: Some("directive".to_string()),
        });
        assert!(key.key.starts_with("directive::"));
        assert_eq!(key.key.len(), "directive::".len() + 16);

        let objective = compute_normalize_objective_arg(&NormalizeObjectiveArgInput {
            value: Some("  ship   lane  ".to_string()),
        });
        assert_eq!(objective.value, "ship lane".to_string());

        let order = compute_maturity_band_order(&MaturityBandOrderInput::default());
        assert_eq!(
            order.bands,
            vec![
                "novice".to_string(),
                "developing".to_string(),
                "mature".to_string(),
                "seasoned".to_string(),
                "legendary".to_string()
            ]
        );

        let mode = compute_current_runtime_mode(&CurrentRuntimeModeInput {
            env_mode: Some("".to_string()),
            args_mode: Some("test".to_string()),
            policy_runtime_mode: Some("live".to_string()),
        });
        assert_eq!(mode.mode, "test".to_string());
    }

    #[test]
    fn helper_primitives_batch8_match_contract() {
        let drift = compute_read_drift_from_state_file(&ReadDriftFromStateFileInput {
            file_path: Some("/tmp/state.json".to_string()),
            source_path: Some("state.json".to_string()),
            payload: Some(json!({"drift_rate": 0.1234567})),
        });
        assert_eq!(drift.value, 0.123457);
        assert_eq!(drift.source, "state.json".to_string());

        let resolved = compute_resolve_lens_gate_drift(&ResolveLensGateDriftInput {
            arg_candidates: vec![json!(null), json!(""), json!("0.2")],
            probe_path: Some("/tmp/state.json".to_string()),
            probe_source: Some("state.json".to_string()),
            probe_payload: Some(json!({"drift_rate": 0.8})),
        });
        assert_eq!(resolved.value, 0.0);
        assert_eq!(resolved.source, "arg".to_string());

        let parity = compute_resolve_parity_confidence(&ResolveParityConfidenceInput {
            arg_candidates: vec![],
            path_hint: Some("/tmp/parity.json".to_string()),
            path_source: Some("parity.json".to_string()),
            payload: Some(json!({"pass_rate": 0.7777777})),
        });
        assert_eq!(parity.value, 0.777778);
        assert_eq!(parity.source, "parity.json".to_string());
    }

    #[test]
    fn helper_primitives_batch9_match_contract() {
        let disabled = compute_attractor_score(&ComputeAttractorScoreInput {
            attractor: Some(json!({"enabled": false})),
            objective: Some("ship safely".to_string()),
            signature: Some("gate first".to_string()),
            ..Default::default()
        });
        assert!(!disabled.enabled);
        assert_eq!(disabled.score, 1.0);
        assert_eq!(disabled.required, 0.0);
        assert!(disabled.pass);
        assert_eq!(disabled.components, json!({}));

        let enabled = compute_attractor_score(&ComputeAttractorScoreInput {
            attractor: Some(json!({
                "enabled": true,
                "weights": {
                    "objective_specificity": 0.3,
                    "evidence_backing": 0.2,
                    "constraint_evidence": 0.15,
                    "measurable_outcome": 0.1,
                    "external_grounding": 0.05,
                    "certainty": 0.1,
                    "trit_alignment": 0.05,
                    "impact_alignment": 0.05,
                    "verbosity_penalty": 0.15
                },
                "verbosity": {
                    "soft_word_cap": 18,
                    "hard_word_cap": 80,
                    "low_diversity_floor": 0.22
                },
                "min_alignment_by_target": {
                    "directive": 0.2
                }
            })),
            objective: Some(
                "Must reduce drift below 2% within 7 days with measurable latency impact."
                    .to_string(),
            ),
            signature: Some(
                "Use github telemetry and external api evidence to improve throughput by 20%."
                    .to_string(),
            ),
            external_signals_count: Some(json!(3)),
            evidence_count: Some(json!(4)),
            effective_certainty: Some(json!(0.9)),
            trit: Some(json!(1)),
            impact: Some("high".to_string()),
            target: Some("directive".to_string()),
        });
        assert!(enabled.enabled);
        assert!(enabled.score >= 0.0 && enabled.score <= 1.0);
        assert!(enabled.required >= 0.0 && enabled.required <= 1.0);
        assert!(enabled.components.as_object().is_some());
        let word_count = enabled
            .components
            .as_object()
            .and_then(|m| m.get("word_count"))
            .and_then(|v| v.as_i64())
            .unwrap_or(-1);
        assert!(word_count >= 0);
    }
