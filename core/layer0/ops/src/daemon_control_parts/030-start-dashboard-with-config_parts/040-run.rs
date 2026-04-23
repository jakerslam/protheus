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
        .or_else(|| std::env::var("INFRINGD_DEFAULT_COMMAND").ok())
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
        "start"
            | "stop"
            | "restart"
            | "status"
            | "heal"
            | "attach"
            | "subscribe"
            | "tick"
            | "diagnostics"
    ) {
        let gateway_banner_enabled =
            parse_bool(parse_flag(argv, "gateway-banner").as_deref(), false);
        if matches!(command.as_str(), "start" | "restart") && gateway_banner_enabled {
            println!("P o w e r  T o  T h e  U s e r s");
        }
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
                started["supervisor"] = ensure_gateway_supervisor(root, &cfg, &mut started, false);
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
                let supervisor = ensure_gateway_supervisor(root, &cfg, &mut started, true);
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
                let auto_heal = parse_bool(parse_flag(argv, "auto-heal").as_deref(), true);
                let self_heal = if auto_heal {
                    heal_gateway_runtime(root, &cfg)
                } else {
                    json!({
                        "ok": true,
                        "auto_heal": false,
                        "reason": "disabled_by_flag"
                    })
                };
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
                    "self_heal": self_heal,
                })
            }
            "heal" => {
                let cfg = parse_dashboard_launch_config(argv, "start");
                heal_gateway_runtime(root, &cfg)
            }
            _ => json!({}),
        };
        receipt["dashboard"] = dashboard;
        receipt["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&receipt));
        print_json_line(&receipt);
        return 0;
    }

    usage();
    print_json_line(&error_receipt("unknown_command", argv));
    2
}

#[cfg(test)]
include!("../030-start-dashboard-with-config-tests.rs");
