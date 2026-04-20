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

