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
fn dashboard_runtime_version_candidate(root: &Path) -> Option<Value> {
    let path = root
        .join("client")
        .join("runtime")
        .join("config")
        .join("runtime_version.json");
    let payload = read_json(&path)?;
    let source = clean_text(
        payload
            .get("source")
            .and_then(Value::as_str)
            .unwrap_or("runtime_version_contract"),
        80,
    );
    dashboard_version_candidate(
        payload.get("version").and_then(Value::as_str).unwrap_or(""),
        payload.get("tag").and_then(Value::as_str).unwrap_or(""),
        if source.is_empty() {
            "runtime_version_contract"
        } else {
            &source
        },
    )
}

fn dashboard_package_version_candidate(root: &Path) -> Option<Value> {
    let payload = read_json(&root.join("package.json"))?;
    dashboard_version_candidate(
        payload.get("version").and_then(Value::as_str).unwrap_or(""),
        "",
        "package_json",
    )
}

fn dashboard_runtime_version_info(root: &Path) -> Value {
    let mut best = None;
    best = pick_dashboard_version_candidate(best, dashboard_runtime_version_candidate(root));
    best = pick_dashboard_version_candidate(best, dashboard_package_version_candidate(root));
    best = pick_dashboard_version_candidate(best, dashboard_installed_release_candidate(root));
    best = pick_dashboard_version_candidate(best, dashboard_git_latest_tag_candidate(root));
    best.unwrap_or_else(|| {
        json!({
            "version": "0.0.0",
            "tag": "v0.0.0",
            "source": "fallback_default"
        })
    })
}

fn status_payload_cache() -> &'static Mutex<Option<StatusPayloadCacheEntry>> {
    static STATUS_PAYLOAD_CACHE: OnceLock<Mutex<Option<StatusPayloadCacheEntry>>> = OnceLock::new();
    STATUS_PAYLOAD_CACHE.get_or_init(|| Mutex::new(None))
}

fn status_payload(root: &Path, snapshot: &Value, host_header: &str) -> Value {
    let cache_key = format!(
        "{}|{}|{}",
        clean_text(host_header, 200),
        clean_text(
            snapshot
                .get("receipt_hash")
                .and_then(Value::as_str)
                .unwrap_or(""),
            128
        ),
        parse_non_negative_i64(
            snapshot
                .pointer("/runtime_sync/uptime_seconds")
                .or_else(|| snapshot.pointer("/runtime_sync/uptime_sec")),
            0
        )
    );
    let now_ms = monotonic_now_ms();
    if let Ok(guard) = status_payload_cache().lock() {
        if let Some(entry) = guard.as_ref() {
            if entry.key == cache_key && now_ms.saturating_sub(entry.built_at_ms) <= 900 {
                return entry.payload.clone();
            }
        }
    }
    let usage = usage_from_state(root, snapshot);
    let runtime = runtime_sync_summary(snapshot);
    let continuity = continuity_pending_payload(root, snapshot);
    let memory_hygiene = memory_hygiene_payload(root, &continuity);
    let task_runtime = task_runtime_summary(root);
    let worker_runtime = worker_runtime_summary(root);
    let hot_path_allocators = protheus_ops_core_v1::hot_path_allocators::snapshot_json();
    let web_conduit = crate::web_conduit::api_status(root);
    let (default_provider, default_model) = effective_app_settings(root, snapshot);
    let version_info = dashboard_runtime_version_info(root);
    let version = version_info
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or("0.0.0")
        .to_string();
    let version_tag = version_info
        .get("tag")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let version_source = version_info
        .get("source")
        .and_then(Value::as_str)
        .unwrap_or("fallback_default")
        .to_string();
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
    let out = json!({
        "ok": true,
        "version": version,
        "version_tag": version_tag,
        "version_source": version_source,
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
        "runtime_sync": runtime,
        "task_runtime": task_runtime,
        "worker_runtime": worker_runtime,
        "hot_path_allocators": hot_path_allocators,
        "web_conduit": {
            "enabled": web_conduit.get("enabled").cloned().unwrap_or_else(|| json!(false)),
            "receipts_total": web_conduit.get("receipts_total").cloned().unwrap_or_else(|| json!(0)),
            "recent_denied": web_conduit.get("recent_denied").cloned().unwrap_or_else(|| json!(0)),
            "last_receipt": web_conduit.get("last_receipt").cloned().unwrap_or(Value::Null)
        },
        "memory_hygiene": memory_hygiene,
        "continuity": {
            "pending_total": continuity.get("pending_total").cloned().unwrap_or_else(|| json!(0)),
            "tasks_pending": continuity.pointer("/tasks/pending").cloned().unwrap_or_else(|| json!(0)),
            "stale_sessions": continuity.pointer("/sessions/stale_48h_count").cloned().unwrap_or_else(|| json!(0)),
            "channel_attention": continuity.pointer("/channels/attention_needed_count").cloned().unwrap_or_else(|| json!(0))
        }
    });
    if let Ok(mut guard) = status_payload_cache().lock() {
        *guard = Some(StatusPayloadCacheEntry {
            key: cache_key,
            built_at_ms: now_ms,
            payload: out.clone(),
        });
    }
    out
}

fn task_runtime_summary(root: &Path) -> Value {
    let path = root.join("local/state/runtime/task_runtime/registry.json");
    let registry = read_json(&path).unwrap_or_else(|| json!({}));
    let tasks = registry
        .get("tasks")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut queued = 0i64;
    let mut running = 0i64;
    let mut done = 0i64;
    let mut cancelled = 0i64;
    for row in tasks {
        let status = clean_text(row.get("status").and_then(Value::as_str).unwrap_or(""), 40)
            .to_ascii_lowercase();
        match status.as_str() {
            "queued" => queued += 1,
            "running" => running += 1,
            "done" => done += 1,
            "cancelled" => cancelled += 1,
            _ => {}
        }
    }
    json!({
        "queued": queued,
        "running": running,
        "done": done,
        "cancelled": cancelled,
        "pending": queued + running
    })
}

fn worker_runtime_summary(root: &Path) -> Value {
    let path = root.join("local/state/runtime/task_runtime/worker_state.json");
    let state = read_json(&path).unwrap_or_else(|| json!({}));
    let active_workers = state
        .get("active_workers")
        .and_then(Value::as_object)
        .map(|rows| rows.len())
        .unwrap_or(0) as i64;
    json!({
        "active_workers": active_workers,
        "total_hibernations": state.get("total_hibernations").and_then(Value::as_i64).unwrap_or(0).max(0),
        "last_hibernated": state.get("last_hibernated").cloned().unwrap_or(Value::Null),
        "last_event": state.get("last_event").cloned().unwrap_or(Value::Null),
        "updated_at_ms": state.get("updated_at_ms").cloned().unwrap_or(Value::Null)
    })
}

fn session_pending_rows(root: &Path, snapshot: &Value, max_rows: usize) -> Vec<Value> {
    let now = Utc::now();
    let mut rows = Vec::<Value>::new();
    for row in session_summary_rows(root, snapshot).into_iter() {
        let message_count = row
            .get("message_count")
            .and_then(Value::as_i64)
            .unwrap_or(0)
            .max(0);
        if message_count <= 0 {
            continue;
        }
        let agent_id = clean_text(
            row.get("agent_id").and_then(Value::as_str).unwrap_or(""),
            140,
        );
        if agent_id.is_empty() {
            continue;
        }
        let updated_at = clean_text(
            row.get("updated_at").and_then(Value::as_str).unwrap_or(""),
            80,
        );
        let age_hours = parse_rfc3339_utc(&updated_at)
            .map(|ts| {
                let delta = now.signed_duration_since(ts).num_minutes().max(0);
                delta as f64 / 60.0
            })
            .unwrap_or(0.0);
        rows.push(json!({
            "agent_id": agent_id,
            "active_session_id": clean_text(row.get("active_session_id").and_then(Value::as_str).unwrap_or(""), 120),
            "message_count": message_count,
            "updated_at": updated_at,
            "age_hours": (age_hours * 10.0).round() / 10.0,
            "stale_48h": age_hours >= 48.0
        }));
    }
    rows.sort_by(|a, b| {
        let left = a.get("age_hours").and_then(Value::as_f64).unwrap_or(0.0);
        let right = b.get("age_hours").and_then(Value::as_f64).unwrap_or(0.0);
        right
            .partial_cmp(&left)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    rows.truncate(max_rows.clamp(1, 100));
    rows
}

fn agent_continuity_markers(root: &Path, snapshot: &Value, max_rows: usize) -> Vec<Value> {
    let roster = build_agent_roster(root, snapshot, false);
    let mut rows = Vec::<Value>::new();
    for profile in roster {
        let agent_id = clean_agent_id(
            profile
                .get("agent_id")
                .or_else(|| profile.get("id"))
                .and_then(Value::as_str)
                .unwrap_or(""),
        );
        if agent_id.is_empty() {
            continue;
        }
        let state = load_session_state(root, &agent_id);
        let messages = session_messages(&state);
        let mut latest_user_text = String::new();
        let mut latest_user_ts = String::new();
        let mut latest_agent_ts = String::new();
        for row in messages.iter().rev() {
            let role = clean_text(row.get("role").and_then(Value::as_str).unwrap_or(""), 24)
                .to_ascii_lowercase();
            if role == "user" && latest_user_text.is_empty() {
                latest_user_text = clean_text(&message_text(row), 180);
                latest_user_ts = message_timestamp_iso(row);
            }
            if (role == "assistant" || role == "agent") && latest_agent_ts.is_empty() {
                latest_agent_ts = message_timestamp_iso(row);
            }
            if !latest_user_text.is_empty() && !latest_agent_ts.is_empty() {
                break;
            }
        }
        let objective = if latest_user_text.is_empty() {
            "No active objective.".to_string()
        } else {
            latest_user_text.clone()
        };
        let completion_percent = if latest_user_text.is_empty() {
            100
        } else if !latest_agent_ts.is_empty()
            && !latest_user_ts.is_empty()
            && latest_agent_ts >= latest_user_ts
        {
            100
        } else if !latest_agent_ts.is_empty() {
            60
        } else {
            20
        };
        rows.push(json!({
            "agent_id": agent_id,
            "name": clean_text(profile.get("name").and_then(Value::as_str).unwrap_or("Agent"), 120),
            "state": clean_text(profile.get("state").and_then(Value::as_str).unwrap_or("Idle"), 40),
            "objective": objective,
            "completion_percent": completion_percent,
            "updated_at": clean_text(profile.get("updated_at").and_then(Value::as_str).unwrap_or(""), 80)
        }));
    }
    rows.sort_by(|a, b| {
        clean_text(
            b.get("updated_at").and_then(Value::as_str).unwrap_or(""),
            80,
        )
        .cmp(&clean_text(
            a.get("updated_at").and_then(Value::as_str).unwrap_or(""),
            80,
        ))
    });
    rows.truncate(max_rows.clamp(1, 24));
    rows
}

fn continuity_pending_payload(root: &Path, snapshot: &Value) -> Value {
    let tasks = task_runtime_summary(root);
    let workers = worker_runtime_summary(root);
    let sessions = session_pending_rows(root, snapshot, 24);
    let continuity_agents = agent_continuity_markers(root, snapshot, 12);
    let stale_sessions = sessions
        .iter()
        .filter(|row| {
            row.get("stale_48h")
                .and_then(Value::as_bool)
                .unwrap_or(false)
        })
        .cloned()
        .collect::<Vec<_>>();
    let channel_rows = dashboard_compat_api_channels::channels_payload(root)
        .get("channels")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let channel_attention = channel_rows
        .into_iter()
        .filter(|row| {
            let configured = row.get("configured").and_then(Value::as_bool).unwrap_or(false);
            let connected = row.get("connected").and_then(Value::as_bool).unwrap_or(false);
            configured && !connected
        })
        .map(|row| {
            json!({
                "name": clean_text(row.get("name").and_then(Value::as_str).unwrap_or(""), 80),
                "provider": clean_text(row.get("provider").and_then(Value::as_str).unwrap_or(""), 80),
                "status": clean_text(row.get("status").and_then(Value::as_str).unwrap_or(""), 40)
            })
        })
        .collect::<Vec<_>>();

    let pending_total = tasks
        .get("pending")
        .and_then(Value::as_i64)
        .unwrap_or(0)
        .max(0)
        + stale_sessions.len() as i64
        + channel_attention.len() as i64;
    json!({
        "ok": true,
        "type": "cross_channel_project_continuity",
        "pending_total": pending_total,
        "tasks": tasks,
        "workers": workers,
        "sessions": {
            "rows": sessions,
            "stale_48h_count": stale_sessions.len(),
            "stale_48h": stale_sessions
        },
        "active_agents": {
            "count": continuity_agents.len(),
            "rows": continuity_agents
        },
        "channels": {
            "attention_needed_count": channel_attention.len(),
            "attention_needed": channel_attention
        }
    })
}

const SNAPSHOT_HISTORY_SOFT_CAP_BYTES: i64 = 100 * 1024 * 1024;

fn memory_hygiene_payload(root: &Path, continuity: &Value) -> Value {
    let stale_48h_count = continuity
        .pointer("/sessions/stale_48h_count")
        .and_then(Value::as_i64)
        .unwrap_or(0)
        .max(0);
    let stale_7d_count = continuity
        .pointer("/sessions/stale_48h")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter(|row| row.get("age_hours").and_then(Value::as_f64).unwrap_or(0.0) >= 168.0)
                .count() as i64
        })
        .unwrap_or(0);
    let snapshot_path = state_path(
        root,
        "client/runtime/local/state/ui/infring_dashboard/snapshot_history.jsonl",
    );
    let snapshot_bytes_u64 = fs::metadata(&snapshot_path)
        .map(|meta| meta.len())
        .unwrap_or(0);
    let snapshot_bytes = if snapshot_bytes_u64 > i64::MAX as u64 {
        i64::MAX
    } else {
        snapshot_bytes_u64 as i64
    };
    let snapshot_over_soft_cap = snapshot_bytes >= SNAPSHOT_HISTORY_SOFT_CAP_BYTES;

    let mut recommendations = Vec::<Value>::new();
    if stale_7d_count > 0 {
        recommendations.push(json!({
            "command": "/continuity",
            "reason": format!("{stale_7d_count} stale memory-backed session(s) exceed 7 days")
        }));
    }
    if snapshot_over_soft_cap {
        recommendations.push(json!({
            "command": "infring cleanup purge --aggressive",
            "reason": format!("snapshot_history.jsonl exceeds soft cap ({} bytes)", snapshot_bytes)
        }));
    }
    if recommendations.is_empty() {
        recommendations.push(json!({
            "command": "/status",
            "reason": "memory hygiene is healthy"
        }));
    }

    json!({
        "ok": true,
        "type": "memory_hygiene",
        "stale_contexts_48h": stale_48h_count,
        "stale_contexts_7d": stale_7d_count,
        "snapshot_history_path": snapshot_path.to_string_lossy().to_string(),
        "snapshot_history_bytes": snapshot_bytes,
        "snapshot_history_soft_cap_bytes": SNAPSHOT_HISTORY_SOFT_CAP_BYTES,
        "snapshot_history_over_soft_cap": snapshot_over_soft_cap,
        "recommendations": recommendations
    })
}

fn predicted_next_actions(
    task_pending: i64,
    queue_depth: i64,
    stale_sessions: i64,
    channel_attention: i64,
    dashboard_alerts: i64,
    memory_hygiene: &Value,
) -> Vec<Value> {
    let mut out = Vec::<Value>::new();
    let mut seen = HashSet::<String>::new();
    let mut push = |command: &str, reason: String, priority: &str| {
        let key = clean_text(command, 60).to_ascii_lowercase();
        if key.is_empty() || seen.contains(&key) {
            return;
        }
        seen.insert(key);
        out.push(json!({
            "command": command,
            "reason": reason,
            "priority": priority
        }));
    };

    if dashboard_alerts > 0 {
        push(
            "/alerts",
            format!("Health lane has {} active alert(s)", dashboard_alerts),
            "high",
        );
    }
    if task_pending > 0 || queue_depth > 0 {
        push(
            "/queue",
            format!(
                "Queue pressure pending={} depth={}",
                task_pending, queue_depth
            ),
            "high",
        );
    }
    if stale_sessions > 0 || channel_attention > 0 {
        push(
            "/continuity",
            format!(
                "Pending continuity work (stale_sessions={}, channel_attention={})",
                stale_sessions, channel_attention
            ),
            "medium",
        );
    }
    if memory_hygiene
        .get("snapshot_history_over_soft_cap")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        push(
            "infring cleanup purge --aggressive",
            "Memory hygiene indicates snapshot history bloat".to_string(),
            "medium",
        );
    }
    if task_pending == 0
        && queue_depth == 0
        && stale_sessions == 0
        && channel_attention == 0
        && dashboard_alerts == 0
    {
        push(
            "/status",
            "System is healthy; run status for a quick confidence check".to_string(),
            "low",
        );
    }
    out
}

fn proactive_telemetry_alerts_payload(root: &Path, snapshot: &Value) -> Value {
    let continuity = continuity_pending_payload(root, snapshot);
    let runtime = runtime_sync_summary(snapshot);
    let task_pending = continuity
        .pointer("/tasks/pending")
        .and_then(Value::as_i64)
        .unwrap_or(0)
        .max(0);
    let active_workers = continuity
        .pointer("/workers/active_workers")
        .and_then(Value::as_i64)
        .unwrap_or(0)
        .max(0);
    let stale_sessions = continuity
        .pointer("/sessions/stale_48h_count")
        .and_then(Value::as_i64)
        .unwrap_or(0)
        .max(0);
    let channel_attention = continuity
        .pointer("/channels/attention_needed_count")
        .and_then(Value::as_i64)
        .unwrap_or(0)
        .max(0);
    let dashboard_alerts = parse_non_negative_i64(snapshot.pointer("/health/alerts/count"), 0);
    let queue_depth = runtime
        .get("queue_depth")
        .and_then(Value::as_i64)
        .unwrap_or(0)
        .max(0);
    let memory_hygiene = memory_hygiene_payload(root, &continuity);
    let stale_memory_7d = memory_hygiene
        .get("stale_contexts_7d")
        .and_then(Value::as_i64)
        .unwrap_or(0)
        .max(0);
    let snapshot_over_soft_cap = memory_hygiene
        .get("snapshot_history_over_soft_cap")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    let mut alerts = Vec::<Value>::new();
    if dashboard_alerts > 0 {
        alerts.push(json!({
            "id": "health_alerts_present",
            "severity": "high",
            "message": format!("Health checks report {} alert(s).", dashboard_alerts),
            "recommended_command": "/status",
            "source": "health"
        }));
    }
    if task_pending >= 22 || queue_depth >= 22 {
        alerts.push(json!({
            "id": "queue_pressure_high",
            "severity": "high",
            "message": format!("Queue pressure is elevated (pending={}, depth={}).", task_pending, queue_depth),
            "recommended_command": "/queue",
            "source": "task_runtime"
        }));
    }
    if stale_sessions > 0 {
        alerts.push(json!({
            "id": "stale_sessions_detected",
            "severity": "medium",
            "message": format!("{} session(s) have pending context older than 48h.", stale_sessions),
            "recommended_command": "/continuity",
            "source": "sessions"
        }));
    }
    if channel_attention > 0 {
        alerts.push(json!({
            "id": "channel_attention_needed",
            "severity": "medium",
            "message": format!("{} configured channel(s) are disconnected.", channel_attention),
            "recommended_command": "/continuity",
            "source": "channels"
        }));
    }
    if active_workers > 0 && task_pending == 0 {
        alerts.push(json!({
            "id": "worker_hibernation_candidate",
            "severity": "low",
            "message": "Workers are active with zero pending tasks; hibernation path can reclaim compute.",
            "recommended_command": "infring task worker --service=1 --idle-hibernate-ms=15000",
            "source": "task_runtime"
        }));
    }
    if stale_memory_7d > 0 {
        alerts.push(json!({
            "id": "memory_hygiene_stale_contexts",
            "severity": "medium",
            "message": format!("{} memory-backed session context(s) are older than 7 days and should be compacted.", stale_memory_7d),
            "recommended_command": "/memory",
            "source": "memory_hygiene"
        }));
    }
    if snapshot_over_soft_cap {
        let bytes = memory_hygiene
            .get("snapshot_history_bytes")
            .and_then(Value::as_i64)
            .unwrap_or(0)
            .max(0);
        alerts.push(json!({
            "id": "snapshot_history_bloat",
            "severity": "high",
            "message": format!("snapshot_history.jsonl is large ({} bytes); cleanup should run aggressively.", bytes),
            "recommended_command": "infring cleanup purge --aggressive",
            "source": "memory_hygiene"
        }));
    }

    let next_actions = predicted_next_actions(
        task_pending,
        queue_depth,
        stale_sessions,
        channel_attention,
        dashboard_alerts,
        &memory_hygiene,
    );

    json!({
        "ok": true,
        "type": "proactive_telemetry_alerts",
        "generated_at": crate::now_iso(),
        "count": alerts.len(),
        "alerts": alerts,
        "continuity": continuity,
        "memory_hygiene": memory_hygiene,
        "next_actions": next_actions
    })
}

#[cfg(test)]
mod continuity_tests {
    use super::*;
    use std::process::Command;
    use tempfile::tempdir;

    fn write_json(path: &Path, value: &Value) {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let raw = serde_json::to_string_pretty(value).expect("json");
        fs::write(path, raw).expect("write");
    }

    fn run_git(root: &Path, args: &[&str]) {
        let output = Command::new("git")
            .args(args)
            .current_dir(root)
            .output()
            .expect("git spawn");
        assert!(
            output.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn task_runtime_summary_counts_pending_and_done() {
        let temp = tempdir().expect("tempdir");
        write_json(
            &temp
                .path()
                .join("local/state/runtime/task_runtime/registry.json"),
            &json!({
                "version": "v1",
                "tasks": [
                    {"id":"a","status":"queued"},
                    {"id":"b","status":"running"},
                    {"id":"c","status":"done"},
                    {"id":"d","status":"cancelled"}
                ]
            }),
        );
        let out = task_runtime_summary(temp.path());
        assert_eq!(out.get("pending").and_then(Value::as_i64), Some(2));
        assert_eq!(out.get("done").and_then(Value::as_i64), Some(1));
        assert_eq!(out.get("cancelled").and_then(Value::as_i64), Some(1));
    }

    #[test]
    fn continuity_payload_surfaces_stale_sessions_and_channel_attention() {
        let temp = tempdir().expect("tempdir");
        let stale_iso = (Utc::now() - chrono::Duration::hours(72)).to_rfc3339();
        write_json(
            &temp.path().join(
                "client/runtime/local/state/ui/infring_dashboard/agent_sessions/agent-alpha.json",
            ),
            &json!({
                "agent_id": "agent-alpha",
                "active_session_id": "default",
                "sessions": [
                    {
                        "session_id": "default",
                        "updated_at": stale_iso,
                        "messages": [
                            {"role": "user", "text": "investigate pending deployment"}
                        ]
                    }
                ]
            }),
        );
        write_json(
            &temp
                .path()
                .join("client/runtime/local/state/ui/infring_dashboard/channel_registry.json"),
            &json!({
                "type": "infring_dashboard_channel_registry",
                "channels": {
                    "slack": {
                        "name": "slack",
                        "provider": "slack",
                        "configured": true,
                        "has_token": false,
                        "status": "disconnected"
                    }
                }
            }),
        );

        let out = continuity_pending_payload(temp.path(), &json!({}));
        assert_eq!(
            out.pointer("/sessions/stale_48h_count")
                .and_then(Value::as_i64),
            Some(1)
        );
        assert_eq!(
            out.pointer("/channels/attention_needed_count")
                .and_then(Value::as_i64),
            Some(1)
        );
        assert!(out
            .pointer("/active_agents/rows")
            .and_then(Value::as_array)
            .map(|rows| !rows.is_empty())
            .unwrap_or(false));
    }

    #[test]
    fn proactive_alerts_raise_queue_pressure_signal() {
        let temp = tempdir().expect("tempdir");
        write_json(
            &temp
                .path()
                .join("local/state/runtime/task_runtime/registry.json"),
            &json!({
                "version": "v1",
                "tasks": (0..24).map(|idx| json!({"id": format!("t-{idx}"), "status": "queued"})).collect::<Vec<_>>()
            }),
        );
        let out = proactive_telemetry_alerts_payload(
            temp.path(),
            &json!({
                "ok": true,
                "health": {
                    "dashboard_metrics": {
                        "queue_depth": { "value": 24 }
                    },
                    "alerts": { "count": 0 }
                }
            }),
        );
        let alerts = out
            .get("alerts")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let ids = alerts
            .iter()
            .filter_map(|row| row.get("id").and_then(Value::as_str))
            .collect::<Vec<_>>();
        assert!(ids.contains(&"queue_pressure_high"));
        let next_actions = out
            .get("next_actions")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let has_queue_next = next_actions.iter().any(|row| {
            row.get("command")
                .and_then(Value::as_str)
                .map(|cmd| cmd == "/queue")
                .unwrap_or(false)
        });
        assert!(has_queue_next);
    }

    #[test]
    fn memory_hygiene_flags_snapshot_history_bloat() {
        let temp = tempdir().expect("tempdir");
        let snapshot_path = temp
            .path()
            .join("client/runtime/local/state/ui/infring_dashboard/snapshot_history.jsonl");
        if let Some(parent) = snapshot_path.parent() {
            fs::create_dir_all(parent).expect("mkdirs");
        }
        fs::write(&snapshot_path, vec![b'x'; 101 * 1024 * 1024]).expect("write large snapshot");

        let out = proactive_telemetry_alerts_payload(
            temp.path(),
            &json!({
                "ok": true,
                "health": {
                    "dashboard_metrics": {
                        "queue_depth": { "value": 0 }
                    },
                    "alerts": { "count": 0 }
                }
            }),
        );
        assert_eq!(
            out.pointer("/memory_hygiene/snapshot_history_over_soft_cap")
                .and_then(Value::as_bool),
            Some(true)
        );
        let alert_rows = out
            .get("alerts")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let ids = alert_rows
            .iter()
            .filter_map(|row| row.get("id").and_then(Value::as_str))
            .collect::<Vec<_>>();
        assert!(ids.contains(&"snapshot_history_bloat"));
    }

    #[test]
    fn dashboard_runtime_version_info_prefers_latest_git_tag_over_stale_contract_files() {
        let temp = tempdir().expect("tempdir");
        write_json(
            &temp.path().join("package.json"),
            &json!({
                "version": "0.2.1-alpha.1"
            }),
        );
        write_json(
            &temp
                .path()
                .join("client/runtime/config/runtime_version.json"),
            &json!({
                "version": "0.2.1-alpha.1",
                "tag": "v0.2.1-alpha.1",
                "source": "runtime_version_contract"
            }),
        );
        fs::write(temp.path().join("README.md"), "demo\n").expect("write readme");
        run_git(temp.path(), &["init"]);
        run_git(temp.path(), &["config", "user.email", "tests@example.com"]);
        run_git(temp.path(), &["config", "user.name", "Dashboard Tests"]);
        run_git(temp.path(), &["add", "."]);
        run_git(temp.path(), &["commit", "-m", "test repo"]);
        run_git(temp.path(), &["tag", "v0.3.10-alpha"]);

        let payload = dashboard_runtime_version_info(temp.path());
        assert_eq!(
            payload.get("version").and_then(Value::as_str),
            Some("0.3.10-alpha")
        );
        assert_eq!(
            payload.get("tag").and_then(Value::as_str),
            Some("v0.3.10-alpha")
        );
        assert_eq!(
            payload.get("source").and_then(Value::as_str),
            Some("git_latest_tag")
        );
    }
}
