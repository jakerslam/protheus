fn collect_assimilation_pain_dashboard_metric(root: &Path) -> Value {
    let pain_path = root.join("client/runtime/local/state/autonomy/pain_signals.jsonl");
    let mut total_score = 0.0f64;
    let mut total_count = 0usize;
    let mut by_source = BTreeMap::<String, (f64, usize)>::new();

    if let Some(raw) = read_text_tail(&pain_path, JSONL_TAIL_MAX_BYTES) {
        for line in raw.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let Ok(row) = serde_json::from_str::<Value>(trimmed) else {
                continue;
            };
            if row.get("type").and_then(Value::as_str) != Some("pain_signal") {
                continue;
            }
            let source = row
                .get("source")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string();
            let score = pain_severity_score(
                row.get("severity")
                    .and_then(Value::as_str)
                    .unwrap_or("medium"),
            );
            total_score += score;
            total_count += 1;
            let entry = by_source.entry(source).or_insert((0.0, 0));
            entry.0 += score;
            entry.1 += 1;
        }
    }

    let avg = if total_count > 0 {
        total_score / total_count as f64
    } else {
        0.0
    };
    let status = if avg < 0.5 { "pass" } else { "warn" };

    let mut top_sources = by_source
        .iter()
        .map(|(source, (sum, count))| {
            let avg = if *count > 0 {
                *sum / *count as f64
            } else {
                0.0
            };
            json!({
                "source": source,
                "avg_score": avg,
                "samples": count
            })
        })
        .collect::<Vec<_>>();
    top_sources.sort_by(|a, b| {
        let av = a.get("avg_score").and_then(Value::as_f64).unwrap_or(0.0);
        let bv = b.get("avg_score").and_then(Value::as_f64).unwrap_or(0.0);
        bv.partial_cmp(&av).unwrap_or(std::cmp::Ordering::Equal)
    });
    top_sources.truncate(5);

    json!({
        "assimilation_pain_score": {
            "value": avg,
            "target_max": 0.5,
            "status": status,
            "samples": total_count,
            "top_sources": top_sources,
            "source": "client/runtime/local/state/autonomy/pain_signals.jsonl"
        }
    })
}

fn collect_human_escalation_dashboard_metric(root: &Path) -> Value {
    let escalation_path =
        root.join("client/runtime/local/state/security/autonomy_human_escalations.jsonl");
    let mut latest_status_by_id = BTreeMap::<String, String>::new();
    let mut total_events = 0usize;

    if let Some(raw) = read_text_tail(&escalation_path, JSONL_TAIL_MAX_BYTES) {
        for line in raw.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let Ok(row) = serde_json::from_str::<Value>(trimmed) else {
                continue;
            };
            if row.get("type").and_then(Value::as_str) != Some("autonomy_human_escalation") {
                continue;
            }
            let escalation_id = row
                .get("escalation_id")
                .and_then(Value::as_str)
                .unwrap_or("")
                .trim();
            if escalation_id.is_empty() {
                continue;
            }
            total_events += 1;
            let status = row
                .get("status")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_ascii_lowercase();
            latest_status_by_id.insert(escalation_id.to_string(), status);
        }
    }

    let mut open_count = 0usize;
    let mut resolved_count = 0usize;
    for status in latest_status_by_id.values() {
        match status.as_str() {
            "open" => open_count += 1,
            "resolved" => resolved_count += 1,
            _ => {}
        }
    }
    let total_unique = latest_status_by_id.len();
    let open_rate = if total_unique > 0 {
        open_count as f64 / total_unique as f64
    } else {
        0.0
    };
    let status = if open_rate <= 0.10 { "pass" } else { "warn" };

    json!({
        "human_escalation_open_rate": {
            "value": open_rate,
            "target_max": 0.10,
            "status": status,
            "open_count": open_count,
            "resolved_count": resolved_count,
            "unique_escalations": total_unique,
            "events_scanned": total_events,
            "source": "client/runtime/local/state/security/autonomy_human_escalations.jsonl"
        }
    })
}

fn value_as_f64(value: Option<&Value>) -> Option<f64> {
    match value {
        Some(Value::Number(n)) => n.as_f64(),
        Some(Value::String(raw)) => raw.trim().parse::<f64>().ok(),
        _ => None,
    }
}

fn collect_token_burn_cost_dashboard_metric(root: &Path) -> Value {
    let budget_path = root.join("client/runtime/local/state/autonomy/budget_events.jsonl");
    let mut latest_day = String::new();
    let mut tokens_by_day = BTreeMap::<String, f64>::new();
    let mut module_tokens = BTreeMap::<String, f64>::new();
    let mut deny_count = 0usize;
    let mut scanned = 0usize;

    if let Some(raw) = read_text_tail(&budget_path, JSONL_TAIL_MAX_BYTES) {
        for line in raw.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let Ok(row) = serde_json::from_str::<Value>(trimmed) else {
                continue;
            };
            let row_type = row.get("type").and_then(Value::as_str).unwrap_or("");
            if row_type != "system_budget_record" && row_type != "system_budget_decision" {
                continue;
            }
            scanned += 1;
            if row_type == "system_budget_decision"
                && row
                    .get("decision")
                    .and_then(Value::as_str)
                    .map(|v| v.eq_ignore_ascii_case("deny"))
                    .unwrap_or(false)
            {
                deny_count += 1;
            }

            if row_type != "system_budget_record" {
                continue;
            }

            let Some(tokens) = value_as_f64(row.get("tokens_est")) else {
                continue;
            };
            if !tokens.is_finite() || tokens < 0.0 {
                continue;
            }
            let module = row
                .get("module")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string();
            *module_tokens.entry(module).or_insert(0.0) += tokens;

            if let Some(date) = row.get("date").and_then(Value::as_str) {
                let date = date.trim();
                if !date.is_empty() {
                    *tokens_by_day.entry(date.to_string()).or_insert(0.0) += tokens;
                    if date > latest_day.as_str() {
                        latest_day = date.to_string();
                    }
                }
            }
        }
    }

    let latest_day_tokens = if latest_day.is_empty() {
        0.0
    } else {
        *tokens_by_day.get(&latest_day).unwrap_or(&0.0)
    };
    let assumed_usd_per_million_tokens = 2.0f64;
    let estimated_cost_usd = (latest_day_tokens / 1_000_000.0) * assumed_usd_per_million_tokens;
    let status = if latest_day_tokens <= 200_000.0 {
        "pass"
    } else {
        "warn"
    };

    let mut top_modules = module_tokens
        .iter()
        .map(|(module, tokens)| {
            json!({
                "module": module,
                "tokens": (*tokens).round() as i64
            })
        })
        .collect::<Vec<_>>();
    top_modules.sort_by(|a, b| {
        let av = a.get("tokens").and_then(Value::as_i64).unwrap_or(0);
        let bv = b.get("tokens").and_then(Value::as_i64).unwrap_or(0);
        bv.cmp(&av)
    });
    top_modules.truncate(5);

    json!({
        "token_burn_cost_attribution": {
            "status": status,
            "latest_day": if latest_day.is_empty() { Value::Null } else { Value::String(latest_day) },
            "latest_day_tokens": latest_day_tokens.round() as i64,
            "target_max_tokens_per_day": 200000,
            "assumed_usd_per_million_tokens": assumed_usd_per_million_tokens,
            "estimated_cost_usd": estimated_cost_usd,
            "deny_decisions": deny_count,
            "events_scanned": scanned,
            "top_modules": top_modules,
            "source": "client/runtime/local/state/autonomy/budget_events.jsonl"
        }
    })
}

fn collect_pqts_slippage_dashboard_metric(root: &Path) -> Value {
    let reports_dir =
        root.join("client/runtime/local/workspaces/pqts/data/client/reports/mape_matrix_no_stress");
    let mut latest_snapshot = None::<String>;

    if let Ok(entries) = fs::read_dir(&reports_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let Some(name) = name.to_str() else {
                continue;
            };
            if !name.starts_with("paper_campaign_snapshot_") || !name.ends_with(".json") {
                continue;
            }
            let candidate = name.to_string();
            if latest_snapshot
                .as_ref()
                .map(|current| candidate > *current)
                .unwrap_or(true)
            {
                latest_snapshot = Some(candidate);
            }
        }
    }

    let Some(snapshot_name) = latest_snapshot else {
        return json!({
            "pqts_slippage_mape_pct": {
                "value": Value::Null,
                "target_max": 15.0,
                "status": "warn",
                "reason": "pqts_snapshot_missing",
                "source": "client/runtime/local/workspaces/pqts/data/client/reports/mape_matrix_no_stress"
            }
        });
    };

    let snapshot_path = reports_dir.join(&snapshot_name);
    let payload = match read_json(&snapshot_path) {
        Ok(v) => v,
        Err(err) => {
            return json!({
                "pqts_slippage_mape_pct": {
                    "value": Value::Null,
                    "target_max": 15.0,
                    "status": "warn",
                    "reason": err,
                    "source": snapshot_path.to_string_lossy()
                }
            });
        }
    };

    let mape = payload
        .get("readiness")
        .and_then(|v| v.get("slippage_mape_pct"))
        .and_then(Value::as_f64);
    let status = match mape {
        Some(v) if v < 15.0 => "pass",
        Some(_) => "warn",
        None => "warn",
    };

    json!({
        "pqts_slippage_mape_pct": {
            "value": mape,
            "target_max": 15.0,
            "status": status,
            "snapshot": snapshot_name,
            "source": "client/runtime/local/workspaces/pqts/data/client/reports/mape_matrix_no_stress"
        }
    })
}

fn collect_skills_plane_dashboard_metric(root: &Path) -> Value {
    let latest_path = root
        .join("core")
        .join("local")
        .join("state")
        .join("ops")
        .join("skills_plane")
        .join("latest.json");
    let runs_path = root
        .join("core")
        .join("local")
        .join("state")
        .join("ops")
        .join("skills_plane")
        .join("runs")
        .join("history.jsonl");

    let latest = read_json(&latest_path).ok();
    let run_count = read_text_tail(&runs_path, JSONL_TAIL_MAX_BYTES)
        .map(|raw| raw.lines().filter(|line| !line.trim().is_empty()).count() as u64)
        .unwrap_or(0);
    let skills_total = latest
        .as_ref()
        .and_then(|v| v.get("discovered_count"))
        .and_then(Value::as_u64)
        .or_else(|| {
            latest
                .as_ref()
                .and_then(|v| v.get("metrics"))
                .and_then(|v| v.get("skills_total"))
                .and_then(Value::as_u64)
        })
        .unwrap_or(0);
    let status = if latest.is_some() { "pass" } else { "warn" };

    json!({
        "skills_plane_health": {
            "value": if latest.is_some() { 1.0 } else { 0.0 },
            "target_min": 1.0,
            "status": status,
            "skills_total": skills_total,
            "run_history_count": run_count,
            "source": latest_path.to_string_lossy()
        }
    })
}

fn collect_binary_vuln_dashboard_metric(root: &Path) -> Value {
    let latest_path = root
        .join("core")
        .join("local")
        .join("state")
        .join("ops")
        .join("binary_vuln_plane")
        .join("latest.json");
    let latest = read_json(&latest_path).ok();
    let findings = latest
        .as_ref()
        .and_then(|v| v.get("output"))
        .and_then(|v| v.get("finding_count"))
        .and_then(Value::as_u64)
        .or_else(|| {
            latest
                .as_ref()
                .and_then(|v| v.get("findings"))
                .and_then(Value::as_array)
                .map(|rows| rows.len() as u64)
        })
        .unwrap_or(0);
    let status = match latest
        .as_ref()
        .and_then(|v| v.get("ok"))
        .and_then(Value::as_bool)
    {
        Some(true) => "pass",
        Some(false) => "warn",
        None => "warn",
    };

    json!({
        "binary_vuln_surface": {
            "value": findings,
            "target_min": 0,
            "status": status,
            "latest_finding_count": findings,
            "source": latest_path.to_string_lossy()
        }
    })
}

fn collect_hermes_dashboard_metric(root: &Path) -> Value {
    let latest_path = root
        .join("core")
        .join("local")
        .join("state")
        .join("ops")
        .join("hermes_plane")
        .join("latest.json");
    let latest = read_json(&latest_path).ok();
    let block_count = latest
        .as_ref()
        .and_then(|v| v.get("cockpit"))
        .and_then(|v| v.get("render"))
        .and_then(|v| v.get("total_blocks"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let status = if block_count > 0 || latest.is_some() {
        "pass"
    } else {
        "warn"
    };

    json!({
        "hermes_cockpit_stream": {
            "value": block_count,
            "target_min": 1,
            "status": status,
            "stream_blocks": block_count,
            "source": latest_path.to_string_lossy()
        }
    })
}

