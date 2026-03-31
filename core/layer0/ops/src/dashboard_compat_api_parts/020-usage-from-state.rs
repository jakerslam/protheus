fn usage_from_state(root: &Path, snapshot: &Value) -> Value {
    let (provider, model) = extract_app_settings(root, snapshot);
    let roster = build_agent_roster(root, snapshot, false);
    let mut agent_rows = Vec::<Value>::new();
    let mut total_input_tokens = 0_i64;
    let mut total_output_tokens = 0_i64;
    let mut total_tool_calls = 0_i64;
    let mut total_cost_usd = 0.0_f64;
    let mut total_call_count = 0_i64;
    let mut earliest_event_date = String::new();
    let mut by_model = HashMap::<String, (String, String, i64, i64, i64, f64)>::new();
    let mut daily = HashMap::<String, (i64, i64, i64, f64)>::new();

    for row in roster {
        let agent_id = clean_agent_id(row.get("agent_id").and_then(Value::as_str).unwrap_or(""));
        if agent_id.is_empty() {
            continue;
        }
        let state = load_session_state(root, &agent_id);
        let sessions = state
            .get("sessions")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let mut input_tokens = 0_i64;
        let mut output_tokens = 0_i64;
        let mut call_count = 0_i64;
        for session in sessions {
            let messages = session
                .get("messages")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            for message in messages {
                let role = clean_text(
                    message.get("role").and_then(Value::as_str).unwrap_or(""),
                    40,
                )
                .to_ascii_lowercase();
                let text = message_text(&message);
                if text.is_empty() {
                    continue;
                }
                let tokens = estimate_tokens(&text);
                if role == "assistant" || role == "agent" {
                    output_tokens += tokens;
                    call_count += 1;
                } else {
                    input_tokens += tokens;
                }
                let timestamp = message_timestamp_iso(&message);
                let date = timestamp.chars().take(10).collect::<String>();
                if !date.is_empty() {
                    let entry = daily.entry(date.clone()).or_insert((0, 0, 0, 0.0));
                    entry.0 += if role == "assistant" || role == "agent" {
                        1
                    } else {
                        0
                    };
                    entry.1 += if role == "assistant" || role == "agent" {
                        0
                    } else {
                        tokens
                    };
                    entry.2 += if role == "assistant" || role == "agent" {
                        tokens
                    } else {
                        0
                    };
                    if earliest_event_date.is_empty() || date < earliest_event_date {
                        earliest_event_date = date;
                    }
                }
            }
        }
        let tool_calls = 0_i64;
        let cost_usd = 0.0_f64;
        let total_tokens = input_tokens + output_tokens;
        let model_provider = clean_text(
            row.get("model_provider")
                .and_then(Value::as_str)
                .unwrap_or("auto"),
            80,
        );
        let model_name = clean_text(
            row.get("model_name").and_then(Value::as_str).unwrap_or(""),
            120,
        );
        let model_key = if model_name.is_empty() {
            format!("{model_provider}/auto")
        } else {
            format!("{model_provider}/{model_name}")
        };
        let entry = by_model.entry(model_key.clone()).or_insert((
            model_provider.clone(),
            model_name.clone(),
            0,
            0,
            0,
            0.0,
        ));
        entry.2 += input_tokens;
        entry.3 += output_tokens;
        entry.4 += call_count;
        entry.5 += cost_usd;

        total_input_tokens += input_tokens;
        total_output_tokens += output_tokens;
        total_tool_calls += tool_calls;
        total_cost_usd += cost_usd;
        total_call_count += call_count;
        agent_rows.push(json!({
            "agent_id": agent_id,
            "id": row.get("id").cloned().unwrap_or_else(|| json!("")),
            "name": row.get("name").cloned().unwrap_or_else(|| json!("Agent")),
            "role": row.get("role").cloned().unwrap_or_else(|| json!("analyst")),
            "state": row.get("state").cloned().unwrap_or_else(|| json!("Idle")),
            "model_provider": model_provider,
            "model_name": model_name,
            "total_input_tokens": input_tokens,
            "total_output_tokens": output_tokens,
            "total_tokens": total_tokens,
            "tool_calls": tool_calls,
            "cost_usd": cost_usd,
            "daily_cost_usd": 0.0,
            "hourly_limit": 0.0,
            "daily_limit": 0.0,
            "monthly_limit": 0.0,
            "max_llm_tokens_per_hour": 0,
            "call_count": call_count,
            "updated_at": row.get("updated_at").cloned().unwrap_or_else(|| json!(""))
        }));
    }

    agent_rows.sort_by_key(|row| {
        std::cmp::Reverse(clean_text(
            row.get("updated_at").and_then(Value::as_str).unwrap_or(""),
            80,
        ))
    });

    let mut model_rows = by_model
        .into_iter()
        .map(
            |(
                key,
                (provider_id, model_name, input_tokens, output_tokens, call_count, cost_usd),
            )| {
                json!({
                    "provider": provider_id,
                    "model": if model_name.is_empty() { key } else { model_name },
                    "total_input_tokens": input_tokens,
                    "total_output_tokens": output_tokens,
                    "total_cost_usd": cost_usd,
                    "call_count": call_count
                })
            },
        )
        .collect::<Vec<_>>();
    model_rows.sort_by(|a, b| {
        clean_text(b.get("model").and_then(Value::as_str).unwrap_or(""), 160).cmp(&clean_text(
            a.get("model").and_then(Value::as_str).unwrap_or(""),
            160,
        ))
    });

    let today = crate::now_iso().chars().take(10).collect::<String>();
    let mut daily_rows = daily
        .into_iter()
        .map(
            |(date, (requests, input_tokens, output_tokens, cost_usd))| {
                json!({
                    "date": date,
                    "requests": requests,
                    "input_tokens": input_tokens,
                    "output_tokens": output_tokens,
                    "cost_usd": cost_usd
                })
            },
        )
        .collect::<Vec<_>>();
    if daily_rows.is_empty() {
        daily_rows.push(json!({
            "date": today,
            "requests": total_call_count,
            "input_tokens": total_input_tokens,
            "output_tokens": total_output_tokens,
            "cost_usd": total_cost_usd
        }));
        if earliest_event_date.is_empty() && !agent_rows.is_empty() {
            earliest_event_date = today.clone();
        }
    }
    daily_rows.sort_by(|a, b| {
        clean_text(a.get("date").and_then(Value::as_str).unwrap_or(""), 20).cmp(&clean_text(
            b.get("date").and_then(Value::as_str).unwrap_or(""),
            20,
        ))
    });
    let today_cost_usd = daily_rows
        .iter()
        .find(|row| clean_text(row.get("date").and_then(Value::as_str).unwrap_or(""), 20) == today)
        .and_then(|row| row.get("cost_usd").and_then(Value::as_f64))
        .unwrap_or(0.0);

    json!({
        "agents": agent_rows,
        "summary": {
            "requests": total_call_count,
            "call_count": total_call_count,
            "input_tokens": total_input_tokens,
            "output_tokens": total_output_tokens,
            "total_input_tokens": total_input_tokens,
            "total_output_tokens": total_output_tokens,
            "total_cost_usd": total_cost_usd,
            "total_tool_calls": total_tool_calls,
            "active_provider": provider,
            "active_model": model
        },
        "models": model_rows,
        "daily": daily_rows,
        "today_cost_usd": today_cost_usd,
        "first_event_date": earliest_event_date
    })
}

fn providers_payload(root: &Path, snapshot: &Value) -> Value {
    crate::dashboard_provider_runtime::providers_payload(root, snapshot)
}

fn config_payload(root: &Path, snapshot: &Value) -> Value {
    let (provider, model) = effective_app_settings(root, snapshot);
    let llm_ready = crate::dashboard_model_catalog::catalog_payload(root, snapshot)
        .get("models")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .any(|row| row.get("available").and_then(Value::as_bool) == Some(true))
        })
        .unwrap_or(false);
    json!({
        "ok": true,
        "api_key": if llm_ready { "set" } else { "not set" },
        "api_key_set": llm_ready,
        "llm_ready": llm_ready,
        "provider": provider,
        "model": model,
        "cli_mode": "ops",
        "workspace_dir": root.to_string_lossy().to_string(),
        "log_level": clean_text(
            &std::env::var("RUST_LOG")
                .or_else(|_| std::env::var("LOG_LEVEL"))
                .unwrap_or_else(|_| "info".to_string()),
            32,
        )
    })
}

fn config_schema_payload() -> Value {
    json!({
        "ok": true,
        "sections": {
            "runtime": {"root_level": true},
            "llm": {"root_level": false}
        }
    })
}

fn auth_check_payload() -> Value {
    json!({
        "ok": true,
        "mode": "none",
        "authenticated": true,
        "user": "operator"
    })
}

fn status_payload(root: &Path, snapshot: &Value, host_header: &str) -> Value {
    let usage = usage_from_state(root, snapshot);
    let runtime = runtime_sync_summary(snapshot);
    let (default_provider, default_model) = effective_app_settings(root, snapshot);
    let version = read_json(&root.join("package.json"))
        .and_then(|v| v.get("version").and_then(Value::as_str).map(str::to_string))
        .unwrap_or_else(|| "0.1.0".to_string());
    let listen = {
        let cleaned = clean_text(host_header, 200);
        if cleaned.is_empty() {
            "127.0.0.1:4173".to_string()
        } else {
            cleaned
        }
    };
    let uptime_seconds = parse_non_negative_i64(
        snapshot
            .pointer("/runtime_sync/uptime_seconds")
            .or_else(|| snapshot.pointer("/runtime_sync/uptime_sec")),
        0,
    );
    let agent_count = usage
        .get("agents")
        .and_then(Value::as_array)
        .map(|rows| rows.len())
        .unwrap_or(0);
    json!({
        "ok": true,
        "version": version,
        "agent_count": agent_count,
        "connected": true,
        "uptime_sec": uptime_seconds,
        "uptime_seconds": uptime_seconds,
        "ws": true,
        "default_provider": default_provider,
        "default_model": default_model,
        "git_branch": crate::dashboard_git_runtime::git_current_branch(root, "main"),
        "api_listen": listen,
        "listen": listen,
        "home_dir": root.to_string_lossy().to_string(),
        "workspace_dir": root.to_string_lossy().to_string(),
        "log_level": clean_text(
            &std::env::var("RUST_LOG")
                .or_else(|_| std::env::var("LOG_LEVEL"))
                .unwrap_or_else(|_| "info".to_string()),
            32,
        ),
        "network_enabled": true,
        "runtime_sync": runtime
    })
}
