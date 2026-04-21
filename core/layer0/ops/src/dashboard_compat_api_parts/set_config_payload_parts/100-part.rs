fn context_keyframes_prompt_context(
    state: &Value,
    max_keyframes: usize,
    max_chars: usize,
) -> String {
    let active_id = clean_text(state.get("active_session_id").and_then(Value::as_str).unwrap_or("default"), 120);
    let sessions = state.get("sessions").and_then(Value::as_array).cloned().unwrap_or_default();
    let mut keyframes = Vec::<String>::new();
    let mut tool_outcomes = Vec::<String>::new();
    for session in sessions {
        let sid = clean_text(session.get("session_id").and_then(Value::as_str).unwrap_or(""), 120);
        if sid != active_id {
            continue;
        }
        let entries = session
            .get("context_keyframes")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        for entry in entries.iter().rev().take(max_keyframes.max(1)) {
            let summary = clean_text(entry.get("summary").and_then(Value::as_str).or_else(|| entry.get("text").and_then(Value::as_str)).unwrap_or(""), 260);
            if summary.is_empty() {
                continue;
            }
            if internal_context_metadata_phrase(&summary)
                || persistent_memory_denied_phrase(&summary)
                || runtime_access_denied_phrase(&summary)
            {
                continue;
            }
            if clean_text(entry.get("kind").and_then(Value::as_str).unwrap_or(""), 40)
                == "tool_outcome"
            {
                tool_outcomes.push(summary);
            } else {
                keyframes.push(summary);
            }
        }
        break;
    }
    let mut sections = Vec::<String>::new();
    if !tool_outcomes.is_empty() {
        let joined = tool_outcomes.into_iter().rev().collect::<Vec<_>>().join(" | ");
        sections.push(trim_text(&format!("Recent tool outcomes:\n- {}", clean_text(&joined, max_chars)), max_chars));
    }
    if !keyframes.is_empty() {
        let joined = keyframes.into_iter().rev().collect::<Vec<_>>().join(" | ");
        sections.push(trim_text(&format!("Compacted thread keyframes:\n- {}", clean_text(&joined, max_chars)), max_chars));
    }
    if sections.is_empty() {
        String::new()
    } else {
        trim_text(&sections.join("\n\n"), max_chars)
    }
}

fn recent_tool_outcome_keyframes(state: &Value, max_keyframes: usize) -> Vec<Value> {
    let active_id = clean_text(state.get("active_session_id").and_then(Value::as_str).unwrap_or("default"), 120);
    let sessions = state.get("sessions").and_then(Value::as_array).cloned().unwrap_or_default();
    for session in sessions {
        let sid = clean_text(session.get("session_id").and_then(Value::as_str).unwrap_or(""), 120);
        if sid != active_id {
            continue;
        }
        return session
            .get("context_keyframes")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter(|entry| {
                clean_text(entry.get("kind").and_then(Value::as_str).unwrap_or(""), 40)
                    == "tool_outcome"
            })
            .rev()
            .take(max_keyframes.max(1))
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>();
    }
    Vec::new()
}

fn first_sentence(raw: &str, max_len: usize) -> String {
    let cleaned = clean_text(raw, max_len.saturating_mul(4).max(200));
    if cleaned.is_empty() {
        return String::new();
    }
    let mut sentence_end = cleaned.len();
    for (idx, ch) in cleaned.char_indices() {
        if ch == '.' || ch == '!' || ch == '?' {
            sentence_end = idx + ch.len_utf8();
            break;
        }
    }
    clean_text(&cleaned[..sentence_end], max_len)
}

fn agent_identity_hydration_prompt(row: &Value) -> String {
    let agent_id = clean_text(row.get("agent_id").or_else(|| row.get("id")).and_then(Value::as_str).unwrap_or(""), 160);
    let name = clean_text(row.get("name").and_then(Value::as_str).unwrap_or(""), 120);
    let resolved_name = if !name.is_empty() {
        name
    } else if !agent_id.is_empty() {
        humanize_agent_name(&agent_id)
    } else {
        "Agent".to_string()
    };
    let role = clean_text(row.get("role").and_then(Value::as_str).unwrap_or("assistant"), 80);
    let archetype = clean_text(row.pointer("/identity/archetype").and_then(Value::as_str).unwrap_or(""), 80);
    let vibe = clean_text(row.pointer("/identity/vibe").and_then(Value::as_str).unwrap_or(""), 80);
    let personality = first_sentence(row.get("system_prompt").and_then(Value::as_str).unwrap_or(""), 220);

    let mut profile_parts = vec![format!("name={resolved_name}"), format!("role={role}")];
    if !archetype.is_empty() {
        profile_parts.push(format!("archetype={archetype}"));
    }
    if !vibe.is_empty() {
        profile_parts.push(format!("vibe={vibe}"));
    }
    let mut lines = vec![format!("Agent identity hydration: {}.", profile_parts.join(", "))];
    if !personality.is_empty() {
        lines.push(format!("Personality directive: {personality}"));
    }
    lines.push(
        "When asked who you are, your name, or your role, reply using this profile in first person. Do not deny this identity unless profile metadata is changed later."
            .to_string(),
    );
    clean_text(&lines.join(" "), 1_600)
}

include!("../031-context-window-and-recall.rs");

fn set_active_session_messages(state: &mut Value, messages: &[Value]) {
    let active_id = clean_text(state.get("active_session_id").and_then(Value::as_str).unwrap_or("default"), 120);
    if let Some(rows) = state.get_mut("sessions").and_then(Value::as_array_mut) {
        for row in rows.iter_mut() {
            let sid = clean_text(row.get("session_id").and_then(Value::as_str).unwrap_or(""), 120);
            if sid != active_id {
                continue;
            }
            row["messages"] = Value::Array(messages.to_vec());
            row["updated_at"] = Value::String(crate::now_iso());
            break;
        }
    }
}

fn context_command_payload(
    root: &Path,
    agent_id: &str,
    row: &Value,
    request: &Value,
    silent: bool,
) -> Value {
    let state = load_session_state(root, agent_id);
    let sessions_total = state
        .get("sessions")
        .and_then(Value::as_array)
        .map(|rows| rows.len())
        .unwrap_or(0);
    let include_all_sessions_context = request
        .get("include_all_sessions_context")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let messages = context_source_messages(&state, include_all_sessions_context);
    let row_system_context_limit = row
        .get("system_context_tokens")
        .or_else(|| row.get("context_pool_limit_tokens"))
        .and_then(Value::as_i64)
        .unwrap_or(1_000_000);
    let context_pool_limit_tokens = request
        .get("context_pool_limit_tokens")
        .and_then(Value::as_i64)
        .unwrap_or(row_system_context_limit)
        .clamp(32_000, 2_000_000);
    let pooled_messages_unfloored = trim_context_pool(&messages, context_pool_limit_tokens);
    let pre_generation_pruned = pooled_messages_unfloored.len() != messages.len();
    let row_context_window = row
        .get("context_window_tokens")
        .or_else(|| row.get("context_window"))
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let context_window = if row_context_window > 0 {
        row_context_window
    } else {
        128_000
    };
    let active_context_target_tokens = request
        .get("active_context_target_tokens")
        .or_else(|| request.get("target_context_window"))
        .and_then(Value::as_i64)
        .unwrap_or_else(|| ((context_window as f64) * 0.68).round() as i64)
        .clamp(4_096, 512_000);
    let active_context_min_recent = request
        .get("active_context_min_recent_messages")
        .or_else(|| request.get("min_recent_messages"))
        .and_then(Value::as_u64)
        .unwrap_or(ACTIVE_CONTEXT_MIN_RECENT_FLOOR as u64)
        .clamp(ACTIVE_CONTEXT_MIN_RECENT_FLOOR as u64, 256)
        as usize;
    let recent_tool_outcomes = recent_tool_outcome_keyframes(&state, 4);
    let recent_floor_target = recent_context_floor_target_count(&messages, active_context_min_recent);
    let recent_floor_missing_before = recent_context_floor_missing_count(
        &messages,
        &pooled_messages_unfloored,
        active_context_min_recent,
    );
    let recent_floor_coverage_before = recent_context_floor_coverage_ratio(
        &messages,
        &pooled_messages_unfloored,
        active_context_min_recent,
    );
    let (pooled_messages, recent_floor_injected) = enforce_recent_context_floor(
        &messages,
        &pooled_messages_unfloored,
        active_context_min_recent,
    );
    let recent_floor_enforced = recent_floor_injected > 0;
    let recent_floor_satisfied =
        recent_context_floor_satisfied(&messages, &pooled_messages, active_context_min_recent);
    let recent_floor_coverage_after = recent_context_floor_coverage_ratio(
        &messages,
        &pooled_messages,
        active_context_min_recent,
    );
    let row_auto_compact_threshold_ratio = row
        .get("auto_compact_threshold_ratio")
        .and_then(Value::as_f64)
        .unwrap_or(0.95);
    let row_auto_compact_target_ratio = row
        .get("auto_compact_target_ratio")
        .and_then(Value::as_f64)
        .unwrap_or(0.72);
    let auto_compact_threshold_ratio = request
        .get("auto_compact_threshold_ratio")
        .and_then(Value::as_f64)
        .unwrap_or(row_auto_compact_threshold_ratio)
        .clamp(0.75, 0.99);
    let auto_compact_target_ratio = request
        .get("auto_compact_target_ratio")
        .and_then(Value::as_f64)
        .unwrap_or(row_auto_compact_target_ratio)
        .clamp(0.40, 0.90);
    let mut active_messages = select_active_context_window(
        &pooled_messages,
        active_context_target_tokens,
        active_context_min_recent,
    );
    let context_pool_tokens = total_message_tokens(&pooled_messages);
    let mut context_tokens = total_message_tokens(&active_messages);
    let mut context_ratio = if context_window > 0 {
        (context_tokens as f64 / context_window as f64).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let mut context_pressure = context_pressure_label(context_ratio).to_string();
    let mut emergency_compact = json!({
        "triggered": false,
        "threshold_ratio": auto_compact_threshold_ratio,
        "target_ratio": auto_compact_target_ratio,
        "removed_messages": 0
    });
    if context_ratio >= auto_compact_threshold_ratio && context_window > 0 {
        let emergency_target_tokens =
            ((context_window as f64) * auto_compact_target_ratio).round() as i64;
        let emergency_min_recent = request
            .get("emergency_min_recent_messages")
            .or_else(|| request.get("min_recent_messages"))
            .and_then(Value::as_u64)
            .unwrap_or(active_context_min_recent as u64)
            .clamp(ACTIVE_CONTEXT_MIN_RECENT_FLOOR as u64, 256)
            as usize;
        let emergency_messages = select_active_context_window(
            &pooled_messages,
            emergency_target_tokens,
            emergency_min_recent,
        );
        let emergency_tokens = total_message_tokens(&emergency_messages);
        let removed_messages = active_messages
            .len()
            .saturating_sub(emergency_messages.len()) as u64;
        emergency_compact = json!({
            "triggered": true,
            "threshold_ratio": auto_compact_threshold_ratio,
            "target_ratio": auto_compact_target_ratio,
            "removed_messages": removed_messages,
            "before_tokens": context_tokens,
            "after_tokens": emergency_tokens,
            "persisted_to_history": false
        });
        if removed_messages > 0 && emergency_tokens <= context_tokens {
            active_messages = emergency_messages;
            context_tokens = emergency_tokens;
            context_ratio = if context_window > 0 {
                (context_tokens as f64 / context_window as f64).clamp(0.0, 1.0)
            } else {
                0.0
            };
            context_pressure = context_pressure_label(context_ratio).to_string();
        }
    }
    let recent_floor_active_missing =
        recent_context_floor_missing_count(&messages, &active_messages, active_context_min_recent);
    let recent_floor_active_satisfied = recent_floor_active_missing == 0;
    let recent_floor_active_coverage =
        recent_context_floor_coverage_ratio(&messages, &active_messages, active_context_min_recent);
    let (
        recent_floor_continuity_status,
        recent_floor_continuity_action,
        recent_floor_continuity_message,
        recent_floor_continuity_reason,
        recent_floor_continuity_retryable,
    ) = if recent_floor_active_satisfied {
        (
            "ready".to_string(),
            "none".to_string(),
            "Active context satisfies the recent-floor continuity contract.".to_string(),
            "none".to_string(),
            false,
        )
    } else {
        (
            "degraded".to_string(),
            "raise_active_context_floor_or_target".to_string(),
            "Active context dropped below the recent-floor contract; increase min recent messages or target context tokens.".to_string(),
            "active_recent_floor_missing".to_string(),
            true,
        )
    };
    json!({
        "ok": true,
        "agent_id": agent_id,
        "command": "context",
        "silent": silent,
        "context_window": context_window,
        "context_tokens": context_tokens,
        "context_used_tokens": context_tokens,
        "context_ratio": context_ratio,
        "context_pressure": context_pressure,
        "context_pool": {
            "pool_limit_tokens": context_pool_limit_tokens,
            "pool_tokens": context_pool_tokens,
            "pool_messages": pooled_messages.len(),
            "session_count": sessions_total,
            "system_context_enabled": true,
            "system_context_limit_tokens": context_pool_limit_tokens,
            "llm_context_window_tokens": context_window,
            "active_target_tokens": active_context_target_tokens,
            "active_tokens": context_tokens,
            "active_messages": active_messages.len(),
            "min_recent_messages": active_context_min_recent,
            "include_all_sessions_context": include_all_sessions_context,
            "pre_generation_pruning_enabled": true,
            "pre_generation_pruned": pre_generation_pruned,
            "recent_floor_enforced": recent_floor_enforced,
            "recent_floor_injected": recent_floor_injected,
            "recent_floor_target": recent_floor_target,
            "recent_floor_missing_before": recent_floor_missing_before,
            "recent_floor_satisfied": recent_floor_satisfied,
            "recent_floor_coverage_before": recent_floor_coverage_before,
            "recent_floor_coverage_after": recent_floor_coverage_after,
            "recent_floor_active_missing": recent_floor_active_missing,
            "recent_floor_active_satisfied": recent_floor_active_satisfied,
            "recent_floor_active_coverage": recent_floor_active_coverage,
            "recent_floor_continuity_status": recent_floor_continuity_status,
            "recent_floor_continuity_action": recent_floor_continuity_action,
            "recent_floor_continuity_message": recent_floor_continuity_message,
            "recent_floor_continuity_reason": recent_floor_continuity_reason,
            "recent_floor_continuity_retryable": recent_floor_continuity_retryable,
            "emergency_compact_enabled": true,
            "emergency_compact": emergency_compact
        },
        "recent_tool_outcomes": recent_tool_outcomes,
        "message": format!(
            "Context window: {} tokens | Active: {} tokens ({}%) | Pressure: {}",
            context_window.max(0),
            context_tokens.max(0),
            ((context_ratio * 100.0).round() as i64).max(0),
            context_pressure
        )
    })
}

fn data_url_from_bytes(bytes: &[u8], content_type: &str) -> String {
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;
    format!(
        "data:{};base64,{}",
        clean_text(content_type, 120),
        STANDARD.encode(bytes)
    )
}
fn git_tree_payload_for_agent(root: &Path, snapshot: &Value, agent_id: &str) -> Value {
    let roster = build_agent_roster(root, snapshot, true);
    let mut counts = HashMap::<String, i64>::new();
    let mut current = Value::Null;
    for row in &roster {
        let branch = clean_text(
            row.get("git_branch").and_then(Value::as_str).unwrap_or(""),
            180,
        );
        if branch.is_empty() {
            continue;
        }
        *counts.entry(branch.clone()).or_insert(0) += 1;
        if clean_agent_id(row.get("id").and_then(Value::as_str).unwrap_or(""))
            == clean_agent_id(agent_id)
        {
            current = row.clone();
        }
    }
    let current_branch = clean_text(current.get("git_branch").and_then(Value::as_str).unwrap_or("main"), 180);
    let current_workspace = clean_text(current.get("workspace_dir").and_then(Value::as_str).unwrap_or(""), 4000);
    let current_workspace_dir = if current_workspace.is_empty() {
        root.to_path_buf()
    } else {
        PathBuf::from(&current_workspace)
    };
    let current_workspace_rel = current.get("workspace_rel").cloned().unwrap_or_else(|| {
        json!(crate::dashboard_git_runtime::workspace_rel(
            root,
            &current_workspace_dir
        ))
    });
    let (main_branch, mut branches) =
        crate::dashboard_git_runtime::list_git_branches(root, 200, &current_branch);
    if branches.is_empty() {
        branches.push(if main_branch.is_empty() {
            "main".to_string()
        } else {
            main_branch.clone()
        });
    }
    for branch in counts.keys() {
        if !branches.iter().any(|row| row == branch) {
            branches.push(branch.clone());
        }
    }
    branches.sort();
    let options = branches
        .iter()
        .map(|branch| {
            let kind = if branch == "main" || branch == "master" {
                "master"
            } else {
                "isolated"
            };
            let workspace = if branch == "main" || branch == "master" {
                root.to_path_buf()
            } else {
                crate::dashboard_git_runtime::workspace_for_agent_branch(root, agent_id, branch)
            };
            let ready = crate::dashboard_git_runtime::git_workspace_ready(root, &workspace);
            json!({
                "branch": branch,
                "current": *branch == current_branch,
                "main": *branch == "main" || *branch == "master",
                "kind": kind,
                "in_use_by_agents": counts.get(branch).copied().unwrap_or(0),
                "workspace_dir": workspace.to_string_lossy().to_string(),
                "workspace_rel": crate::dashboard_git_runtime::workspace_rel(root, &workspace),
                "git_tree_ready": if kind == "master" { true } else { ready },
                "git_tree_error": if kind == "master" || ready { "" } else { "git_worktree_missing" }
            })
        })
        .collect::<Vec<_>>();
    json!({
        "ok": true,
        "current": {
            "git_branch": if current_branch.is_empty() { "main" } else { &current_branch },
            "git_tree_kind": if current_branch == "main" || current_branch == "master" { "master" } else { "isolated" },
            "workspace_dir": if current_workspace.is_empty() { root.to_string_lossy().to_string() } else { current_workspace },
            "workspace_rel": current_workspace_rel,
            "git_tree_ready": current.get("git_tree_ready").cloned().unwrap_or_else(|| json!(true)),
            "git_tree_error": current.get("git_tree_error").cloned().unwrap_or_else(|| json!(""))
        },
        "options": options
    })
}

fn tool_name_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | ':')
}

fn normalize_tool_name(raw: &str) -> String {
    clean_text(raw, 80).to_ascii_lowercase().replace('-', "_")
}
