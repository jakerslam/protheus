include!("security_planes_parts/010-now-iso.rs");
include!("security_planes_parts/020-run-guard.rs");

pub fn normalize_security_plane_name(raw: &str) -> String {
    raw.trim()
        .chars()
        .filter_map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | ':' | '.') {
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

pub fn normalize_security_plane_list(planes: &[String]) -> Vec<String> {
    let mut out = planes
        .iter()
        .map(|plane| normalize_security_plane_name(plane))
        .filter(|plane| !plane.is_empty())
        .collect::<Vec<String>>();
    out.sort();
    out.dedup();
    out
}

pub fn security_planes_fail_closed(planes: &[String]) -> bool {
    !normalize_security_plane_list(planes).is_empty()
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
}
