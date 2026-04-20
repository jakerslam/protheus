
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
