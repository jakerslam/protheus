fn chat_ui_has_explicit_web_intent(lowered: &str) -> bool {
    lowered.contains("web search")
        || lowered.contains("websearch")
        || lowered.contains("search the web")
        || lowered.contains("search online")
        || lowered.contains("find information")
        || lowered.contains("finding information")
        || lowered.contains("look it up")
        || lowered.contains("look this up")
        || lowered.contains("search again")
        || lowered.contains("best chili recipes")
}
