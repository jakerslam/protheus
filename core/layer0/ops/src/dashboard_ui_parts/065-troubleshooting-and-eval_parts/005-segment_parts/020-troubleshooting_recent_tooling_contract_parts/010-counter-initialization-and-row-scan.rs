    let mut tool_lane_rows = 0_i64;
    let mut execution_attempted_count = 0_i64;
    let mut execution_skipped_count = 0_i64;
    let mut provider_resolved_count = 0_i64;
    let mut provider_missing_count = 0_i64;
    let mut policy_block_count = 0_i64;
    let mut meta_block_count = 0_i64;
    let mut unknown_provider_count = 0_i64;
    let mut watchdog_warning_count = 0_i64;
    let mut watchdog_critical_count = 0_i64;
    let mut completion_signal_missing_count = 0_i64;
    let mut context_mismatch_count = 0_i64;
    let mut hallucination_pattern_count = 0_i64;
    let mut invalid_draft_count = 0_i64;
    let mut placeholder_output_count = 0_i64;
    let mut no_result_pattern_count = 0_i64;
    let mut answer_emitted_count = 0_i64;
    let mut answer_missing_after_completion_count = 0_i64;

    for row in rows {
        let lane = dashboard_troubleshooting_recent_lane(row);
        let classification = clean_text(
            row.pointer("/workflow/classification")
                .and_then(Value::as_str)
                .unwrap_or(""),
            120,
        )
        .to_ascii_lowercase();
        let error_code = clean_text(
            row.pointer("/workflow/error_code")
                .and_then(Value::as_str)
                .unwrap_or(""),
            160,
        )
        .to_ascii_lowercase();
        let transaction_status = clean_text(
            row.pointer("/workflow/transaction_status")
                .and_then(Value::as_str)
                .unwrap_or(""),
            40,
        )
        .to_ascii_lowercase();
        let output_text = [
            "/workflow/final_response_text",
            "/workflow/final_response",
            "/workflow/assistant_response",
            "/workflow/response_text",
            "/workflow/output_text",
            "/assistant_text",
            "/message_text",
        ]
        .iter()
        .find_map(|pointer| row.pointer(pointer).and_then(Value::as_str))
        .map(|raw| clean_text(raw, 240).to_ascii_lowercase())
        .unwrap_or_default();
        if !output_text.is_empty() {
            answer_emitted_count += 1;
        }
        let completion_like = matches!(transaction_status.as_str(), "completed" | "success")
            || classification.contains("response_synth")
            || classification.contains("synthesized");
        if completion_like && output_text.is_empty() {
            answer_missing_after_completion_count += 1;
        }
        if classification.contains("context") || error_code.contains("context") {
            context_mismatch_count += 1;
        }
        if classification.contains("halluc")
            || error_code.contains("halluc")
            || output_text.contains("<｜begin")
            || output_text.contains("patch v2")
        {
            hallucination_pattern_count += 1;
        }
        if classification.contains("invalid_draft") || error_code.contains("invalid_draft") {
            invalid_draft_count += 1;
        }
        if output_text.contains("i'll get you an update")
            || output_text.contains("would you like me to try")
            || output_text.contains("no results were returned")
            || output_text.contains("no search was actually performed")
        {
            placeholder_output_count += 1;
        }
        if output_text.contains("low-signal or no-result")
            || output_text.contains("low-signal or no result")
            || output_text.contains("low signal or no result")
            || output_text.contains("tool path ran, but this turn only produced low-signal")
            || output_text.contains("search request entirely blocked")
            || output_text.contains("web search was blocked")
        {
            no_result_pattern_count += 1;
        }

        let tooling_lane = lane == "tool_completion"
            || classification.contains("tool")
            || classification.contains("provider")
            || error_code.starts_with("web_")
            || error_code.contains("provider")
            || error_code.contains("tool");
        if !tooling_lane {
            continue;
        }
        tool_lane_rows += 1;

        let tool_execution_attempted = row
            .pointer("/workflow/tooling/tool_execution_attempted")
            .and_then(Value::as_bool)
            .or_else(|| {
                row.pointer("/workflow/tool_execution_attempted")
                    .and_then(Value::as_bool)
            })
            .or_else(|| {
                row.pointer("/tooling/tool_execution_attempted")
                    .and_then(Value::as_bool)
            })
            .unwrap_or(!matches!(
                transaction_status.as_str(),
                "not_started" | "skipped" | "none"
            ));

        if tool_execution_attempted {
            execution_attempted_count += 1;
        } else {
            execution_skipped_count += 1;
        }

        let completion_signal = clean_text(
            row.pointer("/workflow/completion_signal")
                .and_then(Value::as_str)
                .unwrap_or(""),
            80,
        );
        if completion_signal.is_empty() {
            completion_signal_missing_count += 1;
        }

        let provider = clean_text(
            row.pointer("/workflow/tooling/provider")
                .and_then(Value::as_str)
                .or_else(|| row.pointer("/workflow/provider").and_then(Value::as_str))
                .or_else(|| row.pointer("/tooling/provider").and_then(Value::as_str))
                .unwrap_or(""),
            120,
        )
        .to_ascii_lowercase();
        if provider.is_empty() || matches!(provider.as_str(), "auto" | "unknown" | "none") {
            provider_missing_count += 1;
        } else {
            provider_resolved_count += 1;
        }

        if error_code.contains("policy")
            || error_code.contains("permission")
            || error_code.contains("auth_missing")
        {
            policy_block_count += 1;
        }
        if error_code.contains("meta_query")
            || error_code.contains("non_search_meta_query")
            || error_code.contains("non_fetch_meta_query")
        {
            meta_block_count += 1;
        }
        if error_code.contains("unknown_search_provider")
            || error_code.contains("unknown_fetch_provider")
            || error_code.contains("unknown_provider")
        {
            unknown_provider_count += 1;
        }

        let loop_level = clean_text(
            row.pointer("/loop_detection/level")
                .and_then(Value::as_str)
                .unwrap_or(""),
            40,
        )
        .to_ascii_lowercase();
        if loop_level == "critical" {
            watchdog_critical_count += 1;
        } else if loop_level == "warning" {
            watchdog_warning_count += 1;
        }
    }

