        );
        if !warnings.is_empty() {
            println!("[infring doctor] warnings: {}", warnings.join(", "));
        }
        if !root_cause_codes.is_empty() {
            println!(
                "[infring doctor] root-cause-codes: {}",
                root_cause_codes.join(", ")
            );
        }
        if ok {
            println!("[infring doctor] verdict: ok");
        } else {
            println!("[infring doctor] verdict: failed ({})", failures.join(", "));
        }
    }
    if ok {
        0
    } else {
        2
    }
}

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
                        "type": "protheusctl_dispatch",
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
                    "type": "protheusctl_dispatch",
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
            cmd.env("PROTHEUS_NEXUS_CONNECTION", clean(&raw, 8_000));
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
                    "type": "protheusctl_dispatch",
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

fn maybe_run_cli_suggestion_engine(root: &Path, cmd: &str, rest: &[String], json_mode: bool) {
    if bool_env("PROTHEUS_GLOBAL_QUIET", false) {
        return;
    }
    if !bool_env("PROTHEUS_CLI_SUGGESTIONS", true) {
        return;
    }
    if suppress_pre_dispatch_side_effects(cmd, json_mode) {
        return;
    }
    if matches!(
        cmd,
        "assimilate"
            | "research"
            | "tutorial"
            | "list"
            | "help"
            | "--help"
            | "-h"
            | "verify"
            | "inspect"
            | "replay"
            | "demo"
            | "examples"
            | "version"
            | "update"
            | "release-semver-contract"
            | "diagram"
            | "shadow"
            | "debug"
            | "setup"
            | "completion"
            | "repl"
            | "status"
            | "toolkit"
            | "task"
    ) {
        return;
    }
    let suggestion_script_ts = root.join("client/runtime/systems/tools/cli_suggestion_engine.ts");
    let suggestion_script_js = root.join("client/runtime/systems/tools/cli_suggestion_engine.js");
    let suggestion_script = if suggestion_script_ts.exists() {
        suggestion_script_ts
    } else if suggestion_script_js.exists() {
        suggestion_script_js
    } else {
        return;
    };
    let request_json = serde_json::to_string(&json!({
        "cmd": cmd,
        "args": rest
    }))
    .unwrap_or_else(|_| "{}".to_string());
    let ts_entrypoint = root.join("client/runtime/lib/ts_entrypoint.ts");
    let script_is_ts = suggestion_script
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("ts"))
        .unwrap_or(false);

    let mut proc = Command::new(node_bin());
    if script_is_ts && ts_entrypoint.exists() {
        proc.arg(ts_entrypoint).arg(&suggestion_script);
    } else {
        proc.arg(&suggestion_script);
    }

    let _ = proc
        .arg("suggest")
        .arg("--origin=main_cli")
        .arg(format!("--cmd={}", clean(cmd, 60)))
        .arg(format!("--argv-json={request_json}"))
        .current_dir(root)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

fn maybe_run_update_checker(root: &Path, cmd: &str, json_mode: bool) {
    if bool_env("PROTHEUS_GLOBAL_QUIET", false) {
        return;
    }
    if bool_env("PROTHEUS_UPDATE_CHECKER_DISABLED", false) {
        return;
    }
    if suppress_pre_dispatch_side_effects(cmd, json_mode) {
        return;
    }
    let _ = run_version_cli_domain(root, &[String::from("check-quiet")]);
}

fn route_edge(rest: &[String]) -> Route {
    let sub = rest
        .first()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());
    match sub.as_str() {
        "lifecycle" => {
            let action = rest
                .get(1)
                .map(|v| v.trim().to_ascii_lowercase())
                .unwrap_or_else(|| "status".to_string());
            Route {
                script_rel: "client/runtime/systems/edge/mobile_lifecycle_resilience.ts"
                    .to_string(),
                args: std::iter::once(action)
                    .chain(rest.iter().skip(2).cloned())
                    .collect(),
                forward_stdin: false,
            }
        }
        "swarm" => {
            let action = rest
                .get(1)
                .map(|v| v.trim().to_ascii_lowercase())
                .unwrap_or_else(|| "status".to_string());
            Route {
                script_rel: "client/runtime/systems/ops/run_protheus_ops.ts".to_string(),
                args: std::iter::once("edge".to_string())
                    .chain(std::iter::once("swarm".to_string()))
                    .chain(std::iter::once(action))
                    .chain(rest.iter().skip(2).cloned())
                    .collect(),
                forward_stdin: false,
            }
        }
        "wrapper" => {
            let action = rest
                .get(1)
                .map(|v| v.trim().to_ascii_lowercase())
                .unwrap_or_else(|| "status".to_string());
            Route {
                script_rel: "client/runtime/systems/ops/mobile_wrapper_distribution_pack.js"
                    .to_string(),
                args: std::iter::once(action)
                    .chain(rest.iter().skip(2).cloned())
                    .collect(),
                forward_stdin: false,
            }
        }
        "benchmark" => {
            let action = rest
                .get(1)
                .map(|v| v.trim().to_ascii_lowercase())
                .unwrap_or_else(|| "status".to_string());
            Route {
                script_rel: "client/runtime/systems/ops/run_protheus_ops.ts".to_string(),
                args: std::iter::once("benchmark-matrix".to_string())
                    .chain(std::iter::once(action))
                    .chain(rest.iter().skip(2).cloned())
                    .collect(),
                forward_stdin: false,
            }
        }
        "top" => Route {
            script_rel: "client/runtime/systems/edge/mobile_ops_top.ts".to_string(),
            args: std::iter::once("status".to_string())
                .chain(rest.iter().skip(1).cloned())
                .collect(),
            forward_stdin: false,
        },
        _ => Route {
            script_rel: "client/runtime/systems/edge/protheus_edge_runtime.ts".to_string(),
            args: std::iter::once(sub)
                .chain(rest.iter().skip(1).cloned())
                .collect(),
            forward_stdin: false,
        },
    }
}

fn resolve_core_shortcuts(cmd: &str, rest: &[String]) -> Option<Route> {
    protheusctl_routes::resolve_core_shortcuts(cmd, rest)
}

fn is_assimilate_wrapper_flag(token: &str) -> bool {
    matches!(token, "--showcase" | "--scaffold-payload" | "--no-prewarm")
        || token.starts_with("--showcase=")
        || token.starts_with("--duration-ms=")
        || token.starts_with("--scaffold-payload=")
        || token.starts_with("--prewarm=")
}

fn split_assimilate_tokens(rest: &[String]) -> (Option<String>, Vec<String>, Vec<String>) {
    let mut target: Option<String> = None;
    let mut core_passthrough = Vec::<String>::new();
    let mut wrapper_flags = Vec::<String>::new();
    for token in rest {
        let trimmed = token.trim();
        if target.is_none() {
            if let Some(value) = trimmed.strip_prefix("--target=") {
                let normalized = value.trim();
                if !normalized.is_empty() {
                    target = Some(normalized.to_string());
                    continue;
                }
            } else if !trimmed.starts_with("--") {
                target = Some(trimmed.to_string());
                continue;
            }
        }
        if is_assimilate_wrapper_flag(trimmed) {
            wrapper_flags.push(token.clone());
        } else {
            core_passthrough.push(token.clone());
        }
    }
    (target, core_passthrough, wrapper_flags)
}
