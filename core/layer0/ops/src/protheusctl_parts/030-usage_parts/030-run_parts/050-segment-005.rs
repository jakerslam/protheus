            "rust-hybrid" => {
                let sub = rest
                    .first()
                    .map(|v| v.trim().to_ascii_lowercase())
                    .unwrap_or_else(|| "status".to_string());
                let normalized = if ["list", "run", "run-all", "status"].contains(&sub.as_str()) {
                    sub
                } else {
                    "status".to_string()
                };
                Route {
                    script_rel: "client/runtime/systems/ops/rust_hybrid_migration_program.js"
                        .to_string(),
                    args: std::iter::once(normalized)
                        .chain(rest.into_iter().skip(1))
                        .collect(),
                    forward_stdin: false,
                }
            }
            "suite" => {
                let sub = rest
                    .first()
                    .map(|v| v.trim().to_ascii_lowercase())
                    .unwrap_or_else(|| "status".to_string());
                let normalized = if ["list", "run", "run-all", "status"].contains(&sub.as_str()) {
                    sub
                } else {
                    "status".to_string()
                };
                Route {
                    script_rel: "client/runtime/systems/ops/productized_suite_program.js"
                        .to_string(),
                    args: std::iter::once(normalized)
                        .chain(rest.into_iter().skip(1))
                        .collect(),
                    forward_stdin: false,
                }
            }
            "rsi" => Route {
                script_rel: "client/cognition/adaptive/rsi/rsi_bootstrap.js".to_string(),
                args: if rest.is_empty() {
                    vec!["status".to_string()]
                } else {
                    rest
                },
                forward_stdin: false,
            },
            "contract-lane" if rest.first().map(String::as_str) == Some("status") => Route {
                script_rel: "client/cognition/adaptive/rsi/rsi_bootstrap.js".to_string(),
                args: std::iter::once("contract-lane-status".to_string())
                    .chain(rest.into_iter().skip(1))
                    .collect(),
                forward_stdin: false,
            },
            "approve" if rest.iter().any(|arg| arg == "--rsi") => Route {
                script_rel: "client/cognition/adaptive/rsi/rsi_bootstrap.js".to_string(),
                args: std::iter::once("approve".to_string())
                    .chain(rest.into_iter().filter(|arg| arg != "--rsi"))
                    .collect(),
                forward_stdin: false,
            },
            _ => Route {
                script_rel: "core://unknown-command".to_string(),
                args: std::iter::once(cmd.clone()).chain(rest).collect(),
                forward_stdin: false,
            },
        })
    };

    let supports_json_flag = matches!(
        route.script_rel.as_str(),
        "core://install-doctor"
            | "core://daemon-control"
            | "core://verity-plane"
            | "core://command-list"
            | "core://completion"
            | "core://version-cli"
            | "core://release-semver-contract"
            | "client/runtime/systems/ops/protheus_command_list.ts"
            | "client/runtime/systems/ops/protheus_command_list.js"
            | "client/runtime/systems/ops/protheus_setup_wizard.js"
            | "client/runtime/systems/ops/protheus_status_dashboard.ts"
            | "client/runtime/systems/ops/protheus_debug_diagnostics.ts"
            | "client/runtime/systems/personas/shadow_cli.ts"
            | "client/runtime/systems/tools/cli_suggestion_engine_bridge.ts"
    ) || [
        SETUP_WIZARD_SCRIPT,
        DEMO_SCRIPT,
        EXAMPLES_SCRIPT,
        VERSION_SCRIPT_JS,
        DIAGRAM_SCRIPT,
        COMPLETION_SCRIPT_JS,
    ]
    .contains(&route.script_rel.as_str());
    if global_json
        && supports_json_flag
        && !route
            .args
            .iter()
            .any(|arg| arg == "--json" || arg.starts_with("--json="))
    {
        route.args.push("--json=1".to_string());
    }
    if global_json
        && route.script_rel == "core://unknown-command"
        && !route
            .args
            .iter()
            .any(|arg| arg == "--json" || arg.starts_with("--json="))
    {
        route.args.push("--json=1".to_string());
    }

    let supports_quiet_flag = matches!(
        route.script_rel.as_str(),
        "core://version-cli" | "core://release-semver-contract"
    ) || [DEMO_SCRIPT, EXAMPLES_SCRIPT, VERSION_SCRIPT_JS]
        .contains(&route.script_rel.as_str());
    if global_quiet
        && supports_quiet_flag
        && !route
            .args
            .iter()
            .any(|arg| arg == "--quiet" || arg.starts_with("--quiet="))
    {
        route.args.push("--quiet=1".to_string());
    }

    if let Err(payload) = command_route_preflight(root, &cmd, &route) {
        eprintln!("{}", payload);
        return 2;
    }

    if let Err(reason) = enforce_command_center_boundary(&cmd, &route) {
        eprintln!(
            "{}",
            json!({
                "ok": false,
                "type": "protheusctl_boundary_guard",
                "error": clean(reason, 220),
                "command": cmd,
                "script_rel": route.script_rel
            })
        );
        return 1;
    }

    if !route.script_rel.starts_with("core://") && !has_node_runtime() {
        if let Some(status) = node_missing_fallback(root, &route, global_json) {
            return status;
        }
        return emit_node_missing_error(root, &cmd, &route.script_rel);
    }

    let gate = evaluate_dispatch_security(root, &route.script_rel, &route.args);
    if !gate.ok {
        eprintln!(
            "{}",
            json!({
                "ok": false,
                "type": "protheusctl_dispatch_security_gate",
                "error": gate.reason
            })
        );
        return 1;
    }

