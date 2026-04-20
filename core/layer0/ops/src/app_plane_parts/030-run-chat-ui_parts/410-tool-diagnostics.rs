fn chat_ui_tool_diagnostics(tools: &[Value]) -> Value {
    let mut search_calls = 0_i64;
    let mut fetch_calls = 0_i64;
    let mut successful_calls = 0_i64;
    let mut failed_calls = 0_i64;
    let mut no_result_calls = 0_i64;
    let mut blocked_calls = 0_i64;
    let mut not_found_calls = 0_i64;
    let mut low_signal_calls = 0_i64;
    let mut silent_failure_calls = 0_i64;
    let mut surface_unavailable_calls = 0_i64;
    let mut surface_degraded_calls = 0_i64;
    let mut error_codes = serde_json::Map::<String, Value>::new();
    let mut execution_receipts = Vec::<Value>::new();

    for (idx, row) in tools.iter().enumerate() {
        let tool_name = tool_name_for_diagnostics(row);
        if tool_name.contains("search")
            || tool_name.contains("web_search")
            || tool_name.contains("batch_query")
        {
            search_calls += 1;
        }
        if tool_name.contains("fetch") || tool_name.contains("web_fetch") {
            fetch_calls += 1;
        }

        let findings = tool_findings_count(row) as i64;
        let ok = row
            .get("ok")
            .and_then(Value::as_bool)
            .or_else(|| row.pointer("/result/ok").and_then(Value::as_bool));
        let raw_status = clean(row.get("status").and_then(Value::as_str).unwrap_or(""), 120)
            .to_ascii_lowercase();
        let error = clean(
            row.get("error")
                .or_else(|| row.pointer("/result/error"))
                .or_else(|| row.pointer("/result/message"))
                .and_then(Value::as_str)
                .unwrap_or(""),
            300,
        );
        let result = clean(
            row.get("result")
                .or_else(|| row.pointer("/result/summary"))
                .or_else(|| row.pointer("/result/text"))
                .and_then(Value::as_str)
                .unwrap_or(""),
            600,
        );
        let duration_ms = row
            .get("duration_ms")
            .or_else(|| row.pointer("/telemetry/duration_ms"))
            .or_else(|| row.pointer("/result/telemetry/duration_ms"))
            .and_then(Value::as_i64)
            .unwrap_or(0);
        let tokens_used = row
            .get("tokens_used")
            .or_else(|| row.pointer("/telemetry/tokens_used"))
            .or_else(|| row.pointer("/result/telemetry/tokens_used"))
            .and_then(Value::as_i64)
            .unwrap_or(0);
        let call_id = format!(
            "toolcall_{}",
            &sha256_hex_str(&format!("{}:{}:{}", idx, tool_name, raw_status))[..12]
        );
        let mut status = crate::tool_output_match_filter::canonical_tool_status(
            &raw_status,
            ok,
            &error,
            findings,
            !result.is_empty(),
        );
        let surface_error_code_hint = chat_ui_surface_error_code_hint_from_row(row);
        if surface_error_code_hint.is_some() && status != "ok" {
            status = "error".to_string();
        }
        let status_hint_error_code =
            crate::tool_output_match_filter::normalize_web_tooling_error_code(&raw_status);
        let prioritized_surface_error_code = if matches!(
            status_hint_error_code.as_str(),
            "web_tool_surface_unavailable" | "web_tool_surface_degraded"
        ) {
            Some(status_hint_error_code.clone())
        } else {
            surface_error_code_hint.clone()
        };
        let policy_blocked_hint = chat_ui_policy_blocked_hint_from_row(row);
        let low_signal_hint = chat_ui_low_signal_hint_from_row(row);
        let not_found_hint = chat_ui_not_found_hint_from_row(row);
        if prioritized_surface_error_code.is_none() && status != "ok" {
            if policy_blocked_hint {
                status = "blocked".to_string();
            } else if not_found_hint {
                status = "not_found".to_string();
            } else if low_signal_hint {
                status = "low_signal".to_string();
            }
        }
        let error_code = if error.is_empty() {
            if status == "error"
                && prioritized_surface_error_code.is_some()
            {
                prioritized_surface_error_code
                    .clone()
                    .unwrap_or_else(|| "web_tool_error".to_string())
            } else {
                match status.as_str() {
                    "blocked" => "web_tool_policy_blocked".to_string(),
                    "not_found" => "web_tool_not_found".to_string(),
                    "low_signal" => "web_tool_low_signal".to_string(),
                    "unknown" => "web_tool_silent_failure".to_string(),
                    _ => "web_tool_error".to_string(),
                }
            }
        } else {
            let normalized = crate::tool_output_match_filter::normalize_web_tooling_error_code(&error);
            if normalized == "web_tool_error" {
                prioritized_surface_error_code.unwrap_or(normalized)
            } else {
                normalized
            }
        };

        match status.as_str() {
            "ok" => {
                successful_calls += 1;
                if findings == 0 {
                    no_result_calls += 1;
                }
            }
            "blocked" => {
                failed_calls += 1;
                blocked_calls += 1;
                let next = error_codes
                    .get(&error_code)
                    .and_then(Value::as_i64)
                    .unwrap_or(0)
                    .saturating_add(1);
                error_codes.insert(error_code.clone(), Value::from(next));
            }
            "not_found" => {
                failed_calls += 1;
                not_found_calls += 1;
                let next = error_codes
                    .get(&error_code)
                    .and_then(Value::as_i64)
                    .unwrap_or(0)
                    .saturating_add(1);
                error_codes.insert(error_code.clone(), Value::from(next));
            }
            "low_signal" => {
                low_signal_calls += 1;
                no_result_calls += 1;
                let next = error_codes
                    .get(&error_code)
                    .and_then(Value::as_i64)
                    .unwrap_or(0)
                    .saturating_add(1);
                error_codes.insert(error_code.clone(), Value::from(next));
            }
            "error" => {
                failed_calls += 1;
                if error_code == "web_tool_surface_unavailable" {
                    surface_unavailable_calls += 1;
                } else if error_code == "web_tool_surface_degraded" {
                    surface_degraded_calls += 1;
                }
                let next = error_codes
                    .get(&error_code)
                    .and_then(Value::as_i64)
                    .unwrap_or(0)
                    .saturating_add(1);
                error_codes.insert(error_code.clone(), Value::from(next));
            }
            _ => {
                failed_calls += 1;
                silent_failure_calls += 1;
                let code = "web_tool_silent_failure".to_string();
                let next = error_codes
                    .get(&code)
                    .and_then(Value::as_i64)
                    .unwrap_or(0)
                    .saturating_add(1);
                error_codes.insert(code, Value::from(next));
            }
        }
        let mut execution_receipt = crate::tool_output_match_filter::canonical_tool_execution_receipt(
            &call_id,
            &tool_name,
            &status,
            ok,
            &error,
            findings,
            duration_ms,
            tokens_used,
            !result.is_empty(),
        );
        if let Some(obj) = execution_receipt.as_object_mut() {
            obj.insert("status".to_string(), json!(status));
            obj.insert("error_code".to_string(), json!(error_code));
        }
        execution_receipts.push(execution_receipt);
    }

    let total_calls = tools.len() as i64;
    let error_ratio = if total_calls > 0 {
        (failed_calls as f64) / (total_calls as f64)
    } else {
        0.0
    };
    let mut diagnostics = json!({
        "total_calls": total_calls,
        "search_calls": search_calls,
        "fetch_calls": fetch_calls,
        "successful_calls": successful_calls,
        "failed_calls": failed_calls,
        "no_result_calls": no_result_calls,
        "blocked_calls": blocked_calls,
        "not_found_calls": not_found_calls,
        "low_signal_calls": low_signal_calls,
        "silent_failure_calls": silent_failure_calls,
        "surface_unavailable_calls": surface_unavailable_calls,
        "surface_degraded_calls": surface_degraded_calls,
        "error_ratio": error_ratio,
        "error_codes": Value::Object(error_codes),
        "execution_receipts": execution_receipts
    });
    let loop_risk = chat_ui_retry_loop_risk_from_diagnostics(&diagnostics);
    if let Some(obj) = diagnostics.as_object_mut() {
        obj.insert("loop_risk".to_string(), loop_risk);
    }
    diagnostics
}
