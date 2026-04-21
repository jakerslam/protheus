{
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

    let provider_resolution_ok =
        tool_lane_rows == 0 || provider_resolved_count >= provider_missing_count;
    let completion_signal_ok =
        tool_lane_rows == 0 || completion_signal_missing_count.saturating_mul(2) < tool_lane_rows;
    let watchdog_triggered = watchdog_critical_count > 0 || watchdog_warning_count >= 2;
    let hallucination_pattern_detected = hallucination_pattern_count > 0 || invalid_draft_count > 0;
    let placeholder_output_detected = placeholder_output_count > 0;
    let no_result_pattern_detected = no_result_pattern_count > 0;
    let llm_reliability_tier = if hallucination_pattern_detected
        || placeholder_output_count >= 2
        || context_mismatch_count >= 2
        || no_result_pattern_count >= 2
    {
        "low"
    } else if context_mismatch_count > 0 || placeholder_output_detected || no_result_pattern_detected
    {
        "medium"
    } else {
        "high"
    };
    let llm_reliability_not_low = llm_reliability_tier != "low";
    let answer_contract_ok = answer_missing_after_completion_count == 0;
    let answer_signal_coverage = if rows.is_empty() {
        1.0
    } else {
        (answer_emitted_count as f64 / rows.len() as f64).clamp(0.0, 1.0)
    };
    let answer_signal_coverage = (answer_signal_coverage * 10_000.0).round() / 10_000.0;
    let final_response_contract_violation_count = completion_signal_missing_count
        .saturating_add(invalid_draft_count)
        .saturating_add(hallucination_pattern_count)
        .saturating_add(placeholder_output_count)
        .saturating_add(no_result_pattern_count)
        .saturating_add(answer_missing_after_completion_count);
    let final_response_contract_ok = final_response_contract_violation_count == 0;
    let response_gate_ready = final_response_contract_ok
        && answer_contract_ok
        && llm_reliability_not_low
        && !watchdog_triggered;
    let response_gate_score = dashboard_response_gate_score_from_flags(
        final_response_contract_ok,
        answer_contract_ok,
        llm_reliability_not_low,
        watchdog_triggered,
    );
    let response_gate_expected_score = dashboard_response_gate_score_from_flags(
        final_response_contract_ok,
        answer_contract_ok,
        llm_reliability_not_low,
        watchdog_triggered,
    );
    let response_gate_score_consistent =
        (response_gate_score - response_gate_expected_score).abs() <= 0.0001;
    let response_gate_severity =
        dashboard_response_gate_severity_from_state(response_gate_ready, response_gate_score);
    let response_gate_expected_severity = dashboard_response_gate_severity_from_state(
        response_gate_ready,
        response_gate_score,
    );
    let response_gate_score_band =
        dashboard_response_gate_score_band_from_state(response_gate_ready, response_gate_score);
    let response_gate_expected_score_band = dashboard_response_gate_score_band_from_state(
        response_gate_expected_severity == "ready",
        response_gate_expected_score,
    );
    let response_gate_score_band_consistent =
        response_gate_score_band == response_gate_expected_score_band;
    let response_gate_score_band_known = matches!(
        response_gate_score_band,
        "ready" | "strong" | "watch" | "weak" | "critical"
    );
    let response_gate_score_vector_key = format!(
        "score={:.4};severity={}",
        response_gate_score, response_gate_severity
    );
    let response_gate_expected_score_vector_key = format!(
        "score={:.4};severity={}",
        response_gate_expected_score, response_gate_expected_severity
    );
    let response_gate_score_vector_consistent =
        response_gate_score_vector_key == response_gate_expected_score_vector_key;
    let response_gate_score_band_vector_key = format!(
        "band={};score={:.4}",
        response_gate_score_band, response_gate_score
    );
    let response_gate_expected_score_band_vector_key = format!(
        "band={};score={:.4}",
        response_gate_expected_score_band, response_gate_expected_score
    );
    let response_gate_score_band_vector_consistent =
        response_gate_score_band_vector_key == response_gate_expected_score_band_vector_key;
    let response_gate_expected_severity_from_score_band = if response_gate_score_band == "ready" {
        "ready"
    } else if matches!(response_gate_score_band, "strong" | "watch") {
        "degraded"
    } else {
        "blocked"
    };
    let response_gate_score_band_severity_consistent =
        response_gate_severity == response_gate_expected_severity_from_score_band;
    let response_gate_score_band_severity_bucket_consistent =
        dashboard_response_gate_score_band_severity_bucket_consistent(
            response_gate_severity,
            response_gate_score_band,
        );
    let response_gate_score_band_severity_bucket_known =
        dashboard_response_gate_score_band_severity_bucket_known(
            response_gate_severity,
            response_gate_score_band,
        );
    let response_gate_blockers = dashboard_response_gate_blockers_from_flags(
        final_response_contract_ok,
        answer_contract_ok,
        llm_reliability_not_low,
        watchdog_triggered,
    );
    let response_gate_expected_blockers = dashboard_response_gate_blockers_from_flags(
        final_response_contract_ok,
        answer_contract_ok,
        llm_reliability_not_low,
        watchdog_triggered,
    );
    let response_gate_blocker_set_consistent = response_gate_blockers == response_gate_expected_blockers;
    let response_gate_blocker_set_key = if response_gate_blockers.is_empty() {
        "none".to_string()
    } else {
        response_gate_blockers.join("|")
    };
    let response_gate_expected_blocker_set_key = if response_gate_expected_blockers.is_empty() {
        "none".to_string()
    } else {
        response_gate_expected_blockers.join("|")
    };
    let response_gate_blocker_set_key_consistent =
        response_gate_blocker_set_key == response_gate_expected_blocker_set_key;
    let response_gate_blocker_count = response_gate_blockers.len() as i64;
    let response_gate_blocker_count_key_consistent =
        if response_gate_blocker_count == 0 {
            response_gate_blocker_set_key == "none"
        } else {
            response_gate_blocker_set_key != "none"
        };
    let response_gate_expected_blocker_count = response_gate_expected_blockers.len() as i64;
    let response_gate_expected_blocker_count_matches =
        response_gate_expected_blocker_count == response_gate_blocker_count;
    let response_gate_blocker_budget_max = 4_i64;
    let response_gate_blocker_budget_consistent = response_gate_blocker_count >= 0
        && response_gate_expected_blocker_count >= 0
        && response_gate_blocker_count <= response_gate_blocker_budget_max
        && response_gate_expected_blocker_count <= response_gate_blocker_budget_max;
    let response_gate_blocker_has_final_response_contract = response_gate_blockers
        .iter()
        .any(|row| row == "final_response_contract");
    let response_gate_blocker_has_answer_contract =
        response_gate_blockers.iter().any(|row| row == "answer_contract");
    let response_gate_blocker_has_llm_reliability =
        response_gate_blockers.iter().any(|row| row == "llm_reliability");
    let response_gate_blocker_has_watchdog = response_gate_blockers.iter().any(|row| row == "watchdog");
    let response_gate_blocker_flags_key = format!(
        "final_response_contract={};answer_contract={};llm_reliability={};watchdog={}",
        response_gate_blocker_has_final_response_contract,
        response_gate_blocker_has_answer_contract,
        response_gate_blocker_has_llm_reliability,
        response_gate_blocker_has_watchdog
    );
    let response_gate_expected_blocker_flags_key = format!(
        "final_response_contract={};answer_contract={};llm_reliability={};watchdog={}",
        !final_response_contract_ok,
        !answer_contract_ok,
        !llm_reliability_not_low,
        watchdog_triggered
    );
    let response_gate_blocker_flags_consistent = response_gate_blocker_flags_key
        == response_gate_expected_blocker_flags_key
        && response_gate_blocker_count
            == i64::from(response_gate_blocker_has_final_response_contract)
                + i64::from(response_gate_blocker_has_answer_contract)
                + i64::from(response_gate_blocker_has_llm_reliability)
                + i64::from(response_gate_blocker_has_watchdog);
    let response_gate_primary_blocker = response_gate_blockers
        .first()
        .map(|row| row.as_str())
        .unwrap_or("none");
    let response_gate_primary_blocker_expected = if !final_response_contract_ok {
        "final_response_contract"
    } else if !answer_contract_ok {
        "answer_contract"
    } else if !llm_reliability_not_low {
        "llm_reliability"
    } else if watchdog_triggered {
        "watchdog"
    } else {
        "none"
    };
    let response_gate_blocker_priority_consistent =
        response_gate_primary_blocker == response_gate_primary_blocker_expected;
    let response_gate_blocker_vector_key = format!(
        "count={};set={};primary={}",
        response_gate_blocker_count, response_gate_blocker_set_key, response_gate_primary_blocker
    );
    let response_gate_expected_blocker_vector_key = format!(
        "count={};set={};primary={}",
        response_gate_expected_blocker_count,
        response_gate_expected_blocker_set_key,
        response_gate_primary_blocker_expected
    );
    let response_gate_blocker_vector_consistent =
        response_gate_blocker_vector_key == response_gate_expected_blocker_vector_key;
    let response_gate_primary_blocker_known = matches!(
        response_gate_primary_blocker,
        "final_response_contract" | "answer_contract" | "llm_reliability" | "watchdog" | "none"
    );
    let response_gate_escalation_lane = match response_gate_primary_blocker {
        "final_response_contract" | "answer_contract" => "dashboard.troubleshooting.recent.state",
        "llm_reliability" => "dashboard.troubleshooting.snapshot.capture",
        "watchdog" => "dashboard.troubleshooting.summary",
        _ => "none",
    };
    let response_gate_escalation_lane_known = matches!(
        response_gate_escalation_lane,
        "dashboard.troubleshooting.recent.state"
            | "dashboard.troubleshooting.snapshot.capture"
            | "dashboard.troubleshooting.summary"
            | "none"
    );
    let response_gate_blockers_consistent = if response_gate_ready {
        response_gate_blocker_count == 0
            && response_gate_primary_blocker == "none"
            && response_gate_escalation_lane == "none"
    } else {
        response_gate_blocker_count > 0
            && response_gate_primary_blocker != "none"
            && response_gate_escalation_lane != "none"
    };
    let response_gate_severity_consistent = if response_gate_ready {
        response_gate_severity == "ready"
    } else if response_gate_score >= 0.6 {
        response_gate_severity == "degraded"
    } else {
        response_gate_severity == "blocked"
    };
    let response_gate_manual_review_consistent = !response_gate_ready;
    let response_gate_requires_manual_review = !response_gate_ready;
    let response_gate_expected_requires_manual_review = response_gate_expected_severity != "ready";
    let response_gate_manual_review_signature_consistent =
        response_gate_requires_manual_review == response_gate_expected_requires_manual_review;
    let response_gate_manual_review_reason = if response_gate_requires_manual_review {
        "gated_response_not_ready"
    } else {
        "none"
    };
    let response_gate_expected_manual_review_reason = if response_gate_expected_requires_manual_review {
        "gated_response_not_ready"
    } else {
        "none"
    };
    let response_gate_manual_review_reason_consistent =
        response_gate_manual_review_reason == response_gate_expected_manual_review_reason;
    let response_gate_manual_review_reason_known =
        matches!(response_gate_manual_review_reason, "gated_response_not_ready" | "none");
    let response_gate_manual_review_vector_key = format!(
        "required={};reason={}",
        response_gate_requires_manual_review, response_gate_manual_review_reason
    );
    let response_gate_expected_manual_review_vector_key = format!(
        "required={};reason={}",
        response_gate_expected_requires_manual_review, response_gate_expected_manual_review_reason
    );
    let response_gate_manual_review_vector_consistent =
        response_gate_manual_review_vector_key == response_gate_expected_manual_review_vector_key;
    let response_gate_manual_review_vector_known = matches!(
        response_gate_manual_review_vector_key.as_str(),
        "required=true;reason=gated_response_not_ready" | "required=false;reason=none"
    );
    let response_gate_blocker_count_matches =
        response_gate_blocker_count == response_gate_blockers.len() as i64;
    let response_gate_primary_blocker_matches = if response_gate_blocker_count == 0 {
        response_gate_primary_blocker == "none"
    } else {
        response_gate_blockers
            .first()
            .is_some_and(|row| row == response_gate_primary_blocker)
    };
    let response_gate_escalation_contract_ok = if response_gate_primary_blocker == "none" {
        response_gate_escalation_lane == "none"
    } else {
        response_gate_escalation_lane != "none"
    };
    let response_gate_escalation_reason_code = match response_gate_primary_blocker {
        "final_response_contract" => "finalization_integrity_failure",
        "answer_contract" => "answer_integrity_failure",
        "llm_reliability" => "llm_reliability_degraded",
        "watchdog" => "watchdog_pressure",
        "none" => "none",
        _ => "unknown",
    };
    let response_gate_next_action_command = match response_gate_primary_blocker {
        "final_response_contract" | "answer_contract" => {
            "dashboard.troubleshooting.recent.state --json"
        }
        "llm_reliability" => "dashboard.troubleshooting.snapshot.capture --include-contract --json",
        "watchdog" => "dashboard.troubleshooting.summary --limit=20 --json",
        _ => "none",
    };
    let response_gate_expected_escalation_lane = match response_gate_primary_blocker_expected {
        "final_response_contract" | "answer_contract" => "dashboard.troubleshooting.recent.state",
        "llm_reliability" => "dashboard.troubleshooting.snapshot.capture",
        "watchdog" => "dashboard.troubleshooting.summary",
        _ => "none",
    };
    let response_gate_expected_escalation_reason_code = match response_gate_primary_blocker_expected {
        "final_response_contract" => "finalization_integrity_failure",
        "answer_contract" => "answer_integrity_failure",
        "llm_reliability" => "llm_reliability_degraded",
        "watchdog" => "watchdog_pressure",
        _ => "none",
    };
    let response_gate_expected_next_action_command = match response_gate_primary_blocker_expected {
        "final_response_contract" | "answer_contract" => {
            "dashboard.troubleshooting.recent.state --json"
        }
        "llm_reliability" => "dashboard.troubleshooting.snapshot.capture --include-contract --json",
        "watchdog" => "dashboard.troubleshooting.summary --limit=20 --json",
        _ => "none",
    };
    let response_gate_next_action_lane = response_gate_escalation_lane;
    let response_gate_expected_next_action_lane = response_gate_expected_escalation_lane;
    let response_gate_next_action_command_consistent =
        response_gate_next_action_command == response_gate_expected_next_action_command;
    let response_gate_next_action_command_known = matches!(
        response_gate_next_action_command,
        "dashboard.troubleshooting.recent.state --json"
            | "dashboard.troubleshooting.snapshot.capture --include-contract --json"
            | "dashboard.troubleshooting.summary --limit=20 --json"
            | "none"
    );
    let response_gate_retry_class = if response_gate_ready {
        "none"
    } else if response_gate_severity == "degraded" {
        "single_retry"
    } else {
        "bounded_retry"
    };
    let response_gate_expected_retry_class = if response_gate_expected_severity == "ready" {
        "none"
    } else if response_gate_expected_severity == "degraded" {
        "single_retry"
    } else {
        "bounded_retry"
    };
    let response_gate_retry_class_consistent =
        response_gate_retry_class == response_gate_expected_retry_class;
    let response_gate_retry_class_known = matches!(
        response_gate_retry_class,
        "none" | "single_retry" | "bounded_retry"
    );
    let response_gate_retry_after_seconds = match response_gate_score_band {
        "ready" => 0_i64,
        "strong" => 15_i64,
        "watch" => 30_i64,
        "weak" => 60_i64,
        _ => 120_i64,
    };
    let response_gate_expected_retry_after_seconds = match response_gate_expected_score_band {
        "ready" => 0_i64,
        "strong" => 15_i64,
        "watch" => 30_i64,
        "weak" => 60_i64,
        _ => 120_i64,
    };
    let response_gate_retry_after_seconds_consistent =
        response_gate_retry_after_seconds == response_gate_expected_retry_after_seconds;
    let response_gate_retry_after_seconds_non_negative =
        response_gate_retry_after_seconds >= 0 && response_gate_expected_retry_after_seconds >= 0;
    let response_gate_next_action_lane_consistent = match response_gate_escalation_lane {
        "none" => response_gate_next_action_command == "none",
        "dashboard.troubleshooting.recent.state" => {
            response_gate_next_action_command
                .starts_with("dashboard.troubleshooting.recent.state")
        }
        "dashboard.troubleshooting.snapshot.capture" => {
            response_gate_next_action_command
                .starts_with("dashboard.troubleshooting.snapshot.capture")
        }
        "dashboard.troubleshooting.summary" => {
            response_gate_next_action_command
                .starts_with("dashboard.troubleshooting.summary")
        }
        _ => false,
    };
    let response_gate_retry_command_consistent = if response_gate_retry_class == "none" {
        response_gate_next_action_command == "none"
    } else {
        response_gate_next_action_command != "none"
    };
    let response_gate_retry_window_consistent = if response_gate_retry_class == "none" {
        response_gate_retry_after_seconds == 0
    } else {
        response_gate_retry_after_seconds > 0
    };
    let response_gate_retry_signature_key = format!(
        "class={};after={};lane={}",
        response_gate_retry_class, response_gate_retry_after_seconds, response_gate_escalation_lane
    );
    let response_gate_expected_retry_signature_key = format!(
        "class={};after={};lane={}",
        response_gate_expected_retry_class,
        response_gate_expected_retry_after_seconds,
        response_gate_expected_escalation_lane
    );
    let response_gate_retry_signature_consistent =
        response_gate_retry_signature_key == response_gate_expected_retry_signature_key;
    let response_gate_retry_signature_known = matches!(
        response_gate_retry_signature_key.as_str(),
        "class=none;after=0;lane=none"
            | "class=single_retry;after=15;lane=dashboard.troubleshooting.snapshot.capture"
            | "class=single_retry;after=15;lane=dashboard.troubleshooting.summary"
            | "class=single_retry;after=30;lane=dashboard.troubleshooting.recent.state"
            | "class=bounded_retry;after=60;lane=dashboard.troubleshooting.recent.state"
            | "class=bounded_retry;after=120;lane=dashboard.troubleshooting.recent.state"
            | "class=bounded_retry;after=120;lane=dashboard.troubleshooting.snapshot.capture"
            | "class=bounded_retry;after=120;lane=dashboard.troubleshooting.summary"
    );
    let response_gate_lane_retry_window_consistent = match response_gate_escalation_lane {
        "none" => response_gate_retry_after_seconds == 0,
        "dashboard.troubleshooting.recent.state" => response_gate_retry_after_seconds >= 30,
        "dashboard.troubleshooting.snapshot.capture" => {
            response_gate_retry_after_seconds > 0 && response_gate_retry_after_seconds <= 30
        }
        "dashboard.troubleshooting.summary" => {
            response_gate_retry_after_seconds > 0 && response_gate_retry_after_seconds <= 30
        }
        _ => false,
    };
    let response_gate_retry_band_consistent = match response_gate_retry_class {
        "none" => response_gate_score_band == "ready",
        "single_retry" => matches!(response_gate_score_band, "strong" | "watch"),
        "bounded_retry" => matches!(response_gate_score_band, "weak" | "critical"),
        _ => false,
    };
    let response_gate_retry_contract_after_seconds_class_consistent =
        match response_gate_retry_class {
            "none" => response_gate_retry_after_seconds == 0,
            "single_retry" => {
                response_gate_retry_after_seconds >= 1 && response_gate_retry_after_seconds < 60
            }
            "bounded_retry" => response_gate_retry_after_seconds >= 60,
            _ => false,
        };
    let response_gate_retry_contract_after_seconds_score_band_consistent =
        match response_gate_score_band {
            "ready" => response_gate_retry_after_seconds == 0,
            "strong" | "watch" => {
                response_gate_retry_after_seconds >= 1 && response_gate_retry_after_seconds < 60
            }
            "weak" | "critical" => response_gate_retry_after_seconds >= 60,
            _ => false,
        };
    let response_gate_retry_contract_after_seconds_lane_consistent =
        match response_gate_next_action_lane {
            "none" => response_gate_retry_after_seconds == 0,
            "dashboard.troubleshooting.recent.state"
            | "dashboard.troubleshooting.snapshot.capture"
            | "dashboard.troubleshooting.summary" => response_gate_retry_after_seconds > 0,
            _ => false,
        };
    let response_gate_retry_contract_after_seconds_next_action_window_consistent =
        match response_gate_next_action_lane {
            "none" => response_gate_retry_after_seconds == 0,
            "dashboard.troubleshooting.recent.state" => response_gate_retry_after_seconds >= 30,
            "dashboard.troubleshooting.snapshot.capture"
            | "dashboard.troubleshooting.summary" => {
                response_gate_retry_after_seconds > 0 && response_gate_retry_after_seconds <= 30
            }
            _ => false,
        };
    let response_gate_retry_contract_lane_command_consistent =
        (response_gate_next_action_lane == "none" && response_gate_next_action_command == "none")
            || (response_gate_next_action_lane == "dashboard.troubleshooting.recent.state"
                && response_gate_next_action_command
                    == "dashboard.troubleshooting.recent.state --json")
            || (response_gate_next_action_lane == "dashboard.troubleshooting.snapshot.capture"
                && response_gate_next_action_command
                    == "dashboard.troubleshooting.snapshot.capture --json")
            || (response_gate_next_action_lane == "dashboard.troubleshooting.summary"
                && response_gate_next_action_command
                    == "dashboard.troubleshooting.summary --json");
    let response_gate_retry_contract_after_seconds_lane_command_consistent =
        if response_gate_retry_after_seconds == 0 {
            response_gate_next_action_lane == "none"
                && response_gate_next_action_command == "none"
        } else {
            response_gate_next_action_lane != "none"
                && response_gate_next_action_command != "none"
        };
    let response_gate_retry_contract_after_seconds_command_consistent =
        if response_gate_next_action_command == "none" {
            response_gate_retry_after_seconds == 0
        } else {
            response_gate_retry_after_seconds > 0
        };
    let response_gate_retry_mode = if response_gate_next_action_command == "none" {
        "passive"
    } else {
        "active"
    };
    let response_gate_expected_retry_mode = if response_gate_expected_next_action_command == "none" {
        "passive"
    } else {
        "active"
    };
    let response_gate_retry_mode_consistent =
        response_gate_retry_mode == response_gate_expected_retry_mode;
    let response_gate_retry_mode_known =
        matches!(response_gate_retry_mode, "passive" | "active");
    let response_gate_retry_contract_lane_mode_consistent = if response_gate_next_action_lane == "none"
    {
        response_gate_retry_mode == "passive"
    } else {
        response_gate_retry_mode == "active"
    };
    let response_gate_retry_contract_after_seconds_mode_consistent = match response_gate_retry_mode {
        "passive" => response_gate_retry_after_seconds == 0,
        "active" => response_gate_retry_after_seconds >= 1,
        _ => false,
    };
    let response_gate_retry_contract_after_seconds_lane_mode_consistent =
        if response_gate_retry_after_seconds == 0 {
            response_gate_next_action_lane == "none" && response_gate_retry_mode == "passive"
        } else {
            response_gate_next_action_lane != "none" && response_gate_retry_mode == "active"
        };
    let response_gate_retry_contract_lane_command_mode_consistent =
        if response_gate_retry_mode == "passive" {
            response_gate_next_action_lane == "none" && response_gate_next_action_command == "none"
        } else {
            response_gate_next_action_lane != "none" && response_gate_next_action_command != "none"
        };
    let response_gate_retry_contract_after_seconds_lane_command_mode_consistent =
        if response_gate_retry_after_seconds == 0 {
            response_gate_next_action_lane == "none"
                && response_gate_next_action_command == "none"
                && response_gate_retry_mode == "passive"
        } else {
            response_gate_next_action_lane != "none"
                && response_gate_next_action_command != "none"
                && response_gate_retry_mode == "active"
        };
    let response_gate_retry_contract_after_seconds_command_mode_consistent =
        if response_gate_retry_after_seconds == 0 {
            response_gate_next_action_command == "none" && response_gate_retry_mode == "passive"
        } else {
            response_gate_next_action_command != "none" && response_gate_retry_mode == "active"
        };
    let response_gate_retry_action_vector_key = format!(
        "{}|{}|{}",
        response_gate_retry_class, response_gate_retry_mode, response_gate_next_action_command
    );
    let response_gate_expected_retry_action_vector_key = format!(
        "{}|{}|{}",
        response_gate_expected_retry_class,
        response_gate_expected_retry_mode,
        response_gate_expected_next_action_command
    );
    let response_gate_retry_action_vector_consistent =
        response_gate_retry_action_vector_key == response_gate_expected_retry_action_vector_key;
    let response_gate_retry_budget_points = (120_i64 - response_gate_retry_after_seconds).max(0);
    let response_gate_expected_retry_budget_points =
        (120_i64 - response_gate_expected_retry_after_seconds).max(0);
    let response_gate_retry_budget_consistent =
        response_gate_retry_budget_points == response_gate_expected_retry_budget_points;
    let response_gate_retry_budget_non_negative =
        response_gate_retry_budget_points >= 0 && response_gate_expected_retry_budget_points >= 0;
    let response_gate_expected_retry_budget_from_band = match response_gate_score_band {
        "ready" => 120_i64,
        "strong" => 105_i64,
        "watch" => 90_i64,
        "weak" => 60_i64,
        _ => 0_i64,
    };
    let response_gate_retry_budget_band_consistent =
        response_gate_retry_budget_points == response_gate_expected_retry_budget_from_band;
    let response_gate_expected_retry_budget_from_expected_band = match response_gate_expected_score_band {
        "ready" => 120_i64,
        "strong" => 105_i64,
        "watch" => 90_i64,
        "weak" => 60_i64,
        _ => 0_i64,
    };
    let response_gate_retry_budget_expected_band_consistent = response_gate_expected_retry_budget_points
        == response_gate_expected_retry_budget_from_expected_band;
    let response_gate_retry_budget_range_consistent = (0_i64..=120_i64)
        .contains(&response_gate_retry_budget_points)
        && (0_i64..=120_i64).contains(&response_gate_expected_retry_budget_points);
    let response_gate_retry_budget_mode_consistent = if response_gate_retry_mode == "passive" {
        response_gate_retry_budget_points == 120 && response_gate_retry_after_seconds == 0
    } else {
        response_gate_retry_budget_points < 120 && response_gate_retry_after_seconds > 0
    };
    let response_gate_retry_contract_after_seconds_budget_consistent =
        if response_gate_retry_after_seconds == 0 {
            response_gate_retry_budget_points == 120
        } else {
            response_gate_retry_budget_points < 120
        };
    let response_gate_retry_pressure_tier = if response_gate_retry_budget_points >= 100 {
        "low"
    } else if response_gate_retry_budget_points >= 70 {
        "medium"
    } else {
        "high"
    };
    let response_gate_expected_retry_pressure_tier = if response_gate_score_band == "ready" {
        "low"
    } else if matches!(response_gate_score_band, "strong" | "watch") {
        "medium"
    } else {
        "high"
    };
    let response_gate_retry_pressure_tier_consistent =
        response_gate_retry_pressure_tier == response_gate_expected_retry_pressure_tier;
    let response_gate_retry_pressure_tier_known =
        matches!(response_gate_retry_pressure_tier, "low" | "medium" | "high");
    let response_gate_retry_contract_after_seconds_pressure_consistent =
        match response_gate_retry_pressure_tier {
            "low" => response_gate_retry_after_seconds == 0,
            "medium" => {
                response_gate_retry_after_seconds >= 1 && response_gate_retry_after_seconds < 60
            }
            "high" => response_gate_retry_after_seconds >= 60,
            _ => false,
        };
    let response_gate_retry_budget_vector_key = format!(
        "points={};tier={};mode={}",
        response_gate_retry_budget_points, response_gate_retry_pressure_tier, response_gate_retry_mode
    );
    let response_gate_expected_retry_budget_vector_key = format!(
        "points={};tier={};mode={}",
        response_gate_expected_retry_budget_from_expected_band,
        response_gate_expected_retry_pressure_tier,
        response_gate_expected_retry_mode
    );
    let response_gate_retry_budget_vector_consistent =
        response_gate_retry_budget_vector_key == response_gate_expected_retry_budget_vector_key;
    let response_gate_retry_budget_vector_known = matches!(
        response_gate_retry_budget_vector_key.as_str(),
        "points=120;tier=low;mode=passive"
            | "points=105;tier=medium;mode=active"
            | "points=90;tier=medium;mode=active"
            | "points=60;tier=high;mode=active"
            | "points=0;tier=high;mode=active"
    );
    let response_gate_retry_tier_window_consistent = match response_gate_retry_pressure_tier {
        "low" => response_gate_retry_after_seconds == 0,
        "medium" => (15_i64..=30_i64).contains(&response_gate_retry_after_seconds),
        "high" => response_gate_retry_after_seconds >= 60,
        _ => false,
    };
    let response_gate_retry_tier_mode_consistent = match response_gate_retry_pressure_tier {
        "low" => response_gate_retry_mode == "passive",
        "medium" | "high" => response_gate_retry_mode == "active",
        _ => false,
    };
    let response_gate_retry_tier_vector_key = format!(
        "tier={};after={};mode={}",
        response_gate_retry_pressure_tier, response_gate_retry_after_seconds, response_gate_retry_mode
    );
    let response_gate_expected_retry_tier_vector_key = format!(
        "tier={};after={};mode={}",
        response_gate_expected_retry_pressure_tier,
        response_gate_expected_retry_after_seconds,
        response_gate_expected_retry_mode
    );
    let response_gate_retry_tier_vector_consistent =
        response_gate_retry_tier_vector_key == response_gate_expected_retry_tier_vector_key;
    let response_gate_retry_tier_vector_known = matches!(
        response_gate_retry_tier_vector_key.as_str(),
        "tier=low;after=0;mode=passive"
            | "tier=medium;after=15;mode=active"
            | "tier=medium;after=30;mode=active"
            | "tier=high;after=60;mode=active"
            | "tier=high;after=90;mode=active"
            | "tier=high;after=120;mode=active"
    );
    let response_gate_retry_contract_vector_key = format!(
        "class={};budget={};tier={};after={};mode={};lane={}",
        response_gate_retry_class,
        response_gate_retry_budget_points,
        response_gate_retry_pressure_tier,
        response_gate_retry_after_seconds,
        response_gate_retry_mode,
        response_gate_next_action_lane
    );
    let response_gate_expected_retry_contract_vector_key = format!(
        "class={};budget={};tier={};after={};mode={};lane={}",
        response_gate_expected_retry_class,
        response_gate_expected_retry_budget_points,
        response_gate_expected_retry_pressure_tier,
        response_gate_expected_retry_after_seconds,
        response_gate_expected_retry_mode,
        response_gate_expected_next_action_lane
    );
    let response_gate_retry_contract_vector_consistent = response_gate_retry_contract_vector_key
        == response_gate_expected_retry_contract_vector_key;
    let response_gate_retry_contract_vector_known = matches!(
        response_gate_retry_contract_vector_key.as_str(),
        "class=none;budget=120;tier=low;after=0;mode=passive;lane=none"
            | "class=single_retry;budget=90;tier=medium;after=30;mode=active;lane=dashboard.troubleshooting.recent.state"
            | "class=bounded_retry;budget=60;tier=high;after=60;mode=active;lane=dashboard.troubleshooting.recent.state"
            | "class=bounded_retry;budget=0;tier=high;after=120;mode=active;lane=dashboard.troubleshooting.recent.state"
    );
    let response_gate_retry_contract_family_consistent = match response_gate_retry_class {
        "none" => {
            response_gate_retry_budget_points == 120
                && response_gate_retry_pressure_tier == "low"
                && response_gate_retry_after_seconds == 0
                && response_gate_retry_mode == "passive"
                && response_gate_next_action_lane == "none"
        }
        "single_retry" => {
            response_gate_retry_budget_points == 90
                && response_gate_retry_pressure_tier == "medium"
                && response_gate_retry_after_seconds == 30
                && response_gate_retry_mode == "active"
                && response_gate_next_action_lane == "dashboard.troubleshooting.recent.state"
        }
        "bounded_retry" => {
            response_gate_retry_budget_points <= 60
                && response_gate_retry_pressure_tier == "high"
                && response_gate_retry_after_seconds >= 60
                && response_gate_retry_mode == "active"
                && response_gate_next_action_lane == "dashboard.troubleshooting.recent.state"
        }
        _ => false,
    };
    let response_gate_retry_contract_severity_consistent = match response_gate_severity {
        "ready" => {
            response_gate_retry_class == "none"
                && response_gate_retry_pressure_tier == "low"
                && response_gate_retry_mode == "passive"
                && response_gate_next_action_lane == "none"
        }
        "degraded" => {
            response_gate_retry_class == "single_retry"
                && response_gate_retry_pressure_tier == "medium"
                && response_gate_retry_mode == "active"
                && response_gate_next_action_lane == "dashboard.troubleshooting.recent.state"
        }
        "blocked" => {
            response_gate_retry_class == "bounded_retry"
                && response_gate_retry_pressure_tier == "high"
                && response_gate_retry_mode == "active"
                && response_gate_next_action_lane == "dashboard.troubleshooting.recent.state"
        }
        _ => false,
    };
    let response_gate_retry_contract_coherence_consistent =
        response_gate_retry_contract_vector_consistent
            && response_gate_retry_contract_family_consistent
            && response_gate_retry_contract_severity_consistent
            && response_gate_retry_mode_consistent
            && response_gate_next_action_lane_consistent
            && response_gate_retry_window_consistent;
    let response_gate_retry_contract_lane_class_consistent = match response_gate_retry_class {
        "none" => response_gate_next_action_lane == "none",
        "single_retry" | "bounded_retry" => {
            response_gate_next_action_lane == "dashboard.troubleshooting.recent.state"
        }
        _ => false,
    };
    let response_gate_retry_contract_command_class_consistent = match response_gate_retry_class {
        "none" => response_gate_next_action_command == "none",
        "single_retry" | "bounded_retry" => {
            response_gate_next_action_command == "dashboard.troubleshooting.recent.state --json"
        }
        _ => false,
    };
    let response_gate_retry_contract_expected_class_consistent =
        match response_gate_expected_retry_class {
            "none" => {
                response_gate_expected_retry_pressure_tier == "low"
                    && response_gate_expected_retry_after_seconds == 0
                    && response_gate_expected_retry_mode == "passive"
                    && response_gate_expected_next_action_lane == "none"
                    && response_gate_expected_next_action_command == "none"
            }
            "single_retry" => {
                response_gate_expected_retry_pressure_tier == "medium"
                    && response_gate_expected_retry_after_seconds == 30
                    && response_gate_expected_retry_mode == "active"
                    && response_gate_expected_next_action_lane
                        == "dashboard.troubleshooting.recent.state"
                    && response_gate_expected_next_action_command
                        == "dashboard.troubleshooting.recent.state --json"
            }
            "bounded_retry" => {
                response_gate_expected_retry_pressure_tier == "high"
                    && response_gate_expected_retry_after_seconds >= 60
                    && response_gate_expected_retry_mode == "active"
                    && response_gate_expected_next_action_lane
                        == "dashboard.troubleshooting.recent.state"
                    && response_gate_expected_next_action_command
                        == "dashboard.troubleshooting.recent.state --json"
            }
            _ => false,
        };
    let response_gate_retry_contract_pressure_class_consistent = match response_gate_retry_class {
        "none" => response_gate_retry_pressure_tier == "low",
        "single_retry" => response_gate_retry_pressure_tier == "medium",
        "bounded_retry" => response_gate_retry_pressure_tier == "high",
        _ => false,
    };
    let response_gate_retry_contract_expected_pressure_class_consistent =
        match response_gate_expected_retry_class {
            "none" => response_gate_expected_retry_pressure_tier == "low",
            "single_retry" => response_gate_expected_retry_pressure_tier == "medium",
            "bounded_retry" => response_gate_expected_retry_pressure_tier == "high",
            _ => false,
        };
    let response_gate_retry_contract_expected_command_class_consistent =
        match response_gate_expected_retry_class {
            "none" => response_gate_expected_next_action_command == "none",
            "single_retry" | "bounded_retry" => {
                response_gate_expected_next_action_command
                    == "dashboard.troubleshooting.recent.state --json"
            }
            _ => false,
        };
    let response_gate_retry_contract_expected_mode_class_consistent =
        match response_gate_expected_retry_class {
            "none" => response_gate_expected_retry_mode == "none",
            "single_retry" | "bounded_retry" => {
                response_gate_expected_retry_mode == "active"
            }
            _ => false,
        };
    let response_gate_retry_contract_expected_after_seconds_class_consistent =
        match response_gate_expected_retry_class {
            "none" => response_gate_expected_retry_after_seconds == 0,
            "single_retry" => response_gate_expected_retry_after_seconds == 30,
            "bounded_retry" => response_gate_expected_retry_after_seconds == 60,
            _ => false,
        };
    let response_gate_retry_contract_expected_after_seconds_band_class_consistent =
        match response_gate_expected_retry_class {
            "none" => response_gate_expected_retry_after_seconds == 0,
            "single_retry" => {
                response_gate_expected_retry_after_seconds >= 1
                    && response_gate_expected_retry_after_seconds < 60
            }
            "bounded_retry" => response_gate_expected_retry_after_seconds >= 60,
            _ => false,
        };
    let response_gate_retry_contract_expected_after_seconds_pressure_consistent =
        match response_gate_expected_retry_pressure_tier {
            "low" => response_gate_expected_retry_after_seconds == 0,
            "medium" => {
                response_gate_expected_retry_after_seconds >= 1
                    && response_gate_expected_retry_after_seconds < 60
            }
            "high" => response_gate_expected_retry_after_seconds >= 60,
            _ => false,
        };
    let response_gate_retry_contract_expected_mode_pressure_consistent =
        match response_gate_expected_retry_pressure_tier {
            "low" => {
                response_gate_expected_retry_mode == "none"
                    || response_gate_expected_retry_mode == "passive"
            }
            "medium" | "high" => response_gate_expected_retry_mode == "active",
            _ => false,
        };
    let response_gate_retry_contract_expected_lane_pressure_consistent =
        match response_gate_expected_retry_pressure_tier {
            "low" => response_gate_expected_next_action_lane == "none",
            "medium" | "high" => {
                response_gate_expected_next_action_lane
                    == "dashboard.troubleshooting.recent.state"
            }
            _ => false,
        };
    let response_gate_retry_contract_expected_command_pressure_consistent =
        match response_gate_expected_retry_pressure_tier {
            "low" => response_gate_expected_next_action_command == "none",
            "medium" | "high" => {
                response_gate_expected_next_action_command
                    == "dashboard.troubleshooting.recent.state --json"
            }
            _ => false,
        };
    let response_gate_retry_contract_expected_pressure_class_inverse_consistent =
        match response_gate_expected_retry_pressure_tier {
            "low" => response_gate_expected_retry_class == "none",
            "medium" => response_gate_expected_retry_class == "single_retry",
            "high" => response_gate_expected_retry_class == "bounded_retry",
            _ => false,
        };
    let response_gate_retry_contract_expected_command_mode_consistent =
        match response_gate_expected_retry_mode {
            "none" | "passive" => response_gate_expected_next_action_command == "none",
            "active" => {
                response_gate_expected_next_action_command
                    == "dashboard.troubleshooting.recent.state --json"
            }
            _ => false,
        };
    let response_gate_retry_contract_expected_after_seconds_mode_consistent =
        match response_gate_expected_retry_mode {
            "none" | "passive" => response_gate_expected_retry_after_seconds == 0,
            "active" => response_gate_expected_retry_after_seconds >= 1,
            _ => false,
        };
    let response_gate_retry_contract_expected_lane_after_seconds_consistent =
        match response_gate_expected_next_action_lane {
            "none" => response_gate_expected_retry_after_seconds == 0,
            "dashboard.troubleshooting.recent.state" => {
                response_gate_expected_retry_after_seconds >= 1
            }
            _ => false,
        };
    let response_gate_retry_contract_expected_command_after_seconds_consistent =
        match response_gate_expected_next_action_command {
            "none" => response_gate_expected_retry_after_seconds == 0,
            "dashboard.troubleshooting.recent.state --json" => {
                response_gate_expected_retry_after_seconds >= 1
            }
            _ => false,
        };
    let response_gate_retry_contract_expected_lane_command_after_seconds_consistent =
        if response_gate_expected_retry_after_seconds == 0 {
            response_gate_expected_next_action_lane == "none"
                && response_gate_expected_next_action_command == "none"
        } else {
            response_gate_expected_next_action_lane
                == "dashboard.troubleshooting.recent.state"
                && response_gate_expected_next_action_command
                    == "dashboard.troubleshooting.recent.state --json"
        };
    let response_gate_retry_contract_expected_lane_command_pressure_consistent =
        match response_gate_expected_retry_pressure_tier {
            "low" => {
                response_gate_expected_next_action_lane == "none"
                    && response_gate_expected_next_action_command == "none"
            }
            "medium" | "high" => {
                response_gate_expected_next_action_lane
                    == "dashboard.troubleshooting.recent.state"
                    && response_gate_expected_next_action_command
                        == "dashboard.troubleshooting.recent.state --json"
            }
            _ => false,
        };
    let response_gate_retry_contract_expected_lane_mode_pressure_consistent =
        match response_gate_expected_retry_pressure_tier {
            "low" => {
                response_gate_expected_next_action_lane == "none"
                    && (response_gate_expected_retry_mode == "none"
                        || response_gate_expected_retry_mode == "passive")
            }
            "medium" | "high" => {
                response_gate_expected_next_action_lane
                    == "dashboard.troubleshooting.recent.state"
                    && response_gate_expected_retry_mode == "active"
            }
            _ => false,
        };
    let response_gate_retry_contract_expected_lane_command_mode_consistent =
        match response_gate_expected_retry_mode {
            "none" | "passive" => {
                response_gate_expected_next_action_lane == "none"
                    && response_gate_expected_next_action_command == "none"
            }
            "active" => {
                response_gate_expected_next_action_lane
                    == "dashboard.troubleshooting.recent.state"
                    && response_gate_expected_next_action_command
                        == "dashboard.troubleshooting.recent.state --json"
            }
            _ => false,
        };
    let response_gate_retry_contract_expected_lane_command_mode_after_seconds_consistent =
        if response_gate_expected_retry_after_seconds == 0 {
            (response_gate_expected_retry_mode == "none"
                || response_gate_expected_retry_mode == "passive")
                && response_gate_expected_next_action_lane == "none"
                && response_gate_expected_next_action_command == "none"
        } else {
            response_gate_expected_retry_mode == "active"
                && response_gate_expected_next_action_lane
                    == "dashboard.troubleshooting.recent.state"
                && response_gate_expected_next_action_command
                    == "dashboard.troubleshooting.recent.state --json"
        };
    let response_gate_retry_contract_expected_lane_command_consistent =
        (response_gate_expected_next_action_lane == "none"
            && response_gate_expected_next_action_command == "none")
            || (response_gate_expected_next_action_lane
                == "dashboard.troubleshooting.recent.state"
                && response_gate_expected_next_action_command
                    == "dashboard.troubleshooting.recent.state --json");
    let response_gate_retry_contract_expected_lane_command_class_consistent =
        match response_gate_expected_retry_class {
            "none" => {
                response_gate_expected_next_action_lane == "none"
                    && response_gate_expected_next_action_command == "none"
            }
            "single_retry" | "bounded_retry" => {
                response_gate_expected_next_action_lane
                    == "dashboard.troubleshooting.recent.state"
                    && response_gate_expected_next_action_command
                        == "dashboard.troubleshooting.recent.state --json"
            }
            _ => false,
        };
    let response_gate_retry_contract_expected_lane_mode_class_consistent =
        match response_gate_expected_retry_class {
            "none" => {
                response_gate_expected_next_action_lane == "none"
                    && (response_gate_expected_retry_mode == "none"
                        || response_gate_expected_retry_mode == "passive")
            }
            "single_retry" | "bounded_retry" => {
                response_gate_expected_next_action_lane
                    == "dashboard.troubleshooting.recent.state"
                    && response_gate_expected_retry_mode == "active"
            }
            _ => false,
        };
    let response_gate_retry_contract_expected_lane_class_consistent =
        match response_gate_expected_retry_class {
            "none" => response_gate_expected_next_action_lane == "none",
            "single_retry" | "bounded_retry" => {
                response_gate_expected_next_action_lane
                    == "dashboard.troubleshooting.recent.state"
            }
            _ => false,
        };
    let response_gate_signature_key = format!(
        "ready={};severity={};primary={};lane={};reason={};count={};set={}",
        response_gate_ready,
        response_gate_severity,
        response_gate_primary_blocker,
        response_gate_escalation_lane,
        response_gate_escalation_reason_code,
        response_gate_blocker_count,
        response_gate_blocker_set_key
    );
    let response_gate_expected_signature_key = format!(
        "ready={};severity={};primary={};lane={};reason={};count={};set={}",
        response_gate_ready,
        response_gate_expected_severity,
        response_gate_primary_blocker_expected,
        response_gate_expected_escalation_lane,
        response_gate_expected_escalation_reason_code,
        response_gate_expected_blocker_count,
        response_gate_expected_blocker_set_key
    );
    let response_gate_signature_consistent =
        response_gate_signature_key == response_gate_expected_signature_key;
    let response_gate_escalation_reason_known = response_gate_escalation_reason_code != "unknown";
    let response_gate_escalation_vector_key = format!(
        "{}|{}|{}",
        response_gate_primary_blocker, response_gate_escalation_lane, response_gate_escalation_reason_code
    );
    let response_gate_expected_escalation_vector_key = format!(
        "{}|{}|{}",
        response_gate_primary_blocker_expected,
        response_gate_expected_escalation_lane,
        response_gate_expected_escalation_reason_code
    );
    let response_gate_escalation_signature_consistent =
        response_gate_escalation_vector_key == response_gate_expected_escalation_vector_key;
    let response_gate_escalation_vector_known = matches!(
        response_gate_escalation_vector_key.as_str(),
        "final_response_contract|dashboard.troubleshooting.recent.state|finalization_integrity_failure"
            | "answer_contract|dashboard.troubleshooting.recent.state|answer_integrity_failure"
            | "llm_reliability|dashboard.troubleshooting.snapshot.capture|llm_reliability_degraded"
            | "watchdog|dashboard.troubleshooting.summary|watchdog_pressure"
            | "none|none|none"
    );
    let response_gate_decision_vector_key = format!(
        "{}|{}|{}",
        response_gate_severity, response_gate_ready, !response_gate_ready
    );
    let response_gate_expected_decision_vector_key = format!(
        "{}|{}|{}",
        response_gate_expected_severity, response_gate_ready, !response_gate_ready
    );
    let response_gate_decision_signature_consistent =
        response_gate_decision_vector_key == response_gate_expected_decision_vector_key;
    let response_gate_decision_vector_known = matches!(
        response_gate_decision_vector_key.as_str(),
        "ready|true|false" | "degraded|false|true" | "blocked|false|true"
    );
    let response_gate_contract_consistent = response_gate_blocker_count_matches
        && response_gate_primary_blocker_matches
        && response_gate_blocker_priority_consistent
        && response_gate_blocker_set_consistent
        && response_gate_blocker_set_key_consistent
        && response_gate_blocker_count_key_consistent
        && response_gate_expected_blocker_count_matches
        && response_gate_blocker_vector_consistent
        && response_gate_signature_consistent
        && response_gate_score_consistent
        && response_gate_score_band_consistent
        && response_gate_score_band_known
        && response_gate_score_vector_consistent
        && response_gate_score_band_vector_consistent
        && response_gate_score_band_severity_consistent
        && response_gate_score_band_severity_bucket_consistent
        && response_gate_score_band_severity_bucket_known
        && response_gate_blocker_flags_consistent
        && response_gate_escalation_signature_consistent
        && response_gate_decision_signature_consistent
        && response_gate_blocker_budget_consistent
        && response_gate_manual_review_signature_consistent
        && response_gate_manual_review_reason_consistent
        && response_gate_manual_review_reason_known
        && response_gate_manual_review_vector_consistent
        && response_gate_manual_review_vector_known
        && response_gate_primary_blocker_known
        && response_gate_blockers_consistent
        && response_gate_severity_consistent
        && response_gate_manual_review_consistent
        && response_gate_escalation_contract_ok
        && response_gate_escalation_lane_known
        && response_gate_escalation_reason_known
        && response_gate_escalation_vector_known
        && response_gate_decision_vector_known
        && response_gate_next_action_command_consistent
        && response_gate_next_action_command_known
        && response_gate_next_action_lane_consistent
        && response_gate_retry_class_consistent
        && response_gate_retry_class_known
        && response_gate_retry_command_consistent
        && response_gate_retry_window_consistent
        && response_gate_retry_signature_consistent
        && response_gate_retry_signature_known
        && response_gate_lane_retry_window_consistent
        && response_gate_retry_band_consistent
        && response_gate_retry_contract_after_seconds_class_consistent
        && response_gate_retry_contract_after_seconds_score_band_consistent
        && response_gate_retry_contract_after_seconds_lane_consistent
        && response_gate_retry_contract_after_seconds_next_action_window_consistent
        && response_gate_retry_contract_lane_command_consistent
        && response_gate_retry_contract_after_seconds_lane_command_consistent
        && response_gate_retry_contract_after_seconds_command_consistent
        && response_gate_retry_mode_consistent
        && response_gate_retry_mode_known
        && response_gate_retry_contract_lane_mode_consistent
        && response_gate_retry_contract_after_seconds_mode_consistent
        && response_gate_retry_contract_after_seconds_lane_mode_consistent
        && response_gate_retry_contract_lane_command_mode_consistent
        && response_gate_retry_contract_after_seconds_lane_command_mode_consistent
        && response_gate_retry_contract_after_seconds_command_mode_consistent
        && response_gate_retry_action_vector_consistent
        && response_gate_retry_budget_consistent
        && response_gate_retry_budget_non_negative
        && response_gate_retry_budget_band_consistent
        && response_gate_retry_budget_expected_band_consistent
        && response_gate_retry_budget_range_consistent
        && response_gate_retry_budget_mode_consistent
        && response_gate_retry_contract_after_seconds_budget_consistent
        && response_gate_retry_pressure_tier_consistent
        && response_gate_retry_pressure_tier_known
        && response_gate_retry_contract_after_seconds_pressure_consistent
        && response_gate_retry_budget_vector_consistent
        && response_gate_retry_budget_vector_known
        && response_gate_retry_tier_window_consistent
        && response_gate_retry_tier_mode_consistent
        && response_gate_retry_tier_vector_consistent
        && response_gate_retry_tier_vector_known
        && response_gate_retry_contract_vector_consistent
        && response_gate_retry_contract_vector_known
        && response_gate_retry_contract_family_consistent
        && response_gate_retry_contract_severity_consistent
        && response_gate_retry_contract_coherence_consistent
        && response_gate_retry_contract_lane_class_consistent
        && response_gate_retry_contract_command_class_consistent
        && response_gate_retry_contract_expected_class_consistent
        && response_gate_retry_contract_pressure_class_consistent
        && response_gate_retry_contract_expected_pressure_class_consistent
        && response_gate_retry_contract_expected_command_class_consistent
        && response_gate_retry_contract_expected_mode_class_consistent
        && response_gate_retry_contract_expected_after_seconds_class_consistent
        && response_gate_retry_contract_expected_after_seconds_band_class_consistent
        && response_gate_retry_contract_expected_after_seconds_pressure_consistent
        && response_gate_retry_contract_expected_mode_pressure_consistent
        && response_gate_retry_contract_expected_lane_pressure_consistent
        && response_gate_retry_contract_expected_command_pressure_consistent
        && response_gate_retry_contract_expected_pressure_class_inverse_consistent
        && response_gate_retry_contract_expected_command_mode_consistent
        && response_gate_retry_contract_expected_after_seconds_mode_consistent
        && response_gate_retry_contract_expected_lane_after_seconds_consistent
        && response_gate_retry_contract_expected_command_after_seconds_consistent
        && response_gate_retry_contract_expected_lane_command_after_seconds_consistent
        && response_gate_retry_contract_expected_lane_command_pressure_consistent
        && response_gate_retry_contract_expected_lane_mode_pressure_consistent
        && response_gate_retry_contract_expected_lane_command_mode_consistent
        && response_gate_retry_contract_expected_lane_command_mode_after_seconds_consistent
        && response_gate_retry_contract_expected_lane_command_consistent
        && response_gate_retry_contract_expected_lane_command_class_consistent
        && response_gate_retry_contract_expected_lane_mode_class_consistent
        && response_gate_retry_contract_expected_lane_class_consistent
        && response_gate_retry_after_seconds_consistent
        && response_gate_retry_after_seconds_non_negative;
    let gate_health_ok = unknown_provider_count == 0
        && watchdog_critical_count == 0
        && (tool_lane_rows == 0 || execution_attempted_count >= execution_skipped_count);
    let manual_intervention_required =
        watchdog_triggered || policy_block_count > 0 || unknown_provider_count > 0;
    let provider_quality_tier = if tool_lane_rows == 0 || provider_missing_count == 0 {
        "good"
    } else if provider_resolved_count >= provider_missing_count {
        "mixed"
    } else {
        "poor"
    };
    let decision_confidence = if tool_lane_rows == 0 {
        1.0
    } else {
        let rows = tool_lane_rows as f64;
        let execution_ratio = (execution_attempted_count as f64 / rows).clamp(0.0, 1.0);
        let provider_ratio = (provider_resolved_count as f64 / rows).clamp(0.0, 1.0);
        let watchdog_ratio = if watchdog_triggered { 0.0 } else { 1.0 };
        let completion_ratio = if completion_signal_ok { 1.0 } else { 0.0 };
        ((execution_ratio * 0.40)
            + (provider_ratio * 0.30)
            + (watchdog_ratio * 0.20)
            + (completion_ratio * 0.10))
        .clamp(0.0, 1.0)
    };
    let decision_confidence = (decision_confidence * 10_000.0).round() / 10_000.0;
    let decision_confidence_label = if decision_confidence >= 0.85 {
        "high_confidence"
    } else if decision_confidence >= 0.60 {
        "medium_confidence"
    } else {
        "low_confidence"
    };
    let decision_rationale_blurb = if !answer_contract_ok {
        "Output contract failed; hold response and repair before surfacing."
    } else if !final_response_contract_ok {
        "Final response contract is incomplete; retry with full contract coverage."
    } else if watchdog_triggered || unknown_provider_count > 0 {
        "Runtime health is degraded; route through troubleshooting before user-facing claims."
    } else if decision_confidence_label == "high_confidence" {
        "Signals are aligned across execution, provider resolution, and completion checks."
    } else if decision_confidence_label == "medium_confidence" {
        "Core checks passed, but some lanes need follow-up monitoring."
    } else {
        "Confidence is limited; collect more evidence before asserting strong conclusions."
    };
    let requires_snapshot = manual_intervention_required
        || !completion_signal_ok
        || provider_quality_tier == "poor"
        || llm_reliability_tier == "low"
        || !final_response_contract_ok
        || !answer_contract_ok;
    let next_action = if !answer_contract_ok {
        "enforce_final_response_contract_and_emit_nonempty_answer"
    } else if hallucination_pattern_detected {
        "capture_workflow_snapshot_and_run_eval_audit"
    } else if placeholder_output_detected {
        "enforce_final_response_contract_and_retry_once"
    } else if watchdog_triggered {
        "inspect_recent_signature_loops_and_pause_auto_retry"
    } else if !completion_signal_ok {
        "verify_workflow_completion_signal_contract_before_retry"
    } else if unknown_provider_count > 0 {
        "refresh_provider_config_and_retry"
    } else if policy_block_count > 0 {
        "resolve_policy_or_auth_and_retry"
    } else if meta_block_count > 0 {
        "narrow_to_explicit_web_query_or_use_override"
    } else {
        "continue_normal_observation"
    };
    let next_action_lane = if !answer_contract_ok {
        "dashboard.troubleshooting.recent.state"
    } else if hallucination_pattern_detected {
        "dashboard.troubleshooting.snapshot.capture"
    } else if placeholder_output_detected {
        "dashboard.troubleshooting.summary"
    } else if watchdog_triggered {
        "dashboard.troubleshooting.summary"
    } else if !completion_signal_ok {
        "dashboard.troubleshooting.recent.state"
    } else if unknown_provider_count > 0 {
        "dashboard.troubleshooting.summary.queue_health"
    } else if policy_block_count > 0 {
        "dashboard.troubleshooting.outbox.state"
    } else if meta_block_count > 0 {
        "dashboard.troubleshooting.summary"
    } else {
        "none"
    };
    let next_action_routable = next_action_lane != "none";

    json!({
        "contract_version": "v1",
        "tool_lane_rows": tool_lane_rows,
        "execution_attempted_count": execution_attempted_count,
        "execution_skipped_count": execution_skipped_count,
        "provider_resolved_count": provider_resolved_count,
        "provider_missing_count": provider_missing_count,
        "policy_block_count": policy_block_count,
        "meta_block_count": meta_block_count,
        "unknown_provider_count": unknown_provider_count,
        "watchdog_warning_count": watchdog_warning_count,
        "watchdog_critical_count": watchdog_critical_count,
        "watchdog_triggered": watchdog_triggered,
        "completion_signal_missing_count": completion_signal_missing_count,
        "context_mismatch_count": context_mismatch_count,
        "hallucination_pattern_count": hallucination_pattern_count,
        "invalid_draft_count": invalid_draft_count,
        "placeholder_output_count": placeholder_output_count,
        "no_result_pattern_count": no_result_pattern_count,
        "answer_emitted_count": answer_emitted_count,
        "answer_missing_after_completion_count": answer_missing_after_completion_count,
        "answer_contract_ok": answer_contract_ok,
        "answer_signal_coverage": answer_signal_coverage,
        "hallucination_pattern_detected": hallucination_pattern_detected,
        "placeholder_output_detected": placeholder_output_detected,
        "no_result_pattern_detected": no_result_pattern_detected,
        "llm_reliability_tier": llm_reliability_tier,
        "llm_reliability_not_low": llm_reliability_not_low,
        "final_response_contract_violation_count": final_response_contract_violation_count,
        "final_response_contract_ok": final_response_contract_ok,
        "response_gate": {
            "ready": response_gate_ready,
            "blockers": response_gate_blockers,
            "expected_blockers": response_gate_expected_blockers,
            "blocker_set_key": response_gate_blocker_set_key,
            "expected_blocker_set_key": response_gate_expected_blocker_set_key,
            "blocker_set_consistent": response_gate_blocker_set_consistent,
            "blocker_set_key_consistent": response_gate_blocker_set_key_consistent,
            "blocker_count_key_consistent": response_gate_blocker_count_key_consistent,
            "expected_blocker_count": response_gate_expected_blocker_count,
            "expected_blocker_count_matches": response_gate_expected_blocker_count_matches,
            "blocker_budget_max": response_gate_blocker_budget_max,
            "blocker_budget_consistent": response_gate_blocker_budget_consistent,
            "blocker_vector_key": response_gate_blocker_vector_key,
            "expected_blocker_vector_key": response_gate_expected_blocker_vector_key,
            "blocker_vector_consistent": response_gate_blocker_vector_consistent,
            "blocker_flags_key": response_gate_blocker_flags_key,
            "expected_blocker_flags_key": response_gate_expected_blocker_flags_key,
            "blocker_flags_consistent": response_gate_blocker_flags_consistent,
            "signature_key": response_gate_signature_key,
            "expected_signature_key": response_gate_expected_signature_key,
            "signature_consistent": response_gate_signature_consistent,
            "blocker_count": response_gate_blocker_count,
            "blocker_count_matches": response_gate_blocker_count_matches,
            "primary_blocker": response_gate_primary_blocker,
            "primary_blocker_expected": response_gate_primary_blocker_expected,
            "primary_blocker_matches": response_gate_primary_blocker_matches,
            "blocker_priority_consistent": response_gate_blocker_priority_consistent,
            "primary_blocker_known": response_gate_primary_blocker_known,
            "blockers_consistent": response_gate_blockers_consistent,
            "severity_consistent": response_gate_severity_consistent,
            "manual_review_consistent": response_gate_manual_review_consistent,
            "escalation_lane": response_gate_escalation_lane,
            "escalation_lane_known": response_gate_escalation_lane_known,
            "escalation_reason_code": response_gate_escalation_reason_code,
            "escalation_reason_known": response_gate_escalation_reason_known,
            "escalation_vector_key": response_gate_escalation_vector_key,
            "expected_escalation_vector_key": response_gate_expected_escalation_vector_key,
            "escalation_signature_consistent": response_gate_escalation_signature_consistent,
            "escalation_vector_known": response_gate_escalation_vector_known,
            "next_action_command": response_gate_next_action_command,
            "expected_next_action_command": response_gate_expected_next_action_command,
            "next_action_command_consistent": response_gate_next_action_command_consistent,
            "next_action_command_known": response_gate_next_action_command_known,
            "next_action_lane_consistent": response_gate_next_action_lane_consistent,
            "decision_vector_key": response_gate_decision_vector_key,
            "expected_decision_vector_key": response_gate_expected_decision_vector_key,
            "decision_signature_consistent": response_gate_decision_signature_consistent,
            "decision_vector_known": response_gate_decision_vector_known,
            "expected_requires_manual_review": response_gate_expected_requires_manual_review,
            "manual_review_signature_consistent": response_gate_manual_review_signature_consistent,
            "manual_review_reason": response_gate_manual_review_reason,
            "expected_manual_review_reason": response_gate_expected_manual_review_reason,
            "manual_review_reason_consistent": response_gate_manual_review_reason_consistent,
            "manual_review_reason_known": response_gate_manual_review_reason_known,
            "manual_review_vector_key": response_gate_manual_review_vector_key,
            "expected_manual_review_vector_key": response_gate_expected_manual_review_vector_key,
            "manual_review_vector_consistent": response_gate_manual_review_vector_consistent,
            "manual_review_vector_known": response_gate_manual_review_vector_known,
            "escalation_contract_ok": response_gate_escalation_contract_ok,
            "contract_consistent": response_gate_contract_consistent,
            "score": response_gate_score,
            "expected_score": response_gate_expected_score,
            "score_consistent": response_gate_score_consistent,
            "score_band": response_gate_score_band,
            "expected_score_band": response_gate_expected_score_band,
            "score_band_consistent": response_gate_score_band_consistent,
            "score_band_known": response_gate_score_band_known,
            "score_vector_key": response_gate_score_vector_key,
            "expected_score_vector_key": response_gate_expected_score_vector_key,
            "score_vector_consistent": response_gate_score_vector_consistent,
            "score_band_vector_key": response_gate_score_band_vector_key,
            "expected_score_band_vector_key": response_gate_expected_score_band_vector_key,
            "score_band_vector_consistent": response_gate_score_band_vector_consistent,
            "expected_severity_from_score_band": response_gate_expected_severity_from_score_band,
            "score_band_severity_consistent": response_gate_score_band_severity_consistent,
            "score_band_severity_bucket_consistent": response_gate_score_band_severity_bucket_consistent,
            "score_band_severity_bucket_known": response_gate_score_band_severity_bucket_known,
            "retry_class": response_gate_retry_class,
            "expected_retry_class": response_gate_expected_retry_class,
            "retry_class_consistent": response_gate_retry_class_consistent,
            "retry_class_known": response_gate_retry_class_known,
            "retry_command_consistent": response_gate_retry_command_consistent,
            "retry_window_consistent": response_gate_retry_window_consistent,
            "retry_signature_key": response_gate_retry_signature_key,
            "expected_retry_signature_key": response_gate_expected_retry_signature_key,
            "retry_signature_consistent": response_gate_retry_signature_consistent,
            "retry_signature_known": response_gate_retry_signature_known,
            "lane_retry_window_consistent": response_gate_lane_retry_window_consistent,
            "retry_band_consistent": response_gate_retry_band_consistent,
            "retry_contract_after_seconds_class_consistent": response_gate_retry_contract_after_seconds_class_consistent,
            "retry_contract_after_seconds_score_band_consistent": response_gate_retry_contract_after_seconds_score_band_consistent,
            "retry_contract_after_seconds_lane_consistent": response_gate_retry_contract_after_seconds_lane_consistent,
            "retry_contract_after_seconds_next_action_window_consistent": response_gate_retry_contract_after_seconds_next_action_window_consistent,
            "retry_contract_lane_command_consistent": response_gate_retry_contract_lane_command_consistent,
            "retry_contract_after_seconds_lane_command_consistent": response_gate_retry_contract_after_seconds_lane_command_consistent,
            "retry_contract_after_seconds_command_consistent": response_gate_retry_contract_after_seconds_command_consistent,
            "retry_mode": response_gate_retry_mode,
            "expected_retry_mode": response_gate_expected_retry_mode,
            "retry_mode_consistent": response_gate_retry_mode_consistent,
            "retry_mode_known": response_gate_retry_mode_known,
            "retry_contract_lane_mode_consistent": response_gate_retry_contract_lane_mode_consistent,
            "retry_contract_after_seconds_mode_consistent": response_gate_retry_contract_after_seconds_mode_consistent,
            "retry_contract_after_seconds_lane_mode_consistent": response_gate_retry_contract_after_seconds_lane_mode_consistent,
            "retry_contract_lane_command_mode_consistent": response_gate_retry_contract_lane_command_mode_consistent,
            "retry_contract_after_seconds_lane_command_mode_consistent": response_gate_retry_contract_after_seconds_lane_command_mode_consistent,
            "retry_contract_after_seconds_command_mode_consistent": response_gate_retry_contract_after_seconds_command_mode_consistent,
            "retry_action_vector_key": response_gate_retry_action_vector_key,
            "expected_retry_action_vector_key": response_gate_expected_retry_action_vector_key,
            "retry_action_vector_consistent": response_gate_retry_action_vector_consistent,
            "retry_budget_points": response_gate_retry_budget_points,
            "expected_retry_budget_points": response_gate_expected_retry_budget_points,
            "retry_budget_consistent": response_gate_retry_budget_consistent,
            "retry_budget_non_negative": response_gate_retry_budget_non_negative,
            "expected_retry_budget_from_band": response_gate_expected_retry_budget_from_band,
            "retry_budget_band_consistent": response_gate_retry_budget_band_consistent,
            "expected_retry_budget_from_expected_band": response_gate_expected_retry_budget_from_expected_band,
            "retry_budget_expected_band_consistent": response_gate_retry_budget_expected_band_consistent,
            "retry_budget_range_consistent": response_gate_retry_budget_range_consistent,
            "retry_budget_mode_consistent": response_gate_retry_budget_mode_consistent,
            "retry_contract_after_seconds_budget_consistent": response_gate_retry_contract_after_seconds_budget_consistent,
            "retry_pressure_tier": response_gate_retry_pressure_tier,
            "expected_retry_pressure_tier": response_gate_expected_retry_pressure_tier,
            "retry_pressure_tier_consistent": response_gate_retry_pressure_tier_consistent,
            "retry_pressure_tier_known": response_gate_retry_pressure_tier_known,
            "retry_contract_after_seconds_pressure_consistent": response_gate_retry_contract_after_seconds_pressure_consistent,
            "retry_budget_vector_key": response_gate_retry_budget_vector_key,
            "expected_retry_budget_vector_key": response_gate_expected_retry_budget_vector_key,
            "retry_budget_vector_consistent": response_gate_retry_budget_vector_consistent,
            "retry_budget_vector_known": response_gate_retry_budget_vector_known,
            "retry_tier_window_consistent": response_gate_retry_tier_window_consistent,
            "retry_tier_mode_consistent": response_gate_retry_tier_mode_consistent,
            "retry_tier_vector_key": response_gate_retry_tier_vector_key,
            "expected_retry_tier_vector_key": response_gate_expected_retry_tier_vector_key,
            "retry_tier_vector_consistent": response_gate_retry_tier_vector_consistent,
            "retry_tier_vector_known": response_gate_retry_tier_vector_known,
            "retry_contract_vector_key": response_gate_retry_contract_vector_key,
            "expected_retry_contract_vector_key": response_gate_expected_retry_contract_vector_key,
            "retry_contract_vector_consistent": response_gate_retry_contract_vector_consistent,
            "retry_contract_vector_known": response_gate_retry_contract_vector_known,
            "retry_contract_family_consistent": response_gate_retry_contract_family_consistent,
            "retry_contract_severity_consistent": response_gate_retry_contract_severity_consistent,
            "retry_contract_coherence_consistent": response_gate_retry_contract_coherence_consistent,
            "retry_contract_lane_class_consistent": response_gate_retry_contract_lane_class_consistent,
            "retry_contract_command_class_consistent": response_gate_retry_contract_command_class_consistent,
            "retry_contract_expected_class_consistent": response_gate_retry_contract_expected_class_consistent,
            "retry_contract_pressure_class_consistent": response_gate_retry_contract_pressure_class_consistent,
            "retry_contract_expected_pressure_class_consistent": response_gate_retry_contract_expected_pressure_class_consistent,
            "retry_contract_expected_command_class_consistent": response_gate_retry_contract_expected_command_class_consistent,
            "retry_contract_expected_mode_class_consistent": response_gate_retry_contract_expected_mode_class_consistent,
            "retry_contract_expected_after_seconds_class_consistent": response_gate_retry_contract_expected_after_seconds_class_consistent,
            "retry_contract_expected_after_seconds_band_class_consistent": response_gate_retry_contract_expected_after_seconds_band_class_consistent,
            "retry_contract_expected_after_seconds_pressure_consistent": response_gate_retry_contract_expected_after_seconds_pressure_consistent,
            "retry_contract_expected_mode_pressure_consistent": response_gate_retry_contract_expected_mode_pressure_consistent,
            "retry_contract_expected_lane_pressure_consistent": response_gate_retry_contract_expected_lane_pressure_consistent,
            "retry_contract_expected_command_pressure_consistent": response_gate_retry_contract_expected_command_pressure_consistent,
            "retry_contract_expected_pressure_class_inverse_consistent": response_gate_retry_contract_expected_pressure_class_inverse_consistent,
            "retry_contract_expected_command_mode_consistent": response_gate_retry_contract_expected_command_mode_consistent,
            "retry_contract_expected_after_seconds_mode_consistent": response_gate_retry_contract_expected_after_seconds_mode_consistent,
            "retry_contract_expected_lane_after_seconds_consistent": response_gate_retry_contract_expected_lane_after_seconds_consistent,
            "retry_contract_expected_command_after_seconds_consistent": response_gate_retry_contract_expected_command_after_seconds_consistent,
            "retry_contract_expected_lane_command_after_seconds_consistent": response_gate_retry_contract_expected_lane_command_after_seconds_consistent,
            "retry_contract_expected_lane_command_pressure_consistent": response_gate_retry_contract_expected_lane_command_pressure_consistent,
            "retry_contract_expected_lane_mode_pressure_consistent": response_gate_retry_contract_expected_lane_mode_pressure_consistent,
            "retry_contract_expected_lane_command_mode_consistent": response_gate_retry_contract_expected_lane_command_mode_consistent,
            "retry_contract_expected_lane_command_mode_after_seconds_consistent": response_gate_retry_contract_expected_lane_command_mode_after_seconds_consistent,
            "retry_contract_expected_lane_command_consistent": response_gate_retry_contract_expected_lane_command_consistent,
            "retry_contract_expected_lane_command_class_consistent": response_gate_retry_contract_expected_lane_command_class_consistent,
            "retry_contract_expected_lane_mode_class_consistent": response_gate_retry_contract_expected_lane_mode_class_consistent,
            "retry_contract_expected_lane_class_consistent": response_gate_retry_contract_expected_lane_class_consistent,
            "retry_after_seconds": response_gate_retry_after_seconds,
            "expected_retry_after_seconds": response_gate_expected_retry_after_seconds,
            "retry_after_seconds_consistent": response_gate_retry_after_seconds_consistent,
            "retry_after_seconds_non_negative": response_gate_retry_after_seconds_non_negative,
            "severity": response_gate_severity,
            "requires_manual_review": response_gate_requires_manual_review
        },
        "completion_signal_ok": completion_signal_ok,
        "provider_resolution_ok": provider_resolution_ok,
        "provider_quality_tier": provider_quality_tier,
        "decision_confidence": decision_confidence,
        "decision_confidence_label": decision_confidence_label,
        "decision_rationale_blurb": decision_rationale_blurb,
        "confidence_copy_standard_version": "v11_confidence_rationale_v1",
        "requires_snapshot": requires_snapshot,
        "gate_health_ok": gate_health_ok,
        "manual_intervention_required": manual_intervention_required,
        "next_action": next_action,
        "next_action_lane": next_action_lane,
        "next_action_routable": next_action_routable
    })
}
