
#[cfg(test)]
mod duality_v4_tests {
    use super::*;

    #[test]
    fn dual_voice_evaluate_emits_harmony_contract() {
        let root = tempfile::tempdir().expect("tempdir");
        let out = op_dispatch(
            root.path(),
            "dual_voice_evaluate",
            Some(&json!({
                "context": {
                    "run_id": "dual-voice-test",
                    "objective": "maintain order and creativity in balance"
                },
                "left": {
                    "objective": "structured planning and safety discipline"
                },
                "right": {
                    "objective": "creative adaptation and inversion exploration"
                }
            })),
        )
        .expect("dual voice");
        assert_eq!(
            out.get("type").and_then(Value::as_str),
            Some("duality_dual_voice_evaluation")
        );
        assert!(out.get("harmony").and_then(Value::as_f64).is_some());
        assert!(out.get("left_voice").is_some());
        assert!(out.get("right_voice").is_some());
    }

    #[test]
    fn duality_toll_update_increases_debt_for_negative_signal() {
        let root = tempfile::tempdir().expect("tempdir");
        let out = op_dispatch(
            root.path(),
            "duality_toll",
            Some(&json!({
                "signal": {
                    "score_trit": -1,
                    "balance_score": -0.72,
                    "zero_point_harmony_potential": 0.08,
                    "lane": "spine"
                },
                "context": {
                    "run_id": "toll-test"
                },
                "opts": {
                    "persist": true
                }
            })),
        )
        .expect("toll");
        let debt_before = out
            .pointer("/toll/debt_before")
            .and_then(Value::as_f64)
            .unwrap_or(0.0);
        let debt_after = out
            .pointer("/toll/debt_after")
            .and_then(Value::as_f64)
            .unwrap_or(0.0);
        assert!(debt_after >= debt_before);
        let state = op_dispatch(root.path(), "loadDualityState", None).expect("state");
        assert!(
            state
                .get("toll_debt")
                .and_then(Value::as_f64)
                .unwrap_or(0.0)
                >= debt_after - 0.000001
        );
    }

    #[test]
    fn duality_toll_update_recovers_debt_for_balanced_signal() {
        let root = tempfile::tempdir().expect("tempdir");
        let _ = op_dispatch(
            root.path(),
            "duality_toll",
            Some(&json!({
                "signal": {
                    "score_trit": -1,
                    "balance_score": -0.81,
                    "zero_point_harmony_potential": 0.05
                },
                "opts": {"persist": true}
            })),
        )
        .expect("seed debt");

        let out = op_dispatch(
            root.path(),
            "duality_toll",
            Some(&json!({
                "signal": {
                    "score_trit": 1,
                    "balance_score": 0.88,
                    "zero_point_harmony_potential": 0.92
                },
                "opts": {"persist": true}
            })),
        )
        .expect("recover debt");
        let debt_before = out
            .pointer("/toll/debt_before")
            .and_then(Value::as_f64)
            .unwrap_or(0.0);
        let debt_after = out
            .pointer("/toll/debt_after")
            .and_then(Value::as_f64)
            .unwrap_or(0.0);
        assert!(debt_after <= debt_before);
    }

    #[test]
    fn duality_memory_tag_marks_extremes_for_review() {
        let root = tempfile::tempdir().expect("tempdir");
        let out = op_dispatch(
            root.path(),
            "duality_memory_tag",
            Some(&json!({
                "nodes": [
                    {
                        "key": "focus.mode",
                        "value": "maximize rigid structure and strict control without adaptation",
                        "signal": {
                            "score_trit": -1,
                            "balance_score": -0.78,
                            "zero_point_harmony_potential": 0.09,
                            "recommended_adjustment": "increase_yin_order"
                        }
                    }
                ]
            })),
        )
        .expect("memory tag");
        let first = out
            .get("nodes")
            .and_then(Value::as_array)
            .and_then(|rows| rows.first())
            .cloned()
            .unwrap_or_else(|| json!({}));
        assert_eq!(
            first
                .pointer("/duality_tags/inversion_review_flag")
                .and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn parse_codex_text_dedupes_flow_values_while_preserving_order() {
        let parsed = parse_codex_text(
            r#"
            [flow_values]
            observe/reflect, fetch/parse; observe/reflect
            "#,
        );
        assert_eq!(
            parsed.get("flow_values").cloned().unwrap_or(Value::Null),
            json!(["observe/reflect", "fetch/parse"])
        );
    }
}

