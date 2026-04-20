
fn test_set_for_scope(status: &StatusState, scope: &str) -> HashSet<String> {
    let mut selected = HashSet::new();
    match scope {
        "all" => {
            selected.extend(status.tests.keys().cloned());
        }
        "critical" => {
            for test in status.tests.values() {
                if test.critical {
                    selected.insert(test.id.clone());
                }
            }
        }
        _ => {
            for module in status.modules.values() {
                if module.changed {
                    for id in &module.mapped_test_ids {
                        selected.insert(id.clone());
                    }
                }
            }
            for test in status.tests.values() {
                if test.critical {
                    selected.insert(test.id.clone());
                }
            }
        }
    }
    selected
}

fn module_stale_ms(module: &ModuleRow, now_ms: i64) -> i64 {
    let last_test_ms = module
        .last_test_ts
        .as_deref()
        .and_then(|v| chrono::DateTime::parse_from_rfc3339(v).ok())
        .map(|v| v.timestamp_millis())
        .unwrap_or(0);
    (now_ms - last_test_ms).max(0)
}

fn prioritize_tests(status: &StatusState, test_ids: &HashSet<String>) -> Vec<PrioritizedTest> {
    let reverse = reverse_module_mapping(status);
    let now_ms = chrono::Utc::now().timestamp_millis();

    let mut out = Vec::new();
    for id in test_ids {
        let Some(test) = status.tests.get(id).cloned() else {
            continue;
        };
        let mapped_modules = reverse.get(id).cloned().unwrap_or_default();
        let mut score = 0i64;
        let mut priority = "normal".to_string();

        if test.critical {
            score += 100;
            priority = "critical".to_string();
        }
        if test.last_status == "fail" {
            score += 40;
            priority = "high".to_string();
        }
        if test.last_status == "untested" {
            score += 30;
        }

        let mut changed_count = 0i64;
        let mut stale_score = 0i64;
        for module_path in mapped_modules {
            if let Some(module) = status.modules.get(&module_path) {
                if module.changed {
                    changed_count += 1;
                }
                stale_score += (module_stale_ms(module, now_ms) / 1000).min(300);
            }
        }
        score += changed_count * 20;
        score += stale_score.min(120);

        out.push(PrioritizedTest {
            id: id.clone(),
            score,
            priority,
            test,
        });
    }

    out.sort_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then_with(|| a.id.cmp(&b.id))
            .then_with(|| a.test.command.cmp(&b.test.command))
    });
    out
}

fn summarize_external_health(paths: &RuntimePaths, policy: &Policy) -> Value {
    let mut sources = Vec::<PathBuf>::new();
    if !policy.external_health_paths.is_empty() {
        for raw in &policy.external_health_paths {
            let p = PathBuf::from(raw);
            sources.push(if p.is_absolute() {
                p
            } else {
                paths
                    .state_dir
                    .parent()
                    .unwrap_or(paths.state_dir.as_path())
                    .join(p)
            });
        }
    } else {
        sources.push(paths.pain_signals_path.clone());
    }

    let since_ms = chrono::Utc::now().timestamp_millis()
        - (policy.external_health_window_hours * 60 * 60 * 1000);

    let mut total = 0usize;
    let mut high_or_critical = 0usize;
    let mut latest_ts = None::<String>;

    for src in &sources {
        for row in read_jsonl(src) {
            let ts = row
                .get("ts")
                .or_else(|| row.get("timestamp"))
                .and_then(Value::as_str)
                .unwrap_or_default();
            let ts_ms = chrono::DateTime::parse_from_rfc3339(ts)
                .ok()
                .map(|v| v.timestamp_millis())
                .unwrap_or(0);
            if ts_ms < since_ms {
                continue;
            }
            total += 1;
            let sev = row
                .get("severity")
                .and_then(Value::as_str)
                .unwrap_or("medium")
                .to_ascii_lowercase();
            if sev == "high" || sev == "critical" {
                high_or_critical += 1;
            }
            latest_ts = Some(ts.to_string());
        }
    }

    let available = total > 0;
    json!({
        "enabled": true,
        "available": available,
        "window_hours": policy.external_health_window_hours,
        "total": total,
        "high_or_critical": high_or_critical,
        "latest_ts": latest_ts,
        "path": sources
            .first()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| paths.pain_signals_path.to_string_lossy().to_string())
    })
}
