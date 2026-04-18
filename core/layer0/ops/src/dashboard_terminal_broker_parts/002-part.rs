pub fn exec_command(root: &Path, request: &Value) -> Value {
    let sid = normalize_session_id(
        request
            .get("session_id")
            .or_else(|| request.get("sessionId"))
            .and_then(Value::as_str)
            .unwrap_or(""),
    );
    let command = request
        .get("command")
        .and_then(Value::as_str)
        .map(|v| clean_text(v, 4000))
        .unwrap_or_default();
    if sid.is_empty() || command.is_empty() {
        return json!({"ok": false, "error": "session_id_and_command_required"});
    }
    let resolution = match resolve_operator_command(&command) {
        Ok(resolution) => resolution,
        Err(mut err) => {
            err["session_id"] = Value::String(sid.clone());
            return err;
        }
    };
    let requested_command = resolution.requested_command.clone();
    let executed_command = resolution.resolved_command.clone();
    let command_translated = resolution.translated;
    let translation_reason = resolution.translation_reason.clone();
    let suggestions = resolution.suggestions.clone();

    let mut state = load_state(root);
    let sessions = as_object_mut(&mut state, "sessions");
    let Some(session) = sessions.get_mut(&sid) else {
        return json!({"ok": false, "error": "session_not_found", "session_id": sid});
    };
    let cwd = resolve_cwd(
        root,
        request
            .get("cwd")
            .and_then(Value::as_str)
            .unwrap_or_else(|| session.get("cwd").and_then(Value::as_str).unwrap_or("")),
    );
    if !cwd_allowed(root, &cwd) {
        return json!({"ok": false, "error": "cwd_outside_workspace"});
    }
    let permission_gate = if pre_tool_gate_enabled() {
        permission_gate_payload(root, request, &executed_command)
    } else {
        json!({"verdict":"allow","matched":[],"deny_rules_count":0,"ask_rules_count":0})
    };
    let permission_verdict = clean_text(
        permission_gate
            .get("verdict")
            .and_then(Value::as_str)
            .unwrap_or("allow"),
        40,
    )
    .to_ascii_lowercase();
    let nexus_connection =
        match crate::dashboard_tool_turn_loop::authorize_ingress_terminal_command_with_nexus(
            &executed_command,
        ) {
            Ok(meta) => meta,
            Err(err) => {
                let recovery_hints = if recovery_hints_enabled() {
                    command_recovery_hints(&executed_command, 126, "deny")
                } else {
                    Vec::new()
                };
                return json!({
                    "ok": false,
                    "type": "dashboard_terminal_exec",
                    "error": "terminal_nexus_delivery_denied",
                    "message": "Terminal command blocked by hierarchical nexus ingress policy.",
                    "blocked": true,
                    "session_id": request.get("session_id").or_else(|| request.get("sessionId")).cloned().unwrap_or_else(|| Value::String(String::new())),
                    "exit_code": 126,
                    "requested_command": requested_command,
                    "executed_command": executed_command,
                    "command_translated": command_translated,
                    "translation_reason": translation_reason,
                    "suggestions": suggestions,
                    "stdout": "",
                    "stderr": "",
                    "permission_gate": permission_gate,
                    "recovery_hints": recovery_hints,
                    "nexus_error": clean_text(&err, 240)
                });
            }
        };
    let started = Instant::now();
    if pre_tool_gate_enabled() && (permission_verdict == "deny" || permission_verdict == "ask") {
        let blocked_error = if permission_verdict == "ask" {
            "permission_confirmation_required"
        } else {
            "permission_denied_by_policy"
        };
        let blocked_message = if permission_verdict == "ask" {
            "Command requires confirmation before execution."
        } else {
            "Command blocked by terminal command policy."
        };
        let recovery_hints = if recovery_hints_enabled() {
            command_recovery_hints(&executed_command, 126, &permission_verdict)
        } else {
            Vec::new()
        };
        let tracking = maybe_track_command(root, &sid, &executed_command, 0);
        let tool_summary = if tool_summary_enabled() {
            build_tool_summary(
                "blocked",
                &cwd,
                &requested_command,
                &executed_command,
                command_translated,
                &translation_reason,
                &permission_gate,
                126,
                started.elapsed().as_millis() as i64,
                "",
                "",
                &[],
                true,
                &recovery_hints,
            )
        } else {
            Value::Null
        };

        session["cwd"] = Value::String(cwd.to_string_lossy().to_string());
        session["updated_at"] = Value::String(now_iso());
        session["last_exit_code"] = json!(126);
        session["last_output"] = Value::String(String::new());
        session["last_error"] = Value::String(blocked_message.to_string());
        session["last_requested_command"] = Value::String(requested_command.clone());
        session["last_executed_command"] = Value::String(executed_command.clone());
        session["last_command_translated"] = Value::Bool(command_translated);
        session["last_translation_reason"] = Value::String(translation_reason.clone());
        session["last_permission_verdict"] = Value::String(permission_verdict.clone());

        let history = as_array_mut(&mut state, "history");
        history.push(json!({
            "session_id": sid,
            "ts": now_iso(),
            "command": requested_command,
            "requested_command": requested_command,
            "executed_command": executed_command,
            "translated": command_translated,
            "translation_reason": translation_reason,
            "permission_verdict": permission_verdict,
            "exit_code": 126,
            "ok": false,
            "blocked": true
        }));
        if history.len() > 500 {
            let drain = history.len() - 500;
            history.drain(0..drain);
        }
        save_state(root, state);
        return json!({
            "ok": false,
            "type": "dashboard_terminal_exec",
            "error": blocked_error,
            "message": blocked_message,
            "blocked": true,
            "session_id": request.get("session_id").or_else(|| request.get("sessionId")).cloned().unwrap_or_else(|| Value::String(String::new())),
            "exit_code": 126,
            "requested_command": requested_command,
            "executed_command": executed_command,
            "command_translated": command_translated,
            "translation_reason": translation_reason,
            "suggestions": suggestions,
            "stdout": "",
            "stderr": "",
            "permission_gate": permission_gate,
            "recovery_hints": recovery_hints,
            "tool_summary": tool_summary,
            "tracking": tracking.unwrap_or(Value::Null),
            "nexus_connection": nexus_connection.clone().unwrap_or(Value::Null)
        });
    }

    let (ok, code, stdout, stderr) =
        if executed_command.trim().eq_ignore_ascii_case("protheus-ops daemon ping") {
            (true, 0, "pong".to_string(), String::new())
        } else {
            let output = Command::new("zsh")
                .arg("-lc")
                .arg(&executed_command)
                .current_dir(&cwd)
                .output();
            match output {
                Ok(out) => (
                    out.status.success(),
                    out.status.code().unwrap_or(1),
                    truncate_output(&String::from_utf8_lossy(&out.stdout)),
                    truncate_output(&String::from_utf8_lossy(&out.stderr)),
                ),
                Err(err) => (
                    false,
                    127,
                    String::new(),
                    clean_text(&err.to_string(), 2000),
                ),
            }
        };
    let (filtered_stdout, filtered_stderr, filter_events, mut low_signal) =
        apply_post_tool_output_filter(stdout, stderr);
    if clean_text(&filtered_stdout, 200).is_empty() && clean_text(&filtered_stderr, 200).is_empty()
    {
        low_signal = true;
    }
    let recovery_hints = if recovery_hints_enabled() && (low_signal || code != 0) {
        command_recovery_hints(&executed_command, code as i64, &permission_verdict)
    } else {
        Vec::new()
    };
    let tracking = maybe_track_command(
        root,
        &sid,
        &executed_command,
        output_tokens_estimate(&filtered_stdout, &filtered_stderr),
    );
    let tool_summary = if tool_summary_enabled() {
        build_tool_summary(
            if ok { "ok" } else { "error" },
            &cwd,
            &requested_command,
            &executed_command,
            command_translated,
            &translation_reason,
            &permission_gate,
            code as i64,
            started.elapsed().as_millis() as i64,
            &filtered_stdout,
            &filtered_stderr,
            &filter_events,
            low_signal,
            &recovery_hints,
        )
    } else {
        Value::Null
    };

    session["cwd"] = Value::String(cwd.to_string_lossy().to_string());
    session["updated_at"] = Value::String(now_iso());
    session["last_exit_code"] = json!(code);
    session["last_output"] = Value::String(filtered_stdout.clone());
    session["last_error"] = Value::String(filtered_stderr.clone());
    session["last_requested_command"] = Value::String(requested_command.clone());
    session["last_executed_command"] = Value::String(executed_command.clone());
    session["last_command_translated"] = Value::Bool(command_translated);
    session["last_translation_reason"] = Value::String(translation_reason.clone());
    session["last_permission_verdict"] = Value::String(permission_verdict.clone());
    session["last_filter_events"] = Value::Array(
        filter_events
            .iter()
            .map(|row| Value::String(clean_text(row, 120)))
            .collect::<Vec<_>>(),
    );

    let history = as_array_mut(&mut state, "history");
    history.push(json!({
        "session_id": sid,
        "ts": now_iso(),
        "command": requested_command,
        "requested_command": requested_command,
        "executed_command": executed_command,
        "translated": command_translated,
        "translation_reason": translation_reason,
        "permission_verdict": permission_verdict,
        "exit_code": code,
        "ok": ok,
        "low_signal": low_signal
    }));
    if history.len() > 500 {
        let drain = history.len() - 500;
        history.drain(0..drain);
    }
    save_state(root, state);
    json!({
        "ok": ok,
        "type": "dashboard_terminal_exec",
        "session_id": request.get("session_id").or_else(|| request.get("sessionId")).cloned().unwrap_or_else(|| Value::String(String::new())),
        "exit_code": code,
        "requested_command": requested_command,
        "executed_command": executed_command,
        "command_translated": command_translated,
        "translation_reason": translation_reason,
        "suggestions": suggestions,
        "stdout": filtered_stdout,
        "stderr": filtered_stderr,
        "permission_gate": permission_gate,
        "filter_events": filter_events,
        "low_signal_output": low_signal,
        "recovery_hints": recovery_hints,
        "tool_summary": tool_summary,
        "duration_ms": started.elapsed().as_millis() as i64,
        "cwd": cwd.to_string_lossy().to_string(),
        "tracking": tracking.unwrap_or(Value::Null),
        "nexus_connection": nexus_connection.unwrap_or(Value::Null)
    })
}

pub fn handle_http(root: &Path, method: &str, path: &str, body: &[u8]) -> Option<Value> {
    if method == "GET" && path == "/api/terminal/sessions" {
        return Some(sessions_payload(root));
    }
    if method == "POST" && path == "/api/terminal/sessions" {
        return Some(create_session(root, &parse_json(body)));
    }
    if method == "POST" && path == "/api/terminal/queue" {
        return Some(exec_command(root, &parse_json(body)));
    }
    if method == "DELETE" && path.starts_with("/api/terminal/sessions/") {
        let sid = path.trim_start_matches("/api/terminal/sessions/");
        return Some(close_session(root, sid));
    }
    None
}

#[cfg(test)]
