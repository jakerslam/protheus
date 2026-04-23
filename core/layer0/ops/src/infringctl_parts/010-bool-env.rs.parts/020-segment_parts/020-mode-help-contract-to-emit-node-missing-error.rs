
fn mode_help_contract(
    mode: &str,
) -> (
    Vec<&'static str>,
    &'static str,
    Vec<(&'static str, &'static str)>,
) {
    match mode {
        "pure" | "tiny-max" => (
            vec![
                "infring help",
                "infring setup",
                "infring setup status --json",
                "infring gateway status",
                "infring doctor --json",
            ],
            "constrained_mode_optional_rich_surfaces_limited",
            vec![
                (
                    "infring dashboard",
                    "constrained_mode_optional_dashboard_surfaces_limited",
                ),
                (
                    "infring gateway start --dashboard-open=1",
                    "constrained_mode_optional_dashboard_surfaces_limited",
                ),
                (
                    "infring assimilate <target> ...",
                    "node_runtime_and_full_mode_required",
                ),
            ],
        ),
        "minimal" => (
            vec![
                "infring help",
                "infring setup",
                "infring setup status --json",
                "infring gateway",
                "infring gateway status",
                "infring doctor --json",
            ],
            "minimal_mode_operator_surface_requires_explicit_setup_on_some_hosts",
            vec![
                (
                    "infring dashboard",
                    "minimal_mode_optional_dashboard_requires_explicit_opt_in",
                ),
                (
                    "infring assimilate <target> ...",
                    "node_runtime_and_full_mode_required",
                ),
            ],
        ),
        _ => (
            vec![
                "infring help",
                "infring setup",
                "infring setup status --json",
                "infring gateway",
                "infring gateway status",
                "infring dashboard",
                "infring doctor --json",
            ],
            "full_mode_complete_operator_surface",
            vec![(
                "infring assimilate <target> ...",
                "node_runtime_required_for_full_surface",
            )],
        ),
    }
}

fn strip_status_dashboard_tokens(args: Vec<String>) -> Vec<String> {
    let mut filtered = Vec::<String>::new();
    for arg in args {
        let token = arg.trim().to_ascii_lowercase();
        if matches!(token.as_str(), "--dashboard" | "dashboard" | "--web") {
            continue;
        }
        filtered.push(arg);
    }
    filtered
}

fn print_node_free_command_list(mode: &str) {
    let install_mode = declared_install_mode();
    let (dashboard_surface, capability_reason) = mode_capability_reason(install_mode.as_str());
    let (mode_valid_commands, mode_help_reason, mode_unavailable_actions) =
        mode_help_contract(install_mode.as_str());
    if mode == "help" {
        usage();
        println!();
        println!("Node.js is not available, so full JS command help is unavailable.");
    } else {
        println!("Command list (Node-free fallback):");
    }
    println!(
        "Mode contract: mode={}, dashboard_surface={}, reason={}",
        install_mode, dashboard_surface, capability_reason
    );
    println!("Mode help reason: {}", mode_help_reason);
    println!("Mode-valid commands:");
    for cmd in mode_valid_commands {
        println!("  - {cmd}");
    }
    if !mode_unavailable_actions.is_empty() {
        println!();
        println!("Mode-unavailable actions:");
        for (command, reason) in mode_unavailable_actions {
            println!("  - {command} ({reason})");
        }
    }
    println!();
    println!("Unavailable until full mode + Node.js 22+:");
    println!("  - infring assimilate <target> ...");
    println!();
    println!("Install Node.js 22+ to unlock all CLI commands.");
    println!("Suggested install command: {}", node_install_command_hint());
    println!("Tip: rerun installer with --install-node to attempt automatic installation.");
    println!("Deterministic recovery path:");
    println!("  1) infring setup --yes --defaults");
    println!("  2) infring setup status --json");
    println!("  3) infring gateway status");
    println!("  4) infring doctor --json");
    let root = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let missing_runtime = runtime_missing_entrypoints(&effective_workspace_root(&root));
    if !missing_runtime.is_empty() {
        println!();
        println!(
            "Runtime assets also appear incomplete (manifest: {INSTALL_RUNTIME_MANIFEST_REL}):"
        );
        for rel in missing_runtime.iter().take(8) {
            println!("  - missing: {rel}");
        }
        if missing_runtime.len() > 8 {
            println!("  - ... {} more", missing_runtime.len() - 8);
        }
        println!("Run `infring doctor --json` for a full install integrity report.");
    }
}

fn emit_node_missing_error(root: &Path, cmd: &str, script_rel: &str) -> i32 {
    let install_hint = node_install_command_hint();
    let install_mode = declared_install_mode();
    let (dashboard_surface, capability_reason) = mode_capability_reason(install_mode.as_str());
    let missing_runtime = runtime_missing_entrypoints(root);
    let runtime_assets_missing = !missing_runtime.is_empty();
    eprintln!(
        "{}",
        json!({
            "ok": false,
            "type": "infringctl_dispatch",
            "error": "node_runtime_missing",
            "command": clean(cmd, 80),
            "script_rel": clean(script_rel, 220),
            "hint": clean(format!("Install Node.js 22+ (try: {install_hint}) or set INFRING_NODE_BINARY to a valid node executable."), 220),
            "node_install_command": clean(install_hint, 220),
            "path_reload_command": ". \"$HOME/.infring/env.sh\" && hash -r 2>/dev/null || true",
            "install_mode": install_mode,
            "mode_dashboard_surface": dashboard_surface,
            "mode_capability_reason": capability_reason,
            "auto_install_hint": "Rerun installer with --install-node to attempt automatic Node installation.",
            "setup_retry_command": "infring setup --yes --defaults",
            "setup_status_command": "infring setup status --json",
            "gateway_status_command": "infring gateway status",
            "doctor_command": "infring doctor --json",
            "runtime_assets_missing": runtime_assets_missing,
            "runtime_manifest_rel": INSTALL_RUNTIME_MANIFEST_REL,
            "missing_runtime_entrypoints": missing_runtime
        })
    );
    1
}
