
fn run_install_doctor_domain(root: &Path, args: &[String]) -> i32 {
    let json_mode = has_json_flag(args);
    let mode = first_positional_command(args);
    let normalized_mode = if mode.is_empty() {
        "doctor".to_string()
    } else {
        mode
    };
    let runtime_mode = resolved_runtime_mode(root);
    let node_detected = has_node_runtime();
    let typescript_module_resolved = node_detected && node_module_resolvable(root, "typescript");
    let ws_module_resolved = node_detected && node_module_resolvable(root, "ws");
    let cargo_detected = cargo_detected();
    let cargo_runnable = cargo_runnable();
    let rustup_detected = rustup_detected();
    let rustup_default_toolchain_configured = if rustup_detected {
        rustup_default_toolchain_configured()
    } else {
        false
    };
    let command_registry_integrity = crate::command_list_kernel::command_registry_integrity();
    let command_registry_ok = command_registry_integrity
        .get("ok")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let tier1_route_mismatches = tier1_route_contract_mismatches();
    let tier1_runtime_targets = crate::command_list_kernel::tier1_runtime_entrypoints();
    let tier1_runtime_missing = tier1_runtime_targets
        .iter()
        .filter_map(|rel| {
            if script_exists_with_ts_js_fallback(root, rel) {
                None
            } else {
                Some((*rel).to_string())
            }
        })
        .collect::<Vec<_>>();
    let missing_runtime = runtime_missing_entrypoints_for_mode(root, runtime_mode.as_str());
    let required_asset_misses = missing_runtime.len() + tier1_runtime_missing.len();
    let stale_path_references = workspace_root_env_mismatches(root);
    let wrappers = json!({
        "infring": command_available_in_current_bin_dir("infring"),
        "infringctl": command_available_in_current_bin_dir("infringctl"),
        "infringd": command_available_in_current_bin_dir("infringd")
    });
    let wrappers_ok = wrappers
        .as_object()
        .map(|map| map.values().all(|v| v.as_bool().unwrap_or(false)))
        .unwrap_or(false);
    let dashboard_route_ok = route_integrity_ok(
        "dashboard",
        &["status".to_string()],
        "core://daemon-control",
    );
    let dashboard_ui_route_ok = route_integrity_ok(
        "dashboard-ui",
        &["status".to_string()],
        "core://daemon-control",
    );
    let verify_route_ok = route_integrity_ok("verify-install", &[], "core://install-doctor");
    let gateway_status_route_ok =
        route_integrity_ok("gateway", &["status".to_string()], "core://daemon-control");
    let dashboard_host =
        parse_flag_value(args, "dashboard-host").unwrap_or_else(|| "127.0.0.1".to_string());
    let dashboard_port_raw =
        parse_flag_value(args, "dashboard-port").unwrap_or_else(|| "4173".to_string());
    let dashboard_port = dashboard_port_raw.parse::<u16>().ok();
    let dashboard_pid_file = dashboard_pid_file(&dashboard_host, &dashboard_port_raw);
    let dashboard_pid = read_pid_file(&dashboard_pid_file);
    let dashboard_pid_running = dashboard_pid.map(process_running).unwrap_or(false);
    let dashboard_watchdog_pid_file =
        dashboard_watchdog_pid_file(&dashboard_host, &dashboard_port_raw);
    let dashboard_watchdog_pid = read_pid_file(&dashboard_watchdog_pid_file);
    let dashboard_watchdog_pid_running =
        dashboard_watchdog_pid.map(process_running).unwrap_or(false);
    let core_watchdog_pid_file = root
        .join("local")
        .join("state")
        .join("ops")
        .join("daemon_control")
        .join("dashboard_watchdog.pid");
    let core_watchdog_pid = read_pid_file(&core_watchdog_pid_file);
    let core_watchdog_pid_running = core_watchdog_pid.map(process_running).unwrap_or(false);
    let dashboard_healthz_reachable = dashboard_port
        .map(|port| dashboard_healthz_reachable(&dashboard_host, port, 450))
        .unwrap_or(false);
    let launchd_loaded = launchd_dashboard_loaded();
    let dashboard_execution_mode = if dashboard_healthz_reachable {
        if dashboard_watchdog_pid_running || core_watchdog_pid_running {
            "watchdog_managed"
        } else if dashboard_pid_running {
            "dashboard_pid_managed"
        } else {
            "manual_foreground"
        }
    } else if dashboard_watchdog_pid_running || core_watchdog_pid_running {
        "watchdog_starting"
    } else if dashboard_pid_running {
        "dashboard_pid_only"
    } else {
        "not_running"
    };
    let process_checks = json!({
        "dashboard_host": dashboard_host,
        "dashboard_port": dashboard_port,
        "dashboard_port_raw": clean(dashboard_port_raw, 32),
        "dashboard_healthz_reachable": dashboard_healthz_reachable,
        "dashboard_execution_mode": dashboard_execution_mode,
        "dashboard_pid_file": clean(dashboard_pid_file.to_string_lossy().to_string(), 500),
        "dashboard_pid": dashboard_pid,
        "dashboard_pid_running": dashboard_pid_running,
        "dashboard_watchdog_pid_file": clean(dashboard_watchdog_pid_file.to_string_lossy().to_string(), 500),
        "dashboard_watchdog_pid": dashboard_watchdog_pid,
        "dashboard_watchdog_pid_running": dashboard_watchdog_pid_running,
        "core_watchdog_pid_file": clean(core_watchdog_pid_file.to_string_lossy().to_string(), 500),
        "core_watchdog_pid": core_watchdog_pid,
        "core_watchdog_pid_running": core_watchdog_pid_running,
        "launchd_loaded": launchd_loaded,
        "launchd_label": "com.protheuslabs.infring.dashboard.shelltest2"
    });

    let checks = json!({
        "runtime_mode": runtime_mode,
        "node_runtime_detected": node_detected,
        "typescript_module_resolved": typescript_module_resolved,
        "ws_module_resolved": ws_module_resolved,
        "toolchains": {
            "cargo_detected": cargo_detected,
            "cargo_runnable": cargo_runnable,
            "rustup_detected": rustup_detected,
            "rustup_default_toolchain_configured": rustup_default_toolchain_configured
        },
        "command_registry_ok": command_registry_ok,
        "command_registry": command_registry_integrity,
        "tier1_route_mismatches": tier1_route_mismatches,
        "tier1_runtime_targets": tier1_runtime_targets,
        "tier1_runtime_missing": tier1_runtime_missing,
        "runtime_assets_missing": missing_runtime.len(),
        "required_asset_misses": required_asset_misses,
        "stale_path_references": stale_path_references,
        "stale_path_reference_count": stale_path_references.len(),
        "wrappers_ok": wrappers_ok,
        "dashboard_route_ok": dashboard_route_ok,
        "dashboard_ui_route_ok": dashboard_ui_route_ok,
        "verify_route_ok": verify_route_ok,
        "gateway_status_route_ok": gateway_status_route_ok,
        "runtime_manifest_rel": INSTALL_RUNTIME_MANIFEST_REL,
        "process_checks": process_checks
    });

    let mut failures = Vec::<String>::new();
    let mut warnings = Vec::<String>::new();
    if !wrappers_ok {
        failures.push("wrapper_missing".to_string());
    }
    if !missing_runtime.is_empty() {
        failures.push("runtime_assets_missing".to_string());
    }
    if required_asset_misses > 0 {
        failures.push("required_asset_misses_nonzero".to_string());
    }
    if !command_registry_ok {
        failures.push("command_registry_integrity_failed".to_string());
    }
    if !tier1_route_mismatches.is_empty() {
        failures.push("tier1_route_contract_failed".to_string());
    }
    if !tier1_runtime_missing.is_empty() {
        failures.push("tier1_runtime_targets_missing".to_string());
    }
    if !dashboard_route_ok {
        failures.push("dashboard_route_mismatch".to_string());
    }
    if !dashboard_ui_route_ok {
        warnings.push("dashboard_ui_route_mismatch".to_string());
    }
    if !verify_route_ok {
        failures.push("verify_install_route_mismatch".to_string());
    }
    if !gateway_status_route_ok {
        failures.push("gateway_status_route_mismatch".to_string());
    }
    if normalized_mode == "verify-install" && !stale_path_references.is_empty() {
        failures.push("stale_workspace_root_reference".to_string());
    } else if !stale_path_references.is_empty() {
        warnings.push("stale_workspace_root_reference".to_string());
    }
    // Full verification expects Node so all JS/TS command surfaces are actionable.
    if normalized_mode == "verify-install" && !node_detected {
        failures.push("node_runtime_missing".to_string());
    }
    if normalized_mode == "verify-install" && node_detected && !typescript_module_resolved {
        failures.push("node_module_typescript_missing".to_string());
    }
    if normalized_mode == "verify-install" && node_detected && !ws_module_resolved {
        failures.push("node_module_ws_missing".to_string());
    }
    if node_detected && !typescript_module_resolved {
        warnings.push("node_module_typescript_missing".to_string());
    }
    if node_detected && !ws_module_resolved {
        warnings.push("node_module_ws_missing".to_string());
    }
    if cargo_detected && !cargo_runnable {
        warnings.push("cargo_not_runnable".to_string());
    }
    if rustup_detected && !rustup_default_toolchain_configured {
        warnings.push("rustup_default_toolchain_missing".to_string());
    }
    if dashboard_port.is_none() {
        failures.push("dashboard_port_invalid".to_string());
    }
    if !dashboard_healthz_reachable {
        warnings.push("dashboard_healthz_unreachable".to_string());
    }
    if !dashboard_pid_running && !dashboard_healthz_reachable {
        warnings.push("dashboard_pid_not_running".to_string());
    }
    if !dashboard_watchdog_pid_running && !core_watchdog_pid_running && !dashboard_healthz_reachable
    {
        warnings.push("dashboard_watchdog_not_running".to_string());
    }
    if env::consts::OS == "macos"
        && !launchd_loaded
        && matches!(
            dashboard_execution_mode,
            "not_running" | "watchdog_starting"
        )
    {
        warnings.push("launchd_not_loaded".to_string());
    }
    let root_cause_codes = collect_root_cause_codes(&failures, &warnings);
    let recovery_hints = collect_recovery_hints(&failures, &warnings);

    let ok = failures.is_empty();
    if json_mode {
        println!(
            "{}",
            json!({
                "ok": ok,
                "type": "install_doctor",
                "mode": normalized_mode,
                "checks": checks,
                "wrappers": wrappers,
                "missing_runtime_entrypoints": missing_runtime,
                "required_asset_misses": required_asset_misses,
                "stale_path_references": stale_path_references,
                "failures": failures,
                "warnings": warnings,
                "root_cause_codes": root_cause_codes,
                "recovery_hints": recovery_hints
                    .iter()
                    .map(|(issue, commands)| json!({
                        "issue": issue,
                        "commands": commands
                    }))
                    .collect::<Vec<_>>()
            })
        );
    } else {
        println!("[infring doctor] mode: {normalized_mode}");
        println!(
            "[infring doctor] node runtime: {}",
            if node_detected { "detected" } else { "missing" }
        );
        println!(
            "[infring doctor] toolchains: cargo-detected={} cargo-runnable={} rustup-detected={} rustup-default={}",
            cargo_detected,
            cargo_runnable,
            rustup_detected,
            rustup_default_toolchain_configured
        );
        println!(
            "[infring doctor] wrappers: infring={}, infringctl={}, infringd={}",
            wrappers
                .get("infring")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            wrappers
                .get("infringctl")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            wrappers
                .get("infringd")
                .and_then(Value::as_bool)
                .unwrap_or(false)
        );
        println!(
            "[infring doctor] runtime assets missing: {}",
            missing_runtime.len()
        );
        println!(
            "[infring doctor] required asset misses: {}",
            required_asset_misses
        );
        println!(
            "[infring doctor] stale workspace root references: {}",
            stale_path_references.len()
        );
        println!(
            "[infring doctor] command registry: ok={} tier1-route-mismatch={} tier1-runtime-missing={}",
            command_registry_ok,
            tier1_route_mismatches.len(),
            tier1_runtime_missing.len()
        );
        if !missing_runtime.is_empty() {
            for rel in missing_runtime.iter().take(10) {
                println!("  - {rel}");
            }
            if missing_runtime.len() > 10 {
                println!("  - ... {} more", missing_runtime.len() - 10);
            }
        }
        if !tier1_route_mismatches.is_empty() {
            for row in tier1_route_mismatches.iter().take(5) {
                println!("  - tier1 route mismatch: {row}");
            }
        }
        if !tier1_runtime_missing.is_empty() {
            for row in tier1_runtime_missing.iter().take(5) {
                println!("  - tier1 runtime missing: {row}");
            }
        }
        if !stale_path_references.is_empty() {
            for row in stale_path_references.iter().take(5) {
                let env_name = row.get("env").and_then(Value::as_str).unwrap_or("unknown");
                let configured = row
                    .get("configured")
                    .and_then(Value::as_str)
                    .unwrap_or("(unset)");
                let active = row
                    .get("active_workspace_root")
                    .and_then(Value::as_str)
                    .unwrap_or("(unknown)");
                println!(
                    "  - stale workspace root ref: {env_name}={configured} (active={active})"
                );
            }
        }
        println!(
            "[infring doctor] route integrity: dashboard={}, dashboard-ui={}, gateway-status={}, verify-install={}",
            dashboard_route_ok, dashboard_ui_route_ok, gateway_status_route_ok, verify_route_ok
        );
        println!(
            "[infring doctor] process: healthz={}, dashboard-pid-running={}, watchdog-running={}, launchd-loaded={}",
            dashboard_healthz_reachable,
            dashboard_pid_running,
            dashboard_watchdog_pid_running || core_watchdog_pid_running,
            launchd_loaded

        );
        if !warnings.is_empty() {
            println!("[infring doctor] warnings: {}", warnings.join(", "));
        }
        if !recovery_hints.is_empty() {
            println!("[infring doctor] next-actions:");
            for (issue, commands) in recovery_hints.iter().take(5) {
                if let Some(cmd) = commands.first() {
                    println!("  - {} -> {}", clean(issue, 120), clean(cmd, 220));
                }
            }
            println!("  - setup status -> infring setup status --json");
            println!("  - gateway status -> infring gateway status");
        }
        if !root_cause_codes.is_empty() {
            println!(
                "[infring doctor] root-cause-codes: {}",
                root_cause_codes.join(", ")
            );
        }
        print_recovery_hints(&recovery_hints);
        if ok {
            println!("[infring doctor] verdict: ok");
        } else {
            println!("[infring doctor] verdict: failed ({})", failures.join(", "));
        }
    }
    if ok {
        0
    } else {
        2
    }
}
