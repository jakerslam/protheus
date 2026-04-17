include!("embedding_helpers_parts/010-normalize-file-ref.rs");
include!("embedding_helpers_parts/015-runtime-index-entry-helpers.rs");
include!("embedding_helpers_parts/020-build-embedding-map-from-entries.rs");
include!("embedding_helpers_parts/030-extract-date-from-path.rs");

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

pub fn normalize_embedding_source_paths(raw_paths: &[String]) -> Vec<String> {
    let mut paths = raw_paths
        .iter()
        .map(|path| normalize_embedding_ref_token(path))
        .filter(|path| !path.is_empty())
        .collect::<Vec<String>>();
    paths.sort();
    paths.dedup();
    paths
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
}
