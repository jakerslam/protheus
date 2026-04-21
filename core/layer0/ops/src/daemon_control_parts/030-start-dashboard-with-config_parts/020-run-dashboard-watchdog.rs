
fn run_dashboard_watchdog(root: &Path, argv: &[String]) -> i32 {
    let cfg = parse_dashboard_launch_config(argv, "start");
    if !cfg.enabled {
        print_json_line(&json!({
            "ok": true,
            "type": "dashboard_watchdog",
            "running": false,
            "reason": "dashboard_disabled",
            "host": cfg.host,
            "port": cfg.port,
        }));
        return 0;
    }
    let _ = fs::write(
        dashboard_watchdog_pid_path(root),
        format!("{}\n", std::process::id()),
    );
    append_watchdog_log(
        root,
        &json!({
            "ok": true,
            "type": "dashboard_watchdog",
            "event": "started",
            "pid": std::process::id(),
            "host": cfg.host,
            "port": cfg.port,
            "interval_ms": cfg.watchdog_interval_ms,
            "fail_streak_threshold": DASHBOARD_WATCHDOG_FAIL_STREAK_THRESHOLD,
            "node_binary": cfg.node_binary,
        }),
    );
    let mut fail_streak = 0usize;
    let mut last_health: Option<bool> = None;
    loop {
        if dashboard_stop_latch_active(root) {
            if dashboard_desired_state_active(root) {
                clear_dashboard_stop_latch(root);
                append_watchdog_log(
                    root,
                    &json!({
                        "ok": true,
                        "type": "dashboard_watchdog",
                        "event": "stop_latch_cleared",
                        "reason": "desired_state_active",
                    }),
                );
            } else {
                break;
            }
        }
        let healthy = dashboard_health_ok(cfg.host.as_str(), cfg.port);
        let dashboard_pid = read_pid_file(&dashboard_pid_path(root));
        let listener_pids = dashboard_listener_pids(cfg.port);
        let process_active =
            dashboard_pid.map(pid_running).unwrap_or(false) || !listener_pids.is_empty();
        if last_health != Some(healthy) {
            append_watchdog_log(
                root,
                &json!({
                    "ok": true,
                    "type": "dashboard_watchdog",
                    "event": "health_transition",
                    "healthy": healthy,
                    "fail_streak": fail_streak,
                    "dashboard_pid": dashboard_pid,
                    "listeners": listener_pids,
                    "process_active": process_active,
                }),
            );
            last_health = Some(healthy);
        }
        if healthy {
            fail_streak = 0;
        } else {
            fail_streak = fail_streak.saturating_add(1);
        }
        let restart_threshold = if !healthy && !process_active {
            1usize
        } else {
            DASHBOARD_WATCHDOG_FAIL_STREAK_THRESHOLD
        };
        if fail_streak >= restart_threshold {
            append_watchdog_log(
                root,
                &json!({
                    "ok": true,
                    "type": "dashboard_watchdog",
                    "event": "restart_triggered",
                    "fail_streak": fail_streak,
                    "restart_threshold": restart_threshold,
                    "process_active": process_active,
                }),
            );
            let restarted = restart_dashboard_for_watchdog(root, &cfg);
            append_watchdog_log(
                root,
                &json!({
                    "ok": true,
                    "type": "dashboard_watchdog",
                    "event": "restart_result",
                    "payload": restarted,
                }),
            );
            if restarted.get("running").and_then(Value::as_bool) == Some(true) {
                fail_streak = 0;
            } else {
                std::thread::sleep(Duration::from_millis(1_500));
            }
        }
        std::thread::sleep(Duration::from_millis(cfg.watchdog_interval_ms));
    }
    let _ = fs::remove_file(dashboard_watchdog_pid_path(root));
    append_watchdog_log(
        root,
        &json!({
            "ok": true,
            "type": "dashboard_watchdog",
            "event": "stopped",
            "reason": "stop_latch",
            "host": cfg.host,
            "port": cfg.port,
        }),
    );
    print_json_line(&json!({
        "ok": true,
        "type": "dashboard_watchdog",
        "running": false,
        "reason": "stop_latch",
        "host": cfg.host,
        "port": cfg.port,
    }));
    0
}

fn ensure_gateway_supervisor(
    root: &Path,
    cfg: &DashboardLaunchConfig,
    dashboard: &mut Value,
    force_refresh: bool,
) -> Value {
    if !cfg.persistent_supervisor {
        let _ = gateway_supervisor::disable(root);
        return gateway_supervisor::status(root).payload;
    }
    let existing = gateway_supervisor::status(root);
    let existing_healthy = supervisor_payload_healthy(&existing.payload);
    let supervisor = if should_refresh_supervisor(force_refresh, existing_healthy) {
        gateway_supervisor_enable(root, cfg)
    } else {
        existing
    };
    let supervisor_healthy = supervisor_payload_healthy(&supervisor.payload);
    if !supervisor_healthy {
        let fallback = spawn_dashboard_watchdog(root, cfg);
        dashboard["watchdog_fallback"] = match fallback {
            Ok(pid) => json!({
                "ok": true,
                "running": true,
                "pid": pid,
                "mode": "local_watchdog_fallback",
            }),
            Err(err) => json!({
                "ok": false,
                "running": false,
                "error": err,
                "mode": "local_watchdog_fallback",
            }),
        };
    }
    supervisor.payload
}

fn supervisor_payload_running(supervisor: &Value) -> bool {
    supervisor
        .get("running")
        .and_then(Value::as_bool)
        .or_else(|| supervisor.get("active").and_then(Value::as_bool))
        .unwrap_or(false)
}

fn supervisor_payload_healthy(supervisor: &Value) -> bool {
    supervisor
        .get("ok")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        && supervisor
            .get("active")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        && supervisor_payload_running(supervisor)
}

fn should_refresh_supervisor(force_refresh: bool, supervisor_healthy: bool) -> bool {
    force_refresh || !supervisor_healthy
}
