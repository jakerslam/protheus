include!("security_planes_parts/010-now-iso.rs");
include!("security_planes_parts/020-run-guard.rs");

const MAX_SECURITY_PLANES: usize = 128;

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

pub fn normalize_security_plane_name(raw: &str) -> String {
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
        if out.len() >= 96 {
            break;
        }
    }
    out.trim_matches('_').to_string()
}

pub fn normalize_security_plane_list(planes: &[String]) -> Vec<String> {
    let mut out = planes
        .iter()
        .map(|plane| normalize_security_plane_name(plane))
        .filter(|plane| !plane.is_empty())
        .take(MAX_SECURITY_PLANES)
        .collect::<Vec<String>>();
    out.sort();
    out.dedup();
    out
}

pub fn normalize_security_plane_list_with_contract(
    planes: &[String],
    strict_contract: bool,
) -> (Vec<String>, bool, &'static str) {
    let normalized = normalize_security_plane_list(planes);
    if normalized.is_empty() {
        return (normalized, false, "security_planes_empty_after_normalization");
    }
    if strict_contract {
        let modified_or_truncated = planes.len() > MAX_SECURITY_PLANES
            || planes
                .iter()
                .any(|plane| normalize_security_plane_name(plane) != plane.trim().to_ascii_lowercase());
        if modified_or_truncated {
            return (
                normalized,
                false,
                "security_planes_modified_under_strict_contract",
            );
        }
    }
    (normalized, true, "security_planes_contract_ok")
}

pub fn security_planes_fail_closed(planes: &[String]) -> bool {
    normalize_security_plane_list_with_contract(planes, false).1
}

#[cfg(test)]
mod assim121_security_planes_tests {
    use super::*;

    #[test]
    fn plane_names_are_deduped_and_sorted() {
        let raw = vec![
            "Guard-Core".to_string(),
            "guard:core".to_string(),
            "Runtime Ops".to_string(),
        ];
        assert_eq!(
            normalize_security_plane_list(&raw),
            vec![
                "guard-core".to_string(),
                "guard:core".to_string(),
                "runtime_ops".to_string()
            ]
        );
    }

    #[test]
    fn fail_closed_when_no_plane_name_survives_normalization() {
        assert!(!security_planes_fail_closed(&[]));
        assert!(!security_planes_fail_closed(&[" \u{200B} ".to_string()]));
        assert!(security_planes_fail_closed(&["identity".to_string()]));
    }

    #[test]
    fn strict_plane_contract_rejects_sanitized_tokens() {
        let (_normalized, ok, reason) =
            normalize_security_plane_list_with_contract(&["Runtime Ops".to_string()], true);
        assert!(!ok);
        assert_eq!(reason, "security_planes_modified_under_strict_contract");
    }
}
