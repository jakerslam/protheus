    #[test]
    fn to_stem_matches_ts_semantics() {
        let short = compute_to_stem(&ToStemInput {
            token: Some("abc".to_string()),
        });
        assert_eq!(short.stem, "abc");

        let long = compute_to_stem(&ToStemInput {
            token: Some("memory".to_string()),
        });
        assert_eq!(long.stem, "memor");
    }

    #[test]
    fn autoscale_json_to_stem_path_works() {
        let payload = serde_json::json!({
            "mode": "to_stem",
            "to_stem_input": {
                "token": "security"
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale to_stem");
        assert!(out.contains("\"mode\":\"to_stem\""));
        assert!(out.contains("\"stem\":\"secur\""));
    }

    #[test]
    fn normalize_directive_text_matches_ts_semantics() {
        let out = compute_normalize_directive_text(&NormalizeDirectiveTextInput {
            text: Some(" Memory++ Drift\nPlan! ".to_string()),
        });
        assert_eq!(out.normalized, "memory drift plan");
    }

    #[test]
    fn autoscale_json_normalize_directive_text_path_works() {
        let payload = serde_json::json!({
            "mode": "normalize_directive_text",
            "normalize_directive_text_input": {
                "text": " Safety-first, always. "
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale normalize_directive_text");
        assert!(out.contains("\"mode\":\"normalize_directive_text\""));
        assert!(out.contains("\"normalized\":\"safety first always\""));
    }

    #[test]
    fn tokenize_directive_text_matches_ts_filters() {
        let out = compute_tokenize_directive_text(&TokenizeDirectiveTextInput {
            text: Some("The memory plan 123 avoids drift".to_string()),
            stopwords: vec!["the".to_string(), "plan".to_string()],
        });
        assert_eq!(
            out.tokens,
            vec![
                "memory".to_string(),
                "avoids".to_string(),
                "drift".to_string()
            ]
        );
    }

    #[test]
    fn autoscale_json_tokenize_directive_text_path_works() {
        let payload = serde_json::json!({
            "mode": "tokenize_directive_text",
            "tokenize_directive_text_input": {
                "text": "The memory plan 123 avoids drift",
                "stopwords": ["the", "plan"]
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale tokenize_directive_text");
        assert!(out.contains("\"mode\":\"tokenize_directive_text\""));
        assert!(out.contains("\"tokens\":[\"memory\",\"avoids\",\"drift\"]"));
    }

    #[test]
    fn normalize_spaces_matches_ts_semantics() {
        let out = compute_normalize_spaces(&NormalizeSpacesInput {
            text: Some("  one\t two\nthree   ".to_string()),
        });
        assert_eq!(out.normalized, "one two three");
    }

    #[test]
    fn autoscale_json_normalize_spaces_path_works() {
        let payload = serde_json::json!({
            "mode": "normalize_spaces",
            "normalize_spaces_input": {
                "text": "  one\t two\nthree   "
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale normalize_spaces");
        assert!(out.contains("\"mode\":\"normalize_spaces\""));
        assert!(out.contains("\"normalized\":\"one two three\""));
    }

    #[test]
    fn parse_lower_list_matches_ts_semantics() {
        let from_list = compute_parse_lower_list(&ParseLowerListInput {
            list: vec![" A ".to_string(), "b".to_string(), "".to_string()],
            csv: Some("x,y".to_string()),
        });
        assert_eq!(from_list.items, vec!["a".to_string(), "b".to_string()]);

        let from_csv = compute_parse_lower_list(&ParseLowerListInput {
            list: vec![],
            csv: Some(" A, B ,,C ".to_string()),
        });
        assert_eq!(
            from_csv.items,
            vec!["a".to_string(), "b".to_string(), "c".to_string()]
        );
    }

    #[test]
    fn autoscale_json_parse_lower_list_path_works() {
        let payload = serde_json::json!({
            "mode": "parse_lower_list",
            "parse_lower_list_input": {
                "list": [],
                "csv": "A, B ,, C"
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale parse_lower_list");
        assert!(out.contains("\"mode\":\"parse_lower_list\""));
        assert!(out.contains("\"items\":[\"a\",\"b\",\"c\"]"));
    }

    #[test]
    fn canary_failed_checks_allowed_matches_subset_rules() {
        let allowed = compute_canary_failed_checks_allowed(&CanaryFailedChecksAllowedInput {
            failed_checks: vec!["lint".to_string(), "format".to_string()],
            allowed_checks: vec![
                "lint".to_string(),
                "format".to_string(),
                "typecheck".to_string(),
            ],
        });
        assert!(allowed.allowed);

        let blocked = compute_canary_failed_checks_allowed(&CanaryFailedChecksAllowedInput {
            failed_checks: vec!["lint".to_string(), "security".to_string()],
            allowed_checks: vec!["lint".to_string(), "format".to_string()],
        });
        assert!(!blocked.allowed);
    }

    #[test]
    fn autoscale_json_canary_failed_checks_allowed_path_works() {
        let payload = serde_json::json!({
            "mode": "canary_failed_checks_allowed",
            "canary_failed_checks_allowed_input": {
                "failed_checks": ["lint", "format"],
                "allowed_checks": ["lint", "format", "typecheck"]
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale canary_failed_checks_allowed");
        assert!(out.contains("\"mode\":\"canary_failed_checks_allowed\""));
        assert!(out.contains("\"allowed\":true"));
    }

    #[test]
    fn proposal_text_blob_matches_join_and_normalization() {
        let out = compute_proposal_text_blob(&ProposalTextBlobInput {
            title: Some("Fix Drift".to_string()),
            summary: Some("Improve safety".to_string()),
            suggested_next_command: Some("run checks".to_string()),
            suggested_command: None,
            notes: Some(" urgent ".to_string()),
            evidence: vec![ProposalTextBlobEvidenceEntryInput {
                evidence_ref: Some("ref://a".to_string()),
                path: Some("docs/client/a.md".to_string()),
                title: Some("Doc A".to_string()),
            }],
        });
        assert_eq!(
            out.blob,
            "fix drift | improve safety | run checks | urgent | ref://a | docs/client/a.md | doc a"
        );
    }

    #[test]
    fn autoscale_json_proposal_text_blob_path_works() {
        let payload = serde_json::json!({
            "mode": "proposal_text_blob",
            "proposal_text_blob_input": {
                "title": "Fix Drift",
                "summary": "Improve safety",
                "suggested_next_command": "run checks",
                "notes": "urgent",
                "evidence": [
                    {
                        "evidence_ref": "ref://a",
                        "path": "docs/client/a.md",
                        "title": "Doc A"
                    }
                ]
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale proposal_text_blob");
        assert!(out.contains("\"mode\":\"proposal_text_blob\""));
        assert!(out.contains("\"blob\":\"fix drift | improve safety | run checks | urgent | ref://a | docs/client/a.md | doc a\""));
    }

    #[test]
    fn percent_mentions_from_text_matches_extraction_rules() {
        let out = compute_percent_mentions_from_text(&PercentMentionsFromTextInput {
            text: Some("improve by 12.5% then -2% then 140%".to_string()),
        });
        assert_eq!(out.values, vec![12.5, 100.0]);
    }

    #[test]
    fn autoscale_json_percent_mentions_from_text_path_works() {
        let payload = serde_json::json!({
            "mode": "percent_mentions_from_text",
            "percent_mentions_from_text_input": {
                "text": "gain 10% and 25%"
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale percent_mentions_from_text");
        assert!(out.contains("\"mode\":\"percent_mentions_from_text\""));
        assert!(out.contains("\"values\":[10.0,25.0]"));
    }

    #[test]
    fn optimization_min_delta_percent_respects_mode() {
        let high = compute_optimization_min_delta_percent(&OptimizationMinDeltaPercentInput {
            high_accuracy_mode: true,
            high_accuracy_value: 3.5,
            base_value: 8.0,
        });
        assert!((high.min_delta_percent - 3.5).abs() < 0.000001);

        let normal = compute_optimization_min_delta_percent(&OptimizationMinDeltaPercentInput {
            high_accuracy_mode: false,
            high_accuracy_value: 3.5,
            base_value: 8.0,
        });
        assert!((normal.min_delta_percent - 8.0).abs() < 0.000001);
    }

    #[test]
    fn autoscale_json_optimization_min_delta_percent_path_works() {
        let payload = serde_json::json!({
            "mode": "optimization_min_delta_percent",
            "optimization_min_delta_percent_input": {
                "high_accuracy_mode": true,
                "high_accuracy_value": 3.5,
                "base_value": 8.0
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale optimization_min_delta_percent");
        assert!(out.contains("\"mode\":\"optimization_min_delta_percent\""));
        assert!(out.contains("\"min_delta_percent\":3.5"));
    }

    #[test]
    fn source_eye_ref_prefers_meta_then_evidence_then_unknown() {
        let meta = compute_source_eye_ref(&SourceEyeRefInput {
            meta_source_eye: Some("primary".to_string()),
            first_evidence_ref: Some("eye:secondary".to_string()),
        });
        assert_eq!(meta.eye_ref, "eye:primary");

        let evidence = compute_source_eye_ref(&SourceEyeRefInput {
            meta_source_eye: None,
            first_evidence_ref: Some("eye:secondary".to_string()),
        });
        assert_eq!(evidence.eye_ref, "eye:secondary");

        let unknown = compute_source_eye_ref(&SourceEyeRefInput {
            meta_source_eye: None,
            first_evidence_ref: Some("ref://other".to_string()),
        });
        assert_eq!(unknown.eye_ref, "eye:unknown_eye");
    }

    #[test]
    fn autoscale_json_source_eye_ref_path_works() {
        let payload = serde_json::json!({
            "mode": "source_eye_ref",
            "source_eye_ref_input": {
                "meta_source_eye": "market",
                "first_evidence_ref": "eye:other"
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale source_eye_ref");
        assert!(out.contains("\"mode\":\"source_eye_ref\""));
        assert!(out.contains("\"eye_ref\":\"eye:market\""));
    }

    #[test]
    fn normalized_risk_only_allows_expected_levels() {
        let high = compute_normalized_risk(&NormalizedRiskInput {
            risk: Some("HIGH".to_string()),
        });
        assert_eq!(high.risk, "high");

        let fallback = compute_normalized_risk(&NormalizedRiskInput {
            risk: Some("critical".to_string()),
        });
        assert_eq!(fallback.risk, "low");
    }

    #[test]
    fn autoscale_json_normalized_risk_path_works() {
        let payload = serde_json::json!({
            "mode": "normalized_risk",
            "normalized_risk_input": {
                "risk": "medium"
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale normalized_risk");
        assert!(out.contains("\"mode\":\"normalized_risk\""));
        assert!(out.contains("\"risk\":\"medium\""));
    }

