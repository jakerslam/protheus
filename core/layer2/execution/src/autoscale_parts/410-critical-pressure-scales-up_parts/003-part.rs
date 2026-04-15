    #[test]
    fn autoscale_json_proposal_score_path_works() {
        let payload = serde_json::json!({
            "mode": "proposal_score",
            "proposal_score_input": {
                "impact_weight": 3,
                "risk_penalty": 2,
                "age_hours": 24,
                "is_stub": false,
                "no_change_count": 1,
                "reverted_count": 0
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale proposal_score");
        assert!(out.contains("\"mode\":\"proposal_score\""));
    }

    #[test]
    fn proposal_admission_preview_returns_object_only() {
        let object_preview = compute_proposal_admission_preview(&ProposalAdmissionPreviewInput {
            admission_preview: Some(serde_json::json!({"allow": true, "reason": "ok"})),
        });
        assert!(object_preview.preview.is_some());

        let array_preview = compute_proposal_admission_preview(&ProposalAdmissionPreviewInput {
            admission_preview: Some(serde_json::json!(["ok"])),
        });
        assert!(array_preview.preview.is_some());

        let scalar_preview = compute_proposal_admission_preview(&ProposalAdmissionPreviewInput {
            admission_preview: Some(serde_json::json!("not-an-object")),
        });
        assert!(scalar_preview.preview.is_none());
    }

    #[test]
    fn autoscale_json_proposal_admission_preview_path_works() {
        let payload = serde_json::json!({
            "mode": "proposal_admission_preview",
            "proposal_admission_preview_input": {
                "admission_preview": {
                    "allow": true,
                    "reason": "ok"
                }
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale proposal_admission_preview");
        assert!(out.contains("\"mode\":\"proposal_admission_preview\""));
    }

    #[test]
    fn impact_weight_maps_expected_impact() {
        let high = compute_impact_weight(&ImpactWeightInput {
            expected_impact: Some("high".to_string()),
        });
        assert_eq!(high.weight, 3);
        let low = compute_impact_weight(&ImpactWeightInput {
            expected_impact: Some("low".to_string()),
        });
        assert_eq!(low.weight, 1);
    }

    #[test]
    fn autoscale_json_impact_weight_path_works() {
        let payload = serde_json::json!({
            "mode": "impact_weight",
            "impact_weight_input": {
                "expected_impact": "medium"
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale impact_weight");
        assert!(out.contains("\"mode\":\"impact_weight\""));
    }

    #[test]
    fn list_proposal_files_filters_and_sorts() {
        let out = compute_list_proposal_files(&ListProposalFilesInput {
            entries: vec![
                "README.md".to_string(),
                "2026-03-02.json".to_string(),
                "2026-03-01.json".to_string(),
                "2026-03-01.jsonl".to_string(),
            ],
        });
        assert_eq!(
            out.files,
            vec!["2026-03-01.json".to_string(), "2026-03-02.json".to_string()]
        );
    }

    #[test]
    fn autoscale_json_list_proposal_files_path_works() {
        let payload = serde_json::json!({
            "mode": "list_proposal_files",
            "list_proposal_files_input": {
                "entries": ["2026-03-02.json", "bad.txt", "2026-03-01.json"]
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale list_proposal_files");
        assert!(out.contains("\"mode\":\"list_proposal_files\""));
        assert!(out.contains("\"files\":[\"2026-03-01.json\",\"2026-03-02.json\"]"));
    }

    #[test]
    fn risk_penalty_maps_risk_levels() {
        let high = compute_risk_penalty(&RiskPenaltyInput {
            risk: Some("high".to_string()),
        });
        assert_eq!(high.penalty, 2);
        let low = compute_risk_penalty(&RiskPenaltyInput {
            risk: Some("low".to_string()),
        });
        assert_eq!(low.penalty, 0);
    }

    #[test]
    fn autoscale_json_risk_penalty_path_works() {
        let payload = serde_json::json!({
            "mode": "risk_penalty",
            "risk_penalty_input": {
                "risk": "medium"
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale risk_penalty");
        assert!(out.contains("\"mode\":\"risk_penalty\""));
    }

    #[test]
    fn estimate_tokens_maps_expected_impact() {
        let high = compute_estimate_tokens(&EstimateTokensInput {
            expected_impact: Some("high".to_string()),
        });
        assert_eq!(high.est_tokens, 1400);
        let low = compute_estimate_tokens(&EstimateTokensInput {
            expected_impact: Some("low".to_string()),
        });
        assert_eq!(low.est_tokens, 300);
    }

    #[test]
    fn autoscale_json_estimate_tokens_path_works() {
        let payload = serde_json::json!({
            "mode": "estimate_tokens",
            "estimate_tokens_input": {
                "expected_impact": "medium"
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale estimate_tokens");
        assert!(out.contains("\"mode\":\"estimate_tokens\""));
    }

    #[test]
    fn proposal_remediation_depth_prefers_explicit_then_trigger() {
        let explicit = compute_proposal_remediation_depth(&ProposalRemediationDepthInput {
            remediation_depth: Some(2.4),
            trigger: Some("consecutive_failures".to_string()),
        });
        assert_eq!(explicit.depth, 2);

        let trigger = compute_proposal_remediation_depth(&ProposalRemediationDepthInput {
            remediation_depth: None,
            trigger: Some("multi_eye_transport_failure".to_string()),
        });
        assert_eq!(trigger.depth, 1);

        let none = compute_proposal_remediation_depth(&ProposalRemediationDepthInput {
            remediation_depth: None,
            trigger: Some("".to_string()),
        });
        assert_eq!(none.depth, 0);
    }

    #[test]
    fn autoscale_json_proposal_remediation_depth_path_works() {
        let payload = serde_json::json!({
            "mode": "proposal_remediation_depth",
            "proposal_remediation_depth_input": {
                "remediation_depth": null,
                "trigger": "consecutive_failures"
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale proposal_remediation_depth");
        assert!(out.contains("\"mode\":\"proposal_remediation_depth\""));
        assert!(out.contains("\"depth\":1"));
    }

    #[test]
    fn proposal_dedup_key_uses_remediation_and_id_paths() {
        let remediation = compute_proposal_dedup_key(&ProposalDedupKeyInput {
            proposal_type: Some("ops_remediation".to_string()),
            source_eye_id: Some("github_release".to_string()),
            remediation_kind: Some("transport".to_string()),
            proposal_id: Some("abc-1".to_string()),
        });
        assert_eq!(
            remediation.dedup_key,
            "ops_remediation|github_release|transport"
        );

        let generic = compute_proposal_dedup_key(&ProposalDedupKeyInput {
            proposal_type: Some("feature".to_string()),
            source_eye_id: Some("unknown_eye".to_string()),
            remediation_kind: None,
            proposal_id: Some("abc-1".to_string()),
        });
        assert_eq!(generic.dedup_key, "feature|unknown_eye|abc-1");
    }

    #[test]
    fn autoscale_json_proposal_dedup_key_path_works() {
        let payload = serde_json::json!({
            "mode": "proposal_dedup_key",
            "proposal_dedup_key_input": {
                "proposal_type": "ops_remediation",
                "source_eye_id": "github_release",
                "remediation_kind": "transport",
                "proposal_id": "abc-1"
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale proposal_dedup_key");
        assert!(out.contains("\"mode\":\"proposal_dedup_key\""));
        assert!(out.contains("\"dedup_key\":\"ops_remediation|github_release|transport\""));
    }

    #[test]
    fn proposal_semantic_fingerprint_builds_unique_sorted_stems() {
        let out = compute_proposal_semantic_fingerprint(&ProposalSemanticFingerprintInput {
            proposal_id: Some("p-1".to_string()),
            proposal_type: Some("ops_remediation".to_string()),
            source_eye: Some("GitHub_Release".to_string()),
            objective_id: Some("T1_Objective".to_string()),
            text_blob: Some("Rust bridge parity tests for transport fixes".to_string()),
            stopwords: vec!["for".to_string()],
            min_tokens: Some(3.0),
        });
        assert_eq!(out.proposal_id, Some("p-1".to_string()));
        assert_eq!(out.proposal_type, "ops_remediation".to_string());
        assert_eq!(out.source_eye, Some("github_release".to_string()));
        assert_eq!(out.objective_id, Some("T1_Objective".to_string()));
        assert!(out.token_stems.windows(2).all(|w| w[0] <= w[1]));
        assert!(out.token_count >= 3);
        assert!(out.eligible);
    }

    #[test]
    fn autoscale_json_proposal_semantic_fingerprint_path_works() {
        let payload = serde_json::json!({
            "mode": "proposal_semantic_fingerprint",
            "proposal_semantic_fingerprint_input": {
                "proposal_id": "p-1",
                "proposal_type": "ops_remediation",
                "source_eye": "github_release",
                "objective_id": "T1_Objective",
                "text_blob": "Rust bridge parity tests",
                "min_tokens": 2
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale proposal_semantic_fingerprint");
        assert!(out.contains("\"mode\":\"proposal_semantic_fingerprint\""));
    }

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
