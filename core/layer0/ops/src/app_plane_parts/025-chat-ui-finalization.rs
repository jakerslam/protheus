const CHAT_UI_FRAMEWORK_TARGETS: [&str; 5] = [
    "LangGraph",
    "OpenAI Agents SDK",
    "AutoGen",
    "CrewAI",
    "smolagents",
];
const CHAT_UI_SPECULATIVE_BLOCKER_MARKERS: [&str; 22] = [
    "security controls",
    "content filtering",
    "allowlists",
    "requires proper authorization",
    "policy restrictions",
    "api gateway",
    "intentional design",
    "blocked by security",
    "blocking external tool execution",
    "system blocked the request",
    "blocked the function calls",
    "function calls from executing entirely",
    "invalid response attempt",
    "processing the queries",
    "preventing any web search operations",
    "wouldn't even attempt to execute",
    "would not even attempt to execute",
    "limiting web tool access",
    "temporary system restriction",
    "broader policy change",
    "would you like me to try a different approach",
    "would you like to try a different approach",
];
const CHAT_UI_ALIGNMENT_IGNORED_TERMS: [&str; 44] = [
    "the",
    "a",
    "an",
    "and",
    "or",
    "to",
    "for",
    "of",
    "in",
    "on",
    "is",
    "are",
    "was",
    "were",
    "it",
    "this",
    "that",
    "with",
    "from",
    "as",
    "by",
    "you",
    "your",
    "we",
    "our",
    "can",
    "could",
    "should",
    "would",
    "do",
    "did",
    "does",
    "system",
    "agent",
    "llm",
    "tool",
    "tools",
    "message",
    "response",
    "search",
    "web",
    "result",
    "results",
    "query",
];

fn chat_ui_alignment_terms(text: &str, max_terms: usize) -> Vec<String> {
    let mut terms = Vec::<String>::new();
    for token in clean(text, 4_000)
        .to_ascii_lowercase()
        .split(|ch: char| !ch.is_ascii_alphanumeric())
    {
        if token.len() < 3 {
            continue;
        }
        if CHAT_UI_ALIGNMENT_IGNORED_TERMS.contains(&token) {
            continue;
        }
        if terms.iter().any(|existing| existing == token) {
            continue;
        }
        terms.push(token.to_string());
        if terms.len() >= max_terms {
            break;
        }
    }
    terms
}

fn chat_ui_response_matches_previous_message(user_message: &str, response_text: &str) -> bool {
    let user_terms = chat_ui_alignment_terms(user_message, 24);
    if user_terms.len() < 2 {
        return true;
    }
    let response_terms = chat_ui_alignment_terms(response_text, 72);
    if response_terms.is_empty() {
        return false;
    }
    let overlap_count = user_terms
        .iter()
        .filter(|term| response_terms.iter().any(|candidate| candidate == *term))
        .count();
    if overlap_count == 0 {
        return false;
    }
    if user_terms.len() >= 3 && response_terms.len() >= 18 && overlap_count < 2 {
        return false;
    }
    true
}

fn chat_ui_contains_kernel_patch_thread_dump(user_message: &str, response_text: &str) -> bool {
    let lowered = clean(response_text, 16_000).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    let marker_hits = [
        "[patch",
        "subject:",
        "from:",
        "to:",
        "in-reply-to:",
        "references:",
        "signed-off-by:",
        "diff --git",
        "@@ -",
        "[thread index]",
    ]
    .iter()
    .filter(|marker| lowered.contains(**marker))
    .count();
    if marker_hits < 4 {
        return false;
    }
    let user_lowered = clean(user_message, 1_200).to_ascii_lowercase();
    let user_requested_patch_context = user_lowered.contains("linux kernel")
        || user_lowered.contains("patch review")
        || user_lowered.contains("git diff")
        || user_lowered.contains("signed-off-by")
        || user_lowered.contains("mailing list patch");
    !user_requested_patch_context
}

fn chat_ui_contains_role_preamble_prompt_dump(user_message: &str, response_text: &str) -> bool {
    let lowered = clean(response_text, 10_000).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    let marker_hits = [
        "i am an expert in the field",
        "my role is to",
        "the user has provided",
        "my task is to refine",
        "workflow metadata",
        "source: the model's training data",
        "mechanism: faulty pattern retrieval",
        "the error: context collapse",
    ]
    .iter()
    .filter(|marker| lowered.contains(**marker))
    .count();
    if marker_hits < 2 {
        return false;
    }
    let user_lowered = clean(user_message, 1_200).to_ascii_lowercase();
    let user_requested_prompting_context = user_lowered.contains("write a prompt")
        || user_lowered.contains("system prompt")
        || user_lowered.contains("role prompt")
        || user_lowered.contains("persona prompt");
    !user_requested_prompting_context
}

fn chat_ui_contains_competitive_programming_dump(user_message: &str, response_text: &str) -> bool {
    let lowered = clean(response_text, 16_000).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    let marker_hits = [
        "given a tree",
        "input specification",
        "output specification",
        "sample input",
        "sample output",
        "#include <stdio.h>",
        "int main()",
        "public class",
        "translate the following java code",
        "intelligent recommendation",
        "smart recommendations",
    ]
    .iter()
    .filter(|marker| lowered.contains(**marker))
    .count();
    if marker_hits < 3 {
        return false;
    }
    let user_lowered = clean(user_message, 1_200).to_ascii_lowercase();
    let user_requested_programming_translation = user_lowered.contains("translate")
        || user_lowered.contains("java code")
        || user_lowered.contains("python function")
        || user_lowered.contains("programming problem");
    !user_requested_programming_translation
}

fn chat_ui_tool_text_blob(row: &Value) -> String {
    let input_blob = row
        .get("input")
        .map(|input| clean(&input.to_string(), 1_200))
        .unwrap_or_default();
    clean(
        &format!(
            "{} {} {} {} {} {} {}",
            clean(row.get("name").and_then(Value::as_str).unwrap_or(""), 120),
            clean(row.get("status").and_then(Value::as_str).unwrap_or(""), 120),
            clean(row.get("type").and_then(Value::as_str).unwrap_or(""), 120),
            clean(row.get("error").and_then(Value::as_str).unwrap_or(""), 220),
            clean(row.get("query").and_then(Value::as_str).unwrap_or(""), 600),
            clean(row.get("result").and_then(Value::as_str).unwrap_or(""), 2_200),
            input_blob
        ),
        4_000,
    )
}

fn chat_ui_tools_have_structured_block_evidence(rows: &[Value]) -> bool {
    rows.iter().any(|row| {
        let status = clean(row.get("status").and_then(Value::as_str).unwrap_or(""), 120)
            .to_ascii_lowercase();
        if matches!(
            status.as_str(),
            "blocked" | "policy_denied" | "permission_denied"
        ) {
            return true;
        }
        let tool_type = clean(row.get("type").and_then(Value::as_str).unwrap_or(""), 120)
            .to_ascii_lowercase();
        if tool_type == "tool_pre_gate_blocked" {
            return true;
        }
        let error = clean(row.get("error").and_then(Value::as_str).unwrap_or(""), 220)
            .to_ascii_lowercase();
        if matches!(
            error.as_str(),
            "tool_permission_denied" | "tool_confirmation_required"
        ) {
            return true;
        }
        row.get("status_code")
            .and_then(Value::as_i64)
            .or_else(|| row.get("http_status").and_then(Value::as_i64))
            .map(|code| matches!(code, 401 | 403 | 404 | 422 | 429))
            .unwrap_or(false)
    })
}

fn chat_ui_structured_block_evidence_codes(rows: &[Value]) -> Vec<String> {
    let mut codes = Vec::<String>::new();
    for row in rows {
        for code in [
            clean(row.get("type").and_then(Value::as_str).unwrap_or(""), 120),
            clean(row.get("error").and_then(Value::as_str).unwrap_or(""), 220),
        ] {
            if code.is_empty() || codes.iter().any(|existing| existing == &code) {
                continue;
            }
            codes.push(code);
            if codes.len() >= 4 {
                return codes;
            }
        }
        let status = clean(row.get("status").and_then(Value::as_str).unwrap_or(""), 120);
        if !status.is_empty() {
            let status_code = format!("status:{status}");
            if !codes.iter().any(|existing| existing == &status_code) {
                codes.push(status_code);
                if codes.len() >= 4 {
                    return codes;
                }
            }
        }
        if let Some(http_status) = row
            .get("status_code")
            .and_then(Value::as_i64)
            .or_else(|| row.get("http_status").and_then(Value::as_i64))
        {
            let status_code = format!("http:{http_status}");
            if !codes.iter().any(|existing| existing == &status_code) {
                codes.push(status_code);
                if codes.len() >= 4 {
                    return codes;
                }
            }
        }
    }
    codes
}

fn chat_ui_detect_tool_surface_error_code(rows: &[Value]) -> Option<&'static str> {
    let mut saw_degraded = false;
    let mut saw_unavailable = false;
    for row in rows {
        let lowered = chat_ui_tool_text_blob(row).to_ascii_lowercase();
        if lowered.contains("web_search_tool_surface_unavailable")
            || lowered.contains("web_fetch_tool_surface_unavailable")
            || lowered.contains("web_tool_surface_unavailable")
        {
            saw_unavailable = true;
        }
        if lowered.contains("web_search_tool_surface_degraded")
            || lowered.contains("web_fetch_tool_surface_degraded")
            || lowered.contains("web_tool_surface_degraded")
        {
            saw_degraded = true;
        }
    }
    if saw_unavailable {
        Some("web_tool_surface_unavailable")
    } else if saw_degraded {
        Some("web_tool_surface_degraded")
    } else {
        None
    }
}

fn chat_ui_tool_surface_classification(error_code: &str) -> &'static str {
    if error_code == "web_tool_surface_unavailable" {
        "tool_surface_unavailable"
    } else {
        "tool_surface_degraded"
    }
}

fn chat_ui_tool_surface_forced_outcome(error_code: &str) -> &'static str {
    if error_code == "web_tool_surface_unavailable" {
        "forced_web_tool_surface_unavailable"
    } else {
        "forced_web_tool_surface_degraded"
    }
}

fn chat_ui_contains_speculative_blocker_language(text: &str) -> bool {
    let lowered = clean(text, 4_000).to_ascii_lowercase();
    let marker_hit = CHAT_UI_SPECULATIVE_BLOCKER_MARKERS
        .iter()
        .any(|marker| lowered.contains(marker));
    if marker_hit {
        return true;
    }
    let structured_ack_hit = crate::tool_output_match_filter::matches_ack_placeholder(&lowered)
        && (lowered.contains("web search")
            || lowered.contains("web tool")
            || lowered.contains("tool execution")
            || lowered.contains("function call"));
    structured_ack_hit
}

fn chat_ui_looks_like_deferred_execution_preamble(text: &str) -> bool {
    let lowered = clean(text, 2_000).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    let token_count = lowered.split_whitespace().count();
    if token_count > 64 {
        return false;
    }
    let has_rich_findings = lowered.contains("http://")
        || lowered.contains("https://")
        || lowered.contains("according to")
        || lowered.contains("1.")
        || lowered.contains("2.");
    if has_rich_findings {
        return false;
    }
    lowered.starts_with("i'll get you an update")
        || lowered.starts_with("i will get you an update")
        || lowered.starts_with("let me get you an update")
        || lowered.starts_with("i'll look into")
        || lowered.starts_with("i will look into")
        || lowered.starts_with("let me look into")
        || lowered.starts_with("i'll check")
        || lowered.starts_with("i will check")
        || lowered.starts_with("let me check")
        || lowered.starts_with("i'm going to check")
        || lowered.starts_with("i am going to check")
        || lowered.starts_with("working on it")
        || lowered.starts_with("one moment")
        || lowered.starts_with("just a moment")
        || lowered.starts_with("stand by")
        || lowered.starts_with("i'll report back")
        || lowered.starts_with("i will report back")
}

fn chat_ui_looks_like_deferred_retry_prompt(text: &str) -> bool {
    let lowered = clean(text, 2_000).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    let has_rich_findings = lowered.contains("http://")
        || lowered.contains("https://")
        || lowered.contains("according to")
        || lowered.contains("1.")
        || lowered.contains("2.");
    if has_rich_findings {
        return false;
    }
    let retry_marker_hit = [
        "would you like me to try",
        "would you like me to retry",
        "would you like me to run",
        "should i retry",
        "should i rerun",
        "i can retry with",
        "i can rerun with",
        "if you'd like, i can retry",
        "if you would like, i can retry",
        "if you'd like, i can rerun",
        "if you would like, i can rerun",
        "i can try a narrower query",
        "i can run a narrower query",
        "i can try a more specific query",
        "i can search again",
    ]
    .iter()
    .any(|marker| lowered.contains(marker));
    if !retry_marker_hit {
        return false;
    }
    lowered.contains("search")
        || lowered.contains("web")
        || lowered.contains("tool")
        || lowered.contains("query")
        || lowered.contains("source url")
}

fn chat_ui_looks_like_unrelated_programming_dump(user_message: &str, text: &str) -> bool {
    let lowered = clean(text, 12_000).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    let marker_hits = [
        "<|begin_of_sentence|>",
        "<｜begin▁of▁sentence｜>",
        "you are an expert python programmer",
        "translate the following java code to python",
        "input specification:",
        "output specification:",
        "sample input:",
        "sample output:",
        "csdn.net",
        "acm",
        "page id",
        "convolution theorem",
        "laplace transform",
        "hc2017hcc98",
        "__c 0.0",
        "implement a supported rust route for `tool::spawn_subagents",
        "run improve command-to-route mapping for higher supported tool hit rate",
        "run `infring web search` as the next safe step",
    ]
    .iter()
    .filter(|marker| lowered.contains(**marker))
    .count();
    if marker_hits < 2 {
        return false;
    }
    let user_lowered = clean(user_message, 1_200).to_ascii_lowercase();
    let user_requested_content = user_lowered.contains("laplace")
        || user_lowered.contains("convolution theorem")
        || user_lowered.contains("input specification")
        || user_lowered.contains("sample input")
        || user_lowered.contains("java")
        || user_lowered.contains("python")
        || user_lowered.contains("salesforce")
        || user_lowered.contains("__c")
        || user_lowered.contains("spawn_subagents")
        || user_lowered.contains("command-to-route mapping");
    !user_requested_content
}

fn chat_ui_partial_framework_coverage_summary(rows: &[Value]) -> Option<String> {
    let aggregated = rows
        .iter()
        .map(chat_ui_tool_text_blob)
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase();
    if aggregated.is_empty() || !aggregated.contains("framework") {
        return None;
    }

    let found = [
        aggregated.contains("langgraph"),
        aggregated.contains("openai agents sdk") || aggregated.contains("openai agents"),
        aggregated.contains("autogen"),
        aggregated.contains("crewai"),
        aggregated.contains("smolagents"),
    ];
    let found_labels = CHAT_UI_FRAMEWORK_TARGETS
        .iter()
        .enumerate()
        .filter_map(|(idx, label)| found[idx].then_some(*label))
        .collect::<Vec<_>>();
    if found_labels.is_empty() {
        return None;
    }
    let missing_labels = CHAT_UI_FRAMEWORK_TARGETS
        .iter()
        .enumerate()
        .filter_map(|(idx, label)| (!found[idx]).then_some(*label))
        .collect::<Vec<_>>();
    if missing_labels.is_empty() {
        return None;
    }

    Some(format!(
        "Web search ran and returned partial framework coverage. Found: {}. Missing in this pass: {}. I can retry with targeted queries for the missing frameworks.",
        found_labels.join(", "),
        missing_labels.join(", ")
    ))
}

fn chat_ui_tool_row_has_valid_findings(row: &Value) -> bool {
    let status = clean(row.get("status").and_then(Value::as_str).unwrap_or(""), 80)
        .to_ascii_lowercase();
    if matches!(
        status.as_str(),
        "error"
            | "failed"
            | "blocked"
            | "policy_denied"
            | "no_results"
            | "low_signal"
            | "partial_no_results"
            | "timeout"
            | "execution_error"
    ) {
        return false;
    }
    let result = clean(row.get("result").and_then(Value::as_str).unwrap_or(""), 2_000);
    if result.is_empty() || result.contains("<function=") {
        return false;
    }
    !crate::tool_output_match_filter::matches_ack_placeholder(&result)
}

fn chat_ui_tools_have_valid_findings(rows: &[Value]) -> bool {
    rows.iter().any(chat_ui_tool_row_has_valid_findings)
}

fn finalize_chat_ui_assistant_response(
    user_message: &str,
    assistant_raw: &str,
    tools: &[Value],
) -> (String, String) {
    if let Some(tool_surface_error) = chat_ui_detect_tool_surface_error_code(tools) {
        return (
            String::new(),
            "tool_surface_error_fail_closed".to_string(),
        );
    }
    let mut cleaned = clean(assistant_raw, 16_000);
    let mut outcome = "unchanged".to_string();
    if let Some((rewritten, rule_id)) =
        crate::tool_output_match_filter::rewrite_raw_payload_dump(&cleaned)
    {
        let _ = rewritten;
        cleaned.clear();
        outcome = format!("rewrote:{rule_id}");
    }
    if let Some((rewritten, rule_id)) =
        crate::tool_output_match_filter::rewrite_unsynthesized_web_dump(&cleaned)
    {
        let _ = rewritten;
        cleaned.clear();
        outcome = format!("rewrote:{rule_id}");
    }
    if let Some((rewritten, rule_id)) =
        crate::tool_output_match_filter::rewrite_failure_placeholder(&cleaned)
    {
        let _ = rewritten;
        cleaned.clear();
        outcome = format!("rewrote_failure:{rule_id}");
    }
    let speculative_blocker_copy = chat_ui_contains_speculative_blocker_language(&cleaned);
    let has_block_evidence = chat_ui_tools_have_structured_block_evidence(tools);
    let partial_framework_coverage = chat_ui_partial_framework_coverage_summary(tools);
    let unrelated_context_dump = chat_ui_contains_kernel_patch_thread_dump(user_message, &cleaned)
        || chat_ui_contains_role_preamble_prompt_dump(user_message, &cleaned)
        || chat_ui_looks_like_unrelated_programming_dump(user_message, &cleaned)
        || crate::tool_output_match_filter::contains_forbidden_runtime_context_markers(&cleaned);
    if unrelated_context_dump {
        return (String::new(), "withheld_unrelated_context_dump".to_string());
    }
    if speculative_blocker_copy && has_block_evidence {
        let codes = chat_ui_structured_block_evidence_codes(tools);
        let detail = if codes.is_empty() {
            None
        } else {
            Some(codes.join(", "))
        };
        let _ = detail;
        return (String::new(), "withheld_blocked_with_structured_evidence".to_string());
    }
    if speculative_blocker_copy && !has_block_evidence {
        return (String::new(), "withheld_unverified_blocker_claim".to_string());
    }
    let low_signal = cleaned.trim().is_empty()
        || cleaned.contains("<function=")
        || crate::tool_output_match_filter::matches_ack_placeholder(&cleaned)
        || chat_ui_looks_like_deferred_execution_preamble(&cleaned)
        || chat_ui_looks_like_deferred_retry_prompt(&cleaned)
        || chat_ui_looks_like_unrelated_programming_dump(user_message, &cleaned)
        || (cleaned.len() > 80
            && !chat_ui_response_matches_previous_message(user_message, &cleaned));
    if !low_signal {
        let lowered = cleaned.to_ascii_lowercase();
        if let Some(summary) = partial_framework_coverage {
            if lowered.contains("low signal")
                || lowered.contains("low-signal")
                || lowered.contains("partial")
            {
                return (summary, "success_with_gaps".to_string());
            }
        }
        return (cleaned, outcome);
    }
    (
        String::new(),
        "withheld_non_llm_finalization_repair".to_string(),
    )
}
