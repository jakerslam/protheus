    let persona_lenses = json!({
        "operator": {
            "attention": if failed > 0 { "incident" } else { "maintenance" },
            "guard_blocked": guard_blocked
        },
        "skeptic": {
            "confidence": if failed == 0 { 0.92 } else { 0.58 },
            "flaky_tests": flaky_count,
            "newly_quarantined_tests": quarantined_count
        }
    });

    let mut out = json!({
        "ok": if strict { failed == 0 && untested == 0 } else { failed == 0 },
        "type": "autotest_run",
        "ts": run_ts,
        "scope": scope,
        "strict": strict,
        "synced": sync_out,
        "selected_tests": results.len(),
        "queued_candidates": prioritized.len(),
        "selection_preview": selection_preview,
        "passed": passed,
        "failed": failed,
        "guard_blocked": guard_blocked,
        "flaky_tests": flaky_count,
        "newly_quarantined_tests": quarantined_count,
        "untested_modules": untested,
        "external_health": external_health,
        "sleep_window_ok": sleep_gate,
        "resource_guard": resources,
        "spine_hot": spine_hot,
        "run_timeout_ms": run_timeout_ms,
        "phase_ms": phase_ms,
        "results": results.iter().take(300).cloned().collect::<Vec<_>>(),
        "claim_evidence": claim_evidence,
        "persona_lenses": persona_lenses,
        "pain_signal": Value::Null
    });
    out["receipt_hash"] = Value::String(receipt_hash(&out));

    let _ = write_json_atomic(&paths.latest_path, &out);
    let _ = append_jsonl(
        &paths.runs_dir.join(format!("{}.jsonl", &run_ts[..10])),
        &out,
    );

    if failed > 0 || untested > 0 || guard_blocked > 0 || flaky_count > 0 {
        let _ = append_jsonl(
            &paths.events_path,
            &json!({
                "ts": run_ts,
                "type": "autotest_alert",
                "severity": if failed > 0 || guard_blocked > 0 { "error" } else { "warn" },
                "alert_kind": if guard_blocked > 0 {
                    "guard_blocked"
                } else if failed > 0 {
                    "test_failures"
                } else if flaky_count > 0 {
                    "flaky_tests"
                } else {
                    "untested_modules"
                },
                "failed": failed,
                "guard_blocked": guard_blocked,
                "flaky_tests": flaky_count,
                "untested_modules": untested,
                "scope": scope
            }),
        );
    }

    out
}
