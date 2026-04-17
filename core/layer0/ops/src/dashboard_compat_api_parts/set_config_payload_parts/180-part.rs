fn direct_tool_intent_from_user_message(message: &str) -> Option<(String, Value)> {
    let trimmed = message.trim();
    if !trimmed.starts_with('/') {
        let lowered = clean_text(trimmed, 400).to_ascii_lowercase();
        if matches!(
            lowered.as_str(),
            "undo" | "undo that" | "undo last turn" | "rewind that" | "rollback that"
        ) {
            return Some(("session_rollback_last_turn".to_string(), json!({})));
        }
        // Conversational turns stay model-first. Even explicit tool syntax in chat is now
        // surfaced as workflow/catalog guidance for the LLM instead of a pre-LLM direct route.
        return None;
    }
    let mut split = trimmed.splitn(2, char::is_whitespace);
    let command = split
        .next()
        .map(|value| value.trim().to_ascii_lowercase())
        .unwrap_or_default();
    let arg = split.next().map(str::trim).unwrap_or("");
    match command.as_str() {
        "/undo" | "/rewind" | "/rollback" => {
            Some(("session_rollback_last_turn".to_string(), json!({})))
        }
        "/cron" | "/schedule" => cron_tool_request_from_args(arg),
        _ => None,
    }
}

fn response_tools_failure_reason_for_user(response_tools: &[Value], max_items: usize) -> String {
    let limit = max_items.clamp(1, 6);
    let mut lines = Vec::<String>::new();
    let mut seen = HashSet::<String>::new();
    for tool in response_tools {
        let normalized_name =
            normalize_tool_name(tool.get("name").and_then(Value::as_str).unwrap_or("tool"));
        let name = clean_text(&normalized_name, 80).replace('_', " ");
        let status = clean_text(tool.get("status").and_then(Value::as_str).unwrap_or(""), 120)
            .to_ascii_lowercase();
        let is_web_tool = matches!(
            normalized_name.as_str(),
            "batch_query"
                | "web_search"
                | "search_web"
                | "search"
                | "web_query"
                | "web_fetch"
                | "browse"
                | "web_conduit_fetch"
                | "web_tooling_health_probe"
        );
        let blocked = tool.get("blocked").and_then(Value::as_bool).unwrap_or(false);
        let errored = tool.get("is_error").and_then(Value::as_bool).unwrap_or(false);
        if name.eq_ignore_ascii_case("thought_process") {
            continue;
        }
        let raw_result = clean_text(tool.get("result").and_then(Value::as_str).unwrap_or(""), 800);
        let rewritten_result = rewrite_tool_result_for_user_summary(&normalized_name, &raw_result)
            .unwrap_or_else(|| raw_result.clone());
        let actionable_diagnostic = !clean_text(&rewritten_result, 420).is_empty()
            && (response_is_actionable_tool_diagnostic(&rewritten_result)
                || response_is_no_findings_placeholder(&rewritten_result)
                || response_looks_like_tool_ack_without_findings(&rewritten_result));
        if !blocked
            && !errored
            && !actionable_diagnostic
            && !matches!(
                status.as_str(),
                "blocked"
                    | "error"
                    | "failed"
                    | "timeout"
                    | "policy_denied"
                    | "no_results"
                    | "low_signal"
                    | "partial_no_results"
            )
        {
            continue;
        }
        let fallback_reason = match status.as_str() {
            "no_results" | "low_signal" | "partial_no_results" => {
                if is_web_tool {
                    "Web retrieval ran, but this turn only produced low-signal or no-results output."
                } else {
                    "The tool ran, but this turn only produced low-signal or no-results output."
                }
            }
            "timeout" => "The tool timed out before it could return usable findings.",
            "policy_denied" | "blocked" => "The tool was blocked by policy before it could run.",
            "error" | "failed" => "The tool failed before it could return usable findings.",
            _ => {
                if blocked {
                    "The tool was blocked before it could return usable findings."
                } else if errored {
                    "The tool failed before it could return usable findings."
                } else {
                    "The tool reported a problem before it could return usable findings."
                }
            }
        };
        let reason = first_sentence(
            &clean_text(
                if rewritten_result.is_empty() {
                    fallback_reason
                } else {
                    &rewritten_result
                },
                400,
            ),
            220,
        );
        if reason.is_empty() {
            continue;
        }
        let line = format!("- {}: {}", clean_text(&name, 60), reason);
        if seen.insert(line.to_ascii_lowercase()) {
            lines.push(line);
        }
        if lines.len() >= limit {
            break;
        }
    }
    if lines.is_empty() {
        String::new()
    } else {
        trim_text(
            &format!("The tool run hit issues:\n{}", lines.join("\n")),
            32_000,
        )
    }
}

fn workspace_analyze_intent_from_message(
    trimmed: &str,
    lowered: &str,
) -> Option<(String, Value)> {
    if lowered.is_empty() {
        return None;
    }
    let asks_ls = lowered == "ls"
        || lowered.starts_with("ls ")
        || lowered.contains(" run ls")
        || lowered.contains("list files")
        || lowered.contains("show files")
        || lowered.contains("directory listing")
        || lowered.contains("folder listing");
    let mentions_workspace = lowered.contains("workspace")
        || lowered.contains("repo")
        || lowered.contains("repository")
        || lowered.contains("project directory")
        || lowered.contains("project folder")
        || lowered.contains("this directory");
    let asks_file_surface = lowered.contains("files")
        || lowered.contains("logs")
        || lowered.contains("directories")
        || lowered.contains("folders")
        || lowered.contains("tree");
    let asks_analysis = lowered.contains("analy")
        || lowered.contains("analyse")
        || lowered.contains("parse")
        || lowered.contains("inspect")
        || lowered.contains("scan")
        || lowered.contains("summarize")
        || lowered.contains("tell me about");
    if !(asks_ls || (mentions_workspace && (asks_file_surface || asks_analysis))) {
        return None;
    }
    let query = clean_text(trimmed, 600);
    if query.is_empty() {
        return None;
    }
    Some(("workspace_analyze".to_string(), json!({ "query": query })))
}

fn message_explicitly_disallows_tool_calls(message: &str) -> bool {
    let lowered = clean_text(message, 400).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    lowered.contains("dont use a tool")
        || lowered.contains("don't use a tool")
        || lowered.contains("do not use a tool")
        || lowered.contains("dont call a tool")
        || lowered.contains("don't call a tool")
        || lowered.contains("do not call a tool")
        || lowered.contains("without tool")
        || lowered.contains("no tool call")
        || lowered.contains("just talk to me")
        || lowered.contains("just answer")
}

fn message_is_meta_control_turn(message: &str) -> bool {
    let lowered = clean_text(message, 1_200).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    let meta_marker_hit = [
        "that was just a test",
        "just a test",
        "just testing",
        "test only",
        "ignore that",
        "never mind",
        "nm",
        "thanks",
        "thank you",
        "cool",
        "sounds good",
        "did you try it",
        "did you do it",
        "what happened",
        "give 10 steps",
        "give me 10 steps",
        "actionable steps",
        "those were broad",
    ]
    .iter()
    .any(|marker| lowered.contains(marker));
    if !meta_marker_hit {
        return false;
    }
    !["search", "web", "online", "internet", "file", "memory", "repo", "codebase"]
        .iter()
        .any(|marker| lowered.contains(marker))
}

fn message_requests_local_file_mutation(message: &str) -> bool {
    let lowered = clean_text(message, 800).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    lowered.contains("edit ")
        || lowered.contains("patch ")
        || lowered.contains("update file")
        || lowered.contains("change file")
        || lowered.contains("modify ")
        || lowered.contains("rewrite ")
        || lowered.contains("create file")
        || lowered.contains("delete file")
}

fn message_requires_information_search(message: &str) -> bool {
    let lowered = clean_text(message, 1_000).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    let online_intent = lowered.contains("latest ")
        || lowered.contains("most recent")
        || lowered.contains("today")
        || lowered.contains("current ")
        || lowered.contains("online")
        || lowered.contains("on the web")
        || lowered.contains("search for")
        || lowered.contains("look up")
        || lowered.contains("web search");
    let local_intent = lowered.contains("in this repo")
        || lowered.contains("in this codebase")
        || lowered.contains("workspace")
        || lowered.contains("local files")
        || lowered.contains("project files");
    online_intent || local_intent
}

fn inline_tool_calls_allowed_for_user_message(message: &str) -> bool {
    let cleaned = clean_text(message, 2_200);
    if cleaned.is_empty() {
        return false;
    }
    if message_is_tooling_status_check(&cleaned) {
        return false;
    }
    if message_is_meta_control_turn(&cleaned) {
        return false;
    }
    if message_explicitly_disallows_tool_calls(&cleaned) {
        return false;
    }
    if message_requests_local_file_mutation(&cleaned) {
        return true;
    }
    let requires_information_search = message_requires_information_search(&cleaned);
    if !chat_workflow_tool_hints_for_message(&cleaned).is_empty() {
        return requires_information_search;
    }
    let lowered = cleaned.to_ascii_lowercase();
    let asks_file_read = lowered.contains("read file")
        || lowered.contains("open file")
        || lowered.contains("show file")
        || lowered.contains("view file")
        || lowered.contains("inspect file")
        || lowered.starts_with("cat ");
    let asks_memory = memory_recall_requested(&cleaned)
        || (lowered.contains("what did we decide") && lowered.contains("about"));
    let asks_workspace = workspace_analyze_intent_from_message(&cleaned, &lowered).is_some();
    let asks_follow_up_tool = follow_up_suggestion_tool_intent_from_message(&cleaned).is_some();
    let asks_live_web = natural_web_intent_from_user_message(&cleaned).is_some();
    let asks_mixed_compare =
        workspace_plus_web_comparison_queries_from_message(&cleaned).is_some();
    if (asks_live_web || asks_mixed_compare) && !requires_information_search {
        return false;
    }
    swarm_intent_requested(&cleaned)
        || asks_file_read
        || asks_memory
        || asks_workspace
        || asks_follow_up_tool
        || asks_live_web
        || asks_mixed_compare
        || lowered.contains("multi-agent")
        || lowered.contains("multi agent")
        || lowered.contains("use tool")
        || lowered.contains("run tool")
        || lowered.contains("call tool")
        || lowered.contains("execute tool")
        || lowered.contains("do a tool call")
        || lowered.contains("run a tool call")
}
