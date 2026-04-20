
fn run_command(root: &Path, command: &str, payload: &Map<String, Value>) -> Result<Value, String> {
    match command {
        "now-iso" => Ok(json!({ "ok": true, "ts": now_iso() })),
        "append-jsonl" => {
            let file_path = resolve_file_path(root, &as_str(payload.get("file_path")));
            let row = payload.get("row").cloned().unwrap_or(Value::Null);
            append_jsonl(&file_path, &row)?;
            Ok(json!({
                "ok": true,
                "file_path": file_path.to_string_lossy(),
                "appended": true,
            }))
        }
        "with-receipt-contract" => Ok(json!({
            "ok": true,
            "record": with_receipt_contract_value(
                &payload.get("record").cloned().unwrap_or_else(|| json!({})),
                parse_attempted(payload),
                parse_verified(payload),
            ),
        })),
        "write-contract-receipt" => write_contract_receipt_value(root, payload),
        "replay-task-lineage" | "query-task-lineage" => replay_task_lineage_value(root, payload),
        _ => Err("action_receipts_kernel_unknown_command".to_string()),
    }
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let Some(command) = argv.first().map(|row| row.as_str()) else {
        usage();
        return 1;
    };
    if matches!(command, "help" | "--help" | "-h") {
        usage();
        return 0;
    }
    let mut payload = match payload_json(argv) {
        Ok(value) => value,
        Err(err) => {
            print_json_line(&cli_error("action_receipts_kernel", &err));
            return 1;
        }
    };
    if matches!(command, "replay-task-lineage" | "query-task-lineage") {
        let mut merged = payload.as_object().cloned().unwrap_or_default();
        if let Some(task_id) = lane_utils::parse_flag(argv, "task-id", false) {
            if !task_id.trim().is_empty() {
                merged.insert(
                    "task_id".to_string(),
                    Value::String(task_id.trim().to_string()),
                );
            }
        }
        if let Some(trace_id) = lane_utils::parse_flag(argv, "trace-id", false) {
            if !trace_id.trim().is_empty() {
                merged.insert(
                    "trace_id".to_string(),
                    Value::String(trace_id.trim().to_string()),
                );
            }
        }
        if let Some(limit) =
            lane_utils::parse_flag(argv, "limit", false).and_then(|value| value.parse::<u64>().ok())
        {
            merged.insert("limit".to_string(), Value::from(limit));
        }
        if let Some(scan_root) = lane_utils::parse_flag(argv, "scan-root", false) {
            if !scan_root.trim().is_empty() {
                merged.insert(
                    "scan_root".to_string(),
                    Value::String(scan_root.trim().to_string()),
                );
            }
        }
        if let Some(sources) = lane_utils::parse_flag(argv, "sources", false) {
            if !sources.trim().is_empty() {
                merged.insert(
                    "sources".to_string(),
                    Value::String(sources.trim().to_string()),
                );
            }
        }
        payload = Value::Object(merged);
    }
    match run_command(root, command, payload_obj(&payload)) {
        Ok(out) => {
            print_json_line(&cli_receipt("action_receipts_kernel", out));
            0
        }
        Err(err) => {
            print_json_line(&cli_error("action_receipts_kernel", &err));
            1
        }
    }
}
