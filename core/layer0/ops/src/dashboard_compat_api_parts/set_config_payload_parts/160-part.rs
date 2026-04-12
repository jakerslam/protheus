fn fallback_memory_query_payload(
    root: &Path,
    actor_agent_id: &str,
    tool_name: &str,
    input: &Value,
) -> Option<Value> {
    let normalized = normalize_tool_name(tool_name);
    if normalized != "web_search"
        && normalized != "search_web"
        && normalized != "search"
        && normalized != "web_query"
        && normalized != "web_fetch"
        && normalized != "browse"
        && normalized != "web_conduit_fetch"
    {
        return None;
    }
    let query =
        if normalized == "web_fetch" || normalized == "browse" || normalized == "web_conduit_fetch"
        {
            clean_text(
                input
                    .get("url")
                    .or_else(|| input.get("query"))
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                600,
            )
        } else {
            clean_text(
                input
                    .get("query")
                    .or_else(|| input.get("q"))
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                600,
            )
        };
    if query.is_empty() {
        return None;
    }
    let fallback =
        crate::dashboard_agent_state::memory_kv_semantic_query(root, actor_agent_id, &query, 5);
    let matches = fallback
        .get("matches")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if matches.is_empty() {
        return None;
    }
    let summary = summarize_tool_payload("memory_semantic_query", &fallback);
    Some(json!({
        "ok": true,
        "type": "tool_degraded_fallback",
        "tool": normalized,
        "fallback_tool": "memory_semantic_query",
        "query": query,
        "summary": summary,
        "matches": matches,
        "fallback_used": true
    }))
}

fn execute_tool_call_with_recovery(
    root: &Path,
    snapshot: &Value,
    actor_agent_id: &str,
    existing: Option<&Value>,
    tool_name: &str,
    input: &Value,
) -> Value {
    if let Some(blocked) =
        crate::dashboard_tool_turn_loop::pre_tool_permission_gate(root, tool_name, input)
    {
        return blocked;
    }
    let nexus_connection =
        match crate::dashboard_tool_turn_loop::authorize_ingress_tool_call_with_nexus(tool_name) {
            Ok(meta) => meta,
            Err(err) => {
                return json!({
                    "ok": false,
                    "error": "tool_nexus_delivery_denied",
                    "message": "Tool execution blocked by hierarchical nexus ingress policy.",
                    "tool": normalize_tool_name(tool_name),
                    "fail_closed": true,
                    "nexus_error": clean_text(&err, 240)
                })
            }
        };
    let mut payload =
        execute_tool_call_by_name(root, snapshot, actor_agent_id, existing, tool_name, input);
    let mut recovery_strategy = "none".to_string();
    let mut recovery_attempts = 0_u64;
    if transient_tool_failure(&payload) {
        for delay_ms in [180_u64, 360, 720] {
            recovery_attempts += 1;
            std::thread::sleep(std::time::Duration::from_millis(delay_ms));
            let retry = execute_tool_call_by_name(
                root,
                snapshot,
                actor_agent_id,
                existing,
                tool_name,
                input,
            );
            if retry.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                payload = retry;
                recovery_strategy = format!("retry_backoff_attempt_{recovery_attempts}");
                break;
            }
            payload = retry;
        }
        if !payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
            if let Some(fallback_payload) = fallback_memory_query_payload(
                root,
                &clean_agent_id(actor_agent_id),
                tool_name,
                input,
            ) {
                payload = fallback_payload;
                recovery_strategy = "semantic_memory_fallback".to_string();
            } else {
                recovery_strategy = "retry_backoff_exhausted".to_string();
            }
        }
    }
    crate::dashboard_tool_turn_loop::annotate_tool_payload_tracking(
        root,
        actor_agent_id,
        tool_name,
        &mut payload,
    );
    let audit_receipt = append_tool_decision_audit(
        root,
        actor_agent_id,
        tool_name,
        input,
        &payload,
        &recovery_strategy,
    );
    if let Some(obj) = payload.as_object_mut() {
        obj.insert(
            "recovery_strategy".to_string(),
            Value::String(recovery_strategy),
        );
        obj.insert(
            "decision_audit_receipt".to_string(),
            Value::String(audit_receipt),
        );
        obj.insert("recovery_attempts".to_string(), json!(recovery_attempts));
        if let Some(meta) = nexus_connection {
            obj.insert("nexus_connection".to_string(), meta);
        }
    }
    if tool_pipeline_supported_tool(tool_name) {
        let trace_id = crate::deterministic_receipt_hash(&json!({
            "type": "tool_pipeline_trace",
            "tool_name": normalize_tool_name(tool_name),
            "actor_agent_id": clean_agent_id(actor_agent_id),
            "task_seed": clean_text(&input.to_string(), 400)
        }));
        let task_id = {
            let cleaned = clean_agent_id(actor_agent_id);
            if cleaned.is_empty() {
                "agent-unknown".to_string()
            } else {
                cleaned
            }
        };
        let raw_snapshot = payload.clone();
        let pipeline =
            tooling_pipeline_execute(&trace_id, &task_id, tool_name, input, |_| Ok(raw_snapshot));
        attach_tool_pipeline(&mut payload, &pipeline);
    }
    payload
}

fn execute_inline_tool_calls(
    root: &Path,
    snapshot: &Value,
    actor_agent_id: &str,
    existing: Option<&Value>,
    response_text: &str,
    user_message: &str,
    allow_inline_calls: bool,
) -> (String, Vec<Value>, Option<Value>, bool) {
    let (cleaned, calls) = extract_inline_tool_calls(response_text, 6);
    if calls.is_empty() {
        return (response_text.to_string(), Vec::new(), None, false);
    }
    if !allow_inline_calls {
        return (trim_text(cleaned.trim(), 32_000), Vec::new(), None, true);
    }
    let mut cards = Vec::<Value>::new();
    let mut fallback_lines = Vec::<String>::new();
    let mut pending_confirmation: Option<Value> = None;
    for (idx, (name, input, _raw)) in calls.into_iter().enumerate() {
        let mut input_for_call = input.clone();
        let normalized_name = normalize_tool_name(&name);
        let user_requested_swarm = swarm_intent_requested(user_message)
            || user_message.to_ascii_lowercase().contains("multi-agent")
            || user_message.to_ascii_lowercase().contains("multi agent");
        if matches!(
            normalized_name.as_str(),
            "spawn_subagents" | "spawn_swarm" | "agent_spawn" | "sessions_spawn"
        ) {
            if !input_for_call.is_object() {
                input_for_call = json!({
                    "objective": clean_text(user_message, 800)
                });
            }
            if !input_has_confirmation(&input_for_call) {
                input_for_call["confirm"] = Value::Bool(true);
            }
            let approval_note = clean_text(
                input_for_call
                    .get("approval_note")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                200,
            );
            if approval_note.is_empty() {
                input_for_call["approval_note"] = Value::String(if user_requested_swarm {
                    "user requested explicit swarm execution".to_string()
                } else {
                    "autonomous decomposition spawn".to_string()
                });
            }
        }
        let payload = execute_tool_call_with_recovery(
            root,
            snapshot,
            actor_agent_id,
            existing,
            &name,
            &input_for_call,
        );
        let ok = payload.get("ok").and_then(Value::as_bool).unwrap_or(false);
        let result_text = summarize_tool_payload(&name, &payload);
        if !ok
            && tool_error_requires_confirmation(&payload)
            && pending_confirmation.is_none()
            && !normalized_name.is_empty()
        {
            pending_confirmation = Some(json!({
                "tool_name": normalized_name,
                "input": input_for_call.clone(),
                "source": "inline_tool_call"
            }));
        }
        cards.push(json!({
            "id": format!("tool-{}-{}", normalize_tool_name(&name), idx),
            "name": normalize_tool_name(&name),
            "input": trim_text(&input_for_call.to_string(), 4000),
            "result": trim_text(&result_text, 24_000),
            "is_error": !ok
        }));
        if ok && !result_text.trim().is_empty() {
            if !response_looks_like_tool_ack_without_findings(&result_text) {
                fallback_lines.push(result_text);
            }
        } else if !ok {
            if let Some(line) = user_facing_tool_failure_summary(&name, &payload) {
                fallback_lines.push(line);
            }
        }
    }
    let cleaned_trimmed = cleaned.trim();
    let cleaned_is_low_signal = response_looks_like_tool_ack_without_findings(cleaned_trimmed)
        || response_looks_like_unsynthesized_web_snippet_dump(cleaned_trimmed)
        || response_looks_like_raw_web_artifact_dump(cleaned_trimmed)
        || response_is_no_findings_placeholder(cleaned_trimmed)
        || response_contains_tool_telemetry_dump(cleaned_trimmed);
    let response = if cleaned_trimmed.is_empty() || cleaned_is_low_signal {
        let joined = fallback_lines.join("\n\n");
        if joined.trim().is_empty() {
            "I executed the requested tool calls, but this turn produced no verified findings. No source-backed evidence was recorded. Run `tool::capabilities` to confirm available command surfaces, then retry with a narrower query or a specific source."
                .to_string()
        } else {
            trim_text(&joined, 32_000)
        }
    } else {
        trim_text(cleaned_trimmed, 32_000)
    };
    let (contracted_response, _contract_report) =
        enforce_tool_completion_contract(response, &cards);
    (contracted_response, cards, pending_confirmation, false)
}

fn first_http_url_in_text(text: &str) -> String {
    let cleaned = clean_text(text, 2200);
    for token in cleaned.split_whitespace() {
        if token.starts_with("http://") || token.starts_with("https://") {
            return clean_text(
                token.trim_matches(|ch| matches!(ch, ')' | ']' | '>' | ',')),
                2200,
            );
        }
    }
    String::new()
}

fn parse_cron_interval_minutes(token: &str) -> Option<i64> {
    let raw = clean_text(token, 40).to_ascii_lowercase();
    if raw.is_empty() {
        return None;
    }
    let (number_part, multiplier) = if raw.ends_with('m') {
        (&raw[..raw.len().saturating_sub(1)], 1i64)
    } else if raw.ends_with('h') {
        (&raw[..raw.len().saturating_sub(1)], 60i64)
    } else if raw.ends_with('d') {
        (&raw[..raw.len().saturating_sub(1)], 1440i64)
    } else {
        (raw.as_str(), 1i64)
    };
    let parsed = number_part.trim().parse::<i64>().ok()?;
    if parsed <= 0 {
        return None;
    }
    Some((parsed * multiplier).clamp(1, 10_080))
}

fn cron_tool_request_from_args(args: &str) -> Option<(String, Value)> {
    let trimmed = clean_text(args, 1_200);
    if trimmed.trim().is_empty() {
        return Some(("cron_list".to_string(), json!({})));
    }
    let mut parts = trimmed.splitn(2, char::is_whitespace);
    let action = parts
        .next()
        .map(|value| value.trim().to_ascii_lowercase())
        .unwrap_or_default();
    let rest = parts.next().map(str::trim).unwrap_or("");
    match action.as_str() {
        "list" | "ls" | "status" | "jobs" => Some(("cron_list".to_string(), json!({}))),
        "cancel" | "delete" | "remove" | "rm" => {
            let job_id = clean_text(rest, 140);
            if job_id.is_empty() {
                None
            } else {
                Some((
                    "cron_cancel".to_string(),
                    json!({"job_id": job_id, "confirm": true}),
                ))
            }
        }
        "run" | "trigger" => {
            let job_id = clean_text(rest, 140);
            if job_id.is_empty() {
                None
            } else {
                Some((
                    "cron_run".to_string(),
                    json!({"job_id": job_id, "confirm": true}),
                ))
            }
        }
        "schedule" | "every" | "in" => {
            let mut schedule_parts = rest.splitn(2, char::is_whitespace);
            let interval_token = schedule_parts.next().map(str::trim).unwrap_or("");
            let mut message = schedule_parts.next().map(str::trim).unwrap_or("");
            let mut interval_minutes = parse_cron_interval_minutes(interval_token);
            if interval_minutes.is_none() {
                if action == "schedule" && !rest.is_empty() {
                    interval_minutes = Some(60);
                    message = rest;
                } else {
                    return None;
                }
            }
            let minutes = interval_minutes.unwrap_or(60);
            let text = clean_text(message, 2_000);
            Some((
                "cron_schedule".to_string(),
                json!({
                    "interval_minutes": minutes,
                    "message": if text.is_empty() {
                        "Scheduled follow-up check."
                    } else {
                        text.as_str()
                    },
                    "confirm": true
                }),
            ))
        }
        _ => {
            if let Some(minutes) = parse_cron_interval_minutes(&action) {
                let text = clean_text(rest, 2_000);
                return Some((
                    "cron_schedule".to_string(),
                    json!({
                        "interval_minutes": minutes,
                        "message": if text.is_empty() {
                            "Scheduled follow-up check."
                        } else {
                            text.as_str()
                        },
                        "confirm": true
                    }),
                ));
            }
            None
        }
    }
}

fn natural_web_intent_from_user_message(message: &str) -> Option<(String, Value)> {
    let trimmed = message.trim();
    if trimmed.is_empty() {
        return None;
    }
    let lowered = clean_text(trimmed, 2200).to_ascii_lowercase();
    let url = first_http_url_in_text(trimmed);
    if !url.is_empty() {
        let asks_browse = lowered.contains("browse")
            || lowered.contains("fetch")
            || lowered.contains("read this")
            || lowered.contains("summarize")
            || lowered.contains("look at")
            || lowered.contains("open")
            || lowered.contains("web");
        if asks_browse {
            return Some(("web_fetch".to_string(), json!({"url": url, "summary_only": true})));
        }
    }

    if let Some(route) = comparative_natural_web_intent_from_message(trimmed) {
        return Some(route);
    }

    let prefixes = [
        "search the web for ",
        "search web for ",
        "search for ",
        "web search for ",
        "look up ",
        "find online ",
    ];
    for prefix in prefixes {
        if lowered.starts_with(prefix) {
            let query = clean_text(&trimmed[prefix.len()..], 600);
            if !query.is_empty() {
                return Some(("batch_query".to_string(), json!({"source": "web", "query": query, "aperture": "medium"})));
            }
        }
    }
    None
}

fn levenshtein_distance(left: &str, right: &str) -> usize {
    if left == right {
        return 0;
    }
    if left.is_empty() {
        return right.chars().count();
    }
    if right.is_empty() {
        return left.chars().count();
    }
    let right_chars = right.chars().collect::<Vec<_>>();
    let mut costs = (0..=right_chars.len()).collect::<Vec<usize>>();
    for (left_idx, left_ch) in left.chars().enumerate() {
        let mut diagonal = costs[0];
        costs[0] = left_idx + 1;
        for (right_idx, right_ch) in right_chars.iter().enumerate() {
            let next_diagonal = costs[right_idx + 1];
            let substitution = diagonal + if left_ch == *right_ch { 0 } else { 1 };
            let insertion = costs[right_idx + 1] + 1;
            let deletion = costs[right_idx] + 1;
            costs[right_idx + 1] = substitution.min(insertion).min(deletion);
            diagonal = next_diagonal;
        }
    }
    costs[right_chars.len()]
}

const EXPLICIT_SUPPORTED_TOOL_COMMANDS: &[&str] = &["capabilities", "web_search", "web_fetch", "spawn_subagents", "manage_agent", "batch_query", "memory_store", "memory_retrieve", "workspace_analyze"];

fn closest_supported_tool_command(command: &str) -> Option<&'static str> {
    let mut best = None::<(&'static str, usize)>;
    for candidate in EXPLICIT_SUPPORTED_TOOL_COMMANDS {
        let distance = levenshtein_distance(command, candidate);
        if best.map(|(_, current)| distance < current).unwrap_or(true) {
            best = Some((candidate, distance));
        }
    }
    let (candidate, distance) = best?;
    if distance <= 3 || distance.saturating_mul(2) <= command.len().max(candidate.len()) {
        Some(candidate)
    } else {
        None
    }
}
