fn parse_memory_capture_text(user_text: &str) -> Option<String> {
    let cleaned = clean_text(user_text, 2000);
    if cleaned.is_empty() {
        return None;
    }
    let lowered = cleaned.to_ascii_lowercase();
    if !(lowered.starts_with("remember ") || lowered.contains("remember this")) {
        return None;
    }
    let extracted = if let Some((_, tail)) = cleaned.split_once(':') {
        clean_text(tail, 1200)
    } else {
        clean_text(cleaned.trim_start_matches("remember"), 1200)
    };
    if extracted.is_empty() {
        None
    } else {
        Some(extracted)
    }
}

fn important_memory_terms(text: &str, limit: usize) -> Vec<String> {
    let stop_words = [
        "the", "and", "for", "with", "that", "this", "from", "have", "your", "you", "are", "was",
        "were", "will", "into", "about", "what", "when", "then", "than", "just", "they", "them",
        "able", "make", "made", "need", "want", "does", "did", "done", "cant", "cannot", "dont",
        "not", "too", "very", "also", "like", "been", "being", "each", "more", "most", "over",
        "under", "after", "before", "because", "while", "where", "which", "who", "whom", "whose",
        "would", "could", "should",
    ];
    let mut seen = HashSet::<String>::new();
    let mut out = Vec::<String>::new();
    for raw in clean_text(text, 2000).to_ascii_lowercase().split(' ') {
        let token = raw
            .chars()
            .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '_' || *ch == '-')
            .collect::<String>();
        if token.len() < 3 || stop_words.contains(&token.as_str()) {
            continue;
        }
        if seen.insert(token.clone()) {
            out.push(token);
            if out.len() >= limit {
                break;
            }
        }
    }
    out
}

fn passive_memory_attention_event(
    agent_id: &str,
    user_text: &str,
    assistant_text: &str,
) -> Option<Value> {
    let user = clean_text(user_text, 1400);
    let assistant = clean_text(assistant_text, 1400);
    if user.is_empty() && assistant.is_empty() {
        return None;
    }
    let summary = if !user.is_empty() {
        format!(
            "{}: {}",
            humanize_agent_name(agent_id),
            clean_text(&user, 220)
        )
    } else {
        format!(
            "{}: {}",
            humanize_agent_name(agent_id),
            clean_text(&assistant, 220)
        )
    };
    let terms = important_memory_terms(&format!("{user} {assistant}"), 12);
    let event = json!({
        "ts": crate::now_iso(),
        "source": format!("agent:{agent_id}"),
        "source_type": "passive_memory_turn",
        "severity": "info",
        "summary": summary,
        "attention_key": format!(
            "agent:{agent_id}:passive_memory:{}",
            crate::deterministic_receipt_hash(&json!({
                "agent_id": agent_id,
                "user": user,
                "assistant": assistant
            }))
            .chars()
            .take(20)
            .collect::<String>()
        ),
        "raw_event": {
            "agent_id": agent_id,
            "memory_kind": "passive_turn",
            "user_text": user,
            "assistant_text": assistant,
            "terms": terms
        }
    });
    Some(event)
}

fn tool_outcome_keyframe_from_turn(user_text: &str, assistant_text: &str) -> Option<Value> {
    let assistant = clean_text(assistant_text, 1_600);
    if assistant.is_empty() {
        return None;
    }
    let lowered = assistant.to_ascii_lowercase();
    let mentions_web = lowered.contains("batch_query")
        || lowered.contains("batch query")
        || lowered.contains("web search")
        || lowered.contains("web fetch")
        || lowered.contains("live web")
        || lowered.contains("source url");
    let low_signal = lowered.contains("low-signal")
        || lowered.contains("no usable findings")
        || lowered.contains("no extractable findings")
        || lowered.contains("couldn't extract usable findings")
        || lowered.contains("could not extract usable findings")
        || lowered.contains("usable tool findings from this turn yet")
        || lowered.contains("source-backed findings in this turn")
        || lowered.contains("search returned no useful information")
        || lowered.contains("fit safely in context")
        || lowered.contains("partial result");
    if !(mentions_web && low_signal) {
        return None;
    }
    let query = natural_web_search_query_from_message(user_text)
        .or_else(|| comparative_web_query_from_message(user_text))
        .unwrap_or_default();
    let url = first_http_url_in_text(user_text);
    let tool = if lowered.contains("web fetch") || !url.is_empty() {
        "web_fetch"
    } else if lowered.contains("batch_query")
        || lowered.contains("batch query")
        || message_requests_live_web_comparison(user_text)
    {
        "batch_query"
    } else {
        "web_search"
    };
    let subject = if !query.is_empty() {
        format!(" for `{}`", trim_text(&query, 120))
    } else if !url.is_empty() {
        format!(" for {}", trim_text(&url, 120))
    } else {
        String::new()
    };
    let summary = if lowered.contains("fit safely in context") || lowered.contains("partial result")
    {
        format!(
            "Recent {tool} outcome{subject}: web output exceeded the safe context budget; rerun with a narrower query or continue from the partial result."
        )
    } else {
        format!(
            "Recent {tool} outcome{subject}: retrieval returned low-signal web output instead of usable findings; rerun with a narrower query or one source URL."
        )
    };
    let key_seed = json!({
        "kind": "tool_outcome",
        "tool": tool,
        "summary": summary
    });
    let key_hash = crate::deterministic_receipt_hash(&key_seed);
    Some(json!({
        "keyframe_id": format!("kf-{}", &key_hash[..12]),
        "kind": "tool_outcome",
        "tool": tool,
        "summary": clean_text(&summary, 260),
        "captured_at": crate::now_iso()
    }))
}

fn append_context_keyframe_to_active_session(root: &Path, agent_id: &str, keyframe: &Value) {
    let id = clean_agent_id(agent_id);
    if id.is_empty() {
        return;
    }
    let mut state = load_session_state(root, &id);
    let active_id = clean_text(
        state
            .get("active_session_id")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        120,
    );
    if let Some(rows) = state.get_mut("sessions").and_then(Value::as_array_mut) {
        for row in rows.iter_mut() {
            let sid = clean_text(
                row.get("session_id").and_then(Value::as_str).unwrap_or(""),
                120,
            );
            if sid != active_id {
                continue;
            }
            if !row
                .get("context_keyframes")
                .map(Value::is_array)
                .unwrap_or(false)
            {
                row["context_keyframes"] = Value::Array(Vec::new());
            }
            if let Some(keyframes) = row
                .get_mut("context_keyframes")
                .and_then(Value::as_array_mut)
            {
                keyframes.push(keyframe.clone());
                if keyframes.len() > 48 {
                    let trim = keyframes.len().saturating_sub(48);
                    keyframes.drain(0..trim);
                }
            }
            row["updated_at"] = Value::String(crate::now_iso());
            break;
        }
    }
    save_session_state(root, &id, &state);
}

fn enqueue_attention_event_best_effort(root: &Path, run_context: &str, event: &Value) -> Value {
    let event_json = match serde_json::to_string(event) {
        Ok(raw) => raw,
        Err(err) => {
            return json!({
                "ok": false,
                "queued": false,
                "reason": "event_encode_failed",
                "error": clean_text(&err.to_string(), 200)
            });
        }
    };
    let encoded = {
        use base64::engine::general_purpose::STANDARD;
        use base64::Engine;
        STANDARD.encode(event_json.as_bytes())
    };
    let args = vec![
        "enqueue".to_string(),
        format!("--event-json-base64={encoded}"),
        format!("--run-context={}", clean_text(run_context, 120)),
    ];
    let exit = crate::attention_queue::run(root, &args);
    if exit == 0 {
        json!({"ok": true, "queued": true, "run_context": run_context, "exit_code": 0})
    } else {
        let staged = json!({
            "ts": crate::now_iso(),
            "run_context": clean_text(run_context, 120),
            "event": event
        });
        append_jsonl_row(&attention_queue_fallback_path(root), &staged);
        json!({
            "ok": false,
            "queued": false,
            "staged": true,
            "run_context": run_context,
            "exit_code": exit,
            "fallback_path": attention_queue_fallback_path(root).to_string_lossy().to_string()
        })
    }
}

fn append_turn_message(
    root: &Path,
    agent_id: &str,
    user_text: &str,
    assistant_text: &str,
) -> Value {
    let mut receipt =
        crate::dashboard_agent_state::append_turn(root, agent_id, user_text, assistant_text);
    if let Some(memory_text) = parse_memory_capture_text(user_text) {
        let key = format!(
            "explicit_memory.{}",
            crate::deterministic_receipt_hash(
                &json!({"agent_id": agent_id, "memory": memory_text})
            )
            .chars()
            .take(12)
            .collect::<String>()
        );
        let value = json!({
            "text": memory_text,
            "captured_at": crate::now_iso(),
            "source": "user_explicit_remember"
        });
        let memory_receipt =
            crate::dashboard_agent_state::memory_kv_set(root, agent_id, &key, &value);
        receipt["memory_capture"] = memory_receipt;
    }
    if let Some(event) = passive_memory_attention_event(agent_id, user_text, assistant_text) {
        receipt["attention_queue"] =
            enqueue_attention_event_best_effort(root, "dashboard_agent_passive_memory", &event);
    } else {
        receipt["attention_queue"] = json!({
            "ok": true,
            "queued": false,
            "reason": "empty_turn"
        });
    }
    if let Some(tool_keyframe) = tool_outcome_keyframe_from_turn(user_text, assistant_text) {
        append_context_keyframe_to_active_session(root, agent_id, &tool_keyframe);
        receipt["tool_outcome_keyframe"] = tool_keyframe;
    }
    receipt
}

fn rollback_last_turn(root: &Path, agent_id: &str) -> Value {
    let id = clean_agent_id(agent_id);
    if id.is_empty() {
        return json!({"ok": false, "error": "agent_id_required"});
    }
    let mut state = load_session_state(root, &id);
    let active_id = clean_text(
        state
            .get("active_session_id")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        120,
    );
    let mut removed = Vec::<Value>::new();
    let mut before_messages = 0usize;
    let mut after_messages = 0usize;
    let mut rollback_id = String::new();
    if let Some(rows) = state.get_mut("sessions").and_then(Value::as_array_mut) {
        for row in rows.iter_mut() {
            let sid = clean_text(
                row.get("session_id").and_then(Value::as_str).unwrap_or(""),
                120,
            );
            if sid != active_id {
                continue;
            }
            if !row.get("messages").map(Value::is_array).unwrap_or(false) {
                row["messages"] = Value::Array(Vec::new());
            }
            let messages = row
                .get_mut("messages")
                .and_then(Value::as_array_mut)
                .expect("messages");
            before_messages = messages.len();

            while messages
                .last()
                .map(|entry| {
                    clean_text(entry.get("role").and_then(Value::as_str).unwrap_or(""), 24)
                        .eq_ignore_ascii_case("system")
                })
                .unwrap_or(false)
            {
                if let Some(last) = messages.pop() {
                    removed.push(last);
                }
            }

            if messages
                .last()
                .map(|entry| {
                    let role =
                        clean_text(entry.get("role").and_then(Value::as_str).unwrap_or(""), 24)
                            .to_ascii_lowercase();
                    role == "assistant" || role == "agent"
                })
                .unwrap_or(false)
            {
                if let Some(last) = messages.pop() {
                    removed.push(last);
                }
            }

            if messages
                .last()
                .map(|entry| {
                    clean_text(entry.get("role").and_then(Value::as_str).unwrap_or(""), 24)
                        .eq_ignore_ascii_case("user")
                })
                .unwrap_or(false)
            {
                if let Some(last) = messages.pop() {
                    removed.push(last);
                }
            }

            if removed.is_empty() {
                if let Some(last) = messages.pop() {
                    removed.push(last);
                }
            }

            after_messages = messages.len();
            let removed_excerpt = removed
                .iter()
                .rev()
                .map(|entry| {
                    json!({
                        "role": clean_text(entry.get("role").and_then(Value::as_str).unwrap_or(""), 24),
                        "text": clean_text(&message_text(entry), 220),
                        "ts": entry.get("ts").cloned().unwrap_or(Value::Null)
                    })
                })
                .collect::<Vec<_>>();
            rollback_id = format!(
                "rbk-{}",
                &crate::deterministic_receipt_hash(&json!({
                    "agent_id": id.as_str(),
                    "removed_count": removed.len(),
                    "before": before_messages,
                    "after": after_messages,
                    "at": crate::now_iso()
                }))[..12]
            );
            if !row
                .get("rollback_archives")
                .map(Value::is_array)
                .unwrap_or(false)
            {
                row["rollback_archives"] = Value::Array(Vec::new());
            }
            if let Some(archives) = row
                .get_mut("rollback_archives")
                .and_then(Value::as_array_mut)
            {
                archives.push(json!({
                    "rollback_id": rollback_id.clone(),
                    "captured_at": crate::now_iso(),
                    "removed_count": removed.len(),
                    "removed_messages": removed_excerpt
                }));
                if archives.len() > 24 {
                    let trim = archives.len().saturating_sub(24);
                    archives.drain(0..trim);
                }
            }
            row["updated_at"] = Value::String(crate::now_iso());
            break;
        }
    }
    save_session_state(root, &id, &state);
    json!({
        "ok": !removed.is_empty(),
        "type": "dashboard_agent_session_rollback",
        "agent_id": id,
        "rollback_id": rollback_id,
        "removed_count": removed.len(),
        "before_messages": before_messages,
        "after_messages": after_messages,
        "removed_excerpt": removed
            .iter()
            .rev()
            .map(|entry| clean_text(&message_text(entry), 160))
            .filter(|text| !text.is_empty())
            .take(3)
            .collect::<Vec<_>>()
    })
}

fn reset_active_session(root: &Path, agent_id: &str) -> Value {
    let id = clean_agent_id(agent_id);
    if id.is_empty() {
        return json!({"ok": false, "error": "agent_id_required"});
    }
    let mut state = load_session_state(root, &id);
    let active_id = clean_text(
        state
            .get("active_session_id")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        120,
    );
    if let Some(rows) = state.get_mut("sessions").and_then(Value::as_array_mut) {
        for row in rows.iter_mut() {
            let sid = clean_text(
                row.get("session_id").and_then(Value::as_str).unwrap_or(""),
                120,
            );
            if sid == active_id {
                row["messages"] = Value::Array(Vec::new());
                row["updated_at"] = Value::String(crate::now_iso());
                break;
            }
        }
    }
    save_session_state(root, &id, &state);
    json!({
        "ok": true,
        "type": "dashboard_agent_session_reset",
        "agent_id": id,
        "active_session_id": active_id
    })
}

fn compaction_message_text(row: &Value) -> String {
    let text = message_text(row);
    if !text.is_empty() {
        return clean_text(&text, 4000);
    }
    clean_text(
        row.get("summary").and_then(Value::as_str).unwrap_or(""),
        4000,
    )
}

fn build_context_keyframes_from_removed(removed: &[Value], max_keyframes: usize) -> Vec<Value> {
    if removed.is_empty() {
        return Vec::new();
    }
    let cap = max_keyframes.clamp(1, 24);
    let chunk_size = ((removed.len() as f64 / cap as f64).ceil() as usize).max(1);
    let mut out = Vec::<Value>::new();
    for (idx, chunk) in removed.chunks(chunk_size).enumerate() {
        if out.len() >= cap {
            break;
        }
        let mut highlights = Vec::<String>::new();
        for row in chunk {
            let role = clean_text(row.get("role").and_then(Value::as_str).unwrap_or(""), 20)
                .to_ascii_lowercase();
            let text = compaction_message_text(row);
            if text.is_empty() {
                continue;
            }
            let prefix = if role.is_empty() {
                "note".to_string()
            } else {
                role
            };
            highlights.push(format!("{prefix}: {}", clean_text(&text, 120)));
            if highlights.len() >= 2 {
                break;
            }
        }
        let summary = if highlights.is_empty() {
            format!(
                "Compaction batch {} summarized {} older turns.",
                idx + 1,
                chunk.len()
            )
        } else {
            highlights.join(" | ")
        };
        let key_seed = json!({"batch": idx + 1, "summary": summary, "count": chunk.len()});
        let key_hash = crate::deterministic_receipt_hash(&key_seed);
        out.push(json!({
            "keyframe_id": format!("kf-{}", &key_hash[..12]),
            "batch": idx + 1,
            "turns_covered": chunk.len(),
            "summary": clean_text(&summary, 260),
            "captured_at": crate::now_iso()
        }));
    }
    out
}
