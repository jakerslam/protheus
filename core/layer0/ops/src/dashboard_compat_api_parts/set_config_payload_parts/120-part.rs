fn store_pending_tool_confirmation(
    root: &Path,
    agent_id: &str,
    tool_name: &str,
    input: &Value,
    source: &str,
) {
    let id = clean_agent_id(agent_id);
    if id.is_empty() {
        return;
    }
    let normalized_tool = normalize_tool_name(tool_name);
    if normalized_tool.is_empty() {
        return;
    }
    let input_payload = if input.is_object() {
        input.clone()
    } else {
        json!({})
    };
    let patch = json!({
        "pending_tool_confirmation": {
            "tool_name": normalized_tool,
            "input": input_payload,
            "source": clean_text(source, 80),
            "updated_at": crate::now_iso()
        }
    });
    let _ = update_profile_patch(root, &id, &patch);
}

fn clear_pending_tool_confirmation(root: &Path, agent_id: &str) {
    let id = clean_agent_id(agent_id);
    if id.is_empty() {
        return;
    }
    let _ = update_profile_patch(
        root,
        &id,
        &json!({"pending_tool_confirmation": Value::Null}),
    );
}

fn message_requests_comparative_answer(message: &str) -> bool {
    let lowered = clean_text(message, 400).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    let asks_direct_compare = lowered.contains("compare")
        || lowered.contains("comparison")
        || lowered.contains("vs")
        || lowered.contains("versus");
    let mentions_subject = lowered.contains("infring")
        || lowered.contains("openclaw")
        || lowered.contains("this platform")
        || lowered.contains("this system")
        || lowered.contains("this workspace")
        || lowered.contains("this repo")
        || lowered.contains("this repository")
        || lowered.contains("this codebase")
        || lowered.contains("this project")
        || lowered.contains("workspace")
        || lowered.contains("repository")
        || lowered.contains("codebase")
        || lowered.contains("project");
    let asks_peer_position = (lowered.contains("peer")
        || lowered.contains("peers")
        || lowered.contains("competitor")
        || lowered.contains("competitors")
        || lowered.contains("among")
        || lowered.contains("rank")
        || lowered.contains("ranking")
        || lowered.contains("grade"))
        && mentions_subject;
    asks_direct_compare || asks_peer_position
}

fn strip_context_guard_markers(text: &str) -> String {
    let mut out = clean_text(text, 8_000);
    if out.is_empty() {
        return String::new();
    }
    let patterns = [
        r"\[\.\.\.\s+\d+\s+more characters truncated\]",
        r"\[\.\.\.\s*middle content omitted[^\]]*\]",
        r"Context overflow:\s*estimated context size exceeds safe threshold during tool loop\.?",
    ];
    for pattern in patterns {
        if let Ok(regex) = regex::Regex::new(pattern) {
            out = regex.replace_all(&out, " ").to_string();
        }
    }
    clean_text(&out, 8_000)
}

fn latest_assistant_message_text(messages: &[Value]) -> String {
    for row in messages.iter().rev() {
        let role = clean_text(row.get("role").and_then(Value::as_str).unwrap_or(""), 40)
            .to_ascii_lowercase();
        if role != "assistant" {
            continue;
        }
        let text = clean_text(
            row.get("text")
                .or_else(|| row.get("content"))
                .or_else(|| row.get("message"))
                .and_then(Value::as_str)
                .unwrap_or(""),
            2_000,
        );
        if !text.is_empty() {
            return text;
        }
    }
    String::new()
}

fn maybe_tooling_failure_fallback(
    message: &str,
    finalized_response: &str,
    latest_assistant_response: &str,
) -> Option<String> {
    if !response_is_no_findings_placeholder(finalized_response)
        && !response_looks_like_tool_ack_without_findings(finalized_response)
        && !response_mentions_context_guard(finalized_response)
    {
        return None;
    }
    if let Some(specialized) = follow_up_suggestion_no_findings_fallback(message) {
        return Some(specialized);
    }
    let asks_diagnosis = message_requests_tooling_failure_diagnosis(message);
    let repeated_placeholder = !latest_assistant_response.trim().is_empty()
        && response_is_no_findings_placeholder(latest_assistant_response)
        && normalize_placeholder_signature(latest_assistant_response)
            == normalize_placeholder_signature(finalized_response);
    if response_mentions_context_guard(finalized_response) && (asks_diagnosis || repeated_placeholder)
    {
        return Some(web_tool_context_guard_fallback(
            "Live web retrieval",
        ));
    }
    if asks_diagnosis || repeated_placeholder {
        return Some(tooling_failure_diagnostic_fallback());
    }
    None
}

fn response_looks_like_raw_web_artifact_dump(text: &str) -> bool {
    let cleaned = clean_text(text, 4_000);
    if cleaned.is_empty() {
        return false;
    }
    let lowered = cleaned.to_ascii_lowercase();
    let has_explanatory_frame = lowered.contains("this means")
        || lowered.contains("this suggests")
        || lowered.contains("root cause")
        || lowered.contains("because ")
        || lowered.contains("in short");
    if has_explanatory_frame {
        return false;
    }
    if looks_like_placeholder_fetch_content(&cleaned, "") {
        return true;
    }
    if looks_like_navigation_chrome_payload(&cleaned)
        || looks_like_search_engine_chrome_summary(&cleaned)
    {
        return true;
    }
    lowered.contains("hacker news")
        && lowered.contains("new | past | comments")
        && lowered.contains("points by")
}

fn response_looks_like_unsynthesized_web_snippet_dump(text: &str) -> bool {
    let cleaned = clean_text(text, 4_000);
    if cleaned.is_empty() {
        return false;
    }
    let lowered = cleaned.to_ascii_lowercase();
    if !(lowered.starts_with("from web retrieval:")
        || lowered.starts_with("web benchmark synthesis:")
        || lowered.starts_with("key findings for"))
    {
        return false;
    }
    if looks_like_search_engine_chrome_summary(&lowered)
        || lowered.contains("search response came from")
        || lowered.contains("bing.com:")
        || lowered.contains("duckduckgo.com:")
    {
        return true;
    }
    let has_analysis_frame = lowered.contains("because ")
        || lowered.contains("therefore")
        || lowered.contains("in short")
        || lowered.contains("overall")
        || lowered.contains("recommend")
        || lowered.contains("trade-off")
        || lowered.contains("tradeoff");
    let domains = extract_search_result_domains(&cleaned, 8);
    domains.len() >= 2 && !has_analysis_frame
}

fn tool_is_autonomous_spawn(normalized: &str) -> bool {
    matches!(
        normalized,
        "spawn_subagents" | "spawn_swarm" | "agent_spawn" | "sessions_spawn"
    )
}

fn tool_capability_tier(normalized: &str, input: &Value) -> &'static str {
    if tool_is_autonomous_spawn(normalized) {
        return "green";
    }
    if is_terminal_tool_name(normalized) {
        return "green";
    }
    match normalized {
        "agent_action" | "manage_agent" => {
            let action = clean_text(
                input.get("action").and_then(Value::as_str).unwrap_or(""),
                80,
            )
            .to_ascii_lowercase();
            if matches!(
                action.as_str(),
                "archive" | "delete" | "spawn" | "spawn_subagent"
            ) {
                "red"
            } else {
                "yellow"
            }
        }
        "memory_kv_set" | "memory_kv_delete" => "yellow",
        "cron_schedule" | "schedule_task" | "cron_create" => "yellow",
        "cron_run" | "schedule_run" | "cron_trigger" => "yellow",
        "cron_cancel" | "cron_delete" | "schedule_cancel" => "yellow",
        _ => "green",
    }
}

fn enforce_tool_capability_tier(
    root: &Path,
    snapshot: &Value,
    actor_agent_id: &str,
    normalized_tool: &str,
    input: &Value,
) -> Option<Value> {
    if parent_can_archive_descendant_without_signoff(
        root,
        snapshot,
        actor_agent_id,
        normalized_tool,
        input,
    ) {
        return None;
    }
    let policy = tool_governance_policy(root);
    if !policy
        .get("enabled")
        .and_then(Value::as_bool)
        .unwrap_or(true)
    {
        return None;
    }
    let tier = tool_capability_tier(normalized_tool, input);
    let confirm_required = policy
        .pointer(&format!("/tiers/{tier}/confirm_required"))
        .and_then(Value::as_bool)
        .unwrap_or(matches!(tier, "yellow" | "red"));
    let min_note = policy
        .pointer(&format!("/tiers/{tier}/approval_note_min"))
        .and_then(Value::as_i64)
        .unwrap_or(if tier == "red" { 8 } else { 0 })
        .max(0) as usize;
    let confirmed = input_has_confirmation(input);
    let note = input_approval_note(input);
    if (!confirm_required || confirmed) && note.len() >= min_note {
        return None;
    }
    let next_step = if tier == "red" {
        format!(
            "Re-run with {{\"confirm\":true,\"approval_note\":\"why this destructive action is needed\"}} for `{}`.",
            normalized_tool
        )
    } else {
        format!(
            "Re-run with {{\"confirm\":true}} to execute `{}`.",
            normalized_tool
        )
    };
    let receipt = crate::deterministic_receipt_hash(&json!({
        "type": "tool_capability_tier_gate",
        "actor_agent_id": actor_agent_id,
        "tool": normalized_tool,
        "tier": tier,
        "confirmed": confirmed,
        "approval_note_len": note.len(),
        "ts": crate::now_iso()
    }));
    Some(json!({
        "ok": false,
        "error": if tier == "red" { "tool_explicit_signoff_required" } else { "tool_confirmation_required" },
        "tool": normalized_tool,
        "capability_tier": tier,
        "confirm_required": confirm_required,
        "approval_note_min_chars": min_note,
        "next_step": next_step,
        "receipt_hash": receipt
    }))
}

fn spawn_guard_policy(root: &Path) -> Value {
    let spawn_policy = read_json_loose(&root.join("client/runtime/config/spawn_policy.json"))
        .unwrap_or_else(|| json!({}));
    let child_policy =
        read_json_loose(&root.join("client/runtime/config/child_organ_runtime_policy.json"))
            .unwrap_or_else(|| json!({}));
    let orchestron_policy =
        read_json_loose(&root.join("client/runtime/config/orchestron_policy.json"))
            .unwrap_or_else(|| json!({}));
    let max_per_spawn = spawn_policy
        .pointer("/pool/max_cells")
        .and_then(Value::as_i64)
        .unwrap_or(8)
        .clamp(1, 64);
    let max_descendants_per_parent = child_policy
        .get("max_children")
        .and_then(Value::as_i64)
        .unwrap_or(24)
        .clamp(1, 4096);
    let max_depth = orchestron_policy
        .get("max_depth")
        .and_then(Value::as_i64)
        .unwrap_or(4)
        .clamp(1, 32);
    let per_child_budget_default = child_policy
        .pointer("/resource_envelope/token_cap_default")
        .and_then(Value::as_i64)
        .unwrap_or(800)
        .clamp(64, 200_000);
    let per_child_budget_max = child_policy
        .pointer("/resource_envelope/token_cap_max")
        .and_then(Value::as_i64)
        .unwrap_or(5000)
        .clamp(per_child_budget_default, 2_000_000);
    let spawn_budget_cap = per_child_budget_max
        .saturating_mul(max_per_spawn)
        .clamp(per_child_budget_max, 20_000_000);
    json!({
        "max_per_spawn": max_per_spawn,
        "max_descendants_per_parent": max_descendants_per_parent,
        "max_depth": max_depth,
        "per_child_budget_default": per_child_budget_default,
        "per_child_budget_max": per_child_budget_max,
        "spawn_budget_cap": spawn_budget_cap
    })
}

fn descendant_count(parent_map: &HashMap<String, String>, actor: &str) -> usize {
    let actor_id = clean_agent_id(actor);
    if actor_id.is_empty() {
        return 0;
    }
    let mut count = 0usize;
    for candidate in parent_map.keys() {
        let mut current = candidate.clone();
        let mut hops = 0usize;
        let mut seen = HashSet::<String>::new();
        while hops < 128 && seen.insert(current.clone()) {
            let Some(parent) = parent_map.get(&current).cloned() else {
                break;
            };
            if parent == actor_id {
                count += 1;
                break;
            }
            current = parent;
            hops += 1;
        }
    }
    count
}

fn agent_depth_from_parent_map(parent_map: &HashMap<String, String>, agent_id: &str) -> usize {
    let mut current = clean_agent_id(agent_id);
    if current.is_empty() {
        return 0;
    }
    let mut depth = 0usize;
    let mut seen = HashSet::<String>::new();
    while depth < 128 && seen.insert(current.clone()) {
        let Some(parent) = parent_map.get(&current).cloned() else {
            break;
        };
        current = parent;
        depth += 1;
    }
    depth
}

fn subagent_context_slice(root: &Path, parent_agent_id: &str, objective: &str) -> Value {
    let state = load_session_state(root, parent_agent_id);
    let mut messages = session_messages(&state);
    if messages.is_empty() {
        return json!({
            "strategy": "objective_scoped_recent_window",
            "selected_messages": [],
            "selected_count": 0
        });
    }
    let objective_tokens = workspace_hint_tokens(objective, 10);
    messages.sort_by_key(message_timestamp_iso);
    let mut scored = Vec::<(i64, Value)>::new();
    for (idx, row) in messages.into_iter().enumerate() {
        let role = clean_text(row.get("role").and_then(Value::as_str).unwrap_or(""), 20)
            .to_ascii_lowercase();
        if role.is_empty() {
            continue;
        }
        let text = message_text(&row);
        if text.is_empty() {
            continue;
        }
        let mut score = (idx as i64).min(40);
        let lowered = text.to_ascii_lowercase();
        for token in &objective_tokens {
            if lowered.contains(token) {
                score += 5;
            }
        }
        if role == "user" {
            score += 2;
        }
        scored.push((
            score,
            json!({
                "role": role,
                "text": trim_text(&text, 600),
                "ts": message_timestamp_iso(&row)
            }),
        ));
    }
    scored.sort_by(|a, b| b.0.cmp(&a.0));
    let selected = scored
        .into_iter()
        .take(12)
        .map(|(_, row)| row)
        .collect::<Vec<_>>();
    let selected_count = selected.len();
    json!({
        "strategy": "objective_scoped_recent_window",
        "objective_tokens": objective_tokens,
        "selected_messages": selected,
        "selected_count": selected_count
    })
}

fn tool_decision_audit_path(root: &Path) -> PathBuf {
    root.join("client/runtime/local/state/ui/infring_dashboard/decision_audit.jsonl")
}

fn append_tool_decision_audit(
    root: &Path,
    actor_agent_id: &str,
    tool_name: &str,
    tool_input: &Value,
    tool_output: &Value,
    recovery_strategy: &str,
) -> String {
    let tier = tool_capability_tier(tool_name, tool_input);
    let row = json!({
        "type": "tool_decision_audit",
        "timestamp": crate::now_iso(),
        "actor_agent_id": clean_agent_id(actor_agent_id),
        "tool": normalize_tool_name(tool_name),
        "capability_tier": tier,
        "ok": tool_output.get("ok").and_then(Value::as_bool).unwrap_or(false),
        "error": clean_text(tool_output.get("error").and_then(Value::as_str).unwrap_or(""), 240),
        "recovery_strategy": clean_text(recovery_strategy, 80),
        "input_hash": crate::deterministic_receipt_hash(tool_input),
        "output_hash": crate::deterministic_receipt_hash(tool_output)
    });
    let receipt = crate::deterministic_receipt_hash(&row);
    append_jsonl_row(&tool_decision_audit_path(root), &row);
    receipt
}
