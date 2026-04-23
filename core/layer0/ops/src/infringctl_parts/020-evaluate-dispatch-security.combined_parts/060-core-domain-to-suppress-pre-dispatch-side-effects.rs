
fn run_core_domain(root: &Path, domain: &str, args: &[String], forward_stdin: bool) -> i32 {
    if domain == "unknown-command" {
        return run_unknown_command_domain(args);
    }
    if domain == "install-doctor" {
        return run_install_doctor_domain(root, args);
    }
    if domain == "command-list" {
        return crate::command_list_kernel::run(root, args);
    }
    if domain == "completion" {
        return run_completion_domain(args);
    }
    if domain == "repl" {
        return run_repl_domain(root, args);
    }
    if domain == "version-cli" {
        return run_version_cli_domain(root, args);
    }
    if domain == "release-semver-contract" {
        return run_release_semver_contract_domain(root, args);
    }
    let nexus_tool = core_domain_nexus_tool_label(domain, args);
    let nexus_connection =
        match crate::dashboard_tool_turn_loop::authorize_ingress_tool_call_with_nexus(
            nexus_tool.as_str(),
        ) {
            Ok(meta) => meta,
            Err(err) => {
                eprintln!(
                    "{}",
                    json!({
                        "ok": false,
                        "type": "infringctl_dispatch",
                        "error": "core_domain_nexus_denied",
                        "domain": clean(domain, 120),
                        "route_label": clean(&nexus_tool, 200),
                        "reason": clean(&err, 240),
                        "fail_closed": true
                    })
                );
                return 1;
            }
        };

    let exe = match env::current_exe() {
        Ok(path) => path,
        Err(err) => {
            eprintln!(
                "{}",
                json!({
                    "ok": false,
                    "type": "infringctl_dispatch",
                    "error": clean(format!("current_exe_failed:{err}"), 220)
                })
            );
            return 1;
        }
    };

    let mut cmd = Command::new(exe);
    cmd.arg(domain)
        .args(args)
        .current_dir(root)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    if let Some(meta) = nexus_connection {
        if let Ok(raw) = serde_json::to_string(&meta) {
            cmd.env("INFRING_NEXUS_CONNECTION", clean(&raw, 8_000));
        }
    }

    if forward_stdin {
        cmd.stdin(Stdio::inherit());
    } else {
        cmd.stdin(Stdio::null());
    }

    match cmd.status() {
        Ok(status) => status.code().unwrap_or(1),
        Err(err) => {
            eprintln!(
                "{}",
                json!({
                    "ok": false,
                    "type": "infringctl_dispatch",
                    "error": clean(format!("core_spawn_failed:{err}"), 220),
                    "domain": domain
                })
            );
            1
        }
    }
}

fn core_domain_nexus_tool_label(domain: &str, args: &[String]) -> String {
    let normalized_domain = clean(domain, 120).to_ascii_lowercase().replace('-', "_");
    if normalized_domain.contains("web") || normalized_domain.contains("search") {
        return "web_search".to_string();
    }
    if normalized_domain.contains("context")
        || normalized_domain.contains("memory")
        || normalized_domain.contains("continuity")
    {
        return "batch_query".to_string();
    }
    if normalized_domain.contains("stomach") {
        return "stomach_status".to_string();
    }
    if normalized_domain.contains("terminal")
        || args
            .iter()
            .any(|row| row.trim().to_ascii_lowercase().contains("terminal"))
    {
        return "terminal_exec".to_string();
    }
    format!("core_domain_{normalized_domain}")
}

fn enforce_command_center_boundary(cmd: &str, route: &Route) -> Result<(), String> {
    if route
        .script_rel
        .contains("client/runtime/systems/red_legion/command_center")
    {
        return Err("red_legion_client_authority_forbidden".to_string());
    }
    if cmd == "session"
        && !route
            .script_rel
            .starts_with("core://command-center-session")
    {
        return Err("session_route_must_be_core_authoritative".to_string());
    }
    Ok(())
}

fn suppress_pre_dispatch_side_effects(cmd: &str, json_mode: bool) -> bool {
    if json_mode {
        return true;
    }
    matches!(
        cmd,
        "list"
            | "status"
            | "doctor"
            | "verify"
            | "inspect"
            | "replay"
            | "verify-install"
            | "gateway"
            | "recover"
            | "dashboard"
            | "setup"
            | "version"
            | "update"
            | "release-semver-contract"
            | "help"
            | "--help"
            | "-h"
            | "completion"
            | "repl"
    )
}
