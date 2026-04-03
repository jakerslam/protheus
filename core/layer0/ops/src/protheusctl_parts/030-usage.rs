fn resolve_assimilate_route(rest: &[String]) -> Route {
    let default_args = if rest.is_empty() {
        vec!["--help".to_string()]
    } else {
        rest.to_vec()
    };
    if rest.is_empty() {
        return Route {
            script_rel: "client/runtime/systems/tools/assimilate.ts".to_string(),
            args: default_args,
            forward_stdin: false,
        };
    }

    let (target, mut core_passthrough, wrapper_flags) = split_assimilate_tokens(rest);
    let Some(target_value) = target else {
        return Route {
            script_rel: "client/runtime/systems/tools/assimilate.ts".to_string(),
            args: default_args,
            forward_stdin: false,
        };
    };
    let mut core_rest = vec![target_value.clone()];
    core_rest.append(&mut core_passthrough);

    let Some(core_route) = resolve_core_shortcuts("assimilate", &core_rest) else {
        return Route {
            script_rel: "client/runtime/systems/tools/assimilate.ts".to_string(),
            args: default_args,
            forward_stdin: false,
        };
    };

    let Some(core_domain) = core_route.script_rel.strip_prefix("core://") else {
        return core_route;
    };
    let encoded_core_args = serde_json::to_string(&core_route.args)
        .map(|raw| BASE64_STANDARD.encode(raw.as_bytes()))
        .unwrap_or_else(|_| BASE64_STANDARD.encode(b"[]"));
    let mut args = vec![
        format!("--target={}", target_value),
        format!("--core-domain={}", core_domain),
        format!("--core-args-base64={}", encoded_core_args),
    ];
    args.extend(wrapper_flags);
    Route {
        script_rel: "client/runtime/systems/tools/assimilate.ts".to_string(),
        args,
        forward_stdin: false,
    }
}

pub fn usage() {
    println!("Usage: infring <command> [flags]");
    println!("Try:");
    println!("  infring gateway");
    println!("  infring dream");
    println!("  infring compact");
    println!("  infring proactive_daemon");
    println!("  infring speculate");
    println!("  infring dashboard");
    println!("  infring task list");
    println!("  infring list");
    println!("  infring --help");
    println!("  infring setup");
}
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
                        script_rel: "client/runtime/systems/ops/protheus_setup_wizard.ts"
                            .to_string(),
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

    maybe_run_update_checker(root, &cmd);
    maybe_run_cli_suggestion_engine(root, &cmd, &rest);

    let mut route = if cmd == "assimilate" {
        resolve_assimilate_route(&rest)
    } else {
        resolve_core_shortcuts(&cmd, &rest).unwrap_or_else(|| match cmd.as_str() {
            "list" => Route {
                script_rel: "client/runtime/systems/ops/protheus_command_list.ts".to_string(),
                args: std::iter::once("--mode=list".to_string())
                    .chain(rest)
                    .collect(),
                forward_stdin: false,
            },
            "completion" => Route {
                script_rel: "client/runtime/systems/ops/protheus_completion.js".to_string(),
                args: if rest.is_empty() {
                    vec!["--help".to_string()]
                } else {
                    rest
                },
                forward_stdin: false,
            },
            "repl" => Route {
                script_rel: "client/runtime/systems/ops/protheus_repl.js".to_string(),
                args: rest,
                forward_stdin: true,
            },
            "setup" => Route {
                script_rel: "client/runtime/systems/ops/protheus_setup_wizard.ts".to_string(),
                args: if rest.is_empty() {
                    vec!["run".to_string()]
                } else {
                    rest
                },
                forward_stdin: true,
            },
            "demo" => Route {
                script_rel: "client/runtime/systems/ops/protheus_demo.js".to_string(),
                args: rest,
                forward_stdin: false,
            },
            "examples" => Route {
                script_rel: "client/runtime/systems/ops/protheus_examples.js".to_string(),
                args: rest,
                forward_stdin: false,
            },
            "version" => Route {
                script_rel: "client/runtime/systems/ops/protheus_version_cli.js".to_string(),
                args: std::iter::once("version".to_string()).chain(rest).collect(),
                forward_stdin: false,
            },
            "update" => Route {
                script_rel: "client/runtime/systems/ops/protheus_version_cli.js".to_string(),
                args: std::iter::once("update".to_string()).chain(rest).collect(),
                forward_stdin: false,
            },
            "diagram" => Route {
                script_rel: "client/runtime/systems/ops/protheus_diagram.js".to_string(),
                args: rest,
                forward_stdin: false,
            },
            "shadow" => Route {
                script_rel: "client/runtime/systems/personas/shadow_cli.ts".to_string(),
                args: if rest.is_empty() {
                    vec!["status".to_string()]
                } else {
                    rest
                },
                forward_stdin: false,
            },
            "help" => Route {
                script_rel: "client/runtime/systems/ops/protheus_command_list.ts".to_string(),
                args: std::iter::once("--mode=help".to_string())
                    .chain(rest)
                    .collect(),
                forward_stdin: false,
            },
            "--help" => Route {
                script_rel: "client/runtime/systems/ops/protheus_command_list.ts".to_string(),
                args: std::iter::once("--mode=help".to_string())
                    .chain(rest)
                    .collect(),
                forward_stdin: false,
            },
            "-h" => Route {
                script_rel: "client/runtime/systems/ops/protheus_command_list.ts".to_string(),
                args: std::iter::once("--mode=help".to_string())
                    .chain(rest)
                    .collect(),
                forward_stdin: false,
            },
            "dashboard" => Route {
                script_rel: "core://daemon-control".to_string(),
                args: std::iter::once("start".to_string()).chain(rest).collect(),
                forward_stdin: false,
            },
            "status" => Route {
                script_rel: "core://daemon-control".to_string(),
                args: std::iter::once("status".to_string())
                    .chain(strip_status_dashboard_tokens(rest))
                    .collect(),
                forward_stdin: false,
            },
            "session" => {
                let sub = rest
                    .first()
                    .map(|v| v.trim().to_ascii_lowercase())
                    .unwrap_or_else(|| "status".to_string());
                let normalized = if [
                    "register",
                    "start",
                    "resume",
                    "attach",
                    "send",
                    "steer",
                    "kill",
                    "terminate",
                    "tail",
                    "inspect",
                    "status",
                    "list",
                ]
                .contains(&sub.as_str())
                {
                    sub
                } else {
                    "status".to_string()
                };
                Route {
                    script_rel: "core://command-center-session".to_string(),
                    args: std::iter::once(normalized)
                        .chain(rest.into_iter().skip(1))
                        .collect(),
                    forward_stdin: false,
                }
            }
            "debug" => Route {
                script_rel: "client/runtime/systems/ops/protheus_debug_diagnostics.ts".to_string(),
                args: rest,
                forward_stdin: false,
            },
            "health" => Route {
                script_rel: "client/runtime/systems/ops/protheus_control_plane.js".to_string(),
                args: std::iter::once("health".to_string()).chain(rest).collect(),
                forward_stdin: false,
            },
            "job-submit" => Route {
                script_rel: "client/runtime/systems/ops/protheus_control_plane.js".to_string(),
                args: std::iter::once("job-submit".to_string())
                    .chain(rest)
                    .collect(),
                forward_stdin: false,
            },
            "protheusctl" => Route {
                script_rel: "client/runtime/systems/ops/protheus_command_list.ts".to_string(),
                args: std::iter::once("--mode=help".to_string())
                    .chain(rest)
                    .collect(),
                forward_stdin: false,
            },
            "skills" if rest.first().map(String::as_str) == Some("discover") => Route {
                script_rel: "client/runtime/systems/ops/protheusctl_skills_discover.js".to_string(),
                args: rest.into_iter().skip(1).collect(),
                forward_stdin: false,
            },
            "edge" => route_edge(&rest),
            "host" => {
                let sub = rest
                    .first()
                    .map(|v| v.trim().to_ascii_lowercase())
                    .unwrap_or_else(|| "status".to_string());
                Route {
                    script_rel: "client/runtime/systems/ops/host_adaptation_operator_surface.js"
                        .to_string(),
                    args: std::iter::once(sub)
                        .chain(rest.into_iter().skip(1))
                        .collect(),
                    forward_stdin: false,
                }
            }
            "socket" => {
                let sub = rest
                    .first()
                    .map(|v| v.trim().to_ascii_lowercase())
                    .unwrap_or_else(|| "status".to_string());
                let args = match sub.as_str() {
                    "list" => std::iter::once("lifecycle".to_string())
                        .chain(std::iter::once("list".to_string()))
                        .chain(rest.into_iter().skip(1))
                        .collect(),
                    "install" | "update" | "test" => std::iter::once("lifecycle".to_string())
                        .chain(std::iter::once(sub))
                        .chain(rest.into_iter().skip(1))
                        .collect(),
                    "admission" | "discover" | "activate" | "status" => std::iter::once(sub)
                        .chain(rest.into_iter().skip(1))
                        .collect(),
                    _ => std::iter::once("status".to_string()).chain(rest).collect(),
                };
                Route {
                    script_rel: "client/runtime/systems/ops/platform_socket_runtime.ts".to_string(),
                    args,
                    forward_stdin: false,
                }
            }
            "mine" => {
                let sub = rest
                    .first()
                    .map(|v| v.trim().to_ascii_lowercase())
                    .unwrap_or_else(|| "dashboard".to_string());
                Route {
                    script_rel: "client/runtime/systems/economy/donor_mining_dashboard.js"
                        .to_string(),
                    args: std::iter::once(sub)
                        .chain(rest.into_iter().skip(1))
                        .collect(),
                    forward_stdin: false,
                }
            }
            "migrate" => {
                let sub = rest
                    .first()
                    .map(|v| v.trim().to_ascii_lowercase())
                    .unwrap_or_default();
                let supported = ["run", "status", "rollback", "help", "--help", "-h"];
                let args = if sub.is_empty()
                    || sub.starts_with("--")
                    || !supported.contains(&sub.as_str())
                {
                    std::iter::once("run".to_string()).chain(rest).collect()
                } else if matches!(sub.as_str(), "help" | "--help" | "-h") {
                    vec!["help".to_string()]
                } else {
                    std::iter::once(sub)
                        .chain(rest.into_iter().skip(1))
                        .collect()
                };
                Route {
                    script_rel: "client/runtime/systems/migration/core_migration_bridge.js"
                        .to_string(),
                    args,
                    forward_stdin: false,
                }
            }
            "import" => {
                let sub = rest
                    .first()
                    .map(|v| v.trim().to_ascii_lowercase())
                    .unwrap_or_default();
                let supported = ["run", "status", "help", "--help", "-h"];
                let args = if sub.is_empty()
                    || sub.starts_with("--")
                    || !supported.contains(&sub.as_str())
                {
                    std::iter::once("run".to_string()).chain(rest).collect()
                } else if matches!(sub.as_str(), "help" | "--help" | "-h") {
                    vec!["help".to_string()]
                } else {
                    std::iter::once(sub)
                        .chain(rest.into_iter().skip(1))
                        .collect()
                };
                Route {
                    script_rel: "client/runtime/systems/migration/universal_importers.js"
                        .to_string(),
                    args,
                    forward_stdin: false,
                }
            }
            "wasi2" => {
                let sub = rest
                    .first()
                    .map(|v| v.trim().to_ascii_lowercase())
                    .unwrap_or_else(|| "status".to_string());
                let normalized = if sub == "run" { "run" } else { "status" };
                Route {
                    script_rel: "client/runtime/systems/ops/wasi2_execution_completeness_gate.js"
                        .to_string(),
                    args: std::iter::once(normalized.to_string())
                        .chain(rest.into_iter().skip(1))
                        .collect(),
                    forward_stdin: false,
                }
            }
            "settle" => {
                let mut sub = rest
                    .first()
                    .map(|v| v.trim().to_ascii_lowercase())
                    .unwrap_or_default();
                let has_revert = rest
                    .iter()
                    .any(|arg| matches!(arg.as_str(), "--revert" | "--revert=1" | "--mode=revert"));
                if has_revert {
                    sub = "revert".to_string();
                }
                let supported = [
                    "list",
                    "run",
                    "run-all",
                    "status",
                    "settle",
                    "revert",
                    "edit-core",
                    "edit-module",
                    "edit",
                ];
                let args = if sub.is_empty()
                    || sub.starts_with("--")
                    || !supported.contains(&sub.as_str())
                {
                    std::iter::once("settle".to_string()).chain(rest).collect()
                } else {
                    std::iter::once(sub)
                        .chain(rest.into_iter().skip(1))
                        .collect()
                };
                Route {
                    script_rel: "client/runtime/systems/ops/settlement_program.js".to_string(),
                    args,
                    forward_stdin: false,
                }
            }
            "edit-core" => Route {
                script_rel: "client/runtime/systems/ops/settlement_program.js".to_string(),
                args: std::iter::once("edit-core".to_string())
                    .chain(rest)
                    .collect(),
                forward_stdin: false,
            },
            "edit" => Route {
                script_rel: "client/runtime/systems/ops/settlement_program.js".to_string(),
                args: if rest.is_empty() {
                    vec!["edit-module".to_string()]
                } else {
                    std::iter::once("edit-module".to_string())
                        .chain(rest)
                        .collect()
                },
                forward_stdin: false,
            },
            "scale" => {
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
                    script_rel: "client/runtime/systems/ops/scale_readiness_program.js".to_string(),
                    args: std::iter::once(normalized)
                        .chain(rest.into_iter().skip(1))
                        .collect(),
                    forward_stdin: false,
                }
            }
            "perception" => {
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
                    script_rel: "client/runtime/systems/ops/perception_polish_program.js"
                        .to_string(),
                    args: std::iter::once(normalized)
                        .chain(rest.into_iter().skip(1))
                        .collect(),
                    forward_stdin: false,
                }
            }
            "fluxlattice" => {
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
                    script_rel: "client/runtime/systems/ops/fluxlattice_program.js".to_string(),
                    args: std::iter::once(normalized)
                        .chain(rest.into_iter().skip(1))
                        .collect(),
                    forward_stdin: false,
                }
            }
            "lensmap" => Route {
                script_rel: "packages/lensmap/lensmap_cli.ts".to_string(),
                args: rest,
                forward_stdin: false,
            },
            "lens" => Route {
                script_rel: "client/runtime/systems/personas/cli.js".to_string(),
                args: rest,
                forward_stdin: true,
            },
            "arbitrate" => Route {
                script_rel: "client/runtime/systems/personas/cli.js".to_string(),
                args: std::iter::once("arbitrate".to_string())
                    .chain(rest)
                    .collect(),
                forward_stdin: true,
            },
            "orchestrate" => Route {
                script_rel: "client/runtime/systems/personas/orchestration.js".to_string(),
                args: if rest.is_empty() {
                    vec!["status".to_string()]
                } else {
                    rest
                },
                forward_stdin: true,
            },
            "persona" => {
                let sub = rest
                    .first()
                    .map(|v| v.trim().to_ascii_lowercase())
                    .unwrap_or_default();
                if sub == "ambient" {
                    Route {
                        script_rel: "client/runtime/systems/personas/ambient_stance.js".to_string(),
                        args: if rest.len() > 1 {
                            rest.into_iter().skip(1).collect()
                        } else {
                            vec!["status".to_string()]
                        },
                        forward_stdin: false,
                    }
                } else {
                    Route {
                        script_rel: "client/runtime/systems/personas/cli.js".to_string(),
                        args: if rest.is_empty() {
                            vec!["--help".to_string()]
                        } else {
                            rest
                        },
                        forward_stdin: true,
                    }
                }
            }
            "assimilate" => Route {
                script_rel: "client/runtime/systems/tools/assimilate.ts".to_string(),
                args: if rest.is_empty() {
                    vec!["--help".to_string()]
                } else {
                    rest
                },
                forward_stdin: false,
            },
            "research" => Route {
                script_rel: "core://research-plane".to_string(),
                args: if rest.is_empty() {
                    vec!["status".to_string()]
                } else {
                    rest
                },
                forward_stdin: false,
            },
            "tutorial" => Route {
                script_rel: "client/runtime/systems/tools/cli_suggestion_engine.ts".to_string(),
                args: if rest.is_empty() {
                    vec!["tutorial".to_string(), "status".to_string()]
                } else {
                    std::iter::once("tutorial".to_string())
                        .chain(rest)
                        .collect()
                },
                forward_stdin: false,
            },
            "toolkit" => Route {
                script_rel: "client/runtime/systems/ops/cognitive_toolkit_cli.ts".to_string(),
                args: if rest.is_empty() {
                    vec!["list".to_string()]
                } else {
                    rest
                },
                forward_stdin: true,
            },
            "spine" => Route {
                script_rel: "client/runtime/systems/spine/spine_safe_launcher.js".to_string(),
                args: if rest.is_empty() {
                    vec!["status".to_string()]
                } else {
                    rest
                },
                forward_stdin: false,
            },
            "hold" => {
                let sub = rest
                    .first()
                    .map(|v| v.trim().to_ascii_lowercase())
                    .unwrap_or_else(|| "status".to_string());
                let normalized =
                    if ["admit", "rehydrate", "simulate", "status"].contains(&sub.as_str()) {
                        sub
                    } else {
                        "status".to_string()
                    };
                Route {
                    script_rel: "client/runtime/systems/autonomy/hold_remediation_engine.js"
                        .to_string(),
                    args: std::iter::once(normalized)
                        .chain(rest.into_iter().skip(1))
                        .collect(),
                    forward_stdin: false,
                }
            }
            "rust" => {
                let sub = rest
                    .first()
                    .map(|v| v.trim().to_ascii_lowercase())
                    .unwrap_or_else(|| "status".to_string());
                let normalized = if ["run", "report", "status"].contains(&sub.as_str()) {
                    sub
                } else {
                    "status".to_string()
                };
                Route {
                    script_rel:
                        "client/runtime/systems/ops/rust_authoritative_microkernel_acceleration.js"
                            .to_string(),
                    args: std::iter::once(normalized)
                        .chain(rest.into_iter().skip(1))
                        .collect(),
                    forward_stdin: false,
                }
            }
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
            | "client/runtime/systems/ops/protheus_command_list.ts"
            | "client/runtime/systems/ops/protheus_command_list.js"
            | "client/runtime/systems/ops/protheus_setup_wizard.ts"
            | "client/runtime/systems/ops/protheus_setup_wizard.js"
            | "client/runtime/systems/ops/protheus_demo.js"
            | "client/runtime/systems/ops/protheus_examples.js"
            | "client/runtime/systems/ops/protheus_version_cli.js"
            | "client/runtime/systems/ops/protheus_diagram.js"
            | "client/runtime/systems/ops/protheus_completion.js"
            | "client/runtime/systems/ops/protheus_status_dashboard.ts"
            | "client/runtime/systems/ops/protheus_debug_diagnostics.ts"
            | "client/runtime/systems/personas/shadow_cli.ts"
            | "client/runtime/systems/tools/cli_suggestion_engine.ts"
    );
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
        "client/runtime/systems/ops/protheus_demo.js"
            | "client/runtime/systems/ops/protheus_examples.js"
            | "client/runtime/systems/ops/protheus_version_cli.js"
    );
    if global_quiet
        && supports_quiet_flag
        && !route
            .args
            .iter()
            .any(|arg| arg == "--quiet" || arg.starts_with("--quiet="))
    {
        route.args.push("--quiet=1".to_string());
    }

    if let Some(expected) = crate::command_list_kernel::tier1_route_contracts()
        .iter()
        .find(|row| row.cmd == cmd)
        .map(|row| row.expected_script)
    {
        if route.script_rel != expected {
            eprintln!(
                "{}",
                json!({
                    "ok": false,
                    "type": "protheusctl_dispatch",
                    "error": "tier1_route_contract_failed",
                    "command": clean(cmd, 120),
                    "expected_script": expected,
                    "resolved_script": route.script_rel
                })
            );
            return 2;
        }
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

    run_node_script(root, &route.script_rel, &route.args, route.forward_stdin)
}
