fn memory_candidate_files(control_runtime_root: &Path) -> Vec<PathBuf> {
    let agent = agent_root(control_runtime_root);
    vec![
        control_runtime_root.join("control_runtime.json"),
        agent.join("models.json"),
        agent.join("routing-policy.json"),
        agent.join("identity.md"),
        control_runtime_root.join("state.json"),
        control_runtime_root.join("decisions.md"),
        control_runtime_root.join("logs/spawn-safe.jsonl"),
        control_runtime_root.join("logs/spawn-run.jsonl"),
        control_runtime_root.join("logs/decision-log.jsonl"),
        agent.join("state.json"),
        agent.join("decisions.md"),
    ]
}

fn search_file_lines(path: &Path, query_lc: &str, limit: usize) -> Vec<Value> {
    let Ok(file) = fs::File::open(path) else {
        return Vec::new();
    };
    let reader = BufReader::new(file);
    let mut out = Vec::<Value>::new();
    for (idx, line) in reader.lines().enumerate() {
        let Ok(text) = line else {
            continue;
        };
        if text.to_ascii_lowercase().contains(query_lc) {
            out.push(json!({
                "file": path.to_string_lossy().to_string(),
                "line": idx + 1,
                "text": clean_text(&text, 360)
            }));
            if out.len() >= limit {
                break;
            }
        }
    }
    out
}

fn run_memory_search(control_runtime_root: &Path, query: &str, limit: usize) -> Value {
    let query_clean = clean_text(query, 240);
    let query_lc = query_clean.to_ascii_lowercase();
    let mut files_scanned = 0usize;
    let mut results = Vec::<Value>::new();

    for file in memory_candidate_files(control_runtime_root) {
        if !file.is_file() {
            continue;
        }
        files_scanned = files_scanned.saturating_add(1);
        let remaining = limit.saturating_sub(results.len()).max(1);
        let mut hits = search_file_lines(&file, &query_lc, remaining);
        results.append(&mut hits);
        if results.len() >= limit {
            break;
        }
    }

    with_receipt(json!({
        "ok": true,
        "type": "operator_tooling_memory_search",
        "query": query_clean,
        "control_runtime_root": control_runtime_root.to_string_lossy().to_string(),
        "files_scanned": files_scanned,
        "match_count": results.len(),
        "matches": results
    }))
}

fn find_trace_jsonl(path: &Path, trace_id: &str, limit: usize) -> Vec<Value> {
    let Ok(file) = fs::File::open(path) else {
        return Vec::new();
    };
    let reader = BufReader::new(file);
    let mut out = Vec::<Value>::new();
    for line in reader.lines() {
        let Ok(raw) = line else {
            continue;
        };
        let parsed = serde_json::from_str::<Value>(&raw).unwrap_or_else(|_| json!({}));
        let matches = parsed
            .get("trace_id")
            .and_then(Value::as_str)
            .map(|value| value == trace_id)
            .unwrap_or(false)
            || parsed
                .get("ids")
                .and_then(Value::as_object)
                .and_then(|ids| ids.get("trace_id"))
                .and_then(Value::as_str)
                .map(|value| value == trace_id)
                .unwrap_or(false)
            || raw.contains(trace_id);
        if matches {
            out.push(parsed);
            if out.len() >= limit {
                break;
            }
        }
    }
    out
}

fn find_trace_decisions(path: &Path, trace_id: &str, limit: usize) -> Vec<Value> {
    let Ok(raw) = fs::read_to_string(path) else {
        return Vec::new();
    };
    raw.split("\n## ")
        .filter(|chunk| chunk.contains(trace_id))
        .take(limit)
        .map(|chunk| {
            let mut lines = chunk.lines();
            let title = clean_text(lines.next().unwrap_or("Decision"), 120);
            json!({
                "source": path.to_string_lossy().to_string(),
                "title": title,
                "context": clean_text(chunk, 500)
            })
        })
        .collect::<Vec<_>>()
}

fn find_trace_state(path: &Path, trace_id: &str) -> Vec<Value> {
    let Some(state) = read_json_file(path) else {
        return Vec::new();
    };
    let mut out = Vec::<Value>::new();
    for section in ["handoffs_recent", "executions_recent"] {
        if let Some(map) = state.get(section).and_then(Value::as_object) {
            for (key, row) in map {
                let hit = row
                    .get("trace_id")
                    .and_then(Value::as_str)
                    .map(|value| value == trace_id)
                    .unwrap_or(false)
                    || row.to_string().contains(trace_id);
                if hit {
                    out.push(json!({
                        "source": format!("{}:{section}", path.to_string_lossy()),
                        "key": key,
                        "entry": row
                    }));
                }
            }
        }
    }
    out
}

fn run_trace_find(control_runtime_root: &Path, trace_id: &str, limit: usize) -> Value {
    let trace = clean_text(trace_id, 160);
    let agent = agent_root(control_runtime_root);
    let spawn_safe = control_runtime_root.join("logs/spawn-safe.jsonl");
    let spawn_run = control_runtime_root.join("logs/spawn-run.jsonl");
    let decisions = agent.join("decisions.md");
    let state = agent.join("state.json");
    let spawn_safe_rows = find_trace_jsonl(&spawn_safe, &trace, limit);
    let spawn_run_rows = find_trace_jsonl(&spawn_run, &trace, limit);
    let decision_rows = find_trace_decisions(&decisions, &trace, limit);
    let state_rows = find_trace_state(&state, &trace);
    let total = spawn_safe_rows.len() + spawn_run_rows.len() + decision_rows.len() + state_rows.len();
    with_receipt(json!({
        "ok": true,
        "type": "operator_tooling_trace_find",
        "trace_id": trace,
        "control_runtime_root": control_runtime_root.to_string_lossy().to_string(),
        "total_matches": total,
        "spawn_safe_logs": spawn_safe_rows,
        "spawn_run_logs": spawn_run_rows,
        "decisions": decision_rows,
        "state_entries": state_rows
    }))
}

fn workspace_root(root: &Path, parsed: &crate::ParsedArgs) -> PathBuf {
    path_from_flag(root, parsed.flags.get("workspace-root")).unwrap_or_else(|| root.to_path_buf())
}

fn run_memory_summarize(control_runtime_root: &Path, query: &str, limit: usize) -> Value {
    let search = run_memory_search(control_runtime_root, query, limit);
    let mut grouped = BTreeMap::<String, Vec<Value>>::new();
    if let Some(rows) = search.get("matches").and_then(Value::as_array) {
        for row in rows {
            let file = clean_text(row.get("file").and_then(Value::as_str).unwrap_or(""), 1024);
            if file.is_empty() {
                continue;
            }
            grouped.entry(file).or_default().push(row.clone());
        }
    }
    let grouped_rows = grouped
        .into_iter()
        .map(|(file, rows)| {
            let excerpts = rows
                .iter()
                .take(8)
                .map(|row| {
                    json!({
                        "line": row.get("line").cloned().unwrap_or(Value::Null),
                        "text": row.get("text").cloned().unwrap_or(Value::Null)
                    })
                })
                .collect::<Vec<_>>();
            json!({
                "file": file,
                "match_count": rows.len(),
                "excerpts": excerpts
            })
        })
        .collect::<Vec<_>>();
    with_receipt(json!({
        "ok": true,
        "type": "operator_tooling_memory_summarize",
        "query": clean_text(query, 240),
        "control_runtime_root": control_runtime_root.to_string_lossy().to_string(),
        "files_with_matches": grouped_rows.len(),
        "grouped": grouped_rows
    }))
}

fn run_memory_last_change(control_runtime_root: &Path, limit: usize) -> Value {
    let mut rows = WalkDir::new(control_runtime_root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter_map(|entry| {
            let path = entry.path().to_path_buf();
            let path_text = path.to_string_lossy().to_string();
            if path_text.contains("/agents/main/sessions/") {
                return None;
            }
            if path_text.ends_with(".jsonl") || path_text.contains(".bak.") {
                return None;
            }
            let meta = fs::metadata(&path).ok()?;
            let modified = meta.modified().ok()?;
            let modified_secs = modified
                .duration_since(UNIX_EPOCH)
                .ok()
                .map(|d| d.as_secs())
                .unwrap_or(0);
            Some((modified_secs, path, meta.len()))
        })
        .collect::<Vec<_>>();
    rows.sort_by(|a, b| b.0.cmp(&a.0));
    let top_rows = rows
        .into_iter()
        .take(limit)
        .map(|(modified_secs, path, size_bytes)| {
            json!({
                "path": path.to_string_lossy().to_string(),
                "modified_epoch_secs": modified_secs,
                "size_bytes": size_bytes
            })
        })
        .collect::<Vec<_>>();
    with_receipt(json!({
        "ok": true,
        "type": "operator_tooling_memory_last_change",
        "control_runtime_root": control_runtime_root.to_string_lossy().to_string(),
        "rows": top_rows,
        "row_count": top_rows.len()
    }))
}

fn run_membrief(control_runtime_root: &Path, query: &str, limit: usize) -> Value {
    let summary = run_memory_summarize(control_runtime_root, query, limit);
    let recent = run_memory_last_change(control_runtime_root, 25);
    with_receipt(json!({
        "ok": true,
        "type": "operator_tooling_membrief",
        "query": clean_text(query, 240),
        "summary": summary,
        "recent_changes": recent
    }))
}

fn all_models_from_policy(policy: &Value) -> Vec<String> {
    let mut out = Vec::<String>::new();
    if let Some(tiers) = policy.get("tiers").and_then(Value::as_object) {
        for value in tiers.values() {
            if let Some(model) = value.as_str() {
                let cleaned = clean_text(model, 240);
                if !cleaned.is_empty() && !out.iter().any(|row| row == &cleaned) {
                    out.push(cleaned);
                }
            }
            if let Some(rows) = value.as_array() {
                for row in rows {
                    if let Some(model) = row.as_str() {
                        let cleaned = clean_text(model, 240);
                        if !cleaned.is_empty() && !out.iter().any(|entry| entry == &cleaned) {
                            out.push(cleaned);
                        }
                    }
                }
            }
        }
    }
    out
}

