
#[cfg(test)]
mod tests {
    use super::*;

    fn has_claim(value: &Value, claim_id: &str) -> bool {
        value
            .get("claim_evidence")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .iter()
            .any(|row| row.get("id").and_then(Value::as_str) == Some(claim_id))
    }

    #[test]
    fn join_then_compute_proof_updates_reputation() {
        let dir = tempfile::tempdir().expect("tempdir");
        assert_eq!(
            run(
                dir.path(),
                &[
                    "join".to_string(),
                    "--profile=hyperspace".to_string(),
                    "--node=n1".to_string(),
                ]
            ),
            0
        );
        assert_eq!(
            run(
                dir.path(),
                &[
                    "compute-proof".to_string(),
                    "--node=n1".to_string(),
                    "--credits=2.5".to_string(),
                ]
            ),
            0
        );
        let rep = reputations(dir.path());
        let score = rep.get("n1").and_then(Value::as_f64).unwrap_or(0.0);
        assert!(score >= 3.5);
        let contributions = contributions(dir.path());
        assert_eq!(
            contributions
                .get("n1")
                .and_then(|v| v.get("proof_count"))
                .and_then(Value::as_u64),
            Some(1)
        );

        let latest = read_json(&latest_path(dir.path())).expect("latest receipt");
        assert!(has_claim(&latest, "V6-NETWORK-004.1"));
        assert!(has_claim(&latest, "V6-NETWORK-004.2"));
        assert!(has_claim(&latest, "V6-NETWORK-004.6"));
        assert!(latest
            .pointer("/proof/challenge_id")
            .and_then(Value::as_str)
            .map(|v| !v.is_empty())
            .unwrap_or(false));
    }

    #[test]
    fn dashboard_reports_current_reputation_state() {
        let dir = tempfile::tempdir().expect("tempdir");
        let _ = run(
            dir.path(),
            &[
                "compute-proof".to_string(),
                "--node=n2".to_string(),
                "--credits=1.0".to_string(),
            ],
        );
        let receipt = dashboard_receipt(dir.path());
        assert_eq!(
            receipt.get("type").and_then(Value::as_str),
            Some("p2p_gossip_seed_dashboard")
        );
        assert_eq!(receipt.get("node_count").and_then(Value::as_u64), Some(1));
        assert!(has_claim(&receipt, "V6-NETWORK-004.2"));
        assert!(has_claim(&receipt, "V6-NETWORK-004.6"));
        assert!(receipt
            .get("event_totals")
            .and_then(Value::as_object)
            .is_some());
    }

    #[test]
    fn strict_conduit_mode_rejects_bypass() {
        let dir = tempfile::tempdir().expect("tempdir");
        let exit = run(
            dir.path(),
            &[
                "compute-proof".to_string(),
                "--strict=1".to_string(),
                "--bypass=1".to_string(),
            ],
        );
        assert_eq!(exit, 1);
    }

    #[test]
    fn strict_mode_rejects_invalid_matmul_size() {
        let dir = tempfile::tempdir().expect("tempdir");
        let exit = run(
            dir.path(),
            &[
                "compute-proof".to_string(),
                "--strict=1".to_string(),
                "--matmul-size=100".to_string(),
            ],
        );
        assert_eq!(exit, 2);
        let latest = read_json(&latest_path(dir.path())).expect("latest");
        assert_eq!(
            latest.get("type").and_then(Value::as_str),
            Some("p2p_gossip_seed_compute_proof_error")
        );
    }
}
