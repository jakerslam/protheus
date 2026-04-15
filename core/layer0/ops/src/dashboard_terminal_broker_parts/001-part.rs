fn build_tool_summary(
    status: &str,
    cwd: &Path,
    requested_command: &str,
    executed_command: &str,
    command_translated: bool,
    translation_reason: &str,
    permission_gate: &Value,
    exit_code: i64,
    duration_ms: i64,
    stdout: &str,
    stderr: &str,
    filter_events: &[String],
    low_signal: bool,
    recovery_hints: &[String],
) -> Value {
    let found = match (
        !clean_text(stdout, 200).is_empty(),
        !clean_text(stderr, 200).is_empty(),
    ) {
        (true, true) => "stdout+stderr",
        (true, false) => "stdout",
        (false, true) => "stderr",
        (false, false) => "none",
    };
    let mut out = json!({
        "status": clean_text(status, 40),
        "cwd": cwd.to_string_lossy().to_string(),
        "requested_command": clean_text(requested_command, 4000),
        "executed_command": clean_text(executed_command, 4000),
        "command_translated": command_translated,
        "translation_reason": clean_text(translation_reason, 240),
        "permission_verdict": clean_text(
            permission_gate.get("verdict").and_then(Value::as_str).unwrap_or("allow"),
            40
        ),
        "permission_matches": permission_gate
            .get("matched")
            .cloned()
            .unwrap_or_else(|| Value::Array(Vec::new())),
        "exit_code": exit_code,
        "duration_ms": duration_ms,
        "found": found,
        "low_signal": low_signal,
        "filter_events": filter_events,
        "recovery_hints": recovery_hints
    });
    if status == "blocked" {
        out["blocked"] = Value::Bool(true);
        out["blocked_reason"] = Value::String(clean_text(
            permission_gate
                .get("verdict")
                .and_then(Value::as_str)
                .unwrap_or("policy"),
            40,
        ));
    }
    out
}

fn memory_context_verify_command() -> String {
    [
        "protheus-ops runtime-systems verify --system-id=V6-MEMORY-CONTEXT-001.1",
        "protheus-ops runtime-systems verify --system-id=V6-MEMORY-CONTEXT-001.2",
        "protheus-ops runtime-systems verify --system-id=V6-MEMORY-CONTEXT-001.3",
        "protheus-ops runtime-systems verify --system-id=V6-MEMORY-CONTEXT-001.4",
        "protheus-ops runtime-systems verify --system-id=V6-MEMORY-CONTEXT-001.5",
    ]
    .join(" && ")
}

fn default_router_suggestions() -> Vec<String> {
    vec![
        "protheus-ops daemon-control diagnostics".to_string(),
        "protheus-ops status --dashboard".to_string(),
        "protheus-ops attention-queue compact --retain=256".to_string(),
        memory_context_verify_command(),
    ]
}

pub fn resolve_operator_command(command: &str) -> Result<CommandResolution, Value> {
    let requested = clean_text(command, 4000);
    if requested.is_empty() {
        return Err(json!({"ok": false, "error": "command_required"}));
    }
    let lowered = requested.to_ascii_lowercase();

    if lowered.starts_with("protheus-ops diagnostic full-scan")
        || lowered.starts_with("protheus-ops diagnostic")
    {
        let resolved = "protheus-ops daemon-control diagnostics && protheus-ops status --dashboard"
            .to_string();
        return Ok(CommandResolution {
            requested_command: requested,
            resolved_command: resolved.clone(),
            translated: true,
            translation_reason: "translated_unsupported_diagnostic_surface_to_daemon_diagnostics"
                .to_string(),
            suggestions: vec![resolved],
        });
    }

    if lowered.starts_with("protheus-ops queue optimize") {
        let retain = if lowered.contains("--strategy=aggressive") {
            128
        } else {
            256
        };
        let resolved =
            format!("protheus-ops attention-queue compact --retain={retain} && protheus-ops attention-queue status");
        return Ok(CommandResolution {
            requested_command: requested,
            resolved_command: resolved.clone(),
            translated: true,
            translation_reason:
                "translated_unsupported_queue_optimize_surface_to_attention_queue_compact"
                    .to_string(),
            suggestions: vec![resolved],
        });
    }

    if lowered.starts_with("infring memory-context validate")
        || lowered.starts_with("protheus-ops memory-context validate")
    {
        let resolved = memory_context_verify_command();
        return Ok(CommandResolution {
            requested_command: requested,
            resolved_command: resolved.clone(),
            translated: true,
            translation_reason:
                "translated_unsupported_memory_context_validate_surface_to_runtime_system_verify"
                    .to_string(),
            suggestions: vec![resolved],
        });
    }

    if lowered == "infring"
        || lowered == "infring help"
        || lowered == "infring --help"
        || lowered == "infring -h"
    {
        let resolved = "protheus-ops command-list-kernel --mode=help".to_string();
        return Ok(CommandResolution {
            requested_command: requested,
            resolved_command: resolved.clone(),
            translated: true,
            translation_reason: "translated_infring_help_surface_to_command_list_help".to_string(),
            suggestions: vec![resolved],
        });
    }

    if lowered == "protheus-ops help"
        || lowered == "protheus-ops --help"
        || lowered == "protheus-ops -h"
    {
        let resolved = "protheus-ops command-list-kernel --mode=help".to_string();
        return Ok(CommandResolution {
            requested_command: requested,
            resolved_command: resolved.clone(),
            translated: true,
            translation_reason: "translated_protheus_help_surface_to_command_list_help".to_string(),
            suggestions: vec![resolved],
        });
    }

    if lowered.starts_with("infring ") {
        let suffix = requested
            .split_once(' ')
            .map(|(_, rest)| rest.trim())
            .unwrap_or("");
        if suffix.is_empty() {
            return Err(json!({
                "ok": false,
                "error": "command_required",
                "message": "infring command requires a subcommand",
                "requested_command": requested,
                "suggestions": default_router_suggestions()
            }));
        }
        let translated = format!("protheus-ops {suffix}");
        return Ok(CommandResolution {
            requested_command: requested,
            resolved_command: translated.clone(),
            translated: true,
            translation_reason: "translated_infring_cli_alias_to_protheus_ops".to_string(),
            suggestions: vec![translated],
        });
    }

    if lowered.starts_with("protheus-ops ") && lowered.contains("full-scan") {
        return Err(json!({
            "ok": false,
            "error": "unsupported_protheus_ops_command_variant",
            "requested_command": requested,
            "suggestions": default_router_suggestions()
        }));
    }

    Ok(CommandResolution {
        requested_command: requested.clone(),
        resolved_command: requested,
        translated: false,
        translation_reason: "passthrough_shell_command".to_string(),
        suggestions: Vec::new(),
    })
}

pub fn sessions_payload(root: &Path) -> Value {
    let state = load_state(root);
    let mut rows = state
        .get("sessions")
        .and_then(Value::as_object)
        .map(|obj| obj.values().cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    rows.sort_by(|a, b| {
        clean_text(a.get("id").and_then(Value::as_str).unwrap_or(""), 120).cmp(&clean_text(
            b.get("id").and_then(Value::as_str).unwrap_or(""),
            120,
        ))
    });
    json!({"ok": true, "sessions": rows})
}

pub fn create_session(root: &Path, request: &Value) -> Value {
    let requested_id = clean_text(request.get("id").and_then(Value::as_str).unwrap_or(""), 120);
    let mut session_id = if requested_id.is_empty() {
        format!(
            "term-{}",
            crate::deterministic_receipt_hash(&json!({"ts": now_iso()}))
                .chars()
                .take(12)
                .collect::<String>()
        )
    } else {
        normalize_session_id(&requested_id)
    };
    if session_id.is_empty() {
        session_id = "term-default".to_string();
    }
    let cwd = resolve_cwd(
        root,
        request.get("cwd").and_then(Value::as_str).unwrap_or(""),
    );
    if !cwd_allowed(root, &cwd) {
        return json!({"ok": false, "error": "cwd_outside_workspace"});
    }
    let mut state = load_state(root);
    let sessions = as_object_mut(&mut state, "sessions");
    sessions.insert(
        session_id.clone(),
        json!({
            "id": session_id,
            "cwd": cwd.to_string_lossy().to_string(),
            "created_at": now_iso(),
            "updated_at": now_iso(),
            "last_exit_code": Value::Null,
            "last_output": ""
        }),
    );
    let out = sessions
        .get(&session_id)
        .cloned()
        .unwrap_or_else(|| json!({}));
    save_state(root, state);
    json!({"ok": true, "type": "dashboard_terminal_session_create", "session": out})
}

pub fn close_session(root: &Path, session_id: &str) -> Value {
    let sid = normalize_session_id(session_id);
    if sid.is_empty() {
        return json!({"ok": false, "error": "session_id_required"});
    }
    let mut state = load_state(root);
    let removed = as_object_mut(&mut state, "sessions").remove(&sid).is_some();
    save_state(root, state);
    json!({"ok": true, "type": "dashboard_terminal_session_close", "session_id": sid, "removed": removed})
}

