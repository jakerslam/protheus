fn normalize_session_state(agent_id: &str, mut state: Value) -> Value {
    let id = clean_agent_id(agent_id);
    if !state.is_object() {
        state = default_session_state(&id);
    }
    state["agent_id"] = Value::String(id);
    if !state
        .get("active_session_id")
        .and_then(Value::as_str)
        .map(|v| !v.trim().is_empty())
        .unwrap_or(false)
    {
        state["active_session_id"] = Value::String("default".to_string());
    }
    if !state.get("sessions").map(Value::is_array).unwrap_or(false) {
        state["sessions"] = Value::Array(Vec::new());
    }
    if state
        .get("sessions")
        .and_then(Value::as_array)
        .map(|rows| rows.is_empty())
        .unwrap_or(true)
    {
        state["sessions"] = Value::Array(vec![json!({
            "session_id": "default",
            "label": "Session",
            "created_at": crate::now_iso(),
            "updated_at": crate::now_iso(),
            "messages": []
        })]);
    }
    if !state
        .get("memory_kv")
        .map(Value::is_object)
        .unwrap_or(false)
    {
        state["memory_kv"] = json!({});
    }
    state
}

fn load_session_state(root: &Path, agent_id: &str) -> Value {
    let path = session_path(root, agent_id);
    let state = read_json_loose(&path).unwrap_or_else(|| default_session_state(agent_id));
    normalize_session_state(agent_id, state)
}

fn save_session_state(root: &Path, agent_id: &str, state: &Value) {
    let path = session_path(root, agent_id);
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let bounded_state = bound_session_state_for_persistence(root, agent_id, state);
    write_json_pretty(&path, &bounded_state);
    write_indexed_session_state_incremental(root, agent_id, &bounded_state);
}

use std::io::{Read, Seek, SeekFrom, Write};

const SESSION_MESSAGE_TEXT_MAX_CHARS: usize = 64_000;
const SESSION_MESSAGE_PREVIEW_MAX_CHARS: usize = 4_000;
const SESSION_TOOL_PREVIEW_MAX_CHARS: usize = 1_200;
const SESSION_TERMINAL_PREVIEW_MAX_CHARS: usize = 1_000;
const SESSION_INDEX_DIR_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/agent_sessions_indexed";
const SESSION_FORBIDDEN_PROJECTION_KEYS: [&str; 9] = [
    "raw",
    "root",
    "trace_body",
    "decision_trace",
    "workflow_graph",
    "execution_observation",
    "response_workflow",
    "response_finalization",
    "process_summary",
];

fn session_artifact_dir(root: &Path, agent_id: &str) -> PathBuf {
    state_path(root, "client/runtime/local/state/ui/infring_dashboard/session_artifacts")
        .join(clean_agent_id(agent_id))
}

fn clean_session_id(raw: &str) -> String {
    let cleaned = clean_text(raw, 120);
    let safe = cleaned
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>();
    if safe.is_empty() {
        "default".to_string()
    } else {
        safe
    }
}

fn session_index_agent_dir(root: &Path, agent_id: &str) -> PathBuf {
    state_path(root, SESSION_INDEX_DIR_REL).join(clean_agent_id(agent_id))
}

fn session_index_meta_path(root: &Path, agent_id: &str) -> PathBuf {
    session_index_agent_dir(root, agent_id).join("meta.json")
}

fn session_index_session_dir(root: &Path, agent_id: &str, session_id: &str) -> PathBuf {
    session_index_agent_dir(root, agent_id).join(clean_session_id(session_id))
}

fn session_index_messages_path(root: &Path, agent_id: &str, session_id: &str) -> PathBuf {
    session_index_session_dir(root, agent_id, session_id).join("messages.jsonl")
}

fn session_index_offsets_path(root: &Path, agent_id: &str, session_id: &str) -> PathBuf {
    session_index_session_dir(root, agent_id, session_id).join("offsets.json")
}

fn write_indexed_session_messages(path: &Path, offsets_path: &Path, messages: &[Value]) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let Ok(mut file) = fs::File::create(path) else {
        return;
    };
    let mut offsets = Vec::<u64>::new();
    let mut cursor = 0u64;
    for message in messages {
        let Ok(mut line) = serde_json::to_string(message) else {
            continue;
        };
        line.push('\n');
        offsets.push(cursor);
        let bytes = line.as_bytes();
        if file.write_all(bytes).is_err() {
            return;
        }
        cursor = cursor.saturating_add(bytes.len() as u64);
    }
    write_json_pretty(offsets_path, &json!(offsets));
}

fn indexed_session_message_count(meta: &Value, session_id: &str) -> Option<usize> {
    let sid = clean_session_id(session_id);
    meta.get("sessions")
        .and_then(Value::as_array)
        .and_then(|rows| {
            rows.iter().find_map(|row| {
                let row_id = clean_session_id(row.get("session_id").and_then(Value::as_str).unwrap_or(""));
                if row_id == sid {
                    row.get("message_count")
                        .and_then(Value::as_u64)
                        .map(|count| count as usize)
                } else {
                    None
                }
            })
        })
}

fn indexed_session_meta_row(session: &Value, session_id: &str, message_count: usize) -> Value {
    json!({
        "session_id": clean_session_id(session_id),
        "label": clean_text(session.get("label").and_then(Value::as_str).unwrap_or("Session"), 80),
        "created_at": session.get("created_at").cloned().unwrap_or(Value::Null),
        "updated_at": session.get("updated_at").cloned().unwrap_or(Value::Null),
        "message_count": message_count
    })
}

fn append_indexed_session_messages(
    root: &Path,
    agent_id: &str,
    session_id: &str,
    messages: &[Value],
    previous_count: usize,
) -> bool {
    if previous_count >= messages.len() {
        return false;
    }
    let path = session_index_messages_path(root, agent_id, session_id);
    let offsets_path = session_index_offsets_path(root, agent_id, session_id);
    let Some(mut offsets) = read_json_loose(&offsets_path).and_then(|value| {
        value
            .as_array()
            .map(|rows| rows.iter().filter_map(Value::as_u64).collect::<Vec<_>>())
    }) else {
        return false;
    };
    if offsets.len() != previous_count || !path.exists() {
        return false;
    }
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(&path) else {
        return false;
    };
    let mut cursor = fs::metadata(&path).map(|meta| meta.len()).unwrap_or(0);
    for message in messages.iter().skip(previous_count) {
        let Ok(mut line) = serde_json::to_string(message) else {
            return false;
        };
        line.push('\n');
        offsets.push(cursor);
        let bytes = line.as_bytes();
        if file.write_all(bytes).is_err() {
            return false;
        }
        cursor = cursor.saturating_add(bytes.len() as u64);
    }
    write_json_pretty(&offsets_path, &json!(offsets));
    true
}

fn indexed_session_meta_payload(agent_id: &str, state: &Value, session_meta: Vec<Value>) -> Value {
    json!({
        "type": "infring_dashboard_agent_session_index_v1",
        "agent_id": clean_agent_id(agent_id),
        "active_session_id": clean_session_id(state.get("active_session_id").and_then(Value::as_str).unwrap_or("default")),
        "sessions": session_meta,
        "source": "bounded_session_projection"
    })
}

fn write_indexed_session_meta(root: &Path, agent_id: &str, meta: &Value) {
    let path = session_index_meta_path(root, agent_id);
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    write_json_pretty(&path, meta);
}

fn write_indexed_session_state(root: &Path, agent_id: &str, state: &Value) {
    let id = clean_agent_id(agent_id);
    if id.is_empty() {
        return;
    }
    let sessions = state
        .get("sessions")
        .and_then(Value::as_array)
        .map(|rows| rows.as_slice())
        .unwrap_or(&[]);
    let mut session_meta = Vec::<Value>::new();
    for session in sessions {
        let sid = clean_session_id(
            session
                .get("session_id")
                .and_then(Value::as_str)
                .unwrap_or("default"),
        );
        let messages = session
            .get("messages")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        write_indexed_session_messages(
            &session_index_messages_path(root, &id, &sid),
            &session_index_offsets_path(root, &id, &sid),
            &messages,
        );
        session_meta.push(indexed_session_meta_row(session, &sid, messages.len()));
    }
    let meta = indexed_session_meta_payload(&id, state, session_meta);
    write_indexed_session_meta(root, &id, &meta);
}

fn write_indexed_session_state_incremental(root: &Path, agent_id: &str, state: &Value) {
    let id = clean_agent_id(agent_id);
    if id.is_empty() {
        return;
    }
    let Some(existing_meta) = load_session_index_meta(root, &id) else {
        write_indexed_session_state(root, &id, state);
        return;
    };
    let active_session_id = clean_session_id(
        state
            .get("active_session_id")
            .and_then(Value::as_str)
            .unwrap_or("default"),
    );
    let sessions = state
        .get("sessions")
        .and_then(Value::as_array)
        .map(|rows| rows.as_slice())
        .unwrap_or(&[]);
    let mut session_meta = Vec::<Value>::new();
    for session in sessions {
        let sid = clean_session_id(
            session
                .get("session_id")
                .and_then(Value::as_str)
                .unwrap_or("default"),
        );
        let messages = session
            .get("messages")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let previous_count = indexed_session_message_count(&existing_meta, &sid);
        let should_full_rewrite = match previous_count {
            Some(count) if count < messages.len() => {
                !append_indexed_session_messages(root, &id, &sid, &messages, count)
            }
            Some(count) if count == messages.len() => {
                let messages_path = session_index_messages_path(root, &id, &sid);
                sid == active_session_id || !messages_path.exists()
            }
            Some(_) => true,
            None => true,
        };
        if should_full_rewrite {
            write_indexed_session_messages(
                &session_index_messages_path(root, &id, &sid),
                &session_index_offsets_path(root, &id, &sid),
                &messages,
            );
        }
        session_meta.push(indexed_session_meta_row(session, &sid, messages.len()));
    }
    let meta = indexed_session_meta_payload(&id, state, session_meta);
    write_indexed_session_meta(root, &id, &meta);
}

fn rebuild_indexed_session_state(root: &Path, agent_id: &str) -> Value {
    let id = clean_agent_id(agent_id);
    if id.is_empty() {
        return json!({"ok": false, "error": "agent_id_required"});
    }
    let legacy_path = session_path(root, &id);
    let legacy_size_bytes = fs::metadata(&legacy_path).map(|meta| meta.len()).unwrap_or(0);
    let Some(legacy_state) = read_json_loose(&legacy_path) else {
        return json!({
            "ok": false,
            "error": "legacy_session_unreadable",
            "agent_id": id,
            "legacy_path": legacy_path.to_string_lossy(),
            "legacy_size_bytes": legacy_size_bytes
        });
    };
    let bounded_state = bound_session_state_for_persistence(root, &id, &legacy_state);
    write_indexed_session_state(root, &id, &bounded_state);
    let meta = load_session_index_meta(root, &id).unwrap_or_else(|| json!({}));
    let sessions = indexed_session_rows_payload(&meta);
    let message_count = sessions
        .iter()
        .filter_map(|row| row.get("message_count").and_then(Value::as_u64))
        .sum::<u64>();
    json!({
        "ok": true,
        "type": "dashboard_agent_session_index_rebuild",
        "agent_id": id,
        "active_session_id": indexed_active_session_id(&meta),
        "session_count": sessions.len(),
        "message_count": message_count,
        "storage_source": "indexed_session_jsonl",
        "legacy_path": legacy_path.to_string_lossy(),
        "legacy_size_bytes": legacy_size_bytes,
        "index_dir": session_index_agent_dir(root, &id).to_string_lossy(),
        "meta_path": session_index_meta_path(root, &id).to_string_lossy(),
        "receipt_ref": format!("agent_session_index_rebuild:{id}:{}", crate::now_iso()),
        "correlation_id": format!("agent_session_index_rebuild:{id}")
    })
}

fn legacy_session_agent_file_rows(root: &Path) -> Vec<(String, PathBuf, u64)> {
    let mut rows = Vec::<(String, PathBuf, u64)>::new();
    let Ok(entries) = fs::read_dir(session_dir(root)) else {
        return rows;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        let agent_id = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .map(clean_agent_id)
            .unwrap_or_default();
        if agent_id.is_empty() {
            continue;
        }
        let size = entry.metadata().map(|meta| meta.len()).unwrap_or(0);
        rows.push((agent_id, path, size));
    }
    rows.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| a.0.cmp(&b.0)));
    rows
}

fn requested_session_index_agents(root: &Path, request: &Value) -> Vec<(String, PathBuf, u64)> {
    let all_rows = legacy_session_agent_file_rows(root);
    let requested = request
        .get("agent_ids")
        .or_else(|| request.get("agents"))
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(clean_agent_id)
                .filter(|id| !id.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if requested.is_empty() {
        return all_rows;
    }
    let requested_set = requested.into_iter().collect::<std::collections::BTreeSet<_>>();
    all_rows
        .into_iter()
        .filter(|(agent_id, _, _)| requested_set.contains(agent_id))
        .collect::<Vec<_>>()
}

fn rebuild_indexed_session_states(root: &Path, request: &Value) -> Value {
    let force = request.get("force").and_then(Value::as_bool).unwrap_or(false);
    let limit = request
        .get("limit")
        .and_then(Value::as_u64)
        .map(|value| value as usize)
        .unwrap_or(0);
    let min_legacy_size_bytes = request
        .get("min_legacy_size_bytes")
        .or_else(|| request.get("minLegacySizeBytes"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let candidates = requested_session_index_agents(root, request);
    let mut rows = Vec::<Value>::new();
    let mut rebuilt = 0usize;
    let mut skipped = 0usize;
    let mut failed = 0usize;
    let mut message_count = 0u64;
    let mut legacy_size_bytes = 0u64;
    for (agent_id, path, size) in candidates.iter() {
        if min_legacy_size_bytes > 0 && *size < min_legacy_size_bytes {
            skipped += 1;
            rows.push(json!({
                "ok": true,
                "agent_id": agent_id,
                "action": "skipped_below_min_legacy_size",
                "legacy_path": path.to_string_lossy(),
                "legacy_size_bytes": size
            }));
            continue;
        }
        if limit > 0 && rebuilt >= limit {
            skipped += 1;
            rows.push(json!({
                "ok": true,
                "agent_id": agent_id,
                "action": "skipped_limit_reached",
                "legacy_path": path.to_string_lossy(),
                "legacy_size_bytes": size
            }));
            continue;
        }
        if !force && load_session_index_meta(root, agent_id).is_some() {
            skipped += 1;
            rows.push(json!({
                "ok": true,
                "agent_id": agent_id,
                "action": "skipped_index_exists",
                "legacy_path": path.to_string_lossy(),
                "legacy_size_bytes": size,
                "meta_path": session_index_meta_path(root, agent_id).to_string_lossy()
            }));
            continue;
        }
        let row = rebuild_indexed_session_state(root, agent_id);
        let ok = row.get("ok").and_then(Value::as_bool).unwrap_or(false);
        if ok {
            rebuilt += 1;
            message_count = message_count
                .saturating_add(row.get("message_count").and_then(Value::as_u64).unwrap_or(0));
            legacy_size_bytes = legacy_size_bytes.saturating_add(*size);
        } else {
            failed += 1;
        }
        rows.push(row);
    }
    json!({
        "ok": failed == 0,
        "type": "dashboard_agent_session_index_batch_rebuild",
        "force": force,
        "candidate_count": candidates.len(),
        "rebuilt_count": rebuilt,
        "skipped_count": skipped,
        "failed_count": failed,
        "message_count": message_count,
        "legacy_size_bytes": legacy_size_bytes,
        "storage_source": "indexed_session_jsonl",
        "rows": rows,
        "receipt_ref": format!("agent_session_index_batch_rebuild:{}", crate::now_iso()),
        "correlation_id": "agent_session_index_batch_rebuild"
    })
}

fn load_session_index_meta(root: &Path, agent_id: &str) -> Option<Value> {
    read_json_loose(&session_index_meta_path(root, agent_id))
}

fn indexed_active_session_id(meta: &Value) -> String {
    let active = clean_session_id(
        meta.get("active_session_id")
            .and_then(Value::as_str)
            .unwrap_or("default"),
    );
    let sessions = meta
        .get("sessions")
        .and_then(Value::as_array)
        .map(|rows| rows.as_slice())
        .unwrap_or(&[]);
    if sessions.iter().any(|row| {
        row.get("session_id")
            .and_then(Value::as_str)
            .map(|sid| clean_session_id(sid) == active)
            .unwrap_or(false)
    }) {
        active
    } else {
        sessions
            .first()
            .and_then(|row| row.get("session_id"))
            .and_then(Value::as_str)
            .map(clean_session_id)
            .unwrap_or_else(|| "default".to_string())
    }
}

fn indexed_session_rows_payload(meta: &Value) -> Vec<Value> {
    let active = indexed_active_session_id(meta);
    meta.get("sessions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|row| {
            let sid = clean_session_id(row.get("session_id").and_then(Value::as_str).unwrap_or(""));
            json!({
                "session_id": sid,
                "label": clean_text(row.get("label").and_then(Value::as_str).unwrap_or("Session"), 80),
                "created_at": row.get("created_at").cloned().unwrap_or(Value::Null),
                "updated_at": row.get("updated_at").cloned().unwrap_or(Value::Null),
                "message_count": row.get("message_count").and_then(Value::as_u64).unwrap_or(0),
                "active": sid == active
            })
        })
        .collect::<Vec<_>>()
}

fn read_indexed_session_window(
    root: &Path,
    agent_id: &str,
    session_id: &str,
    limit: usize,
    offset: usize,
) -> Option<(Vec<Value>, usize)> {
    let offsets = read_json_loose(&session_index_offsets_path(root, agent_id, session_id))?
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|value| value.as_u64())
        .collect::<Vec<_>>();
    let total = offsets.len();
    if total == 0 {
        return Some((Vec::new(), 0));
    }
    let bounded_limit = if limit == 0 { total } else { limit.min(500) };
    let end = total.saturating_sub(offset);
    let start = end.saturating_sub(bounded_limit);
    let start_byte = *offsets.get(start)?;
    let path = session_index_messages_path(root, agent_id, session_id);
    let mut file = fs::File::open(&path).ok()?;
    let end_byte = if end < total {
        *offsets.get(end)?
    } else {
        file.metadata().ok()?.len()
    };
    if end_byte < start_byte {
        return None;
    }
    let byte_len = (end_byte - start_byte) as usize;
    let mut raw = vec![0u8; byte_len];
    file.seek(SeekFrom::Start(start_byte)).ok()?;
    file.read_exact(&mut raw).ok()?;
    let text = String::from_utf8_lossy(&raw);
    let messages = text
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .collect::<Vec<_>>();
    Some((messages, total))
}

fn persist_session_artifact_ref(root: &Path, agent_id: &str, kind: &str, value: &Value) -> Value {
    if value.is_null() {
        return Value::Null;
    }
    let id = clean_agent_id(agent_id);
    let artifact_kind = clean_text(kind, 80)
        .replace('/', "_")
        .replace('\\', "_");
    if id.is_empty() || artifact_kind.is_empty() {
        return Value::Null;
    }
    let hash = crate::deterministic_receipt_hash(value);
    let dir = session_artifact_dir(root, &id);
    let _ = fs::create_dir_all(&dir);
    let path = dir.join(format!("{artifact_kind}-{hash}.json"));
    if !path.exists() {
        write_json_pretty(&path, value);
    }
    json!({
        "kind": artifact_kind,
        "ref": format!("session_artifact:{id}:{artifact_kind}:{hash}"),
        "sha256": hash,
        "bytes": value.to_string().len()
    })
}

fn compact_session_tool_rows(root: &Path, agent_id: &str, tools: &Value) -> Value {
    let rows = tools.as_array().cloned().unwrap_or_default();
    Value::Array(rows.into_iter().take(24).enumerate().map(|(idx, tool)| {
        let existing_ref = tool.get("detail_ref").and_then(Value::as_str).unwrap_or("");
        let detail_ref = if existing_ref.is_empty() {
            persist_session_artifact_ref(root, agent_id, &format!("tool_{idx}"), &tool)
        } else {
            json!({"ref": existing_ref})
        };
        let name = clean_text(tool.get("name").or_else(|| tool.get("tool")).and_then(Value::as_str).unwrap_or("tool"), 120);
        let status = clean_text(tool.get("status").and_then(Value::as_str).unwrap_or(""), 80);
        let is_error = tool.get("is_error").and_then(Value::as_bool).unwrap_or(false);
        let blocked = tool.get("blocked").and_then(Value::as_bool).unwrap_or(false);
        let result_preview = clean_text(
            tool.get("summary")
                .or_else(|| tool.get("result"))
                .or_else(|| tool.get("output"))
                .map(Value::to_string)
                .as_deref()
                .unwrap_or(""),
            SESSION_TOOL_PREVIEW_MAX_CHARS,
        );
        let input_preview = clean_text(
            tool.get("input")
                .or_else(|| tool.get("arguments"))
                .or_else(|| tool.get("payload"))
                .map(Value::to_string)
                .as_deref()
                .unwrap_or(""),
            SESSION_TOOL_PREVIEW_MAX_CHARS,
        );
        json!({
            "name": name,
            "tool": name,
            "status": status,
            "is_error": is_error,
            "blocked": blocked,
            "result_preview": result_preview,
            "input_preview": input_preview,
            "detail_ref": detail_ref.get("ref").cloned().unwrap_or(Value::Null)
        })
    }).collect::<Vec<_>>())
}

fn compact_workflow_visibility(value: &Value) -> Value {
    json!({
        "contract": value.get("contract").cloned().unwrap_or_else(|| json!("workflow_visibility_payload_v1")),
        "current_stage": clean_text(value.get("current_stage").and_then(Value::as_str).unwrap_or(""), 120),
        "current_stage_status": clean_text(value.get("current_stage_status").and_then(Value::as_str).unwrap_or(""), 120),
        "ui_status": clean_text(value.get("ui_status").and_then(Value::as_str).unwrap_or(""), 240),
        "agent_process_status": clean_text(value.get("agent_process_status").and_then(Value::as_str).unwrap_or(""), 240),
        "debug_status": clean_text(value.get("debug_status").and_then(Value::as_str).unwrap_or(""), 240),
        "selected_workflow_id": clean_text(value.get("selected_workflow_id").and_then(Value::as_str).unwrap_or(""), 160),
        "visible_response_source": clean_text(value.get("visible_response_source").and_then(Value::as_str).unwrap_or(""), 120),
        "visible_chat_text_authority": clean_text(value.get("visible_chat_text_authority").and_then(Value::as_str).unwrap_or(""), 120),
        "system_chat_injection_used": value.get("system_chat_injection_used").and_then(Value::as_bool).unwrap_or(false)
    })
}

fn compact_terminal_transcript(root: &Path, agent_id: &str, transcript: &Value) -> Value {
    let rows = transcript.as_array().cloned().unwrap_or_default();
    Value::Array(rows.into_iter().take(12).enumerate().map(|(idx, row)| {
        let detail_ref = persist_session_artifact_ref(root, agent_id, &format!("terminal_{idx}"), &row);
        json!({
            "tool": clean_text(row.get("tool").and_then(Value::as_str).unwrap_or("terminal"), 120),
            "command": clean_text(row.get("command").and_then(Value::as_str).unwrap_or(""), 500),
            "cwd": clean_text(row.get("cwd").and_then(Value::as_str).unwrap_or(""), 500),
            "output_preview": clean_text(row.get("output").and_then(Value::as_str).unwrap_or(""), SESSION_TERMINAL_PREVIEW_MAX_CHARS),
            "is_error": row.get("is_error").and_then(Value::as_bool).unwrap_or(false),
            "detail_ref": detail_ref.get("ref").cloned().unwrap_or(Value::Null)
        })
    }).collect::<Vec<_>>())
}

fn session_safe_turn_metadata(root: &Path, agent_id: &str, metadata: &Value) -> Value {
    let mut out = Map::<String, Value>::new();
    let mut refs = Map::<String, Value>::new();
    if let Some(tools) = metadata.get("tools") {
        refs.insert("tools".to_string(), persist_session_artifact_ref(root, agent_id, "tools", tools));
        out.insert("tools".to_string(), compact_session_tool_rows(root, agent_id, tools));
    }
    for key in ["response_workflow", "response_finalization", "process_summary"] {
        if let Some(value) = metadata.get(key) {
            refs.insert(key.to_string(), persist_session_artifact_ref(root, agent_id, key, value));
        }
    }
    if let Some(value) = metadata.get("workflow_visibility") {
        refs.insert("workflow_visibility".to_string(), persist_session_artifact_ref(root, agent_id, "workflow_visibility", value));
        out.insert("workflow_visibility".to_string(), compact_workflow_visibility(value));
    }
    if let Some(value) = metadata.get("turn_transaction") {
        refs.insert("turn_transaction".to_string(), persist_session_artifact_ref(root, agent_id, "turn_transaction", value));
        out.insert("turn_transaction".to_string(), json!({
            "contract_version": value.get("contract_version").cloned().unwrap_or(Value::Null),
            "complete": value.get("complete").cloned().unwrap_or(Value::Null),
            "first_incomplete_stage": clean_text(value.get("first_incomplete_stage").and_then(Value::as_str).unwrap_or(""), 120),
            "receipt_id": clean_text(value.get("receipt_id").and_then(Value::as_str).unwrap_or(""), 160)
        }));
    }
    if let Some(value) = metadata.get("terminal_transcript") {
        refs.insert("terminal_transcript".to_string(), persist_session_artifact_ref(root, agent_id, "terminal_transcript", value));
        out.insert("terminal_transcript".to_string(), compact_terminal_transcript(root, agent_id, value));
    }
    if !refs.is_empty() {
        out.insert("detail_refs".to_string(), Value::Object(refs));
        out.insert("session_projection_contract".to_string(), json!("session_message_projection_v1"));
    }
    Value::Object(out)
}

fn strip_forbidden_session_projection_keys(value: &mut Value) {
    match value {
        Value::Array(rows) => {
            for row in rows {
                strip_forbidden_session_projection_keys(row);
            }
        }
        Value::Object(obj) => {
            let keys = obj.keys().cloned().collect::<Vec<_>>();
            for key in keys {
                if SESSION_FORBIDDEN_PROJECTION_KEYS.contains(&key.as_str()) {
                    obj.remove(&key);
                    continue;
                }
                if key == "detail_refs" {
                    continue;
                }
                if let Some(nested) = obj.get_mut(&key) {
                    strip_forbidden_session_projection_keys(nested);
                }
            }
        }
        _ => {}
    }
}

fn bound_session_message_for_persistence(root: &Path, agent_id: &str, message: &Value) -> Value {
    let Some(obj) = message.as_object() else {
        return json!({});
    };
    let mut out = obj.clone();
    for key in SESSION_FORBIDDEN_PROJECTION_KEYS {
        if let Some(value) = out.remove(key) {
            let detail = persist_session_artifact_ref(root, agent_id, key, &value);
            out.entry("detail_refs".to_string())
                .or_insert_with(|| json!({}))
                .as_object_mut()
                .map(|refs| { refs.insert(key.to_string(), detail); });
        }
    }
    if let Some(text) = out.get("text").and_then(Value::as_str) {
        out.insert("text".to_string(), Value::String(clean_text(text, SESSION_MESSAGE_TEXT_MAX_CHARS)));
    }
    if let Some(tools) = out.get("tools").cloned() {
        out.insert("tools".to_string(), compact_session_tool_rows(root, agent_id, &tools));
    }
    if let Some(visibility) = out.get("workflow_visibility").cloned() {
        out.insert("workflow_visibility".to_string(), compact_workflow_visibility(&visibility));
    }
    if let Some(transcript) = out.get("terminal_transcript").cloned() {
        out.insert("terminal_transcript".to_string(), compact_terminal_transcript(root, agent_id, &transcript));
    }
    let mut projected = Value::Object(out);
    strip_forbidden_session_projection_keys(&mut projected);
    projected
}

fn bound_session_state_for_persistence(root: &Path, agent_id: &str, state: &Value) -> Value {
    let mut bounded = state.clone();
    if let Some(sessions) = bounded.get_mut("sessions").and_then(Value::as_array_mut) {
        for session in sessions.iter_mut() {
            if let Some(messages) = session.get_mut("messages").and_then(Value::as_array_mut) {
                *messages = messages
                    .iter()
                    .map(|message| bound_session_message_for_persistence(root, agent_id, message))
                    .collect::<Vec<_>>();
            }
            if let Some(keyframes) = session.get_mut("context_keyframes").and_then(Value::as_array_mut) {
                keyframes.truncate(24);
                for keyframe in keyframes.iter_mut() {
                    if let Some(obj) = keyframe.as_object_mut() {
                        if let Some(summary) = obj.get("summary").and_then(Value::as_str) {
                            obj.insert("summary".to_string(), Value::String(clean_text(summary, SESSION_MESSAGE_PREVIEW_MAX_CHARS)));
                        }
                    }
                }
            }
        }
    }
    bounded
}

#[cfg(test)]
mod session_output_boundary_tests {
    use super::*;

    fn collect_forbidden_projection_hits(value: &Value, path: &str, hits: &mut Vec<String>) {
        match value {
            Value::Array(rows) => {
                for (idx, row) in rows.iter().enumerate() {
                    collect_forbidden_projection_hits(row, &format!("{path}.{idx}"), hits);
                }
            }
            Value::Object(obj) => {
                for (key, nested) in obj {
                    let next_path = format!("{path}.{key}");
                    if key != "detail_refs" && SESSION_FORBIDDEN_PROJECTION_KEYS.contains(&key.as_str()) {
                        hits.push(next_path.clone());
                    }
                    if key != "detail_refs" {
                        collect_forbidden_projection_hits(nested, &next_path, hits);
                    }
                }
            }
            _ => {}
        }
    }

    #[test]
    fn session_persistence_projects_messages_and_routes_raw_details_to_refs() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        let agent_id = "agent-boundary";
        let state = json!({
            "agent_id": agent_id,
            "active_session_id": "default",
            "sessions": [{
                "session_id": "default",
                "messages": [{
                    "id": "msg-1",
                    "role": "assistant",
                    "text": "visible text",
                    "raw": {"secret": "raw-runtime-state"},
                    "root": {"store": "full-root-state"},
                    "trace_body": {"trace": ["too", "large"]},
                    "metadata": {
                        "raw": {"nested": "raw-runtime-state"},
                        "workflow_graph": {"nodes": [1, 2, 3]}
                    },
                    "tools": [{
                        "name": "search",
                        "status": "done",
                        "input": {"query": "full input payload"},
                        "result": {"very": "large result payload"},
                        "raw": {"tool": "raw detail"}
                    }],
                    "workflow_visibility": {
                        "current_stage": "Searching",
                        "workflow_graph": {"nodes": ["not for shell"]}
                    }
                }]
            }]
        });

        let bounded = bound_session_state_for_persistence(root, agent_id, &state);
        let message = bounded
            .pointer("/sessions/0/messages/0")
            .expect("projected message");
        let mut hits = Vec::<String>::new();
        collect_forbidden_projection_hits(message, "message", &mut hits);
        assert!(hits.is_empty(), "forbidden projection keys leaked: {hits:?}");

        assert_eq!(message.pointer("/tools/0/name").and_then(Value::as_str), Some("search"));
        assert!(message.pointer("/tools/0/input").is_none());
        assert!(message.pointer("/tools/0/result").is_none());
        assert!(message.pointer("/tools/0/raw").is_none());
        assert!(message.pointer("/tools/0/input_preview").and_then(Value::as_str).unwrap_or("").contains("query"));
        assert!(message.pointer("/tools/0/result_preview").and_then(Value::as_str).unwrap_or("").contains("large result"));
        assert!(message.pointer("/tools/0/detail_ref").and_then(Value::as_str).unwrap_or("").starts_with("session_artifact:agent-boundary:tool_0:"));
        let raw_ref = message.pointer("/detail_refs/raw/ref").and_then(Value::as_str).unwrap_or("");
        assert!(raw_ref.starts_with("session_artifact:agent-boundary:raw:"));

        let artifact_dir = session_artifact_dir(root, agent_id);
        let artifact_count = fs::read_dir(&artifact_dir).expect("artifact dir").count();
        assert!(artifact_count >= 2, "expected raw/tool detail artifacts in {artifact_dir:?}");

        let lazy_detail = shell_socket_detail_projection(root, raw_ref, "/api/shell/details/session_artifact?view=full")
            .expect("session artifact detail projection");
        assert_eq!(
            lazy_detail.pointer("/detail_projection/value/secret").and_then(Value::as_str),
            Some("raw-runtime-state")
        );
        assert_eq!(
            lazy_detail.pointer("/detail_kind").and_then(Value::as_str),
            Some("session_artifact_detail")
        );
    }

    #[test]
    fn turn_metadata_projection_keeps_workflow_status_but_not_raw_workflow_payloads() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        let metadata = json!({
            "response_workflow": {"plan_graph": {"nodes": [1, 2, 3]}},
            "response_finalization": {"execution_observation": {"raw": true}},
            "process_summary": {"trace_body": "hidden"},
            "workflow_visibility": {
                "current_stage": "Searching the web",
                "current_stage_status": "running",
                "ui_status": "Searching the web..."
            },
            "tools": [{
                "name": "web",
                "input": {"q": "payload"},
                "result": {"items": ["payload"]}
            }]
        });

        let projected = session_safe_turn_metadata(root, "agent-meta", &metadata);
        let mut hits = Vec::<String>::new();
        collect_forbidden_projection_hits(&projected, "metadata", &mut hits);
        assert!(hits.is_empty(), "forbidden metadata projection keys leaked: {hits:?}");
        assert_eq!(
            projected.pointer("/workflow_visibility/current_stage").and_then(Value::as_str),
            Some("Searching the web")
        );
        assert!(projected.pointer("/response_workflow").is_none());
        assert!(projected.pointer("/response_finalization").is_none());
        assert!(projected.pointer("/process_summary").is_none());
        assert!(projected.pointer("/detail_refs/response_workflow/ref").and_then(Value::as_str).unwrap_or("").starts_with("session_artifact:agent-meta:response_workflow:"));
    }

    #[test]
    fn session_payload_prefers_indexed_window_without_reading_legacy_monolith() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        let agent_id = "agent-indexed";
        let messages = (0..6)
            .map(|idx| {
                json!({
                    "id": format!("msg-{idx}"),
                    "role": if idx % 2 == 0 { "user" } else { "assistant" },
                    "text": format!("message {idx}")
                })
            })
            .collect::<Vec<_>>();
        let state = json!({
            "agent_id": agent_id,
            "active_session_id": "default",
            "sessions": [{
                "session_id": "default",
                "label": "Default",
                "created_at": "2026-05-15T00:00:00Z",
                "updated_at": "2026-05-15T00:00:01Z",
                "messages": messages
            }]
        });

        save_session_state(root, agent_id, &state);
        fs::write(session_path(root, agent_id), b"{ legacy monolith intentionally unreadable")
            .expect("poison legacy monolith");

        let payload = session_payload_paged(root, agent_id, 2, 0);
        assert_eq!(
            payload.get("storage_source").and_then(Value::as_str),
            Some("indexed_session_jsonl")
        );
        assert_eq!(payload.get("message_count").and_then(Value::as_u64), Some(6));
        let rows = payload
            .pointer("/message_window/rows")
            .and_then(Value::as_array)
            .expect("indexed message rows");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].get("id").and_then(Value::as_str), Some("msg-4"));
        assert_eq!(rows[1].get("id").and_then(Value::as_str), Some("msg-5"));
    }

    #[test]
    fn session_index_rebuild_converts_existing_legacy_monolith() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        let agent_id = "agent-rebuild";
        let messages = (0..4)
            .map(|idx| {
                json!({
                    "id": format!("legacy-msg-{idx}"),
                    "role": "assistant",
                    "text": format!("legacy message {idx}")
                })
            })
            .collect::<Vec<_>>();
        let legacy_state = json!({
            "agent_id": agent_id,
            "active_session_id": "default",
            "sessions": [{
                "session_id": "default",
                "label": "Existing",
                "created_at": "2026-05-15T00:00:00Z",
                "updated_at": "2026-05-15T00:00:01Z",
                "messages": messages
            }]
        });
        let legacy_path = session_path(root, agent_id);
        fs::create_dir_all(legacy_path.parent().expect("legacy parent"))
            .expect("legacy parent mkdir");
        write_json_pretty(&legacy_path, &legacy_state);

        let rebuilt = rebuild_indexed_session_state(root, agent_id);
        assert_eq!(rebuilt.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(rebuilt.get("message_count").and_then(Value::as_u64), Some(4));
        fs::write(&legacy_path, b"{ legacy monolith intentionally unreadable")
            .expect("poison legacy monolith");

        let payload = session_payload_paged(root, agent_id, 2, 0);
        assert_eq!(
            payload.get("storage_source").and_then(Value::as_str),
            Some("indexed_session_jsonl")
        );
        let rows = payload
            .pointer("/message_window/rows")
            .and_then(Value::as_array)
            .expect("indexed message rows");
        assert_eq!(rows.len(), 2);
        assert_eq!(
            rows[1].get("id").and_then(Value::as_str),
            Some("legacy-msg-3")
        );
    }

    #[test]
    fn session_index_batch_rebuild_scans_legacy_session_files() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        for agent_id in ["agent-batch-a", "agent-batch-b"] {
            let legacy_path = session_path(root, agent_id);
            fs::create_dir_all(legacy_path.parent().expect("legacy parent"))
                .expect("legacy parent mkdir");
            write_json_pretty(
                &legacy_path,
                &json!({
                    "agent_id": agent_id,
                    "active_session_id": "default",
                    "sessions": [{
                        "session_id": "default",
                        "label": "Batch",
                        "created_at": "2026-05-15T00:00:00Z",
                        "updated_at": "2026-05-15T00:00:01Z",
                        "messages": [{"id": format!("{agent_id}-msg"), "role": "assistant", "text": "hello"}]
                    }]
                }),
            );
        }

        let payload = rebuild_indexed_session_states(root, &json!({"force": true}));
        assert_eq!(payload.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(payload.get("rebuilt_count").and_then(Value::as_u64), Some(2));
        assert_eq!(payload.get("message_count").and_then(Value::as_u64), Some(2));
        assert!(session_index_meta_path(root, "agent-batch-a").exists());
        assert!(session_index_meta_path(root, "agent-batch-b").exists());
    }

    #[test]
    fn session_save_appends_new_indexed_messages_without_rewriting_prefix() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        let agent_id = "agent-append-index";
        let base_state = json!({
            "agent_id": agent_id,
            "active_session_id": "default",
            "sessions": [{
                "session_id": "default",
                "label": "Append",
                "created_at": "2026-05-15T00:00:00Z",
                "updated_at": "2026-05-15T00:00:01Z",
                "messages": [
                    {"id": "append-msg-1", "role": "user", "text": "one"},
                    {"id": "append-msg-2", "role": "assistant", "text": "two"}
                ]
            }]
        });
        save_session_state(root, agent_id, &base_state);
        let messages_path = session_index_messages_path(root, agent_id, "default");
        let offsets_path = session_index_offsets_path(root, agent_id, "default");
        let before_text = fs::read_to_string(&messages_path).expect("indexed messages");
        let before_offsets = read_json_loose(&offsets_path)
            .and_then(|value| value.as_array().cloned())
            .expect("indexed offsets");

        let appended_state = json!({
            "agent_id": agent_id,
            "active_session_id": "default",
            "sessions": [{
                "session_id": "default",
                "label": "Append",
                "created_at": "2026-05-15T00:00:00Z",
                "updated_at": "2026-05-15T00:00:02Z",
                "messages": [
                    {"id": "append-msg-1", "role": "user", "text": "one"},
                    {"id": "append-msg-2", "role": "assistant", "text": "two"},
                    {"id": "append-msg-3", "role": "user", "text": "three"}
                ]
            }]
        });
        save_session_state(root, agent_id, &appended_state);

        let after_text = fs::read_to_string(&messages_path).expect("indexed messages after append");
        let after_offsets = read_json_loose(&offsets_path)
            .and_then(|value| value.as_array().cloned())
            .expect("indexed offsets after append");
        assert!(after_text.starts_with(&before_text));
        assert_eq!(after_text.lines().count(), 3);
        assert_eq!(before_offsets.len(), 2);
        assert_eq!(after_offsets.len(), 3);
        assert_eq!(after_offsets[0], before_offsets[0]);
        assert_eq!(after_offsets[1], before_offsets[1]);
        assert!(after_text.contains("append-msg-3"));
    }
}

fn estimate_tokens(text: &str) -> i64 {
    ((clean_text(text, 20_000).chars().count() as i64) / 4).max(1)
}

fn active_session_row(state: &Value) -> Value {
    active_session_row_ref(state)
        .cloned()
        .unwrap_or_else(|| json!({"messages": []}))
}

fn active_session_row_ref(state: &Value) -> Option<&Value> {
    let active_id = clean_text(
        state
            .get("active_session_id")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        120,
    );
    let rows = state
        .get("sessions")
        .and_then(Value::as_array)
        .map(|rows| rows.as_slice())
        .unwrap_or(&[]);
    if let Some(found) = rows.iter().find(|row| {
        row.get("session_id")
            .and_then(Value::as_str)
            .map(|v| clean_text(v, 120) == active_id)
            .unwrap_or(false)
    }) {
        return Some(found);
    }
    rows.first()
}

fn session_messages(state: &Value) -> Vec<Value> {
    active_session_row(state)
        .get("messages")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn session_messages_paged(state: &Value, limit: usize, offset: usize) -> (Vec<Value>, usize) {
    let messages = active_session_row_ref(state)
        .and_then(|row| row.get("messages"))
        .and_then(Value::as_array);
    let Some(all) = messages else {
        return (Vec::new(), 0);
    };
    let total = all.len();
    if limit == 0 {
        return (all.clone(), total);
    }
    let end = total.saturating_sub(offset);
    let start = end.saturating_sub(limit);
    (all[start..end].to_vec(), total)
}

fn all_session_messages(state: &Value) -> Vec<Value> {
    let sessions = state
        .get("sessions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut rows = Vec::<Value>::new();
    for session in sessions {
        let messages = session
            .get("messages")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        rows.extend(messages);
    }
    rows.sort_by_key(message_timestamp_iso);
    rows
}

const ACTIVE_CONTEXT_MIN_RECENT_FLOOR: usize = 28;

fn active_session_messages_sorted(state: &Value) -> Vec<Value> {
    let mut rows = session_messages(state);
    rows.sort_by_key(message_timestamp_iso);
    rows
}

fn context_source_messages(state: &Value, include_all_sessions: bool) -> Vec<Value> {
    if include_all_sessions {
        all_session_messages(state)
    } else {
        active_session_messages_sorted(state)
    }
}

fn recall_prefers_earliest(user_message: &str) -> bool {
    let lowered = clean_text(user_message, 800).to_ascii_lowercase();
    lowered.contains("first chat")
        || lowered.contains("first conversation")
        || lowered.contains("first message")
        || lowered.contains("very first")
        || lowered.contains("earliest")
        || lowered.contains("at the start")
        || lowered.contains("from the beginning")
}

fn recall_message_candidate(row: &Value, require_remember_term: bool) -> Option<String> {
    let role =
        clean_text(row.get("role").and_then(Value::as_str).unwrap_or(""), 20).to_ascii_lowercase();
    if role != "user" {
        return None;
    }
    let text = message_text(row);
    if text.is_empty() {
        return None;
    }
    if require_remember_term && !text.to_ascii_lowercase().contains("remember") {
        return None;
    }
    Some(text)
}

fn collect_user_recall_messages(
    messages: &[Value],
    prefer_earliest: bool,
    require_remember_term: bool,
    limit: usize,
) -> Vec<String> {
    let take_limit = limit.clamp(1, 8);
    let mut out = Vec::<String>::new();
    let mut seen = HashSet::<String>::new();
    if prefer_earliest {
        for row in messages {
            let Some(text) = recall_message_candidate(row, require_remember_term) else {
                continue;
            };
            let key = clean_text(&text, 320).to_ascii_lowercase();
            if key.is_empty() || !seen.insert(key) {
                continue;
            }
            out.push(text);
            if out.len() >= take_limit {
                break;
            }
        }
        return out;
    }
    for row in messages.iter().rev() {
        let Some(text) = recall_message_candidate(row, require_remember_term) else {
            continue;
        };
        let key = clean_text(&text, 320).to_ascii_lowercase();
        if key.is_empty() || !seen.insert(key) {
            continue;
        }
        out.push(text);
        if out.len() >= take_limit {
            break;
        }
    }
    out
}

fn build_memory_recall_response(
    state: &Value,
    history_messages: &[Value],
    message: &str,
) -> String {
    let prefer_earliest = recall_prefers_earliest(message);
    let active_history_messages = active_session_messages_sorted(state);
    let mut remembered =
        collect_user_recall_messages(&active_history_messages, prefer_earliest, true, 4);
    if remembered.is_empty() {
        remembered =
            collect_user_recall_messages(&active_history_messages, prefer_earliest, false, 4);
    }
    if remembered.is_empty() {
        remembered = collect_user_recall_messages(history_messages, prefer_earliest, true, 3);
    }
    if remembered.is_empty() {
        remembered = collect_user_recall_messages(history_messages, prefer_earliest, false, 3);
    }
    if remembered.is_empty() {
        "I don't have enough earlier context to reference yet. Share what you want me to track, and I'll carry it forward.".to_string()
    } else {
        format!(
            "Here's what I remember from earlier: {}",
            remembered.join(" | ")
        )
    }
}

fn memory_kv_pairs_from_state(state: &Value) -> Vec<Value> {
    let mut out = state
        .get("memory_kv")
        .and_then(Value::as_object)
        .map(|rows| {
            rows.iter()
                .map(|(key, value)| {
                    json!({
                        "key": clean_text(key, 200),
                        "value": value
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    out.sort_by_key(|row| clean_text(row.get("key").and_then(Value::as_str).unwrap_or(""), 200));
    out
}

fn memory_value_timestamp(value: &Value) -> Option<DateTime<Utc>> {
    if let Some(raw) = value
        .get("captured_at")
        .or_else(|| value.get("updated_at"))
        .or_else(|| value.get("ts"))
    {
        if let Some(text) = raw.as_str() {
            if let Some(parsed) = parse_rfc3339_utc(text) {
                return Some(parsed);
            }
        } else if let Some(ms) = raw.as_i64() {
            if let Some(parsed) = DateTime::<Utc>::from_timestamp_millis(ms) {
                return Some(parsed);
            }
        }
    }
    None
}

fn memory_bucket_for_kv(key: &str, value: &Value) -> (&'static str, bool) {
    let key_lc = clean_text(key, 200).to_ascii_lowercase();
    let mut pinned = key_lc.starts_with("pin.")
        || key_lc.contains(".pin.")
        || key_lc.contains(".pinned")
        || key_lc.starts_with("fact.")
        || key_lc.starts_with("profile.")
        || key_lc.starts_with("preference.")
        || key_lc.starts_with("identity.")
        || key_lc.starts_with("user.");

    let mut memory_type = String::new();
    if let Some(obj) = value.as_object() {
        if obj.get("pinned").and_then(Value::as_bool).unwrap_or(false) {
            pinned = true;
        }
        memory_type = clean_text(
            obj.get("memory_type")
                .or_else(|| obj.get("kind"))
                .and_then(Value::as_str)
                .unwrap_or(""),
            60,
        );
        if memory_type.eq_ignore_ascii_case("semantic") {
            pinned = true;
        }
    }

    let bucket = if pinned || memory_type.eq_ignore_ascii_case("semantic") {
        "semantic"
    } else {
        "episodic"
    };
    (bucket, pinned)
}

fn episodic_memory_is_stale(value: &Value, max_age_days: i64) -> bool {
    let Some(captured_at) = memory_value_timestamp(value) else {
        return false;
    };
    let age_days = Utc::now()
        .signed_duration_since(captured_at)
        .num_days()
        .max(0);
    age_days > max_age_days.max(1)
}

fn memory_kv_prompt_context(state: &Value, max_entries: usize) -> String {
    let mut semantic_lines = Vec::<String>::new();
    let mut episodic_lines = Vec::<String>::new();
    let kv_pairs = memory_kv_pairs_from_state(state);
    for row in kv_pairs.into_iter().take(max_entries.max(1)) {
        let key = clean_text(row.get("key").and_then(Value::as_str).unwrap_or(""), 120);
        if key.is_empty() {
            continue;
        }
        let value = row.get("value").cloned().unwrap_or(Value::Null);
        let rendered = if value.is_string() {
            clean_text(value.as_str().unwrap_or(""), 280)
        } else {
            clean_text(&value.to_string(), 280)
        };
        if rendered.is_empty() {
            continue;
        }
        if internal_context_metadata_phrase(&rendered)
            || persistent_memory_denied_phrase(&rendered)
            || runtime_access_denied_phrase(&rendered)
        {
            continue;
        }
        let (bucket, pinned) = memory_bucket_for_kv(&key, &value);
        if bucket == "episodic" && !pinned && episodic_memory_is_stale(&value, 14) {
            continue;
        }
        let line = format!("- {key}: {rendered}");
        if bucket == "semantic" {
            semantic_lines.push(line);
        } else {
            episodic_lines.push(line);
        }
    }
    semantic_lines.truncate(16);
    episodic_lines.truncate(8);

    let mut sections = Vec::<String>::new();
    if !semantic_lines.is_empty() {
        sections.push(format!(
            "Pinned semantic memory (stable facts/preferences):\n{}",
            semantic_lines.join("\n")
        ));
    }
    if !episodic_lines.is_empty() {
        sections.push(format!(
            "Recent episodic memory (working context):\n{}",
            episodic_lines.join("\n")
        ));
    }
    sections.join("\n\n")
}

fn session_rows_payload(state: &Value) -> Vec<Value> {
    let active_id = clean_text(
        state
            .get("active_session_id")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        120,
    );
    state
        .get("sessions")
        .and_then(Value::as_array)
        .map(|rows| rows.as_slice())
        .unwrap_or(&[])
        .iter()
        .map(|row| {
            let sid = clean_text(
                row.get("session_id").and_then(Value::as_str).unwrap_or(""),
                120,
            );
            let label = clean_text(
                row.get("label")
                    .and_then(Value::as_str)
                    .unwrap_or("Session"),
                80,
            );
            let updated_at = clean_text(
                row.get("updated_at").and_then(Value::as_str).unwrap_or(""),
                80,
            );
            let message_count = row
                .get("messages")
                .and_then(Value::as_array)
                .map(|rows| rows.len() as i64)
                .unwrap_or(0);
            json!({
                "id": sid,
                "session_id": sid,
                "label": if label.is_empty() { "Session" } else { &label },
                "updated_at": updated_at,
                "message_count": message_count,
                "active": sid == active_id
            })
        })
        .collect::<Vec<_>>()
}

fn split_model_ref(
    model_ref: &str,
    fallback_provider: &str,
    fallback_model: &str,
) -> (String, String) {
    fn normalize_runtime_provider_id(raw: &str) -> String {
        let lowered = clean_text(raw, 80)
            .replace('_', "-")
            .to_ascii_lowercase();
        match lowered.as_str() {
            "google" => "gemini".to_string(),
            "xai" => "grok".to_string(),
            "moonshot" => "kimi".to_string(),
            "azure-openai-responses" => "openai".to_string(),
            _ => lowered,
        }
    }

    let cleaned = clean_text(model_ref, 200);
    if cleaned.contains('/') {
        let mut parts = cleaned.splitn(2, '/');
        let provider = normalize_runtime_provider_id(parts.next().unwrap_or(""));
        let model = clean_text(parts.next().unwrap_or(""), 120);
        if !provider.is_empty() && !model.is_empty() {
            return (provider, model);
        }
    }
    let provider = if fallback_provider.is_empty() {
        "auto".to_string()
    } else {
        normalize_runtime_provider_id(fallback_provider)
    };
    let model = if cleaned.is_empty() {
        clean_text(fallback_model, 120)
    } else {
        cleaned
    };
    (provider, model)
}

fn parse_i64_loose(value: Option<&Value>) -> i64 {
    value
        .and_then(|row| {
            row.as_i64()
                .or_else(|| row.as_u64().map(|num| num as i64))
                .or_else(|| {
                    row.as_str()
                        .and_then(|text| clean_text(text, 40).parse::<i64>().ok())
                })
        })
        .unwrap_or(0)
        .max(0)
}
