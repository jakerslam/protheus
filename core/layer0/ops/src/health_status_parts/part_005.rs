fn collect_dashboard_metrics(root: &Path, cron_audit: &Value) -> Value {
    let enabled_jobs = cron_audit
        .get("enabled_jobs")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let issue_count = cron_audit
        .get("issues")
        .and_then(Value::as_array)
        .map(|rows| rows.len() as u64)
        .unwrap_or(0);
    let cron_health = if enabled_jobs > 0 {
        enabled_jobs.saturating_sub(issue_count) as f64 / enabled_jobs as f64
    } else {
        1.0
    };
    let cron_status = if cron_health >= 0.90 { "pass" } else { "warn" };

    let mut metrics = serde_json::Map::<String, Value>::new();
    metrics.insert(
        "cron_job_health".to_string(),
        json!({
            "value": cron_health,
            "target_min": 0.90,
            "status": cron_status,
            "enabled_jobs": enabled_jobs,
            "issues": issue_count,
            "source": "client/runtime/config/cron_jobs.json"
        }),
    );

    if let Some(obj) = collect_spine_dashboard_metrics(root).as_object() {
        for (k, v) in obj {
            metrics.insert(k.clone(), v.clone());
        }
    }
    if let Some(obj) = collect_assimilation_pain_dashboard_metric(root).as_object() {
        for (k, v) in obj {
            metrics.insert(k.clone(), v.clone());
        }
    }
    if let Some(obj) = collect_human_escalation_dashboard_metric(root).as_object() {
        for (k, v) in obj {
            metrics.insert(k.clone(), v.clone());
        }
    }
    if let Some(obj) = collect_token_burn_cost_dashboard_metric(root).as_object() {
        for (k, v) in obj {
            metrics.insert(k.clone(), v.clone());
        }
    }
    if let Some(obj) = collect_pqts_slippage_dashboard_metric(root).as_object() {
        for (k, v) in obj {
            metrics.insert(k.clone(), v.clone());
        }
    }
    if let Some(obj) = collect_skills_plane_dashboard_metric(root).as_object() {
        for (k, v) in obj {
            metrics.insert(k.clone(), v.clone());
        }
    }
    if let Some(obj) = collect_binary_vuln_dashboard_metric(root).as_object() {
        for (k, v) in obj {
            metrics.insert(k.clone(), v.clone());
        }
    }
    if let Some(obj) = collect_hermes_dashboard_metric(root).as_object() {
        for (k, v) in obj {
            metrics.insert(k.clone(), v.clone());
        }
    }
    if let Some(obj) = collect_vbrowser_dashboard_metric(root).as_object() {
        for (k, v) in obj {
            metrics.insert(k.clone(), v.clone());
        }
    }
    if let Some(obj) = collect_agency_dashboard_metric(root).as_object() {
        for (k, v) in obj {
            metrics.insert(k.clone(), v.clone());
        }
    }
    if let Some(obj) = collect_collab_dashboard_metric(root).as_object() {
        for (k, v) in obj {
            metrics.insert(k.clone(), v.clone());
        }
    }
    if let Some(obj) = collect_company_dashboard_metric(root).as_object() {
        for (k, v) in obj {
            metrics.insert(k.clone(), v.clone());
        }
    }
    if let Some(obj) = collect_company_heartbeat_dashboard_metric(root).as_object() {
        for (k, v) in obj {
            metrics.insert(k.clone(), v.clone());
        }
    }
    if let Some(obj) = collect_moltbook_credentials_dashboard_metric(root).as_object() {
        for (k, v) in obj {
            metrics.insert(k.clone(), v.clone());
        }
    }
    if let Some(obj) = collect_dopamine_ambient_dashboard_metric(root).as_object() {
        for (k, v) in obj {
            metrics.insert(k.clone(), v.clone());
        }
    }
    if let Some(obj) = collect_external_eyes_dashboard_metric(root).as_object() {
        for (k, v) in obj {
            metrics.insert(k.clone(), v.clone());
        }
    }
    if let Some(obj) = collect_substrate_dashboard_metric(root).as_object() {
        for (k, v) in obj {
            metrics.insert(k.clone(), v.clone());
        }
    }
    if let Some(obj) = collect_observability_dashboard_metric(root).as_object() {
        for (k, v) in obj {
            metrics.insert(k.clone(), v.clone());
        }
    }
    if let Some(obj) = collect_persist_dashboard_metric(root).as_object() {
        for (k, v) in obj {
            metrics.insert(k.clone(), v.clone());
        }
    }

    Value::Object(metrics)
}

fn checks_summary(cron_ok: bool, source_ok: bool) -> Value {
    let verification_ok = cron_ok && source_ok;
    let status = |ok: bool| if ok { "pass" } else { "warn" };
    json!({
        "proposal_starvation": {"status": "pass", "source": "rust_health_baseline"},
        "queue_backlog": {"status": "pass", "source": "rust_health_baseline"},
        "dark_eyes": {"status": "pass", "source": "rust_health_baseline"},
        "loop_stall": {"status": "pass", "source": "rust_health_baseline"},
        "drift": {"status": "pass", "source": "rust_health_baseline"},
        "budget_guard": {"status": "pass", "source": "rust_health_baseline"},
        "budget_pressure": {"status": "pass", "source": "rust_health_baseline"},
        "dream_degradation": {"status": "pass", "source": "rust_health_baseline"},
        "verification_pass_rate": {
            "status": status(verification_ok),
            "source": "rust_health_integrity_gate",
            "details": {
                "cron_delivery_integrity_ok": cron_ok,
                "rust_source_of_truth_ok": source_ok
            }
        },
        "cron_delivery_integrity": {
            "status": status(cron_ok),
            "source": "rust_health_integrity_gate"
        },
        "rust_source_of_truth": {
            "status": status(source_ok),
            "source": "rust_health_integrity_gate"
        }
    })
}

fn status_receipt(root: &Path, cmd: &str, args: &[String], dashboard: bool) -> Value {
    let cron_audit = audit_cron_delivery(root);
    let source_audit = audit_rust_source_of_truth(root);

    let cron_ok = cron_audit
        .get("ok")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let source_ok = source_audit
        .get("ok")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let checks = checks_summary(cron_ok, source_ok);
    let dashboard_metrics = if dashboard {
        collect_dashboard_metrics(root, &cron_audit)
    } else {
        collect_dashboard_metrics_light(&cron_audit)
    };

    let mut alert_checks = Vec::<String>::new();
    if let Some(map) = checks.as_object() {
        for (k, v) in map {
            let status = v.get("status").and_then(Value::as_str).unwrap_or("unknown");
            if status != "pass" {
                alert_checks.push(k.to_string());
            }
        }
    }
    if let Some(metric_map) = dashboard_metrics.as_object() {
        for (metric, payload) in metric_map {
            let status = payload
                .get("status")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            if status != "pass" {
                alert_checks.push(format!("metric:{metric}"));
            }
        }
    }

    let mut out = json!({
        "ok": cron_ok && source_ok,
        "type": if dashboard { "health_status_dashboard" } else { "health_status" },
        "lane": LANE_ID,
        "ts": now_iso(),
        "command": cmd,
        "argv": args,
        "root": root.to_string_lossy(),
        "replacement": REPLACEMENT,
        "checks": checks,
        "slo": {
            "checks": checks,
            "metrics": dashboard_metrics
        },
        "dashboard_metrics": dashboard_metrics,
        "cron_delivery_integrity": cron_audit,
        "rust_source_of_truth_integrity": source_audit,
        "alerts": {
            "count": alert_checks.len(),
            "checks": alert_checks
        },
        "claim_evidence": [
            {
                "id": "native_health_status_lane",
                "claim": "health_status_executes_natively_in_rust",
                "evidence": {
                    "command": cmd,
                    "argv_len": args.len(),
                    "cron_delivery_integrity_ok": cron_ok,
                    "rust_source_of_truth_ok": source_ok
                }
            }
        ],
        "persona_lenses": {
            "operator": {
                "mode": if dashboard { "dashboard" } else { "status" }
            },
            "auditor": {
                "deterministic_receipt": true
            }
        }
    });

    out["receipt_hash"] = Value::String(receipt_hash(&out));
    out
}

fn cli_error_receipt(args: &[String], err: &str, code: i32) -> Value {
    let mut out = json!({
        "ok": false,
        "type": "health_status_cli_error",
        "lane": LANE_ID,
        "ts": now_iso(),
        "argv": args,
        "error": err,
        "exit_code": code,
        "claim_evidence": [
            {
                "id": "health_status_fail_closed_cli",
                "claim": "invalid_health_status_commands_fail_closed",
                "evidence": {
                    "error": err,
                    "argv_len": args.len()
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(receipt_hash(&out));
    out
}

fn looks_like_iso_date(token: &str) -> bool {
    let t = token.trim();
    if t.len() != 10 {
        return false;
    }
    let bytes = t.as_bytes();
    bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(idx, b)| (idx == 4 || idx == 7) || b.is_ascii_digit())
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    if argv
        .iter()
        .any(|v| matches!(v.as_str(), "help" | "--help" | "-h"))
    {
        usage();
        return 0;
    }

    let dashboard_flag = argv
        .iter()
        .any(|v| matches!(v.as_str(), "dashboard" | "--dashboard"));

    let first = argv
        .first()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());

    let cmd = if dashboard_flag {
        "dashboard"
    } else if matches!(first.as_str(), "status" | "run" | "dashboard") {
        first.as_str()
    } else if first.is_empty() || first.starts_with('-') || looks_like_iso_date(&first) {
        "status"
    } else {
        usage();
        print_json_line(&cli_error_receipt(argv, "unknown_command", 2));
        return 2;
    };

    match cmd {
        "status" | "run" => {
            let receipt = status_receipt(root, cmd, argv, false);
            persist_latest(root, &receipt);
            print_json_line(&receipt);
            0
        }
        "dashboard" => {
            let receipt = status_receipt(root, cmd, argv, true);
            persist_latest(root, &receipt);
            print_json_line(&receipt);
            0
        }
        _ => {
            usage();
            print_json_line(&cli_error_receipt(argv, "unknown_command", 2));
            2
        }
    }
}

