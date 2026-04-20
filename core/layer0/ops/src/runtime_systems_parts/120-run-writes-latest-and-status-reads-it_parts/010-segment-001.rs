#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_system_contracts::actionable_ids;

    fn runtime_temp_root() -> tempfile::TempDir {
        tempfile::tempdir().expect("tempdir")
    }

    #[test]
    fn run_writes_latest_and_status_reads_it() {
        let root = runtime_temp_root();
        let exit = run(
            root.path(),
            &[
                "run".to_string(),
                "--system-id=systems-memory-causal_temporal_graph".to_string(),
                "--apply=1".to_string(),
                "--payload-json={\"k\":1}".to_string(),
            ],
        );
        assert_eq!(exit, 0);

        let latest = latest_path(root.path(), "systems-memory-causal_temporal_graph");
        assert!(latest.exists());

        let status = status_payload(
            root.path(),
            "systems-memory-causal_temporal_graph",
            "status",
        );
        assert_eq!(
            status.get("has_state").and_then(Value::as_bool),
            Some(true),
            "status should reflect latest state"
        );
    }

    #[test]
    fn verify_is_read_only_and_does_not_write_state() {
        let root = runtime_temp_root();
        let exit = run(
            root.path(),
            &[
                "verify".to_string(),
                "--system-id=systems-autonomy-gated_self_improvement_loop".to_string(),
            ],
        );
        assert_eq!(exit, 0);
        let latest = latest_path(root.path(), "systems-autonomy-gated_self_improvement_loop");
        assert!(!latest.exists());
    }

    #[test]
    fn assimilation_lane_emits_protocol_summary_and_artifacts() {
        let root = runtime_temp_root();
        let out = run_payload(
            root.path(),
            "SYSTEMS-ASSIMILATION-SOURCE_ATTESTATION_EXTENSION",
            "attest",
            &[
                "--strict=1".to_string(),
                "--apply=1".to_string(),
                "--payload-json={\"source\":\"repo\",\"phase\":\"attestation\"}".to_string(),
            ],
        )
        .expect("assimilation lane run should succeed");
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            out.get("contract_execution")
                .and_then(Value::as_object)
                .and_then(|row| row.get("protocol_version"))
                .and_then(Value::as_str),
            Some("infring_assimilation_protocol_v1")
        );
        let contract = out
            .get("contract_execution")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        for key in [
            "IntentSpec",
            "ReconIndex",
            "CandidateSet",
            "CandidateClosure",
            "ProvisionalGapReport",
            "AdmissionVerdict",
            "AdmittedAssimilationPlan",
            "ProtocolStepReceipt",
        ] {
            assert!(
                contract.get(key).is_some(),
                "missing canonical assimilation protocol stage key: {key}"
            );
        }
        let artifacts = out
            .get("artifacts")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(
            !artifacts.is_empty(),
            "assimilation protocol should emit state/history artifacts"
        );
        let state_path = artifacts[0].as_str().unwrap_or_default().to_string();
        assert!(
            root.path().join(state_path).exists(),
            "assimilation state artifact should exist"
        );
    }

    #[test]
    fn assimilation_lane_hard_selector_cannot_bypass_closure() {
        let root = runtime_temp_root();
        let err = run_payload(
            root.path(),
            "SYSTEMS-ASSIMILATION-WORLD_MODEL_FRESHNESS",
            "freshness",
            &[
                "--strict=1".to_string(),
                "--apply=1".to_string(),
                "--hard-selector=nonexistent-surface".to_string(),
            ],
        )
        .expect_err("hard selector mismatch should fail closed under strict mode");
        assert!(
            err.contains("assimilation_hard_selector_closure_reject"),
            "expected hard selector closure rejection, got {err}"
        );
    }

    #[test]
    fn assimilation_lane_selector_bypass_rejected_under_strict_mode() {
        let root = runtime_temp_root();
        let err = run_payload(
            root.path(),
            "SYSTEMS-ASSIMILATION-SOURCE_ATTESTATION_EXTENSION",
            "attest",
            &[
                "--strict=1".to_string(),
                "--apply=1".to_string(),
                "--selector-bypass=1".to_string(),
            ],
        )
        .expect_err("selector bypass should be blocked under strict mode");
        assert!(
            err.contains("assimilation_selector_bypass_rejected"),
            "expected selector bypass rejection, got {err}"
        );
    }

    #[test]
    fn assimilation_lane_strict_rejects_unknown_operation() {
        let root = runtime_temp_root();
        let err = run_payload(
            root.path(),
            "SYSTEMS-ASSIMILATION-TRAJECTORY_SKILL_DISTILLER",
            "calibrate",
            &["--strict=1".to_string()],
        )
        .expect_err("unsupported strict operation should fail");
        assert!(
            err.contains("assimilation_protocol_op_not_allowed"),
            "expected assimilation protocol op gate error, got {err}"
        );
    }

