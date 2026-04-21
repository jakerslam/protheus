// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)
//
// Imported pattern contract (RTK intake):
// - source: local/workspace/vendor/rtk/src/core/toml_filter.rs
// - concept: match_output short-circuit rules with optional "unless" guard.

use regex::Regex;
use serde_json::{json, Value};
use std::sync::OnceLock;

struct MatchRule {
    pattern: &'static str,
    unless: Option<&'static str>,
}

const RAW_PAYLOAD_KEY_MARKERS: &[&str] = &[
    "\"agent_id\"",
    "\"input_tokens\"",
    "\"output_tokens\"",
    "\"latent_tool_candidates\"",
    "\"nexus_connection\"",
    "\"turn_loop_tracking\"",
    "\"turn_transaction\"",
    "\"response_finalization\"",
    "\"decision_audit_receipt\"",
    "\"workspace_hints\"",
    "\"provider\"",
    "\"runtime_model\"",
    "\"tools\"",
    "\"attention_queue\"",
    "\"memory_capture\"",
    "\"turn_loop_post_filter\"",
    "\"tool_completion\"",
    "\"live_tool_status\"",
    "\"live_tool_steps\"",
    "\"tool_execute\"",
    "\"session_persist\"",
    "\"nexus_receipt_\"",
    "\"lease_id\"",
    "\"policy_decision_ref\"",
    "\"tool_count\"",
    "\"successful_tools\"",
    "\"error_tools\"",
];

const NO_FINDINGS_USER_COPY: &str =
    "The tool path ran, but this turn only produced low-signal or no-result output, so there are no source-backed findings yet and we don't have usable tool findings from this turn yet. web_status: provider_low_signal error_code: web_tool_low_signal";

const UNSYNTHESIZED_WEB_MARKERS: &[&str] = &[
    "web benchmark synthesis:",
    "key findings for",
    "potential sources:",
    "bing.com:",
    "duckduckgo.com:",
    "www.bing.com:",
    "search response came from",
    "all regions",
    "safe search",
    "any time",
    "unfortunately, bots use duckduckgo too",
    "please complete the following challenge",
    "select all squares containing",
    "images not loading",
    "error-lite@duckduckgo.com",
    "duckduckgo duckduckgo",
    "the search response came from",
    "couldn't extract usable findings",
    "no relevant results found for that request yet",
    "couldn't produce source-backed findings in this turn",
    "don't have usable tool findings from this turn yet",
    "low-signal or no-result output",
    "originalurl:",
    "featuredcontent:",
    "provider:",
    "publisheddatetime:",
    "type: video",
    "price: free",
];

const ANALYSIS_MARKERS: &[&str] = &[
    "in short",
    "overall",
    "therefore",
    "recommend",
    "trade-off",
    "tradeoff",
    "because",
];

const THINKING_CHATTER_MARKERS: &[&str] = &[
    "planning next step",
    "analyzing request",
    "active [end]",
    "working",
];

const FORBIDDEN_RUNTIME_CONTEXT_MARKERS: &[&str] = &[
    "begin_openclaw_internal_context",
    "end_openclaw_internal_context",
    "begin_untrusted_child_result",
    "end_untrusted_child_result",
    "<|begin_of_sentence|>",
    "<｜begin▁of▁sentence｜>",
    "you are an expert python programmer",
    "translate the following java code to python",
    "<function=",
    "</function>",
    "<tool_call",
    "</tool_call>",
    "[patch v",
    "signed-off-by:",
    "diff --git",
    "workflow metadata",
];

fn repeated_line_count(text: &str) -> usize {
    let mut seen = std::collections::HashMap::<String, usize>::new();
    for line in text.lines() {
        let normalized = clean_text(line, 240).to_ascii_lowercase();
        if normalized.is_empty() {
            continue;
        }
        *seen.entry(normalized).or_insert(0) += 1;
    }
    seen.into_values().max().unwrap_or(0)
}

fn ack_rules() -> &'static Vec<(MatchRule, Regex, Option<Regex>)> {
    static RULES: OnceLock<Vec<(MatchRule, Regex, Option<Regex>)>> = OnceLock::new();
    RULES.get_or_init(|| {
        let specs = vec![
            MatchRule {
                pattern: r"(?is)^\s*web search completed\.?\s*$",
                unless: None,
            },
            MatchRule {
                pattern: r"(?is)^\s*tool call finished\.?\s*$",
                unless: None,
            },
            MatchRule {
                pattern: r"(?is)^\s*batch execution initiated(?:\b.*)?$",
                unless: Some(r"(?is)https?://|according to|source"),
            },
            MatchRule {
                pattern: r"(?is)^\s*this demonstrates the full pipeline(?:\b.*)?$",
                unless: Some(r"(?is)https?://|according to|source"),
            },
            MatchRule {
                pattern: r"(?is)^\s*the system will:\s*(?:\b.*)$",
                unless: Some(r"(?is)https?://|according to|source"),
            },
            MatchRule {
                pattern: r"(?is)^\s*(?:i\s+)?could(?:n't| not)\s+extract\s+(?:usable|reliable)\s+findings(?:\b.*)?$",
                unless: Some(r"(?is)(?:key finding|sources?:|according to)"),
            },
            MatchRule {
                pattern: r"(?is)key findings for .*potential sources:",
                unless: Some(r"(?is)https?://"),
            },
            MatchRule {
                pattern: r"(?is)from web retrieval,\s*candidate domains include",
                unless: Some(r"(?is)https?://"),
            },
            MatchRule {
                pattern: r"(?is)could(?:n't| not) extract (?:usable|reliable) findings.*search response came from https?://duckduckgo\.com/html/\?q=",
                unless: Some(r"(?is)(?:key finding|sources?:|according to)"),
            },
        ];
        specs
            .into_iter()
            .map(|rule| {
                let compiled = Regex::new(rule.pattern).expect("valid ack rule regex");
                let unless = rule
                    .unless
                    .map(|value| Regex::new(value).expect("valid ack unless regex"));
                (rule, compiled, unless)
            })
            .collect()
    })
}

fn failure_rewrite_pattern() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?is)^\s*i couldn't complete [`'"]?([a-z0-9_.\-]+)[`'"]? right now\.?\s*$"#)
            .expect("valid failure rewrite regex")
    })
}

fn clean_text(raw: &str, max_len: usize) -> String {
    raw.chars()
        .take(max_len)
        .collect::<String>()
        .trim()
        .to_string()
}

fn contains_any_marker(text: &str, markers: &[&str]) -> bool {
    markers.iter().any(|marker| text.contains(marker))
}

pub fn normalize_web_tooling_error_code(raw: &str) -> String {
    let lowered = clean_text(raw, 240).to_ascii_lowercase();
    if lowered.is_empty() {
        return "web_tool_error".to_string();
    }
    if lowered.contains("web_tool_surface_unavailable")
        || lowered.contains("tool surface is unavailable")
        || lowered.contains("surface_unavailable")
    {
        return "web_tool_surface_unavailable".to_string();
    }
    if lowered.contains("web_tool_surface_degraded")
        || lowered.contains("tool surface is degraded")
        || lowered.contains("surface_degraded")
    {
        return "web_tool_surface_degraded".to_string();
    }
    if lowered.contains("not found")
        || lowered.contains("unknown tool")
        || lowered.contains("tool_not_found")
        || lowered.contains("unrecognized tool")
    {
        return "web_tool_not_found".to_string();
    }
    if lowered.contains("invalid response")
        || lowered.contains("invalid_response")
        || lowered.contains("parse_failed")
    {
        return "web_tool_invalid_response".to_string();
    }
    if lowered.contains("auth")
        || lowered.contains("token")
        || lowered.contains("api key")
        || lowered.contains("credential")
    {
        return "web_tool_auth_missing".to_string();
    }
    if lowered.contains("blocked") || lowered.contains("policy") || lowered.contains("denied") {
        return "web_tool_policy_blocked".to_string();
    }
    if lowered.contains("timeout") {
        return "web_tool_timeout".to_string();
    }
    if lowered.contains("429") {
        return "web_tool_http_429".to_string();
    }
    if lowered.contains("404") {
        return "web_tool_http_404".to_string();
    }
    if lowered.contains("403") {
        return "web_tool_http_403".to_string();
    }
    if lowered.contains("401") {
        return "web_tool_http_401".to_string();
    }
    if lowered.contains("500")
        || lowered.contains("502")
        || lowered.contains("503")
        || lowered.contains("504")
    {
        return "web_tool_http_5xx".to_string();
    }
    if lowered.contains("low_signal")
        || lowered.contains("low-signal")
        || lowered.contains("no_result")
        || lowered.contains("no_results")
    {
        return "web_tool_low_signal".to_string();
    }
    if lowered.starts_with("web_tool_") {
        return lowered;
    }
    "web_tool_error".to_string()
}

pub fn contains_forbidden_runtime_context_markers(raw: &str) -> bool {
    let lowered = clean_text(raw, 16_000).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    FORBIDDEN_RUNTIME_CONTEXT_MARKERS
        .iter()
        .any(|marker| lowered.contains(marker))
}

pub fn canonical_tool_status(
    raw_status: &str,
    ok: Option<bool>,
    error: &str,
    findings_count: i64,
    has_result_text: bool,
) -> String {
    let lowered_status = clean_text(raw_status, 120).to_ascii_lowercase();
    let error_clean = clean_text(error, 300);
    let mut status = if lowered_status.contains("web_tool_surface_unavailable")
        || lowered_status.contains("web_tool_surface_degraded")
        || lowered_status.contains("surface_unavailable")
        || lowered_status.contains("surface_degraded")
    {
        "error".to_string()
    } else if lowered_status.contains("blocked")
        || lowered_status.contains("policy")
        || lowered_status.contains("denied")
    {
        "blocked".to_string()
    } else if lowered_status.contains("not_found")
        || lowered_status.contains("no_result")
        || lowered_status.contains("no-results")
    {
        "not_found".to_string()
    } else if lowered_status.contains("low_signal") || lowered_status.contains("low-signal") {
        "low_signal".to_string()
    } else if lowered_status.contains("ok") || lowered_status.contains("success") {
        "ok".to_string()
    } else if lowered_status.contains("failed")
        || lowered_status.contains("error")
        || lowered_status.contains("timeout")
    {
        "error".to_string()
    } else {
        "unknown".to_string()
    };
    if status == "unknown" {
        if !error_clean.is_empty() || ok == Some(false) {
            status = if normalize_web_tooling_error_code(&error_clean) == "web_tool_not_found" {
                "not_found".to_string()
            } else {
                "error".to_string()
            };
        } else if ok == Some(true) || findings_count > 0 {
            status = "ok".to_string();
        } else if has_result_text {
            status = "low_signal".to_string();
        }
    }
    if status == "error" && normalize_web_tooling_error_code(&error_clean) == "web_tool_not_found" {
        return "not_found".to_string();
    }
    status
}

fn default_error_code_for_status(status: &str) -> &'static str {
    match status {
        "ok" => "none",
        "blocked" => "web_tool_policy_blocked",
        "not_found" => "web_tool_not_found",
        "low_signal" => "web_tool_low_signal",
        "unknown" => "web_tool_silent_failure",
        _ => "web_tool_error",
    }
}

pub fn canonical_tool_execution_receipt(
    call_id: &str,
    tool: &str,
    raw_status: &str,
    ok: Option<bool>,
    error: &str,
    findings_count: i64,
    duration_ms: i64,
    tokens_used: i64,
    has_result_text: bool,
) -> Value {
    let status = canonical_tool_status(raw_status, ok, error, findings_count, has_result_text);
    let cleaned_error = clean_text(error, 300);
    let status_hint_error_code = normalize_web_tooling_error_code(raw_status);
    let error_code = if cleaned_error.is_empty() {
        if status == "error"
            && matches!(
                status_hint_error_code.as_str(),
                "web_tool_surface_unavailable" | "web_tool_surface_degraded"
            )
        {
            status_hint_error_code
        } else {
            default_error_code_for_status(&status).to_string()
        }
    } else {
        normalize_web_tooling_error_code(&cleaned_error)
    };
    json!({
        "call_id": clean_text(call_id, 120),
        "tool": clean_text(tool, 120),
        "status": status,
        "ok": ok.unwrap_or(false),
        "findings_count": findings_count.max(0),
        "error": cleaned_error,
        "error_code": error_code,
        "has_result": has_result_text,
        "telemetry": {
            "duration_ms": duration_ms.max(0),
            "tokens_used": tokens_used.max(0)
        }
    })
}

pub fn canonical_tooling_fallback_copy(
    web_status: &str,
    error_code: &str,
    detail: Option<&str>,
) -> String {
    let status = clean_text(web_status, 80).to_ascii_lowercase();
    let status = if status.is_empty() {
        "failed".to_string()
    } else {
        status
    };
    let mut normalized_error = clean_text(error_code, 120).to_ascii_lowercase();
    if normalized_error.is_empty() {
        normalized_error = default_error_code_for_status(&status).to_string();
    } else if !normalized_error.starts_with("web_tool_") && normalized_error != "none" {
        normalized_error = normalize_web_tooling_error_code(&normalized_error);
    }
    let detail = detail.map(|value| clean_text(value, 260)).unwrap_or_default();
    if normalized_error == "web_tool_surface_unavailable"
        || normalized_error == "web_tool_surface_degraded"
    {
        let mut out = if normalized_error == "web_tool_surface_unavailable" {
            "I could not complete live web retrieval in this turn because the web tool surface is unavailable. Retry after restoring web tooling, or provide a source URL and I can continue with local analysis.".to_string()
        } else {
            "I could not complete live web retrieval in this turn because the web tool surface is degraded. Retry after restoring provider credentials/runtime, or provide a source URL and I can continue with local analysis.".to_string()
        };
        if !detail.is_empty() {
            out.push_str(&format!(" detail: {detail}."));
        }
        return out;
    }
    let mut out = format!(
        "Tool execution did not produce a usable final answer in this turn. web_status: {status}. error_code: {normalized_error}."
    );
    if !detail.is_empty() {
        out.push_str(&format!(" detail: {detail}."));
    }
    out
}

fn looks_like_json_payload_envelope(text: &str) -> bool {
    let brace_count = text.matches('{').count() + text.matches('}').count();
    let colon_count = text.matches(':').count();
    let quote_count = text.matches('"').count();
    brace_count >= 6 && colon_count >= 8 && quote_count >= 16
}

fn looks_like_speculative_web_blocker_explanation(raw: &str) -> bool {
    let lowered = clean_text(raw, 8_000).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    let mentions_blocked_search_lane = lowered.contains("unable to access web search")
        || lowered.contains("web search functionality")
        || lowered.contains("web search and fetch")
        || lowered.contains("web search and fetch operations")
        || lowered.contains("web search operations")
        || lowered.contains("web search operations related")
        || lowered.contains("search function isn't currently operational")
        || lowered.contains("search function is not currently operational")
        || lowered.contains("blocking tool execution attempts")
        || lowered.contains("block external tool execution")
        || lowered.contains("external tool execution")
        || lowered.contains("blocked the function calls")
        || lowered.contains("blocked the function calls from executing entirely")
        || lowered.contains("function calls entirely")
        || lowered.contains("wouldn't even attempt to execute")
        || lowered.contains("would not even attempt to execute")
        || lowered.contains("function execution level")
        || lowered.contains("invalid response attempt")
        || lowered.contains("processing the queries")
        || lowered.contains("preventing any web search operations")
        || lowered.contains("limiting web tool access")
        || lowered.contains("tool execution attempts")
        || lowered.contains("web capability")
        || lowered.contains("web tooling")
        || lowered.contains("function call format")
        || lowered.contains("function requests")
        || lowered.contains("ack_only")
        || lowered.contains("recognized function");
    if !mentions_blocked_search_lane {
        return false;
    }
    let mentions_execution_denial = lowered.contains("didn't execute")
        || lowered.contains("did not execute")
        || lowered.contains("couldn't execute")
        || lowered.contains("could not execute")
        || lowered.contains("prevents actual execution")
        || lowered.contains("preventing actual execution")
        || lowered.contains("execution permissions are currently restricted")
        || lowered.contains("execution permissions are restricted")
        || lowered.contains("requires proper authorization")
        || lowered.contains("requires authorization before executing")
        || lowered.contains("requires authorization")
        || lowered.contains("security controls")
        || lowered.contains("allowlists")
        || lowered.contains("execution policy settings");
    mentions_execution_denial
        || lowered.contains("likely reasons")
        || lowered.contains("configuration restrictions")
        || lowered.contains("authentication issues")
        || lowered.contains("rate limiting")
        || lowered.contains("intentional design")
        || lowered.contains("sandboxed")
        || lowered.contains("policy restrictions")
        || lowered.contains("api gateway")
}

pub fn matches_raw_payload_dump(raw: &str) -> bool {
    let cleaned = clean_text(raw, 32_000);
    if cleaned.is_empty() {
        return false;
    }
    if !(cleaned.starts_with('{') || cleaned.starts_with("```json") || cleaned.starts_with("```")) {
        return false;
    }
    let lowered = cleaned.to_ascii_lowercase();
    let hit_count = RAW_PAYLOAD_KEY_MARKERS
        .iter()
        .filter(|marker| lowered.contains(**marker))
        .count();
    hit_count >= 4 || (hit_count >= 2 && looks_like_json_payload_envelope(&lowered))
}

pub fn no_findings_user_copy() -> &'static str {
    NO_FINDINGS_USER_COPY
}

pub fn rewrite_raw_payload_dump(raw: &str) -> Option<(String, String)> {
    if !matches_raw_payload_dump(raw) {
        return None;
    }
    Some((
        "I suppressed raw runtime payload output. I can provide a concise synthesized summary instead."
            .to_string(),
        "raw_payload_dump_suppressed".to_string(),
    ))
}

fn looks_like_unsynthesized_web_dump(raw: &str) -> bool {
    let cleaned = clean_text(raw, 8_000);
    if cleaned.is_empty() {
        return false;
    }
    let lowered = cleaned.to_ascii_lowercase();
    if lowered.starts_with("web benchmark synthesis:") {
        let has_source_hint = lowered.contains("http://")
            || lowered.contains("https://")
            || lowered.contains(".com:")
            || lowered.contains(".org:")
            || lowered.contains(".ai:")
            || lowered.contains(".dev:");
        let has_metric_hint = lowered.contains('%')
            || lowered.contains(" latency")
            || lowered.contains("throughput")
            || lowered.contains(" success rate")
            || lowered.contains("tokens/s")
            || lowered.contains("ops/sec")
            || lowered.contains("qps")
            || lowered.contains(" ms")
            || lowered.contains(" seconds");
        if has_source_hint && has_metric_hint {
            return false;
        }
    }
    if !contains_any_marker(&lowered, UNSYNTHESIZED_WEB_MARKERS) {
        return false;
    }
    if contains_any_marker(&lowered, ANALYSIS_MARKERS) {
        return false;
    }
    true
}

pub fn rewrite_unsynthesized_web_dump(raw: &str) -> Option<(String, String)> {
    if !looks_like_unsynthesized_web_dump(raw) {
        return None;
    }
    let lowered = clean_text(raw, 8_000).to_ascii_lowercase();
    let challenge = lowered.contains("please complete the following challenge")
        || lowered.contains("unfortunately, bots use duckduckgo too")
        || lowered.contains("select all squares containing");
    if challenge {
        return Some((
            "Web retrieval hit an anti-bot challenge before usable content was extracted. Ask me to retry with alternate providers or specific source URLs for a source-backed synthesis."
                .to_string(),
            "unsynthesized_web_dump_antibot_rewritten".to_string(),
        ));
    }
    Some((
        "Web retrieval returned low-signal snippets without synthesis. Ask me to rerun with a narrower query and I will return a concise source-backed answer.".to_string(),
        "unsynthesized_web_dump_rewritten".to_string(),
    ))
}

fn looks_like_repetitive_thinking_chatter(raw: &str) -> bool {
    let cleaned = clean_text(raw, 2_000);
    if cleaned.is_empty() {
        return false;
    }
    let lowered = cleaned.to_ascii_lowercase();
    if !contains_any_marker(&lowered, THINKING_CHATTER_MARKERS) {
        return false;
    }
    repeated_line_count(&lowered) >= 2
}

pub fn rewrite_repetitive_thinking_chatter(raw: &str) -> Option<(String, String)> {
    if !looks_like_repetitive_thinking_chatter(raw) {
        return None;
    }
    Some((
        "Thinking.".to_string(),
        "thinking_chatter_compacted".to_string(),
    ))
}

pub fn matches_ack_placeholder(raw: &str) -> bool {
    let cleaned = clean_text(raw, 4_000);
    if cleaned.is_empty() {
        return true;
    }
    for (_rule, pattern, unless) in ack_rules() {
        if !pattern.is_match(&cleaned) {
            continue;
        }
        if let Some(unless_pattern) = unless {
            if unless_pattern.is_match(&cleaned) {
                continue;
            }
        }
        return true;
    }
    if looks_like_speculative_web_blocker_explanation(&cleaned) {
        return true;
    }
    false
}

pub fn rewrite_failure_placeholder(raw: &str) -> Option<(String, String)> {
    let cleaned = clean_text(raw, 4_000);
    if cleaned.is_empty() {
        return None;
    }
    let captures = failure_rewrite_pattern().captures(&cleaned)?;
    let tool_name = captures
        .get(1)
        .map(|value| value.as_str().trim())
        .unwrap_or("tool");
    let rule_id = "tool_could_not_complete".to_string();
    let replacement = format!(
        "`{}` couldn't run in this turn. Ask me to retry, or run `infringctl doctor --json` and share the output.",
        tool_name
    );
    Some((replacement, rule_id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_web_search_completed_placeholder() {
        assert!(matches_ack_placeholder("Web search completed."));
    }

    #[test]
    fn detects_duckduckgo_placeholder_without_real_findings() {
        assert!(matches_ack_placeholder(
            "I couldn't extract usable findings for this yet. The search response came from https://duckduckgo.com/html/?q=agent+framework+ranking"
        ));
    }

    #[test]
    fn ignores_duckduckgo_like_copy_when_findings_are_present() {
        assert!(!matches_ack_placeholder(
            "I couldn't extract usable findings. Sources: https://example.com/one https://example.com/two"
        ));
    }

    #[test]
    fn detects_plain_no_findings_placeholder_without_sources() {
        assert!(matches_ack_placeholder(
            "I couldn't extract usable findings from that search yet."
        ));
    }

    #[test]
    fn detects_speculative_web_blocker_explanation_as_ack_placeholder() {
        let raw = "I understand you're looking for a comparison between this platform and OpenClaw, but I'm currently unable to access web search functionality to gather the necessary information. The system is blocking tool execution attempts, which prevents me from retrieving current details.\n\nBased on system behavior, likely reasons include Configuration Restrictions, Authentication Issues, or Rate Limiting.";
        assert!(matches_ack_placeholder(raw));
    }

    #[test]
    fn detects_ack_only_function_call_format_blocker_as_ack_placeholder() {
        let raw = "I attempted the web search again with the exact function call format, but the system rejected it with an \"ack_only\" response, meaning it recognized the function but didn't execute it. The platform is actively blocking external tool execution. The rejection appears to be a deliberate system policy rather than a technical failure.";
        assert!(matches_ack_placeholder(raw));
    }

    #[test]
    fn detects_security_control_authorization_blocker_as_ack_placeholder() {
        let raw = "I attempted the web search and fetch operations again using the exact function call format, but the system continues to block external tool execution. The platform recognizes the function requests, but consistently prevents actual execution, likely due to security controls. The system requires proper authorization before executing external calls; check API gateway configurations, external service allowlists, and execution policy settings.";
        assert!(matches_ack_placeholder(raw));
    }

    #[test]
    fn detects_blocked_function_calls_security_controls_copy_as_ack_placeholder() {
        let raw = "I attempted to run those searches but the system blocked the function calls entirely - it wouldn't even attempt to execute them. The security controls are preventing any web search operations related to AI frameworks. This confirms the security policy is working at the function execution level.";
        assert!(matches_ack_placeholder(raw));
    }

    #[test]
    fn detects_key_findings_potential_sources_scaffold_without_urls() {
        assert!(matches_ack_placeholder(
            "Key findings for \"Infring AI\": - Potential sources: nlplogix.com, gartner.com."
        ));
    }

    #[test]
    fn detects_from_web_retrieval_candidate_domains_scaffold_without_urls() {
        assert!(matches_ack_placeholder(
            "From web retrieval, candidate domains include reuters.com, cnbc.com, bloomberg.com."
        ));
    }

    #[test]
    fn rewrites_generic_tool_failure_placeholder_to_actionable_copy() {
        let rewritten =
            rewrite_failure_placeholder("I couldn't complete system_diagnostic right now.")
                .expect("rewrite");
        assert!(rewritten.0.to_ascii_lowercase().contains("doctor --json"));
        assert_eq!(rewritten.1, "tool_could_not_complete");
    }

    #[test]
    fn detects_raw_payload_dump_from_tool_router() {
        let payload = r#"{"agent_id":"agent-1","input_tokens":10,"output_tokens":20,"latent_tool_candidates":[],"nexus_connection":{},"turn_transaction":{},"response_finalization":{},"tools":[]}"#;
        assert!(matches_raw_payload_dump(payload));
    }

    #[test]
    fn rewrites_raw_payload_dump_to_synth_hint() {
        let payload = r#"{"agent_id":"agent-1","input_tokens":10,"output_tokens":20,"latent_tool_candidates":[],"nexus_connection":{},"turn_transaction":{},"response_finalization":{},"tools":[]}"#;
        let rewritten = rewrite_raw_payload_dump(payload).expect("rewrite");
        assert!(rewritten
            .0
            .to_ascii_lowercase()
            .contains("suppressed raw runtime payload"));
        assert_eq!(rewritten.1, "raw_payload_dump_suppressed");
    }

    #[test]
    fn rewrites_unsynthesized_web_dump_copy() {
        let raw = "Web benchmark synthesis: bing.com: compare [A with B] vs compare A [with B].";
        let rewritten = rewrite_unsynthesized_web_dump(raw).expect("rewrite");
        assert!(rewritten
            .0
            .to_ascii_lowercase()
            .contains("source-backed answer"));
        assert_eq!(rewritten.1, "unsynthesized_web_dump_rewritten");
    }

    #[test]
    fn keeps_synthesized_from_web_retrieval_summary_copy() {
        let raw = "From web retrieval: reuters.com: Reuters reported a launch timeline and rollout window for the deployment.";
        assert!(rewrite_unsynthesized_web_dump(raw).is_none());
    }

    #[test]
    fn keeps_metric_rich_web_benchmark_synthesis_copy() {
        let raw = "Web benchmark synthesis: artificialanalysis.ai: median latency 820ms, throughput 48 tokens/s, and success rate 86%. Source: https://artificialanalysis.ai/benchmarks/agent-frameworks";
        assert!(rewrite_unsynthesized_web_dump(raw).is_none());
    }

    #[test]
    fn rewrites_antibot_unsynthesized_web_dump_copy() {
        let raw = "DuckDuckGo DuckDuckGo Unfortunately, bots use DuckDuckGo too. Please complete the following challenge to confirm this search was made by a human. Select all squares containing a duck.";
        let rewritten = rewrite_unsynthesized_web_dump(raw).expect("rewrite");
        assert!(rewritten
            .0
            .to_ascii_lowercase()
            .contains("anti-bot challenge"));
        assert_eq!(rewritten.1, "unsynthesized_web_dump_antibot_rewritten");
    }

    #[test]
    fn rewrites_repetitive_thinking_chatter_to_single_sentence() {
        let raw = "Planning next step\nAnalyzing request\nAnalyzing request\nactive [end]";
        let rewritten = rewrite_repetitive_thinking_chatter(raw).expect("rewrite");
        assert_eq!(rewritten.0, "Thinking.");
        assert_eq!(rewritten.1, "thinking_chatter_compacted");
    }

    #[test]
    fn normalizes_web_tooling_not_found_error_codes() {
        assert_eq!(
            normalize_web_tooling_error_code("unknown tool: web.search"),
            "web_tool_not_found"
        );
    }

    #[test]
    fn detects_forbidden_runtime_context_markers() {
        let raw =
            "Subject: [PATCH v2 1/2]\nSigned-off-by: user\nDiff --git a/file.rs b/file.rs\n";
        assert!(contains_forbidden_runtime_context_markers(raw));
        assert!(!contains_forbidden_runtime_context_markers(
            "source-backed answer with no patch markers"
        ));
    }

    #[test]
    fn canonical_tool_execution_receipt_normalizes_unknown_statuses() {
        let row = canonical_tool_execution_receipt(
            "toolcall_123",
            "parse_workspace",
            "",
            Some(false),
            "unknown tool",
            0,
            0,
            0,
            false,
        );
        assert_eq!(row.get("status").and_then(Value::as_str), Some("not_found"));
        assert_eq!(
            row.get("error_code").and_then(Value::as_str),
            Some("web_tool_not_found")
        );
    }

    #[test]
    fn canonical_tool_status_marks_surface_status_as_error() {
        assert_eq!(
            canonical_tool_status(
                "web_tool_surface_unavailable",
                None,
                "",
                0,
                false
            ),
            "error"
        );
        assert_eq!(
            canonical_tool_status("surface_degraded", None, "", 0, false),
            "error"
        );
    }

    #[test]
    fn canonical_tool_execution_receipt_uses_surface_error_from_status_when_error_empty() {
        let row = canonical_tool_execution_receipt(
            "toolcall_surface_1",
            "batch_query",
            "web_tool_surface_degraded",
            Some(false),
            "",
            0,
            0,
            0,
            false,
        );
        assert_eq!(row.get("status").and_then(Value::as_str), Some("error"));
        assert_eq!(
            row.get("error_code").and_then(Value::as_str),
            Some("web_tool_surface_degraded")
        );
    }

    #[test]
    fn canonical_tooling_fallback_copy_includes_status_and_error_code() {
        let copy = canonical_tooling_fallback_copy("parse_failed", "web_tool_invalid_response", None);
        let lowered = copy.to_ascii_lowercase();
        assert!(lowered.contains("web_status: parse_failed"));
        assert!(lowered.contains("error_code: web_tool_invalid_response"));
    }

    #[test]
    fn normalize_web_tooling_error_code_maps_surface_errors() {
        assert_eq!(
            normalize_web_tooling_error_code("web_search_tool_surface_unavailable"),
            "web_tool_surface_unavailable"
        );
        assert_eq!(
            normalize_web_tooling_error_code("tool surface is degraded"),
            "web_tool_surface_degraded"
        );
    }

    #[test]
    fn canonical_tooling_fallback_copy_uses_surface_specific_copy() {
        let unavailable =
            canonical_tooling_fallback_copy("failed", "web_tool_surface_unavailable", None);
        let degraded =
            canonical_tooling_fallback_copy("failed", "web_tool_surface_degraded", None);
        assert!(
            unavailable
                .to_ascii_lowercase()
                .contains("web tool surface is unavailable"),
            "{unavailable}"
        );
        assert!(
            degraded
                .to_ascii_lowercase()
                .contains("web tool surface is degraded"),
            "{degraded}"
        );
    }

    #[test]
    fn canonical_tooling_fallback_copy_normalizes_surface_alias_to_surface_copy() {
        let copy = canonical_tooling_fallback_copy("failed", "surface_unavailable", None);
        assert!(
            copy.to_ascii_lowercase()
                .contains("web tool surface is unavailable"),
            "{copy}"
        );
    }
}
