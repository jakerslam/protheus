fn resolve_core_shortcuts_family_ops1_group_1(cmd: &str, rest: &[String]) -> Option<Route> {
    match cmd {
        "verity" => {
            let first = rest.first().map(|value| value.trim().to_ascii_lowercase());
            let (subcommand, passthrough_start_idx) = match first.as_deref() {
                Some("status") => ("status".to_string(), 1usize),
                Some("drift-status" | "drift") => ("drift-status".to_string(), 1usize),
                Some("vector-check" | "vector") => ("vector-check".to_string(), 1usize),
                Some("record-event" | "record") => ("record-event".to_string(), 1usize),
                Some("refine-event" | "refinement-event") => ("refine-event".to_string(), 1usize),
                _ => ("status".to_string(), 0usize),
            };
            Some(Route {
                script_rel: "core://verity-plane".to_string(),
                args: std::iter::once(subcommand)
                    .chain(rest.iter().skip(passthrough_start_idx).cloned())
                    .collect(),
                forward_stdin: false,
            })
        }
        "rag" => Some(Route {
            script_rel: "core://rag".to_string(),
            args: if rest.is_empty() {
                vec!["status".to_string()]
            } else {
                rest.to_vec()
            },
            forward_stdin: false,
        }),
        "swarm" => Some(Route {
            script_rel: "core://swarm-runtime".to_string(),
            args: if rest.is_empty() {
                vec!["status".to_string()]
            } else {
                rest.to_vec()
            },
            forward_stdin: false,
        }),
        _ => None,
    }
}
