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
