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
                    script_rel: "client/runtime/systems/migration/kernel_migration_bridge.js"
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
