fn resolve_core_shortcuts_family_daemon(cmd: &str, rest: &[String]) -> Option<Route> {
    match cmd {
        "gateway" => {
            let first = rest.first().map(|value| value.trim().to_ascii_lowercase());
            let (subcommand, passthrough_start_idx) = match first.as_deref() {
                other => parse_daemon_control_subcommand(other, false)
                    .unwrap_or_else(|| ("start".to_string(), 0usize)),
            };
            let mut args = std::iter::once(subcommand.clone())
                .chain(rest.iter().skip(passthrough_start_idx).cloned())
                .collect::<Vec<_>>();
            if matches!(subcommand.as_str(), "start" | "restart")
                && !args.iter().any(|arg| {
                    let token = arg.trim();
                    token == "--gateway-banner"
                        || token == "--gateway-banner=1"
                        || token.starts_with("--gateway-banner=")
                })
            {
                args.push("--gateway-banner=1".to_string());
            }
            Some(Route {
                script_rel: "core://daemon-control".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "dashboard-ui" => Some(route_dashboard_compat(rest, true)),
        "dashboard" => Some(route_dashboard_compat(rest, false)),
        "help" | "--help" | "-h" => Some(Route {
            script_rel: "core://command-list".to_string(),
            args: std::iter::once("--mode=help".to_string())
                .chain(rest.iter().cloned())
                .collect(),
            forward_stdin: false,
        }),
        "stop" => Some(Route {
            script_rel: "core://daemon-control".to_string(),
            args: std::iter::once("stop".to_string())
                .chain(rest.iter().cloned())
                .collect(),
            forward_stdin: false,
        }),
        "restart" => Some(Route {
            script_rel: "core://daemon-control".to_string(),
            args: std::iter::once("restart".to_string())
                .chain(rest.iter().cloned())
                .collect(),
            forward_stdin: false,
        }),
        _ => resolve_core_shortcuts_family_shell(cmd, rest),
    }
}
