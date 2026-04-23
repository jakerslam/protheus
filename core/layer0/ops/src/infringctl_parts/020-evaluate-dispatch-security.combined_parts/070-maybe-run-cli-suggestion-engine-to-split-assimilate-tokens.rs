
fn maybe_run_cli_suggestion_engine(root: &Path, cmd: &str, rest: &[String], json_mode: bool) {
    if bool_env("INFRING_GLOBAL_QUIET", false) {
        return;
    }
    if !bool_env("INFRING_CLI_SUGGESTIONS", true) {
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
    let suggestion_script_ts = root.join("client/runtime/systems/tools/cli_suggestion_engine_bridge.ts");
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
    if bool_env("INFRING_GLOBAL_QUIET", false) {
        return;
    }
    if bool_env("INFRING_UPDATE_CHECKER_DISABLED", false) {
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
                script_rel: "client/runtime/systems/ops/run_infring_ops.ts".to_string(),
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
                script_rel: "client/runtime/systems/ops/run_infring_ops.ts".to_string(),
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
            script_rel: "client/runtime/systems/edge/infring_edge_runtime.ts".to_string(),
            args: std::iter::once(sub)
                .chain(rest.iter().skip(1).cloned())
                .collect(),
            forward_stdin: false,
        },
    }
}

fn resolve_core_shortcuts(cmd: &str, rest: &[String]) -> Option<Route> {
    infringctl_routes::resolve_core_shortcuts(cmd, rest)
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
