fn resolve_core_shortcuts_family_misc(cmd: &str, rest: &[String]) -> Option<Route> {
    match cmd {
        "start" | "boot" => Some(Route {
            script_rel: "core://daemon-control".to_string(),
            args: std::iter::once("start".to_string())
                .chain(rest.iter().cloned())
                .collect(),
            forward_stdin: false,
        }),
        _ => protheusctl_plane_shortcuts::resolve_plane_shortcuts(cmd, rest),
    }
}
