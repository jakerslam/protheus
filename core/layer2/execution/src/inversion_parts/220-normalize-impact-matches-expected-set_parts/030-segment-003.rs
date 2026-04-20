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

