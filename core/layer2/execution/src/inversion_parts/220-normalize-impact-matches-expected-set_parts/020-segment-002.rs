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

