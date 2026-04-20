
pub fn run(root: &Path, argv: &[String]) -> i32 {
    let parsed = parse_args(argv);
    let action = parsed
        .positional
        .first()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());
    if matches!(action.as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }
    let strict = parse_bool(parsed.flags.get("strict"), true);
    let conduit = if action != "status" {
        Some(conduit_enforcement(root, &parsed, strict, action.as_str()))
    } else {
        None
    };
    if strict
        && conduit
            .as_ref()
            .and_then(|v| v.get("ok"))
            .and_then(Value::as_bool)
            == Some(false)
    {
        return emit(
            root,
            json!({
                "ok": false,
                "strict": strict,
                "type": "eval_plane_conduit_gate",
                "errors": ["conduit_bypass_rejected"],
                "conduit_enforcement": conduit
            }),
        );
    }
    let payload = dispatch(root, &parsed, strict);
    if action == "status" {
        print_json(&payload);
        return 0;
    }
    emit(root, attach_conduit(payload, conduit.as_ref()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn experiment_persists_fixture_trace_and_rewards() {
        let root = tempfile::tempdir().expect("tempdir");
        let _ = run_enable(
            root.path(),
            &crate::parse_args(&["enable-neuralavb".to_string(), "--enabled=1".to_string()]),
            true,
        );
        let out = run_experiment(
            root.path(),
            &crate::parse_args(&[
                "experiment-loop".to_string(),
                "--iterations=3".to_string(),
                "--baseline-cost-usd=24".to_string(),
                "--run-cost-usd=8".to_string(),
                "--baseline-accuracy=0.92".to_string(),
                "--run-accuracy=0.91".to_string(),
            ]),
            true,
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert!(fixture_path(root.path()).exists());
        assert!(loop_latest_path(root.path()).exists());
        assert!(trace_history_path(root.path()).exists());
    }

    #[test]
    fn benchmark_emits_cost_accuracy_deltas() {
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

