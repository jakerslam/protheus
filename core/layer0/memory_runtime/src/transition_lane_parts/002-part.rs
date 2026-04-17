mod tests {
    use super::{evaluate_auto_selector, load_policy, policy_scope_id, write_transition_receipt};
    use crate::lane_contracts::ClaimEvidenceRow;
    use serde_json::json;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_dir(prefix: &str) -> PathBuf {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|dur| dur.as_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!("{prefix}-{now}"));
        let _ = fs::create_dir_all(&dir);
        dir
    }

    #[test]
    fn evaluate_auto_selector_matches_threshold_gate() {
        let root = unique_temp_dir("transition-lane-auto-selector");
        let policy_path = root.join("policy.json");
        fs::create_dir_all(root.join("local/state/client/memory/rust_transition"))
            .expect("mkdir state");
        fs::write(&policy_path, "{}").expect("write policy");
        let policy = load_policy(&root, &policy_path);
        let scope_id = policy_scope_id(&policy);
        let history_path =
            root.join("local/state/client/memory/rust_transition/benchmark_history.json");
        let rows = (0..12)
            .map(|idx| {
                json!({
                    "policy_scope": scope_id,
                    "speedup": if idx < 10 { 1.5 } else { 1.3 },
                    "parity_error_count": 0
                })
            })
            .collect::<Vec<_>>();
        fs::write(
            &history_path,
            serde_json::to_string_pretty(&json!({ "rows": rows })).expect("encode"),
        )
        .expect("write history");

        let decision = evaluate_auto_selector(&policy);
        assert!(decision.eligible);
        assert_eq!(decision.backend, "rust_shadow");
        assert_eq!(decision.active_engine, "rust");
    }

    #[test]
    fn transition_receipt_write_is_deterministic_and_claim_bounded() {
        let root = unique_temp_dir("transition-lane-receipt");
        let policy_path = root.join("policy.json");
        fs::write(&policy_path, "{}").expect("write policy");
        let policy = load_policy(&root, &policy_path);
        let claims = vec![ClaimEvidenceRow {
            claim: "auto selector is benchmark-threshold gated".to_string(),
            evidence: vec![
                "path:local/state/client/memory/rust_transition/benchmark_history.json".to_string(),
                "stable_runs:10".to_string(),
            ],
            persona_lenses: vec![
                "migration_guard".to_string(),
                "performance_governor".to_string(),
            ],
        }];
        let payload_a = json!({
            "ts": "2026-03-05T00:00:00Z",
            "type": "rust_memory_auto_selector",
            "ok": true,
            "stable_runs": 10,
            "avg_speedup": 1.4
        });
        let payload_b = json!({
            "avg_speedup": 1.4,
            "stable_runs": 10,
            "ok": true,
            "type": "rust_memory_auto_selector",
            "ts": "2026-03-05T00:00:00Z"
        });

        write_transition_receipt(&policy, &payload_a, &claims);
        let first =
            fs::read_to_string(root.join("local/state/client/memory/rust_transition/latest.json"))
                .expect("read first");
        write_transition_receipt(&policy, &payload_b, &claims);
        let second =
            fs::read_to_string(root.join("local/state/client/memory/rust_transition/latest.json"))
                .expect("read second");

        let first_hash = serde_json::from_str::<serde_json::Value>(&first)
            .expect("decode first")
            .get("receipt_hash")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let second_hash = serde_json::from_str::<serde_json::Value>(&second)
            .expect("decode second")
            .get("receipt_hash")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        assert_eq!(first_hash, second_hash);
    }
}
