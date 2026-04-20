
pub fn run(root: &Path, argv: &[String]) -> i32 {
    let workspace_root = effective_workspace_root(root);
    let root = workspace_root.as_path();
    let mut skip_setup_flag = false;
    let mut global_json = false;
    let mut global_quiet = false;
    let mut global_help = false;
    let mut global_version = false;
    let mut global_example = false;
    let mut filtered_argv = Vec::new();
    for arg in argv {
        match arg.as_str() {
            "--skip-setup" => skip_setup_flag = true,
            "--json" | "--json=1" => global_json = true,
            "--quiet" | "--quiet=1" => global_quiet = true,
            "--help" | "-h" => global_help = true,
            "--version" => global_version = true,
            "--example" => global_example = true,
            _ => filtered_argv.push(arg.clone()),
        }
    }

    if global_json {
        env::set_var("PROTHEUS_GLOBAL_JSON", "1");
    }
    if global_quiet {
        env::set_var("PROTHEUS_GLOBAL_QUIET", "1");
    }

    let mut cmd = if filtered_argv.is_empty() {
        if global_version {
            "version".to_string()
        } else if global_help {
            "help".to_string()
        } else {
            let force_repl = bool_env("PROTHEUS_FORCE_REPL", false);
            let repl_disabled = bool_env("PROTHEUS_REPL_DISABLED", false);
            if !repl_disabled && (force_repl || std::io::stdin().is_terminal()) {
                if should_offer_setup(root, skip_setup_flag) {
                    let setup_route = Route {
                        script_rel: SETUP_WIZARD_SCRIPT.to_string(),
                        args: vec!["run".to_string()],
                        forward_stdin: true,
                    };
                    let setup_gate = evaluate_dispatch_security(
                        root,
                        &setup_route.script_rel,
                        &setup_route.args,
                    );
                    if !setup_gate.ok {
                        eprintln!(
                            "{}",
                            json!({
                                "ok": false,
                                "type": "protheusctl_dispatch_security_gate",
                                "error": setup_gate.reason
                            })
                        );
                        return 1;
                    }
                    let setup_status = run_node_script(
                        root,
                        &setup_route.script_rel,
                        &setup_route.args,
                        setup_route.forward_stdin,
                    );
                    if setup_status != 0 {
                        return setup_status;
                    }
                }
                "repl".to_string()
            } else {
                "status".to_string()
            }
        }
    } else {
        filtered_argv
            .first()
            .cloned()
            .unwrap_or_else(|| "status".to_string())
    };
    let mut rest = filtered_argv.iter().skip(1).cloned().collect::<Vec<_>>();

    if global_version {
        cmd = "version".to_string();
        rest.clear();
    }

    if cmd == "kairos" {
        cmd = "proactive_daemon".to_string();
    }
    if cmd.eq_ignore_ascii_case("dashboard-ui") {
        let compat_enabled = dashboard_ui_compat_enabled(&rest);
        rest = strip_dashboard_ui_compat_flags(rest);
        if compat_enabled {
            cmd = "dashboard".to_string();
        } else {
            eprintln!(
                "{}",
                json!({
                    "ok": false,
                    "type": "protheusctl_dispatch",
                    "error": "legacy_dashboard_alias_disabled",
                    "root_cause_code": "INF-ROUTE-004-DASHBOARD-UI-LEGACY-MISMATCH",
                    "message": "dashboard-ui alias is disabled by default",
                    "next_step": "use `infring dashboard` or enable compatibility with INFRING_ENABLE_DASHBOARD_UI_ALIAS=1"
                })
            );
            return 1;
        }
    }

    if global_help
        && !matches!(cmd.as_str(), "help" | "--help" | "-h")
        && !rest
            .iter()
            .any(|arg| matches!(arg.as_str(), "--help" | "-h"))
    {
        rest.push("--help".to_string());
    }

    if global_example && !matches!(cmd.as_str(), "examples" | "demo") {
        let target = cmd.clone();
        cmd = "examples".to_string();
        rest = vec![target];
    }

    maybe_run_update_checker(root, &cmd, global_json);
    maybe_run_cli_suggestion_engine(root, &cmd, &rest, global_json);

