
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn stomach_run_and_status_roundtrip() {
        let root = tempdir().expect("tmp");
        let source = root.path().join("import");
        fs::create_dir_all(&source).expect("mkdir");
        fs::write(
            source.join("Cargo.toml"),
            "[package]\nname=\"x\"\nversion=\"0.1.0\"\n",
        )
        .expect("write");
        fs::write(source.join("LICENSE"), "MIT").expect("license");
        let run_exit = run(
            root.path(),
            &[
                "run".to_string(),
                "--id=demo".to_string(),
                format!("--source-root={}", source.display()),
                "--origin=https://github.com/acme/repo".to_string(),
                "--commit=abc".to_string(),
                "--spdx=MIT".to_string(),
            ],
        );
        assert_eq!(run_exit, 0);
        let status_exit = run(
            root.path(),
            &["status".to_string(), "--id=demo".to_string()],
        );
        assert_eq!(status_exit, 0);
    }

    #[test]
    fn purge_requires_retention_eligibility_and_explicit_approval() {
        let root = tempdir().expect("tmp");
        let source = root.path().join("import");
        fs::create_dir_all(&source).expect("mkdir");
        fs::write(
            source.join("Cargo.toml"),
            "[package]\nname=\"x\"\nversion=\"0.1.0\"\n",
        )
        .expect("write");
        fs::write(source.join("LICENSE"), "MIT").expect("license");

        let run_exit = run(
            root.path(),
            &[
                "run".to_string(),
                "--id=purge-demo".to_string(),
                format!("--source-root={}", source.display()),
                "--origin=https://github.com/acme/repo".to_string(),
                "--commit=abc".to_string(),
                "--spdx=MIT".to_string(),
            ],
        );
        assert_eq!(run_exit, 0);
        let purge_blocked = run(
            root.path(),
            &["purge".to_string(), "--id=purge-demo".to_string()],
        );
        assert_eq!(purge_blocked, 1);

        let now_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let retention_ok = run(
            root.path(),
            &[
                "retention".to_string(),
                "--id=purge-demo".to_string(),
                "--action=eligible".to_string(),
                format!("--retained-until={}", now_secs.saturating_sub(1)),
                "--approve-receipt=receipt:purge-demo:approve".to_string(),
            ],
        );
        assert_eq!(retention_ok, 0);
        let purge_ok = run(
            root.path(),
            &["purge".to_string(), "--id=purge-demo".to_string()],
        );
        assert_eq!(purge_ok, 0);
    }

    #[test]
    fn nexus_authorization_succeeds_for_stomach_route() {
        let out = authorize_stomach_command_with_nexus_inner("status", false).expect("nexus auth");
        assert_eq!(out.get("enabled").and_then(Value::as_bool), Some(true));
        assert!(out
            .get("lease_id")
            .and_then(Value::as_str)
            .map(|row| !row.is_empty())
            .unwrap_or(false));
    }

    #[test]
    fn nexus_authorization_fails_closed_when_blocked_pair_enabled() {
        let err = authorize_stomach_command_with_nexus_inner("status", true)
            .err()
            .unwrap_or_else(|| "missing_error".to_string());
        assert!(err.contains("lease_denied") || err.contains("delivery_denied"));
    }

    #[test]
    fn run_writes_mandatory_scoring_gate_ledger_and_report() {
        let root = tempdir().expect("tmp");
        let source = root.path().join("import");
        fs::create_dir_all(source.join("core")).expect("mkdir");
        fs::write(
            source.join("core").join("mod.rs"),
            "pub fn hello() -> &'static str { \"world\" }\n",
        )
        .expect("write source");
        fs::write(
            source.join("Cargo.toml"),
            "[package]\nname=\"x\"\nversion=\"0.1.0\"\n",
        )
        .expect("write cargo");
        fs::write(source.join("LICENSE"), "MIT").expect("license");

        let run_exit = run(
            root.path(),
            &[
                "run".to_string(),
                "--id=score-demo".to_string(),
                format!("--source-root={}", source.display()),
                "--origin=https://github.com/acme/repo".to_string(),
                "--commit=abc".to_string(),
                "--spdx=MIT".to_string(),
            ],
        );
        assert_eq!(run_exit, 0);

        let ledger_path = root
            .path()
            .join("local/state/stomach/ledgers/score-demo_file_scores.json");
        assert!(ledger_path.exists(), "expected scoring ledger to exist");
        let ledger: Value =
            serde_json::from_str(&fs::read_to_string(&ledger_path).expect("read ledger"))
                .expect("decode ledger");
        assert_eq!(
            ledger
                .get("mandatory_scoring_gate")
                .and_then(Value::as_bool),
            Some(true)
        );
        let rows = ledger
            .get("rows")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(!rows.is_empty(), "expected scored rows");
        for row in &rows {
            assert!(row.get("authority_risk_score").is_some());
            assert!(row.get("migration_potential_score").is_some());
            assert!(row.get("concept_opportunity_score").is_some());
            assert!(row.get("priority_score").is_some());
            assert_eq!(row.get("state").and_then(Value::as_str), Some("done"));
            let history = row
                .get("state_history")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            let states = history
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>();
            assert_eq!(states, vec!["queued", "in_progress", "done"]);
        }
        let report_glob_root = root.path().join("local/workspace/reports");
        assert!(report_glob_root.exists(), "expected report root to exist");
    }
}
