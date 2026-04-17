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

#[derive(Clone)]
struct StatusPayloadCacheEntry {
    key: String,
    built_at_ms: u128,
    payload: Value,
}

fn monotonic_now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

fn normalize_dashboard_version_text(value: &str) -> String {
    clean_text(value.trim_start_matches(['v', 'V']), 120)
}

fn compare_dashboard_version_text(left: &str, right: &str) -> std::cmp::Ordering {
    let left_normalized = normalize_dashboard_version_text(left);
    let right_normalized = normalize_dashboard_version_text(right);
    match (
        semver::Version::parse(&left_normalized),
        semver::Version::parse(&right_normalized),
    ) {
        (Ok(a), Ok(b)) => a.cmp(&b),
        (Ok(_), Err(_)) => std::cmp::Ordering::Greater,
        (Err(_), Ok(_)) => std::cmp::Ordering::Less,
        _ => left_normalized.cmp(&right_normalized),
    }
}

fn dashboard_version_source_priority(source: &str) -> i32 {
    match clean_text(source, 80).as_str() {
        "git_latest_tag" => 40,
        "install_release_meta" => 30,
        "install_release_tag" => 28,
        "runtime_version_contract" => 20,
        "package_json" => 10,
        _ => 0,
    }
}

fn dashboard_version_candidate(version: &str, tag: &str, source: &str) -> Option<Value> {
    let normalized_version = normalize_dashboard_version_text(version);
    if normalized_version.is_empty() {
        return None;
    }
    let normalized_tag = {
        let cleaned = clean_text(tag, 120);
        if cleaned.is_empty() {
            format!("v{normalized_version}")
        } else {
            cleaned
        }
    };
    Some(json!({
        "version": normalized_version,
        "tag": normalized_tag,
        "source": clean_text(source, 80)
    }))
}

fn pick_dashboard_version_candidate(best: Option<Value>, candidate: Option<Value>) -> Option<Value> {
    let Some(candidate_value) = candidate else {
        return best;
    };
    let candidate_version = candidate_value
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or("");
    let candidate_source = candidate_value
        .get("source")
        .and_then(Value::as_str)
        .unwrap_or("");
    match best {
        None => Some(candidate_value),
        Some(best_value) => {
            let best_version = best_value.get("version").and_then(Value::as_str).unwrap_or("");
            let cmp = compare_dashboard_version_text(candidate_version, best_version);
            if cmp == std::cmp::Ordering::Greater {
                Some(candidate_value)
            } else if cmp == std::cmp::Ordering::Less {
                Some(best_value)
            } else {
                let best_source = best_value
                    .get("source")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                if dashboard_version_source_priority(candidate_source)
                    >= dashboard_version_source_priority(best_source)
                {
                    Some(candidate_value)
                } else {
                    Some(best_value)
                }
            }
        }
    }
}

fn dashboard_git_latest_tag_candidate(root: &Path) -> Option<Value> {
    let output = std::process::Command::new("git")
        .args(["tag", "--list", "--sort=-v:refname", "v*"])
        .current_dir(root)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let tag = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|row| clean_text(row, 120))
        .find(|row| !row.is_empty())?;
    dashboard_version_candidate(&tag, &tag, "git_latest_tag")
}

fn dashboard_installed_release_candidate(root: &Path) -> Option<Value> {
    let meta_path = root
        .join("local")
        .join("state")
        .join("ops")
        .join("install_release_meta.json");
    if let Some(meta) = read_json(&meta_path) {
        let value = clean_text(
            meta.get("release_version_normalized")
                .and_then(Value::as_str)
                .or_else(|| meta.get("release_tag").and_then(Value::as_str))
                .unwrap_or(""),
            120,
        );
        let tag = clean_text(
            meta.get("release_tag")
                .and_then(Value::as_str)
                .unwrap_or(""),
            120,
        );
        let candidate = dashboard_version_candidate(&value, &tag, "install_release_meta");
        if candidate.is_some() {
            return candidate;
        }
    }
    let tag_path = root
        .join("local")
        .join("state")
        .join("ops")
        .join("install_release_tag.txt");
    let raw = std::fs::read_to_string(tag_path).ok()?;
    let tag = clean_text(raw.lines().next().unwrap_or(""), 120);
    dashboard_version_candidate(&tag, &tag, "install_release_tag")
}
