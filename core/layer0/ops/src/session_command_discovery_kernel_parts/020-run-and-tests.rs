fn usage() {
    println!("session-command-discovery-kernel commands:");
    println!("  protheus-ops session-command-discovery-kernel classify [--payload=<json>|--payload-base64=<json>]");
    println!("  protheus-ops session-command-discovery-kernel classify-text [--payload=<json>|--payload-base64=<json>]");
}

fn print_json_line(value: &Value) {
    println!(
        "{}",
        serde_json::to_string(value)
            .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode_failed\"}".to_string())
    );
}

fn cli_receipt(kind: &str, payload: Value) -> Value {
    let ts = now_iso();
    let ok = payload.get("ok").and_then(Value::as_bool).unwrap_or(true);
    let mut out = json!({
        "ok": ok,
        "type": kind,
        "ts": ts,
        "date": ts[..10].to_string(),
        "payload": payload
    });
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    out
}

fn cli_error(kind: &str, error: &str) -> Value {
    let ts = now_iso();
    let mut out = json!({
        "ok": false,
        "type": kind,
        "ts": ts,
        "date": ts[..10].to_string(),
        "error": error,
        "fail_closed": true
    });
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    out
}

fn payload_json(argv: &[String]) -> Result<Value, String> {
    if let Some(raw) = lane_utils::parse_flag(argv, "payload", false) {
        return serde_json::from_str::<Value>(&raw)
            .map_err(|err| format!("session_command_discovery_payload_decode_failed:{err}"));
    }
    if let Some(raw_b64) = lane_utils::parse_flag(argv, "payload-base64", false) {
        let bytes = BASE64_STANDARD.decode(raw_b64.as_bytes()).map_err(|err| {
            format!("session_command_discovery_payload_base64_decode_failed:{err}")
        })?;
        let text = String::from_utf8(bytes)
            .map_err(|err| format!("session_command_discovery_payload_utf8_decode_failed:{err}"))?;
        return serde_json::from_str::<Value>(&text)
            .map_err(|err| format!("session_command_discovery_payload_decode_failed:{err}"));
    }
    Ok(json!({}))
}

fn payload_obj<'a>(value: &'a Value) -> &'a Map<String, Value> {
    value.as_object().unwrap_or_else(|| {
        static EMPTY: OnceLock<Map<String, Value>> = OnceLock::new();
        EMPTY.get_or_init(Map::new)
    })
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
    let payload = match payload_json(&argv[1..]) {
        Ok(payload) => payload,
        Err(err) => {
            print_json_line(&cli_error("session_command_discovery_kernel_error", &err));
            return 1;
        }
    };
    let input = payload_obj(&payload);
    let limit = input.get("limit").and_then(Value::as_u64).unwrap_or(12) as usize;
    let result = match command.as_str() {
        "classify" => {
            let commands = command_list_from_payload(input);
            cli_receipt(
                "session_command_discovery_kernel_classify",
                classify_command_list(&commands, limit),
            )
        }
        "classify-text" => {
            let commands = command_list_from_text(input);
            cli_receipt(
                "session_command_discovery_kernel_classify_text",
                classify_command_list(&commands, limit),
            )
        }
        _ => cli_error(
            "session_command_discovery_kernel_error",
            "session_command_discovery_kernel_unknown_command",
        ),
    };
    let ok = result.get("ok").and_then(Value::as_bool).unwrap_or(false);
    print_json_line(&result);
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
}
