fn classify_metric(metric_text: &str, target_text: &str, source_text: &str) -> String {
    let metric = normalize_spaces(metric_text).to_ascii_lowercase();
    let text = normalize_spaces(&format!("{} {} {}", metric_text, target_text, source_text))
        .to_ascii_lowercase();
    let contains_any = |tokens: &[&str]| tokens.iter().any(|token| text.contains(token));

    if metric.is_empty() && contains_any(&["reply", "interview"]) {
        return "reply_or_interview_count".to_string();
    }
    if metric.is_empty()
        && text.contains("outreach")
        && contains_any(&["artifact", "draft", "offer", "proposal"])
    {
        return "outreach_artifact".to_string();
    }

    match metric.as_str() {
        "validation_metric" | "validation_check" | "verification_metric" | "verification_check" => {
            return "postconditions_ok".to_string()
        }
        "outreach_artifact" => return "outreach_artifact".to_string(),
        "reply_or_interview_count"
        | "reply_count"
        | "interview_count"
        | "outreach_reply_count"
        | "outreach_interview_count" => return "reply_or_interview_count".to_string(),
        "artifact_count"
        | "experiment_artifact"
        | "collector_success_runs"
        | "hypothesis_signal_lift"
        | "outreach_artifact_count"
        | "offer_draft_count"
        | "proposal_draft_count" => return "artifact_count".to_string(),
        "verification_checks_passed" | "postconditions_ok" => {
            return "postconditions_ok".to_string()
        }
        "collector_failure_streak" | "queue_outcome_logged" => {
            return "queue_outcome_logged".to_string()
        }
        "retry_count" | "timeout_count" | "abort_count" => {
            return "queue_outcome_logged".to_string()
        }
        "backoff_ms" | "retry_backoff_ms" => return "duration_ms".to_string(),
        "entries_count" => return "entries_count".to_string(),
        "revenue_actions_count" => return "revenue_actions_count".to_string(),
        "token_usage" => return "token_usage".to_string(),
        "duration_ms" => return "duration_ms".to_string(),
        "execution_success" => return "execution_success".to_string(),
        _ => {}
    }

    if contains_any(&["reply", "interview"]) {
        return "reply_or_interview_count".to_string();
    }
    if contains_any(&[
        "backoff ms",
        "backoff_ms",
        "retry delay",
        "cooldown",
        "wait window",
        "sleep window",
    ]) {
        return "duration_ms".to_string();
    }
    if text.contains("outreach") && contains_any(&["artifact", "draft", "offer", "proposal"]) {
        return "outreach_artifact".to_string();
    }
    if contains_any(&[
        "artifact",
        "draft",
        "experiment",
        "patch",
        "plan",
        "deliverable",
    ]) {
        return "artifact_count".to_string();
    }
    if contains_any(&[
        "postcondition",
        "contract",
        "verify",
        "verification",
        "check pass",
        "checks pass",
    ]) {
        return "postconditions_ok".to_string();
    }
    if contains_any(&[
        "receipt",
        "evidence",
        "queue outcome",
        "logged",
        "retry",
        "backoff",
        "abort",
        "aborted",
        "timeout",
        "rate limit",
        "throttle",
        "circuit breaker",
    ]) {
        return "queue_outcome_logged".to_string();
    }
    if text.contains("revenue") {
        return "revenue_actions_count".to_string();
    }
    if contains_any(&["entries", "entry", "notes"]) {
        return "entries_count".to_string();
    }
    if contains_any(&["token", "tokens"]) {
        return "token_usage".to_string();
    }
    if contains_any(&[
        "latency",
        "duration",
        "time",
        " ms",
        "msec",
        "millisecond",
        "second",
        " sec",
        " min",
        "minute",
    ]) {
        return "duration_ms".to_string();
    }
    if contains_any(&[
        "execute",
        "executed",
        "execution",
        "run",
        "runnable",
        "success",
    ]) {
        return "execution_success".to_string();
    }
    "execution_success".to_string()
}

fn compile_success_criteria_rows(
    rows: Option<&Value>,
    source: &str,
) -> Vec<SuccessCriteriaCompiledRow> {
    let mut out = Vec::<SuccessCriteriaCompiledRow>::new();
    let source = normalize_text(source);
    let src = if source.is_empty() {
        "success_criteria".to_string()
    } else {
        source
    };
    let Some(Value::Array(entries)) = rows else {
        return out;
    };

    let mut seen = BTreeSet::<String>::new();
    for row in entries {
        let (metric_raw, target_raw, horizon_raw) = if let Some(raw) = row.as_str() {
            (String::new(), normalize_spaces(raw), String::new())
        } else if let Some(obj) = row.as_object() {
            (
                normalize_spaces(&value_to_string(obj.get("metric")).to_ascii_lowercase()),
                normalize_spaces(
                    &[
                        value_to_string(obj.get("target")),
                        value_to_string(obj.get("threshold")),
                        value_to_string(obj.get("description")),
                        value_to_string(obj.get("goal")),
                    ]
                    .into_iter()
                    .find(|value| !value.is_empty())
                    .unwrap_or_default(),
                ),
                normalize_spaces(
                    &[
                        value_to_string(obj.get("horizon")),
                        value_to_string(obj.get("window")),
                        value_to_string(obj.get("by")),
                    ]
                    .into_iter()
                    .find(|value| !value.is_empty())
                    .unwrap_or_default(),
                ),
            )
        } else {
            continue;
        };

        if metric_raw.is_empty() && target_raw.is_empty() && horizon_raw.is_empty() {
            continue;
        }
        let metric = classify_metric(&metric_raw, &target_raw, &src);
        let horizon = if horizon_raw.is_empty() {
            parse_horizon(&target_raw)
        } else {
            horizon_raw
        };
        let target = normalize_target(&metric, &target_raw, &horizon);
        let key = format!("{}|{}|{}|{}", src, metric, target, horizon).to_ascii_lowercase();
        if seen.insert(key) {
            out.push(SuccessCriteriaCompiledRow {
                source: src.clone(),
                metric,
                target,
            });
        }
    }
    out
}

fn compile_proposal_success_criteria(
    proposal: Option<&Value>,
    capability_key: &str,
) -> Vec<SuccessCriteriaCompiledRow> {
    let proposal = proposal.and_then(Value::as_object);
    let action_spec = proposal
        .and_then(|obj| obj.get("action_spec"))
        .and_then(Value::as_object);

    let mut compiled = Vec::<SuccessCriteriaCompiledRow>::new();
    compiled.extend(compile_success_criteria_rows(
        proposal.and_then(|obj| obj.get("success_criteria")),
        "success_criteria",
    ));
    compiled.extend(compile_success_criteria_rows(
        action_spec.and_then(|obj| obj.get("success_criteria")),
        "action_spec.success_criteria",
    ));
    compiled.extend(compile_success_criteria_rows(
        action_spec.and_then(|obj| obj.get("verify")),
        "action_spec.verify",
    ));
    compiled.extend(compile_success_criteria_rows(
        proposal.and_then(|obj| obj.get("validation")),
        "validation",
    ));

    if compiled.is_empty() {
        compiled.push(SuccessCriteriaCompiledRow {
            source: "compiler_fallback".to_string(),
            metric: "execution_success".to_string(),
            target: "execution success".to_string(),
        });
    }

    let mut out = Vec::<SuccessCriteriaCompiledRow>::new();
    let mut seen = BTreeSet::<String>::new();
    for row in compiled {
        let metric = remap_metric_for_capability(&row.metric, capability_key);
        let target = normalize_target(&metric, &row.target, "");
        let source = if row.source.trim().is_empty() {
            "success_criteria".to_string()
        } else {
            normalize_spaces(&row.source)
        };
        let key = format!("{}|{}|{}", source, metric, target).to_ascii_lowercase();
        if seen.insert(key) {
            out.push(SuccessCriteriaCompiledRow {
                source,
                metric,
                target,
            });
        }
    }
    out
}

pub fn parse_success_criteria_rows_from_proposal(
    proposal: Option<&Value>,
    capability_key: &str,
) -> Vec<SuccessCriteriaCompiledRow> {
    let compiled = compile_proposal_success_criteria(proposal, capability_key);
    let mut out = Vec::<SuccessCriteriaCompiledRow>::new();
    let mut seen = BTreeSet::<String>::new();
    for row in compiled {
        let metric = normalize_spaces(&row.metric).to_ascii_lowercase();
        let target = normalize_spaces(&row.target);
        if metric.is_empty() && target.is_empty() {
            continue;
        }
        let key = format!("{}|{}", metric, target).to_ascii_lowercase();
        if seen.insert(key) {
            out.push(SuccessCriteriaCompiledRow {
                source: if row.source.trim().is_empty() {
                    "compiled".to_string()
                } else {
                    row.source
                },
                metric: if metric.is_empty() {
                    "execution_success".to_string()
                } else {
                    metric
                },
                target: if target.is_empty() {
                    "execution success".to_string()
                } else {
                    target
                },
            });
        }
    }
    out
}

fn capability_metric_contract(capability_key: &str) -> CapabilityMetricContract {
    let key = normalize_capability_key(capability_key);
    if key.is_empty() {
        return CapabilityMetricContract {
            capability_key: None,
            enforced: false,
            allowed_metrics: None,
        };
    }
    if key.starts_with("actuation:") {
        return CapabilityMetricContract {
            capability_key: Some(key),
            enforced: true,
            allowed_metrics: Some(ALL_KNOWN_METRICS.iter().map(|v| v.to_string()).collect()),
        };
    }
    if key.starts_with("proposal:") {
        let mut allowed: HashSet<String> = PROPOSAL_BASE_METRICS
            .iter()
            .map(|v| v.to_string())
            .collect();
        if capability_allows_outreach(&key) {
            for metric in OUTREACH_METRICS {
                allowed.insert((*metric).to_string());
            }
        }
        return CapabilityMetricContract {
            capability_key: Some(key),
            enforced: true,
            allowed_metrics: Some(allowed),
        };
    }
    CapabilityMetricContract {
        capability_key: Some(key),
        enforced: true,
        allowed_metrics: Some(ALL_KNOWN_METRICS.iter().map(|v| v.to_string()).collect()),
    }
}

fn metric_allowed_by_contract(contract: &CapabilityMetricContract, metric_name: &str) -> bool {
    let Some(allowed) = contract.allowed_metrics.as_ref() else {
        return false;
    };
    let norm = metric_name.to_ascii_lowercase().replace([' ', '-'], "_");
    !norm.is_empty() && allowed.contains(&norm)
}

fn backfill_contract_safe_rows(
    rows: &[SuccessCriteriaCompiledRow],
    contract: &CapabilityMetricContract,
    min_count: i64,
) -> (Vec<SuccessCriteriaCompiledRow>, i64) {
    let mut out = rows.to_vec();
    if min_count <= 0 || !contract.enforced || contract.allowed_metrics.is_none() {
        return (out, 0);
    }
    let mut seen = BTreeSet::<String>::new();
    for row in &out {
        seen.insert(format!(
            "{}|{}",
            row.metric.to_ascii_lowercase().replace([' ', '-'], "_"),
            row.target.to_ascii_lowercase()
        ));
    }
    let mut supported_count = out
        .iter()
        .filter(|row| metric_allowed_by_contract(contract, &row.metric))
        .count() as i64;
    let mut added = 0i64;
    for (source, metric, target) in CONTRACT_SAFE_BACKFILL_ROWS {
        if supported_count >= min_count {
            break;
        }
        if !metric_allowed_by_contract(contract, metric) {
            continue;
        }
        let key = format!(
            "{}|{}",
            metric.to_ascii_lowercase().replace([' ', '-'], "_"),
            target.to_ascii_lowercase()
        );
        if seen.insert(key) {
            out.push(SuccessCriteriaCompiledRow {
                source: (*source).to_string(),
                metric: (*metric).to_string(),
                target: (*target).to_string(),
            });
            supported_count += 1;
            added += 1;
        }
    }
    (out, added)
}

fn as_f64(value: Option<&Value>) -> Option<f64> {
    match value {
        Some(Value::Number(n)) => n.as_f64(),
        Some(Value::String(v)) => v.trim().parse::<f64>().ok(),
        _ => None,
    }
}

fn compare_numeric(value: Option<f64>, threshold: Option<f64>, comparator: &str) -> Option<bool> {
    let value = value?;
    let threshold = threshold?;
    if comparator == "gte" {
        Some(value >= threshold)
    } else {
        Some(value <= threshold)
    }
}

fn bool_verdict(reason: &str, pass: bool, value: Value, target: Value) -> EvaluationVerdict {
    EvaluationVerdict {
        evaluated: true,
        pass: Some(pass),
        reason: reason.to_string(),
        comparator: None,
        value: Some(value),
        target: Some(target),
        unit: None,
    }
}

fn read_numeric_metric(context: &Value, keys: &[&str]) -> Option<f64> {
    let top = context.as_object()?;
    let metric_values = top.get("metric_values").and_then(Value::as_object);
    let dod_diff = top.get("dod_diff").and_then(Value::as_object);
    for key in keys {
        if let Some(value) = metric_values
            .and_then(|map| map.get(*key))
            .and_then(|v| as_f64(Some(v)))
        {
            return Some(value);
        }
        if let Some(value) = top.get(*key).and_then(|v| as_f64(Some(v))) {
            return Some(value);
        }
        if let Some(value) = dod_diff
            .and_then(|map| map.get(*key))
            .and_then(|v| as_f64(Some(v)))
        {
            return Some(value);
        }
    }
    None
}
