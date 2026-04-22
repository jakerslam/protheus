fn evaluate_consensus(reports: &[AgentReport], fields: &[String], threshold: f64) -> Value {
    if reports.is_empty() {
        return json!({
            "consensus_reached": false,
            "reason_code": "no_reports",
            "confidence": 0.0,
            "outliers": []
        });
    }

    let mut groups: BTreeMap<String, Vec<(String, Map<String, Value>)>> = BTreeMap::new();
    for report in reports {
        let mut selected = Map::new();
        for field in fields {
            selected.insert(
                field.clone(),
                report.values.get(field).cloned().unwrap_or(Value::Null),
            );
        }
        let fingerprint = crate::deterministic_receipt_hash(&Value::Object(selected.clone()));
        groups
            .entry(fingerprint)
            .or_default()
            .push((report.agent_id.clone(), selected));
    }

    let Some((leader_fp, leader_group)) = groups.iter().max_by_key(|(_, rows)| rows.len()) else {
        return json!({
            "consensus_reached": false,
            "reason_code": "grouping_failed",
            "confidence": 0.0,
            "outliers": []
        });
    };

    let extractable_count = groups.values().map(Vec::len).sum::<usize>();
    let confidence = if extractable_count == 0 {
        0.0
    } else {
        leader_group.len() as f64 / extractable_count as f64
    };
    let mut outliers = Vec::new();
    for (fingerprint, rows) in &groups {
        if fingerprint == leader_fp {
            continue;
        }
        for (agent_id, selected) in rows {
            outliers.push(json!({
                "agent": agent_id,
                "values": selected,
                "deviation": "outlier_group"
            }));
        }
    }

    let agreed_value = leader_group
        .first()
        .map(|(_, selected)| Value::Object(selected.clone()))
        .unwrap_or(Value::Object(Map::new()));
    let disagreement_count = extractable_count.saturating_sub(leader_group.len());
    let outlier_rate = if extractable_count == 0 {
        0.0
    } else {
        disagreement_count as f64 / extractable_count as f64
    };
    let confidence_band = if confidence >= 0.9 {
        "high"
    } else if confidence >= threshold {
        "medium"
    } else {
        "low"
    };
    let reason_code = if confidence >= threshold && disagreement_count == 0 {
        "majority_unanimous"
    } else if confidence >= threshold {
        "majority_with_outliers"
    } else {
        "insufficient_majority"
    };
    let recommended_action = if confidence >= threshold && disagreement_count == 0 {
        "accept_majority"
    } else if confidence >= threshold {
        "accept_with_outlier_review"
    } else {
        "request_additional_agents"
    };

    json!({
        "consensus_reached": confidence >= threshold,
        "reason_code": reason_code,
        "confidence": confidence,
        "confidence_band": confidence_band,
        "threshold": threshold,
        "sample_size": reports.len(),
        "extractable_count": extractable_count,
        "group_count": groups.len(),
        "agreement_count": leader_group.len(),
        "disagreement_count": disagreement_count,
        "outlier_rate": outlier_rate,
        "dominant_fingerprint": clean_text(leader_fp, 24),
        "agreed_value": agreed_value,
        "recommended_action": recommended_action,
        "outliers": outliers,
        "fields": fields,
    })
}

fn default_swarm_test_spawn_options() -> SpawnOptions {
    SpawnOptions {
        verify: true,
        timeout_ms: 1_000,
        metrics_detailed: true,
        simulate_unreachable: false,
        byzantine: false,
        corruption_type: "data_falsification".to_string(),
        token_budget: None,
        token_warning_threshold: 0.8,
        budget_exhaustion_action: BudgetAction::FailHard,
        adaptive_complexity: false,
        execution_mode: ExecutionMode::TaskOriented,
        role: None,
        capabilities: Vec::new(),
        auto_publish_results: false,
        agent_label: None,
        result_value: None,
        result_text: None,
        result_confidence: 1.0,
        verification_status: "not_verified".to_string(),
    }
}

fn run_test_recursive(state: &mut SwarmState, argv: &[String]) -> Result<Value, String> {
    let levels = parse_u8_flag(argv, "levels", 5);
    let mut options = default_swarm_test_spawn_options();
    options.timeout_ms = parse_u64_flag(argv, "timeout-ms", 1_000);

    let result = recursive_spawn_with_tracking(
        state,
        None,
        &format!("recursive-test-{levels}"),
        levels,
        levels.saturating_add(1),
        &options,
    )?;

    Ok(json!({
        "ok": true,
        "test": "recursive",
        "levels_requested": levels,
        "levels_completed": result
            .get("lineage")
            .and_then(Value::as_array)
            .map(|rows| rows.len())
            .unwrap_or(0),
        "result": result
    }))
}

fn run_test_byzantine(state: &mut SwarmState, argv: &[String]) -> Result<Value, String> {
    let agent_count = parse_u64_flag(argv, "agents", 5).max(1);
    let corrupt_count = parse_u64_flag(argv, "corrupt", 2).min(agent_count);
    let threshold = parse_f64_flag(argv, "threshold", 0.6);

    state.byzantine_test_mode = true;
    let mut reports = Vec::new();
    for idx in 0..agent_count {
        let is_corrupt = idx < corrupt_count;
        let values = if is_corrupt {
            let mut map = BTreeMap::new();
            map.insert("file".to_string(), Value::String("SOUL.md".to_string()));
            map.insert("file_size".to_string(), Value::Number(9999u64.into()));
            map.insert("word_count".to_string(), Value::Number(5000u64.into()));
            map.insert(
                "first_line".to_string(),
                Value::String("FAKE DATA HERE".to_string()),
            );
            map
        } else {
            let mut map = BTreeMap::new();
            map.insert("file".to_string(), Value::String("SOUL.md".to_string()));
            map.insert("file_size".to_string(), Value::Number(1847u64.into()));
            map.insert("word_count".to_string(), Value::Number(292u64.into()));
            map.insert(
                "first_line".to_string(),
                Value::String("# SOUL.md".to_string()),
            );
            map
        };
        reports.push(AgentReport {
            agent_id: format!("agent-{:02}", idx + 1),
            values,
        });
    }

    let fields = vec![
        "file".to_string(),
        "file_size".to_string(),
        "word_count".to_string(),
        "first_line".to_string(),
    ];
    let consensus = evaluate_consensus(&reports, &fields, threshold);
    let outliers = consensus
        .get("outliers")
        .and_then(Value::as_array)
        .map(|rows| rows.len())
        .unwrap_or(0);

    Ok(json!({
        "ok": true,
        "test": "byzantine",
        "byzantine_test_mode": state.byzantine_test_mode,
        "agents": agent_count,
        "corrupt_requested": corrupt_count,
        "corrupt_detected": outliers,
        "consensus": consensus,
        "truth_constraints_disabled_for_testing": true
    }))
}
