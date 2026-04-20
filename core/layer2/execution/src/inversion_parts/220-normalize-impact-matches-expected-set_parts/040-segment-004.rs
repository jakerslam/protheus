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

