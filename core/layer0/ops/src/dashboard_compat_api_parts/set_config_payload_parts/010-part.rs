fn set_config_payload(root: &Path, snapshot: &Value, request: &Value) -> Value {
    let path =
        clean_text(request.get("path").and_then(Value::as_str).unwrap_or(""), 120).to_ascii_lowercase();
    let string_value = clean_text(
        request
            .get("value")
            .and_then(|value| {
                value.as_str().map(|row| row.to_string()).or_else(|| {
                    if value.is_null() {
                        None
                    } else {
                        Some(value.to_string())
                    }
                })
            })
            .unwrap_or_default()
            .trim_matches('"'),
        4000,
    );
    if path.is_empty() {
        return json!({"ok": false, "error": "config_path_required"});
    }
    let field = path.rsplit('.').next().unwrap_or(path.as_str());
    let (current_provider, current_model) = extract_app_settings(root, snapshot);
    match field {
        "provider" => {
            let provider = if string_value.is_empty() {
                "auto".to_string()
            } else {
                string_value
            };
            let saved = save_app_settings(root, &provider, &current_model);
            json!({"ok": true, "path": path, "value": provider, "settings": saved})
        }
        "model" => {
            let saved = save_app_settings(root, &current_provider, &string_value);
            json!({"ok": true, "path": path, "value": string_value, "settings": saved})
        }
        "api_key" => crate::dashboard_provider_runtime::save_provider_key(
            root,
            &current_provider,
            &string_value,
        ),
        _ => {
            json!({"ok": true, "path": path, "value": request.get("value").cloned().unwrap_or(Value::Null)})
        }
    }
}

fn extract_profiles(root: &Path) -> Vec<Value> {
    let state = read_json(&state_path(root, AGENT_PROFILES_REL)).unwrap_or_else(|| json!({}));
    let mut rows = state
        .get("agents")
        .and_then(Value::as_object)
        .map(|obj| obj.values().map(|v| v.clone()).collect::<Vec<Value>>())
        .unwrap_or_default();
    rows.sort_by(|a, b| {
        clean_text(a.get("agent_id").and_then(Value::as_str).unwrap_or(""), 120)
            .cmp(&clean_text(b.get("agent_id").and_then(Value::as_str).unwrap_or(""), 120))
    });
    rows
}

fn recent_audit_entries(root: &Path, snapshot: &Value) -> Vec<Value> {
    let from_snapshot = snapshot
        .pointer("/receipts/recent")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if !from_snapshot.is_empty() {
        return from_snapshot;
    }
    let raw = fs::read_to_string(state_path(root, ACTION_HISTORY_REL)).unwrap_or_default();
    raw.lines()
        .rev()
        .take(200)
        .filter_map(|row| serde_json::from_str::<Value>(row).ok())
        .collect::<Vec<_>>()
}

fn clean_agent_id(raw: &str) -> String {
    let mut out = String::new();
    for ch in clean_text(raw, 140).chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
            out.push(ch);
        }
    }
    out
}

fn tooling_pipeline_display_meta(tool_name: &str, tool_args: &Value) -> Value {
    let normalized = normalize_tool_name(tool_name);
    let primary_subject = match normalized.as_str() {
        "batch_query" | "web_search" | "search_web" | "search" | "web_query" => clean_text(
            tool_args
                .get("query")
                .or_else(|| tool_args.get("q"))
                .and_then(Value::as_str)
                .unwrap_or(""),
            240,
        ),
        "web_tooling_health_probe" => clean_text(
            tool_args
                .get("query")
                .and_then(Value::as_str)
                .or_else(|| {
                    tool_args
                        .get("queries")
                        .and_then(Value::as_array)
                        .and_then(|rows| rows.first())
                        .and_then(Value::as_str)
                })
                .filter(|raw| !raw.is_empty())
                .unwrap_or("web tooling health probe"),
            240,
        ),
        "web_fetch" | "browse" | "web_conduit_fetch" => clean_text(
            tool_args
                .get("url")
                .or_else(|| tool_args.get("link"))
                .and_then(Value::as_str)
                .unwrap_or(""),
            240,
        ),
        "file_read" | "file_read_many" | "folder_export" | "workspace_analyze" => clean_text(
            tool_args
                .get("path")
                .or_else(|| tool_args.get("query"))
                .and_then(Value::as_str)
                .unwrap_or(""),
            240,
        ),
        "spawn_subagents" => clean_text(
            tool_args
                .get("objective")
                .or_else(|| tool_args.get("task"))
                .and_then(Value::as_str)
                .unwrap_or(""),
            240,
        ),
        "terminal_exec" => clean_text(
            tool_args
                .get("command")
                .or_else(|| tool_args.get("cmd"))
                .and_then(Value::as_str)
                .unwrap_or(""),
            240,
        ),
        _ => String::new(),
    };
    let summary = if primary_subject.is_empty() {
        normalized.clone()
    } else {
        format!("{normalized}: {}", trim_text(&primary_subject, 180))
    };
    json!({
        "tool": normalized,
        "primary_subject": primary_subject,
        "summary": clean_text(&summary, 260)
    })
}

fn tooling_pipeline_execute<F>(
    trace_id: &str,
    task_id: &str,
    tool_name: &str,
    tool_args: &Value,
    executor: F,
) -> Value
where
    F: FnOnce(&Value) -> Result<Value, String>,
{
    let mut broker = crate::infring_tooling_core_v1_bridge::ToolBroker::default();
    let _ = broker.recover_from_ledger();
    let extractor = crate::infring_tooling_core_v1_bridge::EvidenceExtractor;
    let mut store = crate::infring_tooling_core_v1_bridge::EvidenceStore::default();
    let _ = store.recover_from_ledger();
    let verifier = crate::infring_tooling_core_v1_bridge::StructuredVerifier;
    let tool_name_clean = clean_text(tool_name, 80);
    let capability_probe = broker.capability_probe(
        crate::infring_tooling_core_v1_bridge::BrokerCaller::Client,
        tool_name_clean.as_str(),
    );
    let request = crate::infring_tooling_core_v1_bridge::ToolCallRequest {
        trace_id: clean_text(trace_id, 160),
        task_id: clean_text(task_id, 160),
        tool_name: tool_name_clean.clone(),
        args: tool_args.clone(),
        lineage: vec!["dashboard_compat_api".to_string()],
        caller: crate::infring_tooling_core_v1_bridge::BrokerCaller::Client,
        policy_revision: Some("policy.tooling.dashboard_compat_api.v1".to_string()),
        tool_version: Some(format!("{tool_name_clean}.v1")),
        freshness_window_ms: None,
        force_no_dedupe: false,
    };
    let attempt = broker.execute_and_envelope(request, executor);
    let attempt_receipt = attempt.attempt.clone();
    let Some(normalized_result) = attempt.normalized_result.clone() else {
        return json!({
            "ok": false,
            "error": attempt.error.clone().unwrap_or_else(|| attempt_receipt.reason.clone()),
            "tool_name": tool_name_clean,
            "task_id": clean_text(task_id, 160),
            "trace_id": clean_text(trace_id, 160),
            "tool_capability_probe": capability_probe,
            "tool_attempt": attempt,
            "tool_attempt_receipt": attempt_receipt
        });
    };
    let raw_payload = attempt.raw_payload.clone().unwrap_or(Value::Null);
    let cards = extractor.extract(&normalized_result, &raw_payload);
    let evidence_ids = store.append_evidence(&cards);
    let bundle = verifier.derive_claim_bundle(task_id, &cards);
    let claim_ref_validation = verifier.validate_claim_evidence_refs(&bundle, &cards).err();
    let synthesis_claims = verifier
        .supported_claims_for_synthesis(&bundle)
        .iter()
        .map(|claim| (*claim).clone())
        .collect::<Vec<_>>();
    let status = if evidence_ids.is_empty() || claim_ref_validation.is_some() {
        crate::infring_tooling_core_v1_bridge::WorkerTaskStatus::Blocked
    } else {
        crate::infring_tooling_core_v1_bridge::WorkerTaskStatus::Completed
    };
    let worker_output = crate::infring_tooling_core_v1_bridge::WorkerOutput {
        task_id: clean_text(task_id, 160),
        status,
        produced_evidence_ids: evidence_ids.clone(),
        open_questions: if evidence_ids.is_empty() {
            vec!["No evidence cards were extracted from this tool result.".to_string()]
        } else {
            Vec::new()
        },
        recommended_next_actions: if evidence_ids.is_empty() {
            vec!["Retry with narrower query/path and rerun through the broker.".to_string()]
        } else {
            Vec::new()
        },
        blockers: if normalized_result.errors.is_empty() {
            claim_ref_validation.into_iter().collect::<Vec<_>>()
        } else {
            let mut rows = normalized_result.errors.clone();
            rows.extend(claim_ref_validation);
            rows
        },
        budget_used: crate::infring_tooling_core_v1_bridge::WorkerBudgetUsed {
            tool_calls: 1,
            input_tokens: (clean_text(&tool_args.to_string(), 8000).len() / 4).max(1),
            output_tokens: (clean_text(&raw_payload.to_string(), 8000).len() / 4).max(1),
        },
    };
    json!({
        "ok": true,
        "schema_contract": crate::infring_tooling_core_v1_bridge::published_schema_contract_v1(),
        "tool_display_meta": tooling_pipeline_display_meta(&tool_name_clean, tool_args),
        "tool_capability_probe": capability_probe,
        "tool_attempt": attempt,
        "tool_attempt_receipt": attempt_receipt,
        "raw_payload": raw_payload,
        "normalized_result": normalized_result,
        "evidence_cards": cards,
        "evidence_store_records": store.records(),
        "worker_output": worker_output,
        "claim_bundle": bundle,
        "synthesis_input": {
            "claims": synthesis_claims
        }
    })
}

fn attach_tool_pipeline(payload: &mut Value, pipeline: &Value) {
    if let Some(obj) = payload.as_object_mut() {
        obj.insert("tool_pipeline".to_string(), pipeline.clone());
    }
}

fn tool_pipeline_supported_tool(tool_name: &str) -> bool {
    matches!(normalize_tool_name(tool_name).as_str(), "web_search" | "web_fetch" | "batch_query" | "web_tooling_health_probe" | "file_read" | "file_read_many" | "folder_export" | "manage_agent" | "spawn_subagents" | "terminal_exec" | "workspace_analyze")
}

fn parse_json_loose(raw: &str) -> Option<Value> {
    if raw.trim().is_empty() {
        return None;
    }
    if let Ok(value) = serde_json::from_str::<Value>(raw) {
        return Some(value);
    }
    for line in raw.lines().rev() {
        let candidate = line.trim();
        if candidate.is_empty() {
            continue;
        }
        if let Ok(value) = serde_json::from_str::<Value>(candidate) {
            return Some(value);
        }
    }
    None
}

fn read_json_loose(path: &Path) -> Option<Value> {
    let raw = fs::read_to_string(path).ok()?;
    parse_json_loose(&raw)
}

fn write_json_pretty(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(raw) = serde_json::to_string_pretty(value) {
        let _ = fs::write(path, format!("{raw}\n"));
    }
}

fn read_jsonl_loose(path: &Path, max_rows: usize) -> Vec<Value> {
    let raw = fs::read_to_string(path).unwrap_or_default();
    let limit = max_rows.max(1);
    raw.lines()
        .rev()
        .take(limit)
        .filter_map(|line| serde_json::from_str::<Value>(line.trim()).ok())
        .collect::<Vec<_>>()
}

fn instinct_dir(root: &Path) -> PathBuf {
    state_path(root, AGENT_INSTINCT_DIR_REL)
}

fn agent_instinct_prompt_context(root: &Path, max_chars: usize) -> String {
    let dir = instinct_dir(root);
    if !dir.is_dir() {
        return String::new();
    }
    let mut files = fs::read_dir(&dir)
        .ok()
        .into_iter()
        .flat_map(|entries| entries.flatten())
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .filter(|path| {
            path.extension()
                .and_then(|value| value.to_str())
                .map(|value| {
                    let lowered = value.to_ascii_lowercase();
                    lowered == "md" || lowered == "txt"
                })
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();
    files.sort();
    let mut chunks = Vec::<String>::new();
    let mut used = 0usize;
    for path in files.into_iter().take(12) {
        let file_name = clean_text(
            path.file_name()
                .and_then(|value| value.to_str())
                .unwrap_or(""),
            120,
        );
        if file_name.is_empty() {
            continue;
        }
        let content = fs::read_to_string(&path).unwrap_or_default();
        let cleaned = clean_text(&content, max_chars.saturating_sub(used));
        if cleaned.is_empty() {
            continue;
        }
        let block = format!("[instinct:{file_name}] {cleaned}");
        used = used.saturating_add(block.len());
        chunks.push(block);
        if used >= max_chars {
            break;
        }
    }
    clean_text(&chunks.join("\n"), max_chars)
}

fn requester_agent_id(headers: &[(&str, &str)]) -> String {
    let primary = header_value(headers, "X-Actor-Agent-Id").or_else(|| header_value(headers, "X-Agent-Id")).or_else(|| header_value(headers, "X-Requester-Agent-Id")).unwrap_or_default();
    clean_agent_id(&primary)
}

fn parent_agent_id_from_row(row: &Value) -> String {
    clean_agent_id(
        row.get("parent_agent_id")
            .and_then(Value::as_str)
            .or_else(|| row.pointer("/contract/parent_agent_id").and_then(Value::as_str))
            .unwrap_or(""),
    )
}

fn parse_rfc3339_utc(raw: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(raw)
        .ok()
        .map(|value| value.with_timezone(&Utc))
}

fn latest_timestamp(values: &[String]) -> String {
    let mut best = String::new();
    for value in values {
        if value.is_empty() {
            continue;
        }
        if best.is_empty() || value > &best {
            best = value.clone();
        }
    }
    best
}

fn message_text(row: &Value) -> String {
    if let Some(text) = row.get("text").and_then(Value::as_str) {
        return clean_chat_text(text, 64_000);
    }
    if let Some(text) = row.get("content").and_then(Value::as_str) {
        return clean_chat_text(text, 64_000);
    }
    if let Some(text) = row.as_str() {
        return clean_chat_text(text, 64_000);
    }
    String::new()
}

fn message_timestamp_iso(row: &Value) -> String {
    if let Some(ts) = row.get("ts").and_then(Value::as_str) {
        return clean_text(ts, 80);
    }
    if let Some(ts_ms) = row.get("ts").and_then(Value::as_i64) {
        if let Some(parsed) = DateTime::<Utc>::from_timestamp_millis(ts_ms) {
            return parsed.to_rfc3339();
        }
    }
    clean_text(row.get("created_at").and_then(Value::as_str).unwrap_or(""), 80)
}

fn humanize_agent_name(agent_id: &str) -> String {
    let cleaned = clean_agent_id(agent_id).replace('-', " ").replace('_', " ");
    let mut words = Vec::<String>::new();
    for word in cleaned.split_whitespace() {
        let mut chars = word.chars();
        if let Some(first) = chars.next() {
            let mut built = String::new();
            built.push(first.to_ascii_uppercase());
            built.push_str(chars.as_str());
            words.push(built);
        }
    }
    if words.is_empty() {
        "Agent".to_string()
    } else {
        words.join(" ")
    }
}

fn profiles_map(root: &Path) -> Map<String, Value> {
    read_json_loose(&state_path(root, AGENT_PROFILES_REL))
        .and_then(|v| v.get("agents").and_then(Value::as_object).cloned())
        .unwrap_or_default()
}

fn contracts_map(root: &Path) -> Map<String, Value> {
    read_json_loose(&state_path(root, AGENT_CONTRACTS_REL))
        .and_then(|v| v.get("contracts").and_then(Value::as_object).cloned())
        .unwrap_or_default()
}

fn session_dir(root: &Path) -> PathBuf {
    state_path(root, AGENT_SESSIONS_DIR_REL)
}

fn session_path(root: &Path, agent_id: &str) -> PathBuf {
    session_dir(root).join(format!("{}.json", clean_agent_id(agent_id)))
}

fn agent_files_dir(root: &Path, agent_id: &str) -> PathBuf {
    state_path(root, AGENT_FILES_DIR_REL).join(clean_agent_id(agent_id))
}

fn agent_tools_path(root: &Path, agent_id: &str) -> PathBuf {
    state_path(root, AGENT_TOOLS_DIR_REL).join(format!("{}.json", clean_agent_id(agent_id)))
}

fn default_session_state(agent_id: &str) -> Value {
    let now = crate::now_iso();
    json!({
        "type": "infring_dashboard_agent_session",
        "agent_id": clean_agent_id(agent_id),
        "active_session_id": "default",
        "sessions": [{"session_id": "default", "label": "Session", "created_at": now, "updated_at": now, "messages": []}],
        "memory_kv": {}
    })
}
