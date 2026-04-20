    #[test]
    fn semantic_token_similarity_uses_jaccard_overlap() {
        let out = compute_semantic_token_similarity(&SemanticTokenSimilarityInput {
            left_tokens: vec![
                "bridge".to_string(),
                "rust".to_string(),
                "parity".to_string(),
                "rust".to_string(),
            ],
            right_tokens: vec![
                "rust".to_string(),
                "parity".to_string(),
                "tests".to_string(),
            ],
        });
        assert!(
            (out.similarity - 0.5).abs() < 1e-6,
            "similarity={}",
            out.similarity
        );

        let empty = compute_semantic_token_similarity(&SemanticTokenSimilarityInput {
            left_tokens: vec![],
            right_tokens: vec!["anything".to_string()],
        });
        assert_eq!(empty.similarity, 0.0);
    }

    #[test]
    fn autoscale_json_semantic_token_similarity_path_works() {
        let payload = serde_json::json!({
            "mode": "semantic_token_similarity",
            "semantic_token_similarity_input": {
                "left_tokens": ["rust", "bridge", "parity"],
                "right_tokens": ["parity", "tests"]
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale semantic_token_similarity");
        assert!(out.contains("\"mode\":\"semantic_token_similarity\""));
        assert!(out.contains("\"similarity\":0.25"));
    }

    #[test]
    fn semantic_context_comparable_requires_type_and_shared_context() {
        let pass = compute_semantic_context_comparable(&SemanticContextComparableInput {
            left_proposal_type: Some("ops_remediation".to_string()),
            right_proposal_type: Some("ops_remediation".to_string()),
            left_source_eye: Some("github_release".to_string()),
            right_source_eye: Some("github_release".to_string()),
            left_objective_id: None,
            right_objective_id: None,
            require_same_type: true,
            require_shared_context: true,
        });
        assert!(pass.comparable);

        let blocked = compute_semantic_context_comparable(&SemanticContextComparableInput {
            left_proposal_type: Some("ops_remediation".to_string()),
            right_proposal_type: Some("feature".to_string()),
            left_source_eye: Some("github_release".to_string()),
            right_source_eye: Some("github_release".to_string()),
            left_objective_id: None,
            right_objective_id: None,
            require_same_type: true,
            require_shared_context: true,
        });
        assert!(!blocked.comparable);
    }

    #[test]
    fn autoscale_json_semantic_context_comparable_path_works() {
        let payload = serde_json::json!({
            "mode": "semantic_context_comparable",
            "semantic_context_comparable_input": {
                "left_proposal_type": "ops_remediation",
                "right_proposal_type": "ops_remediation",
                "left_source_eye": "github_release",
                "right_source_eye": "github_release",
                "left_objective_id": "",
                "right_objective_id": "",
                "require_same_type": true,
                "require_shared_context": true
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale semantic_context_comparable");
        assert!(out.contains("\"mode\":\"semantic_context_comparable\""));
        assert!(out.contains("\"comparable\":true"));
    }

    #[test]
    fn semantic_near_duplicate_match_selects_best_eligible_candidate() {
        let out = compute_semantic_near_duplicate_match(&SemanticNearDuplicateMatchInput {
            fingerprint: SemanticNearDuplicateFingerprintInput {
                proposal_id: Some("new-1".to_string()),
                proposal_type: Some("ops_remediation".to_string()),
                source_eye: Some("github_release".to_string()),
                objective_id: Some("obj-a".to_string()),
                token_stems: vec![
                    "rust".to_string(),
                    "bridge".to_string(),
                    "parity".to_string(),
                ],
                eligible: true,
            },
            seen_fingerprints: vec![
                SemanticNearDuplicateFingerprintInput {
                    proposal_id: Some("old-1".to_string()),
                    proposal_type: Some("ops_remediation".to_string()),
                    source_eye: Some("github_release".to_string()),
                    objective_id: Some("obj-a".to_string()),
                    token_stems: vec![
                        "rust".to_string(),
                        "bridge".to_string(),
                        "tests".to_string(),
                    ],
                    eligible: true,
                },
                SemanticNearDuplicateFingerprintInput {
                    proposal_id: Some("old-2".to_string()),
                    proposal_type: Some("ops_remediation".to_string()),
                    source_eye: Some("github_release".to_string()),
                    objective_id: Some("obj-a".to_string()),
                    token_stems: vec![
                        "rust".to_string(),
                        "bridge".to_string(),
                        "parity".to_string(),
                    ],
                    eligible: true,
                },
            ],
            min_similarity: 0.5,
            require_same_type: true,
            require_shared_context: true,
        });
        assert!(out.matched);
        assert_eq!(out.proposal_id.as_deref(), Some("old-2"));
        assert!(
            (out.similarity - 1.0).abs() < 1e-6,
            "similarity={}",
            out.similarity
        );
    }

    #[test]
    fn autoscale_json_semantic_near_duplicate_match_path_works() {
        let payload = serde_json::json!({
            "mode": "semantic_near_duplicate_match",
            "semantic_near_duplicate_match_input": {
                "fingerprint": {
                    "proposal_id": "new-1",
                    "proposal_type": "ops_remediation",
                    "source_eye": "github_release",
                    "objective_id": "obj-a",
                    "token_stems": ["rust", "bridge", "parity"],
                    "eligible": true
                },
                "seen_fingerprints": [{
                    "proposal_id": "old-1",
                    "proposal_type": "ops_remediation",
                    "source_eye": "github_release",
                    "objective_id": "obj-a",
                    "token_stems": ["rust", "bridge", "tests"],
                    "eligible": true
                }],
                "min_similarity": 0.4,
                "require_same_type": true,
                "require_shared_context": true
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale semantic_near_duplicate_match");
        assert!(out.contains("\"mode\":\"semantic_near_duplicate_match\""));
        assert!(out.contains("\"matched\":true"));
    }

    #[test]
    fn strategy_rank_score_matches_weighted_formula() {
        let out = compute_strategy_rank_score(&StrategyRankScoreInput {
            composite_weight: 0.35,
            actionability_weight: 0.2,
            directive_fit_weight: 0.15,
            signal_quality_weight: 0.15,
            expected_value_weight: 0.1,
            value_density_weight: 0.08,
            risk_penalty_weight: 0.05,
            time_to_value_weight: 0.0,
            composite: 80.0,
            actionability: 70.0,
            directive_fit: 60.0,
            signal_quality: 75.0,
            expected_value: 55.0,
            value_density: 50.0,
            risk_penalty: 50.0,
            time_to_value: 40.0,
            non_yield_penalty: 1.5,
            collective_shadow_penalty: 0.5,
            collective_shadow_bonus: 0.2,
        });
        assert!((out.score - 67.45).abs() < 0.000001);
    }

    #[test]
    fn autoscale_json_strategy_rank_score_path_works() {
        let payload = serde_json::json!({
            "mode": "strategy_rank_score",
            "strategy_rank_score_input": {
                "composite_weight": 0.35,
                "actionability_weight": 0.2,
                "directive_fit_weight": 0.15,
                "signal_quality_weight": 0.15,
                "expected_value_weight": 0.1,
                "value_density_weight": 0.08,
                "risk_penalty_weight": 0.05,
                "time_to_value_weight": 0.0,
                "composite": 80,
                "actionability": 70,
                "directive_fit": 60,
                "signal_quality": 75,
                "expected_value": 55,
                "value_density": 50,
                "risk_penalty": 50,
                "time_to_value": 40,
                "non_yield_penalty": 1.5,
                "collective_shadow_penalty": 0.5,
                "collective_shadow_bonus": 0.2
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale strategy_rank_score");
        assert!(out.contains("\"mode\":\"strategy_rank_score\""));
        assert!(out.contains("\"score\":67.45"));
    }

    #[test]
    fn strategy_rank_adjusted_matches_pulse_and_objective_bonus_formula() {
        let out = compute_strategy_rank_adjusted(&StrategyRankAdjustedInput {
            base: 65.4,
            pulse_score: 82.0,
            pulse_weight: 0.25,
            objective_allocation_score: 70.0,
            base_objective_weight: 0.3,
            canary_mode: false,
        });
        assert!((out.adjusted - 93.25).abs() < 0.000001);
        assert!((out.bonus.total - 27.85).abs() < 0.000001);
        assert!((out.bonus.objective_weight - 0.105).abs() < 0.000001);
    }

    #[test]
    fn autoscale_json_strategy_rank_adjusted_path_works() {
        let payload = serde_json::json!({
            "mode": "strategy_rank_adjusted",
            "strategy_rank_adjusted_input": {
                "base": 65.4,
                "pulse_score": 82,
                "pulse_weight": 0.25,
                "objective_allocation_score": 70,
                "base_objective_weight": 0.3,
                "canary_mode": false
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale strategy_rank_adjusted");
        assert!(out.contains("\"mode\":\"strategy_rank_adjusted\""));
        assert!(out.contains("\"adjusted\":93.25"));
    }

    #[test]
    fn trit_shadow_rank_score_normalizes_belief_with_confidence_bonus() {
        let out = compute_trit_shadow_rank_score(&TritShadowRankScoreInput {
            score: 0.35,
            confidence: 0.6,
        });
        assert!((out.score - 73.5).abs() < 0.000001);
    }

    #[test]
    fn autoscale_json_trit_shadow_rank_score_path_works() {
        let payload = serde_json::json!({
            "mode": "trit_shadow_rank_score",
            "trit_shadow_rank_score_input": {
                "score": 0.35,
                "confidence": 0.6
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale trit_shadow_rank_score");
        assert!(out.contains("\"mode\":\"trit_shadow_rank_score\""));
        assert!(out.contains("\"score\":73.5"));
    }

    #[test]
    fn strategy_circuit_cooldown_matches_error_classification() {
        let out = compute_strategy_circuit_cooldown(&StrategyCircuitCooldownInput {
            last_error_code: Some("HTTP 503".to_string()),
            last_error: None,
            http_429_cooldown_hours: 1.0,
            http_5xx_cooldown_hours: 6.0,
            dns_error_cooldown_hours: 3.0,
        });
        assert!((out.cooldown_hours - 6.0).abs() < 0.000001);
    }

    #[test]
    fn autoscale_json_strategy_circuit_cooldown_path_works() {
        let payload = serde_json::json!({
            "mode": "strategy_circuit_cooldown",
            "strategy_circuit_cooldown_input": {
                "last_error_code": "rate_limit_hit",
                "http_429_cooldown_hours": 2,
                "http_5xx_cooldown_hours": 8,
                "dns_error_cooldown_hours": 4
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale strategy_circuit_cooldown");
        assert!(out.contains("\"mode\":\"strategy_circuit_cooldown\""));
        assert!(out.contains("\"cooldown_hours\":2.0"));
    }

    #[test]
    fn strategy_trit_shadow_adjusted_applies_bonus_blend() {
        let out = compute_strategy_trit_shadow_adjusted(&StrategyTritShadowAdjustedInput {
            base_score: 68.75,
            bonus_raw: 12.345,
            bonus_blend: 0.4,
        });
        assert!((out.bonus_applied - 4.938).abs() < 0.000001);
        assert!((out.adjusted_score - 73.688).abs() < 0.000001);
    }

