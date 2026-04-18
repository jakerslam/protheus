include!("main_parts/010-strip-ticks.rs");
include!("main_parts/020-embedding-helpers.rs");
include!("main_parts/030-extract-tags-from-chunk.rs");
include!("main_parts/035-hybrid-recall.rs");
include!("main_parts/040-query-index-payload.rs");
include!("main_parts/050-run-query-index.rs");
include!("main_parts/055-predictive-defrag.rs");
include!("main_parts/060-run-daemon.rs");
include!("main_parts/070-cli-entrypoint.rs");

const MAX_MEMORY_RUNTIME_CLI_ARGS: usize = 64;

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

pub fn normalize_memory_runtime_cli_arg(raw: &str) -> String {
    assim120_strip_invisible_unicode(raw)
        .chars()
        .filter(|ch| !ch.is_control() || *ch == '\n' || *ch == '\t')
        .map(|ch| if ch.is_whitespace() { ' ' } else { ch })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ")
        .chars()
        .take(160)
        .collect::<String>()
}

pub fn normalize_memory_runtime_cli_args_with_contract(
    raw_args: &[String],
    strict_contract: bool,
) -> (Vec<String>, bool, &'static str) {
    let normalized = raw_args
        .iter()
        .map(|arg| normalize_memory_runtime_cli_arg(arg))
        .filter(|arg| !arg.is_empty())
        .take(MAX_MEMORY_RUNTIME_CLI_ARGS)
        .collect::<Vec<String>>();
    if normalized.is_empty() {
        return (normalized, false, "args_empty_after_normalization");
    }
    let truncated = raw_args.len() > MAX_MEMORY_RUNTIME_CLI_ARGS;
    let traversal_like = normalized.iter().any(|arg| arg.contains(".."));
    if strict_contract && traversal_like {
        return (
            normalized,
            false,
            "args_parent_traversal_blocked_under_strict_contract",
        );
    }
    if strict_contract && truncated {
        return (normalized, false, "args_truncated_under_strict_contract");
    }
    let reason = if truncated {
        "args_truncated_non_strict_contract"
    } else {
        "args_contract_ok"
    };
    (normalized, true, reason)
}

pub fn normalize_memory_runtime_transport_mode(raw: &str) -> &'static str {
    match normalize_memory_runtime_cli_arg(raw)
        .to_ascii_lowercase()
        .replace('-', "_")
        .as_str()
    {
        "strict" => "strict",
        "trusted_env_proxy" | "trusted" | "env_proxy" => "trusted_env_proxy",
        _ => "strict",
    }
}

#[cfg(test)]
mod assim120_runtime_main_tests {
    use super::*;

    #[test]
    fn strips_invisible_and_control_chars_from_cli_args() {
        assert_eq!(
            normalize_memory_runtime_cli_arg("  hello\u{200B}\u{0000}   world "),
            "hello world"
        );
    }

    #[test]
    fn transport_mode_aliases_normalize_to_known_values() {
        assert_eq!(
            normalize_memory_runtime_transport_mode("trusted-env-proxy"),
            "trusted_env_proxy"
        );
        assert_eq!(normalize_memory_runtime_transport_mode("unknown"), "strict");
    }

    #[test]
    fn strict_cli_arg_contract_blocks_parent_traversal_like_tokens() {
        let raw = vec!["--root=../tmp".to_string()];
        let (_normalized, ok, reason) = normalize_memory_runtime_cli_args_with_contract(&raw, true);
        assert!(!ok);
        assert_eq!(
            reason,
            "args_parent_traversal_blocked_under_strict_contract"
        );
    }
}
