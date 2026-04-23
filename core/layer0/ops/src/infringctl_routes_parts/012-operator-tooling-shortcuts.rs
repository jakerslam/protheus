fn operator_tooling_route(action: &str, rest: &[String]) -> Route {
    Route {
        script_rel: "core://operator-tooling-kernel".to_string(),
        args: std::iter::once(action.to_string())
            .chain(rest.iter().cloned())
            .collect(),
        forward_stdin: false,
    }
}

fn operator_tooling_shortcut_action(cmd: &str) -> Option<&'static str> {
    Some(match cmd {
        "route-model" | "model-route" => "route-model",
        "escalate-model" | "model-escalate" => "escalate-model",
        "plan-auto" | "plan-first" => "plan-auto",
        "plan-validate" => "plan-validate",
        "postflight-validate" | "postflight-check" => "postflight-validate",
        "output-validate" | "output-check" => "output-validate",
        "state-read" | "read-state" => "state-read",
        "state-write" | "write-state" => "state-write",
        "decision-log-append" | "append-decision" => "decision-log-append",
        "safe-apply" => "safe-apply",
        "memory-search" => "memory-search",
        "memory-summarize" => "memory-summarize",
        "memory-last-change" | "memlast" => "memory-last-change",
        "membrief" => "membrief",
        "trace-find" => "trace-find",
        "sync-allowed-models" => "sync-allowed-models",
        "smoke-routing" => "smoke-routing",
        "spawn-safe" => "spawn-safe",
        "smart-spawn" => "smart-spawn",
        "auto-spawn" => "auto-spawn",
        "execute-handoff" => "execute-handoff",
        "safe-run" | "control_runtime-safe" | "watch-exec" => "safe-run",
        "control_runtime-health" | "safe-health" => "control_runtime-health",
        "cron-drift" => "cron-drift",
        "cron-sync" => "cron-sync",
        "control_runtime-doctor" => "doctor",
        "audit-plane" | "control-plane-audit" => "audit-plane",
        "daily-brief" => "daily-brief",
        "fail-playbook" => "fail-playbook",
        _ => return None,
    })
}

fn resolve_operator_tooling_shortcuts(cmd: &str, rest: &[String]) -> Option<Route> {
    operator_tooling_shortcut_action(cmd).map(|action| operator_tooling_route(action, rest))
}
