// AUTO-SPLIT: this file is composed from smaller parts to enforce <=1000 line policy.
include!("rag_runtime_parts/010-now-iso.rs");
include!("rag_runtime_parts/020-ingest-payload.rs");
include!("rag_runtime_parts/030-byterover-upgrade-payload.rs");
include!("rag_runtime_parts/040-memory-share-payload.rs");
include!("rag_runtime_parts/050-base-args.rs");

const RAG_CONTEXT_WINDOW_HARD_MIN_TOKENS: u32 = 16_000;
const RAG_CONTEXT_WINDOW_WARN_BELOW_TOKENS: u32 = 32_000;

fn assim120_strip_invisible_unicode(raw: &str) -> String {
    raw.chars()
        .filter(|ch| {
            !matches!(
                *ch,
                '\u{200B}'
                    | '\u{200C}'
                    | '\u{200D}'
                    | '\u{200E}'
                    | '\u{200F}'
                    | '\u{202A}'
                    | '\u{202B}'
                    | '\u{202C}'
                    | '\u{202D}'
                    | '\u{202E}'
                    | '\u{2060}'
                    | '\u{FEFF}'
            )
        })
        .collect::<String>()
}

pub fn normalize_rag_query_text(raw: &str, max_chars: usize) -> String {
    let bounded = max_chars.clamp(1, 8192);
    assim120_strip_invisible_unicode(raw)
        .chars()
        .filter(|ch| !ch.is_control() || *ch == '\n' || *ch == '\t')
        .map(|ch| if ch.is_whitespace() { ' ' } else { ch })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ")
        .chars()
        .take(bounded)
        .collect::<String>()
}

pub fn normalize_rag_retry_attempts(requested: u16) -> u16 {
    requested.clamp(1, 8)
}

pub fn evaluate_rag_context_window_guard(tokens: u32) -> (bool, bool) {
    let safe = tokens.min(2_000_000);
    let should_warn = safe > 0 && safe < RAG_CONTEXT_WINDOW_WARN_BELOW_TOKENS;
    let should_block = safe > 0 && safe < RAG_CONTEXT_WINDOW_HARD_MIN_TOKENS;
    (should_warn, should_block)
}

pub fn should_retry_rag_transport_error(raw_error: &str) -> bool {
    let folded = normalize_rag_query_text(raw_error, 240).to_ascii_lowercase();
    [
        "429",
        "timeout",
        "connect",
        "reset",
        "closed",
        "unavailable",
        "temporarily",
    ]
    .iter()
    .any(|needle| folded.contains(needle))
}

#[cfg(test)]
mod assim120_rag_runtime_tests {
    use super::*;

    #[test]
    fn rag_query_text_sanitization_is_bounded() {
        let out = normalize_rag_query_text("a\u{200B}\u{0000} b", 3);
        assert_eq!(out, "a b");
    }

    #[test]
    fn rag_retry_classifier_matches_transient_errors() {
        assert!(should_retry_rag_transport_error(
            "Request failed with 429 timeout"
        ));
        assert!(!should_retry_rag_transport_error("permission denied"));
        assert_eq!(normalize_rag_retry_attempts(0), 1);
        assert_eq!(normalize_rag_retry_attempts(99), 8);
    }

    #[test]
    fn rag_context_window_guard_warns_and_blocks_for_small_windows() {
        let (warn, block) = evaluate_rag_context_window_guard(12_000);
        assert!(warn);
        assert!(block);
        let (warn2, block2) = evaluate_rag_context_window_guard(64_000);
        assert!(!warn2);
        assert!(!block2);
    }
}
