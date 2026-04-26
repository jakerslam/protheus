fn entrypoint_context_string(payload: &Value, key: &str) -> String {
    payload
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase()
}

fn entrypoint_context_i64(payload: &Value, key: &str, fallback: i64) -> i64 {
    payload
        .get(key)
        .and_then(Value::as_i64)
        .or_else(|| payload.get(key).and_then(Value::as_u64).map(|value| value as i64))
        .or_else(|| {
            payload
                .get(key)
                .and_then(Value::as_str)
                .and_then(|value| value.trim().parse::<i64>().ok())
        })
        .unwrap_or(fallback)
}

fn entrypoint_context_arg_list(payload: &Value) -> Vec<String> {
    payload
        .get("passthrough_args")
        .or_else(|| payload.get("args"))
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|row| row.trim().to_ascii_lowercase())
                .filter(|row| !row.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn entrypoint_context_contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| haystack.contains(needle))
}

fn entrypoint_context_error_class(status: i64, error_text: &str, error_code: &str) -> &'static str {
    if error_code == "bridge_no_output" {
        return "transport_no_output";
    }
    if status == 124
        || entrypoint_context_contains_any(error_text, &["timeout", "timed out", "deadline"])
    {
        return "timeout";
    }
    if status == 126
        || status == 127
        || entrypoint_context_contains_any(
            error_text,
            &[
                "enoent",
                "not found",
                "missing",
                "permission denied",
                " eacces",
                "spawn ",
            ],
        )
    {
        return "transport_unavailable";
    }
    if entrypoint_context_contains_any(
        error_text,
        &[
            "429",
            "rate limit",
            "connect",
            "reset",
            "closed",
            "temporary",
            "temporarily",
            "unavailable",
            "retry",
        ],
    ) {
        return "transient";
    }
    if !error_text.trim().is_empty() || status != 0 {
        return "execution_error";
    }
    "none"
}

fn entrypoint_context_error_code(
    error_class: &str,
    status: i64,
    explicit_error_code: &str,
) -> Value {
    if !explicit_error_code.trim().is_empty() {
        return json!(explicit_error_code);
    }
    let code = match error_class {
        "timeout" if status == 124 => "bridge_timeout",
        "timeout" => "bridge_timeout",
        "transport_unavailable" => "bridge_transport_unavailable",
        "transient" => "bridge_transient_failure",
        "transport_no_output" => "bridge_no_output",
        "execution_error" => "bridge_execution_failed",
        _ => "",
    };
    if code.is_empty() {
        Value::Null
    } else {
        json!(code)
    }
}

fn entrypoint_context_retry(error_class: &str, retry_after_ms: i64) -> Value {
    if matches!(error_class, "timeout" | "transient") {
        let min_delay = if retry_after_ms > 0 {
            retry_after_ms.max(200)
        } else {
            400
        };
        let max_delay = if retry_after_ms > 0 {
            retry_after_ms.max(min_delay)
        } else {
            5000
        };
        return json!({
            "recommended": true,
            "strategy": "bounded_backoff",
            "lane": "same_lane_retry",
            "attempts": 2,
            "min_delay_ms": min_delay,
            "max_delay_ms": max_delay,
            "jitter": if retry_after_ms > 0 { 0.0 } else { 0.1 },
            "retry_after_ms": if retry_after_ms > 0 { json!(retry_after_ms) } else { Value::Null }
        });
    }
    if error_class == "transport_no_output" {
        return json!({
            "recommended": true,
            "strategy": "quick_retry",
            "lane": "same_lane_retry",
            "attempts": 1,
            "min_delay_ms": 250,
            "max_delay_ms": 1000,
            "jitter": 0.0
        });
    }
    if error_class == "transport_unavailable" {
        return json!({
            "recommended": false,
            "strategy": "manual_recovery",
            "lane": "operator_fix"
        });
    }
    json!({
        "recommended": false,
        "strategy": "none",
        "lane": "none"
    })
}

fn entrypoint_context_mutation_likely(args: &[String]) -> bool {
    let action = args.first().map(String::as_str).unwrap_or("").trim();
    if action.is_empty() {
        return false;
    }
    if matches!(
        action,
        "check" | "fetch" | "help" | "inspect" | "list" | "poll" | "probe" | "query"
            | "read" | "search" | "show" | "status" | "view"
    ) {
        return false;
    }
    matches!(
        action,
        "apply"
            | "append"
            | "delete"
            | "edit"
            | "kill"
            | "patch"
            | "restart"
            | "run"
            | "send"
            | "start"
            | "stop"
            | "submit"
            | "update"
            | "write"
    ) || action.starts_with("set_")
        || action.starts_with("update_")
        || action.starts_with("delete_")
        || action.starts_with("patch_")
}

fn entrypoint_authority_context_payload(args: &[String]) -> Result<Value, String> {
    let payload = payload_object(lane_utils::parse_flag(args, "payload-json", true).as_deref())?;
    let status = entrypoint_context_i64(&payload, "status", 1);
    let error_text = entrypoint_context_string(&payload, "error_text");
    let error_code = entrypoint_context_string(&payload, "error_code");
    let retry_after_ms = entrypoint_context_i64(&payload, "retry_after_ms", 0);
    let passthrough_args = entrypoint_context_arg_list(&payload);
    let error_class = entrypoint_context_error_class(status, &error_text, &error_code);
    let mut out = json!({
        "ok": true,
        "type": "runtime_systems_entrypoint_authority_context",
        "authority": "core/layer0/ops::runtime_systems",
        "status": status,
        "error_class": error_class,
        "error_code": entrypoint_context_error_code(error_class, status, &error_code),
        "retry": entrypoint_context_retry(error_class, retry_after_ms),
        "mutation_likely": entrypoint_context_mutation_likely(&passthrough_args),
        "passthrough_args_count": passthrough_args.len()
    });
    out["receipt_hash"] = Value::String(receipt_hash(&out));
    Ok(out)
}
