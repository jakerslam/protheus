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
        .collect::<Vec<String>>();
    out.sort();
    out.dedup();
    out
}

pub fn fail_closed_subject_gate(subjects: &[String]) -> bool {
    !normalize_subject_list(subjects).is_empty()
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
}
