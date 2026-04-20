fn chat_ui_expected_classification_from_diagnostics(
    diagnostics: &Value,
    requires_live_web: bool,
    web_search_calls: i64,
) -> &'static str {
    let error_codes = diagnostics
        .get("error_codes")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let has_surface_unavailable = error_codes.contains_key("web_tool_surface_unavailable")
        || chat_ui_receipt_has_error_code(diagnostics, "web_tool_surface_unavailable");
    if has_surface_unavailable {
        return "tool_surface_unavailable";
    }
    let has_surface_degraded = error_codes.contains_key("web_tool_surface_degraded")
        || chat_ui_receipt_has_error_code(diagnostics, "web_tool_surface_degraded");
    if has_surface_degraded {
        return "tool_surface_degraded";
    }
    if requires_live_web && web_search_calls == 0 {
        return "tool_not_invoked";
    }
    let blocked_signal = error_codes.contains_key("web_tool_policy_blocked")
        || chat_ui_receipt_status_count(diagnostics, "blocked") > 0;
    if blocked_signal {
        return "policy_blocked";
    }
    let not_found_signal = error_codes.contains_key("web_tool_not_found")
        || chat_ui_receipt_status_count(diagnostics, "not_found") > 0;
    if not_found_signal {
        return "tool_not_found";
    }
    let loop_risk_signal = diagnostics
        .pointer("/loop_risk/detected")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if loop_risk_signal {
        return "low_signal";
    }
    let low_signal_signal = error_codes.contains_key("web_tool_low_signal")
        || chat_ui_receipt_status_count(diagnostics, "low_signal") > 0
        || diagnostics
            .get("no_result_calls")
            .and_then(Value::as_i64)
            .unwrap_or(0)
            > 0;
    if low_signal_signal {
        return "low_signal";
    }
    if requires_live_web {
        "healthy"
    } else {
        "not_required"
    }
}
