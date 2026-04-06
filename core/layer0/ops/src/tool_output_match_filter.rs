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
                pattern: r"(?is)^\s*(?:i\s+)?could(?:n't| not)\s+extract\s+(?:usable|reliable)\s+findings(?:\b.*)?$",
                unless: Some(r"(?is)\b(?:key finding|sources?:|according to)\b"),
            },
            MatchRule {
                pattern: r"(?is)key findings for .*potential sources:",
                unless: Some(r"(?is)https?://"),
            },
            MatchRule {
                pattern: r"(?is)could(?:n't| not) extract (?:usable|reliable) findings.*search response came from https?://duckduckgo\.com/html/\?q=",
                unless: Some(r"(?is)\b(?:key finding|sources?:|according to)\b"),
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
}
