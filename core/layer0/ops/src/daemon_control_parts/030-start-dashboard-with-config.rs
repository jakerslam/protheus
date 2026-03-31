fn start_dashboard_with_config(
    root: &Path,
    cfg: &DashboardLaunchConfig,
    allow_browser: bool,
    spawn_local_watchdog: bool,
) -> Value {
    let url = cfg.url();
    if !cfg.enabled {
        return json!({
            "enabled": false,
            "running": dashboard_health_ok(cfg.host.as_str(), cfg.port),
            "opened_browser": false,
            "url": url
        });
    }
    if dashboard_health_ok(cfg.host.as_str(), cfg.port) {
        return json!({
            "enabled": true,
            "running": true,
            "launched": false,
            "opened_browser": false,
            "url": url,
            "ready_timeout_ms": cfg.ready_timeout_ms
        });
    }

    let wait_attempts = dashboard_wait_attempts(&cfg);
    let first_spawn = spawn_dashboard(root, &cfg);
    let mut running = wait_for_dashboard_stable(
        cfg.host.as_str(),
        cfg.port,
        wait_attempts,
        DASHBOARD_WATCHDOG_STABLE_RETRIES,
    );
    let mut launched = first_spawn.is_ok();
    let mut pid = first_spawn.as_ref().ok().copied();
    let mut recovery_attempted = false;
    let mut recovery = Value::Null;

    if !running {
        recovery_attempted = true;
        let stopped = kill_dashboard_process(root, &cfg);
        let second_spawn = spawn_dashboard(root, &cfg);
        running = wait_for_dashboard_stable(
            cfg.host.as_str(),
            cfg.port,
            wait_attempts,
            DASHBOARD_WATCHDOG_STABLE_RETRIES,
        );
        launched = second_spawn.is_ok();
        pid = second_spawn.as_ref().ok().copied();
        let mut recovery_payload = json!({
            "attempted": true,
            "stopped": stopped,
            "launched": second_spawn.is_ok()
        });
        if let Err(err) = second_spawn {
            recovery_payload["error"] = Value::String(err);
        }
        recovery = recovery_payload;
    }

    let mut out = json!({
        "enabled": true,
        "running": running,
        "launched": launched,
        "pid": pid,
        "opened_browser": false,
        "url": url,
        "node_binary": cfg.node_binary,
        "log_path": dashboard_log_path(root).to_string_lossy().to_string(),
        "ready_timeout_ms": cfg.ready_timeout_ms,
        "recovery_attempted": recovery_attempted,
        "recovery": recovery
    });
    if allow_browser && cfg.open_browser && running {
        out["opened_browser"] = Value::Bool(open_browser(cfg.url().as_str()));
    }
    if let Err(err) = first_spawn {
        out["spawn_error"] = Value::String(err);
    }
    if !running {
        out["error"] = Value::String("dashboard_healthz_not_ready".to_string());
        let tail = dashboard_log_tail(root, 8);
        if !tail.is_empty() {
            out["log_tail"] = Value::String(tail);
        }
    }
    if running && spawn_local_watchdog {
        let watchdog = spawn_dashboard_watchdog(root, cfg);
        out["watchdog"] = match watchdog {
            Ok(pid) => json!({
                "running": true,
                "pid": pid,
                "interval_ms": cfg.watchdog_interval_ms,
                "log_path": dashboard_watchdog_log_path(root).to_string_lossy().to_string(),
            }),
            Err(err) => json!({
                "running": false,
                "error": err,
                "interval_ms": cfg.watchdog_interval_ms,
                "log_path": dashboard_watchdog_log_path(root).to_string_lossy().to_string(),
            }),
        };
    } else {
        out["watchdog"] = dashboard_watchdog_status(root);
    }
    out
}

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
        if last_health != Some(healthy) {
            append_watchdog_log(
                root,
                &json!({
                    "ok": true,
                    "type": "dashboard_watchdog",
                    "event": "health_transition",
                    "healthy": healthy,
                    "fail_streak": fail_streak,
                    "dashboard_pid": read_pid_file(&dashboard_pid_path(root)),
                }),
            );
            last_health = Some(healthy);
        }
        if healthy {
            fail_streak = 0;
        } else {
            fail_streak = fail_streak.saturating_add(1);
        }
        if fail_streak >= DASHBOARD_WATCHDOG_FAIL_STREAK_THRESHOLD {
            append_watchdog_log(
                root,
                &json!({
                    "ok": true,
                    "type": "dashboard_watchdog",
                    "event": "restart_triggered",
                    "fail_streak": fail_streak,
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
) -> Value {
    if !cfg.persistent_supervisor {
        let _ = gateway_supervisor::disable(root);
        return gateway_supervisor::status(root).payload;
    }
    let supervisor = gateway_supervisor_enable(root, cfg);
    let dashboard_running = dashboard
        .get("running")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if !supervisor.active && dashboard_running {
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
    println!("  protheus-ops daemon-control <start|stop|restart|status|attach|subscribe|tick|diagnostics|drift-status|watchdog> [--mode=<value>]");
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
pub fn run(root: &Path, argv: &[String]) -> i32 {
    let command = argv
        .first()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());
    if matches!(command.as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }
    let mode = parse_mode(argv)
        .or_else(|| std::env::var("PROTHEUSD_DEFAULT_COMMAND").ok())
        .filter(|value| !value.trim().is_empty());
    if command == "watchdog" {
        return run_dashboard_watchdog(root, argv);
    }
    if command == "drift-status" {
        print_json_line(&verity_drift_status_receipt(root, argv));
        return 0;
    }
    if matches!(
        command.as_str(),
        "start" | "stop" | "restart" | "status" | "attach" | "subscribe" | "tick" | "diagnostics"
    ) {
        let mut receipt = success_receipt(command.as_str(), mode.as_deref(), argv, root);
        let dashboard = match command.as_str() {
            "start" => {
                let cfg = parse_dashboard_launch_config(argv, "start");
                set_dashboard_desired_state(root, cfg.enabled);
                if cfg.enabled {
                    clear_dashboard_stop_latch(root);
                } else {
                    set_dashboard_stop_latch(root);
                }
                if cfg.persistent_supervisor {
                    let _ = stop_dashboard_watchdog(root);
                }
                let mut started =
                    start_dashboard_with_config(root, &cfg, true, !cfg.persistent_supervisor);
                started["supervisor"] = ensure_gateway_supervisor(root, &cfg, &mut started);
                started
            }
            "restart" => {
                let cfg = parse_dashboard_launch_config(argv, "restart");
                set_dashboard_desired_state(root, cfg.enabled);
                set_dashboard_stop_latch(root);
                let supervisor_stopped = gateway_supervisor::disable(root);
                let watchdog_stopped = stop_dashboard_watchdog(root);
                let stopped = kill_dashboard_process(root, &cfg);
                if cfg.enabled {
                    clear_dashboard_stop_latch(root);
                }
                let mut started =
                    start_dashboard_with_config(root, &cfg, true, !cfg.persistent_supervisor);
                let supervisor = ensure_gateway_supervisor(root, &cfg, &mut started);
                json!({
                    "supervisor_stopped": supervisor_stopped.payload,
                    "watchdog_stopped": watchdog_stopped,
                    "stopped": stopped,
                    "supervisor": supervisor,
                    "started": started
                })
            }
            "stop" => {
                set_dashboard_desired_state(root, false);
                set_dashboard_stop_latch(root);
                let cfg = parse_dashboard_launch_config(argv, "stop");
                let supervisor_stopped = gateway_supervisor::disable(root);
                let watchdog_stopped = stop_dashboard_watchdog(root);
                let stopped = kill_dashboard_process(root, &cfg);
                json!({
                    "supervisor_stopped": supervisor_stopped.payload,
                    "watchdog_stopped": watchdog_stopped,
                    "stopped": stopped
                })
            }
            "status" => {
                let cfg = parse_dashboard_launch_config(argv, "start");
                json!({
                    "enabled": cfg.enabled,
                    "persistent_supervisor": cfg.persistent_supervisor,
                    "running": dashboard_health_ok(cfg.host.as_str(), cfg.port),
                    "url": cfg.url(),
                    "log_path": dashboard_log_path(root).to_string_lossy().to_string(),
                    "stop_latch_active": dashboard_stop_latch_active(root),
                    "desired_active": dashboard_desired_state_active(root),
                    "watchdog": dashboard_watchdog_status(root),
                    "supervisor": gateway_supervisor::status(root).payload,
                })
            }
            _ => json!({}),
        };
        receipt["dashboard"] = dashboard;
        receipt["receipt_hash"] = Value::String(deterministic_receipt_hash(&receipt));
        print_json_line(&receipt);
        return 0;
    }

    usage();
    print_json_line(&error_receipt("unknown_command", argv));
    2
}
