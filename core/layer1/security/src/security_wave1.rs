include!("security_wave1_parts/010-now-iso.rs");
include!("security_wave1_parts/020-run-capability-switchboard.rs");
include!("security_wave1_parts/030-black-box-paths.rs");
include!("security_wave1_parts/040-append-black-box-entry.rs");
include!("security_wave1_parts/050-run-black-box-ledger.rs");
include!("security_wave1_parts/060-parse-blocked-pattern-match.rs");
include!("security_wave1_parts/070-dream-warden-load-policy.rs");
include!("security_wave1_parts/080-directive-hierarchy-paths.rs");
include!("security_wave1_parts/090-default.rs");
include!("security_wave1_parts/100-run-truth-seeking-gate.rs");
include!("security_wave1_parts/110-abac-denies-when-no-rule-matches.rs");

const MAX_SECURITY_SUBJECTS: usize = 128;

pub fn normalize_security_subject(raw: &str) -> String {
    raw.trim()
        .chars()
        .filter_map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, ':' | '-' | '_' | '.') {
                Some(ch.to_ascii_lowercase())
            } else if ch.is_whitespace() {
                Some('_')
            } else {
                None
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .chars()
        .take(96)
        .collect::<String>()
}

pub fn normalize_subject_list(subjects: &[String]) -> Vec<String> {
    let mut out: Vec<String> = subjects
        .iter()
        .map(|subject| normalize_security_subject(subject))
        .filter(|subject| !subject.is_empty())
        .take(MAX_SECURITY_SUBJECTS)
        .collect::<Vec<String>>();
    out.sort();
    out.dedup();
    out
}

pub fn normalize_subject_list_with_contract(
    subjects: &[String],
    strict_contract: bool,
) -> (Vec<String>, bool, &'static str) {
    let normalized = normalize_subject_list(subjects);
    if normalized.is_empty() {
        return (normalized, false, "subjects_empty_after_normalization");
    }
    if strict_contract {
        let dropped_or_changed = subjects
            .iter()
            .any(|subject| normalize_security_subject(subject) != subject.trim().to_ascii_lowercase());
        if dropped_or_changed || subjects.len() > MAX_SECURITY_SUBJECTS {
            return (normalized, false, "subjects_sanitized_or_truncated_under_strict_contract");
        }
    }
    (normalized, true, "subjects_contract_ok")
}

pub fn fail_closed_subject_gate(subjects: &[String]) -> bool {
    normalize_subject_list_with_contract(subjects, false).1
}

#[cfg(test)]
mod security_wave1_subject_normalization_tests {
    use super::*;

    #[test]
    fn subject_normalization_drops_control_or_invisible_chars() {
        assert_eq!(
            normalize_security_subject("Admin:\u{200B}Root\u{0000} "),
            "admin:root"
        );
    }

    #[test]
    fn fail_closed_subject_gate_denies_empty_subjects() {
        assert!(!fail_closed_subject_gate(&[]));
        assert!(!fail_closed_subject_gate(&[" \u{200B} ".to_string()]));
        assert!(fail_closed_subject_gate(&["system:operator".to_string()]));
    }

    #[test]
    fn strict_subject_contract_rejects_sanitized_entries() {
        let (_normalized, ok, reason) =
            normalize_subject_list_with_contract(&["Admin Root".to_string()], true);
        assert!(!ok);
        assert_eq!(
            reason,
            "subjects_sanitized_or_truncated_under_strict_contract"
        );
    }
}
