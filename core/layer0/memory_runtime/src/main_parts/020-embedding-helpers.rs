include!("embedding_helpers_parts/010-normalize-file-ref.rs");
include!("embedding_helpers_parts/015-runtime-index-entry-helpers.rs");
include!("embedding_helpers_parts/020-build-embedding-map-from-entries.rs");
include!("embedding_helpers_parts/030-extract-date-from-path.rs");

const MAX_EMBEDDING_SOURCE_PATHS: usize = 512;

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

pub fn normalize_embedding_ref_token(raw: &str) -> String {
    assim121_strip_invisible_unicode(raw)
        .chars()
        .filter(|ch| !ch.is_control() || *ch == '\n' || *ch == '\t')
        .map(|ch| if ch.is_whitespace() { ' ' } else { ch })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ")
        .chars()
        .take(240)
        .collect::<String>()
}

fn has_parent_segment(raw: &str) -> bool {
    raw.split(['/', '\\']).any(|segment| segment.trim() == "..")
}

fn is_absolute_like_path(raw: &str) -> bool {
    raw.starts_with('/') || raw.starts_with('\\') || raw.get(1..3) == Some(":\\")
}

pub fn normalize_embedding_source_paths(raw_paths: &[String]) -> Vec<String> {
    let mut paths = raw_paths
        .iter()
        .map(|path| normalize_embedding_ref_token(path))
        .filter(|path| !path.is_empty())
        .take(MAX_EMBEDDING_SOURCE_PATHS)
        .collect::<Vec<String>>();
    paths.sort();
    paths.dedup();
    paths
}

pub fn normalize_embedding_source_paths_with_contract(
    raw_paths: &[String],
    strict_contract: bool,
) -> (Vec<String>, bool, &'static str) {
    let normalized = normalize_embedding_source_paths(raw_paths);
    if normalized.is_empty() {
        return (
            normalized,
            false,
            "embedding_paths_empty_after_normalization",
        );
    }
    let truncated = raw_paths.len() > MAX_EMBEDDING_SOURCE_PATHS;
    let traversal_like = normalized.iter().any(|path| has_parent_segment(path));
    let absolute_like = normalized.iter().any(|path| is_absolute_like_path(path));
    if strict_contract && traversal_like {
        return (
            normalized,
            false,
            "embedding_paths_parent_traversal_blocked_under_strict_contract",
        );
    }
    if strict_contract && absolute_like {
        return (
            normalized,
            false,
            "embedding_paths_absolute_blocked_under_strict_contract",
        );
    }
    if strict_contract && truncated {
        return (
            normalized,
            false,
            "embedding_paths_truncated_under_strict_contract",
        );
    }
    let reason = if truncated {
        "embedding_paths_truncated_non_strict_contract"
    } else {
        "embedding_paths_contract_ok"
    };
    (normalized, true, reason)
}

#[cfg(test)]
mod assim121_embedding_helper_tests {
    use super::*;

    #[test]
    fn embedding_ref_token_strips_hidden_and_control_chars() {
        assert_eq!(
            normalize_embedding_ref_token("  src/\u{200B}main.rs\u{0000}  "),
            "src/main.rs"
        );
    }

    #[test]
    fn source_paths_are_deduped_and_sorted() {
        let raw = vec![
            "src/b.rs".to_string(),
            "src/a.rs".to_string(),
            "src/a.rs".to_string(),
        ];
        assert_eq!(
            normalize_embedding_source_paths(&raw),
            vec!["src/a.rs".to_string(), "src/b.rs".to_string()]
        );
    }

    #[test]
    fn strict_embedding_contract_rejects_parent_traversal_like_paths() {
        let raw = vec!["../secret.bin".to_string()];
        let (_normalized, ok, reason) = normalize_embedding_source_paths_with_contract(&raw, true);
        assert!(!ok);
        assert_eq!(
            reason,
            "embedding_paths_parent_traversal_blocked_under_strict_contract"
        );
    }
}
