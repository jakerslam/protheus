fn run_test_communication(state: &mut SwarmState, argv: &[String]) -> Result<Value, String> {
    let delivery = DeliveryGuarantee::from_flag(parse_flag(argv, "delivery"));
    let options = SpawnOptions {
        verify: true,
        timeout_ms: 1_000,
        metrics_detailed: true,
        simulate_unreachable: false,
        byzantine: false,
        corruption_type: "data_falsification".to_string(),
        token_budget: Some(1200),
        token_warning_threshold: 0.8,
        budget_exhaustion_action: BudgetAction::AllowWithWarning,
        adaptive_complexity: true,
        execution_mode: ExecutionMode::TaskOriented,
        role: None,
        capabilities: Vec::new(),
        auto_publish_results: false,
        agent_label: None,
        result_value: None,
        result_text: None,
        result_confidence: 1.0,
        verification_status: "not_verified".to_string(),
    };

    let generator_id = spawn_single(
        state,
        None,
        "swarm-test-6-generator",
        8,
        &SpawnOptions {
            role: Some("generator".to_string()),
            capabilities: vec!["generate".to_string(), "relay".to_string()],
            ..options.clone()
        },
    )?
    .get("session_id")
    .and_then(Value::as_str)
    .ok_or_else(|| "generator_session_id_missing".to_string())?
    .to_string();
    let filter_id = spawn_single(
        state,
        None,
        "swarm-test-6-filter",
        8,
        &SpawnOptions {
            role: Some("filter".to_string()),
            capabilities: vec!["filter".to_string()],
            ..options.clone()
        },
    )?
    .get("session_id")
    .and_then(Value::as_str)
    .ok_or_else(|| "filter_session_id_missing".to_string())?
    .to_string();
    let summarizer_id = spawn_single(
        state,
        None,
        "swarm-test-6-summarizer",
        8,
        &SpawnOptions {
            role: Some("summarizer".to_string()),
            capabilities: vec!["summarize".to_string()],
            ..options.clone()
        },
    )?
    .get("session_id")
    .and_then(Value::as_str)
    .ok_or_else(|| "summarizer_session_id_missing".to_string())?
    .to_string();
    let validator_id = spawn_single(
        state,
        None,
        "swarm-test-6-validator",
        8,
        &SpawnOptions {
            role: Some("validator".to_string()),
            capabilities: vec!["validate".to_string()],
            ..options
        },
    )?
    .get("session_id")
    .and_then(Value::as_str)
    .ok_or_else(|| "validator_session_id_missing".to_string())?
    .to_string();

    let top_10 = vec![
        "Write clear, self-documenting code",
        "Practice test-driven development",
        "Use version control with clear commits",
        "Conduct regular code reviews",
        "Keep modules single-responsibility",
        "Document APIs and interfaces",
        "Automate CI/CD",
        "Refactor to reduce debt",
        "Design for scalability",
        "Treat security as default",
    ];
    let generator_payload = serde_json::to_string(&json!({
        "report": {"original_list_size": top_10.len()},
        "payload": {"items": top_10},
    }))
    .map_err(|err| format!("encode_generator_payload_failed:{err}"))?;
    let send_1 = send_session_message(
        state,
        &generator_id,
        &filter_id,
        &generator_payload,
        delivery.clone(),
        parse_bool_flag(argv, "simulate-first-attempt-fail", false),
        DEFAULT_MESSAGE_TTL_MS,
    )?;

    let filter_inbox = receive_session_messages(state, &filter_id, 1, true)?;
    let filter_message = filter_inbox
        .get("messages")
        .and_then(Value::as_array)
        .and_then(|rows| rows.first())
        .and_then(|row| row.get("payload"))
        .and_then(Value::as_str)
        .ok_or_else(|| "filter_message_missing".to_string())?;
    let filter_payload_json: Value = serde_json::from_str(filter_message)
        .map_err(|err| format!("filter_payload_parse_failed:{err}"))?;
    let items = filter_payload_json
        .get("payload")
        .and_then(|row| row.get("items"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let top_5 = items.into_iter().take(5).collect::<Vec<_>>();
    let filter_payload = serde_json::to_string(&json!({
        "report": {"filtered_list_size": top_5.len()},
        "payload": {"top_5_items": top_5},
    }))
    .map_err(|err| format!("encode_filter_payload_failed:{err}"))?;
    let send_2 = send_session_message(
        state,
        &filter_id,
        &summarizer_id,
        &filter_payload,
        delivery.clone(),
        false,
        DEFAULT_MESSAGE_TTL_MS,
    )?;

    let summarizer_inbox = receive_session_messages(state, &summarizer_id, 1, true)?;
    let summarizer_message = summarizer_inbox
        .get("messages")
        .and_then(Value::as_array)
        .and_then(|rows| rows.first())
        .and_then(|row| row.get("payload"))
        .and_then(Value::as_str)
        .ok_or_else(|| "summarizer_message_missing".to_string())?;
    let summarizer_payload_json: Value = serde_json::from_str(summarizer_message)
        .map_err(|err| format!("summarizer_payload_parse_failed:{err}"))?;
    let top_5_values = summarizer_payload_json
        .get("payload")
        .and_then(|row| row.get("top_5_items"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let top_3 = top_5_values.into_iter().take(3).collect::<Vec<_>>();
    let summarizer_payload = serde_json::to_string(&json!({
        "payload": {
            "top_3_items": top_3,
            "summary": "Top practices prioritize readable code, TDD, and disciplined version control."
        }
    }))
    .map_err(|err| format!("encode_summarizer_payload_failed:{err}"))?;
    let send_3 = send_session_message(
        state,
        &summarizer_id,
        &validator_id,
        &summarizer_payload,
        delivery,
        false,
        DEFAULT_MESSAGE_TTL_MS,
    )?;

    let validator_inbox = receive_session_messages(state, &validator_id, 1, true)?;
    let validator_message = validator_inbox
        .get("messages")
        .and_then(Value::as_array)
        .and_then(|rows| rows.first())
        .cloned()
        .unwrap_or(Value::Null);
    let chain_complete = validator_message
        .get("payload")
        .and_then(Value::as_str)
        .and_then(|payload| serde_json::from_str::<Value>(payload).ok())
        .and_then(|payload| payload.get("payload").cloned())
        .and_then(|payload| payload.get("top_3_items").cloned())
        .and_then(|payload| payload.as_array().map(|rows| rows.len()))
        .map(|len| len == 3)
        .unwrap_or(false);

    Ok(json!({
        "ok": true,
        "type": "swarm_runtime_test_communication",
        "delivery": DeliveryGuarantee::from_flag(parse_flag(argv, "delivery")).as_label(),
        "chain_complete": chain_complete,
        "sessions": {
            "generator": generator_id,
            "filter": filter_id,
            "summarizer": summarizer_id,
            "validator": validator_id,
        },
        "messages": [send_1, send_2, send_3],
        "validator_inbox": validator_inbox,
    }))
}

fn run_test_heterogeneous(
    state: &mut SwarmState,
    state_file: &Path,
    argv: &[String],
) -> Result<Value, String> {
    let start = parse_u64_flag(argv, "range-start", 1);
    let end = parse_u64_flag(argv, "range-end", 100);
    if end < start {
        return Err(format!("invalid_range:start={start}:end={end}"));
    }
    let expected_sum = ((start + end) * (end - start + 1)) as f64 / 2.0;
    let label_pattern = parse_flag(argv, "label-pattern")
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "swarm-test-7-het-agent-*".to_string());
    let timeout_ms = (parse_f64_flag(argv, "timeout-sec", 30.0).max(0.1) * 1000.0) as u64;
    let min_count = parse_u64_flag(argv, "min-count", 2) as usize;

    let base = SpawnOptions {
        verify: true,
        timeout_ms: 1_000,
        metrics_detailed: true,
        simulate_unreachable: false,
        byzantine: false,
        corruption_type: "data_falsification".to_string(),
        token_budget: Some(1_200),
        token_warning_threshold: 0.8,
        budget_exhaustion_action: BudgetAction::AllowWithWarning,
        adaptive_complexity: true,
        execution_mode: ExecutionMode::TaskOriented,
        role: Some("calculator".to_string()),
        capabilities: vec!["calculate".to_string(), "verify".to_string()],
        auto_publish_results: true,
        agent_label: None,
        result_value: Some(expected_sum),
        result_text: None,
        result_confidence: 1.0,
        verification_status: "not_verified".to_string(),
    };

    let fast = spawn_single(
        state,
        None,
        &format!("Calculate {start}-{end} quickly"),
        8,
        &SpawnOptions {
            agent_label: Some("swarm-test-7-het-agent-fast".to_string()),
            result_confidence: 1.0,
            verification_status: "not_verified".to_string(),
            ..base.clone()
        },
    )?;
    let thorough = spawn_single(
        state,
        None,
        &format!("Calculate and verify {start}-{end}"),
        8,
        &SpawnOptions {
            agent_label: Some("swarm-test-7-het-agent-thorough".to_string()),
            result_confidence: 0.98,
            verification_status: "verified".to_string(),
            ..base.clone()
        },
    )?;
    let coordinator = spawn_single(
        state,
        None,
        "Coordinate heterogeneous consensus",
        8,
        &SpawnOptions {
            role: Some("coordinator".to_string()),
            capabilities: vec!["consensus".to_string(), "audit".to_string()],
            auto_publish_results: false,
            result_value: None,
            agent_label: Some("swarm-test-7-het-agent-coordinator".to_string()),
            ..base
        },
    )?;

    let filters = ResultFilters {
        label_pattern: Some(label_pattern.clone()),
        role: None,
        task_id: None,
        session_id: None,
    };
    let results = wait_for_results(state_file, state, &filters, min_count, timeout_ms)?;
    let consensus = analyze_result_consensus(&results, "value", 1.0);
    let outliers = analyze_result_outliers(&results, "value");
    let coordination_success = consensus
        .get("consensus_reached")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    Ok(json!({
        "ok": true,
        "type": "swarm_runtime_test_heterogeneous",
        "range_start": start,
        "range_end": end,
        "expected_sum": expected_sum,
        "coordinator_status": if coordination_success { "success" } else { "failed" },
        "coordination_success": coordination_success,
        "result_count": results.len(),
        "sessions": {
            "fast": fast,
            "thorough": thorough,
            "coordinator": coordinator,
        },
        "consensus": consensus,
        "outliers": outliers,
        "results": results,
    }))
}

fn parse_reports_from_flag(reports_flag: Option<String>) -> Vec<AgentReport> {
    reports_flag
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .map(|value| parse_reports(&value))
        .unwrap_or_default()
}
