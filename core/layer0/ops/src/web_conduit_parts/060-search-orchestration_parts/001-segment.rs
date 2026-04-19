fn search_strip_invisible_unicode(raw: &str) -> String {
    raw.chars()
        .filter(|ch| {
            !matches!(
                *ch,
                '\u{200B}'..='\u{200F}'
                    | '\u{202A}'..='\u{202E}'
                    | '\u{2060}'..='\u{2064}'
                    | '\u{206A}'..='\u{206F}'
                    | '\u{FEFF}'
                    | '\u{E0000}'..='\u{E007F}'
            )
        })
        .collect()
}

fn search_invisible_unicode_removed_count(raw: &str) -> usize {
    let stripped = search_strip_invisible_unicode(raw);
    raw.chars().count().saturating_sub(stripped.chars().count())
}

fn normalize_search_query_url_candidate(raw: &str) -> String {
    let mut out = search_strip_invisible_unicode(&clean_text(raw, 2_200))
        .trim()
        .to_string();
    if out.starts_with('<') && out.ends_with('>') && out.len() > 2 {
        out = out[1..out.len() - 1].trim().to_string();
    }
    if ((out.starts_with('"') && out.ends_with('"'))
        || (out.starts_with('\'') && out.ends_with('\''))
        || (out.starts_with('`') && out.ends_with('`')))
        && out.len() > 1
    {
        out = out[1..out.len() - 1].trim().to_string();
    }
    while out.ends_with('.')
        || out.ends_with(',')
        || out.ends_with('!')
        || out.ends_with('?')
        || out.ends_with(';')
        || out.ends_with(':')
        || out.ends_with(')')
        || out.ends_with(']')
    {
        out.pop();
    }
    if out.starts_with("//") && out.len() > 2 {
        out = format!("https:{}", out);
    }
    out = out.replace("&amp;", "&");
    clean_text(&out, 2_100)
}

fn search_query_looks_like_bare_domain(raw: &str) -> bool {
    let candidate = clean_text(raw, 2_100).trim().to_ascii_lowercase();
    if candidate.is_empty() || candidate.contains(char::is_whitespace) {
        return false;
    }
    if candidate.starts_with("http://") || candidate.starts_with("https://") {
        return false;
    }
    let host = candidate
        .split('/')
        .next()
        .unwrap_or("")
        .split('?')
        .next()
        .unwrap_or("")
        .split('#')
        .next()
        .unwrap_or("");
    if host.is_empty() || !host.contains('.') {
        return false;
    }
    let labels = host.split('.').collect::<Vec<_>>();
    if labels.len() < 2 {
        return false;
    }
    let tld = labels.last().copied().unwrap_or("");
    if tld.len() < 2 || !tld.chars().all(|ch| ch.is_ascii_alphabetic()) {
        return false;
    }
    labels.iter().all(|label| {
        !label.is_empty()
            && label.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '-')
            && !label.starts_with('-')
            && !label.ends_with('-')
    })
}

fn search_extract_inline_http_url(raw: &str) -> Option<String> {
    for token in clean_text(raw, 2_200).split_whitespace() {
        let normalized = normalize_search_query_url_candidate(token);
        if (normalized.starts_with("http://") || normalized.starts_with("https://"))
            && !normalized.chars().any(|ch| ch.is_whitespace())
        {
            return Some(normalized);
        }
    }
    None
}

fn search_query_repetition_ratio(raw: &str) -> f64 {
    let lowered = clean_text(raw, 1_200).to_ascii_lowercase();
    let mut tokens = Vec::<String>::new();
    for token in lowered.split(|ch: char| !ch.is_ascii_alphanumeric()) {
        let trimmed = token.trim();
        if trimmed.len() >= 2 {
            tokens.push(trimmed.to_string());
        }
    }
    if tokens.len() < 6 {
        return 0.0;
    }
    let mut counts = std::collections::BTreeMap::<String, usize>::new();
    for token in tokens {
        let current = counts.get(&token).copied().unwrap_or(0);
        counts.insert(token, current.saturating_add(1));
    }
    let total = counts.values().sum::<usize>();
    if total == 0 {
        return 0.0;
    }
    let max_count = counts.values().copied().max().unwrap_or(0);
    (max_count as f64) / (total as f64)
}

fn search_query_fetch_url_candidate_with_kind(query: &str) -> Option<(String, &'static str)> {
    let cleaned = clean_text(query, 2_200);
    let trimmed = cleaned.trim();
    if trimmed.is_empty() {
        return None;
    }
    let direct_candidate = normalize_search_query_url_candidate(trimmed);
    if direct_candidate.starts_with("https://") && trimmed.trim_start().starts_with("//") {
        return Some((direct_candidate, "protocol_relative"));
    }
    if (direct_candidate.starts_with("http://") || direct_candidate.starts_with("https://"))
        && !direct_candidate.chars().any(|ch| ch.is_whitespace())
    {
        return Some((direct_candidate, "direct_url"));
    }
    if trimmed.contains(char::is_whitespace)
        && let Some(candidate) = search_extract_inline_http_url(trimmed)
    {
        return Some((candidate, "inline_url"));
    }
    if direct_candidate.starts_with("www.") && !direct_candidate.chars().any(|ch| ch.is_whitespace()) {
        return Some((format!("https://{}", direct_candidate), "www_domain"));
    }
    if search_query_looks_like_bare_domain(&direct_candidate) {
        return Some((format!("https://{}", direct_candidate), "bare_domain"));
    }
    if trimmed.starts_with('[') && trimmed.contains("](") && trimmed.ends_with(')') {
        if let (Some(open_idx), Some(close_idx)) = (trimmed.rfind('('), trimmed.rfind(')')) {
            if close_idx > open_idx + 1 {
                let candidate =
                    normalize_search_query_url_candidate(trimmed[open_idx + 1..close_idx].trim());
                if (candidate.starts_with("http://") || candidate.starts_with("https://"))
                    && !candidate.chars().any(|ch| ch.is_whitespace())
                {
                    return Some((candidate, "markdown_link"));
                }
            }
        }
    }
    None
}

fn search_query_fetch_url_candidate(query: &str) -> Option<String> {
    search_query_fetch_url_candidate_with_kind(query).map(|row| row.0)
}

fn search_query_fetch_url_candidate_kind(query: &str) -> &'static str {
    search_query_fetch_url_candidate_with_kind(query)
        .map(|row| row.1)
        .unwrap_or("none")
}

fn search_query_shape_error_code(query: &str) -> &'static str {
    let lowered = search_strip_invisible_unicode(&clean_text(query, 1_200)).to_ascii_lowercase();
    let trimmed = lowered.trim();
    if trimmed.is_empty() {
        return "query_required";
    }
    if trimmed.contains("<html")
        || trimmed.contains("</html>")
        || trimmed.contains("<body")
        || trimmed.contains("sample input:")
        || trimmed.contains("sample output:")
    {
        return "query_payload_dump_detected";
    }
    if (trimmed.starts_with('{') && trimmed.contains(':'))
        || (trimmed.starts_with('[') && trimmed.contains('{'))
        || trimmed.starts_with("\"query\"")
    {
        return "query_payload_dump_detected";
    }
    if lowered.contains("```")
        || lowered.contains("diff --git")
        || lowered.contains("[patch v")
        || lowered.contains("input specification")
        || lowered.contains("sample output")
        || lowered.contains("you are an expert")
    {
        return "query_payload_dump_detected";
    }
    if search_query_fetch_url_candidate(trimmed).is_some() {
        return "query_prefers_fetch_url";
    }
    let line_count = lowered.lines().count();
    if line_count > 8 || lowered.len() > 520 {
        return "query_shape_invalid";
    }
    let mut total_terms = 0usize;
    let mut unique = Vec::<&str>::new();
    for token in lowered.split(|ch: char| !ch.is_ascii_alphanumeric()) {
        let trimmed = token.trim();
        if trimmed.len() < 2 {
            continue;
        }
        total_terms += 1;
        if !unique.iter().any(|existing| *existing == trimmed) {
            unique.push(trimmed);
        }
    }
    if total_terms >= 7 && unique.len() <= 1 {
        return "query_shape_invalid";
    }
    let repetition_ratio = search_query_repetition_ratio(trimmed);
    if total_terms >= 8 && repetition_ratio >= 0.65 {
        return "query_shape_repetitive_loop";
    }
    let url_count = trimmed.match_indices("http://").count() + trimmed.match_indices("https://").count();
    if url_count > 1 {
        return "query_shape_invalid";
    }
    "none"
}

fn search_query_shape_invalid(query: &str) -> bool {
    search_query_shape_error_code(query) != "none"
}

fn search_query_shape_category(reason: &str) -> &'static str {
    match reason {
        "query_required" => "missing_input",
        "query_payload_dump_detected" => "payload_dump",
        "query_prefers_fetch_url" => "prefers_fetch",
        "query_shape_repetitive_loop" => "repetition_loop",
        "query_shape_invalid" => "invalid_shape",
        _ => "none",
    }
}

fn search_query_shape_recommended_action(reason: &str) -> &'static str {
    match reason {
        "query_required" => "provide a concise search query describing what to find",
        "query_payload_dump_detected" => {
            "replace pasted logs/pages with a short web intent and 2-8 focused keywords"
        }
        "query_prefers_fetch_url" => {
            "input is a direct URL; use web fetch action for page retrieval instead of search"
        }
        "query_shape_repetitive_loop" => {
            "query appears repetitive; replace repeated terms with 2-8 specific keywords"
        }
        "query_shape_invalid" => {
            "rewrite query as one concise sentence (recommended <= 300 chars)"
        }
        _ => "none",
    }
}

fn search_query_shape_route_hint(reason: &str) -> &'static str {
    if reason == "query_prefers_fetch_url" {
        "web_fetch"
    } else {
        "web_search"
    }
}

fn search_retry_strategy_for_error(error: &str) -> &'static str {
    if error == "query_prefers_fetch_url" {
        "use_web_fetch_route"
    } else if error == "non_search_meta_query" {
        "answer_directly_without_web_search"
    } else if error == "query_required" {
        "provide_query_text"
    } else if error == "conflicting_time_filters" {
        "remove_conflicting_time_filters"
    } else if error == "unknown_search_provider" {
        "use_supported_provider_or_auto"
    } else if error == "unsupported_search_filter" {
        "remove_unsupported_filter"
    } else if error == "query_shape_repetitive_loop" {
        "rewrite_without_repetition"
    } else {
        "rewrite_query_shape"
    }
}

fn search_retry_lane_for_error(error: &str) -> &'static str {
    if error == "query_prefers_fetch_url" {
        "web_fetch"
    } else {
        "web_search"
    }
}

fn search_retry_reason_for_error(error: &str) -> &'static str {
    if error == "query_prefers_fetch_url"
        || error == "non_search_meta_query"
        || error == "query_required"
        || error == "conflicting_time_filters"
        || error == "unknown_search_provider"
        || error == "unsupported_search_filter"
        || error == "query_shape_repetitive_loop"
    {
        error
    } else {
        "request_contract_adjustment_required"
    }
}

fn search_retry_category_for_error(error: &str) -> &'static str {
    if error == "query_prefers_fetch_url" || error == "query_required" {
        "input_contract"
    } else if error == "non_search_meta_query" {
        "intent_contract"
    } else if error == "conflicting_time_filters" || error == "unsupported_search_filter" {
        "filter_contract"
    } else if error == "unknown_search_provider" {
        "provider_contract"
    } else if error == "web_search_duplicate_attempt_suppressed" {
        "replay_guard"
    } else if error.starts_with("web_search_tool_surface_") {
        "tool_surface"
    } else {
        "request_contract"
    }
}

fn search_retry_recovery_mode_for_error(error: &str) -> &'static str {
    if error == "query_prefers_fetch_url" {
        "reroute_fetch"
    } else if error == "non_search_meta_query" {
        "answer_directly"
    } else if error == "conflicting_time_filters" || error == "unsupported_search_filter" {
        "adjust_filters"
    } else if error == "unknown_search_provider" {
        "switch_provider"
    } else if error == "web_search_duplicate_attempt_suppressed" {
        "adjust_query_or_provider"
    } else if error.starts_with("web_search_tool_surface_") {
        "restore_tool_surface"
    } else {
        "adjust_request"
    }
}

fn search_retry_priority_for_error(error: &str) -> &'static str {
    if error == "query_required" || error.starts_with("web_search_tool_surface_") {
        "high"
    } else if error == "non_search_meta_query" {
        "low"
    } else {
        "medium"
    }
}

fn search_retry_operator_action_hint_for_error(error: &str) -> &'static str {
    if error == "query_prefers_fetch_url" {
        "invoke_web_fetch_with_requested_url"
    } else if error == "non_search_meta_query" {
        "answer_without_web_search_or_set_force_web_search"
    } else if error == "query_required" {
        "provide_non_empty_query"
    } else if error == "conflicting_time_filters" {
        "remove_freshness_or_date_range_conflict"
    } else if error == "unknown_search_provider" {
        "set_provider_auto_or_supported_provider"
    } else if error == "unsupported_search_filter" {
        "remove_or_replace_unsupported_filter"
    } else if error == "web_search_duplicate_attempt_suppressed" {
        "adjust_query_or_wait_for_retry_window"
    } else if error.starts_with("web_search_tool_surface_") {
        "restore_web_tool_surface_and_retry"
    } else {
        "adjust_search_request_and_retry"
    }
}

fn search_retry_operator_owner_for_error(error: &str) -> &'static str {
    if error == "query_prefers_fetch_url"
        || error == "non_search_meta_query"
        || error == "query_required"
        || error == "conflicting_time_filters"
        || error == "unsupported_search_filter"
    {
        "user"
    } else if error == "unknown_search_provider" {
        "operator"
    } else if error == "web_search_duplicate_attempt_suppressed"
        || error.starts_with("web_search_tool_surface_")
    {
        "system_operator"
    } else {
        "operator"
    }
}

fn search_retry_diagnostic_code_for_error(error: &str) -> &'static str {
    if error == "query_prefers_fetch_url" {
        "search_retry_query_prefers_fetch_url"
    } else if error == "non_search_meta_query" {
        "search_retry_non_search_meta_query"
    } else if error == "query_required" {
        "search_retry_query_required"
    } else if error == "conflicting_time_filters" {
        "search_retry_conflicting_time_filters"
    } else if error == "unknown_search_provider" {
        "search_retry_unknown_search_provider"
    } else if error == "unsupported_search_filter" {
        "search_retry_unsupported_search_filter"
    } else if error == "web_search_duplicate_attempt_suppressed" {
        "search_retry_duplicate_attempt_suppressed"
    } else if error.starts_with("web_search_tool_surface_") {
        "search_retry_tool_surface"
    } else {
        "search_retry_request_contract_adjustment_required"
    }
}

fn search_retry_blocking_kind_for_error(error: &str) -> &'static str {
    if error == "query_prefers_fetch_url"
        || error == "query_required"
        || error == "conflicting_time_filters"
        || error == "unsupported_search_filter"
    {
        "input_adjustment_required"
    } else if error == "non_search_meta_query" {
        "direct_answer_required"
    } else if error == "unknown_search_provider" {
        "provider_configuration_required"
    } else if error == "web_search_duplicate_attempt_suppressed" {
        "cooldown_required"
    } else if error.starts_with("web_search_tool_surface_") {
        "tool_surface_restore_required"
    } else {
        "none"
    }
}

fn search_retry_auto_retry_allowed_for_error(error: &str) -> bool {
    matches!(
        search_retry_blocking_kind_for_error(error),
        "provider_configuration_required" | "cooldown_required" | "none"
    )
}

fn search_retry_escalation_lane_for_error(error: &str) -> &'static str {
    match search_retry_blocking_kind_for_error(error) {
        "input_adjustment_required" | "direct_answer_required" => "user_input",
        "provider_configuration_required" => "operations",
        "cooldown_required" => "automation",
        "tool_surface_restore_required" => "platform",
        _ => "none",
    }
}

fn search_retry_requires_manual_confirmation_for_error(error: &str) -> bool {
    matches!(
        search_retry_blocking_kind_for_error(error),
        "input_adjustment_required"
            | "direct_answer_required"
            | "provider_configuration_required"
            | "tool_surface_restore_required"
    )
}

fn search_retry_execution_policy_for_error(error: &str) -> &'static str {
    let blocking_kind = search_retry_blocking_kind_for_error(error);
    if search_retry_requires_manual_confirmation_for_error(error) {
        "manual_gate_required"
    } else if blocking_kind == "cooldown_required" {
        "deferred_auto_retry"
    } else if search_retry_auto_retry_allowed_for_error(error) {
        "auto_retry"
    } else {
        "manual_gate_required"
    }
}

fn search_retry_manual_gate_reason_for_error(error: &str) -> &'static str {
    match search_retry_blocking_kind_for_error(error) {
        "input_adjustment_required" => "input_adjustment_required",
        "direct_answer_required" => "direct_answer_required",
        "provider_configuration_required" => "provider_configuration_required",
        "tool_surface_restore_required" => "tool_surface_restore_required",
        _ => "none",
    }
}

fn search_retry_requeue_strategy_for_error(error: &str) -> &'static str {
    match search_retry_execution_policy_for_error(error) {
        "auto_retry" => "immediate",
        "deferred_auto_retry" => "deferred",
        _ => "manual",
    }
}

fn search_retry_can_execute_without_human_for_error(error: &str) -> bool {
    matches!(
        search_retry_execution_policy_for_error(error),
        "auto_retry" | "deferred_auto_retry"
    )
}

fn search_retry_execution_window_for_error(error: &str, retry_after_seconds: i64) -> &'static str {
    match search_retry_requeue_strategy_for_error(error) {
        "immediate" => "now",
        "deferred" => {
            if retry_after_seconds.max(0) > 0 {
                "after_retry_after"
            } else {
                "deferred"
            }
        }
        _ => "after_manual_gate",
    }
}

fn search_retry_manual_gate_timeout_seconds_for_error(error: &str) -> i64 {
    match search_retry_manual_gate_reason_for_error(error) {
        "input_adjustment_required" => 1800,
        "direct_answer_required" => 900,
        "provider_configuration_required" => 3600,
        "tool_surface_restore_required" => 1200,
        _ => 0,
    }
}

fn search_retry_next_action_after_seconds_for_error(error: &str, retry_after_seconds: i64) -> i64 {
    match search_retry_execution_window_for_error(error, retry_after_seconds) {
        "now" => 0,
        "after_retry_after" => retry_after_seconds.max(0),
        "deferred" => 60,
        _ => search_retry_manual_gate_timeout_seconds_for_error(error),
    }
}

fn search_retry_next_action_kind_for_error(error: &str, retry_after_seconds: i64) -> &'static str {
    if !search_retry_can_execute_without_human_for_error(error) {
        "manual_gate"
    } else if search_retry_next_action_after_seconds_for_error(error, retry_after_seconds) > 0 {
        "deferred_retry"
    } else {
        "execute_now"
    }
}

fn search_retry_window_class_for_error(error: &str, retry_after_seconds: i64) -> &'static str {
    let wait_seconds = search_retry_next_action_after_seconds_for_error(error, retry_after_seconds);
    if wait_seconds <= 0 {
        "immediate"
    } else if wait_seconds <= 60 {
        "short"
    } else if wait_seconds <= 900 {
        "medium"
    } else {
        "long"
    }
}

fn search_retry_readiness_state_for_error(error: &str, retry_after_seconds: i64) -> &'static str {
    if !search_retry_can_execute_without_human_for_error(error) {
        "manual_gate_pending"
    } else if search_retry_next_action_after_seconds_for_error(error, retry_after_seconds) > 0 {
        "deferred_retry_pending"
    } else {
        "ready_now"
    }
}

fn search_retry_readiness_reason_for_error(error: &str, retry_after_seconds: i64) -> &'static str {
    match search_retry_next_action_kind_for_error(error, retry_after_seconds) {
        "manual_gate" => search_retry_manual_gate_reason_for_error(error),
        "deferred_retry" => {
            if retry_after_seconds.max(0) > 0 {
                "retry_after_pending"
            } else {
                "deferred_retry_pending"
            }
        }
        _ => "none",
    }
}

fn search_retry_automation_safe_for_error(error: &str) -> bool {
    search_retry_auto_retry_allowed_for_error(error)
        && search_retry_can_execute_without_human_for_error(error)
}

fn search_retry_decision_route_hint_for_error(error: &str, retry_after_seconds: i64) -> &'static str {
    match search_retry_next_action_kind_for_error(error, retry_after_seconds) {
        "manual_gate" => "manual_review_lane",
        "deferred_retry" => "deferred_retry_lane",
        _ => "auto_execute_lane",
    }
}

fn search_retry_decision_urgency_tier_for_error(error: &str, retry_after_seconds: i64) -> &'static str {
    if !search_retry_automation_safe_for_error(error) {
        return "manual";
    }
    match search_retry_window_class_for_error(error, retry_after_seconds) {
        "immediate" => "high",
        "short" => "medium",
        "medium" => "low",
        _ => "deferred",
    }
}

fn search_retry_decision_retry_budget_class_for_error(
    error: &str,
    retry_after_seconds: i64,
) -> &'static str {
    if !search_retry_automation_safe_for_error(error) {
        return "manual_only";
    }
    match search_retry_window_class_for_error(error, retry_after_seconds) {
        "immediate" => "single_attempt",
        "short" => "bounded_backoff_short",
        "medium" => "bounded_backoff_medium",
        _ => "bounded_backoff_long",
    }
}

fn search_retry_decision_lane_token_for_error(error: &str, retry_after_seconds: i64) -> String {
    let route_hint = search_retry_decision_route_hint_for_error(error, retry_after_seconds);
    let urgency_tier = search_retry_decision_urgency_tier_for_error(error, retry_after_seconds);
    format!("{}::{}", route_hint, urgency_tier)
}

fn search_retry_decision_dispatch_mode_for_error(
    error: &str,
    retry_after_seconds: i64,
) -> &'static str {
    match search_retry_next_action_kind_for_error(error, retry_after_seconds) {
        "manual_gate" => "manual_review",
        "deferred_retry" => "scheduled_retry",
        _ => "immediate_execute",
    }
}

fn search_retry_decision_manual_ack_required_for_error(
    error: &str,
    retry_after_seconds: i64,
) -> bool {
    !search_retry_automation_safe_for_error(error)
        || search_retry_next_action_kind_for_error(error, retry_after_seconds) == "manual_gate"
}

fn search_retry_decision_execution_guard_for_error(
    error: &str,
    retry_after_seconds: i64,
) -> &'static str {
    if search_retry_decision_manual_ack_required_for_error(error, retry_after_seconds) {
        "manual_gate_guard"
    } else if search_retry_next_action_kind_for_error(error, retry_after_seconds) == "deferred_retry" {
        "retry_window_guard"
    } else {
        "none"
    }
}

fn search_retry_decision_followup_required_for_error(
    error: &str,
    retry_after_seconds: i64,
) -> bool {
    search_retry_next_action_kind_for_error(error, retry_after_seconds) != "execute_now"
}

fn search_retry_decision_vector_key_for_error(error: &str, retry_after_seconds: i64) -> String {
    let next_action_after_seconds =
        search_retry_next_action_after_seconds_for_error(error, retry_after_seconds).max(0);
    let next_action_kind = search_retry_next_action_kind_for_error(error, retry_after_seconds);
    let retry_window_class = search_retry_window_class_for_error(error, retry_after_seconds);
    let readiness_state = search_retry_readiness_state_for_error(error, retry_after_seconds);
    let readiness_reason = search_retry_readiness_reason_for_error(error, retry_after_seconds);
    let automation_safe = if search_retry_automation_safe_for_error(error) {
        "1"
    } else {
        "0"
    };
    format!(
        "{}|{}|{}|{}|{}|{}",
        next_action_kind,
        retry_window_class,
        readiness_state,
        readiness_reason,
        automation_safe,
        next_action_after_seconds
    )
}

fn search_retry_decision_vector_for_error(error: &str, retry_after_seconds: i64) -> Value {
    let next_action_after_seconds =
        search_retry_next_action_after_seconds_for_error(error, retry_after_seconds).max(0);
    let next_action_kind = search_retry_next_action_kind_for_error(error, retry_after_seconds);
    let retry_window_class = search_retry_window_class_for_error(error, retry_after_seconds);
    let readiness_state = search_retry_readiness_state_for_error(error, retry_after_seconds);
    let readiness_reason = search_retry_readiness_reason_for_error(error, retry_after_seconds);
    let automation_safe = search_retry_automation_safe_for_error(error);
    let route_hint = search_retry_decision_route_hint_for_error(error, retry_after_seconds);
    let urgency_tier = search_retry_decision_urgency_tier_for_error(error, retry_after_seconds);
    let retry_budget_class =
        search_retry_decision_retry_budget_class_for_error(error, retry_after_seconds);
    let lane_token = search_retry_decision_lane_token_for_error(error, retry_after_seconds);
    let dispatch_mode = search_retry_decision_dispatch_mode_for_error(error, retry_after_seconds);
    let manual_ack_required =
        search_retry_decision_manual_ack_required_for_error(error, retry_after_seconds);
    let execution_guard =
        search_retry_decision_execution_guard_for_error(error, retry_after_seconds);
    let followup_required =
        search_retry_decision_followup_required_for_error(error, retry_after_seconds);
    let decision_vector_key =
        search_retry_decision_vector_key_for_error(error, retry_after_seconds);
    json!({
        "next_action_after_seconds": next_action_after_seconds,
        "next_action_kind": next_action_kind,
        "retry_window_class": retry_window_class,
        "readiness_state": readiness_state,
        "readiness_reason": readiness_reason,
        "automation_safe": automation_safe,
        "route_hint": route_hint,
        "urgency_tier": urgency_tier,
        "retry_budget_class": retry_budget_class,
        "lane_token": lane_token,
        "dispatch_mode": dispatch_mode,
        "manual_ack_required": manual_ack_required,
        "execution_guard": execution_guard,
        "followup_required": followup_required,
        "decision_vector_version": "v1",
        "decision_vector_key": decision_vector_key
    })
}

fn search_retry_envelope_for_error(error: &str) -> Value {
    search_retry_envelope_runtime(
        search_retry_strategy_for_error(error),
        search_retry_reason_for_error(error),
        search_retry_lane_for_error(error),
        0,
    )
}

fn search_parse_nonnegative_i64(value: Option<&Value>) -> i64 {
    let Some(value) = value else {
        return 0;
    };
    if let Some(raw) = value.as_i64() {
        return raw.max(0);
    }
    if let Some(raw) = value.as_u64() {
        return raw.min(i64::MAX as u64) as i64;
    }
    if let Some(raw) = value.as_f64() {
        if raw.is_finite() {
            return raw.floor().max(0.0).min(i64::MAX as f64) as i64;
        }
    }
    if let Some(raw) = value.as_str() {
        let trimmed = clean_text(raw, 32);
        if !trimmed.is_empty()
            && let Ok(parsed) = trimmed.parse::<i64>()
        {
            return parsed.max(0);
        }
    }
    0
}

const SEARCH_RETRY_AFTER_SECONDS_MAX: i64 = 86_400;

fn search_retry_after_seconds_from_value(value: Option<&Value>) -> i64 {
    let raw = search_parse_nonnegative_i64(value);
    if raw <= 0 {
        return 0;
    }
    let now_epoch_seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs().min(i64::MAX as u64) as i64)
        .unwrap_or(0);
    let normalized = if now_epoch_seconds > 0 && raw > now_epoch_seconds {
        raw.saturating_sub(now_epoch_seconds)
    } else {
        raw
    };
    normalized.clamp(0, SEARCH_RETRY_AFTER_SECONDS_MAX)
}

fn search_retry_envelope_runtime(
    strategy: &str,
    reason: &str,
    lane: &str,
    retry_after_seconds: i64,
) -> Value {
    let decision_vector = search_retry_decision_vector_for_error(reason, retry_after_seconds);
    let next_action_after_seconds = decision_vector
        .get("next_action_after_seconds")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let next_action_kind = decision_vector
        .get("next_action_kind")
        .and_then(Value::as_str)
        .unwrap_or("execute_now");
    let retry_window_class = decision_vector
        .get("retry_window_class")
        .and_then(Value::as_str)
        .unwrap_or("immediate");
    let readiness_state = decision_vector
        .get("readiness_state")
        .and_then(Value::as_str)
        .unwrap_or("ready_now");
    let readiness_reason = decision_vector
        .get("readiness_reason")
        .and_then(Value::as_str)
        .unwrap_or("none");
    let automation_safe = decision_vector
        .get("automation_safe")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let decision_vector_key = decision_vector
        .get("decision_vector_key")
        .and_then(Value::as_str)
        .unwrap_or("none")
        .to_string();
    let decision_route_hint = decision_vector
        .get("route_hint")
        .and_then(Value::as_str)
        .unwrap_or("auto_execute_lane");
    let decision_urgency_tier = decision_vector
        .get("urgency_tier")
        .and_then(Value::as_str)
        .unwrap_or("deferred");
    let decision_retry_budget_class = decision_vector
        .get("retry_budget_class")
        .and_then(Value::as_str)
        .unwrap_or("manual_only");
    let decision_lane_token = decision_vector
        .get("lane_token")
        .and_then(Value::as_str)
        .unwrap_or("auto_execute_lane::deferred")
        .to_string();
    let decision_dispatch_mode = decision_vector
        .get("dispatch_mode")
        .and_then(Value::as_str)
        .unwrap_or("immediate_execute");
    let decision_manual_ack_required = decision_vector
        .get("manual_ack_required")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let decision_execution_guard = decision_vector
        .get("execution_guard")
        .and_then(Value::as_str)
        .unwrap_or("none");
    let decision_followup_required = decision_vector
        .get("followup_required")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let decision_vector_version = decision_vector
        .get("decision_vector_version")
        .and_then(Value::as_str)
        .unwrap_or("v1");
    json!({
        "recommended": true,
        "retryable": true,
        "idempotent": true,
        "contract_family": "web_retry_contract_v1",
        "strategy": strategy,
        "lane": lane,
        "reason": reason,
        "category": search_retry_category_for_error(reason),
        "recovery_mode": search_retry_recovery_mode_for_error(reason),
        "priority": search_retry_priority_for_error(reason),
        "operator_action_hint": search_retry_operator_action_hint_for_error(reason),
        "operator_owner": search_retry_operator_owner_for_error(reason),
        "diagnostic_code": search_retry_diagnostic_code_for_error(reason),
        "blocking_kind": search_retry_blocking_kind_for_error(reason),
        "auto_retry_allowed": search_retry_auto_retry_allowed_for_error(reason),
        "escalation_lane": search_retry_escalation_lane_for_error(reason),
        "requires_manual_confirmation": search_retry_requires_manual_confirmation_for_error(reason),
        "execution_policy": search_retry_execution_policy_for_error(reason),
        "manual_gate_reason": search_retry_manual_gate_reason_for_error(reason),
        "requeue_strategy": search_retry_requeue_strategy_for_error(reason),
        "can_execute_without_human": search_retry_can_execute_without_human_for_error(reason),
        "execution_window": search_retry_execution_window_for_error(reason, retry_after_seconds),
        "manual_gate_timeout_seconds": search_retry_manual_gate_timeout_seconds_for_error(reason),
        "next_action_after_seconds": next_action_after_seconds,
        "next_action_kind": next_action_kind,
        "retry_window_class": retry_window_class,
        "readiness_state": readiness_state,
        "readiness_reason": readiness_reason,
        "automation_safe": automation_safe,
        "decision_route_hint": decision_route_hint,
        "decision_urgency_tier": decision_urgency_tier,
        "decision_retry_budget_class": decision_retry_budget_class,
        "decision_lane_token": decision_lane_token,
        "decision_dispatch_mode": decision_dispatch_mode,
        "decision_manual_ack_required": decision_manual_ack_required,
        "decision_execution_guard": decision_execution_guard,
        "decision_followup_required": decision_followup_required,
        "decision_vector_version": decision_vector_version,
        "decision_vector_key": decision_vector_key,
        "decision_vector": decision_vector,
        "contract_version": "v1",
        "retry_after_seconds": retry_after_seconds.max(0)
    })
}

fn search_query_shape_suggested_next_action(query: &str, reason: &str) -> Value {
    if reason == "query_prefers_fetch_url" {
        let requested_url = search_query_fetch_url_candidate(query)
            .unwrap_or_else(|| clean_text(query, 2_200).trim().to_string());
        json!({
            "action": "web_conduit_fetch",
            "payload": {
                "requested_url": requested_url,
                "requested_url_input": clean_text(query, 2_200),
                "summary_only": true
            }
        })
    } else {
        Value::Null
    }
}

fn search_query_shape_contract(
    query: &str,
    reason: &str,
    override_used: bool,
    override_source: &str,
) -> Value {
    let cleaned = clean_text(query, 2_200);
    let stripped = search_strip_invisible_unicode(&cleaned);
    let invisible_unicode_removed_count =
        cleaned.chars().count().saturating_sub(stripped.chars().count()) as i64;
    let invisible_unicode_stripped = invisible_unicode_removed_count > 0;
    let fetch_url_candidate = search_query_fetch_url_candidate(&stripped).unwrap_or_default();
    let fetch_url_candidate_kind = search_query_fetch_url_candidate_kind(&stripped);
    json!({
        "blocked": reason != "none" && !override_used,
        "error": reason,
        "category": search_query_shape_category(reason),
        "recommended_action": search_query_shape_recommended_action(reason),
        "route_hint": search_query_shape_route_hint(reason),
        "suggested_next_action": search_query_shape_suggested_next_action(query, reason),
        "fetch_url_candidate": fetch_url_candidate,
        "fetch_url_candidate_kind": fetch_url_candidate_kind,
        "invisible_unicode_stripped": invisible_unicode_stripped,
        "invisible_unicode_removed_count": invisible_unicode_removed_count,
        "override_used": override_used,
        "override_source": override_source,
        "stats": search_query_shape_stats(&stripped)
    })
}

fn search_truthy_value(value: &Value) -> bool {
    value.as_bool().unwrap_or_else(|| {
        value
            .as_str()
            .map(|raw| {
                let lowered = clean_text(raw, 24).to_ascii_lowercase();
                matches!(lowered.as_str(), "1" | "true" | "yes" | "on")
            })
            .or_else(|| value.as_i64().map(|raw| raw != 0))
            .unwrap_or(false)
    })
}

fn search_query_shape_override(policy: &Value, request: &Value) -> bool {
    for key in [
        "/allow_query_blob_search",
        "/allowQueryBlobSearch",
        "/allow_query_shape_override",
        "/allowQueryShapeOverride",
        "/force_query_shape_override",
        "/forceQueryShapeOverride",
    ] {
        if let Some(value) = request.pointer(key) {
            if search_truthy_value(value) {
                return true;
            }
        }
    }
    for key in [
        "/web_conduit/search_policy/allow_query_blob_search",
        "/web_conduit/search_policy/allowQueryBlobSearch",
        "/web_conduit/search_policy/allow_query_shape_override",
        "/web_conduit/search_policy/allowQueryShapeOverride",
        "/web_conduit/search_policy/force_query_shape_override",
        "/web_conduit/search_policy/forceQueryShapeOverride",
    ] {
        if let Some(value) = policy.pointer(key) {
            if search_truthy_value(value) {
                return true;
            }
        }
    }
    false
}

fn search_query_shape_override_source(policy: &Value, request: &Value) -> &'static str {
    for key in [
        "/allow_query_blob_search",
        "/allowQueryBlobSearch",
        "/allow_query_shape_override",
        "/allowQueryShapeOverride",
        "/force_query_shape_override",
        "/forceQueryShapeOverride",
    ] {
        if let Some(value) = request.pointer(key) {
            if search_truthy_value(value) {
                return "request";
            }
        }
    }
    for key in [
        "/web_conduit/search_policy/allow_query_blob_search",
        "/web_conduit/search_policy/allowQueryBlobSearch",
        "/web_conduit/search_policy/allow_query_shape_override",
        "/web_conduit/search_policy/allowQueryShapeOverride",
        "/web_conduit/search_policy/force_query_shape_override",
        "/web_conduit/search_policy/forceQueryShapeOverride",
    ] {
        if let Some(value) = policy.pointer(key) {
            if search_truthy_value(value) {
                return "policy";
            }
        }
    }
    "none"
}

fn search_query_shape_stats(query: &str) -> Value {
    let cleaned = clean_text(query, 1_200).to_ascii_lowercase();
    let mut total_terms = 0usize;
    let mut unique = Vec::<String>::new();
    let mut term_counts = std::collections::BTreeMap::<String, usize>::new();
    for token in cleaned.split(|ch: char| !ch.is_ascii_alphanumeric()) {
        let trimmed = token.trim();
        if trimmed.is_empty() {
            continue;
        }
        total_terms += 1;
        if !unique.iter().any(|existing| existing == trimmed) {
            unique.push(trimmed.to_string());
        }
        let current = term_counts.get(trimmed).copied().unwrap_or(0);
        term_counts.insert(trimmed.to_string(), current.saturating_add(1));
    }
    let (dominant_term, dominant_term_count) = term_counts
        .iter()
        .max_by_key(|(_, count)| **count)
        .map(|(term, count)| (term.clone(), *count))
        .unwrap_or_else(|| (String::new(), 0usize));
    let repetition_ratio = if total_terms == 0 {
        0.0
    } else {
        (dominant_term_count as f64) / (total_terms as f64)
    };
    let fetch_url_candidate = search_query_fetch_url_candidate(query).unwrap_or_default();
    let fetch_url_candidate_kind = search_query_fetch_url_candidate_kind(query);
    json!({
        "line_count": cleaned.lines().count(),
        "char_count": cleaned.len(),
        "total_terms": total_terms,
        "unique_terms": unique.len(),
        "repetition_ratio": repetition_ratio,
        "dominant_term": dominant_term,
        "dominant_term_count": dominant_term_count,
        "url_candidate_detected": !fetch_url_candidate.is_empty(),
        "url_candidate": fetch_url_candidate,
        "url_candidate_kind": fetch_url_candidate_kind,
    })
}

fn search_query_source_kind(source: &str) -> &'static str {
    if source == "none" {
        "none"
    } else if source.starts_with("payload.request.") && source.contains('[') {
        "request_array_field"
    } else if source.starts_with("request.") && source.contains('[') {
        "request_array_field"
    } else if source.starts_with("payload.request.") || source.starts_with("request.") {
        "request_field"
    } else if source.starts_with("payload.") && source.contains('[') {
        "payload_array_field"
    } else if source.contains('[') {
        "array_field"
    } else if source.starts_with("payload.") {
        "payload_field"
    } else {
        "direct_field"
    }
}

fn search_query_source_confidence(source_kind: &str) -> &'static str {
    match source_kind {
        "none" => "none",
        "array_field" | "payload_array_field" | "request_array_field" => "medium",
        _ => "high",
    }
}

fn search_query_source_recovery_mode(source: &str) -> &'static str {
    if source == "none" {
        "none"
    } else if source.starts_with("query")
        || source.starts_with("q")
        || source.starts_with("search_query")
        || source.starts_with("searchQuery")
        || source.starts_with("prompt")
    {
        "direct"
    } else {
        "derived"
    }
}

fn search_query_source_lineage(source: &str, source_kind: &str, source_confidence: &str) -> Value {
    let normalized_source = clean_text(source, 180);
    let source_lane = if normalized_source.starts_with("payload.request.") {
        "payload_request"
    } else if normalized_source.starts_with("request.") {
        "request"
    } else if normalized_source.starts_with("payload.") {
        "payload"
    } else if normalized_source == "none" {
        "none"
    } else {
        "direct"
    };
    let path_depth = if normalized_source.is_empty() || normalized_source == "none" {
        0usize
    } else {
        normalized_source.split('.').count()
    };
    json!({
        "source": normalized_source,
        "kind": source_kind,
        "confidence": source_confidence,
        "lane": source_lane,
        "is_request_wrapped": source_lane == "request" || source_lane == "payload_request",
        "is_payload_wrapped": source_lane == "payload" || source_lane == "payload_request",
        "is_array_source": source.contains('['),
        "path_depth": path_depth
    })
}

fn search_query_and_source(request: &Value) -> (String, &'static str) {
    for (key, source) in [
        ("query", "query"),
        ("q", "q"),
        ("search_query", "search_query"),
        ("searchQuery", "searchQuery"),
        ("prompt", "prompt"),
        ("input", "input"),
        ("text", "text"),
        ("message", "message"),
        ("question", "question"),
    ] {
        let value = clean_text(request.get(key).and_then(Value::as_str).unwrap_or(""), 600);
        if !value.trim().is_empty() {
            return (value, source);
        }
    }
    for (pointer, source) in [
        ("/request/query", "request.query"),
        ("/request/q", "request.q"),
        ("/request/search_query", "request.search_query"),
        ("/request/searchQuery", "request.searchQuery"),
        ("/request/data/query", "request.data.query"),
        ("/request/data/q", "request.data.q"),
        ("/request/data/search_query", "request.data.search_query"),
        ("/request/data/searchQuery", "request.data.searchQuery"),
        ("/request/body/query", "request.body.query"),
        ("/request/body/q", "request.body.q"),
        ("/request/body/search_query", "request.body.search_query"),
        ("/request/body/searchQuery", "request.body.searchQuery"),
        ("/request/input", "request.input"),
        ("/request/text", "request.text"),
        ("/request/message", "request.message"),
        ("/request/question", "request.question"),
        ("/payload/request/query", "payload.request.query"),
        ("/payload/request/q", "payload.request.q"),
        ("/payload/request/search_query", "payload.request.search_query"),
        ("/payload/request/searchQuery", "payload.request.searchQuery"),
        ("/payload/request/data/query", "payload.request.data.query"),
        ("/payload/request/data/q", "payload.request.data.q"),
        (
            "/payload/request/data/search_query",
            "payload.request.data.search_query",
        ),
        (
            "/payload/request/data/searchQuery",
            "payload.request.data.searchQuery",
        ),
        ("/payload/request/body/query", "payload.request.body.query"),
        ("/payload/request/body/q", "payload.request.body.q"),
        ("/payload/request/body/search_query", "payload.request.body.search_query"),
        ("/payload/request/body/searchQuery", "payload.request.body.searchQuery"),
        ("/payload/request/input", "payload.request.input"),
        ("/payload/request/text", "payload.request.text"),
        ("/payload/request/message", "payload.request.message"),
        ("/payload/request/question", "payload.request.question"),
        ("/payload/query", "payload.query"),
        ("/payload/q", "payload.q"),
        ("/payload/search_query", "payload.search_query"),
        ("/payload/searchQuery", "payload.searchQuery"),
        ("/payload/data/query", "payload.data.query"),
        ("/payload/data/q", "payload.data.q"),
        ("/payload/data/search_query", "payload.data.search_query"),
        ("/payload/data/searchQuery", "payload.data.searchQuery"),
        ("/payload/body/query", "payload.body.query"),
        ("/payload/body/q", "payload.body.q"),
        ("/payload/body/search_query", "payload.body.search_query"),
        ("/payload/body/searchQuery", "payload.body.searchQuery"),
        ("/payload/input", "payload.input"),
        ("/payload/text", "payload.text"),
        ("/payload/message", "payload.message"),
        ("/payload/question", "payload.question"),
    ] {
        let value = clean_text(request.pointer(pointer).and_then(Value::as_str).unwrap_or(""), 600);
        if !value.trim().is_empty() {
            return (value, source);
        }
    }
    for (key, source) in [("queries", "queries[0]"), ("search_queries", "search_queries[0]")] {
        if let Some(value) = request
            .get(key)
            .and_then(Value::as_array)
            .and_then(|rows| rows.first())
            .and_then(Value::as_str)
        {
            let cleaned = clean_text(value, 600);
            if !cleaned.trim().is_empty() {
                return (cleaned, source);
            }
        }
    }
    for (pointer, source) in [
        ("/request/queries/0", "request.queries[0]"),
        ("/request/search_queries/0", "request.search_queries[0]"),
        ("/payload/request/queries/0", "payload.request.queries[0]"),
        (
            "/payload/request/search_queries/0",
            "payload.request.search_queries[0]",
        ),
    ] {
        let value = clean_text(request.pointer(pointer).and_then(Value::as_str).unwrap_or(""), 600);
        if !value.trim().is_empty() {
            return (value, source);
        }
    }
    for (pointer, source) in [
        ("/request/body/queries/0", "request.body.queries[0]"),
        (
            "/request/body/search_queries/0",
            "request.body.search_queries[0]",
        ),
        ("/payload/body/queries/0", "payload.body.queries[0]"),
        (
            "/payload/body/search_queries/0",
            "payload.body.search_queries[0]",
        ),
        (
            "/payload/request/body/queries/0",
            "payload.request.body.queries[0]",
        ),
        (
            "/payload/request/body/search_queries/0",
            "payload.request.body.search_queries[0]",
        ),
    ] {
        let value = clean_text(request.pointer(pointer).and_then(Value::as_str).unwrap_or(""), 600);
        if !value.trim().is_empty() {
            return (value, source);
        }
    }
    for (key, source_prefix) in [("queries", "queries"), ("search_queries", "search_queries")] {
        if let Some(first_row) = request
            .get(key)
            .and_then(Value::as_array)
            .and_then(|rows| rows.first())
        {
            for (field, source) in [
                ("query", "query"),
                ("q", "q"),
                ("text", "text"),
                ("prompt", "prompt"),
                ("input", "input"),
                ("message", "message"),
            ] {
                let value = clean_text(first_row.get(field).and_then(Value::as_str).unwrap_or(""), 600);
                if !value.trim().is_empty() {
                    let source_name = if source_prefix == "queries" {
                        match source {
                            "query" => "queries[0].query",
                            "q" => "queries[0].q",
                            "text" => "queries[0].text",
                            "prompt" => "queries[0].prompt",
                            "input" => "queries[0].input",
                            "message" => "queries[0].message",
                            _ => "queries[0]",
                        }
                    } else {
                        match source {
                            "query" => "search_queries[0].query",
                            "q" => "search_queries[0].q",
                            "text" => "search_queries[0].text",
                            "prompt" => "search_queries[0].prompt",
                            "input" => "search_queries[0].input",
                            "message" => "search_queries[0].message",
                            _ => "search_queries[0]",
                        }
                    };
                    return (value, source_name);
                }
            }
        }
    }
    for (pointer, source) in [
        ("/payload/queries/0", "payload.queries[0]"),
        ("/payload/search_queries/0", "payload.search_queries[0]"),
    ] {
        let value = clean_text(request.pointer(pointer).and_then(Value::as_str).unwrap_or(""), 600);
        if !value.trim().is_empty() {
            return (value, source);
        }
    }
    for (pointer, source_prefix) in [
        ("/payload/queries/0", "payload.queries[0]"),
        ("/payload/search_queries/0", "payload.search_queries[0]"),
    ] {
        for (field, source_suffix) in [
            ("query", ".query"),
            ("q", ".q"),
            ("text", ".text"),
            ("prompt", ".prompt"),
            ("input", ".input"),
            ("message", ".message"),
        ] {
            let pointer_with_field = format!("{}/{}", pointer, field);
            let value = clean_text(
                request.pointer(&pointer_with_field).and_then(Value::as_str).unwrap_or(""),
                600,
            );
            if !value.trim().is_empty() {
                let source_name = match (source_prefix, source_suffix) {
                    ("payload.queries[0]", ".query") => "payload.queries[0].query",
                    ("payload.queries[0]", ".q") => "payload.queries[0].q",
                    ("payload.queries[0]", ".text") => "payload.queries[0].text",
                    ("payload.queries[0]", ".prompt") => "payload.queries[0].prompt",
                    ("payload.queries[0]", ".input") => "payload.queries[0].input",
                    ("payload.queries[0]", ".message") => "payload.queries[0].message",
                    ("payload.search_queries[0]", ".query") => "payload.search_queries[0].query",
                    ("payload.search_queries[0]", ".q") => "payload.search_queries[0].q",
                    ("payload.search_queries[0]", ".text") => "payload.search_queries[0].text",
                    ("payload.search_queries[0]", ".prompt") => "payload.search_queries[0].prompt",
                    ("payload.search_queries[0]", ".input") => "payload.search_queries[0].input",
                    ("payload.search_queries[0]", ".message") => "payload.search_queries[0].message",
                    _ => "none",
                };
                if source_name != "none" {
                    return (value, source_name);
                }
            }
        }
    }
    for (pointer, source_prefix) in [
        ("/request/queries/0", "request.queries[0]"),
        ("/request/search_queries/0", "request.search_queries[0]"),
        ("/payload/request/queries/0", "payload.request.queries[0]"),
        (
            "/payload/request/search_queries/0",
            "payload.request.search_queries[0]",
        ),
    ] {
        for (field, source_suffix) in [
            ("query", ".query"),
            ("q", ".q"),
            ("text", ".text"),
            ("prompt", ".prompt"),
            ("input", ".input"),
            ("message", ".message"),
        ] {
            let pointer_with_field = format!("{}/{}", pointer, field);
            let value = clean_text(
                request.pointer(&pointer_with_field).and_then(Value::as_str).unwrap_or(""),
                600,
            );
            if !value.trim().is_empty() {
                let source_name = match (source_prefix, source_suffix) {
                    ("request.queries[0]", ".query") => "request.queries[0].query",
                    ("request.queries[0]", ".q") => "request.queries[0].q",
                    ("request.queries[0]", ".text") => "request.queries[0].text",
                    ("request.queries[0]", ".prompt") => "request.queries[0].prompt",
                    ("request.queries[0]", ".input") => "request.queries[0].input",
                    ("request.queries[0]", ".message") => "request.queries[0].message",
                    ("request.search_queries[0]", ".query") => "request.search_queries[0].query",
                    ("request.search_queries[0]", ".q") => "request.search_queries[0].q",
                    ("request.search_queries[0]", ".text") => "request.search_queries[0].text",
                    ("request.search_queries[0]", ".prompt") => "request.search_queries[0].prompt",
                    ("request.search_queries[0]", ".input") => "request.search_queries[0].input",
                    ("request.search_queries[0]", ".message") => "request.search_queries[0].message",
                    ("payload.request.queries[0]", ".query") => "payload.request.queries[0].query",
                    ("payload.request.queries[0]", ".q") => "payload.request.queries[0].q",
                    ("payload.request.queries[0]", ".text") => "payload.request.queries[0].text",
                    ("payload.request.queries[0]", ".prompt") => "payload.request.queries[0].prompt",
                    ("payload.request.queries[0]", ".input") => "payload.request.queries[0].input",
                    ("payload.request.queries[0]", ".message") => "payload.request.queries[0].message",
                    ("payload.request.search_queries[0]", ".query") => {
                        "payload.request.search_queries[0].query"
                    }
                    ("payload.request.search_queries[0]", ".q") => {
                        "payload.request.search_queries[0].q"
                    }
                    ("payload.request.search_queries[0]", ".text") => {
                        "payload.request.search_queries[0].text"
                    }
                    ("payload.request.search_queries[0]", ".prompt") => {
                        "payload.request.search_queries[0].prompt"
                    }
                    ("payload.request.search_queries[0]", ".input") => {
                        "payload.request.search_queries[0].input"
                    }
                    ("payload.request.search_queries[0]", ".message") => {
                        "payload.request.search_queries[0].message"
                    }
                    _ => "none",
                };
                if source_name != "none" {
                    return (value, source_name);
                }
            }
        }
    }
    for (pointer, source_prefix) in [
        ("/request/body/queries/0", "request.body.queries[0]"),
        (
            "/request/body/search_queries/0",
            "request.body.search_queries[0]",
        ),
        ("/payload/body/queries/0", "payload.body.queries[0]"),
        (
            "/payload/body/search_queries/0",
            "payload.body.search_queries[0]",
        ),
        (
            "/payload/request/body/queries/0",
            "payload.request.body.queries[0]",
        ),
        (
            "/payload/request/body/search_queries/0",
            "payload.request.body.search_queries[0]",
        ),
    ] {
        for (field, source_suffix) in [
            ("query", ".query"),
            ("q", ".q"),
            ("text", ".text"),
            ("prompt", ".prompt"),
            ("input", ".input"),
            ("message", ".message"),
            ("question", ".question"),
            ("search_query", ".search_query"),
            ("searchQuery", ".searchQuery"),
        ] {
            let pointer_with_field = format!("{}/{}", pointer, field);
            let value = clean_text(
                request.pointer(&pointer_with_field).and_then(Value::as_str).unwrap_or(""),
                600,
            );
            if !value.trim().is_empty() {
                let source_name = match (source_prefix, source_suffix) {
                    ("request.body.queries[0]", ".query") => "request.body.queries[0].query",
                    ("request.body.queries[0]", ".q") => "request.body.queries[0].q",
                    ("request.body.queries[0]", ".text") => "request.body.queries[0].text",
                    ("request.body.queries[0]", ".prompt") => "request.body.queries[0].prompt",
                    ("request.body.queries[0]", ".input") => "request.body.queries[0].input",
                    ("request.body.queries[0]", ".message") => "request.body.queries[0].message",
                    ("request.body.queries[0]", ".question") => "request.body.queries[0].question",
                    ("request.body.queries[0]", ".search_query") => {
                        "request.body.queries[0].search_query"
                    }
                    ("request.body.queries[0]", ".searchQuery") => {
                        "request.body.queries[0].searchQuery"
                    }
                    ("request.body.search_queries[0]", ".query") => {
                        "request.body.search_queries[0].query"
                    }
                    ("request.body.search_queries[0]", ".q") => "request.body.search_queries[0].q",
                    ("request.body.search_queries[0]", ".text") => {
                        "request.body.search_queries[0].text"
                    }
                    ("request.body.search_queries[0]", ".prompt") => {
                        "request.body.search_queries[0].prompt"
                    }
                    ("request.body.search_queries[0]", ".input") => {
                        "request.body.search_queries[0].input"
                    }
                    ("request.body.search_queries[0]", ".message") => {
                        "request.body.search_queries[0].message"
                    }
                    ("request.body.search_queries[0]", ".question") => {
                        "request.body.search_queries[0].question"
                    }
                    ("request.body.search_queries[0]", ".search_query") => {
                        "request.body.search_queries[0].search_query"
                    }
                    ("request.body.search_queries[0]", ".searchQuery") => {
                        "request.body.search_queries[0].searchQuery"
                    }
                    ("payload.body.queries[0]", ".query") => "payload.body.queries[0].query",
                    ("payload.body.queries[0]", ".q") => "payload.body.queries[0].q",
                    ("payload.body.queries[0]", ".text") => "payload.body.queries[0].text",
                    ("payload.body.queries[0]", ".prompt") => "payload.body.queries[0].prompt",
                    ("payload.body.queries[0]", ".input") => "payload.body.queries[0].input",
                    ("payload.body.queries[0]", ".message") => "payload.body.queries[0].message",
                    ("payload.body.queries[0]", ".question") => "payload.body.queries[0].question",
                    ("payload.body.queries[0]", ".search_query") => {
                        "payload.body.queries[0].search_query"
                    }
                    ("payload.body.queries[0]", ".searchQuery") => {
                        "payload.body.queries[0].searchQuery"
                    }
                    ("payload.body.search_queries[0]", ".query") => {
                        "payload.body.search_queries[0].query"
                    }
                    ("payload.body.search_queries[0]", ".q") => "payload.body.search_queries[0].q",
                    ("payload.body.search_queries[0]", ".text") => {
                        "payload.body.search_queries[0].text"
                    }
                    ("payload.body.search_queries[0]", ".prompt") => {
                        "payload.body.search_queries[0].prompt"
                    }
                    ("payload.body.search_queries[0]", ".input") => {
                        "payload.body.search_queries[0].input"
                    }
                    ("payload.body.search_queries[0]", ".message") => {
                        "payload.body.search_queries[0].message"
                    }
                    ("payload.body.search_queries[0]", ".question") => {
                        "payload.body.search_queries[0].question"
                    }
                    ("payload.body.search_queries[0]", ".search_query") => {
                        "payload.body.search_queries[0].search_query"
                    }
                    ("payload.body.search_queries[0]", ".searchQuery") => {
                        "payload.body.search_queries[0].searchQuery"
                    }
                    ("payload.request.body.queries[0]", ".query") => {
                        "payload.request.body.queries[0].query"
                    }
                    ("payload.request.body.queries[0]", ".q") => "payload.request.body.queries[0].q",
                    ("payload.request.body.queries[0]", ".text") => {
                        "payload.request.body.queries[0].text"
                    }
                    ("payload.request.body.queries[0]", ".prompt") => {
                        "payload.request.body.queries[0].prompt"
                    }
                    ("payload.request.body.queries[0]", ".input") => {
                        "payload.request.body.queries[0].input"
                    }
                    ("payload.request.body.queries[0]", ".message") => {
                        "payload.request.body.queries[0].message"
                    }
                    ("payload.request.body.queries[0]", ".question") => {
                        "payload.request.body.queries[0].question"
                    }
                    ("payload.request.body.queries[0]", ".search_query") => {
                        "payload.request.body.queries[0].search_query"
                    }
                    ("payload.request.body.queries[0]", ".searchQuery") => {
                        "payload.request.body.queries[0].searchQuery"
                    }
                    ("payload.request.body.search_queries[0]", ".query") => {
                        "payload.request.body.search_queries[0].query"
                    }
                    ("payload.request.body.search_queries[0]", ".q") => {
                        "payload.request.body.search_queries[0].q"
                    }
                    ("payload.request.body.search_queries[0]", ".text") => {
                        "payload.request.body.search_queries[0].text"
                    }
                    ("payload.request.body.search_queries[0]", ".prompt") => {
                        "payload.request.body.search_queries[0].prompt"
                    }
                    ("payload.request.body.search_queries[0]", ".input") => {
                        "payload.request.body.search_queries[0].input"
                    }
                    ("payload.request.body.search_queries[0]", ".message") => {
                        "payload.request.body.search_queries[0].message"
                    }
                    ("payload.request.body.search_queries[0]", ".question") => {
                        "payload.request.body.search_queries[0].question"
                    }
                    ("payload.request.body.search_queries[0]", ".search_query") => {
                        "payload.request.body.search_queries[0].search_query"
                    }
                    ("payload.request.body.search_queries[0]", ".searchQuery") => {
                        "payload.request.body.search_queries[0].searchQuery"
                    }
                    _ => "none",
                };
                if source_name != "none" {
                    return (value, source_name);
                }
            }
        }
    }
    for (pointer, source) in [
        ("/queries/0/question", "queries[0]"),
        ("/queries/0/search_query", "queries[0]"),
        ("/queries/0/searchQuery", "queries[0]"),
        ("/search_queries/0/question", "search_queries[0]"),
        ("/search_queries/0/search_query", "search_queries[0]"),
        ("/search_queries/0/searchQuery", "search_queries[0]"),
        ("/payload/queries/0/question", "payload.queries[0]"),
        ("/payload/queries/0/search_query", "payload.queries[0]"),
        ("/payload/queries/0/searchQuery", "payload.queries[0]"),
        ("/payload/search_queries/0/question", "payload.search_queries[0]"),
        (
            "/payload/search_queries/0/search_query",
            "payload.search_queries[0]",
        ),
        (
            "/payload/search_queries/0/searchQuery",
            "payload.search_queries[0]",
        ),
        ("/request/queries/0/question", "request.queries[0]"),
        ("/request/queries/0/search_query", "request.queries[0]"),
        ("/request/queries/0/searchQuery", "request.queries[0]"),
        (
            "/request/search_queries/0/question",
            "request.search_queries[0]",
        ),
        (
            "/request/search_queries/0/search_query",
            "request.search_queries[0]",
        ),
        (
            "/request/search_queries/0/searchQuery",
            "request.search_queries[0]",
        ),
        (
            "/payload/request/queries/0/question",
            "payload.request.queries[0]",
        ),
        (
            "/payload/request/queries/0/search_query",
            "payload.request.queries[0]",
        ),
        (
            "/payload/request/queries/0/searchQuery",
            "payload.request.queries[0]",
        ),
        (
            "/payload/request/search_queries/0/question",
            "payload.request.search_queries[0]",
        ),
        (
            "/payload/request/search_queries/0/search_query",
            "payload.request.search_queries[0]",
        ),
        (
            "/payload/request/search_queries/0/searchQuery",
            "payload.request.search_queries[0]",
        ),
    ] {
        let value = clean_text(request.pointer(pointer).and_then(Value::as_str).unwrap_or(""), 600);
        if !value.trim().is_empty() {
            return (value, source);
        }
    }
    (String::new(), "none")
}

pub fn api_search(root: &Path, request: &Value) -> Value {
    let (query_raw, query_source) = search_query_and_source(request);
    let query = search_strip_invisible_unicode(&clean_text(&query_raw, 600));
    let _query_invisible_unicode_removed_count =
        search_invisible_unicode_removed_count(&query_raw);
    let _query_invisible_unicode_stripped = _query_invisible_unicode_removed_count > 0;
    let query_source_kind = search_query_source_kind(query_source);
    let query_source_confidence = search_query_source_confidence(query_source_kind);
    let query_source_recovery_mode = search_query_source_recovery_mode(query_source);
    let query_source_lineage =
        search_query_source_lineage(query_source, query_source_kind, query_source_confidence);
    let provider_hint = clean_text(
        request
            .get("provider")
            .or_else(|| request.get("source"))
            .or_else(|| request.get("search_provider"))
            .or_else(|| request.get("searchProvider"))
            .and_then(Value::as_str)
            .unwrap_or("auto"),
        40,
    )
    .to_ascii_lowercase();
    let (policy, _policy_path_value) = load_policy(root);
    let query_shape_override = search_query_shape_override(&policy, request);
    let query_shape_override_source = search_query_shape_override_source(&policy, request);
    let query_shape_error = search_query_shape_error_code(&query);
    let query_shape_fetch_url_candidate = search_query_fetch_url_candidate(&query).unwrap_or_default();
    let query_shape_fetch_url_candidate_kind = search_query_fetch_url_candidate_kind(&query);
    if query_shape_error != "none" && !query_shape_override {
        let reason = query_shape_error;
        let receipt = build_receipt("", "deny", None, 0, reason, Some(reason));
        let _ = append_jsonl(&receipts_path(root), &receipt);
        let summary = if reason == "query_payload_dump_detected" {
            "Query looks like pasted output/log content instead of a concise web request. Submit a short intent-focused query."
        } else if reason == "query_prefers_fetch_url" {
            "Query is a direct URL. Use web fetch for this input instead of web search."
        } else {
            "Query shape is invalid for web search. Submit concise query text with clear keywords."
        };
        let mut out = search_early_validation_payload(
            reason,
            &query,
            Some(summary),
            &provider_hint,
            "skipped_validation",
            reason,
            reason,
            false,
            Some("submit concise query text (recommended <= 300 chars)"),
            receipt,
        );
        if let Some(obj) = out.as_object_mut() {
            obj.insert("query_shape_blocked".to_string(), json!(true));
            obj.insert("query_shape_error".to_string(), json!(reason));
            obj.insert("query_shape_stats".to_string(), search_query_shape_stats(&query));
            obj.insert(
                "query_shape_fetch_url_candidate".to_string(),
                json!(query_shape_fetch_url_candidate.clone()),
            );
            obj.insert(
                "query_shape_fetch_url_candidate_kind".to_string(),
                json!(query_shape_fetch_url_candidate_kind),
            );
            obj.insert("query_shape_override_allowed".to_string(), json!(false));
            obj.insert("query_shape_override_used".to_string(), json!(false));
            obj.insert(
                "query_shape_override_source".to_string(),
                json!(query_shape_override_source),
            );
            obj.insert(
                "query_shape_category".to_string(),
                json!(search_query_shape_category(reason)),
            );
            obj.insert(
                "query_shape_recommended_action".to_string(),
                json!(search_query_shape_recommended_action(reason)),
            );
            obj.insert(
                "query_shape_route_hint".to_string(),
                json!(search_query_shape_route_hint(reason)),
            );
            obj.insert(
                "query_shape".to_string(),
                search_query_shape_contract(&query, reason, false, query_shape_override_source),
            );
            obj.insert("query_source".to_string(), json!(query_source));
            obj.insert("query_source_kind".to_string(), json!(query_source_kind));
            obj.insert(
                "query_source_confidence".to_string(),
                json!(query_source_confidence),
            );
            obj.insert(
                "query_source_recovery_mode".to_string(),
                json!(query_source_recovery_mode),
            );
            obj.insert("query_source_lineage".to_string(), query_source_lineage.clone());
            obj.insert(
                "suggested_next_action".to_string(),
                search_query_shape_suggested_next_action(&query, reason),
            );
            obj.insert(
                "retry".to_string(),
                search_retry_envelope_for_error(reason),
            );
        }
        return out;
    }
    if let Some(mut early) = search_early_validation_response(root, request, &query) {
        let early_error = clean_text(
            early.get("error").and_then(Value::as_str).unwrap_or(""),
            120,
        );
        if let Some(obj) = early.as_object_mut() {
            obj.insert("query_source".to_string(), json!(query_source));
            obj.insert("query_source_kind".to_string(), json!(query_source_kind));
            obj.insert(
                "query_source_confidence".to_string(),
                json!(query_source_confidence),
            );
            obj.insert(
                "query_source_recovery_mode".to_string(),
                json!(query_source_recovery_mode),
            );
            obj.insert("query_source_lineage".to_string(), query_source_lineage.clone());
            obj.insert(
                "query_shape_fetch_url_candidate".to_string(),
                json!(query_shape_fetch_url_candidate.clone()),
            );
            obj.insert(
                "query_shape_fetch_url_candidate_kind".to_string(),
                json!(query_shape_fetch_url_candidate_kind),
            );
            obj.insert("query_shape_error".to_string(), json!(query_shape_error));
            obj.insert(
                "query_shape_category".to_string(),
                json!(search_query_shape_category(query_shape_error)),
            );
            obj.insert(
                "query_shape_recommended_action".to_string(),
                json!(search_query_shape_recommended_action(query_shape_error)),
            );
            obj.insert(
                "query_shape_route_hint".to_string(),
                json!(search_query_shape_route_hint(query_shape_error)),
            );
            obj.insert(
                "query_shape".to_string(),
                search_query_shape_contract(
                    &query,
                    query_shape_error,
                    query_shape_override,
                    query_shape_override_source,
                ),
            );
            obj.insert(
                "suggested_next_action".to_string(),
                search_query_shape_suggested_next_action(&query, query_shape_error),
            );
            obj.insert(
                "retry".to_string(),
                search_retry_envelope_for_error(&early_error),
            );
        }
        return early;
    }
    let normalized_filters = normalized_search_filters(request);
    let allowed_domains =
        normalize_allowed_domains(request.get("allowed_domains").unwrap_or(&Value::Null));
    let exclude_subdomains = request
        .get("exclude_subdomains")
        .or_else(|| request.get("exact_domain_only"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let raw_freshness = clean_text(
        request
            .get("freshness")
            .or_else(|| request.get("search_recency_filter"))
            .or_else(|| request.get("searchRecencyFilter"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        60,
    );
    let raw_date_after = clean_text(
        request
            .get("date_after")
            .or_else(|| request.get("dateAfter"))
            .or_else(|| request.get("search_after_date"))
            .or_else(|| request.get("searchAfterDate"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        40,
    );
    let raw_date_before = clean_text(
        request
            .get("date_before")
            .or_else(|| request.get("dateBefore"))
            .or_else(|| request.get("search_before_date"))
            .or_else(|| request.get("searchBeforeDate"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        40,
    );
    if !raw_freshness.is_empty() && (!raw_date_after.is_empty() || !raw_date_before.is_empty()) {
        let receipt = build_receipt(
            "",
            "deny",
            None,
            0,
            "conflicting_time_filters",
            Some("conflicting_time_filters"),
        );
        let _ = append_jsonl(&receipts_path(root), &receipt);
        return json!({
            "ok": false,
            "error": "conflicting_time_filters",
            "query": query,
            "query_source": query_source,
            "query_source_kind": query_source_kind,
            "query_source_confidence": query_source_confidence,
            "query_source_recovery_mode": query_source_recovery_mode,
            "query_source_lineage": query_source_lineage,
            "query_shape_fetch_url_candidate": query_shape_fetch_url_candidate,
            "query_shape_fetch_url_candidate_kind": query_shape_fetch_url_candidate_kind,
            "query_shape_error": query_shape_error,
            "query_shape_category": search_query_shape_category(query_shape_error),
            "query_shape_recommended_action": search_query_shape_recommended_action(query_shape_error),
            "query_shape_route_hint": search_query_shape_route_hint(query_shape_error),
            "query_shape": search_query_shape_contract(
                &query,
                query_shape_error,
                query_shape_override,
                query_shape_override_source
            ),
            "suggested_next_action": search_query_shape_suggested_next_action(&query, query_shape_error),
            "freshness": raw_freshness,
            "date_after": raw_date_after,
            "date_before": raw_date_before,
            "summary": "freshness cannot be combined with date_after/date_before. Use either freshness or an explicit date range.",
            "filters": normalized_filters.clone(),
            "provider_hint": provider_hint.clone(),
            "tool_execution_attempted": false,
            "tool_execution_gate": {
                "should_execute": false,
                "mode": "blocked",
                "reason": "conflicting_time_filters",
                "source": "request_contract"
            },
            "meta_query_blocked": false,
            "cache_status": "skipped_validation",
            "cache_store_allowed": false,
            "cache_write_attempted": false,
            "cache_skip_reason": "conflicting_time_filters",
            "retry": search_retry_envelope_for_error("conflicting_time_filters"),
            "provider_catalog": provider_catalog_snapshot(root, &policy),
            "process_summary": runtime_web_process_summary(
                "web_search",
                "request_contract_blocked",
                false,
                &json!({
                    "should_execute": false,
                    "mode": "blocked",
                    "reason": "conflicting_time_filters",
                    "source": "request_contract"
                }),
                &json!({
                    "blocked": false,
                    "reason": "not_evaluated"
                }),
                &json!([]),
                "none",
                Some("conflicting_time_filters")
            ),
            "receipt": receipt
        });
    }
    if let Some(unknown_provider) = validate_explicit_provider_hint(&provider_hint) {
        let receipt = build_receipt(
            "",
            "deny",
            None,
            0,
            "unknown_search_provider",
            Some(&unknown_provider),
        );
        let _ = append_jsonl(&receipts_path(root), &receipt);
        return json!({
            "ok": false,
            "error": "unknown_search_provider",
            "query": query,
            "query_source": query_source,
            "query_source_kind": query_source_kind,
            "query_source_confidence": query_source_confidence,
            "query_source_recovery_mode": query_source_recovery_mode,
            "query_source_lineage": query_source_lineage,
            "requested_provider": unknown_provider,
            "supported_filters": search_provider_request_contract(&policy)
                .get("supports_filters")
                .cloned()
                .unwrap_or_else(|| json!({})),
            "tool_execution_attempted": false,
            "tool_execution_gate": {
                "should_execute": false,
                "mode": "blocked",
                "reason": "unknown_search_provider",
                "source": "request_contract"
            },
            "meta_query_blocked": false,
            "cache_status": "skipped_validation",
            "cache_store_allowed": false,
            "cache_write_attempted": false,
            "cache_skip_reason": "unknown_search_provider",
            "retry": search_retry_envelope_for_error("unknown_search_provider"),
            "provider_catalog": provider_catalog_snapshot(root, &policy),
            "process_summary": runtime_web_process_summary(
                "web_search",
                "request_contract_blocked",
                false,
                &json!({
                    "should_execute": false,
                    "mode": "blocked",
                    "reason": "unknown_search_provider",
                    "source": "request_contract"
                }),
                &json!({
                    "blocked": false,
                    "reason": "not_evaluated"
                }),
                &json!([]),
                "none",
                Some("unknown_search_provider")
            ),
            "receipt": receipt
        });
    }
    if let Some(mut unsupported) = unsupported_search_filter_response(request) {
        let receipt = build_receipt(
            "",
            "deny",
            None,
            0,
            unsupported
                .get("error")
                .and_then(Value::as_str)
                .unwrap_or("unsupported_search_filter"),
            unsupported
                .get("unsupported_filter")
                .and_then(Value::as_str),
        );
        let _ = append_jsonl(&receipts_path(root), &receipt);
        if let Some(obj) = unsupported.as_object_mut() {
            obj.insert("query".to_string(), Value::String(query.clone()));
            obj.insert("query_source".to_string(), json!(query_source));
            obj.insert("query_source_kind".to_string(), json!(query_source_kind));
            obj.insert(
                "query_source_confidence".to_string(),
                json!(query_source_confidence),
            );
            obj.insert(
                "query_source_recovery_mode".to_string(),
                json!(query_source_recovery_mode),
            );
            obj.insert("query_source_lineage".to_string(), query_source_lineage.clone());
            obj.insert(
                "query_shape_fetch_url_candidate".to_string(),
                json!(query_shape_fetch_url_candidate.clone()),
            );
            obj.insert(
                "query_shape_fetch_url_candidate_kind".to_string(),
                json!(query_shape_fetch_url_candidate_kind),
            );
            obj.insert("query_shape_error".to_string(), json!(query_shape_error));
            obj.insert(
                "query_shape_category".to_string(),
                json!(search_query_shape_category(query_shape_error)),
            );
            obj.insert(
                "query_shape_recommended_action".to_string(),
                json!(search_query_shape_recommended_action(query_shape_error)),
            );
            obj.insert(
                "query_shape_route_hint".to_string(),
                json!(search_query_shape_route_hint(query_shape_error)),
            );
            obj.insert(
                "query_shape".to_string(),
                search_query_shape_contract(
                    &query,
                    query_shape_error,
                    query_shape_override,
                    query_shape_override_source,
                ),
            );
            obj.insert(
                "suggested_next_action".to_string(),
                search_query_shape_suggested_next_action(&query, query_shape_error),
            );
            obj.insert(
                "provider_hint".to_string(),
                Value::String(provider_hint.clone()),
            );
            obj.insert("filters".to_string(), normalized_filters.clone());
            obj.insert(
                "provider_catalog".to_string(),
                provider_catalog_snapshot(root, &policy),
            );
            obj.insert(
                "supported_filters".to_string(),
                search_provider_request_contract(&policy)
                    .get("supports_filters")
                    .cloned()
                    .unwrap_or_else(|| json!({})),
            );
            obj.insert("tool_execution_attempted".to_string(), json!(false));
            obj.insert(
                "tool_execution_gate".to_string(),
                json!({
                    "should_execute": false,
                    "mode": "blocked",
                    "reason": "unsupported_search_filter",
                    "source": "request_contract"
                }),
            );
            obj.insert("meta_query_blocked".to_string(), json!(false));
            obj.insert("cache_status".to_string(), json!("skipped_validation"));
            obj.insert("cache_store_allowed".to_string(), json!(false));
            obj.insert("cache_write_attempted".to_string(), json!(false));
            obj.insert("cache_skip_reason".to_string(), json!("unsupported_search_filter"));
            obj.insert(
                "retry".to_string(),
                search_retry_envelope_for_error("unsupported_search_filter"),
            );
            obj.insert(
                "process_summary".to_string(),
                runtime_web_process_summary(
                    "web_search",
                    "request_contract_blocked",
                    false,
                    &json!({
                        "should_execute": false,
                        "mode": "blocked",
                        "reason": "unsupported_search_filter",
                        "source": "request_contract"
                    }),
                    &json!({
                        "blocked": false,
                        "reason": "not_evaluated"
                    }),
                    &json!([]),
                    "none",
                    Some("unsupported_search_filter")
                ),
            );
            obj.insert("receipt".to_string(), receipt);
        }
        return unsupported;
    }
    let top_k = resolve_search_count(request, &policy);
    let timeout_ms = resolve_search_timeout_ms(request, &policy);
    let scoped_query = scoped_search_query(&query, &allowed_domains, exclude_subdomains);
    let summary_only = request
        .get("summary_only")
        .or_else(|| request.get("summary"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let human_approved = request
        .get("human_approved")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let approval_id = request
        .get("approval_id")
        .and_then(Value::as_str)
        .unwrap_or("");
    let (mut provider_resolution, provider_chain, selected_provider, allow_fallback) =
        resolved_search_provider_selection(root, &policy, request, &provider_hint);
    let cache_ttl_seconds = resolve_search_cache_ttl_seconds(request, &policy, "ok");
    let cache_ttl_minutes = if cache_ttl_seconds <= 0 {
        0
    } else {
        ((cache_ttl_seconds + 59) / 60) as u64
    };
    let cache_key = search_cache_key(
        &query,
        &scoped_query,
        &allowed_domains,
        exclude_subdomains,
        top_k,
        summary_only,
        &provider_chain,
    );
    if let Some(mut cached) = load_search_cache(root, &cache_key) {
        if let Some(obj) = cached.as_object_mut() {
            obj.insert(
                "type".to_string(),
                Value::String("web_conduit_search".to_string()),
            );
            obj.insert("query".to_string(), Value::String(query.clone()));
            obj.insert(
                "effective_query".to_string(),
                Value::String(scoped_query.clone()),
            );
            obj.insert("allowed_domains".to_string(), json!(allowed_domains));
            obj.insert("exclude_subdomains".to_string(), json!(exclude_subdomains));
            obj.insert("top_k".to_string(), json!(top_k));
            obj.insert("count".to_string(), json!(top_k));
            obj.insert("timeout_ms".to_string(), json!(timeout_ms));
            obj.insert("cache_ttl_minutes".to_string(), json!(cache_ttl_minutes));
            obj.insert("filters".to_string(), normalized_filters.clone());
            obj.insert(
                "provider_hint".to_string(),
                Value::String(provider_hint.clone()),
            );
            obj.insert("query_source".to_string(), json!(query_source));
            obj.insert("query_source_kind".to_string(), json!(query_source_kind));
            obj.insert("provider_chain".to_string(), json!(provider_chain));
            obj.insert(
                "provider_resolution".to_string(),
                provider_resolution.clone(),
            );
            obj.insert(
                "query_shape_override_used".to_string(),
                json!(query_shape_override),
            );
            obj.insert(
                "query_shape_override_source".to_string(),
                json!(query_shape_override_source),
            );
            obj.insert(
                "query_shape_stats".to_string(),
                search_query_shape_stats(&query),
            );
            obj.insert(
                "query_shape_fetch_url_candidate".to_string(),
                json!(query_shape_fetch_url_candidate.clone()),
            );
            obj.insert(
                "query_shape_fetch_url_candidate_kind".to_string(),
                json!(query_shape_fetch_url_candidate_kind),
            );
            obj.insert(
                "query_shape_error".to_string(),
                json!(query_shape_error),
            );
            obj.insert(
                "query_shape_category".to_string(),
                json!(search_query_shape_category(query_shape_error)),
            );
            obj.insert(
                "query_shape_recommended_action".to_string(),
                json!(search_query_shape_recommended_action(query_shape_error)),
            );
            obj.insert(
                "query_shape_route_hint".to_string(),
                json!(search_query_shape_route_hint(query_shape_error)),
            );
            obj.insert(
                "query_shape".to_string(),
                search_query_shape_contract(
                    &query,
                    query_shape_error,
                    query_shape_override,
                    query_shape_override_source,
                ),
            );
            obj.insert(
                "suggested_next_action".to_string(),
                search_query_shape_suggested_next_action(&query, query_shape_error),
            );
            obj.insert(
                "provider_health".to_string(),
                provider_health_snapshot(root, &provider_chain),
            );
            obj.insert("cache_status".to_string(), json!("hit"));
        }
        return cached;
    }
    let primary_url = web_search_url(&scoped_query);
    let lite_url = web_search_lite_url(&scoped_query);
    let mut selected = Value::Null;
    let initial_selected_provider = selected_provider.clone();
    let mut executed_provider = String::new();
    let mut attempted = Vec::<String>::new();
    let mut skipped = Vec::<Value>::new();
    let mut provider_errors = Vec::<Value>::new();
    let mut last_payload = None::<Value>;
    let tool_surface_status = provider_resolution
        .get("tool_surface_status")
        .or_else(|| provider_resolution.pointer("/tool_surface_health/status"))
        .and_then(Value::as_str)
        .unwrap_or("unavailable")
        .to_string();
    let tool_surface_ready = provider_resolution
        .get("tool_surface_ready")
        .or_else(|| {
            provider_resolution.pointer("/tool_surface_health/selected_provider_ready")
        })
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let tool_surface_blocking_reason = provider_resolution
        .pointer("/tool_surface_health/blocking_reason")
        .and_then(Value::as_str)
        .unwrap_or("none")
        .to_string();
    let tool_execution_gate = provider_resolution
        .get("tool_execution_gate")
        .cloned()
        .unwrap_or_else(|| {
            runtime_web_execution_gate(
                &tool_surface_status,
                tool_surface_ready,
                allow_fallback,
                &tool_surface_blocking_reason,
            )
        });
    let tool_execution_allowed = tool_execution_gate
        .get("should_execute")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let search_attempt_signature = sha256_hex(&format!(
        "{}|{}|{}|{}|{}|{}|{}",
        scoped_query,
        provider_chain.join(","),
        top_k,
        summary_only,
        timeout_ms,
        allow_fallback,
        tool_surface_status
    ));
    let replay_policy =
        runtime_web_replay_policy(&policy, request, &tool_surface_status, tool_surface_ready);
    let replay_window = replay_policy
        .get("window")
        .and_then(Value::as_u64)
        .unwrap_or(24)
        .clamp(1, 200) as usize;
    let replay_threshold = replay_policy
        .get("block_threshold")
        .and_then(Value::as_u64)
        .unwrap_or(3)
        .clamp(2, 200) as usize;
    let replay_enabled = replay_policy
        .get("enabled")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let replay_bypass = runtime_web_replay_bypass(&policy, request, human_approved);
    let replay_bypassed = replay_bypass
        .get("bypassed")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let replay_cooldown_base_seconds = replay_policy
        .get("cooldown_base_seconds")
        .and_then(Value::as_u64)
        .unwrap_or(30);
    let replay_cooldown_step_seconds = replay_policy
        .get("cooldown_step_seconds")
        .and_then(Value::as_u64)
        .unwrap_or(15);
    let replay_cooldown_max_seconds = replay_policy
        .get("cooldown_max_seconds")
        .and_then(Value::as_u64)
        .unwrap_or(180);
    let attempt_replay_guard =
        if replay_enabled && !replay_bypassed {
            recent_tool_attempt_replay_guard(
                root,
                &search_attempt_signature,
                replay_window,
                replay_threshold,
                replay_cooldown_base_seconds,
                replay_cooldown_step_seconds,
                replay_cooldown_max_seconds,
            )
        } else if replay_bypassed {
            runtime_web_replay_guard_passthrough(
                "replay_guard_bypassed",
                &search_attempt_signature,
                replay_window,
                replay_threshold,
                replay_cooldown_base_seconds,
                replay_cooldown_step_seconds,
                replay_cooldown_max_seconds,
                &replay_bypass,
            )
        } else {
            runtime_web_replay_guard_passthrough(
                "replay_policy_disabled",
                &search_attempt_signature,
                replay_window,
                replay_threshold,
                replay_cooldown_base_seconds,
                replay_cooldown_step_seconds,
                replay_cooldown_max_seconds,
                &replay_bypass,
            )
        };
    let attempt_replay_blocked = attempt_replay_guard
        .get("blocked")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let replay_retry_after_seconds = attempt_replay_guard.get("retry_after_seconds");
    let replay_retry_after_seconds =
        search_retry_after_seconds_from_value(replay_retry_after_seconds) as u64;
    let replay_retry_lane = clean_text(
        attempt_replay_guard
            .get("retry_lane")
            .and_then(Value::as_str)
            .unwrap_or("change_query_or_provider"),
        80,
    );
    if !tool_execution_allowed {
        let preflight_error = if tool_surface_status == "unavailable" {
            "web_search_tool_surface_unavailable"
        } else if tool_surface_status == "degraded" {
            "web_search_tool_surface_degraded"
        } else {
            "web_search_tool_execution_blocked"
        };
        let search_url = match selected_provider.as_str() {
            "duckduckgo_lite" => lite_url.clone(),
            "bing_rss" => web_search_bing_rss_url(&scoped_query),
            _ => primary_url.clone(),
        };
        let mut receipt = build_receipt(
            &search_url,
            "deny",
            None,
            0,
            "search_preflight_gate_blocked",
            Some(preflight_error),
        );
        if let Some(receipt_obj) = receipt.as_object_mut() {
            receipt_obj.insert(
                "attempt_signature".to_string(),
                Value::String(search_attempt_signature.clone()),
            );
            receipt_obj.insert(
                "gate_mode".to_string(),
                Value::String(
                    tool_execution_gate
                        .get("mode")
                        .and_then(Value::as_str)
                        .unwrap_or("blocked")
                        .to_string(),
                ),
            );
        }
        let _ = append_jsonl(&receipts_path(root), &receipt);
        let mut out = serde_json::Map::<String, Value>::new();
        out.insert("ok".to_string(), Value::Bool(false));
        out.insert("error".to_string(), Value::String(preflight_error.to_string()));
        out.insert(
            "summary".to_string(),
            Value::String(
                "Web search execution was blocked by runtime tooling gate before provider calls were attempted."
                    .to_string(),
            ),
        );
        out.insert("content".to_string(), Value::String(String::new()));
        out.insert(
            "type".to_string(),
            Value::String("web_conduit_search".to_string()),
        );
        out.insert("query".to_string(), Value::String(query.clone()));
        out.insert(
            "effective_query".to_string(),
            Value::String(scoped_query.clone()),
        );
        out.insert("allowed_domains".to_string(), json!(allowed_domains.clone()));
        out.insert("exclude_subdomains".to_string(), json!(exclude_subdomains));
        out.insert("top_k".to_string(), json!(top_k));
        out.insert("count".to_string(), json!(top_k));
        out.insert("timeout_ms".to_string(), json!(timeout_ms));
        out.insert(
            "provider_hint".to_string(),
            Value::String(provider_hint.clone()),
        );
        out.insert("query_source".to_string(), json!(query_source));
        out.insert("query_source_kind".to_string(), json!(query_source_kind));
        out.insert(
            "query_source_confidence".to_string(),
            json!(query_source_confidence),
        );
        out.insert(
            "query_source_recovery_mode".to_string(),
            json!(query_source_recovery_mode),
        );
        out.insert("query_source_lineage".to_string(), query_source_lineage.clone());
        out.insert(
            "query_shape_fetch_url_candidate".to_string(),
            json!(query_shape_fetch_url_candidate.clone()),
        );
        out.insert(
            "query_shape_fetch_url_candidate_kind".to_string(),
            json!(query_shape_fetch_url_candidate_kind),
        );
        out.insert("query_shape_error".to_string(), json!(query_shape_error));
        out.insert(
            "query_shape_category".to_string(),
            json!(search_query_shape_category(query_shape_error)),
        );
        out.insert(
            "query_shape_recommended_action".to_string(),
            json!(search_query_shape_recommended_action(query_shape_error)),
        );
        out.insert(
            "query_shape_route_hint".to_string(),
            json!(search_query_shape_route_hint(query_shape_error)),
        );
        out.insert(
            "query_shape".to_string(),
            search_query_shape_contract(
                &query,
                query_shape_error,
                query_shape_override,
                query_shape_override_source,
            ),
        );
        out.insert(
            "suggested_next_action".to_string(),
            search_query_shape_suggested_next_action(&query, query_shape_error),
        );
        out.insert("provider_chain".to_string(), json!(provider_chain.clone()));
        out.insert("provider_resolution".to_string(), provider_resolution);
        out.insert(
            "tool_surface_status".to_string(),
            Value::String(tool_surface_status.clone()),
        );
        out.insert("tool_surface_ready".to_string(), Value::Bool(tool_surface_ready));
        out.insert(
            "tool_surface_blocking_reason".to_string(),
            Value::String(tool_surface_blocking_reason.clone()),
        );
        out.insert("tool_execution_gate".to_string(), tool_execution_gate.clone());
        out.insert("replay_policy".to_string(), replay_policy.clone());
        out.insert("replay_bypass".to_string(), replay_bypass.clone());
        out.insert("attempt_replay_guard".to_string(), attempt_replay_guard.clone());
        out.insert(
            "attempt_signature".to_string(),
            Value::String(search_attempt_signature.clone()),
        );
        out.insert(
            "retry".to_string(),
            search_retry_envelope_runtime(
                if preflight_error == "web_search_tool_surface_unavailable" {
                    "restore_tool_surface_or_use_supported_provider"
                } else if preflight_error == "web_search_tool_surface_degraded" {
                    "stabilize_provider_runtime_and_retry"
                } else {
                    "change_query_or_provider"
                },
                preflight_error,
                &replay_retry_lane,
                replay_retry_after_seconds,
            ),
        );
        out.insert(
            "process_summary".to_string(),
            runtime_web_process_summary(
                "web_search",
                "preflight_blocked",
                false,
                &tool_execution_gate,
                &attempt_replay_guard,
                &json!(provider_chain.clone()),
                &selected_provider,
                Some(preflight_error),
            ),
        );
        out.insert("receipt".to_string(), receipt);
        return Value::Object(out);
    }
    if attempt_replay_blocked {
        let search_url = match selected_provider.as_str() {
            "duckduckgo_lite" => lite_url.clone(),
            "bing_rss" => web_search_bing_rss_url(&scoped_query),
            _ => primary_url.clone(),
        };
        let mut receipt = build_receipt(
            &search_url,
            "deny",
            None,
            0,
            "search_replay_guard_blocked",
            Some("web_search_duplicate_attempt_suppressed"),
        );
        if let Some(receipt_obj) = receipt.as_object_mut() {
            receipt_obj.insert(
                "attempt_signature".to_string(),
                Value::String(search_attempt_signature.clone()),
            );
            receipt_obj.insert(
                "gate_mode".to_string(),
                Value::String(
                    tool_execution_gate
                        .get("mode")
                        .and_then(Value::as_str)
                        .unwrap_or("blocked")
                        .to_string(),
                ),
            );
        }
        let _ = append_jsonl(&receipts_path(root), &receipt);
        let mut out = serde_json::Map::<String, Value>::new();
        out.insert("ok".to_string(), Value::Bool(false));
        out.insert(
            "error".to_string(),
            Value::String("web_search_duplicate_attempt_suppressed".to_string()),
        );
        out.insert(
            "summary".to_string(),
            Value::String(
                "Repeated identical web search attempts were suppressed by replay guard. Adjust the query or provider constraints before retrying."
                    .to_string(),
            ),
        );
        out.insert("content".to_string(), Value::String(String::new()));
        out.insert(
            "type".to_string(),
            Value::String("web_conduit_search".to_string()),
        );
        out.insert("query".to_string(), Value::String(query.clone()));
        out.insert(
            "effective_query".to_string(),
            Value::String(scoped_query.clone()),
        );
        out.insert("allowed_domains".to_string(), json!(allowed_domains.clone()));
        out.insert("exclude_subdomains".to_string(), json!(exclude_subdomains));
        out.insert("top_k".to_string(), json!(top_k));
        out.insert("count".to_string(), json!(top_k));
        out.insert("timeout_ms".to_string(), json!(timeout_ms));
        out.insert(
            "provider_hint".to_string(),
            Value::String(provider_hint.clone()),
        );
        out.insert("query_source".to_string(), json!(query_source));
        out.insert("query_source_kind".to_string(), json!(query_source_kind));
        out.insert(
            "query_source_confidence".to_string(),
            json!(query_source_confidence),
        );
        out.insert(
            "query_source_recovery_mode".to_string(),
            json!(query_source_recovery_mode),
        );
        out.insert("query_source_lineage".to_string(), query_source_lineage.clone());
        out.insert(
            "query_shape_fetch_url_candidate".to_string(),
            json!(query_shape_fetch_url_candidate.clone()),
        );
        out.insert(
            "query_shape_fetch_url_candidate_kind".to_string(),
            json!(query_shape_fetch_url_candidate_kind),
        );
        out.insert("query_shape_error".to_string(), json!(query_shape_error));
        out.insert(
            "query_shape_category".to_string(),
            json!(search_query_shape_category(query_shape_error)),
        );
        out.insert(
            "query_shape_recommended_action".to_string(),
            json!(search_query_shape_recommended_action(query_shape_error)),
        );
        out.insert(
            "query_shape_route_hint".to_string(),
            json!(search_query_shape_route_hint(query_shape_error)),
        );
        out.insert(
            "query_shape".to_string(),
            search_query_shape_contract(
                &query,
                query_shape_error,
                query_shape_override,
                query_shape_override_source,
            ),
        );
        out.insert(
            "suggested_next_action".to_string(),
            search_query_shape_suggested_next_action(&query, query_shape_error),
        );
        out.insert("provider_chain".to_string(), json!(provider_chain.clone()));
        out.insert("provider_resolution".to_string(), provider_resolution);
        out.insert(
            "tool_surface_status".to_string(),
            Value::String(tool_surface_status.clone()),
        );
        out.insert("tool_surface_ready".to_string(), Value::Bool(tool_surface_ready));
        out.insert(
            "tool_surface_blocking_reason".to_string(),
            Value::String(tool_surface_blocking_reason.clone()),
        );
        out.insert("tool_execution_gate".to_string(), tool_execution_gate.clone());
        out.insert("replay_policy".to_string(), replay_policy.clone());
        out.insert("replay_bypass".to_string(), replay_bypass.clone());
        out.insert("attempt_replay_guard".to_string(), attempt_replay_guard.clone());
        out.insert(
            "attempt_signature".to_string(),
            Value::String(search_attempt_signature.clone()),
        );
        out.insert(
            "retry".to_string(),
            search_retry_envelope_runtime(
                "narrow_query_or_wait_for_replay_window",
                "web_search_duplicate_attempt_suppressed",
                &replay_retry_lane,
                replay_retry_after_seconds,
            ),
        );
        out.insert(
            "process_summary".to_string(),
            runtime_web_process_summary(
                "web_search",
                "replay_suppressed",
                false,
                &tool_execution_gate,
                &attempt_replay_guard,
                &json!(provider_chain.clone()),
                &selected_provider,
                Some("web_search_duplicate_attempt_suppressed"),
            ),
        );
        out.insert("receipt".to_string(), receipt);
        return Value::Object(out);
    }

    for provider in &provider_chain {
        if let Some(open_until) = provider_circuit_open_until(root, provider, &policy) {
            skipped.push(json!({
                "provider": provider,
                "reason": "circuit_open",
                "open_until": open_until
            }));
            if !allow_fallback {
                last_payload = Some(json!({
                    "ok": false,
                    "error": "provider_circuit_open",
                    "summary": format!(
                        "Search provider \"{provider}\" is temporarily unavailable because its circuit breaker is open."
                    ),
                    "content": "",
                    "provider": provider,
                    "provider_unavailable_reason": "circuit_open",
                    "circuit_open_until": open_until
                }));
                break;
            }
            continue;
        }
        attempted.push(provider.clone());
        let candidate = match provider.as_str() {
            "serperdev" => api_search_serper(
                root,
                &scoped_query,
                summary_only,
                human_approved,
                &allowed_domains,
                exclude_subdomains,
                top_k,
                timeout_ms,
            ),
            "duckduckgo_lite" => api_fetch(
                root,
                &json!({
                    "url": lite_url,
                    "summary_only": summary_only,
                    "human_approved": human_approved,
                    "approval_id": approval_id,
                    "timeout_ms": timeout_ms
                }),
            ),
            "bing_rss" => api_search_bing_rss(
                &scoped_query,
                summary_only,
                &allowed_domains,
                exclude_subdomains,
                top_k,
                timeout_ms,
            ),
            _ => api_fetch(
                root,
                &json!({
                    "url": primary_url,
                    "summary_only": summary_only,
                    "human_approved": human_approved,
                    "approval_id": approval_id,
                    "timeout_ms": timeout_ms
                }),
            ),
        };
        if search_payload_usable_for_query(&candidate, &scoped_query) {
            record_provider_attempt(root, provider, true, "", &policy);
            executed_provider = provider.clone();
            selected = candidate;
            break;
        }
        let reason = search_payload_error_for_query(&candidate, &scoped_query);
        record_provider_attempt(root, provider, false, &reason, &policy);
        provider_errors.push(json!({
            "provider": provider,
            "error": reason,
            "challenge": payload_looks_like_search_challenge(&candidate),
            "low_signal": payload_looks_low_signal_search(&candidate),
            "query_mismatch": search_payload_query_mismatch(&candidate, &scoped_query),
            "status_code": candidate.get("status_code").and_then(Value::as_i64).unwrap_or(0)
        }));
        last_payload = Some(candidate);
        if !allow_fallback {
            break;
        }
    }

    let mut out = if !selected.is_null() {
        selected
    } else {
        last_payload.unwrap_or_else(|| {
            json!({
                "ok": false,
                "error": "search_providers_exhausted",
                "summary": "Search providers returned no usable findings. Retry with narrower query or explicit source URLs.",
                "content": ""
            })
        })
    };
    let final_selected_provider = if executed_provider.is_empty() {
        selected_provider.clone()
    } else {
        executed_provider.clone()
    };
    if let Some(obj) = provider_resolution.as_object_mut() {
        if final_selected_provider != initial_selected_provider {
            obj.insert(
                "initial_selected_provider".to_string(),
                json!(initial_selected_provider),
            );
            obj.insert("selection_fallback_used".to_string(), json!(true));
        } else {
            obj.insert("selection_fallback_used".to_string(), json!(false));
        }
        obj.insert(
            "selected_provider".to_string(),
            json!(final_selected_provider),
        );
    }
    if out
        .get("provider")
        .and_then(Value::as_str)
        .unwrap_or("")
        .is_empty()
    {
        if let Some(obj) = out.as_object_mut() {
            obj.insert(
                "provider".to_string(),
                if final_selected_provider.is_empty() {
                    Value::String("none".to_string())
                } else {
                    Value::String(final_selected_provider.clone())
                },
            );
        }
    }
    if !out.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        let query_mismatch_only_failure = !provider_errors.is_empty()
            && provider_errors.iter().all(|row| {
                row.get("query_mismatch")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
            });
        if let Some(obj) = out.as_object_mut() {
            let current_error = obj.get("error").and_then(Value::as_str).unwrap_or("");
            if current_error.is_empty() || current_error == "search_providers_exhausted" {
                if query_mismatch_only_failure {
                    obj.insert(
                        "error".to_string(),
                        Value::String("query_result_mismatch".to_string()),
                    );
                    obj.insert(
                        "summary".to_string(),
                        Value::String(
                            "Search providers returned off-topic results for this query. Retry with narrower terms or explicit source URLs."
                                .to_string(),
                        ),
                    );
                } else if tool_surface_status == "unavailable" {
                    obj.insert(
                        "error".to_string(),
                        Value::String("web_search_tool_surface_unavailable".to_string()),
                    );
                    obj.insert(
                        "summary".to_string(),
                        Value::String(
                            "Web search tool surface is currently unavailable. Retry after provider runtime is restored."
                                .to_string(),
                        ),
                    );
                } else if tool_surface_status == "degraded"
                    && (attempted.is_empty() || !tool_surface_ready)
                {
                    obj.insert(
                        "error".to_string(),
                        Value::String("web_search_tool_surface_degraded".to_string()),
                    );
                    obj.insert(
                        "summary".to_string(),
                        Value::String(
                            "Web search tooling is degraded (provider readiness mismatch). Retry after credentials or provider runtime are repaired."
                                .to_string(),
                        ),
                    );
                }
            }
            if obj
                .get("summary")
                .and_then(Value::as_str)
                .unwrap_or("")
                .is_empty()
            {
                obj.insert(
                    "summary".to_string(),
                    Value::String(
                        "Search providers returned no usable findings. Retry with narrower query or explicit source URLs.".to_string(),
                    ),
                );
            }
            if obj.get("error").is_none() {
                obj.insert(
                    "error".to_string(),
                    Value::String("search_providers_exhausted".to_string()),
                );
            }
        }
    }
    let used_lite_fallback = final_selected_provider == "duckduckgo_lite";
    let used_bing_fallback = final_selected_provider == "bing_rss";
    let tool_execution_attempted = !attempted.is_empty();
    let final_error_code = out
        .get("error")
        .and_then(Value::as_str)
        .map(|raw| clean_text(raw, 120));
    let query_mismatch_only_failure = out
        .get("ok")
        .and_then(Value::as_bool)
        .map(|ok| !ok)
        .unwrap_or(true)
        && !provider_errors.is_empty()
        && provider_errors.iter().all(|row| {
            row.get("query_mismatch")
                .and_then(Value::as_bool)
                .unwrap_or(false)
        });
    let challenge_like_failure = search_failure_is_challenge_like(&out, provider_errors.as_slice());
    if let Some(obj) = out.as_object_mut() {
        obj.insert(
            "type".to_string(),
            Value::String("web_conduit_search".to_string()),
        );
        obj.insert("query".to_string(), Value::String(query.clone()));
        obj.insert("query_source".to_string(), json!(query_source));
        obj.insert("query_source_kind".to_string(), json!(query_source_kind));
        obj.insert(
            "query_source_confidence".to_string(),
            json!(query_source_confidence),
        );
        obj.insert(
            "query_source_recovery_mode".to_string(),
            json!(query_source_recovery_mode),
        );
        obj.insert("query_source_lineage".to_string(), query_source_lineage);
        obj.insert(
            "effective_query".to_string(),
            Value::String(scoped_query.clone()),
        );
        obj.insert(
            "allowed_domains".to_string(),
            json!(allowed_domains.clone()),
        );
        obj.insert("exclude_subdomains".to_string(), json!(exclude_subdomains));
        obj.insert("top_k".to_string(), json!(top_k));
        obj.insert("count".to_string(), json!(top_k));
        obj.insert("timeout_ms".to_string(), json!(timeout_ms));
        obj.insert("cache_ttl_minutes".to_string(), json!(cache_ttl_minutes));
        obj.insert("filters".to_string(), normalized_filters.clone());
        obj.insert("provider_chain".to_string(), json!(provider_chain.clone()));
        obj.insert("providers_attempted".to_string(), json!(attempted));
        obj.insert("providers_skipped".to_string(), json!(skipped));
        obj.insert("provider_errors".to_string(), json!(provider_errors));
        obj.insert(
            "provider_resolution".to_string(),
            provider_resolution.clone(),
        );
        obj.insert("tool_execution_gate".to_string(), tool_execution_gate.clone());
        obj.insert("replay_policy".to_string(), replay_policy.clone());
        obj.insert("replay_bypass".to_string(), replay_bypass.clone());
        obj.insert(
            "attempt_signature".to_string(),
            Value::String(search_attempt_signature.clone()),
        );
        obj.insert(
            "attempt_replay_guard".to_string(),
            attempt_replay_guard.clone(),
        );
        obj.insert(
            "provider_health".to_string(),
            provider_health_snapshot(root, &provider_chain),
        );
        obj.insert(
            "tool_surface_status".to_string(),
            Value::String(tool_surface_status.clone()),
        );
        obj.insert("tool_surface_ready".to_string(), json!(tool_surface_ready));
        obj.insert(
            "tool_surface_blocking_reason".to_string(),
            Value::String(tool_surface_blocking_reason.clone()),
        );
        obj.insert(
            "search_lite_fallback".to_string(),
            json!(used_lite_fallback),
        );
        obj.insert(
            "search_bing_fallback".to_string(),
            json!(used_bing_fallback),
        );
        obj.insert("provider_hint".to_string(), Value::String(provider_hint));
        obj.insert(
            "query_shape_override_used".to_string(),
            json!(query_shape_override),
        );
        obj.insert(
            "query_shape_override_source".to_string(),
            json!(query_shape_override_source),
        );
        obj.insert(
            "query_shape_stats".to_string(),
            search_query_shape_stats(&query),
        );
        obj.insert(
            "query_shape_fetch_url_candidate".to_string(),
            json!(query_shape_fetch_url_candidate.clone()),
        );
        obj.insert(
            "query_shape_fetch_url_candidate_kind".to_string(),
            json!(query_shape_fetch_url_candidate_kind),
        );
        obj.insert("query_shape_error".to_string(), json!(query_shape_error));
        obj.insert(
            "query_shape_category".to_string(),
            json!(search_query_shape_category(query_shape_error)),
        );
        obj.insert(
            "query_shape_recommended_action".to_string(),
            json!(search_query_shape_recommended_action(query_shape_error)),
        );
        obj.insert(
            "query_shape_route_hint".to_string(),
            json!(search_query_shape_route_hint(query_shape_error)),
        );
        obj.insert(
            "query_shape".to_string(),
            search_query_shape_contract(
                &query,
                query_shape_error,
                query_shape_override,
                query_shape_override_source,
            ),
        );
        obj.insert(
            "suggested_next_action".to_string(),
            search_query_shape_suggested_next_action(&query, query_shape_error),
        );
        obj.insert(
            "process_summary".to_string(),
            runtime_web_process_summary(
                "web_search",
                "provider_chain_result",
                tool_execution_attempted,
                &tool_execution_gate,
                &attempt_replay_guard,
                &json!(provider_chain.clone()),
                &final_selected_provider,
                final_error_code.as_deref()
            ),
        );
        obj.insert(
            "cache_store_allowed".to_string(),
            json!(!(challenge_like_failure || query_mismatch_only_failure)),
        );
        if query_mismatch_only_failure {
            obj.insert(
                "cache_skip_reason".to_string(),
                json!("query_result_mismatch"),
            );
        } else if challenge_like_failure {
            obj.insert(
                "cache_skip_reason".to_string(),
                json!("challenge_or_low_signal_response"),
            );
        }
        obj.insert("cache_status".to_string(), json!("miss"));
        let summary_raw = clean_text(
            obj.get("summary").and_then(Value::as_str).unwrap_or(""),
            1_400,
        );
        let content_raw = clean_text(
            obj.get("content").and_then(Value::as_str).unwrap_or(""),
            120_000,
        );
        let summary_wrapped = if summary_raw.is_empty() {
            String::new()
        } else {
            wrap_external_untrusted_content(&summary_raw, false, "Web Search")
        };
        let content_wrapped = if content_raw.is_empty() {
            String::new()
        } else {
            wrap_external_untrusted_content(&content_raw, true, "Web Search")
        };
        obj.insert(
            "summary_wrapped".to_string(),
            Value::String(summary_wrapped),
        );
        obj.insert(
            "content_wrapped".to_string(),
            Value::String(content_wrapped),
        );
        obj.insert(
            "external_content".to_string(),
            json!({
                "untrusted": true,
                "source": "web_search",
                "wrapped": true,
                "provider_chain": provider_chain.clone(),
                "tool_surface_status": tool_surface_status.clone(),
                "query_alignment_checked": true,
                "provider": if final_selected_provider.is_empty() {
                    "none"
                } else {
                    final_selected_provider.as_str()
                }
            }),
        );
    }
    let cache_status = if out.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        "ok"
    } else if challenge_like_failure {
        "challenge"
    } else {
        "no_results"
    };
    let search_url = match final_selected_provider.as_str() {
        "duckduckgo_lite" => lite_url.clone(),
        "bing_rss" => web_search_bing_rss_url(&scoped_query),
        _ => primary_url.clone(),
    };
    let response_hash = out
        .get("content")
        .and_then(Value::as_str)
        .filter(|text| !text.trim().is_empty())
        .map(sha256_hex);
    let error = out
        .get("error")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty());
    let receipt = build_receipt(
        &search_url,
        if out.get("ok").and_then(Value::as_bool).unwrap_or(false) {
            "allow"
        } else {
            "deny"
        },
        response_hash.as_deref(),
        out.get("status_code").and_then(Value::as_i64).unwrap_or(0),
        "search_provider_chain",
        error,
    );
    let mut receipt = receipt;
    if let Some(receipt_obj) = receipt.as_object_mut() {
        receipt_obj.insert(
            "attempt_signature".to_string(),
            Value::String(search_attempt_signature),
        );
        receipt_obj.insert(
            "provider".to_string(),
            Value::String(final_selected_provider.clone()),
        );
    }
    let _ = append_jsonl(&receipts_path(root), &receipt);
    if let Some(obj) = out.as_object_mut() {
        obj.insert("receipt".to_string(), receipt);
    }
    if cache_ttl_seconds > 0 && !challenge_like_failure {
        store_search_cache(
            root,
            &cache_key,
            &out,
            cache_status,
            Some(cache_ttl_seconds),
        );
    }
    out
}
