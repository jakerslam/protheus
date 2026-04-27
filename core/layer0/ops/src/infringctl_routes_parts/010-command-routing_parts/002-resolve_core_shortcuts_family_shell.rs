fn resolve_core_shortcuts_family_shell(cmd: &str, rest: &[String]) -> Option<Route> {
    match cmd {
        "list" => Some(Route {
            script_rel: "core://command-list".to_string(),
            args: std::iter::once("--mode=list".to_string())
                .chain(rest.iter().cloned())
                .collect(),
            forward_stdin: false,
        }),
        "completion" => Some(Route {
            script_rel: "core://completion".to_string(),
            args: if rest.is_empty() {
                vec!["--help".to_string()]
            } else {
                rest.to_vec()
            },
            forward_stdin: false,
        }),
        "repl" => Some(Route {
            script_rel: "core://repl".to_string(),
            args: rest.to_vec(),
            forward_stdin: true,
        }),
        "version" => Some(Route {
            script_rel: "core://version-cli".to_string(),
            args: std::iter::once("version".to_string())
                .chain(rest.iter().cloned())
                .collect(),
            forward_stdin: false,
        }),
        "--version" | "-v" => Some(Route {
            script_rel: "core://version-cli".to_string(),
            args: std::iter::once("version".to_string())
                .chain(rest.iter().cloned())
                .collect(),
            forward_stdin: false,
        }),
        "update" => Some(Route {
            script_rel: "core://version-cli".to_string(),
            args: std::iter::once("update".to_string())
                .chain(rest.iter().cloned())
                .collect(),
            forward_stdin: false,
        }),
        "health" => Some(Route {
            script_rel: "core://infring-control-plane".to_string(),
            args: std::iter::once("status".to_string())
                .chain(rest.iter().cloned())
                .collect(),
            forward_stdin: false,
        }),
        "job-submit" => Some(Route {
            script_rel: "core://infring-control-plane".to_string(),
            args: std::iter::once("run".to_string())
                .chain(rest.iter().cloned())
                .collect(),
            forward_stdin: false,
        }),
        "infringctl" => Some(Route {
            script_rel: "core://command-list".to_string(),
            args: std::iter::once("--mode=help".to_string())
                .chain(rest.iter().cloned())
                .collect(),
            forward_stdin: false,
        }),
        _ => resolve_core_shortcuts_family_ops1(cmd, rest),
    }
}
