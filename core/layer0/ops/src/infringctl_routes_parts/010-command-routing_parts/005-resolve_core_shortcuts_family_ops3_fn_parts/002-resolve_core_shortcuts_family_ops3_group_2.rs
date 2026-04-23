fn resolve_core_shortcuts_family_ops3_group_2(cmd: &str, rest: &[String]) -> Option<Route> {
    match cmd {
        "ai" => Some(Route {
            script_rel: "core://enterprise-hardening".to_string(),
            args: std::iter::once("ai".to_string())
                .chain(rest.iter().cloned())
                .collect(),
            forward_stdin: false,
        }),
        "chaos" => Some(Route {
            script_rel: "core://enterprise-hardening".to_string(),
            args: if rest
                .first()
                .map(|v| v.trim().eq_ignore_ascii_case("isolate"))
                .unwrap_or(false)
            {
                std::iter::once("chaos-run".to_string())
                    .chain(std::iter::once("--suite=isolate".to_string()))
                    .chain(rest.iter().skip(1).cloned())
                    .collect()
            } else {
                std::iter::once("chaos-run".to_string())
                    .chain(rest.iter().skip(1).cloned())
                    .collect()
            },
            forward_stdin: false,
        }),
        "assistant" => Some(Route {
            script_rel: "core://enterprise-hardening".to_string(),
            args: std::iter::once("assistant-mode".to_string())
                .chain(rest.iter().cloned())
                .collect(),
            forward_stdin: false,
        }),
        _ => None,
    }
}
