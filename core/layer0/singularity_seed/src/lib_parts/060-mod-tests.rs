
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        atomic::{AtomicU64, Ordering},
        Mutex,
    };

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn temp_blob_root() -> PathBuf {
        let counter = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "infring-singularity-seed-test-{}-{}",
            std::process::id(),
            counter
        ));
        if dir.exists() {
            let _ = std::fs::remove_dir_all(&dir);
        }
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    #[test]
    fn cycle_runs_and_advances_generation() {
        let _lock = ENV_LOCK.lock().expect("lock env");
        let root = temp_blob_root();
        std::env::set_var("INFRING_SINGULARITY_BLOB_DIR", root.display().to_string());

        let report = run_guarded_cycle(&CycleRequest::default()).expect("cycle should run");
        assert!(report.ok);
        assert!(!report.fail_closed);
        assert_eq!(report.outcomes.len(), 4);
        assert!(report.outcomes.iter().all(|row| row.next_generation >= 2));

        std::env::remove_var("INFRING_SINGULARITY_BLOB_DIR");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn cycle_fail_closed_when_drift_exceeds_threshold() {
        let _lock = ENV_LOCK.lock().expect("lock env");
        let root = temp_blob_root();
        std::env::set_var("INFRING_SINGULARITY_BLOB_DIR", root.display().to_string());

        let request = CycleRequest {
            drift_overrides: vec![DriftOverride {
                loop_id: RED_LEGION_LOOP_ID.to_string(),
                drift_pct: 2.4,
            }],
        };
        let report = run_guarded_cycle(&request).expect("cycle should run");
        assert!(!report.ok);
        assert!(report.fail_closed);
        assert!(report.max_drift_pct > DRIFT_FAIL_CLOSED_THRESHOLD_PCT);

        std::env::remove_var("INFRING_SINGULARITY_BLOB_DIR");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn cycle_request_normalization_filters_unknown_or_invalid_overrides() {
        let request = CycleRequest {
            drift_overrides: vec![
                DriftOverride {
                    loop_id: " RED-LEGION LOOP ".to_string(),
                    drift_pct: 2.4,
                },
                DriftOverride {
                    loop_id: "unknown-loop".to_string(),
                    drift_pct: 1.2,
                },
                DriftOverride {
                    loop_id: DUAL_BRAIN_LOOP_ID.to_string(),
                    drift_pct: f64::NAN,
                },
            ],
        };
        let normalized = normalize_cycle_request(request);
        assert_eq!(normalized.drift_overrides.len(), 1);
        assert_eq!(normalized.drift_overrides[0].loop_id, RED_LEGION_LOOP_ID);
        assert_eq!(normalized.drift_overrides[0].drift_pct, 2.4);
    }

    #[test]
    fn strict_cycle_request_contract_rejects_unknown_override() {
        let request = CycleRequest {
            drift_overrides: vec![DriftOverride {
                loop_id: "unknown-loop".to_string(),
                drift_pct: 1.0,
            }],
        };
        let out = normalize_cycle_request_with_contract(request, true);
        assert!(matches!(out, Err(SeedError::InvalidRequest(_))));
    }

    #[test]
    fn blob_root_override_sanitizer_blocks_parent_traversal() {
        assert!(sanitize_blob_root_override("../bad/path").is_none());
        assert!(sanitize_blob_root_override("/tmp/singularity-seed").is_some());
    }
}
