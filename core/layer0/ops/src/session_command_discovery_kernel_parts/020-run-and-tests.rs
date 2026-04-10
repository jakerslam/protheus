fn usage() {
    println!("session-command-discovery-kernel commands:");
    println!("  protheus-ops session-command-discovery-kernel classify [--payload=<json>|--payload-base64=<json>]");
    println!("  protheus-ops session-command-discovery-kernel classify-text [--payload=<json>|--payload-base64=<json>]");
}

fn command_list_from_payload(payload: &Map<String, Value>) -> Vec<String> {
    let mut out = Vec::<String>::new();
    if let Some(commands) = payload.get("commands").and_then(Value::as_array) {
        for row in commands {
            let command = clean_text(row.as_str().unwrap_or(""), 600);
            if !command.is_empty() {
                out.push(command);
            }
        }
    }
    out
}

fn command_list_from_text(payload: &Map<String, Value>) -> Vec<String> {
    let text = clean_text(
        payload.get("text").and_then(Value::as_str).unwrap_or(""),
        64_000,
    );
    if text.is_empty() {
        return vec![];
    }
    text.lines()
        .map(|row| clean_text(row, 600))
        .filter(|row| !row.is_empty())
        .collect::<Vec<_>>()
}

pub fn run(_root: &std::path::Path, argv: &[String]) -> i32 {
    let command = argv
        .first()
        .map(|value| value.to_ascii_lowercase())
        .unwrap_or_else(|| "help".to_string());
    if matches!(command.as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }
    let payload = match lane_utils::payload_json(&argv[1..], "session_command_discovery") {
        Ok(payload) => payload,
        Err(err) => {
            lane_utils::print_json_line(&lane_utils::cli_error("session_command_discovery_kernel_error", &err));
            return 1;
        }
    };
    let input = lane_utils::payload_obj(&payload);
    let limit = input.get("limit").and_then(Value::as_u64).unwrap_or(12) as usize;
    let result = match command.as_str() {
        "classify" => {
            let commands = command_list_from_payload(input);
            lane_utils::cli_receipt(
                "session_command_discovery_kernel_classify",
                classify_command_list(&commands, limit),
            )
        }
        "classify-text" => {
            let commands = command_list_from_text(input);
            lane_utils::cli_receipt(
                "session_command_discovery_kernel_classify_text",
                classify_command_list(&commands, limit),
            )
        }
        _ => lane_utils::cli_error(
            "session_command_discovery_kernel_error",
            "session_command_discovery_kernel_unknown_command",
        ),
    };
    let ok = result.get("ok").and_then(Value::as_bool).unwrap_or(false);
    lane_utils::print_json_line(&result);
    if ok {
        0
    } else {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_git_command_as_supported() {
        let classified = classify_command("git status");
        assert!(matches!(classified, Classification::Supported { .. }));
    }

    #[test]
    fn classify_strips_env_prefix_and_absolute_path() {
        let classified = classify_command("sudo FOO=bar /usr/bin/grep needle file.txt");
        assert!(matches!(
            classified,
            Classification::Supported {
                category: "Files",
                ..
            }
        ));
    }

    #[test]
    fn classify_handles_quoted_absolute_executable() {
        let classified = classify_command("\"/usr/bin/git\" status");
        assert!(matches!(classified, Classification::Supported { .. }));
    }

    #[test]
    fn extract_base_command_keeps_second_token_for_quoted_executable() {
        assert_eq!(
            extract_base_command("\"/usr/local/bin/cargo\" test --workspace"),
            "cargo test"
        );
    }

    #[test]
    fn split_chain_keeps_quoted_operators_inside_segment() {
        let rows = split_command_chain("echo \"a && b\" && git status; cargo test");
        assert_eq!(
            rows,
            vec![
                "echo \"a && b\"".to_string(),
                "git status".to_string(),
                "cargo test".to_string()
            ]
        );
    }

    #[test]
    fn classify_cat_redirect_as_unsupported() {
        let classified = classify_command("cat foo.txt > out.txt");
        assert!(matches!(
            classified,
            Classification::Unsupported { ref base_command } if base_command == "cat"
        ));
    }

    #[test]
    fn classify_text_payload_builds_report() {
        let report = classify_command_list(
            &vec![
                "git status".to_string(),
                "unknowncmd --help".to_string(),
                "cargo fmt".to_string(),
                "echo ok".to_string(),
            ],
            10,
        );
        assert_eq!(report.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            report.get("total_commands").and_then(Value::as_u64),
            Some(4)
        );
        assert_eq!(
            report.get("supported_count").and_then(Value::as_u64),
            Some(2)
        );
        assert_eq!(
            report.get("unsupported_count").and_then(Value::as_u64),
            Some(1)
        );
    }

    #[test]
    fn classify_explicit_batch_query_tool_alias_as_supported() {
        let classified = classify_command("tool::batch_query latest runtime benchmarks");
        assert!(matches!(
            classified,
            Classification::Supported {
                command_key,
                canonical,
                ..
            } if command_key == "batch-query" && canonical == "infring batch-query"
        ));
    }
}
