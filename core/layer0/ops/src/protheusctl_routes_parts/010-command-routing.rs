use super::Route;
// FILE_SIZE_EXCEPTION: reason=Atomic CLI routing block requires semantic extraction to preserve command behavior; owner=jay; expires=2026-04-23

#[path = "../protheusctl_plane_shortcuts.rs"]
mod protheusctl_plane_shortcuts;
include!("012-operator-tooling-shortcuts.rs");
fn contains_help_flag(args: &[String]) -> bool {
    args.iter().any(|arg| matches!(arg.trim(), "--help" | "-h"))
}

fn parse_true_flag(args: &[String], key: &str) -> bool {
    let exact = format!("--{key}");
    let prefix = format!("--{key}=");
    for arg in args {
        let token = arg.trim();
        if token == exact {
            return true;
        }
        if let Some(value) = token.strip_prefix(&prefix) {
            let norm = value.trim().to_ascii_lowercase();
            return matches!(norm.as_str(), "1" | "true" | "yes" | "on");
        }
    }
    false
}

fn has_prefix_flag(args: &[String], key: &str) -> bool {
    let prefix = format!("--{key}=");
    args.iter().any(|arg| arg.trim().starts_with(&prefix))
}

fn normalize_dashboard_flag(token: &str) -> String {
    let trimmed = token.trim();
    if let Some(value) = trimmed.strip_prefix("--host=") {
        return format!("--dashboard-host={value}");
    }
    if let Some(value) = trimmed.strip_prefix("--port=") {
        return format!("--dashboard-port={value}");
    }
    if trimmed == "--host" {
        return "--dashboard-host".to_string();
    }
    if trimmed == "--port" {
        return "--dashboard-port".to_string();
    }
    trimmed.to_string()
}

fn parse_daemon_control_subcommand(
    first: Option<&str>,
    allow_dashboard_aliases: bool,
) -> Option<(String, usize)> {
    let mut normalized = first?;
    if normalized == "boot" || (allow_dashboard_aliases && normalized == "serve") {
        normalized = "start";
    }
    if matches!(
        normalized,
        "start"
            | "stop"
            | "restart"
            | "status"
            | "heal"
            | "attach"
            | "subscribe"
            | "tick"
            | "diagnostics"
    ) {
        Some((normalized.to_string(), 1usize))
    } else {
        None
    }
}

fn route_dashboard_compat(rest: &[String], from_dashboard_ui: bool) -> Route {
    let first = rest.first().map(|value| value.trim().to_ascii_lowercase());
    let (subcommand, passthrough_start_idx) = match first.as_deref() {
        Some("help" | "--help" | "-h") => ("status".to_string(), 0usize),
        other => parse_daemon_control_subcommand(other, true)
            .unwrap_or_else(|| ("start".to_string(), 0usize)),
    };

    let mut args = std::iter::once(subcommand.clone())
        .chain(
            rest.iter()
                .skip(passthrough_start_idx)
                .map(|token| normalize_dashboard_flag(token)),
        )
        .collect::<Vec<_>>();
    if subcommand == "start" {
        let has_open_flag = args.iter().any(|arg| {
            let token = arg.trim();
            token == "--dashboard-open"
                || token == "--no-browser"
                || token.starts_with("--dashboard-open=")
        });
        if !has_open_flag {
            args.push(format!(
                "--dashboard-open={}",
                if from_dashboard_ui { 0 } else { 1 }
            ));
        }
    }
    Route {
        script_rel: "core://daemon-control".to_string(),
        args,
        forward_stdin: false,
    }
}

include!("010-command-routing_parts/001-resolve_core_shortcuts_family_daemon.rs");
include!("010-command-routing_parts/002-resolve_core_shortcuts_family_shell.rs");
include!("010-command-routing_parts/003-resolve_core_shortcuts_family_ops1.rs");
include!("010-command-routing_parts/004-resolve_core_shortcuts_family_ops2.rs");
include!("010-command-routing_parts/005-resolve_core_shortcuts_family_ops3.rs");
include!("010-command-routing_parts/006-resolve_core_shortcuts_family_misc.rs");
pub(super) fn resolve_core_shortcuts(cmd: &str, rest: &[String]) -> Option<Route> {
    if let Some(route) = resolve_operator_tooling_shortcuts(cmd, rest) {
        return Some(route);
    }
    resolve_core_shortcuts_family_daemon(cmd, rest)
}
