// AUTO-SPLIT: this file is composed from smaller parts to enforce <=1000 line policy.
include!("wave1_parts/010-now-iso.rs");
include!("wave1_parts/020-build-matrix-payload.rs");
include!("wave1_parts/030-auto-recall-filed-payload.rs");
include!("wave1_parts/040-memory-matrix-payload.rs");

const MAX_WAVE1_SUBJECTS: usize = 128;

fn strip_invisible_unicode(raw: &str) -> String {
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

pub fn normalize_wave1_subject_token(raw: &str) -> String {
    let mut out = String::new();
    for ch in strip_invisible_unicode(raw).trim().chars() {
        let normalized = if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | ':' | '.') {
            ch.to_ascii_lowercase()
        } else if ch.is_whitespace() {
            '_'
        } else {
            continue;
        };
        if normalized == '_' && out.ends_with('_') {
            continue;
        }
        out.push(normalized);
        if out.len() >= 80 {
            break;
        }
    }
    out
        .chars()
        .trim_matches('_')
        .collect::<String>()
}

pub fn normalize_wave1_subjects(subjects: &[String]) -> Vec<String> {
    let mut out = subjects
        .iter()
        .map(|subject| normalize_wave1_subject_token(subject))
        .filter(|subject| !subject.is_empty())
        .take(MAX_WAVE1_SUBJECTS)
        .collect::<Vec<String>>();
    out.sort();
    out.dedup();
    out
}

pub fn normalize_wave1_subjects_with_contract(
    subjects: &[String],
    strict_contract: bool,
) -> (Vec<String>, bool, &'static str) {
    let normalized = normalize_wave1_subjects(subjects);
    if normalized.is_empty() {
        return (
            normalized,
            false,
            "wave1_subjects_empty_after_normalization",
        );
    }
    if strict_contract {
        let modified_or_truncated = subjects.len() > MAX_WAVE1_SUBJECTS
            || subjects.iter().any(|subject| {
                normalize_wave1_subject_token(subject) != subject.trim().to_ascii_lowercase()
            });
        if modified_or_truncated {
            return (
                normalized,
                false,
                "wave1_subjects_modified_under_strict_contract",
            );
        }
    }
    (normalized, true, "wave1_subjects_contract_ok")
}

pub fn wave1_has_subjects(subjects: &[String]) -> bool {
    normalize_wave1_subjects_with_contract(subjects, false).1
}

#[cfg(test)]
mod assim120_wave1_tests {
    use super::*;

    #[test]
    fn subject_normalization_is_deduped_and_sorted() {
        let subjects = vec![
            " Team:Ops ".to_string(),
            "team:ops".to_string(),
            "Sec-Core".to_string(),
        ];
        assert_eq!(
            normalize_wave1_subjects(&subjects),
            vec!["sec-core".to_string(), "team:ops".to_string()]
        );
    }

    #[test]
    fn wave1_gate_is_fail_closed_for_empty_subjects() {
        assert!(!wave1_has_subjects(&[]));
        assert!(!wave1_has_subjects(&[" \u{200B} ".to_string()]));
        assert!(wave1_has_subjects(&["system:runtime".to_string()]));
    }

    #[test]
    fn strict_subject_contract_rejects_sanitized_tokens() {
        let (_normalized, ok, reason) =
            normalize_wave1_subjects_with_contract(&["Team Ops".to_string()], true);
        assert!(!ok);
        assert_eq!(reason, "wave1_subjects_modified_under_strict_contract");
    }
}
