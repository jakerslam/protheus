
fn validate_value(root: &Path, payload: &Map<String, Value>) -> Value {
    let args = as_array(payload.get("args"))
        .iter()
        .map(|value| as_str(Some(value)))
        .collect::<Vec<_>>();
    let options = as_object(payload.get("options"))
        .cloned()
        .unwrap_or_default();
    let (positional, flags) = parse_cli_args(&args);
    let command = {
        let explicit = as_str(options.get("command"));
        if !explicit.is_empty() {
            explicit.to_ascii_lowercase()
        } else {
            positional
                .first()
                .map(|value| value.to_ascii_lowercase())
                .unwrap_or_else(|| "status".to_string())
        }
    };
    let require_session = to_bool(
        options
            .get("requireSession")
            .or_else(|| options.get("require_session")),
        true,
    ) && !matches!(command.as_str(), "status" | "verify" | "health" | "help");
    let session_id = find_session_id(&flags, &options);

    if require_session && session_id.is_empty() {
        return json!({
            "ok": false,
            "type": "memory_session_isolation",
            "reason_code": "missing_session_id",
            "command": command
        });
    }
    if !session_id.is_empty() && !session_id_pattern().is_match(&session_id) {
        return json!({
            "ok": false,
            "type": "memory_session_isolation",
            "reason_code": "invalid_session_id",
            "session_id": session_id
        });
    }

    let resource_keys = collect_resource_keys(&flags);
    if resource_keys.is_empty() {
        return json!({
            "ok": true,
            "type": "memory_session_isolation",
            "reason_code": "no_resource_keys",
            "command": command,
            "session_id": if session_id.is_empty() { Value::Null } else { Value::String(session_id) }
        });
    }

    let state_path = state_path_from_map(root, &options);
    let mut state = load_state_value(&state_path)
        .as_object()
        .cloned()
        .unwrap_or_default();
    let mut resources = state
        .get("resources")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    for key in &resource_keys {
        let existing = resources.get(key);
        let existing_session = existing
            .and_then(Value::as_object)
            .and_then(|obj| obj.get("session_id"))
            .and_then(Value::as_str)
            .unwrap_or("");
        if !existing_session.is_empty() && !session_id.is_empty() && existing_session != session_id
        {
            return json!({
                "ok": false,
                "type": "memory_session_isolation",
                "reason_code": "cross_session_leak_blocked",
                "resource_key": key,
                "expected_session_id": existing_session,
                "session_id": session_id
            });
        }
    }

    let persist = to_bool(options.get("persist"), true);
    if persist && !session_id.is_empty() {
        let now = now_iso();
        for key in &resource_keys {
            resources.insert(
                key.to_string(),
                json!({
                    "session_id": session_id,
                    "last_seen_at": now
                }),
            );
        }
        state.insert("resources".to_string(), Value::Object(resources));
        if let Err(err) = save_state_value(&state_path, &Value::Object(state)) {
            return json!({
                "ok": false,
                "type": "memory_session_isolation",
                "reason_code": "state_persist_failed",
                "command": command,
                "session_id": session_id,
                "error": err
            });
        }
    }

    json!({
        "ok": true,
        "type": "memory_session_isolation",
        "reason_code": "session_isolation_ok",
        "session_id": if session_id.is_empty() { Value::Null } else { Value::String(session_id) },
        "resource_key_count": resource_keys.len()
    })
}

fn failure_result_value(payload: &Map<String, Value>) -> Value {
    let validation = as_object(payload.get("validation"))
        .cloned()
        .unwrap_or_default();
    let context = as_object(payload.get("context"))
        .cloned()
        .unwrap_or_default();
    let reason = as_str(validation.get("reason_code"));
    let mut envelope = Map::new();
    envelope.insert("ok".to_string(), Value::Bool(false));
    envelope.insert(
        "type".to_string(),
        Value::String("memory_session_isolation_reject".to_string()),
    );
    envelope.insert(
        "reason".to_string(),
        Value::String(if reason.is_empty() {
            "session_isolation_failed".to_string()
        } else {
            reason.clone()
        }),
    );
    envelope.insert("fail_closed".to_string(), Value::Bool(true));
    for (key, value) in context {
        envelope.insert(key, value);
    }
    let payload = Value::Object(envelope.clone());
    json!({
        "ok": false,
        "status": 2,
        "stdout": format!("{}\n", serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string())),
        "stderr": format!(
            "memory_session_isolation_reject:{}\n",
            payload.get("reason").and_then(Value::as_str).unwrap_or("session_isolation_failed")
        ),
        "payload": Value::Object(envelope)
    })
}

fn run_command(root: &Path, command: &str, payload: &Map<String, Value>) -> Result<Value, String> {
    match command {
        "load-state" => {
            let state_path = state_path_from_map(root, payload);
            Ok(json!({
                "ok": true,
                "state": load_state_value(&state_path)
            }))
        }
        "save-state" => {
            let state_path = state_path_from_map(root, payload);
            let state = payload
                .get("state")
                .cloned()
                .unwrap_or_else(default_state_value);
            Ok(json!({
                "ok": true,
                "state": save_state_value(&state_path, &state)?
            }))
        }
        "validate" => {
            let args = as_array(payload.get("args"))
                .iter()
                .map(|value| as_str(Some(value)))
                .collect::<Vec<_>>();
            Ok(json!({
                "ok": true,
                "validation": validate_value(root, payload),
                "parsed": parsed_args_value(&args)
            }))
        }
        "failure-result" => Ok(json!({
            "ok": true,
            "result": failure_result_value(payload)
        })),
        _ => Err("memory_session_isolation_kernel_unknown_command".to_string()),
    }
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let Some(command) = argv.first().map(|value| value.as_str()) else {
        usage();
        return 1;
    };
    if matches!(command, "help" | "--help" | "-h") {
        usage();
        return 0;
    }
    let payload = match payload_json(argv) {
        Ok(value) => value,
        Err(err) => {
            print_json_line(&cli_error("memory_session_isolation_kernel", &err));
            return 1;
        }
    };
    let payload = payload_obj(&payload).clone();
    match run_command(root, command, &payload) {
        Ok(out) => {
            print_json_line(&cli_receipt("memory_session_isolation_kernel", out));
            0
        }
        Err(err) => {
            print_json_line(&cli_error("memory_session_isolation_kernel", &err));
            1
        }
    }
}
