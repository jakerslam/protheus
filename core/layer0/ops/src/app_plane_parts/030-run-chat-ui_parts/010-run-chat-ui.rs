fn run_chat_ui(root: &Path, parsed: &crate::ParsedArgs, strict: bool, action: &str) -> Value {
    let contract = load_json_or(
        root,
        CHAT_UI_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "chat_ui_contract",
            "providers": ["openai", "frontier_provider", "google", "gemini", "groq", "deepseek", "openrouter", "xai", "ollama", "claude-code"],
            "default_provider": "openai",
            "default_model": "gpt-5"
        }),
    );
    let providers = contract
        .get("providers")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| row.as_str().map(str::to_string))
        .collect::<Vec<_>>();
    let default_provider = contract
        .get("default_provider")
        .and_then(Value::as_str)
        .unwrap_or("openai")
        .to_string();
    let default_model = contract
        .get("default_model")
        .and_then(Value::as_str)
        .unwrap_or("gpt-5")
        .to_string();

    let mut settings = read_json(&chat_ui_settings_path(root)).unwrap_or_else(|| {
        json!({
            "provider": default_provider,
            "model": default_model,
            "updated_at": crate::now_iso()
        })
    });
    let session_id = clean_id(
        parsed
            .flags
            .get("session-id")
            .map(String::as_str)
            .or_else(|| parsed.flags.get("session").map(String::as_str)),
        "chat-ui-default",
    );
    let path = chat_ui_session_path(root, &session_id);
    let mut session = read_json(&path).unwrap_or_else(|| {
        json!({
            "version": "v1",
            "session_id": session_id,
            "turns": []
        })
    });
    if !session.get("turns").map(Value::is_array).unwrap_or(false) {
        session["turns"] = Value::Array(Vec::new());
    }

    if action == "switch-provider" {
        let provider = clean(
            parsed
                .flags
                .get("provider")
                .cloned()
                .or_else(|| parsed.positional.get(2).cloned())
                .unwrap_or_else(|| default_provider.clone()),
            60,
        )
        .to_ascii_lowercase();
        if strict && !providers.iter().any(|row| row == &provider) {
            return json!({
                "ok": false,
                "strict": strict,
                "type": "app_plane_chat_ui",
                "action": action,
                "errors": ["chat_ui_provider_invalid"]
            });
        }
        let model = clean(
            parsed
                .flags
                .get("model")
                .cloned()
                .unwrap_or_else(|| format!("{}-default", provider)),
            120,
        );
        settings["provider"] = Value::String(provider.clone());
        settings["model"] = Value::String(model.clone());
        settings["updated_at"] = Value::String(crate::now_iso());
        let _ = write_json(&chat_ui_settings_path(root), &settings);
        let mut out = json!({
            "ok": true,
            "strict": strict,
            "type": "app_plane_chat_ui",
            "lane": "core/layer0/ops",
            "action": action,
            "provider": provider,
            "model": model,
            "artifact": {
                "path": chat_ui_settings_path(root).display().to_string(),
                "sha256": sha256_hex_str(&settings.to_string())
            },
            "claim_evidence": [
                {
                    "id": "V6-APP-007.1",
                    "claim": "chat_ui_switches_provider_and_model_with_deterministic_receipts",
                    "evidence": {
                        "provider": settings.get("provider"),
                        "model": settings.get("model")
                    }
                }
            ]
        });
        out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
        return out;
    }

    if matches!(action, "history" | "status") {
        let mut out = json!({
            "ok": true,
            "strict": strict,
            "type": "app_plane_chat_ui",
            "lane": "core/layer0/ops",
            "action": action,
            "session_id": session_id,
            "settings": settings,
            "turn_count": session.get("turns").and_then(Value::as_array).map(|rows| rows.len()).unwrap_or(0),
            "turns": if action == "history" { session.get("turns").cloned().unwrap_or_else(|| Value::Array(Vec::new())) } else { Value::Array(Vec::new()) },
            "claim_evidence": [
                {
                    "id": "V6-APP-007.1",
                    "claim": "chat_ui_surfaces_sidebar_history_and_provider_settings_over_core_receipts",
                    "evidence": {
                        "session_id": session_id
                    }
                }
            ]
        });
        out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
        return out;
    }

    if action == "replay" {
        let turn_index = parse_u64(parsed.flags.get("turn"), 0) as usize;
        let turns = session
            .get("turns")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let selected = if turns.is_empty() {
            None
        } else if turn_index >= turns.len() {
            turns.last().cloned()
        } else {
            turns.get(turn_index).cloned()
        };
        if strict && selected.is_none() {
            return json!({
                "ok": false,
                "strict": strict,
                "type": "app_plane_chat_ui",
                "action": "replay",
                "errors": ["chat_ui_turn_not_found"]
            });
        }
        let mut out = json!({
            "ok": true,
            "strict": strict,
            "type": "app_plane_chat_ui",
            "lane": "core/layer0/ops",
            "action": "replay",
            "session_id": session_id,
            "turn": selected,
            "turn_index": turn_index,
            "claim_evidence": [
                {
                    "id": "V6-APP-007.1",
                    "claim": "chat_ui_replay_supports_receipted_history_sidebar_navigation",
                    "evidence": {
                        "session_id": session_id,
                        "turn_index": turn_index
                    }
                }
            ]
        });
        out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
        return out;
    }
    if action == "view-logs" {
        let request_id = clean(
            parsed
                .flags
                .get("request-id")
                .cloned()
                .or_else(|| parsed.flags.get("trace-id").cloned())
                .or_else(|| parsed.flags.get("call-id").cloned())
                .or_else(|| parsed.positional.get(2).cloned())
                .unwrap_or_default(),
            160,
        );
        if strict && request_id.is_empty() {
            return json!({
                "ok": false,
                "strict": strict,
                "type": "app_plane_chat_ui",
                "action": "view-logs",
                "errors": ["chat_ui_request_id_required"]
            });
        }
        let history_path = state_root(root).join("chat_ui").join("history.jsonl");
        let rows = chat_ui_read_jsonl_rows(&history_path, 400);
        let request_key = request_id.to_ascii_lowercase();
        let mut matches = Vec::<Value>::new();
        for row in rows.into_iter().rev() {
            if request_key.is_empty() {
                matches.push(row);
            } else {
                let row_blob = clean(&row.to_string(), 20_000).to_ascii_lowercase();
                if row_blob.contains(&request_key) {
                    matches.push(row);
                }
            }
            if matches.len() >= 24 {
                break;
            }
        }
        let mut out = json!({
            "ok": true,
            "strict": strict,
            "type": "app_plane_chat_ui",
            "lane": "core/layer0/ops",
            "action": "view-logs",
            "session_id": session_id,
            "request_id": request_id,
            "history_path": history_path.display().to_string(),
            "match_count": matches.len(),
            "matches": matches,
            "claim_evidence": [
                {
                    "id": "V6-APP-007.1",
                    "claim": "chat_ui_supports_request_trace_debug_lookup_with_receipted_results",
                    "evidence": {
                        "session_id": session_id
                    }
                }
            ]
        });
        out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
        return out;
    }

    let (
        provider,
        model,
        message,
        selected_provider,
        selected_model,
        response,
        mut tools,
        requires_live_web,
        assistant_raw,
        forced_web_outcome,
        forced_web_error_code,
        forced_web_fallback,
        detected_tool_surface_error,
    ) = include!("010-run-chat-ui_parts/010-run-turn-invoke-and-web-fallback.expr.rs");

    let (
        assistant,
        hard_guard,
        forced_web_error_code,
        final_outcome,
        response_tool_surface_error_code,
        tool_diagnostics,
        receipt_summary,
        rewrite_outcome,
        web_classification,
        guard_retry_recommended,
        guard_retry_strategy,
        guard_retry_lane,
        guard_retry_plan,
        retry_suppressed_by_loop_risk,
        retry_loop_risk,
        classification_guard,
    ) = include!("010-run-chat-ui_parts/020-run-turn-finalize-guards-and-classification.expr.rs");

    include!("010-run-chat-ui_parts/030-run-turn-persist-and-build-output.expr.rs")
}
