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
    let _ = (response_tools, max_items);
    String::new()
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
        || lowered.contains("dont run tools")
        || lowered.contains("don't run tools")
        || lowered.contains("do not run tools")
        || lowered.contains("without running tools")
        || lowered.contains("without tool")
        || lowered.contains("no tool call")
        || lowered.contains("no tools yet")
        || lowered.contains("dry run only")
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
    let lowered = cleaned.to_ascii_lowercase();
    let is_explicit_slash_tool_turn = lowered.starts_with("/file")
        || lowered.starts_with("/search")
        || lowered.starts_with("/browse")
        || lowered.starts_with("/batch")
        || lowered.starts_with("/tool");
    is_explicit_slash_tool_turn
}
