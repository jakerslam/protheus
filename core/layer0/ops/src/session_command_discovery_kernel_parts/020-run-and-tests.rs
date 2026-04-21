fn usage() {
    lane_utils::print_json_line(&json!({
        "ok": true,
        "type": "session_command_discovery_usage",
        "command": "session-command-discovery",
        "commands": ["report", "classify", "detail", "split"],
        "flags": ["--payload-json=...", "--limit=<n>"]
    }));
}

fn payload_json(args: &[String]) -> Result<Value, String> {
    lane_utils::payload_json(args, "session_command_discovery_kernel")
}

fn payload_obj(value: &Value) -> &serde_json::Map<String, Value> {
    lane_utils::payload_obj(value)
}

pub fn run(_root: &std::path::Path, argv: &[String]) -> i32 {
    let command = argv
        .first()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "help".to_string());

    if matches!(command.as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }

    let payload = match payload_json(&argv[1..]) {
        Ok(value) => value,
        Err(err) => {
            lane_utils::print_json_line(&lane_utils::cli_error(
                "session_command_discovery_kernel_error",
                &err,
            ));
            return 1;
        }
    };
    let input = payload_obj(&payload);

    let limit = input
        .get("limit")
        .and_then(Value::as_u64)
        .map(|v| v.clamp(1, 256) as usize)
        .unwrap_or(16);

    let commands = input
        .get("commands")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|s| clean_text(s, 400))
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let out = match command.as_str() {
        "report" | "classify" => {
            let mut report = classify_command_list(commands.as_slice(), limit.max(1));
            if let Some(obj) = report.as_object_mut() {
                obj.insert("execution_receipt".to_string(), json!({
                    "lane": "session_command_discovery_kernel",
                    "command": command,
                    "status": "success"
                }));
            }
            lane_utils::cli_receipt("session_command_discovery_kernel_report", report)
        }
        "detail" => {
            let cmd = input
                .get("command")
                .and_then(Value::as_str)
                .map(|s| clean_text(s, 400))
                .unwrap_or_default();
            lane_utils::cli_receipt(
                "session_command_discovery_kernel_detail",
                classify_command_detail_for_kernel(&cmd),
            )
        }
        "split" => {
            let cmd = input
                .get("command")
                .and_then(Value::as_str)
                .map(|s| clean_text(s, 400))
                .unwrap_or_default();
            lane_utils::cli_receipt(
                "session_command_discovery_kernel_split",
                json!({
                    "ok": true,
                    "segments": split_command_chain_for_kernel(&cmd),
                }),
            )
        }
        _ => lane_utils::cli_error(
            "session_command_discovery_kernel_error",
            "session_command_discovery_kernel_unknown_command",
        ),
    };

    let ok = out.get("ok").and_then(Value::as_bool).unwrap_or(false);
    lane_utils::print_json_line(&out);
    if ok { 0 } else { 1 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_report_recognizes_git_status() {
        let report = classify_command_list_for_kernel(&["git status".to_string()], 8);
        assert_eq!(report.get("ok").and_then(Value::as_bool), Some(true));
        let supported = report
            .get("supported")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(!supported.is_empty());
    }

    #[test]
    fn classify_detail_recognizes_wrapper_prefixed_git_command() {
        let detail = classify_command_detail_for_kernel("sudo env FOO=bar /usr/bin/git status");
        assert_eq!(detail.get("supported").and_then(Value::as_bool), Some(true));
        assert_eq!(
            detail.get("canonical").and_then(Value::as_str),
            Some("infring git")
        );
        assert_eq!(detail.get("command_key").and_then(Value::as_str), Some("status"));
    }

    #[test]
    fn classify_detail_recognizes_wrapper_prefixed_quoted_cargo_command() {
        let detail = classify_command_detail_for_kernel("command \"/usr/local/bin/cargo\" test");
        assert_eq!(detail.get("supported").and_then(Value::as_bool), Some(true));
        assert_eq!(
            detail.get("canonical").and_then(Value::as_str),
            Some("infring cargo")
        );
        assert_eq!(detail.get("command_key").and_then(Value::as_str), Some("test"));
    }
}
