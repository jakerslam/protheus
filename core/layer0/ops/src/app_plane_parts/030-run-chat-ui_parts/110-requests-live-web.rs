fn chat_ui_requests_live_web(raw_input: &str) -> bool {
    if chat_ui_turn_is_meta_control_message(raw_input) {
        return false;
    }
    if chat_ui_message_is_tooling_status_check(raw_input) {
        return false;
    }
    let lowered = clean(raw_input, 2_000).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    if chat_ui_has_explicit_web_intent(&lowered) {
        return true;
    }
    if chat_ui_is_meta_diagnostic_request(&lowered) {
        return false;
    }
    ((lowered.contains("framework") || lowered.contains("frameworks"))
        && (lowered.contains("current")
            || lowered.contains("latest")
            || lowered.contains("top")
            || lowered.contains("best")))
        || (lowered.contains("search")
            && (lowered.contains("latest")
                || lowered.contains("current")
                || lowered.contains("framework")
                || lowered.contains("recipes")
                || lowered.contains("update")))
}
