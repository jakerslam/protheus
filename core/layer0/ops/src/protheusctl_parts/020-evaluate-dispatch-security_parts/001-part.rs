fn tier1_route_contract_mismatches() -> Vec<String> {
    let mut out = Vec::<String>::new();
    for row in crate::command_list_kernel::tier1_route_contracts() {
        let rest = row
            .rest
            .iter()
            .map(|token| token.to_string())
            .collect::<Vec<_>>();
        if !route_integrity_ok(row.cmd, &rest, row.expected_script) {
            out.push(format!(
                "{} {} -> expected {}",
                row.cmd,
                row.rest.join(" "),
                row.expected_script
            ));
        }
    }
    out
}

fn root_cause_code_for_issue(issue: &str) -> &'static str {
    match issue {
        "wrapper_missing" => "INF-INSTALL-001-WRAPPER-MISSING",
        "runtime_assets_missing" => "INF-INSTALL-002-RUNTIME-ASSETS-MISSING",
        "command_registry_integrity_failed" => "INF-REGISTRY-001-INTEGRITY-FAILED",
        "tier1_route_contract_failed" => "INF-REGISTRY-002-TIER1-ROUTE-MISMATCH",
        "tier1_runtime_targets_missing" => "INF-REGISTRY-003-TIER1-RUNTIME-MISSING",
        "dashboard_route_mismatch" => "INF-ROUTE-001-DASHBOARD-ROUTE-MISMATCH",
        "verify_install_route_mismatch" => "INF-ROUTE-002-VERIFY-ROUTE-MISMATCH",
        "gateway_status_route_mismatch" => "INF-ROUTE-003-GATEWAY-ROUTE-MISMATCH",
        "node_runtime_missing" => "INF-RUNTIME-001-NODE-MISSING",
        "node_module_typescript_missing" => "INF-RUNTIME-002-TYPESCRIPT-MISSING",
        "node_module_ws_missing" => "INF-RUNTIME-003-WS-MISSING",
        "cargo_not_runnable" => "INF-RUST-001-CARGO-NOT-RUNNABLE",
        "rustup_default_toolchain_missing" => "INF-RUST-002-RUSTUP-DEFAULT-MISSING",
        "dashboard_port_invalid" => "INF-DASH-001-PORT-INVALID",
        "dashboard_healthz_unreachable" => "INF-DASH-002-HEALTHZ-UNREACHABLE",
        "dashboard_pid_not_running" => "INF-DASH-003-PID-NOT-RUNNING",
        "dashboard_watchdog_not_running" => "INF-DASH-004-WATCHDOG-NOT-RUNNING",
        "launchd_not_loaded" => "INF-DASH-005-LAUNCHD-NOT-LOADED",
        "stale_workspace_root_reference" => "INF-RUNTIME-004-STALE-WORKSPACE-ROOT",
        "dashboard_ui_route_mismatch" => "INF-ROUTE-004-DASHBOARD-UI-LEGACY-MISMATCH",
        _ => "INF-UNKNOWN-000-UNCLASSIFIED",
    }
}

fn collect_root_cause_codes(failures: &[String], warnings: &[String]) -> Vec<String> {
    let mut out = Vec::<String>::new();
    for issue in failures.iter().chain(warnings.iter()) {
        let code = root_cause_code_for_issue(issue.as_str()).to_string();
        if !out.contains(&code) {
            out.push(code);
        }
    }
    out
}

fn recovery_commands_for_issue(issue: &str) -> &'static [&'static str] {
    match issue {
        "node_runtime_missing" => &[
            "Install Node.js 22+ then rerun: infring setup --yes --defaults",
            "If PATH did not refresh: . \"$HOME/.infring/env.sh\" && hash -r 2>/dev/null || true",
        ],
        "node_module_typescript_missing" | "node_module_ws_missing" => &[
            "Repair runtime closure: infring update --repair --full",
            "Re-run diagnostics: infring doctor --json",
        ],
        "cargo_not_runnable" => &[
            "Install Rust toolchain and set default: rustup default stable",
            "Re-run diagnostics: infring doctor --json",
        ],
        "rustup_default_toolchain_missing" => &[
            "Configure default Rust toolchain: rustup default stable",
            "Verify toolchain: cargo --version",
        ],
        "dashboard_port_invalid" => &[
            "Use a valid dashboard port and retry: infring gateway restart --dashboard-port=4173",
            "Inspect status: infring gateway status",
        ],
        "dashboard_healthz_unreachable" | "dashboard_pid_not_running" | "dashboard_watchdog_not_running" => &[
            "Restart gateway and dashboard: infring gateway restart",
            "Validate health endpoint: curl -fsS http://127.0.0.1:4173/healthz",
        ],
        "stale_workspace_root_reference" => &[
            "Set active root for this workspace: export INFRING_WORKSPACE_ROOT=\"$(pwd)\"",
            "Re-run diagnostics: infringctl doctor --json",
        ],
        "runtime_assets_missing" | "tier1_runtime_targets_missing" => &[
            "Repair runtime assets: infring update --repair --full",
            "Verify required runtime manifest: client/runtime/config/install_runtime_manifest_v1.txt",
        ],
        "wrapper_missing" => &[
            "Re-run installer in repair mode: curl -fsSL https://raw.githubusercontent.com/protheuslabs/InfRing/main/install.sh | sh -s -- --repair --full",
            "Check wrappers directly: \"$HOME/.infring/bin/infring\" --help",
        ],
        "dashboard_route_mismatch" | "verify_install_route_mismatch" | "gateway_status_route_mismatch" => &[
            "Re-run installer repair to restore route wrappers: infring update --repair --full",
            "Validate route contracts: infring verify-install --json",
        ],
        _ => &[],
    }
}

fn collect_recovery_hints(failures: &[String], warnings: &[String]) -> Vec<(String, Vec<String>)> {
    let mut out = Vec::<(String, Vec<String>)>::new();
    let mut seen = Vec::<String>::new();
    for issue in failures.iter().chain(warnings.iter()) {
        if seen.contains(issue) {
            continue;
        }
        seen.push(issue.clone());
        let commands = recovery_commands_for_issue(issue.as_str());
        if commands.is_empty() {
            continue;
        }
        out.push((
            issue.clone(),
            commands.iter().map(|row| row.to_string()).collect::<Vec<_>>(),
        ));
    }
    out
}

fn print_recovery_hints(rows: &[(String, Vec<String>)]) {
    if rows.is_empty() {
        return;
    }
    println!("[infring doctor] recovery-hints:");
    for (issue, commands) in rows {
        println!("  - {}:", clean(issue, 120));
        for cmd in commands {
            println!("      * {}", clean(cmd, 260));
        }
    }
}

fn env_flag_true(name: &str) -> bool {
    std::env::var(name)
        .ok()
        .map(|raw| {
            let normalized = raw.trim().to_ascii_lowercase();
            matches!(normalized.as_str(), "1" | "true" | "yes" | "on")
        })
        .unwrap_or(false)
}

fn node_module_resolvable(root: &Path, module_name: &str) -> bool {
    if !has_node_runtime() {
        return false;
    }
    let module_literal = serde_json::to_string(module_name).unwrap_or_else(|_| "\"\"".to_string());
    let probe = format!(
        "try{{require.resolve({module_literal});process.exit(0);}}catch(_e){{process.exit(1);}}"
    );
    Command::new(node_bin())
        .arg("-e")
        .arg(probe)
        .current_dir(root)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn cargo_detected() -> bool {
    Command::new("cargo")
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok()
}

fn cargo_runnable() -> bool {
    Command::new("cargo")
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn rustup_detected() -> bool {
    Command::new("rustup")
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok()
}

fn rustup_default_toolchain_configured() -> bool {
    Command::new("rustup")
        .arg("default")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn normalized_install_toolchain_policy() -> String {
    let raw = env::var("INFRING_INSTALL_TOOLCHAIN_POLICY").unwrap_or_else(|_| "auto".to_string());
    match raw.trim().to_ascii_lowercase().as_str() {
        "fail" | "fail_closed" | "strict" => "fail_closed".to_string(),
        _ => "auto".to_string(),
    }
}

fn wrapper_candidate_path(wrapper_name: &str) -> String {
    let file_name = if cfg!(windows) {
        format!("{wrapper_name}.cmd")
    } else {
        wrapper_name.to_string()
    };
    if let Ok(exe) = env::current_exe() {
        if let Some(parent) = exe.parent() {
            return clean(parent.join(file_name).to_string_lossy().to_string(), 500);
        }
    }
    clean(file_name, 500)
}

fn resolve_executable_path(bin_name: &str) -> Option<String> {
    let path_var = env::var_os("PATH")?;
    let mut candidates = vec![bin_name.to_string()];
    if cfg!(windows) {
        for ext in [".exe", ".cmd", ".bat"] {
            candidates.push(format!("{bin_name}{ext}"));
        }
    }
    for dir in env::split_paths(&path_var) {
        for candidate_name in &candidates {
            let candidate = dir.join(candidate_name);
            if candidate.is_file() {
                return Some(clean(candidate.to_string_lossy().to_string(), 500));
            }
        }
    }
    None
}

fn runtime_manifest_status(root: &Path, runtime_mode: &str, missing_entrypoints_count: usize) -> Value {
    let manifest_path = root.join(INSTALL_RUNTIME_MANIFEST_REL);
    let manifest_raw = std::fs::read_to_string(&manifest_path).ok();
    let declared_entry_count = manifest_raw
        .as_deref()
        .map(|raw| {
            raw.lines()
                .map(str::trim)
                .filter(|line| !line.is_empty() && !line.starts_with('#'))
                .count()
        })
        .unwrap_or(0usize);
    json!({
        "manifest_rel": INSTALL_RUNTIME_MANIFEST_REL,
        "manifest_path": clean(manifest_path.to_string_lossy().to_string(), 500),
        "manifest_exists": manifest_path.is_file(),
        "runtime_mode": runtime_mode,
        "declared_entry_count": declared_entry_count,
        "missing_entrypoints_count": missing_entrypoints_count
    })
}

fn port_availability_status(host: &str, port: Option<u16>) -> Value {
    if let Some(port_value) = port {
        let bind_target = format!("{host}:{port_value}");
        match std::net::TcpListener::bind(bind_target.as_str()) {
            Ok(listener) => {
                drop(listener);
                json!({
                    "host": host,
                    "port": port_value,
                    "parse_ok": true,
                    "bind_available": true,
                    "status": "available"
                })
            }
            Err(err) => {
                let status = if err.kind() == std::io::ErrorKind::AddrInUse {
                    "in_use"
                } else {
                    "unavailable"
                };
                json!({
                    "host": host,
                    "port": port_value,
                    "parse_ok": true,
                    "bind_available": false,
                    "status": status,
                    "error_kind": format!("{:?}", err.kind()),
                    "error": clean(err.to_string(), 220)
                })
            }
        }
    } else {
        json!({
            "host": host,
            "parse_ok": false,
            "bind_available": false,
            "status": "invalid_port"
        })
    }
}

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
    let install_toolchain_policy = normalized_install_toolchain_policy();
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
    let wrappers = json!({
        "infring": command_available_in_current_bin_dir("infring"),
        "infringctl": command_available_in_current_bin_dir("infringctl"),
        "infringd": command_available_in_current_bin_dir("infringd")
    });
    let binary_paths = json!({
        "infring_wrapper": wrapper_candidate_path("infring"),
        "infringctl_wrapper": wrapper_candidate_path("infringctl"),
        "infringd_wrapper": wrapper_candidate_path("infringd"),
        "node_runtime": resolve_executable_path(node_bin()),
        "cargo": resolve_executable_path("cargo"),
        "rustup": resolve_executable_path("rustup")
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
    let dashboard_ui_compat_enabled = env_flag_true("INFRING_ENABLE_DASHBOARD_UI_ALIAS");
    let dashboard_ui_route_ok = if dashboard_ui_compat_enabled {
        route_integrity_ok(
            "dashboard-ui",
            &["status".to_string()],
            "core://daemon-control",
        )
    } else {
        true
    };
    let verify_route_ok = route_integrity_ok("verify-install", &[], "core://install-doctor");
    let gateway_status_route_ok =
        route_integrity_ok("gateway", &["status".to_string()], "core://daemon-control");
    let dashboard_host =
        parse_flag_value(args, "dashboard-host").unwrap_or_else(|| "127.0.0.1".to_string());
    let dashboard_port_raw =
        parse_flag_value(args, "dashboard-port").unwrap_or_else(|| "4173".to_string());
    let dashboard_port = dashboard_port_raw.parse::<u16>().ok();
    let port_availability = port_availability_status(&dashboard_host, dashboard_port);
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
    let dashboard_probe_status = json!({
        "healthz_reachable": dashboard_healthz_reachable,
        "execution_mode": dashboard_execution_mode,
        "dashboard_pid_running": dashboard_pid_running,
        "watchdog_running": dashboard_watchdog_pid_running || core_watchdog_pid_running,
        "launchd_loaded": launchd_loaded
    });
    let module_closure_status = json!({
        "required_modules": ["typescript", "ws"],
        "node_runtime_detected": node_detected,
        "typescript_module_resolved": typescript_module_resolved,
        "ws_module_resolved": ws_module_resolved,
        "all_required_resolved": node_detected && typescript_module_resolved && ws_module_resolved
    });
    let runtime_manifest_state =
        runtime_manifest_status(root, runtime_mode.as_str(), missing_runtime.len());

    let checks = json!({
        "runtime_mode": runtime_mode,
        "node_runtime_detected": node_detected,
        "typescript_module_resolved": typescript_module_resolved,
        "ws_module_resolved": ws_module_resolved,
        "binary_paths": binary_paths,
        "runtime_manifest_state": runtime_manifest_state,
        "module_closure_status": module_closure_status,
        "dashboard_probe_status": dashboard_probe_status,
        "port_availability": port_availability,
        "toolchains": {
            "cargo_detected": cargo_detected,
            "cargo_runnable": cargo_runnable,
            "rustup_detected": rustup_detected,
            "rustup_default_toolchain_configured": rustup_default_toolchain_configured,
            "install_toolchain_policy": install_toolchain_policy
        },
        "command_registry_ok": command_registry_ok,
        "command_registry": command_registry_integrity,
        "tier1_route_mismatches": tier1_route_mismatches,
        "tier1_runtime_targets": tier1_runtime_targets,
        "tier1_runtime_missing": tier1_runtime_missing,
        "runtime_assets_missing": missing_runtime.len(),
        "wrappers_ok": wrappers_ok,
        "dashboard_route_ok": dashboard_route_ok,
        "dashboard_ui_compat_enabled": dashboard_ui_compat_enabled,
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
    if dashboard_ui_compat_enabled && !dashboard_ui_route_ok {
        warnings.push("dashboard_ui_route_mismatch".to_string());
    }
    if !verify_route_ok {
        failures.push("verify_install_route_mismatch".to_string());
    }
    if !gateway_status_route_ok {
        failures.push("gateway_status_route_mismatch".to_string());
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
            "[infring doctor] toolchains: cargo-detected={} cargo-runnable={} rustup-detected={} rustup-default={} install-policy={}",
            cargo_detected,
            cargo_runnable,
            rustup_detected,
            rustup_default_toolchain_configured,
            install_toolchain_policy
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
        println!(
            "[infring doctor] route integrity: dashboard={}, dashboard-ui(compat:{})={}, gateway-status={}, verify-install={}",
            dashboard_route_ok,
            dashboard_ui_compat_enabled,
            dashboard_ui_route_ok,
            gateway_status_route_ok,
            verify_route_ok
        );
        println!(
            "[infring doctor] process: healthz={}, dashboard-pid-running={}, watchdog-running={}, launchd-loaded={}",
            dashboard_healthz_reachable,
            dashboard_pid_running,
            dashboard_watchdog_pid_running || core_watchdog_pid_running,
            launchd_loaded
