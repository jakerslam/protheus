
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

    #[test]
    fn parse_iso_ts_returns_timestamp_when_valid() {
        let valid = compute_parse_iso_ts(&ParseIsoTsInput {
            ts: Some("2026-03-01T00:00:00.000Z".to_string()),
        });
        assert!(valid.timestamp_ms.is_some());

        let invalid = compute_parse_iso_ts(&ParseIsoTsInput {
            ts: Some("not-a-date".to_string()),
        });
        assert!(invalid.timestamp_ms.is_none());
    }

    #[test]
    fn autoscale_json_parse_iso_ts_path_works() {
        let payload = serde_json::json!({
            "mode": "parse_iso_ts",
            "parse_iso_ts_input": {
                "ts": "2026-03-01T00:00:00.000Z"
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale parse_iso_ts");
        assert!(out.contains("\"mode\":\"parse_iso_ts\""));
        assert!(out.contains("\"timestamp_ms\":"));
    }

    #[test]
    fn extract_objective_id_token_matches_expected_patterns() {
        let direct = compute_extract_objective_id_token(&ExtractObjectiveIdTokenInput {
            value: Some("T12_build_router".to_string()),
        });
        assert_eq!(direct.objective_id.as_deref(), Some("T12_build_router"));

        let embedded = compute_extract_objective_id_token(&ExtractObjectiveIdTokenInput {
            value: Some("objective: T8_fix_drift soon".to_string()),
        });
        assert_eq!(embedded.objective_id.as_deref(), Some("T8_fix_drift"));

        let none = compute_extract_objective_id_token(&ExtractObjectiveIdTokenInput {
            value: Some("no token".to_string()),
        });
        assert!(none.objective_id.is_none());
    }

    #[test]
    fn autoscale_json_extract_objective_id_token_path_works() {
        let payload = serde_json::json!({
            "mode": "extract_objective_id_token",
            "extract_objective_id_token_input": {
                "value": "objective: T8_fix_drift soon"
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale extract_objective_id_token");
        assert!(out.contains("\"mode\":\"extract_objective_id_token\""));
        assert!(out.contains("\"objective_id\":\"T8_fix_drift\""));
    }

    #[test]
    fn autoscale_json_value_density_score_path_works() {
        let payload = serde_json::json!({
            "mode": "value_density_score",
            "value_density_score_input": {
                "expected_value": 40,
                "est_tokens": 1000
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale value_density_score");
        assert!(out.contains("\"mode\":\"value_density_score\""));
    }

    #[test]
    fn execution_reserve_snapshot_applies_reserve_math() {
        let out = compute_execution_reserve_snapshot(&ExecutionReserveSnapshotInput {
            cap: 1000.0,
            used: 950.0,
            reserve_enabled: true,
            reserve_ratio: 0.12,
            reserve_min_tokens: 600.0,
        });
        assert!(out.enabled);
        assert_eq!(out.reserve_tokens, 600.0);
        assert_eq!(out.reserve_remaining, 50.0);
    }

    #[test]
    fn autoscale_json_execution_reserve_snapshot_path_works() {
        let payload = serde_json::json!({
            "mode": "execution_reserve_snapshot",
            "execution_reserve_snapshot_input": {
                "cap": 1000,
                "used": 950,
                "reserve_enabled": true,
                "reserve_ratio": 0.12,
                "reserve_min_tokens": 600
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale execution_reserve_snapshot");
        assert!(out.contains("\"mode\":\"execution_reserve_snapshot\""));
    }

    #[test]
    fn budget_pacing_gate_blocks_high_token_low_value_when_tight() {
        let out = compute_budget_pacing_gate(&BudgetPacingGateInput {
            est_tokens: 1800.0,
            value_signal_score: 45.0,
            risk: Some("medium".to_string()),
            snapshot_tight: true,
            snapshot_autopause_active: false,
            snapshot_remaining_ratio: 0.18,
            snapshot_pressure: Some("hard".to_string()),
            execution_floor_deficit: false,
            execution_reserve_enabled: true,
            execution_reserve_remaining: 200.0,
            execution_reserve_min_value_signal: 70.0,
            budget_pacing_enabled: true,
            min_remaining_ratio: 0.2,
            high_token_threshold: 1200.0,
            min_value_signal_score: 60.0,
        });
        assert!(!out.pass);
        assert_eq!(
            out.reason.as_deref(),
            Some("budget_pacing_high_token_low_value")
        );
    }

    #[test]
    fn autoscale_json_budget_pacing_gate_path_works() {
        let payload = serde_json::json!({
            "mode": "budget_pacing_gate",
            "budget_pacing_gate_input": {
                "est_tokens": 300,
                "value_signal_score": 80,
                "risk": "low",
                "snapshot_tight": true,
                "snapshot_autopause_active": false,
                "snapshot_remaining_ratio": 0.12,
                "snapshot_pressure": "warn",
                "execution_floor_deficit": true,
                "execution_reserve_enabled": true,
                "execution_reserve_remaining": 500,
                "execution_reserve_min_value_signal": 70,
                "budget_pacing_enabled": true,
                "min_remaining_ratio": 0.2,
                "high_token_threshold": 1200,
                "min_value_signal_score": 60
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale budget_pacing_gate");
        assert!(out.contains("\"mode\":\"budget_pacing_gate\""));
        assert!(out.contains("\"pass\":true"));
    }

    #[test]
    fn capability_cap_prefers_primary_then_aliases() {
        let out = compute_capability_cap(&CapabilityCapInput {
            caps: std::collections::BTreeMap::from([
                ("proposal:ops_remediation".to_string(), 4.2),
                ("proposal:feature".to_string(), 2.0),
            ]),
            primary_key: Some("proposal:ops_remediation".to_string()),
            aliases: vec!["proposal:feature".to_string()],
        });
        assert_eq!(out.cap, Some(4));

        let alias = compute_capability_cap(&CapabilityCapInput {
            caps: std::collections::BTreeMap::from([("alias:key".to_string(), 3.0)]),
            primary_key: Some("missing:key".to_string()),
            aliases: vec!["alias:key".to_string()],
        });
        assert_eq!(alias.cap, Some(3));
    }

    #[test]
    fn autoscale_json_capability_cap_path_works() {
        let payload = serde_json::json!({
            "mode": "capability_cap",
            "capability_cap_input": {
                "caps": {
                    "proposal:ops_remediation": 5
                },
                "primary_key": "proposal:ops_remediation",
                "aliases": ["proposal:feature"]
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale capability_cap");
        assert!(out.contains("\"mode\":\"capability_cap\""));
        assert!(out.contains("\"cap\":5"));
    }

    #[test]
    fn estimate_tokens_for_candidate_prefers_direct_then_route_then_fallback() {
