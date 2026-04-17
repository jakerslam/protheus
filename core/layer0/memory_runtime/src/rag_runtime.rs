// AUTO-SPLIT: this file is composed from smaller parts to enforce <=1000 line policy.
include!("rag_runtime_parts/010-now-iso.rs");
include!("rag_runtime_parts/020-ingest-payload.rs");
include!("rag_runtime_parts/030-byterover-upgrade-payload.rs");
include!("rag_runtime_parts/040-memory-share-payload.rs");
include!("rag_runtime_parts/050-base-args.rs");

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

pub fn should_retry_rag_transport_error(raw_error: &str) -> bool {
    let folded = normalize_rag_query_text(raw_error, 240).to_ascii_lowercase();
    ["429", "timeout", "connect", "reset", "closed", "unavailable", "temporarily"]
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
        assert!(should_retry_rag_transport_error("Request failed with 429 timeout"));
        assert!(!should_retry_rag_transport_error("permission denied"));
        assert_eq!(normalize_rag_retry_attempts(0), 1);
        assert_eq!(normalize_rag_retry_attempts(99), 8);
    }
}
