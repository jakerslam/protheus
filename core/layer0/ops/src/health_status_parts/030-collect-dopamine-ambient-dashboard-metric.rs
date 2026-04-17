fn collect_dopamine_ambient_dashboard_metric(root: &Path) -> Value {
    let now_ms = Utc::now().timestamp_millis();
    let primary_path = root.join("client/runtime/local/state/dopamine/ambient/latest.json");
    let legacy_path = root.join("local/state/client-runtime/state/dopamine_state.json");

    let primary = read_json(&primary_path).ok();
    let legacy = read_json(&legacy_path).ok();

    let mut source = "missing".to_string();
    let mut score = 0.0f64;
    let mut status = "warn".to_string();
    let mut freshness = "missing".to_string();
    let mut age_sec = None::<i64>;
    let mut severity = "unknown".to_string();
    let mut threshold_breached = false;
    let mut last_recorded_date = String::new();
    let runtime_web_tooling_auth_sources = runtime_web_tooling_auth_sources();
    let runtime_web_tooling_auth_present = !runtime_web_tooling_auth_sources.is_empty();
    let runtime_web_tooling_strict_auth_required = runtime_web_tooling_strict_auth_required(root);

    if let Some(row) = primary.as_ref() {
        source = primary_path.to_string_lossy().to_string();
        score = row
            .get("sds")
            .and_then(Value::as_f64)
            .or_else(|| {
                row.get("summary")
                    .and_then(Value::as_object)
                    .and_then(|s| s.get("sds"))
                    .and_then(Value::as_f64)
            })
            .unwrap_or(0.0);
        severity = row
            .get("severity")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_ascii_lowercase();
        threshold_breached = row
            .get("threshold_breached")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let ts_ms = row.get("ts").and_then(Value::as_str).and_then(parse_ts_ms);
        age_sec = age_seconds(now_ms, ts_ms);
        let stale = age_sec
            .map(|age| age > DOPAMINE_METRICS_FRESH_WINDOW_SECONDS)
            .unwrap_or(true);
        freshness = if stale { "stale" } else { "fresh" }.to_string();
        status = if stale
            || threshold_breached
            || severity == "critical"
            || severity == "warn"
            || (runtime_web_tooling_strict_auth_required && !runtime_web_tooling_auth_present)
        {
            "warn".to_string()
        } else {
            "pass".to_string()
        };
    } else if let Some(row) = legacy.as_ref() {
        source = legacy_path.to_string_lossy().to_string();
        score = row
            .get("last_score")
            .and_then(Value::as_f64)
            .or_else(|| {
                row.get("last_score")
                    .and_then(Value::as_i64)
                    .map(|v| v as f64)
            })
            .unwrap_or(0.0);
        last_recorded_date = row
            .get("last_recorded_date")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let ts_ms = if last_recorded_date.len() == 10 {
            parse_ts_ms(&format!("{last_recorded_date}T00:00:00Z"))
        } else {
            None
        };
        age_sec = age_seconds(now_ms, ts_ms);
        let stale = age_sec
            .map(|age| age > DOPAMINE_METRICS_FRESH_WINDOW_SECONDS)
            .unwrap_or(true);
        freshness = if stale { "stale" } else { "fresh" }.to_string();
        severity = if score <= 0.0 {
            "warn".to_string()
        } else {
            "info".to_string()
        };
        status = if stale
            || score <= 0.0
            || (runtime_web_tooling_strict_auth_required && !runtime_web_tooling_auth_present)
        {
            "warn".to_string()
        } else {
            "pass".to_string()
        };
    }

    json!({
        "dopamine_ambient": {
            "value": score,
            "target_min": 1.0,
            "status": status,
            "severity": severity,
            "threshold_breached": threshold_breached,
            "freshness_status": freshness,
            "latest_event_age_seconds": age_sec,
            "fresh_window_seconds": DOPAMINE_METRICS_FRESH_WINDOW_SECONDS,
            "last_recorded_date": if last_recorded_date.is_empty() { Value::Null } else { Value::String(last_recorded_date) },
            "source": source,
            "runtime_web_tooling_auth_present": runtime_web_tooling_auth_present,
            "runtime_web_tooling_strict_auth_required": runtime_web_tooling_strict_auth_required,
            "runtime_web_tooling_auth_sources": runtime_web_tooling_auth_sources
        }
    })
}

fn runtime_web_tooling_auth_sources() -> Vec<String> {
    let env_candidates = [
        "BRAVE_API_KEY",
        "EXA_API_KEY",
        "TAVILY_API_KEY",
        "PERPLEXITY_API_KEY",
        "SERPAPI_API_KEY",
        "GOOGLE_SEARCH_API_KEY",
        "GOOGLE_CSE_ID",
        "FIRECRAWL_API_KEY",
        "XAI_API_KEY",
        "MOONSHOT_API_KEY",
        "OPENAI_API_KEY",
    ];
    let mut sources = Vec::<String>::new();
    for env_name in env_candidates {
        let present = env::var(env_name)
            .ok()
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false);
        if present {
            sources.push(format!("env:{env_name}"));
        }
    }
    sources
}

fn runtime_web_tooling_strict_auth_required(root: &Path) -> bool {
    let profile_path = root.join("client/runtime/local/state/dashboard/web_tooling_profile.json");
    let profile = read_json(&profile_path).ok();
    profile
        .as_ref()
        .and_then(|value| value.get("strict_auth_required"))
        .and_then(Value::as_bool)
        .or_else(|| {
            env::var("INFRING_WEB_TOOLING_STRICT_AUTH")
                .ok()
                .map(|raw| {
                    matches!(
                        raw.trim().to_ascii_lowercase().as_str(),
                        "1" | "true" | "yes" | "y" | "on"
                    )
                })
        })
        .unwrap_or(true)
}

fn collect_external_eyes_dashboard_metric(root: &Path) -> Value {
    let now_ms = Utc::now().timestamp_millis();
    let attention_latest = root.join("client/runtime/local/state/attention/latest.json");
    let attention_queue = root.join("client/runtime/local/state/attention/queue.jsonl");
    let legacy_bridge =
        root.join("local/state/client-runtime/state/memory/eyes_memory_bridge.jsonl");

    let latest = read_json(&attention_latest).ok();
    let mut source = if attention_latest.exists() {
        attention_latest.to_string_lossy().to_string()
    } else if legacy_bridge.exists() {
        legacy_bridge.to_string_lossy().to_string()
    } else {
        "missing".to_string()
    };

    let latest_ts_ms = latest
        .as_ref()
        .and_then(|row| row.get("ts").and_then(Value::as_str))
        .and_then(parse_ts_ms);
    let mut age_sec = age_seconds(now_ms, latest_ts_ms);
    let mut external_events = 0u64;
    let mut cross_signal_events = 0u64;
    let mut rows_scanned = 0u64;

    if let Some(raw) = read_text_tail(&attention_queue, JSONL_TAIL_MAX_BYTES) {
        source = attention_queue.to_string_lossy().to_string();
        for line in raw.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let Ok(row) = serde_json::from_str::<Value>(trimmed) else {
                continue;
            };
            rows_scanned += 1;
            let row_source = row
                .get("source")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_ascii_lowercase();
            let row_type = row
                .get("source_type")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_ascii_lowercase();
            let summary = row
                .get("summary")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_ascii_lowercase();
            let is_external = row_source.contains("external_eyes")
                || row_type.contains("external")
                || row_type.contains("eye_");
            if !is_external {
                continue;
            }
            external_events += 1;
            if row_type.contains("cross_signal")
                || row_type.contains("cross-signal")
                || summary.contains("cross signal")
                || summary.contains("cross-signal")
            {
                cross_signal_events += 1;
            }
            if age_sec.is_none() {
                age_sec = parse_row_ts_ms(&row).map(|ts_ms| ((now_ms - ts_ms).max(0)) / 1000);
            }
        }
    } else if let Some(raw) = read_text_tail(&legacy_bridge, JSONL_TAIL_MAX_BYTES) {
        source = legacy_bridge.to_string_lossy().to_string();
        for line in raw.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let Ok(row) = serde_json::from_str::<Value>(trimmed) else {
                continue;
            };
            rows_scanned += 1;
            external_events += 1;
            let ts_age = parse_row_ts_ms(&row).map(|ts_ms| ((now_ms - ts_ms).max(0)) / 1000);
            if let Some(candidate) = ts_age {
                age_sec = Some(match age_sec {
                    Some(current) => current.min(candidate),
                    None => candidate,
                });
            }
            if row
                .get("source")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_ascii_lowercase()
                .contains("cross_signal")
            {
                cross_signal_events += 1;
            }
        }
    }

    let stale = age_sec
        .map(|age| age > EXTERNAL_EYES_METRICS_FRESH_WINDOW_SECONDS)
        .unwrap_or(true);
    let cross_signal_absent = external_events >= EXTERNAL_EYES_CROSS_SIGNAL_MIN_EVENTS
        && cross_signal_events == 0
        && !stale;
    let status = if stale || cross_signal_absent {
        "warn"
    } else {
        "pass"
    };
    let freshness_status = if stale { "stale" } else { "fresh" };
    let ratio = if external_events > 0 {
        cross_signal_events as f64 / external_events as f64
    } else {
        0.0
    };

    json!({
        "external_eyes_cross_signal_surface": {
            "value": ratio,
            "target_min": 0.05,
            "status": status,
            "freshness_status": freshness_status,
            "latest_event_age_seconds": age_sec,
            "fresh_window_seconds": EXTERNAL_EYES_METRICS_FRESH_WINDOW_SECONDS,
            "external_events_scanned": external_events,
            "cross_signal_events": cross_signal_events,
            "cross_signal_absent": cross_signal_absent,
            "rows_scanned": rows_scanned,
            "source": source
        }
    })
}

fn collect_spine_dashboard_metrics(root: &Path) -> Value {
    let runs_dir = root.join("client/runtime/local/state/spine/runs");
    let mut completed = 0usize;
    let mut failed = 0usize;
    let mut latency_ms = Vec::<f64>::new();
    let mut files_scanned = 0usize;
    let mut latest_event_ts_ms = None::<i64>;

    let mut run_files = Vec::new();
    if let Ok(entries) = fs::read_dir(&runs_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|v| v.to_str()) == Some("jsonl") {
                run_files.push(path);
            }
        }
    }
    run_files.sort_by(|a, b| b.file_name().cmp(&a.file_name()));

    for path in run_files.into_iter().take(SPINE_RUN_FILES_MAX) {
        files_scanned += 1;
        let Some(raw) = read_text_tail(&path, JSONL_TAIL_MAX_BYTES) else {
            continue;
        };
        for line in raw.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let Ok(row) = serde_json::from_str::<Value>(trimmed) else {
                continue;
            };
            if let Some(ts_ms) = parse_row_ts_ms(&row) {
                latest_event_ts_ms = Some(match latest_event_ts_ms {
                    Some(current) => current.max(ts_ms),
                    None => ts_ms,
                });
            }
            match row.get("type").and_then(Value::as_str).unwrap_or("") {
                "spine_run_complete" => {
                    completed += 1;
                    if let Some(ms) = row.get("elapsed_ms").and_then(Value::as_f64) {
                        latency_ms.push(ms);
                    }
                }
                "spine_run_failed" => {
                    failed += 1;
                }
                "spine_observability_trace" => {
                    if let Some(ms) = row.get("trace_duration_ms").and_then(Value::as_f64) {
                        latency_ms.push(ms);
                    }
                }
                _ => {}
            }
        }
    }

    let total = completed + failed;
    let success_rate = if total > 0 {
        completed as f64 / total as f64
    } else {
        1.0
    };
    let now_ms = Utc::now().timestamp_millis();
    let latest_event_age_seconds = latest_event_ts_ms.map(|ts_ms| ((now_ms - ts_ms).max(0)) / 1000);
    let stale = total > 0
        && latest_event_age_seconds
            .map(|age_sec| age_sec > SPINE_METRICS_FRESH_WINDOW_SECONDS)
            .unwrap_or(true);
    let freshness_status = if total == 0 {
        "missing"
    } else if stale {
        "stale"
    } else {
        "fresh"
    };
    let p95_latency = percentile_95(&latency_ms);
    let p99_latency = percentile_99(&latency_ms);

    let success_status = if total == 0 {
        "warn"
    } else if stale {
        "stale"
    } else if success_rate >= 0.999 {
        "pass"
    } else {
        "warn"
    };
    let latency_status = if stale {
        "stale"
    } else {
        match p95_latency {
            Some(v) if v < 100.0 => "pass",
            Some(_) => "warn",
            None => "warn",
        }
    };
    let latency_p99_status = if stale {
        "stale"
    } else {
        match p99_latency {
            Some(v) if v < 150.0 => "pass",
            Some(_) => "warn",
            None => "warn",
        }
    };

    json!({
        "spine_success_rate": {
            "value": success_rate,
            "target_min": 0.999,
            "status": success_status,
            "samples": total,
            "completed_runs": completed,
            "failed_runs": failed,
            "stale": stale,
            "freshness_status": freshness_status,
            "latest_event_age_seconds": latest_event_age_seconds,
            "fresh_window_seconds": SPINE_METRICS_FRESH_WINDOW_SECONDS,
            "source": "client/runtime/local/state/spine/runs/*.jsonl"
        },
        "receipt_latency_p95_ms": {
            "value": p95_latency,
            "target_max": 100.0,
            "status": latency_status,
            "samples": latency_ms.len(),
            "files_scanned": files_scanned,
            "stale": stale,
            "freshness_status": freshness_status,
            "latest_event_age_seconds": latest_event_age_seconds,
            "fresh_window_seconds": SPINE_METRICS_FRESH_WINDOW_SECONDS,
            "source": "client/runtime/local/state/spine/runs/*.jsonl"
        },
        "receipt_latency_p99_ms": {
            "value": p99_latency,
            "target_max": 150.0,
            "status": latency_p99_status,
            "samples": latency_ms.len(),
            "files_scanned": files_scanned,
            "stale": stale,
            "freshness_status": freshness_status,
            "latest_event_age_seconds": latest_event_age_seconds,
            "fresh_window_seconds": SPINE_METRICS_FRESH_WINDOW_SECONDS,
            "source": "client/runtime/local/state/spine/runs/*.jsonl"
        }
    })
}

fn pain_severity_score(severity: &str) -> f64 {
    match severity.trim().to_ascii_lowercase().as_str() {
        "low" => 0.25,
        "medium" => 0.50,
        "high" => 0.75,
        "critical" => 1.0,
        _ => 0.50,
    }
}
