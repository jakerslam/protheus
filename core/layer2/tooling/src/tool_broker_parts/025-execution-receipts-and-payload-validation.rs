struct ToolExecutionReceiptInput<'a> {
    attempt: &'a ToolAttemptReceipt,
    input_hash: String,
    started_at: u64,
    ended_at: u64,
    data_ref: Option<String>,
    evidence_count: usize,
    error_code: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ToolPayloadValidationError {
    code: &'static str,
    reason: &'static str,
}

fn build_tool_execution_receipt(input: ToolExecutionReceiptInput<'_>) -> ToolExecutionReceipt {
    let status = execution_receipt_status(&input.attempt.status);
    let latency_ms = input.ended_at.saturating_sub(input.started_at);
    let mut receipt = ToolExecutionReceipt {
        attempt_id: input.attempt.attempt_id.clone(),
        trace_id: input.attempt.trace_id.clone(),
        task_id: input.attempt.task_id.clone(),
        status,
        tool_id: input.attempt.tool_name.clone(),
        input_hash: input.input_hash,
        started_at: input.started_at,
        ended_at: input.ended_at,
        latency_ms,
        error_code: input.error_code,
        data_ref: input.data_ref,
        evidence_count: input.evidence_count,
        receipt_hash: String::new(),
    };
    receipt.receipt_hash = deterministic_hash(&json!({
        "kind": "tool_execution_receipt",
        "attempt_id": &receipt.attempt_id,
        "trace_id": &receipt.trace_id,
        "task_id": &receipt.task_id,
        "status": &receipt.status,
        "tool_id": &receipt.tool_id,
        "input_hash": &receipt.input_hash,
        "started_at": receipt.started_at,
        "ended_at": receipt.ended_at,
        "latency_ms": receipt.latency_ms,
        "error_code": &receipt.error_code,
        "data_ref": &receipt.data_ref,
        "evidence_count": receipt.evidence_count,
    }));
    receipt
}

fn execution_receipt_status(status: &ToolAttemptStatus) -> ToolExecutionReceiptStatus {
    match status {
        ToolAttemptStatus::Ok => ToolExecutionReceiptStatus::Success,
        ToolAttemptStatus::Blocked | ToolAttemptStatus::PolicyDenied => {
            ToolExecutionReceiptStatus::Blocked
        }
        ToolAttemptStatus::Unavailable
        | ToolAttemptStatus::InvalidArgs
        | ToolAttemptStatus::ExecutionError
        | ToolAttemptStatus::TransportError
        | ToolAttemptStatus::Timeout => ToolExecutionReceiptStatus::Error,
    }
}

fn input_hash_for_tool(tool_name: &str, args: &Value) -> String {
    deterministic_hash(&json!({
        "kind": "tool_input",
        "tool_name": tool_name,
        "args": args,
    }))
}

fn error_code_for_attempt(attempt: &ToolAttemptReceipt) -> Option<String> {
    match attempt.reason_code {
        ToolReasonCode::Ok => None,
        ToolReasonCode::UnknownTool => Some("tool_not_found".to_string()),
        ToolReasonCode::TransportUnavailable
        | ToolReasonCode::DaemonUnavailable
        | ToolReasonCode::WebsocketUnavailable => Some("tool_unavailable".to_string()),
        ToolReasonCode::AuthRequired => Some("missing_credentials".to_string()),
        ToolReasonCode::CallerNotAuthorized | ToolReasonCode::PolicyDenied => {
            Some("policy_denied".to_string())
        }
        ToolReasonCode::InvalidArgs => Some("invalid_args".to_string()),
        ToolReasonCode::BackendDegraded => Some("backend_degraded".to_string()),
        ToolReasonCode::ExecutionError => Some(error_code_from_execution_error(&attempt.reason)),
        ToolReasonCode::Timeout => Some("timeout".to_string()),
    }
}

fn error_code_from_execution_error(error: &str) -> String {
    let Some(rest) = error.strip_prefix("structured_tool_error:") else {
        return "execution_error".to_string();
    };
    rest.split(':')
        .next()
        .map(|row| clean_text(row, 120))
        .filter(|row| !row.is_empty())
        .unwrap_or_else(|| "execution_error".to_string())
}

fn execute_tool_with_payload_validation<F>(
    tool_name: &str,
    normalized_args: &Value,
    executor: F,
) -> Result<Value, String>
where
    F: FnOnce(&Value) -> Result<Value, String>,
{
    let payload = executor(normalized_args)?;
    validate_tool_payload_for_synthesis(tool_name, &payload)
        .map_err(|err| format!("structured_tool_error:{}:{}", err.code, err.reason))?;
    Ok(payload)
}

fn validate_tool_payload_for_synthesis(
    tool_name: &str,
    payload: &Value,
) -> Result<(), ToolPayloadValidationError> {
    if payload.is_null() {
        return Err(payload_error("empty_payload", "tool_returned_null_payload"));
    }
    let text = payload_text(payload).to_ascii_lowercase();
    if anti_bot_or_access_wall(&text) {
        return Err(payload_error(
            "anti_bot_challenge",
            "payload_contains_access_or_human_challenge",
        ));
    }
    if file_payload_error(&text) {
        return Err(payload_error(
            "workspace_access_error",
            "payload_contains_workspace_access_error",
        ));
    }
    if requires_evidence(tool_name) && tool_payload_evidence_count(payload) == 0 {
        return Err(payload_error(
            "empty_result_set",
            "payload_has_no_usable_evidence",
        ));
    }
    Ok(())
}

fn payload_error(code: &'static str, reason: &'static str) -> ToolPayloadValidationError {
    ToolPayloadValidationError { code, reason }
}

fn requires_evidence(tool_name: &str) -> bool {
    matches!(
        tool_name,
        "web_search"
            | "batch_query"
            | "web_fetch"
            | "file_read"
            | "file_read_many"
            | "folder_export"
            | "workspace_analyze"
    )
}

fn anti_bot_or_access_wall(text: &str) -> bool {
    [
        "captcha",
        "confirm this search was made by a human",
        "unusual traffic",
        "are you a human",
        "access denied",
        "login required",
        "sign in to continue",
        "cloudflare",
        "enable javascript",
    ]
    .iter()
    .any(|needle| text.contains(needle))
}

fn file_payload_error(text: &str) -> bool {
    [
        "outside workspace",
        "path traversal",
        "permission denied",
        "operation not permitted",
        "unsafe path",
    ]
    .iter()
    .any(|needle| text.contains(needle))
}

fn tool_payload_evidence_count(payload: &Value) -> usize {
    match payload {
        Value::Array(rows) => rows.iter().map(tool_payload_evidence_count).sum(),
        Value::Object(map) => {
            for key in [
                "results",
                "items",
                "documents",
                "files",
                "evidence",
                "matches",
            ] {
                if let Some(Value::Array(rows)) = map.get(key) {
                    return rows.len();
                }
            }
            ["content", "text", "summary", "body", "markdown"]
                .iter()
                .filter_map(|key| map.get(*key).and_then(Value::as_str))
                .filter(|row| !row.trim().is_empty())
                .count()
        }
        Value::String(row) => usize::from(!row.trim().is_empty()),
        _ => 0,
    }
}

fn payload_text(payload: &Value) -> String {
    match payload {
        Value::String(row) => row.clone(),
        Value::Array(rows) => rows.iter().map(payload_text).collect::<Vec<_>>().join("\n"),
        Value::Object(map) => map
            .values()
            .map(payload_text)
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    }
}
