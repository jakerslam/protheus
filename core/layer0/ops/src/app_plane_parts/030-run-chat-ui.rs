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
    let provider = settings
        .get("provider")
        .and_then(Value::as_str)
        .unwrap_or(default_provider.as_str())
        .to_string();
    let model = settings
        .get("model")
        .and_then(Value::as_str)
        .unwrap_or(default_model.as_str())
        .to_string();
    let message = message_from_parsed(parsed, 2, "hello from chat ui");
    if strict && message.trim().is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "app_plane_chat_ui",
            "action": "run",
            "errors": ["chat_ui_message_required"]
        });
    }
    let mut selected_provider = provider.clone();
    let mut selected_model = model.clone();
    let (resolved_provider, resolved_model, _) =
        crate::dashboard_model_catalog::resolve_model_selection(
            root,
            &json!({
                "app": {
                    "settings": {
                        "provider": settings.get("provider").cloned().unwrap_or_else(|| json!(provider.clone())),
                        "model": settings.get("model").cloned().unwrap_or_else(|| json!(model.clone()))
                    }
                }
            }),
            &selected_provider,
            &selected_model,
            &json!({
                "task_type": "general",
                "message": message,
                "token_count": ((message.len() as i64) / 4).max(1)
            }),
        );
    selected_provider = resolved_provider;
    selected_model = resolved_model;
    let system_prompt = clean(parsed.flags.get("system").cloned().unwrap_or_else(|| "You are an Infring dashboard runtime agent. You have host-integrated access to runtime telemetry, agent session memory, and approved protheus/infring command surfaces. Never claim you lack system access; if a value is missing, request a runtime sync or the exact command needed and continue.".to_string()), 12_000);
    let history_messages = chat_ui_history_messages(&session);
    let invoke = crate::dashboard_provider_runtime::invoke_chat(
        root,
        &selected_provider,
        &selected_model,
        &system_prompt,
        &history_messages,
        &message,
    );
    let response = match invoke {
        Ok(value) => value,
        Err(err) => {
            return json!({
                "ok": false,
                "strict": strict,
                "type": "app_plane_chat_ui",
                "action": "run",
                "provider": selected_provider,
                "model": selected_model,
                "errors": [clean(err, 240)]
            });
        }
    };
    let assistant = clean(
        response
            .get("response")
            .and_then(Value::as_str)
            .unwrap_or(""),
        16_000,
    );
    let turn = json!({
        "turn_id": format!(
            "turn_{}",
            &sha256_hex_str(&format!("{}:{}:{}:{}", session_id, selected_provider, selected_model, crate::now_iso()))[..10]
        ),
        "ts": crate::now_iso(),
        "provider": selected_provider,
        "model": selected_model,
        "user": message,
        "assistant": assistant
    });
    let mut turns = session
        .get("turns")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    turns.push(turn.clone());
    session["turns"] = Value::Array(turns);
    session["updated_at"] = Value::String(crate::now_iso());
    let _ = write_json(&path, &session);
    let _ = append_jsonl(
        &state_root(root).join("chat_ui").join("history.jsonl"),
        &json!({"action":"run","session_id":session_id,"turn":turn,"ts":crate::now_iso()}),
    );
    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "app_plane_chat_ui",
        "lane": "core/layer0/ops",
        "action": "run",
        "session_id": session_id,
        "turn": turn,
        "provider": response.get("provider").cloned().unwrap_or_else(|| json!(provider)),
        "model": response.get("model").cloned().unwrap_or_else(|| json!(model)),
        "runtime_model": response.get("runtime_model").cloned().unwrap_or_else(|| json!(selected_model)),
        "input_tokens": response.get("input_tokens").cloned().unwrap_or_else(|| json!(0)),
        "output_tokens": response.get("output_tokens").cloned().unwrap_or_else(|| json!(0)),
        "cost_usd": response.get("cost_usd").cloned().unwrap_or_else(|| json!(0.0)),
        "context_window": response.get("context_window").cloned().unwrap_or_else(|| json!(0)),
        "tools": response.get("tools").cloned().unwrap_or_else(|| json!([])),
        "artifact": {
            "path": path.display().to_string(),
            "sha256": sha256_hex_str(&session.to_string())
        },
        "claim_evidence": [
            {
                "id": "V6-APP-007.1",
                "claim": "chat_ui_runs_multi_provider_conversation_with_receipted_model_calls",
                "evidence": {
                    "provider": settings.get("provider"),
                    "model": settings.get("model"),
                    "session_id": session_id
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}

fn ensure_file(path: &Path, content: &str) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("missing_parent:{}", path.display()))?;
    fs::create_dir_all(parent).map_err(|e| format!("mkdir_failed:{}:{e}", parent.display()))?;
    fs::write(path, content).map_err(|e| format!("write_failed:{}:{e}", path.display()))
}

fn code_engineer_templates_path(root: &Path) -> PathBuf {
    state_root(root)
        .join("code_engineer")
        .join("builders_templates.json")
}

fn slug_from_goal(goal: &str, fallback_prefix: &str) -> String {
    let mut out = String::new();
    for ch in goal.chars() {
        if out.len() >= 48 {
            break;
        }
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if ch.is_ascii_whitespace() || ch == '-' || ch == '_' {
            out.push('-');
        }
    }
    let trimmed = out.trim_matches('-');
    if trimmed.is_empty() {
        format!("{fallback_prefix}-{}", &sha256_hex_str("default")[..8])
    } else {
        trimmed.to_string()
    }
}

fn classify_builder_risk(goal: &str, explicit: Option<&String>) -> String {
    if let Some(raw) = explicit {
        let normalized = raw.trim().to_ascii_lowercase();
        if matches!(normalized.as_str(), "low" | "medium" | "high") {
            return normalized;
        }
    }
    let lower = goal.to_ascii_lowercase();
    let high_terms = [
        "delete",
        "drop table",
        "production",
        "payment",
        "security",
        "auth bypass",
    ];
    if high_terms.iter().any(|term| lower.contains(term)) {
        return "high".to_string();
    }
    let medium_terms = [
        "deploy",
        "migration",
        "schema",
        "customer data",
        "live traffic",
    ];
    if medium_terms.iter().any(|term| lower.contains(term)) {
        return "medium".to_string();
    }
    "low".to_string()
}

fn build_reasoning_receipt(contract: &Value, goal: &str, risk: &str, approved: bool) -> Value {
    let auto_allow = contract
        .get("reasoning_gate")
        .and_then(|v| v.get("auto_allow_risks"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_else(|| vec![Value::String("low".to_string())]);
    let auto_allow_risks = auto_allow
        .iter()
        .filter_map(Value::as_str)
        .map(|v| v.to_ascii_lowercase())
        .collect::<Vec<_>>();
    let requires_explicit_approval = !auto_allow_risks.iter().any(|v| v == risk);
    let continue_allowed = !requires_explicit_approval || approved;
    let mut out = json!({
        "type": "app_plane_reasoning_gate",
        "goal": clean(goal, 2000),
        "risk_class": risk,
        "approved": approved,
        "requires_explicit_approval": requires_explicit_approval,
        "continue_allowed": continue_allowed,
        "plan": [
            {"stage":"research","intent":"collect constraints and edge cases"},
            {"stage":"plan","intent":"derive execution graph and acceptance criteria"},
            {"stage":"code","intent":"materialize deterministic artifacts"},
            {"stage":"test","intent":"run bounded verification and critique loop"},
            {"stage":"package","intent":"emit delivery manifest with provenance"}
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}
