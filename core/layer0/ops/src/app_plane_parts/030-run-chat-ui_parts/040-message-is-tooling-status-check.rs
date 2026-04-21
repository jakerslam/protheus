fn chat_ui_message_is_tooling_status_check(raw_input: &str) -> bool {
    let lowered = clean(raw_input, 1_200).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    let status_frame = lowered.starts_with("did you")
        || lowered.starts_with("what happened")
        || lowered.starts_with("whats going on")
        || lowered.starts_with("what's going on")
        || lowered.starts_with("why did")
        || lowered.starts_with("status")
        || lowered.contains("what went wrong")
        || lowered.contains("what is going on")
        || lowered.contains("did that run")
        || lowered.contains("did it run")
        || lowered.contains("did it work")
        || lowered.contains("is it working")
        || lowered.contains("did it fail")
        || lowered.contains("did that fail");
    if !status_frame {
        return false;
    }
    let tooling_reference = lowered.contains("web request")
        || lowered.contains("web tooling")
        || lowered.contains("web tool")
        || lowered.contains("web search")
        || lowered.contains("search request")
        || lowered.contains("tooling workflow")
        || lowered.contains("tool workflow")
        || lowered.contains("tool call")
        || lowered.contains("tool run")
        || lowered.contains("workflow run")
        || lowered.contains("last run")
        || lowered.contains("workspace analysis")
        || lowered.contains("workspace analyze")
        || lowered.contains("batch query")
        || lowered.contains("file tooling")
        || lowered.contains("tool routing")
        || lowered.contains("gate")
        || lowered.contains("gating")
        || lowered.contains("receipt");
    if !tooling_reference {
        return false;
    }
    let asks_fresh_query = lowered.contains("search for ")
        || lowered.contains("look up ")
        || lowered.contains("find information")
        || lowered.contains("about ")
        || lowered.contains("latest ")
        || lowered.contains("top ")
        || lowered.contains("best ")
        || lowered.contains("read file ")
        || lowered.contains("open file ")
        || lowered.contains("analyze ");
    !asks_fresh_query
}
