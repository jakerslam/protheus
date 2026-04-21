
fn run_build_run_options(payload: &Map<String, Value>) -> Value {
    let parsed = parsed_map(payload);
    json!({
        "ok": true,
        "options": build_run_options_value(&parsed)
    })
}

fn run_prepare_run(payload: &Map<String, Value>) -> Value {
    let parsed = parsed_map(payload);
    let positionals = parsed_positionals(&parsed);
    let domain = if let Some(value) = parsed.get("domain") {
        clean_text(Some(value), 120)
    } else {
        positionals
            .first()
            .map(|value| value.split_whitespace().collect::<Vec<_>>().join(" "))
            .unwrap_or_default()
            .chars()
            .take(120)
            .collect::<String>()
    };
    json!({
        "ok": !domain.is_empty(),
        "domain": domain,
        "args": Value::Array(build_pass_args_vec(&parsed).into_iter().map(Value::String).collect()),
        "options": build_run_options_value(&parsed)
    })
}

fn resolve_command_and_args(domain: &str) -> (String, Vec<String>) {
    let explicit = std::env::var("PROTHEUS_OPS_BIN")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());
    if let Some(cmd) = explicit {
        return (cmd, vec![domain.to_string()]);
    }
    if let Ok(current) = std::env::current_exe() {
        return (
            current.to_string_lossy().to_string(),
            vec![domain.to_string()],
        );
    }
    (
        "cargo".to_string(),
        vec![
            "run".to_string(),
            "--quiet".to_string(),
            "-p".to_string(),
            "protheus-ops-core".to_string(),
            "--bin".to_string(),
            "protheus-ops".to_string(),
            "--".to_string(),
            domain.to_string(),
        ],
    )
}

fn parse_json_payload(stdout: &str) -> Option<Value> {
    let raw = stdout.trim();
    if raw.is_empty() {
        return None;
    }
    if let Ok(parsed) = serde_json::from_str::<Value>(raw) {
        return Some(parsed);
    }
    for line in raw.lines().rev() {
        let candidate = line.trim();
        if !candidate.starts_with('{') {
            continue;
        }
        if let Ok(parsed) = serde_json::from_str::<Value>(candidate) {
            return Some(parsed);
        }
    }
    None
}

fn run_domain_once(root: &Path, domain: &str, args: &[String]) -> Result<(i32, Value), String> {
    let clean_domain = domain
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(120)
        .collect::<String>();
    if clean_domain.is_empty() {
        return Ok((
            2,
            json!({
                "ok": false,
                "type": "ops_domain_conduit_bridge_error",
                "reason": "missing_domain",
                "routed_via": "core_local"
            }),
        ));
    }

    let (command, mut command_args) = resolve_command_and_args(&clean_domain);
    command_args.extend(args.iter().cloned());
    let run = Command::new(&command)
        .args(&command_args)
        .current_dir(root)
        .output()
        .map_err(|err| format!("ops_domain_conduit_runner_kernel_spawn_failed:{err}"))?;

    if !run.stdout.is_empty() {
        std::io::stdout()
            .write_all(&run.stdout)
            .map_err(|err| format!("ops_domain_conduit_runner_kernel_stdout_write_failed:{err}"))?;
    }
    if !run.stderr.is_empty() {
        std::io::stderr()
            .write_all(&run.stderr)
            .map_err(|err| format!("ops_domain_conduit_runner_kernel_stderr_write_failed:{err}"))?;
    }

    let status = run.status.code().unwrap_or(1);
    let parsed = parse_json_payload(String::from_utf8_lossy(&run.stdout).as_ref());
    let payload = if let Some(object) = parsed.and_then(|value| value.as_object().cloned()) {
        let mut owned = Value::Object(object);
        if owned.get("routed_via").is_none() {
            owned["routed_via"] = Value::String("core_local".to_string());
        }
        if status != 0 && owned.get("ok").is_none() {
            owned["ok"] = Value::Bool(false);
        }
        owned
    } else {
        let stderr = String::from_utf8_lossy(&run.stderr);
        let reason = if status == 0 {
            "ok".to_string()
        } else {
            lane_utils::clean_text(Some(stderr.as_ref()), 320)
        };
        json!({
            "ok": status == 0,
            "type": if status == 0 { "ops_domain_conduit_bridge_result" } else { "ops_domain_conduit_bridge_error" },
            "reason": reason,
            "routed_via": "core_local"
        })
    };
    Ok((status, payload))
}

fn run_execute(root: &Path, payload: &Map<String, Value>) -> Value {
    let parsed = parsed_map(payload);
    let positionals = parsed_positionals(&parsed);
    let domain = if let Some(value) = parsed.get("domain") {
        clean_text(Some(value), 120)
    } else {
        positionals
            .first()
            .map(|value| value.split_whitespace().collect::<Vec<_>>().join(" "))
            .unwrap_or_default()
            .chars()
            .take(120)
            .collect::<String>()
    };
    if domain.is_empty() {
        return json!({
            "ok": false,
            "status": 2,
            "payload": {
                "ok": false,
                "type": "ops_domain_conduit_bridge_error",
                "reason": "missing_domain",
                "routed_via": "core_local"
            }
        });
    }
    let args = build_pass_args_vec(&parsed);
    match run_domain_once(root, &domain, &args) {
        Ok((status, payload)) => json!({
            "ok": status == 0 && payload.get("ok").and_then(Value::as_bool).unwrap_or(true),
            "status": status,
            "payload": payload
        }),
        Err(err) => json!({
            "ok": false,
            "status": 1,
            "payload": {
                "ok": false,
                "type": "ops_domain_conduit_bridge_error",
                "reason": err,
                "routed_via": "core_local"
            }
        }),
    }
}

fn queue_dir_from_argv(root: &Path, argv: &[String]) -> std::path::PathBuf {
    let raw = lane_utils::parse_flag(argv, "queue-dir", false)
        .unwrap_or_else(|| "local/state/tools/ops_bridge_ipc".to_string());
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return root.join("local/state/tools/ops_bridge_ipc");
    }
    let candidate = std::path::PathBuf::from(trimmed);
    if candidate.is_absolute() {
        candidate
    } else {
        root.join(candidate)
    }
}

fn poll_ms_from_argv(argv: &[String]) -> u64 {
    let raw = lane_utils::parse_flag(argv, "poll-ms", false).unwrap_or_else(|| "20".to_string());
    parse_i64_text(raw.as_str(), 20).clamp(5, 1000) as u64
}

fn write_json_atomic(path: &Path, payload: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("ops_domain_conduit_runner_kernel_queue_mkdir_failed:{err}"))?;
    }
    let tmp = path.with_extension("json.tmp");
    let body = serde_json::to_string(payload)
        .map_err(|err| format!("ops_domain_conduit_runner_kernel_queue_encode_failed:{err}"))?;
    fs::write(&tmp, format!("{body}\n"))
        .map_err(|err| format!("ops_domain_conduit_runner_kernel_queue_write_failed:{err}"))?;
    fs::rename(&tmp, path)
        .map_err(|err| format!("ops_domain_conduit_runner_kernel_queue_rename_failed:{err}"))?;
    Ok(())
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

fn write_ipc_heartbeat(path: &Path, poll_ms: u64) -> Result<(), String> {
    write_json_atomic(
        path,
        &json!({
            "ok": true,
            "type": "ops_domain_ipc_daemon_heartbeat",
            "pid": std::process::id(),
            "ts_ms": now_ms(),
            "poll_ms": poll_ms
        }),
    )
}
