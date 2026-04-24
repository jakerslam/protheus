fn rewrite_chat_ui_placeholder_with_tool_diagnostics(
    assistant: &str,
    diagnostics: &Value,
) -> (String, String) {
    let current = clean(assistant, 16_000);
    if current.is_empty() || !crate::tool_output_match_filter::matches_ack_placeholder(&current) {
        return (current, "unchanged".to_string());
    }
    let errors = diagnostics
        .get("error_codes")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let has_error = !errors.is_empty();
    let total_calls = diagnostics
        .get("total_calls")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let has_surface_unavailable = errors.contains_key("web_tool_surface_unavailable");
    let has_surface_degraded = errors.contains_key("web_tool_surface_degraded");
    let has_auth_missing = errors.contains_key("web_tool_auth_missing");
    let has_policy_blocked = errors.contains_key("web_tool_policy_blocked");
    let has_invalid_response = errors.contains_key("web_tool_invalid_response");
    let has_not_found = errors.contains_key("web_tool_not_found");
    let has_silent_failure = errors.contains_key("web_tool_silent_failure");

    if has_surface_unavailable {
        return ("".to_string(), "placeholder_withheld_surface_unavailable".to_string());
    }
    if has_surface_degraded {
        return ("".to_string(), "placeholder_withheld_surface_degraded".to_string());
    }
    if has_auth_missing {
        return ("".to_string(), "placeholder_withheld_auth".to_string());
    }
    if has_policy_blocked {
        return ("".to_string(), "placeholder_withheld_policy".to_string());
    }
    if has_invalid_response {
        return ("".to_string(), "placeholder_withheld_invalid_response".to_string());
    }
    if has_not_found {
        return ("".to_string(), "placeholder_withheld_not_found".to_string());
    }
    if has_silent_failure {
        return ("".to_string(), "placeholder_withheld_silent_failure".to_string());
    }
    if has_error {
        return ("".to_string(), "placeholder_withheld_error".to_string());
    }
    if total_calls > 0 {
        return ("".to_string(), "placeholder_withheld_low_signal".to_string());
    }
    (current, "unchanged".to_string())
}

fn chat_ui_contains_legacy_route_classifier_copy(text: &str) -> bool {
    let lowered = clean(text, 8_000).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    let route_classifier_template = lowered.contains("the first gate")
        && (lowered.contains("workflow_route") || lowered.contains("task_or_info_route"))
        && (lowered.contains("still classifying this as an \"info\" route rather than a \"task\" route")
            || lowered.contains("still classifying this as an 'info' route rather than a 'task' route")
            || lowered.contains("binary classification")
            || lowered.contains("task classification path"));
    let decision_tree_autoclassifier_template = lowered.contains("decision tree")
        && lowered.contains("automatically classifies")
        && lowered.contains("\"info\"")
        && lowered.contains("\"task\"")
        && lowered.contains("semantic analysis");
    route_classifier_template
        || decision_tree_autoclassifier_template
        || lowered.contains("[source:workflow_gate]")
        || lowered.contains("[source:tool_gate]")
        || lowered.contains("[source:tool_decision_tree_v3]")
        || lowered.contains("[source:workflow_route_classification]")
        || lowered.contains("[source:gate_enforcement_mode]")
        || lowered.contains("[source:tool_decision_policy]")
        || lowered.contains("[source:conversation_bypass_control]")
        || lowered.contains("[source:agent_framework_analysis]")
        || lowered.contains("source:workflow_route_classification")
        || lowered.contains("source:gate_enforcement_mode")
        || lowered.contains("source:tool_decision_policy")
        || lowered.contains("source:conversation_bypass_control")
        || lowered.contains("source:agent_framework_analysis")
        || lowered.contains("conversation bypass mode is currently active")
        || lowered.contains("restricted from running web searches")
        || lowered.contains("can't autonomously decide to use web tools")
        || lowered.contains("requires manual step-by-step authorization for tool usage")
}

fn rewrite_chat_ui_legacy_route_classifier_copy(assistant: &str) -> (String, String) {
    let current = clean(assistant, 16_000);
    if current.is_empty() || !chat_ui_contains_legacy_route_classifier_copy(&current) {
        return (current, "unchanged".to_string());
    }
    let mut rewritten = current;
    for marker in [
        "[source:workflow_gate]",
        "[source:tool_gate]",
        "[source:tool_decision_tree_v3]",
        "[source:workflow_route_classification]",
        "[source:gate_enforcement_mode]",
        "[source:tool_decision_policy]",
        "[source:conversation_bypass_control]",
        "[source:agent_framework_analysis]",
        "source:workflow_route_classification",
        "source:gate_enforcement_mode",
        "source:tool_decision_policy",
        "source:conversation_bypass_control",
        "source:agent_framework_analysis",
    ] {
        rewritten = rewritten.replace(marker, "");
    }
    let lowered = rewritten.to_ascii_lowercase();
    if chat_ui_contains_legacy_route_classifier_copy(&lowered) {
        return ("".to_string(), "legacy_route_classifier_copy_withheld".to_string());
    }
    (
        clean(&rewritten, 16_000),
        "legacy_route_classifier_copy_stripped".to_string(),
    )
}
