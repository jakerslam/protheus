fn resolve_operator_tooling_shortcuts(cmd: &str, rest: &[String]) -> Option<Route> {
    match cmd {
        "route-model" | "model-route" => Some(Route {
            script_rel: "core://operator-tooling-kernel".to_string(),
            args: std::iter::once("route-model".to_string())
                .chain(rest.iter().cloned())
                .collect(),
            forward_stdin: false,
        }),
        "escalate-model" | "model-escalate" => Some(Route {
            script_rel: "core://operator-tooling-kernel".to_string(),
            args: std::iter::once("escalate-model".to_string())
                .chain(rest.iter().cloned())
                .collect(),
            forward_stdin: false,
        }),
        "plan-auto" | "plan-first" => Some(Route {
            script_rel: "core://operator-tooling-kernel".to_string(),
            args: std::iter::once("plan-auto".to_string())
                .chain(rest.iter().cloned())
                .collect(),
            forward_stdin: false,
        }),
        "plan-validate" => Some(Route {
            script_rel: "core://operator-tooling-kernel".to_string(),
            args: std::iter::once("plan-validate".to_string())
                .chain(rest.iter().cloned())
                .collect(),
            forward_stdin: false,
        }),
        "postflight-validate" | "postflight-check" => Some(Route {
            script_rel: "core://operator-tooling-kernel".to_string(),
            args: std::iter::once("postflight-validate".to_string())
                .chain(rest.iter().cloned())
                .collect(),
            forward_stdin: false,
        }),
        "output-validate" | "output-check" => Some(Route {
            script_rel: "core://operator-tooling-kernel".to_string(),
            args: std::iter::once("output-validate".to_string())
                .chain(rest.iter().cloned())
                .collect(),
            forward_stdin: false,
        }),
        "state-read" | "read-state" => Some(Route {
            script_rel: "core://operator-tooling-kernel".to_string(),
            args: std::iter::once("state-read".to_string())
                .chain(rest.iter().cloned())
                .collect(),
            forward_stdin: false,
        }),
        "state-write" | "write-state" => Some(Route {
            script_rel: "core://operator-tooling-kernel".to_string(),
            args: std::iter::once("state-write".to_string())
                .chain(rest.iter().cloned())
                .collect(),
            forward_stdin: false,
        }),
        "decision-log-append" | "append-decision" => Some(Route {
            script_rel: "core://operator-tooling-kernel".to_string(),
            args: std::iter::once("decision-log-append".to_string())
                .chain(rest.iter().cloned())
                .collect(),
            forward_stdin: false,
        }),
        "safe-apply" => Some(Route {
            script_rel: "core://operator-tooling-kernel".to_string(),
            args: std::iter::once("safe-apply".to_string())
                .chain(rest.iter().cloned())
                .collect(),
            forward_stdin: false,
        }),
        "memory-search" => Some(Route {
            script_rel: "core://operator-tooling-kernel".to_string(),
            args: std::iter::once("memory-search".to_string())
                .chain(rest.iter().cloned())
                .collect(),
            forward_stdin: false,
        }),
        "memory-summarize" => Some(Route {
            script_rel: "core://operator-tooling-kernel".to_string(),
            args: std::iter::once("memory-summarize".to_string())
                .chain(rest.iter().cloned())
                .collect(),
            forward_stdin: false,
        }),
        "memory-last-change" | "memlast" => Some(Route {
            script_rel: "core://operator-tooling-kernel".to_string(),
            args: std::iter::once("memory-last-change".to_string())
                .chain(rest.iter().cloned())
                .collect(),
            forward_stdin: false,
        }),
        "membrief" => Some(Route {
            script_rel: "core://operator-tooling-kernel".to_string(),
            args: std::iter::once("membrief".to_string())
                .chain(rest.iter().cloned())
                .collect(),
            forward_stdin: false,
        }),
        "trace-find" => Some(Route {
            script_rel: "core://operator-tooling-kernel".to_string(),
            args: std::iter::once("trace-find".to_string())
                .chain(rest.iter().cloned())
                .collect(),
            forward_stdin: false,
        }),
        "sync-allowed-models" => Some(Route {
            script_rel: "core://operator-tooling-kernel".to_string(),
            args: std::iter::once("sync-allowed-models".to_string())
                .chain(rest.iter().cloned())
                .collect(),
            forward_stdin: false,
        }),
        "smoke-routing" => Some(Route {
            script_rel: "core://operator-tooling-kernel".to_string(),
            args: std::iter::once("smoke-routing".to_string())
                .chain(rest.iter().cloned())
                .collect(),
            forward_stdin: false,
        }),
        "spawn-safe" => Some(Route {
            script_rel: "core://operator-tooling-kernel".to_string(),
            args: std::iter::once("spawn-safe".to_string())
                .chain(rest.iter().cloned())
                .collect(),
            forward_stdin: false,
        }),
        "smart-spawn" => Some(Route {
            script_rel: "core://operator-tooling-kernel".to_string(),
            args: std::iter::once("smart-spawn".to_string())
                .chain(rest.iter().cloned())
                .collect(),
            forward_stdin: false,
        }),
        "auto-spawn" => Some(Route {
            script_rel: "core://operator-tooling-kernel".to_string(),
            args: std::iter::once("auto-spawn".to_string())
                .chain(rest.iter().cloned())
                .collect(),
            forward_stdin: false,
        }),
        "execute-handoff" => Some(Route {
            script_rel: "core://operator-tooling-kernel".to_string(),
            args: std::iter::once("execute-handoff".to_string())
                .chain(rest.iter().cloned())
                .collect(),
            forward_stdin: false,
        }),
        "safe-run" | "openclaw-safe" | "watch-exec" => Some(Route {
            script_rel: "core://operator-tooling-kernel".to_string(),
            args: std::iter::once("safe-run".to_string())
                .chain(rest.iter().cloned())
                .collect(),
            forward_stdin: false,
        }),
        "openclaw-health" | "safe-health" => Some(Route {
            script_rel: "core://operator-tooling-kernel".to_string(),
            args: std::iter::once("openclaw-health".to_string())
                .chain(rest.iter().cloned())
                .collect(),
            forward_stdin: false,
        }),
        "cron-drift" => Some(Route {
            script_rel: "core://operator-tooling-kernel".to_string(),
            args: std::iter::once("cron-drift".to_string())
                .chain(rest.iter().cloned())
                .collect(),
            forward_stdin: false,
        }),
        "cron-sync" => Some(Route {
            script_rel: "core://operator-tooling-kernel".to_string(),
            args: std::iter::once("cron-sync".to_string())
                .chain(rest.iter().cloned())
                .collect(),
            forward_stdin: false,
        }),
        "doctor" | "openclaw-doctor" => Some(Route {
            script_rel: "core://operator-tooling-kernel".to_string(),
            args: std::iter::once("doctor".to_string())
                .chain(rest.iter().cloned())
                .collect(),
            forward_stdin: false,
        }),
        "audit-plane" | "control-plane-audit" => Some(Route {
            script_rel: "core://operator-tooling-kernel".to_string(),
            args: std::iter::once("audit-plane".to_string())
                .chain(rest.iter().cloned())
                .collect(),
            forward_stdin: false,
        }),
        "daily-brief" => Some(Route {
            script_rel: "core://operator-tooling-kernel".to_string(),
            args: std::iter::once("daily-brief".to_string())
                .chain(rest.iter().cloned())
                .collect(),
            forward_stdin: false,
        }),
        "fail-playbook" => Some(Route {
            script_rel: "core://operator-tooling-kernel".to_string(),
            args: std::iter::once("fail-playbook".to_string())
                .chain(rest.iter().cloned())
                .collect(),
            forward_stdin: false,
        }),
        _ => None,
    }
}
