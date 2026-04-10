mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn ethical_run_emits_expected_flags() {
        let tmp = tempdir().expect("tmp");
        let root = tmp.path();

        let policy_path = root.join("config/ethical_reasoning_policy.json");
        let state_dir = root.join("local/state/autonomy/ethical_reasoning");
        let weaver_path = root.join("local/state/autonomy/weaver/latest.json");
        let mirror_path = root.join("local/state/autonomy/mirror_organ/latest.json");

        write_json_atomic(
            &policy_path,
            &json!({
                "enabled": true,
                "thresholds": {
                    "monoculture_warn_share": 0.6,
                    "high_impact_share": 0.7,
                    "maturity_min_for_prior_updates": 0.4,
                    "mirror_pressure_warn": 0.5
                },
                "max_prior_delta_per_run": 0.05,
                "integration": {
                    "weaver_latest_path": weaver_path,
                    "mirror_latest_path": mirror_path
                }
            }),
        )
        .expect("policy");

        write_json_atomic(
            &weaver_path,
            &json!({
                "run_id": "weaver_demo",
                "objective_id": "heroic_growth",
                "value_context": {
                    "allocations": [
                        { "metric_id": "revenue", "share": 0.81, "raw_score": 0.9 },
                        { "metric_id": "learning", "share": 0.11, "raw_score": 0.5 },
                        { "metric_id": "quality", "share": 0.08, "raw_score": 0.45 }
                    ]
                }
            }),
        )
        .expect("weaver");
        write_json_atomic(&mirror_path, &json!({ "pressure_score": 0.77 })).expect("mirror");

        let out = run_ethical_reasoning(
            root,
            &json!({ "objective_id": "heroic_growth", "maturity_score": 0.9 }),
            Some(&policy_path),
            Some(&state_dir),
            true,
        );

        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        let reasons = out
            .get("reason_codes")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(reasons
            .iter()
            .any(|v| v.as_str() == Some("ethical_monoculture_warning")));
        assert!(reasons
            .iter()
            .any(|v| v.as_str() == Some("ethical_mirror_pressure_warning")));
        assert!(out
            .get("tradeoff_receipts")
            .and_then(Value::as_array)
            .map(|v| !v.is_empty())
            .unwrap_or(false));
        assert_eq!(
            out.get("summary")
                .and_then(Value::as_object)
                .and_then(|m| m.get("priors_updated"))
                .and_then(Value::as_bool),
            Some(true)
        );

        let status = ethical_reasoning_status(root, Some(&policy_path), Some(&state_dir));
        assert_eq!(status.get("ok").and_then(Value::as_bool), Some(true));
        assert!(status.get("priors").map(Value::is_object).unwrap_or(false));
    }
}
