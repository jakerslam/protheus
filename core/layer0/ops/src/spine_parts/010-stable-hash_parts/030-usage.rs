
fn usage() {
    eprintln!("Usage:");
    eprintln!("  infring-ops spine eyes [YYYY-MM-DD] [--max-eyes=N]");
    eprintln!("  infring-ops spine daily [YYYY-MM-DD] [--max-eyes=N]");
    eprintln!("  infring-ops spine run [eyes|daily] [YYYY-MM-DD] [--max-eyes=N]");
    eprintln!("  infring-ops spine status [--mode=eyes|daily] [--date=YYYY-MM-DD]");
    eprintln!(
        "  infring-ops spine sleep-cleanup <run|plan|status|purge> [--apply=1|0] [--force=1|0]"
    );
    eprintln!(
        "  infring-ops spine background-hands-scheduler <configure|schedule|status> [flags]"
    );
    eprintln!("  infring-ops spine rsi-idle-hands-scheduler <run|status> [flags]");
    eprintln!("  infring-ops spine evidence-run-plan [--configured-runs=N] [--budget-pressure=none|soft|hard] [--projected-pressure=none|soft|hard]");
}

fn print_json_line(value: &Value) {
    crate::contract_lane_utils::print_json_line(value);
}

fn cli_error_receipt(argv: &[String], error: &str, code: i32) -> Value {
    let ts = now_iso();
    let mut out = json!({
        "ok": false,
        "type": "spine_cli_error",
        "ts": ts,
        "mode": "unknown",
        "date": ts[..10].to_string(),
        "argv": argv,
        "error": error,
        "exit_code": code,
        "claim_evidence": [
            {
                "id": "fail_closed_cli",
                "claim": "spine_cli_invalid_args_fail_closed_with_deterministic_receipt",
                "evidence": {
                    "error": error,
                    "argv_len": argv.len()
                }
            }
        ],
        "persona_lenses": {
            "guardian": {
                "constitution_integrity_ok": true
            },
            "strategist": {
                "mode": "cli_error"
            }
        }
    });
    out["receipt_hash"] = Value::String(receipt_hash(&out));
    out
}

fn step_failure_reason(name: &str, result: &StepResult) -> String {
    let detail = clean_reason(&result.stderr, &result.stdout);
    if detail.is_empty() {
        format!("step_failed:{name}:{}", result.code)
    } else {
        format!("step_failed:{name}:{}:{detail}", result.code)
    }
}

fn run_node_json(root: &Path, args: &[String]) -> StepResult {
    let output = Command::new("node")
        .args(args)
        .current_dir(root)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
            let payload = parse_json_payload(&stdout);
            StepResult {
                ok: out.status.success(),
                code: out.status.code().unwrap_or(1),
                payload,
                stdout,
                stderr,
            }
        }
        Err(err) => StepResult {
            ok: false,
            code: 1,
            payload: None,
            stdout: String::new(),
            stderr: format!("spawn_failed:{err}"),
        },
    }
}

fn run_ops_domain_json(
    root: &Path,
    domain: &str,
    args: &[String],
    run_context: Option<&str>,
) -> StepResult {
    let root_buf = root.to_path_buf();
    let (command, mut command_args) = resolve_infring_ops_command(&root_buf, domain);
    command_args.extend(args.iter().cloned());

    let mut cmd = Command::new(command);
    cmd.args(command_args)
        .current_dir(root)
        .env(
            "INFRING_NODE_BINARY",
            std::env::var("INFRING_NODE_BINARY").unwrap_or_else(|_| "node".to_string()),
        )
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(context) = run_context {
        let trimmed = context.trim();
        if !trimmed.is_empty() {
            cmd.env("SPINE_RUN_CONTEXT", trimmed);
        }
    }

    match cmd.output() {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
            let payload = parse_json_payload(&stdout);
            StepResult {
                ok: out.status.success(),
                code: out.status.code().unwrap_or(1),
                payload,
                stdout,
                stderr,
            }
        }
        Err(err) => StepResult {
            ok: false,
            code: 1,
            payload: None,
            stdout: String::new(),
            stderr: format!("spawn_failed:{err}"),
        },
    }
}

fn resolve_profile_binary(root: &Path, profile: &str, stem: &str) -> Option<PathBuf> {
    let dir = root.join("target").join(profile);
    if cfg!(windows) {
        let exe = dir.join(format!("{stem}.exe"));
        if exe.is_file() {
            return Some(exe);
        }
    }

    let plain = dir.join(stem);
    if plain.is_file() {
        return Some(plain);
    }
    None
}

fn resolve_infring_ops_command(root: &Path, domain: &str) -> (String, Vec<String>) {
    if let Some(bin) = std::env::var("INFRING_OPS_BIN")
        .ok()
        .or_else(|| std::env::var("INFRING_OPS_BIN").ok())
    {
        let trimmed = bin.trim();
        if !trimmed.is_empty() {
            return (trimmed.to_string(), vec![domain.to_string()]);
        }
    }

    if let Some(release) = resolve_profile_binary(root, "release", "infring-ops") {
        return (
            release.to_string_lossy().to_string(),
            vec![domain.to_string()],
        );
    }
    if let Some(release_legacy) = resolve_profile_binary(root, "release", "infring-ops") {
        return (
            release_legacy.to_string_lossy().to_string(),
            vec![domain.to_string()],
        );
    }
    if let Some(debug) = resolve_profile_binary(root, "debug", "infring-ops") {
        return (
            debug.to_string_lossy().to_string(),
            vec![domain.to_string()],
        );
    }
    if let Some(debug_legacy) = resolve_profile_binary(root, "debug", "infring-ops") {
        return (
            debug_legacy.to_string_lossy().to_string(),
            vec![domain.to_string()],
        );
    }

    (
        "cargo".to_string(),
        vec![
            "run".to_string(),
            "--quiet".to_string(),
            "--manifest-path".to_string(),
            "core/layer0/ops/Cargo.toml".to_string(),
            "--bin".to_string(),
            "infring-ops".to_string(),
            "--".to_string(),
            domain.to_string(),
        ],
    )
}

fn enqueue_spine_attention(root: &Path, source_type: &str, severity: &str, summary: &str) {
    let mut event = json!({
        "ts": now_iso(),
        "type": source_type,
        "source": "spine",
        "source_type": source_type,
        "severity": severity,
        "summary": summary,
        "attention_key": format!("spine:{source_type}")
    });
    event["receipt_hash"] = Value::String(receipt_hash(&event));
    let encoded =
        BASE64_STANDARD.encode(serde_json::to_string(&event).unwrap_or_else(|_| "{}".to_string()));
    let (command, mut args) = resolve_infring_ops_command(root, "attention-queue");
    args.push("enqueue".to_string());
    args.push(format!("--event-json-base64={encoded}"));
    args.push("--run-context=spine".to_string());

    let _ = Command::new(command)
        .args(args)
        .current_dir(root)
        .env(
            "INFRING_NODE_BINARY",
            std::env::var("INFRING_NODE_BINARY").unwrap_or_else(|_| "node".to_string()),
        )
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}
