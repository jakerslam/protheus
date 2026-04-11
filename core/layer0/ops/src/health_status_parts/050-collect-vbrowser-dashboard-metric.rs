fn collect_vbrowser_dashboard_metric(root: &Path) -> Value {
    let latest_path = root
        .join("core")
        .join("local")
        .join("state")
        .join("ops")
        .join("vbrowser_plane")
        .join("latest.json");
    let latest = read_json(&latest_path).ok();
    let session_id = latest.as_ref().and_then(|v| {
        v.pointer("/session/session_id")
            .and_then(Value::as_str)
            .or_else(|| v.get("session_id").and_then(Value::as_str))
            .or_else(|| v.pointer("/policy/session_id").and_then(Value::as_str))
            .map(ToString::to_string)
    });
    let stream_latency_ms = latest
        .as_ref()
        .and_then(|v| {
            v.pointer("/session/stream/latency_ms")
                .and_then(Value::as_u64)
                .or_else(|| v.pointer("/stream/latency_ms").and_then(Value::as_u64))
        })
        .unwrap_or(0);
    let receipt_type = latest
        .as_ref()
        .and_then(|v| v.get("type"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let receipt_hash_present = latest
        .as_ref()
        .and_then(|v| v.get("receipt_hash"))
        .and_then(Value::as_str)
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false);
    let status = match latest
        .as_ref()
        .and_then(|v| v.get("ok"))
        .and_then(Value::as_bool)
    {
        Some(true) if receipt_hash_present && receipt_type.starts_with("vbrowser_plane_") => "pass",
        Some(true) => "warn",
        Some(false) => "warn",
        None => "warn",
    };
    json!({
        "vbrowser_session_surface": {
            "value": if latest.is_some() { 1.0 } else { 0.0 },
            "target_min": 1.0,
            "status": status,
            "session_id": session_id,
            "stream_latency_ms": stream_latency_ms,
            "receipt_type": receipt_type,
            "receipt_hash_present": receipt_hash_present,
            "source": latest_path.to_string_lossy()
        }
    })
}

fn collect_agency_dashboard_metric(root: &Path) -> Value {
    let latest_path = root
        .join("core")
        .join("local")
        .join("state")
        .join("ops")
        .join("agency_plane")
        .join("latest.json");
    let latest = read_json(&latest_path).ok();
    let active_template = latest
        .as_ref()
        .and_then(|v| v.get("shadow"))
        .and_then(|v| v.get("template"))
        .and_then(Value::as_str)
        .map(ToString::to_string);
    let handoff_count = latest
        .as_ref()
        .and_then(|v| v.get("handoff_receipts"))
        .and_then(Value::as_array)
        .map(|rows| rows.len() as u64)
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
        "agency_topology_surface": {
            "value": if latest.is_some() { 1.0 } else { 0.0 },
            "target_min": 1.0,
            "status": status,
            "active_template": active_template,
            "handoff_count": handoff_count,
            "source": latest_path.to_string_lossy()
        }
    })
}

fn collect_collab_dashboard_metric(root: &Path) -> Value {
    let latest_path = root
        .join("core")
        .join("local")
        .join("state")
        .join("ops")
        .join("collab_plane")
        .join("latest.json");
    let latest = read_json(&latest_path).ok();
    let refresh_ms = latest
        .as_ref()
        .and_then(|v| v.get("dashboard"))
        .and_then(|v| v.get("refresh_ms"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let handoff_count = latest
        .as_ref()
        .and_then(|v| v.get("dashboard"))
        .and_then(|v| v.get("handoff_history"))
        .and_then(Value::as_array)
        .map(|rows| rows.len() as u64)
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
        "collab_team_surface": {
            "value": if latest.is_some() { 1.0 } else { 0.0 },
            "target_min": 1.0,
            "status": status,
            "refresh_ms": refresh_ms,
            "handoff_count": handoff_count,
            "source": latest_path.to_string_lossy()
        }
    })
}

fn collect_company_dashboard_metric(root: &Path) -> Value {
    let latest_path = root
        .join("core")
        .join("local")
        .join("state")
        .join("ops")
        .join("company_plane")
        .join("latest.json");
    let latest = read_json(&latest_path).ok();
    let budget_hard_stop = latest
        .as_ref()
        .and_then(|v| v.get("decision"))
        .and_then(|v| v.get("hard_stop"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let org_edges = latest
        .as_ref()
        .and_then(|v| v.get("hierarchy"))
        .and_then(|v| v.get("hierarchy"))
        .and_then(|v| v.get("reporting_edges"))
        .and_then(Value::as_array)
        .map(|rows| rows.len() as u64)
        .unwrap_or(0);
    let status = if budget_hard_stop {
        "warn"
    } else if latest.is_some() {
        "pass"
    } else {
        "warn"
    };
    json!({
        "company_governance_surface": {
            "value": if latest.is_some() { 1.0 } else { 0.0 },
            "target_min": 1.0,
            "status": status,
            "budget_hard_stop": budget_hard_stop,
            "org_reporting_edges": org_edges,
            "source": latest_path.to_string_lossy()
        }
    })
}

fn collect_company_heartbeat_dashboard_metric(root: &Path) -> Value {
    let feed_path = root
        .join("core")
        .join("local")
        .join("state")
        .join("ops")
        .join("company_plane")
        .join("heartbeat")
        .join("remote_feed.json");
    let feed = read_json(&feed_path).ok();
    let teams = feed
        .as_ref()
        .and_then(|v| v.get("teams"))
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let active_teams = teams.len() as u64;
    let degraded_teams = teams
        .values()
        .filter(|row| {
            row.get("degraded")
                .and_then(Value::as_bool)
                .unwrap_or(false)
        })
        .count() as u64;
    let status = if active_teams == 0 || degraded_teams > 0 {
        "warn"
    } else {
        "pass"
    };
    json!({
        "company_heartbeat_surface": {
            "value": if active_teams > 0 { 1.0 } else { 0.0 },
            "target_min": 1.0,
            "status": status,
            "active_teams": active_teams,
            "degraded_teams": degraded_teams,
            "source": feed_path.to_string_lossy()
        }
    })
}

fn collect_substrate_dashboard_metric(root: &Path) -> Value {
    let latest_path = root
        .join("core")
        .join("local")
        .join("state")
        .join("ops")
        .join("substrate_plane")
        .join("latest.json");
    let latest = read_json(&latest_path).ok();
    let status = match latest
        .as_ref()
        .and_then(|v| v.get("ok"))
        .and_then(Value::as_bool)
    {
        Some(true) => "pass",
        Some(false) => "warn",
        None => "warn",
    };
    let feedback_mode = latest
        .as_ref()
        .and_then(|v| v.get("feedback"))
        .and_then(|v| v.get("mode"))
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_string();
    let bio_enable = read_json(
        &root
            .join("core")
            .join("local")
            .join("state")
            .join("ops")
            .join("substrate_plane")
            .join("bio")
            .join("enable")
            .join("latest.json"),
    )
    .ok();
    let biological_mode = bio_enable
        .as_ref()
        .and_then(|v| v.get("mode"))
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_string();
    json!({
        "substrate_signal_surface": {
            "value": if latest.is_some() { 1.0 } else { 0.0 },
            "target_min": 1.0,
            "status": status,
            "feedback_mode": feedback_mode,
            "biological_mode": biological_mode,
            "source": latest_path.to_string_lossy()
        }
    })
}

fn collect_observability_dashboard_metric(root: &Path) -> Value {
    let latest_path = root
        .join("core")
        .join("local")
        .join("state")
        .join("ops")
        .join("observability_plane")
        .join("latest.json");
    let latest = read_json(&latest_path).ok();
    let status = match latest
        .as_ref()
        .and_then(|v| v.get("ok"))
        .and_then(Value::as_bool)
    {
        Some(true) => "pass",
        Some(false) => "warn",
        None => "warn",
    };
    let lane = latest
        .as_ref()
        .and_then(|v| v.get("type"))
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_string();
    json!({
        "observability_control_surface": {
            "value": if latest.is_some() { 1.0 } else { 0.0 },
            "target_min": 1.0,
            "status": status,
            "lane_type": lane,
            "source": latest_path.to_string_lossy()
        }
    })
}

fn collect_persist_dashboard_metric(root: &Path) -> Value {
    let latest_path = root
        .join("core")
        .join("local")
        .join("state")
        .join("ops")
        .join("persist_plane")
        .join("latest.json");
    let latest = read_json(&latest_path).ok();
    let status = match latest
        .as_ref()
        .and_then(|v| v.get("ok"))
        .and_then(Value::as_bool)
    {
        Some(true) => "pass",
        Some(false) => "warn",
        None => "warn",
    };
    let mobile = read_json(
        &root
            .join("core")
            .join("local")
            .join("state")
            .join("ops")
            .join("persist_plane")
            .join("mobile")
            .join("latest.json"),
    )
    .ok();
    let mobile_connected = mobile
        .as_ref()
        .and_then(|v| v.get("connected"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    json!({
        "persist_background_surface": {
            "value": if latest.is_some() { 1.0 } else { 0.0 },
            "target_min": 1.0,
            "status": status,
            "mobile_connected": mobile_connected,
            "source": latest_path.to_string_lossy()
        }
    })
}

fn collect_dashboard_metrics_light(cron_audit: &Value) -> Value {
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
    Value::Object(metrics)
}
