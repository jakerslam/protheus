include!("../../legacy_rust_sources/self_audit/illusion_integrity_auditor.rs");

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
        .collect::<String>()
        .trim()
        .chars()
        .take(120)
        .collect::<String>()
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
}
