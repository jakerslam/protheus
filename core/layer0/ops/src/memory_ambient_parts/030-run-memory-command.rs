fn invalid_payload(kind: &str, reason: &str) -> Value {
    json!({
        "ok": false,
        "type": kind,
        "reason": reason
    })
}

fn parse_or_invalid(stdout: &str, kind: &str, reason: &str) -> Value {
    parse_json_payload(stdout).unwrap_or_else(|| invalid_payload(kind, reason))
}

fn ensure_object_payload(payload: Value, kind: &str, reason: &str) -> Value {
    if payload.is_object() {
        payload
    } else {
        invalid_payload(kind, reason)
    }
}

fn run_memory_command(
    root: &Path,
    memory_command: &str,
    memory_args: &[String],
) -> Result<(Value, String, String, i32, Value), String> {
    let root_buf = root.to_path_buf();
    let (command, mut args) = resolve_memory_command(&root_buf);
    args.push(memory_command.to_string());
    args.extend(memory_args.iter().cloned());

    let output = Command::new(&command)
        .args(&args)
        .current_dir(root)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|err| format!("memory_cli_spawn_failed:{err}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let exit_code = output.status.code().unwrap_or(1);

    let payload = parse_or_invalid(
        &stdout,
        "memory_cli_invalid_payload",
        "memory_cli_invalid_json",
    );

    let command_info = json!({
        "binary": command,
        "args": args,
        "exit_code": exit_code
    });

    Ok((payload, stdout, stderr, exit_code, command_info))
}

fn extract_memory_invocation(argv: &[String]) -> Result<(String, Vec<String>, String), String> {
    let flags = parse_cli_flags(argv);
    let run_context = clean_text(flags.get("run-context").map(String::as_str), 40);
    let run_context = if run_context.is_empty() {
        "memory".to_string()
    } else {
        run_context
    };

    if let Some(command) = flags
        .get("memory-command")
        .map(|raw| clean_text(Some(raw), 64).to_ascii_lowercase())
        .filter(|raw| !raw.is_empty())
    {
        let mut memory_args = collect_flag_values(argv, "memory-arg")
            .into_iter()
            .filter(|row| !row.trim().is_empty())
            .collect::<Vec<_>>();

        if let Some(encoded) = flags.get("memory-args-json") {
            let parsed = serde_json::from_str::<Value>(encoded)
                .map_err(|err| format!("memory_args_json_invalid:{err}"))?;
            let rows = parse_string_array(Some(&parsed), 128, 4_096);
            memory_args.extend(rows);
        }

        return Ok((command, memory_args, run_context));
    }

    let mut memory_command = String::new();
    let mut memory_args = Vec::new();
    let mut used_command = false;
    for token in argv {
        if token.starts_with("--") {
            continue;
        }
        if !used_command {
            memory_command = clean_text(Some(token), 64).to_ascii_lowercase();
            used_command = true;
            continue;
        }
        memory_args.push(token.clone());
    }

    if memory_command.is_empty() {
        return Err("missing_memory_command".to_string());
    }

    Ok((memory_command, memory_args, run_context))
}

fn classify_severity(memory_command: &str, op_ok: bool, payload: &Value) -> String {
    if !op_ok {
        return "critical".to_string();
    }
    if memory_command == "recall"
        && payload
            .get("hit_count")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            == 0
    {
        return "warn".to_string();
    }
    "info".to_string()
}

fn should_surface(policy: &MemoryAmbientPolicy, severity: &str) -> bool {
    policy
        .surface_levels
        .iter()
        .any(|level| level.as_str() == severity)
}

fn enqueue_attention(
    root: &Path,
    memory_command: &str,
    severity: &str,
    op_ok: bool,
    run_context: &str,
    summary_line: &str,
) -> Result<Value, String> {
    let summary_hash = crate::deterministic_receipt_hash(&json!({
        "memory_command": memory_command,
        "severity": severity,
        "ok": op_ok,
        "summary": summary_line
    }));
    let event = json!({
        "ts": now_iso(),
        "source": "memory_ambient",
        "source_type": "memory_operation",
        "severity": severity,
        "summary": summary_line,
        "attention_key": format!("memory:{memory_command}:{}", &summary_hash[..16]),
        "memory_command": memory_command,
        "operation_ok": op_ok
    });

    let payload = serde_json::to_string(&event)
        .map_err(|err| format!("attention_event_encode_failed:{err}"))?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(payload.as_bytes());

    let root_buf = root.to_path_buf();
    let (command, mut args) = resolve_infring_ops_command(&root_buf, "attention-queue");
    args.push("enqueue".to_string());
    args.push(format!("--event-json-base64={encoded}"));
    args.push(format!("--run-context={run_context}"));

    let output = Command::new(command)
        .args(args)
        .current_dir(root)
        .env(
            "INFRING_NODE_BINARY",
            std::env::var("INFRING_NODE_BINARY").unwrap_or_else(|_| "node".to_string()),
        )
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|err| format!("attention_queue_spawn_failed:{err}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let mut receipt = ensure_object_payload(
        parse_or_invalid(
            &stdout,
            "attention_queue_enqueue_error",
            "attention_queue_invalid_payload",
        ),
        "attention_queue_enqueue_error",
        "attention_queue_invalid_payload",
    );
    receipt["bridge_exit_code"] = Value::Number((output.status.code().unwrap_or(1) as i64).into());
    if !stderr.trim().is_empty() {
        receipt["bridge_stderr"] = Value::String(clean_text(Some(&stderr), 280));
    }

    let decision = receipt
        .get("decision")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let accepted = matches!(
        decision,
        "admitted" | "deduped" | "backpressure_drop" | "disabled"
    );

    if !output.status.success() && !accepted {
        return Err(format!("attention_queue_enqueue_failed:{decision}"));
    }

    Ok(receipt)
}

fn policy_snapshot(policy: &MemoryAmbientPolicy) -> Value {
    json!({
        "enabled": policy.enabled,
        "rust_authoritative": policy.rust_authoritative,
        "push_attention_queue": policy.push_attention_queue,
        "quiet_non_critical": policy.quiet_non_critical,
        "surface_levels": policy.surface_levels,
        "latest_path": policy.latest_path.to_string_lossy().to_string(),
        "receipts_path": policy.receipts_path.to_string_lossy().to_string()
    })
}

fn update_mech_suit_status(policy: &MemoryAmbientPolicy, patch: Value) {
    let mut latest = read_json(&policy.status_path).unwrap_or_else(|| {
        json!({
            "ts": Value::Null,
            "active": policy.enabled,
            "components": {}
        })
    });
    if !latest.is_object() {
        latest = json!({
            "ts": Value::Null,
            "active": policy.enabled,
            "components": {}
        });
    }

    latest["ts"] = Value::String(now_iso());
    latest["active"] = Value::Bool(policy.enabled);
    if !latest
        .get("components")
        .map(Value::is_object)
        .unwrap_or(false)
    {
        latest["components"] = json!({});
    }
    latest["policy_path"] = Value::String(policy.policy_path.to_string_lossy().to_string());
    latest["components"]["memory"] = patch.clone();
    write_json(&policy.status_path, &latest);

    append_jsonl(
        &policy.history_path,
        &json!({
            "ts": now_iso(),
            "type": "mech_suit_status",
            "component": "memory",
            "active": policy.enabled,
            "patch": patch
        }),
    );
}

fn cli_error_receipt(
    policy: &MemoryAmbientPolicy,
    command: &str,
    reason: &str,
    exit_code: i32,
) -> Value {
    let mut out = json!({
        "ok": false,
        "type": "memory_ambient_error",
        "ts": now_iso(),
        "command": command,
        "reason": reason,
        "exit_code": exit_code,
        "ambient_mode_active": policy.enabled,
        "rust_authoritative": policy.rust_authoritative,
        "policy": policy_snapshot(policy)
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}

fn cryonics_action(memory_args: &[String]) -> String {
    for token in memory_args {
        if let Some(v) = token.strip_prefix("--action=") {
            let out = clean_text(Some(v), 48).to_ascii_lowercase();
            if !out.is_empty() {
                return out;
            }
        }
    }
    memory_args
        .iter()
        .find(|row| !row.trim().starts_with("--"))
        .map(|row| clean_text(Some(row), 48).to_ascii_lowercase())
        .filter(|row| !row.is_empty())
        .unwrap_or_else(|| "run".to_string())
}

fn cryonics_compat_receipt(
    policy: &MemoryAmbientPolicy,
    command: &str,
    run_context: &str,
    memory_args: &[String],
) -> Value {
    let action = cryonics_action(memory_args);
    let mut out = json!({
        "ok": true,
        "type": "memory_ambient_compat",
        "ts": now_iso(),
        "command": command,
        "ambient_mode_active": policy.enabled,
        "rust_authoritative": policy.rust_authoritative,
        "memory_command": "cryonics-tier",
        "compatibility_only": true,
        "action": action,
        "run_context": run_context,
        "memory_args_count": memory_args.len(),
        "memory_args_hash": crate::deterministic_receipt_hash(&json!(memory_args)),
        "severity": "info",
        "surfaced": false,
        "attention_queue": {
            "ok": true,
            "queued": false,
            "decision": "compatibility_no_enqueue",
            "routed_via": "rust_attention_queue"
        },
        "memory_payload": {
            "ok": true,
            "type": "cryonics_tier_compat",
            "action": action,
            "compatibility_only": true
        },
        "policy": policy_snapshot(policy)
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}
