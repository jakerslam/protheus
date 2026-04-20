
fn load_policy(root: &Path, argv: &[String]) -> Policy {
    let policy_path = resolve_path(
        root,
        parse_cli_flag(argv, "policy"),
        Path::new(DEFAULT_POLICY_PATH),
    );
    let raw = read_json(&policy_path).unwrap_or_else(|| json!({}));
    let auto = raw
        .get("auto_approval")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let degradation = auto
        .get("degradation_threshold")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let paths = raw
        .get("paths")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    let state_path = resolve_path(
        root,
        parse_cli_flag(argv, "state-path").or_else(|| {
            paths
                .get("state_path")
                .and_then(Value::as_str)
                .map(str::to_string)
        }),
        Path::new(DEFAULT_STATE_PATH),
    );
    let latest_path = resolve_path(
        root,
        parse_cli_flag(argv, "latest-path").or_else(|| {
            paths
                .get("latest_path")
                .and_then(Value::as_str)
                .map(str::to_string)
        }),
        Path::new(DEFAULT_LATEST_PATH),
    );
    let receipts_path = resolve_path(
        root,
        parse_cli_flag(argv, "receipts-path").or_else(|| {
            paths
                .get("receipts_path")
                .and_then(Value::as_str)
                .map(str::to_string)
        }),
        Path::new(DEFAULT_RECEIPTS_PATH),
    );
    let regrets_path = resolve_path(
        root,
        parse_cli_flag(argv, "regrets-path").or_else(|| {
            paths
                .get("regrets_path")
                .and_then(Value::as_str)
                .map(str::to_string)
        }),
        Path::new(DEFAULT_REGRETS_PATH),
    );

    Policy {
        enabled: raw.get("enabled").and_then(Value::as_bool).unwrap_or(true)
            && auto.get("enabled").and_then(Value::as_bool).unwrap_or(true),
        min_confidence: auto
            .get("min_confidence")
            .and_then(Value::as_f64)
            .unwrap_or(0.85),
        min_historical_success_rate: auto
            .get("min_historical_success_rate")
            .and_then(Value::as_f64)
            .unwrap_or(0.90),
        max_impact_score: auto
            .get("max_impact_score")
            .and_then(Value::as_f64)
            .unwrap_or(50.0),
        excluded_types: auto
            .get("excluded_types")
            .and_then(Value::as_array)
            .map(|rows| {
                rows.iter()
                    .filter_map(Value::as_str)
                    .map(|v| v.trim().to_ascii_lowercase())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default(),
        auto_rollback_on_degradation: auto
            .get("auto_rollback_on_degradation")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        max_drift_delta: degradation
            .get("max_drift_delta")
            .and_then(Value::as_f64)
            .unwrap_or(0.01),
        max_yield_drop: degradation
            .get("max_yield_drop")
            .and_then(Value::as_f64)
            .unwrap_or(0.05),
        rollback_window_minutes: auto
            .get("rollback_window_minutes")
            .and_then(Value::as_i64)
            .unwrap_or(30)
            .clamp(1, 10080),
        regret_issue_label: auto
            .get("regret_issue_label")
            .and_then(Value::as_str)
            .unwrap_or("auto_approval_regret")
            .to_string(),
        state_path,
        latest_path,
        receipts_path,
        regrets_path,
    }
}

fn load_state(state_path: &Path) -> Value {
    read_json(state_path).unwrap_or_else(|| {
        json!({
            "version": "1.0",
            "pending_commit": [],
            "committed": [],
            "rolled_back": []
        })
    })
}

fn store_state(policy: &Policy, state: &Value) -> Result<(), String> {
    write_json(&policy.state_path, state)
}

fn parse_proposal(argv: &[String]) -> Result<Value, String> {
    if let Some(raw) = parse_cli_flag(argv, "proposal-json") {
        return serde_json::from_str::<Value>(&raw)
            .map_err(|e| format!("proposal_json_parse_failed:{e}"));
    }
    if let Some(file) = parse_cli_flag(argv, "proposal-file") {
        let raw = fs::read_to_string(file).map_err(|e| format!("proposal_file_read_failed:{e}"))?;
        return serde_json::from_str::<Value>(&raw)
            .map_err(|e| format!("proposal_file_parse_failed:{e}"));
    }
    Err("missing_proposal_payload".to_string())
}

fn proposal_summary(proposal: &Value) -> ProposalSummary {
    let id = proposal
        .get("id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| stable_proposal_id(proposal));
    ProposalSummary {
        id,
        title: value_string(proposal.get("title"), "Untitled proposal"),
        proposal_type: value_string(
            proposal
                .get("proposal_type")
                .or_else(|| proposal.get("type"))
                .or_else(|| proposal.get("kind")),
            "generic",
        )
        .to_ascii_lowercase(),
        confidence: value_f64(proposal.get("confidence"), 0.0),
        historical_success_rate: value_f64(
            proposal
                .get("historical_success_rate")
                .or_else(|| proposal.get("historical_success")),
            0.0,
        ),
        impact_score: value_f64(proposal.get("impact_score"), 100.0),
        raw: proposal.clone(),
    }
}

fn evaluate_proposal(policy: &Policy, proposal: &ProposalSummary) -> (bool, Vec<String>) {
    let mut reasons = Vec::new();
    if !policy.enabled {
        reasons.push("auto_approval_disabled".to_string());
    }
    if policy
        .excluded_types
        .iter()
        .any(|entry| entry == &proposal.proposal_type)
    {
        reasons.push(format!("excluded_type:{}", proposal.proposal_type));
    }
    if proposal.confidence < policy.min_confidence {
        reasons.push(format!(
            "confidence_below_floor:{:.3}<{:.3}",
            proposal.confidence, policy.min_confidence
        ));
    }
    if proposal.historical_success_rate < policy.min_historical_success_rate {
        reasons.push(format!(
            "historical_success_below_floor:{:.3}<{:.3}",
            proposal.historical_success_rate, policy.min_historical_success_rate
        ));
    }
    if proposal.impact_score > policy.max_impact_score {
        reasons.push(format!(
            "impact_score_above_cap:{:.3}>{:.3}",
            proposal.impact_score, policy.max_impact_score
        ));
    }
    (reasons.is_empty(), reasons)
}
