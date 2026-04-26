include!("illusion_integrity_auditor.rs");

const MAX_SELF_AUDIT_SUBJECT_CHARS: usize = 120;

fn assim121_strip_invisible_unicode(raw: &str) -> String {
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

pub fn normalize_self_audit_subject(raw: &str) -> String {
    assim121_strip_invisible_unicode(raw)
        .chars()
        .filter(|ch| !ch.is_control() || *ch == '\n' || *ch == '\t')
        .map(|ch| if ch.is_whitespace() { ' ' } else { ch })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ")
        .chars()
        .take(MAX_SELF_AUDIT_SUBJECT_CHARS)
        .collect::<String>()
}

fn has_parent_segment(raw: &str) -> bool {
    raw.split(['/', '\\']).any(|segment| segment.trim() == "..")
}

pub fn normalize_self_audit_subject_with_contract(
    raw: &str,
    strict_contract: bool,
) -> (String, bool, &'static str) {
    let normalized = normalize_self_audit_subject(raw);
    if normalized.is_empty() {
        return (normalized, false, "self_audit_subject_empty_after_normalization");
    }
    if strict_contract && has_parent_segment(&normalized) {
        return (
            normalized,
            false,
            "self_audit_subject_invalid_under_strict_contract",
        );
    }
    (normalized, true, "self_audit_subject_contract_ok")
}

pub fn classify_self_audit_error_kind(raw: &str) -> &'static str {
    let folded = normalize_self_audit_subject(raw).to_ascii_lowercase();
    if folded.contains("timeout") {
        "timeout"
    } else if folded.contains("denied") || folded.contains("forbidden") {
        "authorization"
    } else if folded.contains("not found") || folded.contains("missing") {
        "not_found"
    } else {
        "unknown"
    }
}

pub fn should_retry_self_audit_transport_error(raw: &str) -> bool {
    let folded = normalize_self_audit_subject(raw).to_ascii_lowercase();
    [
        "408",
        "425",
        "429",
        "500",
        "502",
        "503",
        "504",
        "timeout",
        "temporarily",
        "unavailable",
        "reset",
        "closed",
    ]
        .iter()
        .any(|needle| folded.contains(needle))
}

#[cfg(test)]
mod assim121_self_audit_runtime_tests {
    use super::*;

    #[test]
    fn self_audit_subject_is_sanitized() {
        assert_eq!(
            normalize_self_audit_subject("  audit\u{200B}\u{0000} subject "),
            "audit subject"
        );
    }

    #[test]
    fn self_audit_error_kinds_are_classified() {
        assert_eq!(classify_self_audit_error_kind("request timeout"), "timeout");
        assert_eq!(classify_self_audit_error_kind("permission denied"), "authorization");
        assert_eq!(classify_self_audit_error_kind("record not found"), "not_found");
        assert_eq!(classify_self_audit_error_kind("misc"), "unknown");
    }

    #[test]
    fn self_audit_retry_classifier_detects_transient_transport_errors() {
        assert!(should_retry_self_audit_transport_error(
            "request timeout while upstream unavailable"
        ));
        assert!(!should_retry_self_audit_transport_error("authorization denied"));
    }
}
