
fn collect_change_fail_rate(
    history_path: &Path,
    window_start: NaiveDate,
    now: NaiveDate,
) -> (f64, usize, usize) {
    let rows = read_jsonl(history_path);
    let mut total = 0usize;
    let mut failed = 0usize;

    for row in rows {
        let Some(day) = parse_iso_day(row.get("ts").or_else(|| row.get("date"))) else {
            continue;
        };
        if day < window_start || day > now {
            continue;
        }
        total += 1;
        let ok = row.get("ok").and_then(Value::as_bool).unwrap_or(false);
        let blocked = row
            .get("gate")
            .and_then(|v| v.get("promotion_blocked"))
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if !ok || blocked {
            failed += 1;
        }
    }

    let rate = if total > 0 {
        failed as f64 / total as f64
    } else {
        0.0
    };
    (rate, total, failed)
}

fn evidence_status(paths: &[PathBuf], min_count: usize) -> Value {
    let mut found = Vec::<String>::new();
    let mut missing = Vec::<String>::new();
    for path in paths {
        if path.exists() {
            found.push(path.to_string_lossy().to_string());
        } else {
            missing.push(path.to_string_lossy().to_string());
        }
    }
    let ok = found.len() >= min_count;
    json!({
        "ok": ok,
        "required_min": min_count,
        "found_count": found.len(),
        "found_paths": found,
        "missing_paths": missing
    })
}
