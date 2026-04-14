// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)
//
// Imported pattern contract (RTK intake):
// - source: local/workspace/vendor/rtk/src/core/toml_filter.rs
// - concept: match_output short-circuit rules with optional "unless" guard.

use regex::Regex;
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
    "The tool path ran, but this turn only produced low-signal or no-result output. Retry with a narrower query or one specific source URL and I’ll give you a source-backed answer.";

const UNSYNTHESIZED_WEB_MARKERS: &[&str] = &[
    "from web retrieval:",
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
    "no relevant results found for that request yet",
    "couldn't produce source-backed findings in this turn",
    "don't have usable tool findings from this turn yet",
    "low-signal or no-result output",
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
        || lowered.contains("search function isn't currently operational")
        || lowered.contains("search function is not currently operational")
        || lowered.contains("blocking tool execution attempts")
        || lowered.contains("tool execution attempts")
        || lowered.contains("function call format")
        || lowered.contains("ack_only")
        || lowered.contains("external tool execution")
        || lowered.contains("recognized function");
    if !mentions_blocked_search_lane {
        return false;
    }
    let mentions_execution_denial = lowered.contains("didn't execute")
        || lowered.contains("did not execute")
        || lowered.contains("couldn't execute")
        || lowered.contains("could not execute")
        || lowered.contains("execution permissions are currently restricted")
        || lowered.contains("execution permissions are restricted");
    mentions_execution_denial
        || lowered.contains("likely reasons")
        || lowered.contains("configuration restrictions")
        || lowered.contains("authentication issues")
        || lowered.contains("rate limiting")
        || lowered.contains("intentional design")
        || lowered.contains("sandboxed")
        || lowered.contains("policy restrictions")
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
    fn detects_key_findings_potential_sources_scaffold_without_urls() {
        assert!(matches_ack_placeholder(
            "Key findings for \"Infring AI\": - Potential sources: nlplogix.com, gartner.com."
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
}
