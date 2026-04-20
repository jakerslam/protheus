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

