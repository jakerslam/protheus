    let mut route = if cmd == "assimilate" {
        resolve_assimilate_route(&rest)
    } else {
        resolve_core_shortcuts(&cmd, &rest).unwrap_or_else(|| match cmd.as_str() {
            "list" => Route {
                script_rel: "core://command-list".to_string(),
                args: std::iter::once("--mode=list".to_string())
                    .chain(rest)
                    .collect(),
                forward_stdin: false,
            },
            "completion" => Route {
                script_rel: "core://completion".to_string(),
                args: if rest.is_empty() {
                    vec!["--help".to_string()]
                } else {
                    rest
                },
                forward_stdin: false,
            },
            "repl" => Route {
                script_rel: "core://repl".to_string(),
                args: rest,
                forward_stdin: true,
            },
            "setup" => Route {
                script_rel: SETUP_WIZARD_SCRIPT.to_string(),
                args: if rest.is_empty() {
                    vec!["run".to_string()]
                } else {
                    rest
                },
                forward_stdin: true,
            },
            "demo" => Route {
                script_rel: DEMO_SCRIPT.to_string(),
                args: rest,
                forward_stdin: false,
            },
            "examples" => Route {
                script_rel: EXAMPLES_SCRIPT.to_string(),
                args: rest,
                forward_stdin: false,
            },
            "version" => Route {
                script_rel: "core://version-cli".to_string(),
                args: std::iter::once("version".to_string()).chain(rest).collect(),
                forward_stdin: false,
            },
            "update" => Route {
                script_rel: "core://version-cli".to_string(),
                args: std::iter::once("update".to_string()).chain(rest).collect(),
                forward_stdin: false,
            },
            "release-semver-contract" => Route {
                script_rel: "core://release-semver-contract".to_string(),
                args: if rest.is_empty() {
                    vec!["status".to_string()]
                } else {
                    rest
                },
                forward_stdin: false,
            },
            "diagram" => Route {
                script_rel: DIAGRAM_SCRIPT.to_string(),
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
                script_rel: "core://command-list".to_string(),
                args: std::iter::once("--mode=help".to_string())
                    .chain(rest)
                    .collect(),
                forward_stdin: false,
            },
            "--help" => Route {
                script_rel: "core://command-list".to_string(),
                args: std::iter::once("--mode=help".to_string())
                    .chain(rest)
                    .collect(),
                forward_stdin: false,
            },
            "-h" => Route {
                script_rel: "core://command-list".to_string(),
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
                script_rel: "client/runtime/systems/ops/infring_debug_diagnostics.ts".to_string(),
                args: rest,
                forward_stdin: false,
            },
            "health" => Route {
                script_rel: "core://infring-control-plane".to_string(),
                args: std::iter::once("status".to_string()).chain(rest).collect(),
                forward_stdin: false,
            },
            "job-submit" => Route {
                script_rel: "core://infring-control-plane".to_string(),
                args: std::iter::once("run".to_string()).chain(rest).collect(),
                forward_stdin: false,
            },
            "infringctl" => Route {
                script_rel: "core://command-list".to_string(),
                args: std::iter::once("--mode=help".to_string())
                    .chain(rest)
                    .collect(),
                forward_stdin: false,
            },
            "skills" if rest.first().map(String::as_str) == Some("discover") => Route {
                script_rel: "client/runtime/systems/ops/infringctl_skills_discover.js".to_string(),
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
