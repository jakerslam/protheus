#[cfg(test)]
mod contract_specific_gate_regression_tests {
    use super::*;
    use serde_json::Value;

    fn run_strict_payload(
        root: &Path,
        id: &str,
        payload_json: Option<&str>,
    ) -> Result<Value, String> {
        let mut args = vec!["--strict=1".to_string(), "--apply=0".to_string()];
        if let Some(payload) = payload_json {
            args.push(format!("--payload-json={payload}"));
        }
        run_payload(root, id, "run", &args)
    }

    #[test]
    fn v6_dashboard_009_1_requires_message_only_hover_scope() {
        let root = tempfile::tempdir().expect("tempdir");
        let err = run_strict_payload(
            root.path(),
            "V6-DASHBOARD-009.1",
            Some(
                r#"{"metadata_hover_scope":"thread","hover_pushdown_layout_enabled":true,"stack_interrupts_on_notifications":true,"messages":[{"source":"system","kind":"message"}]}"#,
            ),
        )
        .expect_err("non-message hover scope should fail strict guard");
        assert!(
            err.contains("specific_dashboard_metadata_hover_scope_mismatch"),
            "expected message hover scope violation, got {err}"
        );
    }

    #[test]
    fn v6_dashboard_009_2_requires_server_status_on_startup_failure() {
        let root = tempfile::tempdir().expect("tempdir");
        let err = run_strict_payload(
            root.path(),
            "V6-DASHBOARD-009.2",
            Some(
                r#"{"boot_retry_enabled":true,"boot_retry_max_attempts":5,"boot_retry_backoff_ms":1000,"startup_failed":true,"server_status_emitted":false,"status_error_code":"backend_http_404"}"#,
            ),
        )
        .expect_err("startup failure without status artifact should fail strict guard");
        assert!(
            err.contains("specific_dashboard_server_status_missing_on_failure"),
            "expected startup status violation, got {err}"
        );
    }

    #[test]
    fn v6_infring_gap_001_4_requires_security_parity_controls() {
        let root = tempfile::tempdir().expect("tempdir");
        let err = run_strict_payload(
            root.path(),
            "V6-INFRING-GAP-001.4",
            Some(
                r#"{"taint_tracking_enabled":false,"merkle_audit_chain_enabled":true,"manifest_signing_enabled":true,"ssrf_deny_paths_enabled":true}"#,
            ),
        )
        .expect_err("missing taint tracking should fail strict guard");
        assert!(
            err.contains("specific_infring_gap_taint_tracking_disabled"),
            "expected taint tracking violation, got {err}"
        );
    }

    #[test]
    fn v10_perf_001_1_enforces_receipt_batch_size_bounds() {
        let root = tempfile::tempdir().expect("tempdir");
        let err = run_strict_payload(
            root.path(),
            "V10-PERF-001.1",
            Some(r#"{"receipt_batching_enabled":true,"receipt_batch_size":2}"#),
        )
        .expect_err("out-of-range receipt batch size should fail strict guard");
        assert!(
            err.contains("specific_perf_receipt_batch_size_out_of_range"),
            "expected receipt batch size violation, got {err}"
        );
    }

    #[test]
    fn v10_perf_001_6_emits_perf_guard_when_defaults_pass() {
        let root = tempfile::tempdir().expect("tempdir");
        let out = run_strict_payload(root.path(), "V10-PERF-001.6", None)
            .expect("perf regression guard contract should succeed with defaults");
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        let perf_guard = out
            .get("contract_execution")
            .and_then(Value::as_object)
            .and_then(|row| row.get("specific_checks"))
            .and_then(Value::as_object)
            .and_then(|row| row.get("perf_guard"))
            .and_then(Value::as_object)
            .cloned()
            .expect("expected perf_guard object");
        assert_eq!(
            perf_guard
                .get("throughput_regression_guard_enabled")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert!(
            perf_guard
                .get("throughput_drop_threshold_pct")
                .and_then(Value::as_f64)
                .unwrap_or(100.0)
                <= 5.0
        );
    }

    #[test]
    fn v4_dual_con_001_requires_duality_bundle() {
        let root = tempfile::tempdir().expect("tempdir");
        let err = run_strict_payload(
            root.path(),
            "V4-DUAL-CON-001",
            Some(r#"{"duality_bundle_emitted":false,"harmony_score":0.91}"#),
        )
        .expect_err("missing duality bundle should fail strict guard");
        assert!(
            err.contains("specific_duality_bundle_missing"),
            "expected duality bundle violation, got {err}"
        );
    }

    #[test]
    fn v4_dual_mem_002_requires_inversion_tagging() {
        let root = tempfile::tempdir().expect("tempdir");
        let err = run_strict_payload(
            root.path(),
            "V4-DUAL-MEM-002",
            Some(r#"{"dual_memory_tagging_enabled":true,"inversion_candidate_tagging_enabled":false}"#),
        )
        .expect_err("missing inversion tagging should fail strict guard");
        assert!(
            err.contains("specific_duality_inversion_tagging_disabled"),
            "expected inversion tagging violation, got {err}"
        );
    }

    #[test]
    fn v6_infring_gap_001_2_passes_with_default_contract_payload() {
        let root = tempfile::tempdir().expect("tempdir");
        let out = run_strict_payload(root.path(), "V6-INFRING-GAP-001.2", None)
            .expect("http/ws parity contract should pass with defaults");
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
    }

    #[test]
    fn v6_infring_gap_001_3_passes_with_default_contract_payload() {
        let root = tempfile::tempdir().expect("tempdir");
        let out = run_strict_payload(root.path(), "V6-INFRING-GAP-001.3", None)
            .expect("channel adapter parity contract should pass with defaults");
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
    }

    #[test]
    fn v6_infring_gap_001_5_passes_with_default_contract_payload() {
        let root = tempfile::tempdir().expect("tempdir");
        let out = run_strict_payload(root.path(), "V6-INFRING-GAP-001.5", None)
            .expect("hands/skills parity contract should pass with defaults");
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
    }
}
