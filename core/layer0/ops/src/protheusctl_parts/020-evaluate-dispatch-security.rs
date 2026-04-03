pub fn evaluate_dispatch_security(
    root: &Path,
    script_rel: &str,
    args: &[String],
) -> DispatchSecurity {
    if bool_env("PROTHEUS_CTL_SECURITY_GATE_DISABLED", false) {
        return DispatchSecurity {
            ok: true,
            reason: "protheusctl_dispatch_gate_disabled".to_string(),
        };
    }

    let workspace_root = effective_workspace_root(root);
    let req = security_request(&workspace_root, script_rel, args);
    let persona_gate = evaluate_persona_dispatch_security(script_rel, args, &req);
    if !persona_gate.ok {
        return persona_gate;
    }
    if req
        .get("covenant_violation")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        || req
            .get("tamper_signal")
            .and_then(Value::as_bool)
            .unwrap_or(false)
    {
        return DispatchSecurity {
            ok: false,
            reason: "security_gate_blocked:local_fail_closed_signal".to_string(),
        };
    }

    let request_json = serde_json::to_string(&req).unwrap_or_else(|_| "{}".to_string());
    let request_base64 = BASE64_STANDARD.encode(request_json.as_bytes());

    let payload = match evaluate_security_decision_payload(&workspace_root, &req, &request_base64) {
        Ok(value) => value,
        Err(reason) => {
            return DispatchSecurity {
                ok: false,
                reason: format!("security_gate_blocked:{}", clean(reason, 220)),
            };
        }
    };

    let decision = payload.get("decision").cloned().unwrap_or(Value::Null);
    let ok = decision.get("ok").and_then(Value::as_bool).unwrap_or(false);
    let fail_closed = decision
        .get("fail_closed")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    if !ok || fail_closed {
        let reason = decision
            .get("reasons")
            .and_then(Value::as_array)
            .and_then(|rows| rows.first())
            .and_then(Value::as_str)
            .unwrap_or("dispatch_security_gate_blocked")
            .to_string();
        return DispatchSecurity {
            ok: false,
            reason: format!("security_gate_blocked:{}", clean(reason, 220)),
        };
    }

    DispatchSecurity {
        ok: true,
        reason: "ok".to_string(),
    }
}

fn evaluate_security_decision_payload(
    workspace_root: &Path,
    req: &Value,
    request_base64: &str,
) -> Result<Value, String> {
    match evaluate_security_decision_embedded(req) {
        Ok(payload) => Ok(payload),
        Err(embedded_error) => {
            if bool_env("PROTHEUS_CTL_SECURITY_DISABLE_CARGO_FALLBACK", false) {
                return Err(format!("embedded_checker_failed:{embedded_error}"));
            }
            match evaluate_security_decision_via_cargo(workspace_root, request_base64) {
                Ok(payload) => Ok(payload),
                Err(cargo_error) => Err(format!(
                    "embedded_checker_failed:{embedded_error}; cargo_fallback_failed:{cargo_error}"
                )),
            }
        }
    }
}

fn evaluate_security_decision_embedded(req: &Value) -> Result<Value, String> {
    let request_json = serde_json::to_string(req).map_err(|err| clean(err.to_string(), 220))?;
    let payload_json = protheus_security_core_v1::evaluate_operation_json(&request_json)
        .map_err(|err| clean(err.to_string(), 220))?;
    parse_json(&payload_json).ok_or_else(|| "invalid_security_payload".to_string())
}

fn evaluate_security_decision_via_cargo(
    workspace_root: &Path,
    request_base64: &str,
) -> Result<Value, String> {
    let manifest = workspace_root.join("core/layer0/security/Cargo.toml");
    if !manifest.exists() {
        return Err("manifest_missing".to_string());
    }

    let output = Command::new("cargo")
        .arg("run")
        .arg("--quiet")
        .arg("--manifest-path")
        .arg(manifest)
        .arg("--bin")
        .arg("security_core")
        .arg("--")
        .arg("check")
        .arg(format!("--request-base64={request_base64}"))
        .current_dir(workspace_root)
        .output()
        .map_err(|_| "spawn_failed".to_string())?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let msg = if stderr.trim().is_empty() {
            stdout.to_string()
        } else {
            stderr.to_string()
        };
        return Err(clean(msg, 220));
    }

    parse_json(&String::from_utf8_lossy(&output.stdout))
        .ok_or_else(|| "invalid_security_payload".to_string())
}

fn run_node_script(root: &Path, script_rel: &str, args: &[String], forward_stdin: bool) -> i32 {
    let workspace_root = effective_workspace_root(root);
    let runtime_mode = resolved_runtime_mode(&workspace_root);
    if let Some(domain) = script_rel.strip_prefix("core://") {
        return run_core_domain(&workspace_root, domain, args, forward_stdin);
    }

    let mut script_abs = workspace_root.join(script_rel);
    if !script_abs.exists() && script_rel.ends_with(".js") {
        let ts_rel = format!("{}{}", script_rel.trim_end_matches(".js"), ".ts");
        let ts_abs = workspace_root.join(&ts_rel);
        if ts_abs.exists() {
            if runtime_mode == "dist" {
                eprintln!(
                    "{}",
                    json!({
                        "ok": false,
                        "type": "protheusctl_dispatch",
                        "error": "dist_source_mismatch",
                        "detail": "runtime_mode=dist requires bundled JS entrypoints; source-only TS fallback detected",
                        "script_rel": clean(script_rel, 220),
                        "script_abs": clean(script_abs.to_string_lossy().to_string(), 500),
                        "ts_candidate_rel": ts_rel,
                        "ts_candidate_exists": true,
                        "runtime_mode": runtime_mode,
                        "node_runtime_detected": has_node_runtime(),
                        "route_found": true
                    })
                );
                return 1;
            }
            script_abs = ts_abs;
        }
    }
    if !script_abs.exists() {
        let synthetic_route = Route {
            script_rel: script_rel.to_string(),
            args: args.to_vec(),
            forward_stdin,
        };
        if let Some(status) = node_missing_fallback(&workspace_root, &synthetic_route, false) {
            return status;
        }
        if matches!(
            script_rel,
            "client/runtime/systems/ops/protheus_setup_wizard.ts"
                | "client/runtime/systems/ops/protheus_setup_wizard.js"
        ) {
            return run_setup_wizard_missing_script_fallback(&workspace_root, args);
        }
        let ts_candidate_rel = if script_rel.ends_with(".js") {
            Some(format!("{}{}", script_rel.trim_end_matches(".js"), ".ts"))
        } else {
            None
        };
        let ts_candidate_exists = ts_candidate_rel
            .as_ref()
            .map(|rel| workspace_root.join(rel).exists())
            .unwrap_or(false);
        let script_missing_kind =
            if runtime_mode == "dist" && script_rel.ends_with(".js") && ts_candidate_exists {
                "dist_source_mismatch"
            } else {
                "script_missing"
            };
        let detail = if script_missing_kind == "dist_source_mismatch" {
            "runtime_mode=dist requires bundled JS entrypoints; source-only TS fallback detected"
        } else {
            "resolved route target script is missing from workspace runtime"
        };
        eprintln!(
            "{}",
            json!({
                "ok": false,
                "type": "protheusctl_dispatch",
                "error": script_missing_kind,
                "detail": detail,
                "script_rel": clean(script_rel, 220),
                "script_abs": clean(script_abs.to_string_lossy().to_string(), 500),
                "ts_candidate_rel": ts_candidate_rel,
                "ts_candidate_exists": ts_candidate_exists,
                "runtime_mode": runtime_mode,
                "node_runtime_detected": has_node_runtime(),
                "route_found": true
            })
        );
        return 1;
    }

    let ts_entrypoint = workspace_root.join("client/runtime/lib/ts_entrypoint.ts");
    let script_is_ts = script_abs
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("ts"))
        .unwrap_or(false);

    let mut cmd = Command::new(node_bin());
    if script_is_ts && ts_entrypoint.exists() {
        cmd.arg(ts_entrypoint).arg(&script_abs);
    } else {
        cmd.arg(&script_abs);
    }

    cmd.args(args)
        .current_dir(workspace_root)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    if forward_stdin {
        cmd.stdin(Stdio::inherit());
    } else {
        cmd.stdin(Stdio::null());
    }

    match cmd.status() {
        Ok(status) => status.code().unwrap_or(1),
        Err(err) => {
            eprintln!(
                "{}",
                json!({
                    "ok": false,
                    "type": "protheusctl_dispatch",
                    "error": clean(format!("spawn_failed:{err}"), 220)
                })
            );
            1
        }
    }
}

fn run_setup_wizard_missing_script_fallback(root: &Path, args: &[String]) -> i32 {
    let state_path = root
        .join("local")
        .join("state")
        .join("ops")
        .join("protheus_setup_wizard")
        .join("latest.json");
    let payload = json!({
        "type": "protheus_setup_wizard_state",
        "completed": true,
        "completed_at": crate::now_iso(),
        "completion_mode": "missing_script_fallback",
        "node_runtime_detected": has_node_runtime(),
        "interaction_style": "silent",
        "notifications": "none",
        "covenant_acknowledged": false,
        "version": 1
    });
    if let Some(parent) = state_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(raw) = serde_json::to_string_pretty(&payload) {
        let _ = std::fs::write(state_path, raw);
    }
    let json_mode = args
        .iter()
        .any(|arg| arg == "--json" || arg == "--json=1");
    if json_mode {
        println!(
            "{}",
            json!({
                "ok": true,
                "type": "protheus_setup_wizard_fallback",
                "mode": "missing_script_fallback",
                "message": "setup wizard script missing in this runtime; wrote fallback state and continued"
            })
        );
    } else {
        println!("Setup wizard script missing in this runtime; applied compatibility fallback.");
        println!("You can rerun `infring setup --force` after updating your runtime.");
    }
    0
}

fn has_json_flag(args: &[String]) -> bool {
    args.iter()
        .any(|arg| arg == "--json" || arg == "--json=1")
}

fn first_positional_command(args: &[String]) -> String {
    for token in args {
        let trimmed = token.trim();
        if trimmed.is_empty() || trimmed.starts_with('-') {
            continue;
        }
        return trimmed.to_string();
    }
    String::new()
}

fn run_unknown_command_domain(args: &[String]) -> i32 {
    let json_mode = has_json_flag(args);
    let command = first_positional_command(args);
    if json_mode {
        println!(
            "{}",
            json!({
                "ok": false,
                "type": "protheusctl_dispatch",
                "error": "unknown_command",
                "command": clean(command, 120),
                "hint": "Run `infring help` to list available commands."
            })
        );
    } else if command.is_empty() {
        eprintln!("[infring] unknown command");
        print_node_free_command_list("help");
    } else {
        eprintln!("[infring] unknown command: {command}");
        print_node_free_command_list("help");
    }
    2
}

fn command_available_in_current_bin_dir(name: &str) -> bool {
    let Ok(exe) = env::current_exe() else {
        return false;
    };
    let Some(dir) = exe.parent() else {
        return false;
    };
    dir.join(name).exists()
}

fn route_integrity_ok(cmd: &str, rest: &[String], expected_script: &str) -> bool {
    resolve_core_shortcuts(cmd, rest)
        .map(|route| route.script_rel == expected_script)
        .unwrap_or(false)
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
    let missing_runtime = runtime_missing_entrypoints_for_mode(root, runtime_mode.as_str());
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
        "dashboard-ui",
        &[
            "serve".to_string(),
            "--host=127.0.0.1".to_string(),
            "--port=4173".to_string(),
        ],
        "core://daemon-control",
    );
    let verify_route_ok = route_integrity_ok("verify-install", &[], "core://install-doctor");
    let gateway_status_route_ok =
        route_integrity_ok("gateway", &["status".to_string()], "core://daemon-control");
    let dashboard_host = parse_flag_value(args, "dashboard-host")
        .unwrap_or_else(|| "127.0.0.1".to_string());
    let dashboard_port_raw =
        parse_flag_value(args, "dashboard-port").unwrap_or_else(|| "4173".to_string());
    let dashboard_port = dashboard_port_raw.parse::<u16>().ok();
    let dashboard_pid_file = dashboard_pid_file(&dashboard_host, &dashboard_port_raw);
    let dashboard_pid = read_pid_file(&dashboard_pid_file);
    let dashboard_pid_running = dashboard_pid.map(process_running).unwrap_or(false);
    let dashboard_watchdog_pid_file = dashboard_watchdog_pid_file(&dashboard_host, &dashboard_port_raw);
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
    let process_checks = json!({
        "dashboard_host": dashboard_host,
        "dashboard_port": dashboard_port,
        "dashboard_port_raw": clean(dashboard_port_raw, 32),
        "dashboard_healthz_reachable": dashboard_healthz_reachable,
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
        "runtime_assets_missing": missing_runtime.len(),
        "wrappers_ok": wrappers_ok,
        "dashboard_route_ok": dashboard_route_ok,
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
    if !dashboard_route_ok {
        failures.push("dashboard_route_mismatch".to_string());
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
    if dashboard_port.is_none() {
        failures.push("dashboard_port_invalid".to_string());
    }
    if !dashboard_healthz_reachable {
        warnings.push("dashboard_healthz_unreachable".to_string());
    }
    if !dashboard_pid_running {
        warnings.push("dashboard_pid_not_running".to_string());
    }
    if !dashboard_watchdog_pid_running && !core_watchdog_pid_running {
        warnings.push("dashboard_watchdog_not_running".to_string());
    }
    if env::consts::OS == "macos" && !launchd_loaded {
        warnings.push("launchd_not_loaded".to_string());
    }

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
                "warnings": warnings
            })
        );
    } else {
        println!("[infring doctor] mode: {normalized_mode}");
        println!("[infring doctor] node runtime: {}", if node_detected { "detected" } else { "missing" });
        println!(
            "[infring doctor] wrappers: infring={}, infringctl={}, infringd={}",
            wrappers.get("infring").and_then(Value::as_bool).unwrap_or(false),
            wrappers.get("infringctl").and_then(Value::as_bool).unwrap_or(false),
            wrappers.get("infringd").and_then(Value::as_bool).unwrap_or(false)
        );
        println!(
            "[infring doctor] runtime assets missing: {}",
            missing_runtime.len()
        );
        if !missing_runtime.is_empty() {
            for rel in missing_runtime.iter().take(10) {
                println!("  - {rel}");
            }
            if missing_runtime.len() > 10 {
                println!("  - ... {} more", missing_runtime.len() - 10);
            }
        }
        println!(
            "[infring doctor] route integrity: dashboard={}, gateway-status={}, verify-install={}",
            dashboard_route_ok, gateway_status_route_ok, verify_route_ok
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

fn run_core_domain(root: &Path, domain: &str, args: &[String], forward_stdin: bool) -> i32 {
    if domain == "unknown-command" {
        return run_unknown_command_domain(args);
    }
    if domain == "install-doctor" {
        return run_install_doctor_domain(root, args);
    }

    let exe = match env::current_exe() {
        Ok(path) => path,
        Err(err) => {
            eprintln!(
                "{}",
                json!({
                    "ok": false,
                    "type": "protheusctl_dispatch",
                    "error": clean(format!("current_exe_failed:{err}"), 220)
                })
            );
            return 1;
        }
    };

    let mut cmd = Command::new(exe);
    cmd.arg(domain)
        .args(args)
        .current_dir(root)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    if forward_stdin {
        cmd.stdin(Stdio::inherit());
    } else {
        cmd.stdin(Stdio::null());
    }

    match cmd.status() {
        Ok(status) => status.code().unwrap_or(1),
        Err(err) => {
            eprintln!(
                "{}",
                json!({
                    "ok": false,
                    "type": "protheusctl_dispatch",
                    "error": clean(format!("core_spawn_failed:{err}"), 220),
                    "domain": domain
                })
            );
            1
        }
    }
}

fn enforce_command_center_boundary(cmd: &str, route: &Route) -> Result<(), String> {
    if route
        .script_rel
        .contains("client/runtime/systems/red_legion/command_center")
    {
        return Err("red_legion_client_authority_forbidden".to_string());
    }
    if cmd == "session"
        && !route
            .script_rel
            .starts_with("core://command-center-session")
    {
        return Err("session_route_must_be_core_authoritative".to_string());
    }
    Ok(())
}

fn maybe_run_cli_suggestion_engine(root: &Path, cmd: &str, rest: &[String]) {
    if bool_env("PROTHEUS_GLOBAL_QUIET", false) {
        return;
    }
    if !bool_env("PROTHEUS_CLI_SUGGESTIONS", true) {
        return;
    }
    if matches!(
        cmd,
        "assimilate"
            | "research"
            | "tutorial"
            | "list"
            | "help"
            | "--help"
            | "-h"
            | "demo"
            | "examples"
            | "version"
            | "update"
            | "diagram"
            | "shadow"
            | "debug"
            | "setup"
            | "completion"
            | "repl"
            | "status"
            | "toolkit"
            | "task"
    ) {
        return;
    }
    let suggestion_script_ts = root.join("client/runtime/systems/tools/cli_suggestion_engine.ts");
    let suggestion_script_js = root.join("client/runtime/systems/tools/cli_suggestion_engine.js");
    let suggestion_script = if suggestion_script_ts.exists() {
        suggestion_script_ts
    } else if suggestion_script_js.exists() {
        suggestion_script_js
    } else {
        return;
    };
    let request_json = serde_json::to_string(&json!({
        "cmd": cmd,
        "args": rest
    }))
    .unwrap_or_else(|_| "{}".to_string());
    let ts_entrypoint = root.join("client/runtime/lib/ts_entrypoint.ts");
    let script_is_ts = suggestion_script
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("ts"))
        .unwrap_or(false);

    let mut proc = Command::new(node_bin());
    if script_is_ts && ts_entrypoint.exists() {
        proc.arg(ts_entrypoint).arg(&suggestion_script);
    } else {
        proc.arg(&suggestion_script);
    }

    let _ = proc
        .arg("suggest")
        .arg("--origin=main_cli")
        .arg(format!("--cmd={}", clean(cmd, 60)))
        .arg(format!("--argv-json={request_json}"))
        .current_dir(root)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status();
}

fn maybe_run_update_checker(root: &Path, cmd: &str) {
    if bool_env("PROTHEUS_GLOBAL_QUIET", false) {
        return;
    }
    if bool_env("PROTHEUS_UPDATE_CHECKER_DISABLED", false) {
        return;
    }
    if matches!(cmd, "version" | "update" | "help" | "--help" | "-h") {
        return;
    }
    let script_js = root.join("client/runtime/systems/ops/protheus_version_cli.js");
    let script_ts = root.join("client/runtime/systems/ops/protheus_version_cli.ts");
    let script = if script_js.exists() {
        script_js
    } else if script_ts.exists() {
        script_ts
    } else {
        return;
    };
    let ts_entrypoint = root.join("client/runtime/lib/ts_entrypoint.ts");
    let script_is_ts = script
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("ts"))
        .unwrap_or(false);

    let mut cmd = Command::new(node_bin());
    if script_is_ts && ts_entrypoint.exists() {
        cmd.arg(ts_entrypoint).arg(&script);
    } else {
        cmd.arg(&script);
    }

    let _ = cmd
        .arg("check-quiet")
        .current_dir(root)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status();
}

fn route_edge(rest: &[String]) -> Route {
    let sub = rest
        .first()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());
    match sub.as_str() {
        "lifecycle" => {
            let action = rest
                .get(1)
                .map(|v| v.trim().to_ascii_lowercase())
                .unwrap_or_else(|| "status".to_string());
            Route {
                script_rel: "client/runtime/systems/edge/mobile_lifecycle_resilience.ts"
                    .to_string(),
                args: std::iter::once(action)
                    .chain(rest.iter().skip(2).cloned())
                    .collect(),
                forward_stdin: false,
            }
        }
        "swarm" => {
            let action = rest
                .get(1)
                .map(|v| v.trim().to_ascii_lowercase())
                .unwrap_or_else(|| "status".to_string());
            Route {
                script_rel: "client/runtime/systems/spawn/mobile_edge_swarm_bridge.ts".to_string(),
                args: std::iter::once(action)
                    .chain(rest.iter().skip(2).cloned())
                    .collect(),
                forward_stdin: false,
            }
        }
        "wrapper" => {
            let action = rest
                .get(1)
                .map(|v| v.trim().to_ascii_lowercase())
                .unwrap_or_else(|| "status".to_string());
            Route {
                script_rel: "client/runtime/systems/ops/mobile_wrapper_distribution_pack.js"
                    .to_string(),
                args: std::iter::once(action)
                    .chain(rest.iter().skip(2).cloned())
                    .collect(),
                forward_stdin: false,
            }
        }
        "benchmark" => {
            let action = rest
                .get(1)
                .map(|v| v.trim().to_ascii_lowercase())
                .unwrap_or_else(|| "status".to_string());
            Route {
                script_rel: "client/runtime/systems/ops/run_protheus_ops.ts".to_string(),
                args: std::iter::once("benchmark-matrix".to_string())
                    .chain(std::iter::once(action))
                    .chain(rest.iter().skip(2).cloned())
                    .collect(),
                forward_stdin: false,
            }
        }
        "top" => Route {
            script_rel: "client/runtime/systems/edge/mobile_ops_top.ts".to_string(),
            args: std::iter::once("status".to_string())
                .chain(rest.iter().skip(1).cloned())
                .collect(),
            forward_stdin: false,
        },
        _ => Route {
            script_rel: "client/runtime/systems/edge/protheus_edge_runtime.ts".to_string(),
            args: std::iter::once(sub)
                .chain(rest.iter().skip(1).cloned())
                .collect(),
            forward_stdin: false,
        },
    }
}

fn resolve_core_shortcuts(cmd: &str, rest: &[String]) -> Option<Route> {
    protheusctl_routes::resolve_core_shortcuts(cmd, rest)
}

fn is_assimilate_wrapper_flag(token: &str) -> bool {
    matches!(token, "--showcase" | "--scaffold-payload" | "--no-prewarm")
        || token.starts_with("--showcase=")
        || token.starts_with("--duration-ms=")
        || token.starts_with("--scaffold-payload=")
        || token.starts_with("--prewarm=")
}

fn split_assimilate_tokens(rest: &[String]) -> (Option<String>, Vec<String>, Vec<String>) {
    let mut target: Option<String> = None;
    let mut core_passthrough = Vec::<String>::new();
    let mut wrapper_flags = Vec::<String>::new();
    for token in rest {
        let trimmed = token.trim();
        if target.is_none() {
            if let Some(value) = trimmed.strip_prefix("--target=") {
                let normalized = value.trim();
                if !normalized.is_empty() {
                    target = Some(normalized.to_string());
                    continue;
                }
            } else if !trimmed.starts_with("--") {
                target = Some(trimmed.to_string());
                continue;
            }
        }
        if is_assimilate_wrapper_flag(trimmed) {
            wrapper_flags.push(token.clone());
        } else {
            core_passthrough.push(token.clone());
        }
    }
    (target, core_passthrough, wrapper_flags)
}
