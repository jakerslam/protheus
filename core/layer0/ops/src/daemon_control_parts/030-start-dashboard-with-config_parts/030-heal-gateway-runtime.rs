
fn heal_gateway_runtime(root: &Path, cfg: &DashboardLaunchConfig) -> Value {
    let desired_before = dashboard_desired_state_active(root);
    let stop_latch_before = dashboard_stop_latch_active(root);
    let mut actions = Vec::<Value>::new();

    if desired_before && stop_latch_before {
        clear_dashboard_stop_latch(root);
        actions.push(json!({
            "action": "clear_stop_latch",
            "ok": true
        }));
    }

    let mut supervisor = gateway_supervisor::status(root).payload;
    let mut supervisor_healthy = supervisor_payload_healthy(&supervisor);
    if cfg.persistent_supervisor && desired_before && !supervisor_healthy {
        let enabled = gateway_supervisor_enable(root, cfg);
        supervisor = enabled.payload;
        supervisor_healthy = supervisor_payload_healthy(&supervisor);
        actions.push(json!({
            "action": "enable_supervisor",
            "ok": supervisor.get("ok").and_then(Value::as_bool).unwrap_or(false),
            "healthy": supervisor_healthy
        }));
    }

    let health_before = dashboard_health_ok(cfg.host.as_str(), cfg.port);
    let dashboard_pid = read_pid_file(&dashboard_pid_path(root));
    let listeners = dashboard_listener_pids(cfg.port);
    let process_active = dashboard_pid.map(pid_running).unwrap_or(false) || !listeners.is_empty();

    let mut restart_payload = Value::Null;
    if desired_before && !health_before && !process_active {
        restart_payload = restart_dashboard_for_watchdog(root, cfg);
        actions.push(json!({
            "action": "restart_dashboard",
            "ok": restart_payload.get("running").and_then(Value::as_bool).unwrap_or(false)
        }));
    }

    let watchdog_before = dashboard_watchdog_status(root);
    let watchdog_running = watchdog_before
        .get("running")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let mut watchdog_fallback = Value::Null;
    if desired_before && !watchdog_running && (!cfg.persistent_supervisor || !supervisor_healthy) {
        watchdog_fallback = match spawn_dashboard_watchdog(root, cfg) {
            Ok(pid) => json!({
                "ok": true,
                "running": true,
                "pid": pid,
                "mode": "local_watchdog_fallback"
            }),
            Err(err) => json!({
                "ok": false,
                "running": false,
                "error": err,
                "mode": "local_watchdog_fallback"
            }),
        };
        actions.push(json!({
            "action": "spawn_watchdog_fallback",
            "ok": watchdog_fallback.get("ok").and_then(Value::as_bool).unwrap_or(false),
            "running": watchdog_fallback.get("running").and_then(Value::as_bool).unwrap_or(false)
        }));
    }

    json!({
        "ok": true,
        "desired_before": desired_before,
        "stop_latch_before": stop_latch_before,
        "health_before": health_before,
        "process_active_before": process_active,
        "dashboard_pid_before": dashboard_pid,
        "listener_pids_before": listeners,
        "watchdog_before": watchdog_before,
        "supervisor": supervisor,
        "restart": restart_payload,
        "watchdog_fallback": watchdog_fallback,
        "desired_after": dashboard_desired_state_active(root),
        "stop_latch_after": dashboard_stop_latch_active(root),
        "health_after": dashboard_health_ok(cfg.host.as_str(), cfg.port),
        "watchdog_after": dashboard_watchdog_status(root),
        "actions": actions,
    })
}

fn verity_drift_status_receipt(root: &Path, argv: &[String]) -> Value {
    let (signed, signature_valid) = load_verity_signed_config(root);
    let mode = normalize_verity_mode(&signed.mode);
    let active_tolerance_ms = if mode == VERITY_DRIFT_MODE_SIMULATION {
        signed.simulation_tolerance_ms
    } else {
        signed.production_tolerance_ms
    };
    let limit = parse_u64(parse_flag(argv, "limit").as_deref(), 10, 1, 50) as usize;
    let recent_events = load_recent_verity_drift_events(root, limit);
    let config_path = resolve_verity_drift_config_path(root);
    let events_path = resolve_verity_drift_events_path(root);
    let lock_path = resolve_verity_judicial_lock_path(root);
    let lock_payload = fs::read_to_string(&lock_path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok());

    let mut out = json!({
        "ok": true,
        "type": "verity_drift_status",
        "mode": mode,
        "policy_version": signed.policy_version,
        "schema_id": signed.schema_id,
        "schema_version": signed.schema_version,
        "signature_valid": signature_valid,
        "active_tolerance_ms": active_tolerance_ms,
        "production_tolerance_ms": signed.production_tolerance_ms,
        "simulation_tolerance_ms": signed.simulation_tolerance_ms,
        "config_path": config_path.to_string_lossy().to_string(),
        "events_path": events_path.to_string_lossy().to_string(),
        "judicial_lock_path": lock_path.to_string_lossy().to_string(),
        "judicial_lock": lock_payload,
        "recent_events_limit": limit,
        "recent_drift_events": recent_events,
    });
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    out
}

fn usage() {
    println!("Usage:");
    println!("  protheus-ops daemon-control <start|stop|restart|status|heal|attach|subscribe|tick|diagnostics|drift-status|watchdog> [--mode=<value>]");
    println!("  Optional start/restart flags:");
    println!("    --dashboard-autoboot=1|0   (default: 1)");
    println!("    --dashboard-open=1|0       (default: 1)");
    println!("    --dashboard-host=<ip>      (default: 127.0.0.1)");
    println!("    --dashboard-port=<n>       (default: 4173)");
    println!("    --dashboard-ready-timeout-ms=<n> (default: 36000)");
    println!("    --dashboard-watchdog-interval-ms=<n> (default: 2000)");
    println!("    --node-binary=<path>       (default: auto-detected node path)");
    println!("    --gateway-persist=1|0      (default: 1 on start/restart)");
}

pub(crate) fn success_receipt(
    command: &str,
    mode: Option<&str>,
    argv: &[String],
    root: &Path,
) -> Value {
    let mut out = protheus_ops_core_v1::daemon_control_receipt(command, mode);
    if let Some(obj) = out.as_object_mut() {
        obj.insert("argv".to_string(), json!(argv));
        obj.insert(
            "root".to_string(),
            Value::String(root.to_string_lossy().to_string()),
        );
        obj.insert(
            "lazy_init".to_string(),
            json!({
                "enabled": true,
                "boot_scope": ["conduit", "safety_kernel"],
                "deferred": ["layer0_noncritical", "layer1_policy_extensions", "client_surfaces"]
            }),
        );
        obj.insert(
            "claim_evidence".to_string(),
            json!([
                {
                    "id": "daemon_control_core_lane",
                    "claim": "daemon_control_commands_are_core_authoritative",
                    "evidence": {
                        "command": command,
                        "mode": mode
                    }
                }
            ]),
        );
    }
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    out
}

pub fn inprocess_lazy_probe_receipt(root: &Path) -> Value {
    success_receipt(
        "start",
        Some("lazy-minimal"),
        &[
            "start".to_string(),
            "--mode=lazy-minimal".to_string(),
            "--lazy-init=1".to_string(),
        ],
        root,
    )
}
fn error_receipt(error: &str, argv: &[String]) -> Value {
    let mut out = json!({
        "ok": false,
        "type": "daemon_control_error",
        "error": error,
        "argv": argv,
    });
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    out
}
