pub fn evaluate_dispatch_security(
    root: &Path,
    script_rel: &str,
    args: &[String],
) -> DispatchSecurity {
    if bool_env("PROTHEUS_CTL_SECURITY_GATE_DISABLED", false) {
        return DispatchSecurity {
            ok: true,
            reason: "protheusctl_dispatch_gate_disabled".to_string(),
        };
    }

    let workspace_root = effective_workspace_root(root);
    let req = security_request(&workspace_root, script_rel, args);
    let persona_gate = evaluate_persona_dispatch_security(script_rel, args, &req);
    if !persona_gate.ok {
        return persona_gate;
    }
    if req
        .get("covenant_violation")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        || req
            .get("tamper_signal")
            .and_then(Value::as_bool)
            .unwrap_or(false)
    {
        return DispatchSecurity {
            ok: false,
            reason: "security_gate_blocked:local_fail_closed_signal".to_string(),
        };
    }

    let request_json = serde_json::to_string(&req).unwrap_or_else(|_| "{}".to_string());
    let request_base64 = BASE64_STANDARD.encode(request_json.as_bytes());

    let manifest = workspace_root.join("core/layer0/security/Cargo.toml");
    if !manifest.exists() {
        return DispatchSecurity {
            ok: false,
            reason: "security_gate_blocked:manifest_missing".to_string(),
        };
    }

    let output = Command::new("cargo")
        .arg("run")
        .arg("--quiet")
        .arg("--manifest-path")
        .arg(manifest)
        .arg("--bin")
        .arg("security_core")
        .arg("--")
        .arg("check")
        .arg(format!("--request-base64={request_base64}"))
        .current_dir(workspace_root)
        .output();

    let Ok(out) = output else {
        return DispatchSecurity {
            ok: false,
            reason: "security_gate_blocked:spawn_failed".to_string(),
        };
    };

    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        let stdout = String::from_utf8_lossy(&out.stdout);
        let msg = if stderr.trim().is_empty() {
            stdout.to_string()
        } else {
            stderr.to_string()
        };
        return DispatchSecurity {
            ok: false,
            reason: format!("security_gate_blocked:{}", clean(msg, 220)),
        };
    }

    let payload = parse_json(&String::from_utf8_lossy(&out.stdout));
    let Some(payload) = payload else {
        return DispatchSecurity {
            ok: false,
            reason: "security_gate_blocked:invalid_security_payload".to_string(),
        };
    };

    let decision = payload.get("decision").cloned().unwrap_or(Value::Null);
    let ok = decision.get("ok").and_then(Value::as_bool).unwrap_or(false);
    let fail_closed = decision
        .get("fail_closed")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    if !ok || fail_closed {
        let reason = decision
            .get("reasons")
            .and_then(Value::as_array)
            .and_then(|rows| rows.first())
            .and_then(Value::as_str)
            .unwrap_or("dispatch_security_gate_blocked")
            .to_string();
        return DispatchSecurity {
            ok: false,
            reason: format!("security_gate_blocked:{}", clean(reason, 220)),
        };
    }

    DispatchSecurity {
        ok: true,
        reason: "ok".to_string(),
    }
}
fn run_node_script(root: &Path, script_rel: &str, args: &[String], forward_stdin: bool) -> i32 {
    let workspace_root = effective_workspace_root(root);
    if let Some(domain) = script_rel.strip_prefix("core://") {
        return run_core_domain(&workspace_root, domain, args, forward_stdin);
    }

    let script_abs = workspace_root.join(script_rel);
    if !script_abs.exists() {
        let synthetic_route = Route {
            script_rel: script_rel.to_string(),
            args: args.to_vec(),
            forward_stdin,
        };
        if let Some(status) = node_missing_fallback(&workspace_root, &synthetic_route, false) {
            return status;
        }
        eprintln!(
            "{}",
            json!({
                "ok": false,
                "type": "protheusctl_dispatch",
                "error": "script_missing",
                "script_rel": clean(script_rel, 220)
            })
        );
        return 1;
    }

    let mut cmd = Command::new(node_bin());
    cmd.arg(script_abs)
        .args(args)
        .current_dir(workspace_root)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

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
                    "error": clean(format!("spawn_failed:{err}"), 220)
                })
            );
            1
        }
    }
}

fn run_core_domain(root: &Path, domain: &str, args: &[String], forward_stdin: bool) -> i32 {
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

fn maybe_run_cli_suggestion_engine(root: &Path, cmd: &str, rest: &[String]) {
    if bool_env("PROTHEUS_GLOBAL_QUIET", false) {
        return;
    }
    if !bool_env("PROTHEUS_CLI_SUGGESTIONS", true) {
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
            | "demo"
            | "examples"
            | "version"
            | "update"
            | "diagram"
            | "shadow"
            | "debug"
            | "setup"
            | "completion"
            | "repl"
            | "status"
    ) {
        return;
    }
    let suggestion_script = root.join("client/runtime/systems/tools/cli_suggestion_engine.js");
    let suggestion_ts = root.join("client/runtime/systems/tools/cli_suggestion_engine.ts");
    if !suggestion_script.exists() || !suggestion_ts.exists() {
        return;
    }
    let request_json = serde_json::to_string(&json!({
        "cmd": cmd,
        "args": rest
    }))
    .unwrap_or_else(|_| "{}".to_string());
    let _ = Command::new(node_bin())
        .arg(suggestion_script)
        .arg("suggest")
        .arg("--origin=main_cli")
        .arg(format!("--cmd={}", clean(cmd, 60)))
        .arg(format!("--argv-json={request_json}"))
        .current_dir(root)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status();
}

fn maybe_run_update_checker(root: &Path, cmd: &str) {
    if bool_env("PROTHEUS_GLOBAL_QUIET", false) {
        return;
    }
    if bool_env("PROTHEUS_UPDATE_CHECKER_DISABLED", false) {
        return;
    }
    if matches!(cmd, "version" | "update" | "help" | "--help" | "-h") {
        return;
    }
    let script = root.join("client/runtime/systems/ops/protheus_version_cli.js");
    if !script.exists() {
        return;
    }
    let _ = Command::new(node_bin())
        .arg(script)
        .arg("check-quiet")
        .current_dir(root)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status();
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
                script_rel: "client/runtime/systems/spawn/mobile_edge_swarm_bridge.ts".to_string(),
                args: std::iter::once(action)
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
                script_rel: "client/runtime/systems/ops/run_protheus_ops.js".to_string(),
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
