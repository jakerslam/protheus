
fn enqueue_attention(persona: &str, patch_hash: &str, run_context: &str) -> Result<Value, String> {
    let root = repo_root_from_current_dir();
    let event = json!({
        "ts": now_iso(),
        "source": "persona_ambient",
        "source_type": "persona_stance_apply",
        "severity": "info",
        "summary": format!("persona ambient stance apply ({persona})"),
        "attention_key": format!("persona_stance:{persona}:{patch_hash}"),
        "persona": persona,
        "patch_hash": patch_hash
    });

    let payload = serde_json::to_string(&event)
        .map_err(|err| format!("attention_event_encode_failed:{err}"))?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(payload.as_bytes());

    let (command, mut args) = resolve_infring_ops_command(&root, "attention-queue");
    args.push("enqueue".to_string());
    args.push(format!("--event-json-base64={encoded}"));
    args.push(format!("--run-context={run_context}"));

    let output = Command::new(&command)
        .args(&args)
        .current_dir(&root)
        .env(
            "INFRING_NODE_BINARY",
            std::env::var("INFRING_NODE_BINARY").unwrap_or_else(|_| "node".to_string()),
        )
        .output()
        .map_err(|err| format!("attention_queue_spawn_failed:{err}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let exit_code = output.status.code().unwrap_or(1);
    let mut receipt = parse_json_payload(&stdout).unwrap_or_else(|| {
        json!({
            "ok": false,
            "type": "attention_queue_enqueue_error",
            "reason": "attention_queue_empty_payload",
            "exit_code": exit_code,
            "stderr": clean_text(Some(&stderr), 280)
        })
    });

    if !receipt.is_object() {
        receipt = json!({
            "ok": false,
            "type": "attention_queue_enqueue_error",
            "reason": "attention_queue_invalid_payload",
            "exit_code": exit_code,
            "stderr": clean_text(Some(&stderr), 280)
        });
    }
    receipt["bridge_exit_code"] = Value::Number((exit_code as i64).into());
    if !stderr.trim().is_empty() {
        receipt["bridge_stderr"] = Value::String(clean_text(Some(&stderr), 280));
    }

    let decision = receipt
        .get("decision")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let accepted = matches!(decision, "admitted" | "deduped" | "disabled");

    if exit_code != 0 && !accepted {
        return Err(format!("attention_queue_enqueue_failed:{decision}"));
    }
    Ok(receipt)
}

fn policy_snapshot(policy: &PersonaAmbientPolicy) -> Value {
    json!({
        "enabled": policy.enabled,
        "ambient_stance": policy.ambient_stance,
        "auto_apply": policy.auto_apply,
        "full_reload": policy.full_reload,
        "push_attention_queue": policy.push_attention_queue,
        "cache_path": policy.cache_path.to_string_lossy().to_string(),
        "latest_path": policy.latest_path.to_string_lossy().to_string(),
        "receipts_path": policy.receipts_path.to_string_lossy().to_string(),
        "max_personas": policy.max_personas,
        "max_patch_bytes": policy.max_patch_bytes
    })
}

fn emit(value: &Value) {
    println!(
        "{}",
        serde_json::to_string(value).unwrap_or_else(|_| {
            "{\"ok\":false,\"type\":\"persona_ambient_encode_failed\"}".to_string()
        })
    );
}

fn stamp_receipt(value: &mut Value) {
    value["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(value));
}

fn persist_and_emit(latest_path: &Path, receipts_path: &Path, value: &mut Value) {
    stamp_receipt(value);
    write_json(latest_path, value);
    append_jsonl(receipts_path, value);
    emit(value);
}

fn fail_receipt(
    policy: &PersonaAmbientPolicy,
    command: &str,
    reason: &str,
    detail: Option<Value>,

) -> Value {
    let mut out = json!({
        "ok": false,
        "type": "persona_ambient_error",
        "ts": now_iso(),
        "command": command,
        "reason": reason,
        "ambient_mode_active": policy.enabled && policy.ambient_stance,
        "policy": policy_snapshot(policy)
    });
    if let Some(extra) = detail {
        out["detail"] = extra;
    }
    stamp_receipt(&mut out);
    out
}

fn stance_diff(
    current: &Map<String, Value>,
    patch: &Map<String, Value>,
    full_reload: bool,
) -> (Map<String, Value>, Vec<String>, Vec<String>) {
    if full_reload {
        let mut changed = Vec::new();
        for (k, v) in patch {
            if current.get(k) != Some(v) {
                changed.push(k.clone());
            }
        }
        let mut removed = Vec::new();
        for key in current.keys() {
            if !patch.contains_key(key) {
                removed.push(key.clone());
            }
        }
        return (patch.clone(), changed, removed);
    }

    let mut next = current.clone();
    let mut changed = Vec::new();
    let mut removed = Vec::new();

    for (key, value) in patch {
        if value.is_null() {
            if next.remove(key).is_some() {
                removed.push(key.clone());
            }
            continue;
        }
        if next.get(key) != Some(value) {
            next.insert(key.clone(), value.clone());
            changed.push(key.clone());
        }
    }

    (next, changed, removed)
}
