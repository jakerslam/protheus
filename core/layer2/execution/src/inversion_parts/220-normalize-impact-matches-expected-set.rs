// FILE_SIZE_EXCEPTION: reason=Atomic test-module block generated during safe decomposition; owner=jay; expires=2026-04-12
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_impact_matches_expected_set() {
        assert_eq!(
            compute_normalize_impact(&NormalizeImpactInput {
                value: Some("CRITICAL".to_string())
            }),
            NormalizeImpactOutput {
                value: "critical".to_string()
            }
        );
        assert_eq!(
            compute_normalize_impact(&NormalizeImpactInput {
                value: Some("unknown".to_string())
            }),
            NormalizeImpactOutput {
                value: "medium".to_string()
            }
        );
    }

    #[test]
    fn normalize_mode_defaults_live() {
        assert_eq!(
            compute_normalize_mode(&NormalizeModeInput {
                value: Some("test".to_string())
            }),
            NormalizeModeOutput {
                value: "test".to_string()
            }
        );
        assert_eq!(
            compute_normalize_mode(&NormalizeModeInput {
                value: Some("prod".to_string())
            }),
            NormalizeModeOutput {
                value: "live".to_string()
            }
        );
    }

    #[test]
    fn normalize_target_enforces_known_targets() {
        assert_eq!(
            compute_normalize_target(&NormalizeTargetInput {
                value: Some("directive".to_string())
            }),
            NormalizeTargetOutput {
                value: "directive".to_string()
            }
        );
        assert_eq!(
            compute_normalize_target(&NormalizeTargetInput {
                value: Some("unknown".to_string())
            }),
            NormalizeTargetOutput {
                value: "tactical".to_string()
            }
        );
    }

    #[test]
    fn normalize_result_enforces_expected_results() {
        assert_eq!(
            compute_normalize_result(&NormalizeResultInput {
                value: Some("SUCCESS".to_string())
            }),
            NormalizeResultOutput {
                value: "success".to_string()
            }
        );
        assert_eq!(
            compute_normalize_result(&NormalizeResultInput {
                value: Some("maybe".to_string())
            }),
            NormalizeResultOutput {
                value: String::new()
            }
        );
    }

    #[test]
    fn inversion_json_mode_routes() {
        let payload = json!({
            "mode": "normalize_target",
            "normalize_target_input": { "value": "belief" }
        });
        let out = run_inversion_json(&payload.to_string()).expect("inversion normalize_target");
        assert!(out.contains("\"mode\":\"normalize_target\""));
        assert!(out.contains("\"value\":\"belief\""));
    }

    #[test]
    fn inversion_json_alias_mode_routes_to_normalize_target() {
        let payload = json!({
            "mode": "target",
            "normalize_target_input": { "value": "directive" }
        });
        let out = run_inversion_json(&payload.to_string()).expect("inversion alias target");
        assert!(out.contains("\"mode\":\"normalize_target\""));
        assert!(out.contains("\"value\":\"directive\""));
    }

    #[test]
    fn inversion_json_unsupported_mode_reports_raw_and_normalized() {
        let payload = json!({ "mode": "Unknown Mode" });
        let err = run_inversion_json(&payload.to_string()).expect_err("unsupported mode");
        assert!(err.contains("inversion_mode_unsupported"));
        assert!(err.contains("raw=unknown mode"));
        assert!(err.contains("normalized=unknown_mode"));
    }

    #[test]
    fn inversion_json_blank_mode_fails_closed() {
        let payload = json!({ "mode": "   " });
        let err = run_inversion_json(&payload.to_string()).expect_err("blank mode");
        assert!(err.contains("inversion_mode_missing"));
    }

    #[test]
    fn inversion_json_get_tier_scope_routes_with_constitution_bucket() {
        let payload = json!({
            "mode": "get_tier_scope",
            "get_tier_scope_input": {
                "state": { "scopes": {} },
                "policy_version": "1.0"
            }
        });
        let out = run_inversion_json(&payload.to_string()).expect("inversion get_tier_scope");
        assert!(out.contains("\"mode\":\"get_tier_scope\""));
        assert!(out.contains("\"constitution\":[]"));
    }

    #[test]
    fn objective_id_validation_matches_expected_pattern() {
        let valid = compute_objective_id_valid(&ObjectiveIdValidInput {
            value: Some("T1_objective-alpha".to_string()),
        });
        assert!(valid.valid);
        let invalid = compute_objective_id_valid(&ObjectiveIdValidInput {
            value: Some("bad".to_string()),
        });
        assert!(!invalid.valid);
    }

    #[test]
    fn trit_vector_from_input_normalizes_numeric_tokens() {
        let out = compute_trit_vector_from_input(&TritVectorFromInputInput {
            trit_vector: Some(vec![json!(-2), json!(0), json!(3)]),
            trit_vector_csv: None,
        });
        assert_eq!(out.vector, vec![-1, 0, 1]);
    }

    #[test]
    fn jaccard_similarity_matches_overlap_ratio() {
        let out = compute_jaccard_similarity(&JaccardSimilarityInput {
            left_tokens: vec!["a".to_string(), "b".to_string()],
            right_tokens: vec!["b".to_string(), "c".to_string()],
        });
        assert!((out.similarity - (1.0 / 3.0)).abs() < 1e-9);
    }

    #[test]
    fn trit_similarity_matches_ts_contract() {
        let equal = compute_trit_similarity(&TritSimilarityInput {
            query_vector: vec![json!(1), json!(1), json!(0)],
            entry_trit: Some(json!(1)),
        });
        assert!((equal.similarity - 1.0).abs() < 1e-9);
        let neutral_mix = compute_trit_similarity(&TritSimilarityInput {
            query_vector: vec![json!(0), json!(0)],
            entry_trit: Some(json!(1)),
        });
        assert!((neutral_mix.similarity - 0.6).abs() < 1e-9);
    }

    #[test]
    fn certainty_threshold_reads_band_and_impact() {
        let out = compute_certainty_threshold(&CertaintyThresholdInput {
            thresholds: Some(json!({
                "novice": { "medium": 0.7 },
                "legendary": { "critical": 0.2 }
            })),
            band: Some("legendary".to_string()),
            impact: Some("critical".to_string()),
            allow_zero_for_legendary_critical: Some(true),
        });
        assert!((out.threshold - 0.0).abs() < 1e-9);
    }

    #[test]
    fn max_target_rank_respects_minimum_one() {
        let out = compute_max_target_rank(&MaxTargetRankInput {
            maturity_max_target_rank_by_band: Some(json!({ "mature": 4 })),
            impact_max_target_rank: Some(json!({ "high": 2 })),
            maturity_band: Some("mature".to_string()),
            impact: Some("high".to_string()),
        });
        assert_eq!(out.rank, 2);
    }

    #[test]
    fn extractors_and_permission_parsers_match_contract() {
        let bullets = compute_extract_bullets(&ExtractBulletsInput {
            markdown: Some("- a\n2. b\nnope".to_string()),
            max_items: Some(4),
        });
        assert_eq!(bullets.items, vec!["a".to_string(), "b".to_string()]);

        let list = compute_extract_list_items(&ExtractListItemsInput {
            markdown: Some("- one\n- two\n3. no".to_string()),
            max_items: Some(8),
        });
        assert_eq!(list.items, vec!["one".to_string(), "two".to_string()]);

        let parsed =
            compute_parse_system_internal_permission(&ParseSystemInternalPermissionInput {
                markdown: Some(
                    "- system_internal: {enabled: true, sources: [memory, loops]}".to_string(),
                ),
            });
        assert_eq!(
            parsed,
            ParseSystemInternalPermissionOutput {
                enabled: true,
                sources: vec!["memory".to_string(), "loops".to_string()]
            }
        );

        let rules = compute_parse_soul_token_data_pass_rules(&ParseSoulTokenDataPassRulesInput {
            markdown: Some(
                "## Data Pass Rules\n- allow-system-internal-passed-data\n- Non Runtime"
                    .to_string(),
            ),
        });
        assert_eq!(
            rules.rules,
            vec![
                "allow-system-internal-passed-data".to_string(),
                "non_runtime".to_string()
            ]
        );
    }

    #[test]
    fn system_passed_helpers_are_deterministic() {
        let ensured = compute_ensure_system_passed_section(&EnsureSystemPassedSectionInput {
            feed_text: Some("# Feed".to_string()),
        });
        assert!(ensured.text.contains("## System Passed"));

        let hash = compute_system_passed_payload_hash(&SystemPassedPayloadHashInput {
            source: Some("loop.inversion".to_string()),
            tags: vec!["loops".to_string(), "drift_alert".to_string()],
            payload: Some("drift=0.05".to_string()),
        });
        assert_eq!(hash.hash.len(), 64);
    }

    #[test]
    fn conclave_summary_and_flags_match_expectations() {
        let summary = compute_build_conclave_proposal_summary(&BuildConclaveProposalSummaryInput {
            objective: Some("Improve memory safety".to_string()),
            objective_id: Some("T1_abc".to_string()),
            target: Some("identity".to_string()),
            impact: Some("high".to_string()),
            mode: Some("live".to_string()),
        });
        assert!(summary.summary.contains("Improve memory safety"));

        let position = compute_build_lens_position(&BuildLensPositionInput {
            objective: Some("memory and security flow".to_string()),
            target: Some("tactical".to_string()),
            impact: Some("medium".to_string()),
        });
        assert!(position.position.contains("security fail-closed"));

        let flags = compute_conclave_high_risk_flags(&ConclaveHighRiskFlagsInput {
            payload: Some(json!({
                "ok": true,
                "winner": "vikram",
                "max_divergence": 0.9,
                "persona_outputs": [{ "confidence": 0.4, "recommendation": "disable covenant" }]
            })),
            query: Some("test".to_string()),
            summary: Some("skip parity".to_string()),
            max_divergence: Some(0.45),
            min_confidence: Some(0.6),
            high_risk_keywords: vec!["disable covenant".to_string(), "skip parity".to_string()],
        });
        assert!(flags.flags.contains(&"high_divergence".to_string()));
        assert!(flags.flags.contains(&"low_confidence".to_string()));
        assert!(flags
            .flags
            .contains(&"keyword:disable_covenant".to_string()));
        assert!(flags.flags.contains(&"keyword:skip_parity".to_string()));
    }

    #[test]
    fn creative_penalty_enforces_bounds() {
        let out = compute_creative_penalty(&CreativePenaltyInput {
            enabled: Some(true),
            preferred_creative_lane_ids: vec!["creative_lane".to_string()],
            non_creative_certainty_penalty: Some(0.7),
            selected_lane: Some("other_lane".to_string()),
        });
        assert!(!out.creative_lane_preferred);
        assert!(out.applied);
        assert!((out.penalty - 0.5).abs() < 1e-9);
    }

    #[test]
    fn parser_and_tokenizer_helpers_match_contract() {
        let tokens = compute_tokenize_text(&TokenizeTextInput {
            value: Some("Alpha alpha beta, gamma!".to_string()),
            max_tokens: Some(64),
        });
        assert_eq!(
            tokens.tokens,
            vec!["alpha".to_string(), "beta".to_string(), "gamma".to_string()]
        );

        let norm_list = compute_normalize_list(&NormalizeListInput {
            value: Some(json!(["A B", "a-b", "c"])),
            max_len: Some(80),
        });
        assert_eq!(
            norm_list.items,
            vec!["a_b".to_string(), "a-b".to_string(), "c".to_string()]
        );

        let text_list = compute_normalize_text_list(&NormalizeTextListInput {
            value: Some(json!(" one , two , one ")),
            max_len: Some(180),
            max_items: Some(64),
        });
        assert_eq!(text_list.items, vec!["one".to_string(), "two".to_string()]);

        let parsed = compute_parse_json_from_stdout(&ParseJsonFromStdoutInput {
            raw: Some("noise\n{\"ok\":true}".to_string()),
        });
        assert_eq!(parsed.parsed, Some(json!({"ok": true})));

        let args = compute_parse_args(&ParseArgsInput {
            argv: vec![
                "--mode=test".to_string(),
                "--target".to_string(),
                "belief".to_string(),
                "run".to_string(),
            ],
        });
        assert_eq!(args.args["mode"], json!("test"));
        assert_eq!(args.args["target"], json!("belief"));
        assert_eq!(args.args["_"], json!(["run"]));
    }

    #[test]
    fn scoring_and_signal_helpers_match_contract() {
        let score = compute_library_match_score(&LibraryMatchScoreInput {
            query_signature_tokens: vec!["alpha".to_string(), "beta".to_string()],
            query_trit_vector: vec![json!(1), json!(1)],
            query_target: Some("identity".to_string()),
            row_signature_tokens: vec!["beta".to_string(), "gamma".to_string()],
            row_outcome_trit: Some(1),
            row_target: Some("identity".to_string()),
            token_weight: Some(0.5),
            trit_weight: Some(0.3),
            target_weight: Some(0.2),
        });
        assert!((score.score - 0.666667).abs() < 1e-6);

        let pressure = compute_known_failure_pressure(&KnownFailurePressureInput {
            failed_repetition_similarity_block: Some(0.72),
            candidates: vec![
                json!({"row":{"outcome_trit":-1},"similarity":0.9}),
                json!({"row":{"outcome_trit":0},"similarity":0.8}),
            ],
        });
        assert_eq!(pressure.fail_count, 1);
        assert!(pressure.hard_block);

        let has_term = compute_has_signal_term_match(&HasSignalTermMatchInput {
            haystack: Some("optimize memory safety gate".to_string()),
            token_set: vec![
                "optimize".to_string(),
                "memory".to_string(),
                "safety".to_string(),
            ],
            term: Some("memory safety".to_string()),
        });
        assert!(has_term.matched);

        let groups = compute_count_axiom_signal_groups(&CountAxiomSignalGroupsInput {
            action_terms: vec!["optimize".to_string()],
            subject_terms: vec!["memory safety".to_string()],
            object_terms: vec!["gate".to_string()],
            min_signal_groups: Some(2),
            haystack: Some("optimize memory safety gate".to_string()),
            token_set: vec![
                "optimize".to_string(),
                "memory".to_string(),
                "safety".to_string(),
                "gate".to_string(),
            ],
        });
        assert_eq!(groups.configured_groups, 3);
        assert_eq!(groups.matched_groups, 3);
        assert!(groups.pass);

        let veto = compute_effective_first_n_human_veto_uses(&EffectiveFirstNHumanVetoUsesInput {
            first_live_uses_require_human_veto: Some(json!({"identity": 2})),
            minimum_first_live_uses_require_human_veto: Some(json!({"identity": 5})),
            target: Some("identity".to_string()),
        });
        assert_eq!(veto.uses, 5);
    }

    #[test]
    fn tree_and_trial_helpers_match_contract() {
        let band = compute_normalize_band_map(&NormalizeBandMapInput {
            raw: Some(json!({"novice": 0.7, "mature": -1})),
            base: Some(
                json!({"novice": 0.4, "developing": 0.5, "mature": 0.6, "seasoned": 0.7, "legendary": 0.8}),
            ),
            lo: Some(0.0),
            hi: Some(1.0),
        });
        assert!((band.novice - 0.7).abs() < 1e-9);
        assert!((band.mature - 0.0).abs() < 1e-9);

        let impact = compute_normalize_impact_map(&NormalizeImpactMapInput {
            raw: Some(json!({"critical": 1.5})),
            base: Some(json!({"low": 0.2, "medium": 0.4, "high": 0.6, "critical": 0.8})),
            lo: Some(0.0),
            hi: Some(1.0),
        });
        assert!((impact.critical - 1.0).abs() < 1e-9);

        let target_map = compute_normalize_target_map(&NormalizeTargetMapInput {
            raw: Some(json!({"identity": 0.9})),
            base: Some(
                json!({"tactical": 0.1, "belief": 0.2, "identity": 0.3, "directive": 0.4, "constitution": 0.5}),
            ),
            lo: Some(0.0),
            hi: Some(1.0),
        });
        assert!((target_map.identity - 0.9).abs() < 1e-9);

        let target_policy = compute_normalize_target_policy(&NormalizeTargetPolicyInput {
            raw: Some(
                json!({"rank": 12, "live_enabled": "yes", "test_enabled": "off", "require_human_veto_live": "1", "min_shadow_hours": 3}),
            ),
            base: Some(
                json!({"rank": 2, "live_enabled": false, "test_enabled": true, "require_human_veto_live": false, "min_shadow_hours": 1}),
            ),
        });
        assert_eq!(target_policy.rank, 10);
        assert!(target_policy.live_enabled);
        assert!(!target_policy.test_enabled);
        assert!(target_policy.require_human_veto_live);
        assert_eq!(target_policy.min_shadow_hours, 3);

        let retention = compute_tier_retention_days(&TierRetentionDaysInput {
            policy: Some(json!({
                "tier_transition": { "window_days_by_target": { "tactical": 60 }, "minimum_window_days_by_target": { "identity": 120 } },
                "shadow_pass_gate": { "window_days_by_target": { "belief": 45 } }
            })),
        });
        assert_eq!(retention.days, 365);

        let window = compute_window_days_for_target(&WindowDaysForTargetInput {
            window_map: Some(json!({"identity": 30})),
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

    #[test]
    fn helper_primitives_batch10_match_contract() {
        let out = compute_build_output_interfaces(&BuildOutputInterfacesInput {
            outputs: Some(json!({
                "default_channel": "strategy_hint",
                "belief_update": { "enabled": true, "test_enabled": true, "live_enabled": false, "require_sandbox_verification": false, "require_explicit_emit": false },
                "strategy_hint": { "enabled": true, "test_enabled": true, "live_enabled": true, "require_sandbox_verification": false, "require_explicit_emit": false },
                "workflow_hint": { "enabled": false, "test_enabled": true, "live_enabled": true, "require_sandbox_verification": false, "require_explicit_emit": false },
                "code_change_proposal": { "enabled": true, "test_enabled": true, "live_enabled": true, "require_sandbox_verification": true, "require_explicit_emit": true }
            })),
            mode: Some("test".to_string()),
            sandbox_verified: Some(json!(false)),
            explicit_code_proposal_emit: Some(json!(false)),
            channel_payloads: Some(json!({
                "strategy_hint": { "hint": "x" }
            })),
            base_payload: Some(json!({ "base": true })),
        });

        assert_eq!(out.default_channel, "strategy_hint".to_string());
        assert_eq!(out.active_channel, Some("strategy_hint".to_string()));
        let channels = out.channels.as_object().expect("channels object");
        assert_eq!(channels.len(), 4);
        let proposal = channels
            .get("code_change_proposal")
            .and_then(|v| v.as_object())
            .expect("proposal object");
        assert!(!proposal
            .get("enabled")
            .and_then(|v| v.as_bool())
            .unwrap_or(true));
        assert!(proposal
            .get("gated_reasons")
            .and_then(|v| v.as_array())
            .map(|rows| rows
                .iter()
                .any(|row| row.as_str() == Some("sandbox_verification_required")))
            .unwrap_or(false));
    }

    #[test]
    fn helper_primitives_batch11_match_contract() {
        let out = compute_build_code_change_proposal_draft(&BuildCodeChangeProposalDraftInput {
            base: Some(json!({
                "objective": "Harden inversion lane",
                "objective_id": "BL-214",
                "ts": "2026-03-04T00:00:00.000Z",
                "mode": "test",
                "impact": "high",
                "target": "directive",
                "certainty": 0.7333333,
                "maturity_band": "developing",
                "reasons": ["one", "two"],
                "shadow_mode": true
            })),
            args: Some(json!({
                "code_change_title": "Migrate proposal draft builder",
                "code_change_summary": "Rust-first proposal generation with parity fallback.",
                "code_change_files": ["client/runtime/systems/autonomy/inversion_controller.ts"],
                "code_change_tests": ["tests/client-memory-tools/inversion_helper_batch11_rust_parity.test.ts"],
                "code_change_risk": "low"
            })),
            opts: Some(json!({
                "session_id": "ivs_123",
                "sandbox_verified": true
            })),
        });
        let proposal = out.proposal.as_object().expect("proposal object");
        assert_eq!(
            proposal.get("type").and_then(|v| v.as_str()).unwrap_or(""),
            "code_change_proposal"
        );
        assert!(proposal
            .get("proposal_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .starts_with("icp_"));
        assert!(proposal
            .get("sandbox_verified")
            .and_then(|v| v.as_bool())
            .unwrap_or(false));
    }

    #[test]
    fn helper_primitives_batch12_match_contract() {
        let out = compute_normalize_library_row(&NormalizeLibraryRowInput {
            row: Some(json!({
                "id": " abc ",
                "ts": "2026-03-04T00:00:00.000Z",
                "objective": " Ship lane ",
                "objective_id": " BL-215 ",
                "signature": "  reduce drift safely ",
                "target": "directive",
                "impact": "high",
                "certainty": 1.2,
                "filter_stack": ["risk_guard", "  "],
                "outcome_trit": 2,
                "result": "OK",
                "maturity_band": "Developing",
                "principle_id": " p1 ",
                "session_id": " s1 "
            })),
        });
        let row = out.row.as_object().expect("row object");
        assert_eq!(row.get("id").and_then(|v| v.as_str()).unwrap_or(""), "abc");
        assert_eq!(
            row.get("target").and_then(|v| v.as_str()).unwrap_or(""),
            "directive"
        );
        assert_eq!(
            row.get("impact").and_then(|v| v.as_str()).unwrap_or(""),
            "high"
        );
        assert_eq!(
            row.get("outcome_trit")
                .and_then(|v| v.as_i64())
                .unwrap_or(0),
            1
        );
    }

    #[test]
    fn helper_primitives_batch13_match_contract() {
        let temp_root = std::env::temp_dir().join("inv_batch13");
        let _ = fs::remove_dir_all(&temp_root);
        let _ = fs::create_dir_all(&temp_root);
        let state_dir = temp_root.join("state");
        let _ = compute_ensure_dir(&EnsureDirInput {
            dir_path: Some(state_dir.to_string_lossy().to_string()),
        });
        assert!(state_dir.exists());

        let json_path = state_dir.join("x.json");
        let _ = compute_write_json_atomic(&WriteJsonAtomicInput {
            file_path: Some(json_path.to_string_lossy().to_string()),
            value: Some(json!({"a": 1})),
        });
        let read_json = compute_read_json(&ReadJsonInput {
            file_path: Some(json_path.to_string_lossy().to_string()),
            fallback: Some(json!({})),
        });
        assert_eq!(read_json.value, json!({"a": 1}));

        let jsonl_path = state_dir.join("x.jsonl");
        let _ = compute_append_jsonl(&AppendJsonlInput {
            file_path: Some(jsonl_path.to_string_lossy().to_string()),
            row: Some(json!({"k": "v"})),
        });
        let read_jsonl = compute_read_jsonl(&ReadJsonlInput {
            file_path: Some(jsonl_path.to_string_lossy().to_string()),
        });
        assert_eq!(read_jsonl.rows.len(), 1);

        let read_text = compute_read_text(&ReadTextInput {
            file_path: Some(json_path.to_string_lossy().to_string()),
            fallback: Some("fallback".to_string()),
        });
        assert!(read_text.text.contains("\"a\""));

        let latest = compute_latest_json_file_in_dir(&LatestJsonFileInDirInput {
            dir_path: Some(state_dir.to_string_lossy().to_string()),
        });
        assert!(latest.file_path.is_some());

        let out_channel = compute_normalize_output_channel(&NormalizeOutputChannelInput {
            base_out: Some(json!({"enabled": false, "test_enabled": true})),
            src_out: Some(json!({"enabled": true})),
        });
        assert!(out_channel.enabled);
        assert!(out_channel.test_enabled);

        let normalized_repo = compute_normalize_repo_path(&NormalizeRepoPathInput {
            value: Some("client/runtime/config/x.json".to_string()),
            fallback: Some("/tmp/fallback.json".to_string()),
            root: Some("/tmp/root".to_string()),
        });
        assert!(normalized_repo.path.contains("/tmp/root"));

        let paths = compute_runtime_paths(&RuntimePathsInput {
            policy_path: Some("/tmp/policy.json".to_string()),
            inversion_state_dir_env: Some("/tmp/state-root".to_string()),
            dual_brain_policy_path_env: Some("/tmp/dual.json".to_string()),
            default_state_dir: Some("/tmp/default-state".to_string()),
            root: Some("/tmp/root".to_string()),
        });
        assert_eq!(
            paths
                .paths
                .as_object()
                .and_then(|m| m.get("state_dir"))
                .and_then(|v| v.as_str())
                .unwrap_or(""),
            "/tmp/state-root"
        );
    }

    #[test]
    fn helper_primitives_batch14_match_contract() {
        let out_axioms = compute_normalize_axiom_list(&NormalizeAxiomListInput {
            raw_axioms: Some(json!([
                {
                    "id": " A1 ",
                    "patterns": [" Do no harm ", ""],
                    "regex": ["^never\\s+harm"],
                    "intent_tags": [" safety ", "guard"],
                    "signals": {
                        "action_terms": ["harm"],
                        "subject_terms": ["operator"],
                        "object_terms": ["user"]
                    },
                    "min_signal_groups": 2,
                    "semantic_requirements": {
                        "actions": ["protect"],
                        "subjects": ["human"],
                        "objects": ["safety"]
                    }
                }
            ])),
            base_axioms: Some(json!([])),
        });
        assert_eq!(out_axioms.axioms.len(), 1);
        let axiom = out_axioms.axioms[0].as_object().expect("axiom object");
        assert_eq!(axiom.get("id").and_then(|v| v.as_str()).unwrap_or(""), "a1");

        let out_suite = compute_normalize_harness_suite(&NormalizeHarnessSuiteInput {
            raw_suite: Some(json!([
                {
                    "id": " HX-1 ",
                    "objective": " validate lane ",
                    "impact": "critical",
                    "target": "directive",
                    "difficulty": "hard"
                }
            ])),
            base_suite: Some(json!([])),
        });
        assert_eq!(out_suite.suite.len(), 1);
        let row = out_suite.suite[0].as_object().expect("suite row");
        assert_eq!(row.get("id").and_then(|v| v.as_str()).unwrap_or(""), "hx-1");
        assert_eq!(
            row.get("target").and_then(|v| v.as_str()).unwrap_or(""),
            "directive"
        );

        let temp_root = std::env::temp_dir().join("inv_batch14");
        let _ = fs::remove_dir_all(&temp_root);
        let _ = fs::create_dir_all(&temp_root);
        let harness_path = temp_root.join("harness.json");
        let first_principles_path = temp_root.join("lock_state.json");
        let approvals_path = temp_root.join("observer_approvals.jsonl");
        let correspondence_path = temp_root.join("correspondence.md");

        let saved_harness = compute_save_harness_state(&SaveHarnessStateInput {
            file_path: Some(harness_path.to_string_lossy().to_string()),
            state: Some(json!({"last_run_ts":"2026-03-04T00:00:00.000Z","cursor":7})),
            now_iso: Some("2026-03-04T12:00:00.000Z".to_string()),
        });
        assert_eq!(
            saved_harness
                .state
                .as_object()
                .and_then(|m| m.get("cursor"))
                .and_then(|v| v.as_i64())
                .unwrap_or(-1),
            7
        );
        let loaded_harness = compute_load_harness_state(&LoadHarnessStateInput {
            file_path: Some(harness_path.to_string_lossy().to_string()),
            now_iso: Some("2026-03-04T12:00:00.000Z".to_string()),
        });
        assert_eq!(
            loaded_harness
                .state
                .as_object()
                .and_then(|m| m.get("cursor"))
                .and_then(|v| v.as_i64())
                .unwrap_or(-1),
            7
        );

        let saved_lock =
            compute_save_first_principle_lock_state(&SaveFirstPrincipleLockStateInput {
                file_path: Some(first_principles_path.to_string_lossy().to_string()),
                state: Some(json!({"locks":{"k":{"confidence":0.9}}})),
                now_iso: Some("2026-03-04T12:01:00.000Z".to_string()),
            });
        assert!(saved_lock
            .state
            .as_object()
            .and_then(|m| m.get("locks"))
            .and_then(|v| v.as_object())
            .is_some());
        let loaded_lock =
            compute_load_first_principle_lock_state(&LoadFirstPrincipleLockStateInput {
                file_path: Some(first_principles_path.to_string_lossy().to_string()),
                now_iso: Some("2026-03-04T12:02:00.000Z".to_string()),
            });
        assert!(loaded_lock
            .state
            .as_object()
            .and_then(|m| m.get("locks"))
            .and_then(|v| v.as_object())
            .is_some());

        let _ = compute_append_observer_approval(&AppendObserverApprovalInput {
            file_path: Some(approvals_path.to_string_lossy().to_string()),
            target: Some("belief".to_string()),
            observer_id: Some("observer_a".to_string()),
            note: Some("first".to_string()),
            now_iso: Some("2026-03-04T12:00:00.000Z".to_string()),
        });
        let _ = compute_append_observer_approval(&AppendObserverApprovalInput {
            file_path: Some(approvals_path.to_string_lossy().to_string()),
            target: Some("belief".to_string()),
            observer_id: Some("observer_a".to_string()),
            note: Some("duplicate".to_string()),
            now_iso: Some("2026-03-04T12:05:00.000Z".to_string()),
        });
        let loaded_observers = compute_load_observer_approvals(&LoadObserverApprovalsInput {
            file_path: Some(approvals_path.to_string_lossy().to_string()),
        });
        assert_eq!(loaded_observers.rows.len(), 2);
        let observer_count = compute_count_observer_approvals(&CountObserverApprovalsInput {
            file_path: Some(approvals_path.to_string_lossy().to_string()),
            target: Some("belief".to_string()),
            window_days: Some(json!(365)),
        });
        assert_eq!(observer_count.count, 1);

        let ensured = compute_ensure_correspondence_file(&EnsureCorrespondenceFileInput {
            file_path: Some(correspondence_path.to_string_lossy().to_string()),
            header: Some("# Shadow Conclave Correspondence\n\n".to_string()),
        });
        assert!(ensured.ok);
        assert!(correspondence_path.exists());
    }

    #[test]
    fn helper_primitives_batch15_match_contract() {
        let temp_root = std::env::temp_dir().join("inv_batch15");
        let _ = fs::remove_dir_all(&temp_root);
        let _ = fs::create_dir_all(&temp_root);
        let maturity_path = temp_root.join("maturity.json");
        let sessions_path = temp_root.join("active_sessions.json");
        let events_dir = temp_root.join("events");
        let receipts_path = temp_root.join("lens_gate_receipts.jsonl");
        let correspondence_path = temp_root.join("correspondence.md");
        let latest_path = temp_root.join("latest.json");
        let history_path = temp_root.join("history.jsonl");
        let interfaces_latest_path = temp_root.join("interfaces_latest.json");
        let interfaces_history_path = temp_root.join("interfaces_history.jsonl");
        let library_path = temp_root.join("library.jsonl");

        let policy = json!({
            "maturity": {
                "target_test_count": 40,
                "score_weights": {
                    "pass_rate": 0.5,
                    "non_destructive_rate": 0.3,
                    "experience": 0.2
                },
                "bands": {
                    "novice": 0.25,
                    "developing": 0.45,
                    "mature": 0.65,
                    "seasoned": 0.82
                }
            }
        });

        let saved_maturity = compute_save_maturity_state(&SaveMaturityStateInput {
            file_path: Some(maturity_path.to_string_lossy().to_string()),
            policy: Some(policy.clone()),
            state: Some(json!({
                "stats": {
                    "total_tests": 20,
                    "passed_tests": 15,
                    "destructive_failures": 1
                }
            })),
            now_iso: Some("2026-03-04T12:00:00.000Z".to_string()),
        });
        assert!(saved_maturity.computed.get("score").is_some());
        let loaded_maturity = compute_load_maturity_state(&LoadMaturityStateInput {
            file_path: Some(maturity_path.to_string_lossy().to_string()),
            policy: Some(policy.clone()),
            now_iso: Some("2026-03-04T12:01:00.000Z".to_string()),
        });
        assert!(loaded_maturity.state.get("band").is_some());

        let saved_sessions = compute_save_active_sessions(&SaveActiveSessionsInput {
            file_path: Some(sessions_path.to_string_lossy().to_string()),
            store: Some(json!({"sessions":[{"session_id":"s1"},{"session_id":"s2"}]})),
            now_iso: Some("2026-03-04T12:02:00.000Z".to_string()),
        });
        assert_eq!(
            saved_sessions
                .store
                .as_object()
                .and_then(|m| m.get("sessions"))
                .and_then(|v| v.as_array())
                .map(|rows| rows.len())
                .unwrap_or(0),
            2
        );
        let loaded_sessions = compute_load_active_sessions(&LoadActiveSessionsInput {
            file_path: Some(sessions_path.to_string_lossy().to_string()),
            now_iso: Some("2026-03-04T12:03:00.000Z".to_string()),
        });
        assert_eq!(
            loaded_sessions
                .store
                .as_object()
                .and_then(|m| m.get("sessions"))
                .and_then(|v| v.as_array())
                .map(|rows| rows.len())
                .unwrap_or(0),
            2
        );

        let emitted = compute_emit_event(&EmitEventInput {
            events_dir: Some(events_dir.to_string_lossy().to_string()),
            date_str: Some("2026-03-04".to_string()),
            event_type: Some("lane_selection".to_string()),
            payload: Some(json!({"ok": true})),
            emit_events: Some(true),
            now_iso: Some("2026-03-04T12:04:00.000Z".to_string()),
        });
        assert!(emitted.emitted);

        let receipt =
            compute_append_persona_lens_gate_receipt(&AppendPersonaLensGateReceiptInput {
                state_dir: Some(temp_root.to_string_lossy().to_string()),
                root: Some(temp_root.to_string_lossy().to_string()),
                cfg_receipts_path: Some(receipts_path.to_string_lossy().to_string()),
                payload: Some(json!({
                    "enabled": true,
                    "persona_id": "vikram",
                    "mode": "auto",
                    "effective_mode": "enforce",
                    "status": "enforced",
                    "fail_closed": false,
                    "drift_rate": 0.01,
                    "drift_threshold": 0.02,
                    "parity_confidence": 0.9,
                    "parity_confident": true,
                    "reasons": ["ok"]
                })),
                decision: Some(json!({
                    "allowed": true,
                    "input": {"objective":"x","target":"belief","impact":"high"}
                })),
                now_iso: Some("2026-03-04T12:05:00.000Z".to_string()),
            });
        assert!(receipt.rel_path.is_some());
        let receipt_again =
            compute_append_persona_lens_gate_receipt(&AppendPersonaLensGateReceiptInput {
                state_dir: Some(temp_root.to_string_lossy().to_string()),
                root: Some(temp_root.to_string_lossy().to_string()),
                cfg_receipts_path: Some(receipts_path.to_string_lossy().to_string()),
                payload: Some(json!({
                    "enabled": true,
                    "persona_id": "vikram",
                    "mode": "auto",
                    "effective_mode": "enforce",
                    "status": "enforced",
                    "fail_closed": false,
                    "drift_rate": 0.01,
                    "drift_threshold": 0.02,
                    "parity_confidence": 0.9,
                    "parity_confident": true,
                    "reasons": ["ok"]
                })),
                decision: Some(json!({
                    "allowed": true,
                    "input": {"objective":"x","target":"belief","impact":"high"}
                })),
                now_iso: Some("2026-03-04T12:05:00.000Z".to_string()),
            });
        assert_eq!(receipt.rel_path, receipt_again.rel_path);
        let receipts_raw = fs::read_to_string(&receipts_path).expect("read persona lens receipts");
        let rows = receipts_raw
            .lines()
            .filter(|line| !line.trim().is_empty())
            .collect::<Vec<_>>();
        assert!(rows.len() >= 2);
        assert_eq!(rows[rows.len() - 1], rows[rows.len() - 2]);
        let parsed: Value =
            serde_json::from_str(rows[rows.len() - 1]).expect("parse persona lens receipt");
        assert_eq!(parsed.get("target").and_then(Value::as_str), Some("belief"));
        assert_eq!(
            parsed.get("type").and_then(Value::as_str),
            Some("inversion_persona_lens_gate")
        );

        let conclave = compute_append_conclave_correspondence(&AppendConclaveCorrespondenceInput {
            correspondence_path: Some(correspondence_path.to_string_lossy().to_string()),
            row: Some(json!({
                "ts": "2026-03-04T12:06:00.000Z",
                "session_or_step": "step-1",
                "pass": true,
                "winner": "vikram",
                "arbitration_rule": "safety_first",
                "high_risk_flags": ["none"],
                "query": "q",
                "proposal_summary": "s",
                "receipt_path": "r",
                "review_payload": {"ok": true}
            })),
        });
        assert!(conclave.ok);
        assert!(correspondence_path.exists());

        let persisted = compute_persist_decision(&PersistDecisionInput {
            latest_path: Some(latest_path.to_string_lossy().to_string()),
            history_path: Some(history_path.to_string_lossy().to_string()),
            payload: Some(json!({"decision":"x"})),
        });
        assert!(persisted.ok);

        let persisted_env = compute_persist_interface_envelope(&PersistInterfaceEnvelopeInput {
            latest_path: Some(interfaces_latest_path.to_string_lossy().to_string()),
            history_path: Some(interfaces_history_path.to_string_lossy().to_string()),
            envelope: Some(json!({"envelope":"x"})),
        });
        assert!(persisted_env.ok);

        let _ = compute_append_jsonl(&AppendJsonlInput {
            file_path: Some(library_path.to_string_lossy().to_string()),
            row: Some(json!({"id":"a","ts":"2026-03-04T00:00:00.000Z","objective":"one"})),
        });
        let _ = compute_append_jsonl(&AppendJsonlInput {
            file_path: Some(library_path.to_string_lossy().to_string()),
            row: Some(json!({"id":"b","ts":"2026-03-04T00:01:00.000Z","objective":"two"})),
        });
        let _ = compute_append_jsonl(&AppendJsonlInput {
            file_path: Some(library_path.to_string_lossy().to_string()),
            row: Some(json!({"id":"c","ts":"2026-03-04T00:02:00.000Z","objective":"three"})),
        });
        let trimmed = compute_trim_library(&TrimLibraryInput {
            file_path: Some(library_path.to_string_lossy().to_string()),
            max_entries: Some(json!(2)),
        });
        assert_eq!(trimmed.rows.len(), 3);
    }

    #[test]
    fn helper_primitives_batch16_match_contract() {
        let temp_root = std::env::temp_dir().join("inv_batch16");
        let _ = fs::remove_dir_all(&temp_root);
        let _ = fs::create_dir_all(temp_root.join("first_principles"));
        let tier_path = temp_root.join("tier_governance.json");
        let lock_path = temp_root.join("first_principles").join("lock_state.json");

        let base_state = json!({
            "schema_id": "inversion_tier_governance_state",
            "schema_version": "1.0",
            "active_policy_version": "1.7",
            "scopes": {
                "1.7": {
                    "live_apply_attempts": {"tactical": ["2026-03-04T00:00:00.000Z"]},
                    "live_apply_successes": {"tactical": []},
                    "live_apply_safe_aborts": {"tactical": []},
                    "shadow_passes": {"tactical": []},
                    "shadow_critical_failures": {"tactical": []}
                }
            }
        });
        let policy = json!({
            "version": "1.7",
            "tier_transition": {
                "window_days_by_target": {"tactical": 45, "directive": 90},
                "minimum_window_days_by_target": {"tactical": 30, "directive": 60}
            },
            "shadow_pass_gate": {
                "window_days_by_target": {"tactical": 60, "directive": 120}
            },
            "first_principles": {
                "anti_downgrade": {
                    "enabled": true,
                    "require_same_or_higher_maturity": true,
                    "prevent_lower_confidence_same_band": true,
                    "same_band_confidence_floor_ratio": 0.92
                }
            }
        });

        let saved = compute_save_tier_governance_state(&SaveTierGovernanceStateInput {
            file_path: Some(tier_path.to_string_lossy().to_string()),
            state: Some(base_state),
            policy_version: Some("1.7".to_string()),
            retention_days: Some(3650),
            now_iso: Some("2026-03-04T12:00:00.000Z".to_string()),
        });
        assert_eq!(
            value_path(Some(&saved.state), &["active_policy_version"])
                .and_then(|v| v.as_str())
                .unwrap_or(""),
            "1.7"
        );
        let loaded = compute_load_tier_governance_state(&LoadTierGovernanceStateInput {
            file_path: Some(tier_path.to_string_lossy().to_string()),
            policy_version: Some("1.7".to_string()),
            now_iso: Some("2026-03-04T12:01:00.000Z".to_string()),
        });
        assert!(value_path(Some(&loaded.state), &["active_scope"]).is_some());

        let pushed = compute_push_tier_event(&PushTierEventInput {
            scope_map: Some(json!({"tactical": []})),
            target: Some("directive".to_string()),
            ts: Some("2026-03-04T12:00:00.000Z".to_string()),
        });
        assert_eq!(
            pushed
                .map
                .as_object()
                .and_then(|m| m.get("directive"))
                .and_then(|v| v.as_array())
                .map(|rows| rows.len())
                .unwrap_or(0),
            1
        );

        let added = compute_add_tier_event(&AddTierEventInput {
            file_path: Some(tier_path.to_string_lossy().to_string()),
            policy: Some(policy.clone()),
            metric: Some("live_apply_attempts".to_string()),
            target: Some("belief".to_string()),
            ts: Some("2026-03-04T12:00:00.000Z".to_string()),
            now_iso: Some("2026-03-04T12:00:00.000Z".to_string()),
        });
        assert!(value_path(
            Some(&added.state),
            &["scopes", "1.7", "live_apply_attempts", "belief"]
        )
        .and_then(|v| v.as_array())
        .map(|rows| !rows.is_empty())
        .unwrap_or(false));

        let inc_attempt = compute_increment_live_apply_attempt(&IncrementLiveApplyAttemptInput {
            file_path: Some(tier_path.to_string_lossy().to_string()),
            policy: Some(policy.clone()),
            target: Some("identity".to_string()),
            now_iso: Some("2026-03-04T12:02:00.000Z".to_string()),
        });
        assert!(value_path(
            Some(&inc_attempt.state),
            &["scopes", "1.7", "live_apply_attempts", "identity"]
        )
        .and_then(|v| v.as_array())
        .map(|rows| !rows.is_empty())
        .unwrap_or(false));

        let inc_success = compute_increment_live_apply_success(&IncrementLiveApplySuccessInput {
            file_path: Some(tier_path.to_string_lossy().to_string()),
            policy: Some(policy.clone()),
            target: Some("identity".to_string()),
            now_iso: Some("2026-03-04T12:03:00.000Z".to_string()),
        });
        assert!(value_path(
            Some(&inc_success.state),
            &["scopes", "1.7", "live_apply_successes", "identity"]
        )
        .and_then(|v| v.as_array())
        .map(|rows| !rows.is_empty())
        .unwrap_or(false));

        let inc_abort =
            compute_increment_live_apply_safe_abort(&IncrementLiveApplySafeAbortInput {
                file_path: Some(tier_path.to_string_lossy().to_string()),
                policy: Some(policy.clone()),
                target: Some("identity".to_string()),
                now_iso: Some("2026-03-04T12:04:00.000Z".to_string()),
            });
        assert!(value_path(
            Some(&inc_abort.state),
            &["scopes", "1.7", "live_apply_safe_aborts", "identity"]
        )
        .and_then(|v| v.as_array())
        .map(|rows| !rows.is_empty())
        .unwrap_or(false));

        let shadow = compute_update_shadow_trial_counters(&UpdateShadowTrialCountersInput {
            file_path: Some(tier_path.to_string_lossy().to_string()),
            policy: Some(policy.clone()),
            session: Some(json!({"mode":"test","apply_requested": false,"target":"directive"})),
            result: Some("success".to_string()),
            destructive: Some(false),
            now_iso: Some("2026-03-04T12:05:00.000Z".to_string()),
        });
        assert!(shadow.state.is_some());

        let upsert = compute_upsert_first_principle_lock(&UpsertFirstPrincipleLockInput {
            file_path: Some(lock_path.to_string_lossy().to_string()),
            session: Some(json!({
                "objective_id":"BL-246",
                "objective":"Guard principle quality",
                "target":"directive",
                "maturity_band":"mature"
            })),
            principle: Some(json!({"id":"fp_guard","confidence":0.91})),
            now_iso: Some("2026-03-04T12:06:00.000Z".to_string()),
        });
        assert!(value_path(Some(&upsert.state), &["locks", upsert.key.as_str()]).is_some());

        let check = compute_check_first_principle_downgrade(&CheckFirstPrincipleDowngradeInput {
            file_path: Some(lock_path.to_string_lossy().to_string()),
            policy: Some(policy),
            session: Some(json!({
                "objective_id":"BL-246",
                "objective":"Guard principle quality",
                "target":"directive",
                "maturity_band":"developing"
            })),
            confidence: Some(0.5),
            now_iso: Some("2026-03-04T12:07:00.000Z".to_string()),
        });
        assert!(!check.allowed);
        assert_eq!(
            check.reason.as_deref().unwrap_or(""),
            "first_principle_downgrade_blocked_lower_maturity"
        );
    }

    #[test]
    fn helper_primitives_batch17_match_contract() {
        let temp_root = std::env::temp_dir().join("inv_batch17");
        let _ = fs::remove_dir_all(&temp_root);
        let _ = fs::create_dir_all(temp_root.join("events"));
        let _ = fs::create_dir_all(temp_root.join("simulation"));
        let _ = fs::create_dir_all(temp_root.join("red_team"));

        let library_path = temp_root.join("library.jsonl");
        let receipts_path = temp_root.join("receipts.jsonl");
        let active_sessions_path = temp_root.join("active_sessions.json");
        let fp_latest_path = temp_root.join("first_principles_latest.json");
        let fp_history_path = temp_root.join("first_principles_history.jsonl");
        let fp_lock_path = temp_root.join("first_principles_lock.json");

        let _ = compute_append_jsonl(&AppendJsonlInput {
            file_path: Some(library_path.to_string_lossy().to_string()),
            row: Some(json!({
                "id":"a1",
                "ts":"2026-03-04T00:00:00.000Z",
                "objective":"Reduce drift safely",
                "objective_id":"BL-263",
                "signature":"drift guard stable",
                "signature_tokens":["drift","guard","stable"],
                "target":"directive",
                "impact":"high",
                "certainty":0.9,
                "filter_stack":["drift_guard"],
                "outcome_trit":-1,
                "result":"fail",
                "maturity_band":"developing"
            })),
        });
        let _ = compute_append_jsonl(&AppendJsonlInput {
            file_path: Some(library_path.to_string_lossy().to_string()),
            row: Some(json!({
                "id":"a2",
                "ts":"2026-03-04T00:10:00.000Z",
                "objective":"Reduce drift safely",
                "objective_id":"BL-263",
                "signature":"drift guard stable",
                "signature_tokens":["drift","guard","stable"],
                "target":"directive",
                "impact":"high",
                "certainty":0.88,
                "filter_stack":["drift_guard","identity_guard"],
                "outcome_trit":-1,
                "result":"fail",
                "maturity_band":"developing"
            })),
        });
        let _ = compute_append_jsonl(&AppendJsonlInput {
            file_path: Some(library_path.to_string_lossy().to_string()),
            row: Some(json!({
                "id":"a3",
                "ts":"2026-03-04T00:20:00.000Z",
                "objective":"Reduce drift safely",
                "objective_id":"BL-263",
                "signature":"drift guard stable",
                "signature_tokens":["drift","guard","stable"],
                "target":"directive",
                "impact":"high",
                "certainty":0.86,
                "filter_stack":["drift_guard","fallback_pathing"],
                "outcome_trit":-1,
                "result":"fail",
                "maturity_band":"developing"
            })),
        });
        let _ = compute_append_jsonl(&AppendJsonlInput {
            file_path: Some(library_path.to_string_lossy().to_string()),
            row: Some(json!({
                "id":"a4",
                "ts":"2026-03-04T00:30:00.000Z",
                "objective":"Reduce drift safely",
                "objective_id":"BL-263",
                "signature":"drift guard stable",
                "signature_tokens":["drift","guard","stable"],
                "target":"directive",
                "impact":"high",
                "certainty":0.84,
                "filter_stack":["drift_guard","constraint_reframe"],
                "outcome_trit":-1,
                "result":"fail",
                "maturity_band":"developing"
            })),
        });
        let _ = compute_append_jsonl(&AppendJsonlInput {
            file_path: Some(library_path.to_string_lossy().to_string()),
            row: Some(json!({
                "id":"ok1",
                "ts":"2026-03-04T01:00:00.000Z",
                "objective":"Ship safely",
                "signature":"safe lane pass",
                "signature_tokens":["safe","lane","pass"],
                "target":"directive",
                "impact":"high",
                "certainty":0.92,
                "filter_stack":["safe_path"],
                "outcome_trit":1,
                "result":"success",
                "maturity_band":"mature"
            })),
        });

        let detect = compute_detect_immutable_axiom_violation(
            &DetectImmutableAxiomViolationInput {
                policy: Some(json!({
                    "immutable_axioms": {
                        "enabled": true,
                        "axioms": [{
                            "id":"safety_guard",
                            "patterns":["drift guard"],
                            "regex":["drift\\s+guard"],
                            "intent_tags":["safety"],
                            "signals":{"action_terms":["drift"],"subject_terms":["guard"],"object_terms":[]},
                            "min_signal_groups": 1
                        }]
                    }
                })),
                decision_input: Some(json!({
                    "objective":"Need drift guard policy",
                    "signature":"drift guard now",
                    "filters":["constraint_reframe"],
                    "intent_tags":["safety"]
                })),
            },
        );
        assert_eq!(detect.hits, vec!["safety_guard".to_string()]);

        let maturity = compute_maturity_score(&ComputeMaturityScoreInput {
            state: Some(json!({
                "stats": {
                    "total_tests": 20,
                    "passed_tests": 15,
                    "destructive_failures": 2
                }
            })),
            policy: Some(json!({
                "maturity": {
                    "target_test_count": 40,
                    "score_weights": {"pass_rate":0.5,"non_destructive_rate":0.3,"experience":0.2},
                    "bands": {"novice":0.25,"developing":0.45,"mature":0.65,"seasoned":0.82}
                }
            })),
        });
        assert_eq!(maturity.band, "seasoned".to_string());
        assert!((maturity.score - 0.745).abs() < 0.000001);

        let candidates = compute_select_library_candidates(&SelectLibraryCandidatesInput {
            policy: Some(json!({
                "library": {
                    "min_similarity_for_reuse": 0.2,
                    "token_weight": 0.6,
                    "trit_weight": 0.3,
                    "target_weight": 0.1
                }
            })),
            query: Some(json!({
                "signature_tokens":["drift","guard","stable"],
                "trit_vector":[-1],
                "target":"directive"
            })),
            file_path: Some(library_path.to_string_lossy().to_string()),
        });
        assert!(!candidates.candidates.is_empty());

        let lane = compute_parse_lane_decision(&ParseLaneDecisionInput {
            args: Some(json!({"brain_lane":"right"})),
        });
        assert_eq!(lane.selected_lane, "right".to_string());
        assert_eq!(lane.source, "arg".to_string());

        let now = now_iso_runtime();
        let expired_at = "2000-01-01T00:00:00.000Z".to_string();
        let live_at = "2999-01-01T00:00:00.000Z".to_string();
        let _ = compute_save_active_sessions(&SaveActiveSessionsInput {
            file_path: Some(active_sessions_path.to_string_lossy().to_string()),
            store: Some(json!({
                "sessions":[
                    {"session_id":"exp","objective":"old","signature":"old sig","target":"directive","impact":"high","certainty":0.5,"expires_at": expired_at},
                    {"session_id":"live","objective":"new","signature":"new sig","target":"directive","impact":"high","certainty":0.6,"expires_at": live_at}
                ]
            })),
            now_iso: Some(now.clone()),
        });
        let sweep = compute_sweep_expired_sessions(&SweepExpiredSessionsInput {
            paths: Some(json!({
                "active_sessions_path": active_sessions_path.to_string_lossy().to_string(),
                "receipts_path": receipts_path.to_string_lossy().to_string(),
                "library_path": library_path.to_string_lossy().to_string(),
                "events_dir": temp_root.join("events").to_string_lossy().to_string()
            })),
            policy: Some(json!({"telemetry":{"emit_events":false},"library":{"max_entries":200}})),
            date_str: Some("2026-03-04".to_string()),
            now_iso: Some(now.clone()),
        });
        assert_eq!(sweep.expired_count, 1);
        assert_eq!(sweep.sessions.len(), 1);

        let _ = fs::write(
            temp_root.join("regime.json"),
            serde_json::to_string(&json!({
                "selected_regime":"constrained",
                "candidate_confidence":0.8,
                "context":{"trit":{"trit":-1}}
            }))
            .unwrap_or_else(|_| "{}".to_string()),
        );
        let _ = fs::write(
            temp_root.join("mirror.json"),
            serde_json::to_string(
                &json!({"pressure_score":0.7,"confidence":0.75,"reasons":["pressure","drift"]}),
            )
            .unwrap_or_else(|_| "{}".to_string()),
        );
        let _ = fs::write(
            temp_root.join("drift_governor.json"),
            serde_json::to_string(&json!({"last_decision":{"trit_shadow":{"belief":{"trit":-1}}}}))
                .unwrap_or_else(|_| "{}".to_string()),
        );
        let _ = fs::write(
            temp_root.join("simulation").join("2026-03-04.json"),
            serde_json::to_string(&json!({"checks_effective":{"drift_rate":{"value":0.09},"yield_rate":{"value":0.4}}}))
                .unwrap_or_else(|_| "{}".to_string()),
        );
        let _ = fs::write(
            temp_root.join("red_team").join("latest.json"),
            serde_json::to_string(
                &json!({"summary":{"critical_fail_cases":2,"pass_cases":1,"fail_cases":3}}),
            )
            .unwrap_or_else(|_| "{}".to_string()),
        );

        let signals = compute_load_impossibility_signals(&LoadImpossibilitySignalsInput {
            policy: Some(json!({
                "organ": {
                    "trigger_detection": {
                        "paths": {
                            "regime_latest_path":"regime.json",
                            "mirror_latest_path":"mirror.json",
                            "simulation_dir":"simulation",
                            "red_team_runs_dir":"red_team",
                            "drift_governor_path":"drift_governor.json"
                        }
                    }
                }
            })),
            date_str: Some("2026-03-04".to_string()),
            root: Some(temp_root.to_string_lossy().to_string()),
        });
        assert_eq!(
            value_path(Some(&signals.signals), &["trit", "value"])
                .and_then(|v| v.as_i64())
                .unwrap_or(0),
            -1
        );

        let trigger = compute_evaluate_impossibility_trigger(&EvaluateImpossibilityTriggerInput {
            policy: Some(json!({
                "organ": {
                    "trigger_detection": {
                        "enabled": true,
                        "min_impossibility_score": 0.58,
                        "min_signal_count": 2,
                        "thresholds": {"predicted_drift_warn":0.03,"predicted_yield_warn":0.68},
                        "weights": {
                            "trit_pain":0.2,
                            "mirror_pressure":0.2,
                            "predicted_drift":0.18,
                            "predicted_yield_gap":0.18,
                            "red_team_critical":0.14,
                            "regime_constrained":0.1
                        }
                    }
                }
            })),
            signals: Some(signals.signals.clone()),
            force: Some(false),
        });
        assert!(trigger.triggered);
        assert!(trigger.signal_count >= 2);

        let fp_policy = json!({
            "first_principles": {
                "enabled": true,
                "auto_extract_on_success": true,
                "max_strategy_bonus": 0.12,
                "allow_failure_cluster_extraction": true,
                "failure_cluster_min": 4
            },
            "library": {
                "min_similarity_for_reuse": 0.2,
                "token_weight": 0.6,
                "trit_weight": 0.3,
                "target_weight": 0.1
            }
        });
        let session = json!({
            "session_id":"sfp",
            "objective":"Reduce drift safely",
            "objective_id":"BL-263",
            "target":"directive",
            "certainty":0.8,
            "filter_stack":["drift_guard"],
            "signature":"drift guard stable",
            "signature_tokens":["drift","guard","stable"]
        });
        let first_principle = compute_extract_first_principle(&ExtractFirstPrincipleInput {
            policy: Some(fp_policy.clone()),
            session: Some(session.clone()),
            args: Some(json!({})),
            result: Some("success".to_string()),
            now_iso: Some(now_iso_runtime()),
        });
        assert!(first_principle.principle.is_some());

        let failure_principle =
            compute_extract_failure_cluster_principle(&ExtractFailureClusterPrincipleInput {
                paths: Some(json!({"library_path": library_path.to_string_lossy().to_string()})),
                policy: Some(fp_policy),
                session: Some(session.clone()),
                now_iso: Some(now_iso_runtime()),
            });
        assert!(failure_principle.principle.is_some());

        let persisted = compute_persist_first_principle(&PersistFirstPrincipleInput {
            paths: Some(json!({
                "first_principles_latest_path": fp_latest_path.to_string_lossy().to_string(),
                "first_principles_history_path": fp_history_path.to_string_lossy().to_string(),
                "first_principles_lock_path": fp_lock_path.to_string_lossy().to_string()
            })),
            session: Some(session),
            principle: first_principle.principle.clone(),
            now_iso: Some(now_iso_runtime()),
        });
        assert!(persisted.principle.is_object());
        assert!(fp_latest_path.exists());
    }

    fn extract_mode_literals(text: &str, call_name: &str) -> std::collections::BTreeSet<String> {
        let pattern = format!(r#"{}\s*\(\s*['"`]([^'"`]+)['"`]"#, regex::escape(call_name));
        let re = Regex::new(&pattern).expect("valid call regex");
        let static_mode_re =
            Regex::new(r"^[a-zA-Z0-9_-]+$").expect("valid static mode token regex");
        let block_comment_re = Regex::new(r"(?s)/\*.*?\*/").expect("valid block comment regex");
        let line_comment_re = Regex::new(r"(?m)//.*$").expect("valid line comment regex");
        let without_block = block_comment_re.replace_all(text, "");
        let cleaned = line_comment_re.replace_all(&without_block, "");
        re.captures_iter(cleaned.as_ref())
            .filter_map(|cap| cap.get(1).map(|m| m.as_str().trim().to_string()))
            .filter(|mode| !mode.is_empty() && static_mode_re.is_match(mode))
            .collect()
    }

    fn extract_bridge_modes(text: &str, fn_name: &str) -> std::collections::BTreeSet<String> {
        let section_re = Regex::new(&format!(
            r#"(?s)function {}\s*\([^)]*\)\s*\{{.*?const fieldByMode:\s*AnyObj\s*=\s*\{{(.*?)\}}\s*(?:;|\r?\n)?"#,
            regex::escape(fn_name)
        ))
        .expect("valid section regex");
        let keys_re = Regex::new(r#"(?m)^\s*(?:([a-zA-Z0-9_]+)|['"]([^'"]+)['"])\s*:"#)
            .expect("valid key regex");
        let Some(section) = section_re
            .captures(text)
            .and_then(|cap| cap.get(1).map(|m| m.as_str()))
        else {
            return std::collections::BTreeSet::new();
        };
        keys_re
            .captures_iter(section)
            .filter_map(|cap| {
                cap.get(1)
                    .or_else(|| cap.get(2))
                    .map(|m| m.as_str().trim().to_string())
            })
            .filter(|key| !key.is_empty())
            .collect()
    }

    fn extract_dispatch_modes(text: &str) -> std::collections::BTreeSet<String> {
        let re = Regex::new(r#"(?m)^\s*(?:if|else if) mode == "([^"]+)""#)
            .expect("valid dispatch regex");
        let block_comment_re = Regex::new(r"(?s)/\*.*?\*/").expect("valid block comment regex");
        let line_comment_re = Regex::new(r"(?m)//.*$").expect("valid line comment regex");
        let without_block = block_comment_re.replace_all(text, "");
        let cleaned = line_comment_re.replace_all(&without_block, "");
        re.captures_iter(cleaned.as_ref())
            .filter_map(|cap| cap.get(1).map(|m| m.as_str().trim().to_string()))
            .filter(|mode| !mode.is_empty())
            .collect()
    }

    #[test]
    fn extract_mode_literals_accepts_all_quote_styles() {
        let text = r#"
const a = runInversionPrimitive("alpha", {});
const b = runInversionPrimitive('beta', {});
const c = runInversionPrimitive(`gamma`, {});
"#;
        let parsed = extract_mode_literals(text, "runInversionPrimitive");
        let expected = ["alpha", "beta", "gamma"]
            .iter()
            .map(|value| value.to_string())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(parsed, expected);
    }

    #[test]
    fn extract_bridge_modes_accepts_quoted_and_unquoted_keys() {
        let bridge = r#"
function runInversionPrimitive(mode: string, data: AnyObj = {}, opts: AnyObj = {}) {
  const fieldByMode: AnyObj = {
    alpha: "payload_alpha",
    "beta-mode": "payload_beta",
    'gamma_mode': "payload_gamma"
  };
}
"#;
        let parsed = extract_bridge_modes(bridge, "runInversionPrimitive");
        let expected = ["alpha", "beta-mode", "gamma_mode"]
            .iter()
            .map(|value| value.to_string())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(parsed, expected);
    }

    #[test]
    fn extract_bridge_modes_allows_non_string_values() {
        let bridge = r#"
function runInversionPrimitive(mode: string, data: AnyObj = {}, opts: AnyObj = {}) {
  const fieldByMode: AnyObj = {
    alpha: payloadAlpha,
    "beta-mode": payloadBeta,
    'gamma_mode': payloadGamma
  };
}
"#;
        let parsed = extract_bridge_modes(bridge, "runInversionPrimitive");
        let expected = ["alpha", "beta-mode", "gamma_mode"]
            .iter()
            .map(|value| value.to_string())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(parsed, expected);
    }

    #[test]
    fn extract_bridge_modes_selects_requested_function_section() {
        let bridge = r#"
function runInversionPrimitive(mode: string, data: AnyObj = {}, opts: AnyObj = {}) {
  const fieldByMode: AnyObj = {
    alpha: "payload_alpha"
  };
}
function runOtherPrimitive(mode: string, data: AnyObj = {}, opts: AnyObj = {}) {
  const fieldByMode: AnyObj = {
    rogue: "payload_rogue"
  };
}
"#;
        let parsed_inversion = extract_bridge_modes(bridge, "runInversionPrimitive");
        let expected_inversion = ["alpha"]
            .iter()
            .map(|value| value.to_string())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(parsed_inversion, expected_inversion);

        let parsed_other = extract_bridge_modes(bridge, "runOtherPrimitive");
        let expected_other = ["rogue"]
            .iter()
            .map(|value| value.to_string())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(parsed_other, expected_other);
    }

    #[test]
    fn extract_bridge_modes_allows_missing_trailing_semicolon() {
        let bridge = r#"
function runInversionPrimitive(mode: string, data: AnyObj = {}, opts: AnyObj = {}) {
  const fieldByMode: AnyObj = {
    alpha: "payload_alpha",
    beta: "payload_beta"
  }
}
"#;
        let parsed = extract_bridge_modes(bridge, "runInversionPrimitive");
        let expected = ["alpha", "beta"]
            .iter()
            .map(|value| value.to_string())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(parsed, expected);
    }

    #[test]
    fn extract_bridge_modes_returns_empty_when_function_missing() {
        let bridge = r#"
function runOtherPrimitive(mode: string, data: AnyObj = {}, opts: AnyObj = {}) {
  const fieldByMode: AnyObj = {
    rogue: "payload_rogue"
  };
}
"#;
        let parsed = extract_bridge_modes(bridge, "runInversionPrimitive");
        assert!(parsed.is_empty());
    }

    #[test]
    fn extract_bridge_modes_supports_crlf_lines() {
        let bridge = "function runInversionPrimitive(mode: string, data: AnyObj = {}, opts: AnyObj = {}) {\r\n  const fieldByMode: AnyObj = {\r\n    alpha: \"payload_alpha\",\r\n    beta: \"payload_beta\"\r\n  }\r\n}\r\n";
        let parsed = extract_bridge_modes(bridge, "runInversionPrimitive");
        let expected = ["alpha", "beta"]
            .iter()
            .map(|value| value.to_string())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(parsed, expected);
    }

    #[test]
    fn extract_mode_literals_ignores_dynamic_template_modes() {
        let text = r#"
const a = runInversionPrimitive("alpha", {});
const b = runInversionPrimitive(`beta_${suffix}`, {});
const c = runInversionPrimitive(modeName, {});
"#;
        let parsed = extract_mode_literals(text, "runInversionPrimitive");
        let expected = ["alpha"]
            .iter()
            .map(|value| value.to_string())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(parsed, expected);
    }

    #[test]
    fn extract_mode_literals_ignores_commented_calls() {
        let text = r#"
// runInversionPrimitive("ignored_line", {});
/* runInversionPrimitive("ignored_block", {}); */
const a = runInversionPrimitive(
  "alpha",
  {}
);
"#;
        let parsed = extract_mode_literals(text, "runInversionPrimitive");
        let expected = ["alpha"]
            .iter()
            .map(|value| value.to_string())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(parsed, expected);
    }

    #[test]
    fn extract_dispatch_modes_accepts_if_and_else_if() {
        let text = r#"
if mode == "alpha" {
}
else if mode == "beta" {
}
if another == "gamma" {
}
"#;
        let parsed = extract_dispatch_modes(text);
        let expected = ["alpha", "beta"]
            .iter()
            .map(|value| value.to_string())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(parsed, expected);
    }

    #[test]
    fn extract_dispatch_modes_ignores_commented_branches() {
        let text = r#"
// if mode == "ignored_line" {
// }
/* else if mode == "ignored_block" {
} */
if mode == "alpha" {
}
"#;
        let parsed = extract_dispatch_modes(text);
        let expected = ["alpha"]
            .iter()
            .map(|value| value.to_string())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(parsed, expected);
    }

    fn read_optional_autonomy_surface(rel: &str) -> String {
        std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(rel))
            .unwrap_or_default()
    }

    #[test]
    fn inversion_bridge_is_wrapper_only_in_coreized_layout() {
        let ts_autonomy = read_optional_autonomy_surface(
            "../../../client/runtime/systems/autonomy/autonomy_controller.ts",
        );
        let ts_inversion = read_optional_autonomy_surface(
            "../../../client/runtime/systems/autonomy/inversion_controller.ts",
        );
        let bridge = read_optional_autonomy_surface(
            "../../../client/runtime/systems/autonomy/backlog_autoscale_rust_bridge.ts",
        );
        let mut called = extract_mode_literals(&ts_inversion, "runInversionPrimitive");
        called.extend(extract_mode_literals(&ts_autonomy, "runInversionPrimitive"));
        if bridge.is_empty() {
            assert!(
                called.is_empty(),
                "coreized wrappers should not carry inversion mode calls"
            );
            return;
        }
        assert!(
            bridge.contains("createLegacyRetiredModule"),
            "backlog_autoscale_rust_bridge.js must remain a thin wrapper"
        );
        assert!(
            !bridge.contains("fieldByMode"),
            "wrapper-only bridge must not contain legacy inversion mode maps"
        );
        assert!(
            called.is_empty(),
            "coreized wrappers should not carry inversion mode calls"
        );
    }

    #[test]
    fn controller_callsite_modes_are_dispatched_by_rust_inversion_json() {
        let ts_autonomy = read_optional_autonomy_surface(
            "../../../client/runtime/systems/autonomy/autonomy_controller.ts",
        );
        let ts_inversion = read_optional_autonomy_surface(
            "../../../client/runtime/systems/autonomy/inversion_controller.ts",
        );
        let rust_src = include_str!("../inversion.rs");
        let mut called = extract_mode_literals(&ts_inversion, "runInversionPrimitive");
        called.extend(extract_mode_literals(&ts_autonomy, "runInversionPrimitive"));
        if !(ts_autonomy.is_empty() && ts_inversion.is_empty()) {
            assert!(
                ts_autonomy.contains("createOpsLaneBridge")
                    || ts_inversion.contains("createLegacyRetiredModule"),
                "expected thin-wrapper bridge markers in autonomy wrappers"
            );
        }
        let dispatched = extract_dispatch_modes(rust_src);
        let missing = called.difference(&dispatched).cloned().collect::<Vec<_>>();
        assert!(
            missing.is_empty(),
            "controller TS sources use inversion modes not dispatched by Rust inversion_json: {:?}",
            missing
        );
    }

    #[test]
    fn rust_dispatch_covers_all_inversion_bridge_modes() {
        let bridge = read_optional_autonomy_surface(
            "../../../client/runtime/systems/autonomy/backlog_autoscale_rust_bridge.ts",
        );
        let rust_src = include_str!("../inversion.rs");
        if bridge.is_empty() {
            return;
        }
        let mapped = extract_bridge_modes(&bridge, "runInversionPrimitive");
        if mapped.is_empty() {
            assert!(
                bridge.contains("createLegacyRetiredModule"),
                "wrapper-only bridge expected when inversion map literals are retired"
            );
            return;
        }
        let dispatched = extract_dispatch_modes(rust_src);
        let missing = mapped.difference(&dispatched).cloned().collect::<Vec<_>>();
        assert!(
            missing.is_empty(),
            "inversion bridge maps modes not dispatched by Rust inversion_json: {:?}",
            missing
        );
    }
}
