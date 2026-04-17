        let _ = run_enable(
            root.path(),
            &crate::parse_args(&["enable-neuralavb".to_string(), "--enabled=1".to_string()]),
            true,
        );
        let _ = run_experiment(
            root.path(),
            &crate::parse_args(&["experiment-loop".to_string(), "--iterations=2".to_string()]),
            true,
        );
        let out = run_benchmark(
            root.path(),
            &crate::parse_args(&["benchmark".to_string()]),
            true,
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert!(benchmark_latest_path(root.path()).exists());
    }

    #[test]
    fn dashboard_surfaces_benchmark_tradeoff_metrics() {
        let root = tempfile::tempdir().expect("tempdir");
        let _ = run_enable(
            root.path(),
            &crate::parse_args(&["enable-neuralavb".to_string(), "--enabled=1".to_string()]),
            true,
        );
        let _ = run_experiment(
            root.path(),
            &crate::parse_args(&["experiment-loop".to_string(), "--iterations=2".to_string()]),
            true,
        );
        let _ = run_benchmark(
            root.path(),
            &crate::parse_args(&["benchmark".to_string()]),
            true,
        );
        let out = run_dashboard(root.path(), true);
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        let has_claim = out
            .get("claim_evidence")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .iter()
            .any(|row| row.get("id").and_then(Value::as_str) == Some("V6-EVAL-001.5"));
        assert!(has_claim);
    }

    #[test]
    fn conduit_rejects_bypass() {
        let root = tempfile::tempdir().expect("tempdir");
        let gate = conduit_enforcement(
            root.path(),
            &crate::parse_args(&[
                "run".to_string(),
                "--strict=1".to_string(),
                "--bypass=1".to_string(),
            ]),
            true,
            "run",
        );
        assert_eq!(gate.get("ok").and_then(Value::as_bool), Some(false));
    }
}
