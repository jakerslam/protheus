

fn cmd_report(root: &Path, cli: &CliArgs, policy: &Policy, paths: &RuntimePaths) -> Value {
    let token = cli
        .positional
        .get(1)
        .map(|s| s.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "latest".to_string());
    let write = to_bool(cli.flags.get("write").map(String::as_str), true);
    let mut status = load_status(paths);

    let latest_run = read_json(&paths.latest_path);
    let ts = now_iso();
    let date = if token == "latest" {
        ts[..10].to_string()
    } else {
        token
    };

    let modules = status.modules.values().cloned().collect::<Vec<_>>();
    let tests = status.tests.values().cloned().collect::<Vec<_>>();
    let external_health = summarize_external_health(paths, policy);

    let mut untested = modules
        .iter()
        .filter(|m| m.untested)
        .cloned()
        .collect::<Vec<_>>();
    untested.sort_by(|a, b| a.path.cmp(&b.path));
    untested.truncate(policy.alerts.max_untested_in_report);

    let mut red_modules = modules
        .iter()
        .filter(|m| m.health_state.as_deref() == Some("red"))
        .cloned()
        .collect::<Vec<_>>();
    red_modules.sort_by(|a, b| a.path.cmp(&b.path));
    red_modules.truncate(policy.alerts.max_untested_in_report);

    let mut failed_tests = tests
        .iter()
        .filter(|t| t.last_status == "fail")
        .cloned()
        .collect::<Vec<_>>();
    failed_tests.sort_by(|a, b| a.command.cmp(&b.command));
    failed_tests.truncate(policy.alerts.max_failed_in_report);

    let checked_modules = modules.iter().filter(|m| m.checked).count();
    let green_modules = modules
        .iter()
        .filter(|m| m.health_state.as_deref() == Some("green"))
        .count();
    let changed_modules = modules.iter().filter(|m| m.changed).count();

    let mut lines = vec![
        "# Autotest Report".to_string(),
        "".to_string(),
        format!("- Generated: {ts}"),
        format!("- Date: {date}"),
        format!("- Modules: {}", modules.len()),
        format!("- Checked: {checked_modules}"),
        format!("- Green Modules: {green_modules}"),
        format!("- Red Modules: {}", red_modules.len()),
        format!("- Changed/Pending: {changed_modules}"),
        format!("- Untested Modules: {}", untested.len()),
        format!("- Failed Tests: {}", failed_tests.len()),
    ];

    if latest_run.is_object() {
        lines.push(format!(
            "- Last Run Scope: {}",
            latest_run
                .get("scope")
                .and_then(Value::as_str)
                .unwrap_or("n/a")
        ));
        lines.push(format!(
            "- Last Run Passed/Failed: {}/{}",
            latest_run
                .get("passed")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            latest_run
                .get("failed")
                .and_then(Value::as_i64)
                .unwrap_or(0)
        ));
    }

    lines.extend(["".to_string(), "## Red Modules (Need Help)".to_string()]);
    if red_modules.is_empty() {
        lines.push("- None".to_string());
    } else {
        for module in &red_modules {
            lines.push(format!("- {}", module.path));
            lines.push(format!(
                "  - reason: {}",
                module
                    .health_reason
                    .clone()
                    .unwrap_or_else(|| "failing_or_guard_blocked_test".to_string())
            ));
        }
    }

    lines.extend(["".to_string(), "## Failed Tests".to_string()]);
    if failed_tests.is_empty() {
        lines.push("- None".to_string());
    } else {
        for test in &failed_tests {
            let label = test.path.clone().unwrap_or_else(|| test.command.clone());
            lines.push(format!("- {label}"));
            if let Some(stderr) = &test.last_stderr_excerpt {
                lines.push(format!("  - stderr: {stderr}"));
            }
        }
    }

    lines.extend([
        "".to_string(),
        "## External Health Signals".to_string(),
        format!(
            "- Total Signals: {}",
            external_health
                .get("total")
                .and_then(Value::as_u64)
                .unwrap_or(0)
        ),
        format!(
            "- High/Critical: {}",
            external_health
                .get("high_or_critical")
                .and_then(Value::as_u64)
                .unwrap_or(0)
        ),
    ]);

    lines.extend(["".to_string(), "## Untested Modules".to_string()]);
    if untested.is_empty() {
        lines.push("- None".to_string());
    } else {
        for module in &untested {
            lines.push(format!("- {}", module.path));
            if module.changed {
                lines.push("  - reason: changed module with no mapped tests".to_string());
            } else if module.is_new {
                lines.push("  - reason: new module with no mapped tests".to_string());
            } else {
                lines.push("  - reason: no mapped tests".to_string());
            }
        }
    }

    let markdown = format!("{}\n", lines.join("\n"));
    let out_path = paths.reports_dir.join(format!("{date}.md"));
    if write {
        let _ = ensure_dir(&paths.reports_dir);
        let _ = fs::write(&out_path, markdown);
    }

    let mut out = json!({
        "ok": true,
        "type": "autotest_report",
        "ts": ts,
        "date": date,
        "modules_total": modules.len(),
        "modules_checked": checked_modules,
        "modules_green": green_modules,
        "modules_red": red_modules.len(),
        "modules_changed": changed_modules,
        "untested_modules": untested.len(),
        "failed_tests": failed_tests.len(),
        "external_health": external_health,
        "output_path": if write { Value::String(rel_path(root, &out_path)) } else { Value::Null },
        "write": write,
        "claim_evidence": [
            {
                "id": "report_composition",
                "claim": "report_counts_match_status_snapshot",
                "evidence": {
                    "modules_total": modules.len(),
                    "tests_total": tests.len(),
                    "failed_tests": failed_tests.len()
                }
            }
        ],
        "persona_lenses": {
            "operator": {
                "risk_focus": if failed_tests.is_empty() { "coverage" } else { "stability" }
            }
        }
    });
    out["receipt_hash"] = Value::String(receipt_hash(&out));

    status.last_report = Some(now_iso());
    let _ = write_json_atomic(
        &paths.status_path,
        &serde_json::to_value(status).unwrap_or(Value::Null),
    );
    let _ = write_json_atomic(&paths.latest_path, &out);
    let _ = append_jsonl(
        &paths.runs_dir.join(format!("{}.jsonl", &now_iso()[..10])),
        &out,
    );

    out
}
