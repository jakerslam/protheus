fn text_field(value: &Value, key: &str, max_len: usize) -> String {
    clean_text(
        value.get(key).and_then(Value::as_str).unwrap_or(""),
        max_len.max(1),
    )
}

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
        let model_provider = {
            let value = text_field(&row, "model_provider", 80);
            if value.is_empty() {
                "auto".to_string()
            } else {
                value
            }
        };
        let model_name = text_field(&row, "model_name", 120);
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

    agent_rows.sort_by_key(|row| std::cmp::Reverse(text_field(row, "updated_at", 80)));

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
    model_rows.sort_by(|a, b| text_field(b, "model", 160).cmp(&text_field(a, "model", 160)));

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
    daily_rows.sort_by(|a, b| text_field(a, "date", 20).cmp(&text_field(b, "date", 20)));
    let today_cost_usd = daily_rows
        .iter()
        .find(|row| text_field(row, "date", 20) == today)
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
