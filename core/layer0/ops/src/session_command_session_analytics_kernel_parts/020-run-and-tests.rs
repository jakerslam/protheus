pub fn run(_root: &Path, argv: &[String]) -> i32 {
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
            print_json_line(&cli_error(
                "session_command_session_analytics_kernel_error",
                &err,
            ));
            return 1;
        }
    };
    let input = payload_obj(&payload);
    let limit = input.get("limit").and_then(Value::as_u64).unwrap_or(10) as usize;

    let response = match command.as_str() {
        "extract-jsonl" => {
            let session_id = clean_text(
                input
                    .get("session_id")
                    .and_then(Value::as_str)
                    .unwrap_or("session"),
                120,
            );
            let jsonl = clean_text(
                input.get("jsonl").and_then(Value::as_str).unwrap_or(""),
                200_000,
            );
            let rows = extract_commands_from_jsonl(&session_id, &jsonl);
            cli_receipt(
                "session_command_session_analytics_kernel_extract_jsonl",
                json!({
                  "ok": true,
                  "session_id": session_id,
                  "extracted_count": rows.len(),
                  "commands": rows.iter().map(|row| json!({
                    "command": row.command,
                    "output_len": row.output_len,
                    "output_preview": row.output_preview,
                    "is_error": row.is_error,
                    "sequence_index": row.sequence_index
                  })).collect::<Vec<_>>()
                }),
            )
        }
        "classify-jsonl" => {
            let session_id = clean_text(
                input
                    .get("session_id")
                    .and_then(Value::as_str)
                    .unwrap_or("session"),
                120,
            );
            let jsonl = clean_text(
                input.get("jsonl").and_then(Value::as_str).unwrap_or(""),
                200_000,
            );
            let rows = extract_commands_from_jsonl(&session_id, &jsonl);
            let commands = rows
                .iter()
                .map(|row| row.command.clone())
                .collect::<Vec<_>>();
            let mut report = classify_command_list_for_kernel(&commands, limit.max(1));
            report["session_id"] = Value::String(session_id);
            report["extracted_count"] = Value::from(rows.len() as u64);
            cli_receipt(
                "session_command_session_analytics_kernel_classify_jsonl",
                report,
            )
        }
        "adoption-report" => cli_receipt(
            "session_command_session_analytics_kernel_adoption_report",
            build_adoption_report(input, limit),
        ),
        _ => cli_error(
            "session_command_session_analytics_kernel_error",
            "session_command_session_analytics_kernel_unknown_command",
        ),
    };

    let ok = response.get("ok").and_then(Value::as_bool).unwrap_or(false);
    print_json_line(&response);
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
    fn extract_jsonl_pairs_tool_use_and_result() {
        let jsonl = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","id":"toolu_1","name":"Bash","input":{"command":"git status"}}]}}
{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"toolu_1","content":"On branch main","is_error":false}]}}"#;
        let rows = extract_commands_from_jsonl("s1", jsonl);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].command, "git status");
        assert!(!rows[0].is_error);
        assert!(rows[0].output_len.unwrap_or(0) > 0);
    }

    #[test]
    fn adoption_report_counts_supported_and_prefixed() {
        let payload = json!({
          "session_id":"s1",
          "commands":["git status","rtk cargo test","echo hello"]
        });
        let report = build_adoption_report(payload_obj(&payload), 10);
        assert_eq!(
            report.get("supported_commands").and_then(Value::as_u64),
            Some(2)
        );
        assert_eq!(
            report.get("unsupported_commands").and_then(Value::as_u64),
            Some(0)
        );
        assert_eq!(
            report
                .get("sessions")
                .and_then(Value::as_array)
                .and_then(|rows| rows.first())
                .and_then(|row| row.get("ignored_commands"))
                .and_then(Value::as_u64),
            Some(1)
        );
    }
}
