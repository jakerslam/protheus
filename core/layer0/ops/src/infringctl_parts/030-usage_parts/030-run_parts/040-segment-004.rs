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
                script_rel: ASSIMILATE_SCRIPT.to_string(),
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
                script_rel: "client/runtime/systems/tools/cli_suggestion_engine_bridge.ts".to_string(),
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
