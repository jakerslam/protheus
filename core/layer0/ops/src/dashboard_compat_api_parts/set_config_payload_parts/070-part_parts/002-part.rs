fn latent_tool_candidates_for_message(message: &str, workspace_hints: &[Value]) -> Vec<Value> {
    let lowered = clean_text(message, 1400).to_ascii_lowercase();
    if lowered.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::<Value>::new();
    let mut seen = HashSet::<String>::new();

    let workflow_hints = chat_workflow_tool_hints_for_message(message);
    let has_workflow_error = workflow_hints.iter().any(|row| {
        row.get("tool")
            .and_then(Value::as_str)
            .map(|tool| tool == "tool_command_router")
            .unwrap_or(false)
            && row
                .get("workflow_only")
                .and_then(Value::as_bool)
                .unwrap_or(false)
    });
    for hint in workflow_hints {
        let normalized = normalize_tool_name(hint.get("tool").and_then(Value::as_str).unwrap_or(""));
        if normalized.is_empty() {
            continue;
        }
        if normalized != "tool_command_router" && seen.contains(&normalized) {
            continue;
        }
        if normalized != "tool_command_router" {
            seen.insert(normalized);
        }
        out.push(hint);
    }
    if has_workflow_error {
        out.truncate(3);
        return out;
    }

    let security_request = (lowered.contains("security")
        || lowered.contains("vulnerability")
        || lowered.contains("exploit")
        || lowered.contains("audit"))
        && (lowered.contains("code")
            || lowered.contains("api")
            || lowered.contains("module")
            || lowered.contains("file"));
    if security_request {
        push_latent_tool_candidate(
            &mut out,
            &mut seen,
            lowered.as_str(),
            "terminal_exec",
            "run security checks",
            "Security concern detected for code-path request.",
            json!({"command": "cargo test --workspace --tests"}),
        );
    }

    if let Some(path) = workspace_hints
        .first()
        .and_then(|row| row.get("path").and_then(Value::as_str))
    {
        if lowered.contains("file")
            || lowered.contains("module")
            || lowered.contains("api")
            || lowered.contains("update")
            || lowered.contains("change")
            || lowered.contains("patch")
            || lowered.contains("refactor")
        {
            push_latent_tool_candidate(
                &mut out,
                &mut seen,
                lowered.as_str(),
                "file_read",
                "open likely file",
                "Workspace file inference found a likely target.",
                json!({"path": path, "full": true}),
            );
        }
    }

    if let Some((workspace_query, web_query)) =
        workspace_plus_web_comparison_queries_from_message(message)
    {
        push_latent_tool_candidate(
            &mut out,
            &mut seen,
            lowered.as_str(),
            "workspace_analyze",
            "inspect local workspace evidence",
            "Message compares the local system/workspace to an external peer, so local workspace evidence is required.",
            json!({"path": ".", "query": workspace_query, "full": true}),
        );
        push_latent_tool_candidate(
            &mut out,
            &mut seen,
            lowered.as_str(),
            "batch_query",
            "collect external peer evidence",
            "Message compares the local system/workspace to an external peer, so external web evidence is required too.",
            workspace_plus_web_comparison_web_payload_from_message(message)
                .unwrap_or_else(|| json!({"source": "web", "query": web_query, "aperture": "medium"})),
        );
    } else if let Some(query) = natural_web_search_query_from_message(message) {
        push_latent_tool_candidate(
            &mut out,
            &mut seen,
            lowered.as_str(),
            "batch_query",
            "search live web",
            "Message explicitly asks for a live web search.",
            json!({"source": "web", "query": query, "aperture": "medium"}),
        );
    } else if let Some(query) = comparative_web_query_from_message(message) {
        push_latent_tool_candidate(
            &mut out,
            &mut seen,
            lowered.as_str(),
            "batch_query",
            "compare on live web",
            "Message asks for a comparative answer that should use live web evidence.",
            json!({"source": "web", "query": query, "aperture": "medium"}),
        );
    } else if ["test web fetch", "do a test web fetch", "try web fetch", "check web fetch"]
        .iter()
        .any(|term| lowered.contains(term))
    {
        push_latent_tool_candidate(
            &mut out,
            &mut seen,
            lowered.as_str(),
            "web_fetch",
            "test web fetch",
            "Message explicitly asks for a diagnostic web fetch probe.",
            json!({"url": "https://example.com", "summary_only": true}),
        );
    } else if lowered.contains("search")
        || lowered.contains("latest")
        || lowered.contains("news")
        || lowered.contains("internet")
        || lowered.contains("online")
        || lowered.contains("look up")
    {
        push_latent_tool_candidate(
            &mut out,
            &mut seen,
            lowered.as_str(),
            "batch_query",
            "search web",
            "Message implies live web research intent.",
            json!({"source": "web", "query": clean_text(message, 600), "aperture": "medium"}),
        );
    }

    if lowered.contains("what did we decide")
        || lowered.contains("remember")
        || lowered.contains("recall")
        || lowered.contains("last month")
        || lowered.contains("previously")
    {
        push_latent_tool_candidate(
            &mut out,
            &mut seen,
            lowered.as_str(),
            "memory_semantic_query",
            "query semantic memory",
            "Message implies historical decision recall intent.",
            json!({"query": clean_text(message, 600), "limit": 8}),
        );
    }

    if lowered.contains("schedule")
        || lowered.contains("remind")
        || lowered.contains("every ")
        || lowered.contains("daily")
        || lowered.contains("cron")
    {
        push_latent_tool_candidate(
            &mut out,
            &mut seen,
            lowered.as_str(),
            "cron_schedule",
            "schedule follow-up",
            "Message implies recurring follow-up intent.",
            json!({"interval_minutes": 60, "message": clean_text(message, 400)}),
        );
    }

    if lowered.contains("swarm")
        || lowered.contains("parallel")
        || lowered.contains("subagent")
        || lowered.contains("multi-agent")
    {
        push_latent_tool_candidate(
            &mut out,
            &mut seen,
            lowered.as_str(),
            "spawn_subagents",
            "parallel subagents",
            "Message implies parallel execution intent.",
            json!({"count": infer_subagent_count_from_message(message), "objective": clean_text(message, 600)}),
        );
    }

    out.truncate(3);
    out
}

fn truncate_utf8_lossy(bytes: &[u8], max_bytes: usize) -> (String, bool) {
    if bytes.len() <= max_bytes {
        return (String::from_utf8_lossy(bytes).to_string(), false);
    }
    let mut end = max_bytes;
    while end > 0 && !std::str::from_utf8(&bytes[..end]).is_ok() {
        end -= 1;
    }
    let slice = if end == 0 {
        &bytes[..max_bytes]
    } else {
        &bytes[..end]
    };
    (String::from_utf8_lossy(slice).to_string(), true)
}

fn bytes_look_binary(bytes: &[u8]) -> bool {
    if bytes.is_empty() {
        return false;
    }
    let probe_len = bytes.len().min(4096);
    let sample = &bytes[..probe_len];
    if sample.iter().any(|byte| *byte == 0) {
        return true;
    }
    let control_count = sample
        .iter()
        .filter(|byte| {
            let b = **byte;
            b < 9 || (b > 13 && b < 32)
        })
        .count();
    let control_ratio = control_count as f64 / probe_len as f64;
    if control_ratio > 0.12 {
        return true;
    }
    std::str::from_utf8(sample).is_err() && control_ratio > 0.04
}

fn guess_mime_type_for_file(path: &Path, bytes: &[u8]) -> String {
    let ext = path
        .extension()
        .and_then(|row| row.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let known = match ext.as_str() {
        "md" => "text/markdown; charset=utf-8",
        "txt" | "log" | "toml" | "yaml" | "yml" | "json" | "jsonl" | "csv" | "tsv" => {
            "text/plain; charset=utf-8"
        }
        "rs" | "ts" | "tsx" | "py" | "sh" | "zsh" | "bash" | "js" | "cjs" | "mjs" | "c" | "h"
        | "cpp" | "hpp" | "go" | "java" | "kt" | "swift" | "sql" | "css" | "html" | "xml" => {
            "text/plain; charset=utf-8"
        }
        "pdf" => "application/pdf",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "mp4" => "video/mp4",
        "zip" => "application/zip",
        "gz" => "application/gzip",
        "tar" => "application/x-tar",
        _ => "",
    };
    if !known.is_empty() {
        return known.to_string();
    }
    if bytes_look_binary(bytes) {
        "application/octet-stream".to_string()
    } else {
        "text/plain; charset=utf-8".to_string()
    }
}

fn attention_policy_path(root: &Path) -> PathBuf {
    let from_env = std::env::var("MECH_SUIT_MODE_POLICY_PATH")
        .ok()
        .map(PathBuf::from);
    if let Some(path) = from_env {
        if path.is_absolute() {
            return path;
        }
        return root.join(path);
    }
    let default_root = root.join("config").join("mech_suit_mode_policy.json");
    if default_root.exists() {
        return default_root;
    }
    root.join("client/runtime/config/mech_suit_mode_policy.json")
}

fn attention_queue_path_for_dashboard(root: &Path) -> PathBuf {
    let fallback = root.join("client/runtime/local/state/attention/queue.jsonl");
    let policy = read_json_loose(&attention_policy_path(root)).unwrap_or_else(|| json!({}));
    let from_policy = clean_text(
        policy
            .pointer("/eyes/attention_queue_path")
            .and_then(Value::as_str)
            .unwrap_or(""),
        4000,
    );
    if from_policy.is_empty() {
        return fallback;
    }
    let raw = PathBuf::from(from_policy);
    if raw.is_absolute() {
        raw
    } else {
        root.join(raw)
    }
}
