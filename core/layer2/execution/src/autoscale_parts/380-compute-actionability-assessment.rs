pub fn compute_actionability_assessment(
    input: &ActionabilityAssessmentInput,
) -> ActionabilityAssessmentOutput {
    let risk = input
        .risk
        .as_ref()
        .map(|value| value.trim().to_ascii_lowercase())
        .unwrap_or_default();
    let risk = match risk.as_str() {
        "med" | "moderate" => "medium".to_string(),
        "critical" | "severe" => "high".to_string(),
        "low" | "medium" | "high" => risk,
        _ => "low".to_string(),
    };
    let impact = input
        .impact
        .as_ref()
        .map(|value| value.trim().to_ascii_lowercase())
        .unwrap_or_default();
    let impact = match impact.as_str() {
        "critical" | "urgent" => "critical".to_string(),
        "high" | "medium" | "low" => impact,
        _ => "low".to_string(),
    };

    let mut reasons = Vec::<String>::new();
    let mut score = 0.0;
    let mut hard_block = false;

    if impact == "critical" {
        score += 28.0;
    } else if impact == "high" {
        score += 24.0;
    } else if impact == "medium" {
        score += 16.0;
    } else {
        score += 8.0;
    }

    if input.specific_validation_count >= 3.0 {
        score += 18.0;
    } else if input.specific_validation_count >= 2.0 {
        score += 12.0;
    } else if input.specific_validation_count >= 1.0 {
        score += 6.0;
    } else if input.validation_count > 0.0 {
        reasons.push("generic_validation_template".to_string());
    } else {
        reasons.push("missing_validation_plan".to_string());
    }

    if input.has_next_cmd {
        if input.generic_route_task {
            score += 4.0;
            reasons.push("generic_next_command_template".to_string());
        } else {
            score += 8.0;
            if !input.next_cmd_has_dry_run {
                score += 4.0;
            } else {
                score += 2.0;
            }
        }
    } else {
        reasons.push("missing_next_command".to_string());
    }

    if input.looks_like_discovery_cmd {
        score -= 18.0;
        reasons.push("discovery_only_command".to_string());
    }

    if input.has_action_verb {
        score += 12.0;
    } else {
        reasons.push("no_action_verb".to_string());
    }

    if input.has_opportunity {
        score += 10.0;
    }

    if let Some(relevance) = input.relevance_score {
        if relevance.is_finite() {
            score += (relevance - 45.0) * 0.3;
        }
    }
    if let Some(fit_score) = input.directive_fit_score {
        if fit_score.is_finite() {
            score += (fit_score - 35.0) * 0.25;
        }
    }

    if input.criteria_requirement_applied {
        if input.measurable_criteria_count >= input.criteria_min_count {
            score += (8.0 + (input.measurable_criteria_count * 2.0)).min(14.0);
        } else {
            score -= 22.0;
            reasons.push("success_criteria_missing".to_string());
            hard_block = true;
        }
    } else if input.measurable_criteria_count > 0.0 {
        score += (input.measurable_criteria_count * 2.0).min(8.0);
    }

    if !input.has_action_verb && !input.has_opportunity && !input.has_concrete_target {
        score -= 20.0;
        reasons.push("missing_concrete_target".to_string());
    }
    if input.is_meta_coordination && !input.has_concrete_target {
        score -= 26.0;
        reasons.push("meta_coordination_without_concrete_target".to_string());
    }
    if input.mentions_proposal && !input.has_concrete_target && !input.has_opportunity {
        score -= 12.0;
        reasons.push("proposal_recursion_without_target".to_string());
    }
    if input.is_explainer && !input.has_action_verb && !input.has_opportunity {
        score -= 12.0;
        reasons.push("explainer_without_execution_path".to_string());
    }
    if input.generic_route_task
        && input.specific_validation_count <= 0.0
        && !input.has_opportunity
        && !input.has_concrete_target
    {
        score -= 18.0;
        reasons.push("boilerplate_execution_path".to_string());
    }
    if input.generic_route_task && !input.next_cmd_has_dry_run {
        score -= 8.0;
        reasons.push("generic_path_missing_dry_run".to_string());
    }

    if input.looks_like_discovery_cmd && impact == "low" && !input.has_action_verb {
        hard_block = true;
        reasons.push("non_actionable_discovery_item".to_string());
    }
    if input.is_meta_coordination
        && !input.has_concrete_target
        && impact == "low"
        && !input.has_opportunity
    {
        hard_block = true;
        reasons.push("non_actionable_meta_item".to_string());
    }

    if input.criteria_pattern_penalty > 0.0 {
        score -= input.criteria_pattern_penalty;
        reasons.push("criteria_pattern_penalty".to_string());
    }

    if risk == "medium" && input.is_executable_proposal && !input.has_rollback_signal {
        score -= 28.0;
        reasons.push("medium_risk_missing_rollback_path".to_string());
        hard_block = true;
    }
    if risk == "high" && input.is_executable_proposal {
        if !input.has_rollback_signal {
            score -= 34.0;
            reasons.push("high_risk_missing_rollback_path".to_string());
            hard_block = true;
        }
        if !input.next_cmd_has_dry_run {
            score -= 12.0;
            reasons.push("high_risk_missing_dry_run".to_string());
            hard_block = true;
        }
    }

    if input.subdirective_required {
        if !input.subdirective_has_concrete_target {
            score -= 18.0;
            reasons.push("subdirective_v2_missing_target".to_string());
            hard_block = true;
        }
        if !input.subdirective_has_expected_delta {
            score -= 20.0;
            reasons.push("subdirective_v2_missing_expected_delta".to_string());
            hard_block = true;
        }
        if !input.subdirective_has_verification_step {
            score -= 20.0;
            reasons.push("subdirective_v2_missing_verification_step".to_string());
            hard_block = true;
        }
    }

    let final_score = score.round().clamp(0.0, 100.0);
    let pass = !hard_block && final_score >= input.min_actionability;
    if !pass && final_score < input.min_actionability {
        reasons.push("below_min_actionability".to_string());
    }

    ActionabilityAssessmentOutput {
        pass,
        score: final_score,
        reasons,
        executable: input.is_executable_proposal,
        rollback_signal: input.has_rollback_signal,
        generic_next_command_template: input.generic_route_task,
        subdirective_v2: serde_json::json!({
            "required": input.subdirective_required,
            "has_concrete_target": input.subdirective_has_concrete_target,
            "has_expected_delta": input.subdirective_has_expected_delta,
            "has_verification_step": input.subdirective_has_verification_step,
            "target_count": input.subdirective_target_count,
            "verify_count": input.subdirective_verify_count,
            "success_criteria_count": input.subdirective_success_criteria_count
        }),
        success_criteria: serde_json::json!({
            "required": input.criteria_requirement_applied,
            "exempt_type": input.criteria_exempt_type,
            "min_count": input.criteria_min_count,
            "measurable_count": input.measurable_criteria_count,
            "total_count": input.criteria_total_count,
            "pattern_penalty": input.criteria_pattern_penalty,
            "pattern_hits": input.criteria_pattern_hits.clone().unwrap_or_else(|| serde_json::json!([]))
        }),
    }
}

fn autoscale_row_id(value: &serde_json::Value) -> String {
    value
        .as_object()
        .and_then(|obj| obj.get("id"))
        .map(js_like_string)
        .map(|v| v.trim().to_string())
        .unwrap_or_default()
}

fn autoscale_non_negative(value: f64) -> f64 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

pub fn compute_strategy_profile(input: &StrategyProfileInput) -> StrategyProfileOutput {
    let strategy = input
        .strategy
        .as_ref()
        .filter(|value| value.is_object())
        .cloned();
    StrategyProfileOutput { strategy }
}

pub fn compute_active_strategy_variants(
    input: &ActiveStrategyVariantsInput,
) -> ActiveStrategyVariantsOutput {
    let mut out: Vec<serde_json::Value> = Vec::new();
    let mut seen = std::collections::BTreeSet::<String>::new();

    for row in &input.listed {
        let Some(obj) = row.as_object() else {
            continue;
        };
        let status = obj
            .get("status")
            .map(js_like_string)
            .map(|v| v.trim().to_ascii_lowercase())
            .unwrap_or_default();
        if status != "active" {
            continue;
        }
        let strict_not_ok = obj
            .get("validation")
            .and_then(|v| v.as_object())
            .and_then(|v| v.get("strict_ok"))
            .and_then(|v| v.as_bool())
            == Some(false);
        if strict_not_ok {
            continue;
        }
        let id = autoscale_row_id(row);
        if id.is_empty() || !seen.insert(id) {
            continue;
        }
        out.push(serde_json::Value::Object(obj.clone()));
    }

    if let Some(primary) = input.primary.as_ref() {
        let id = autoscale_row_id(primary);
        if !id.is_empty() && !seen.contains(&id) && primary.is_object() {
            out.push(primary.clone());
        }
    }

    out.sort_by_key(autoscale_row_id);
    ActiveStrategyVariantsOutput { variants: out }
}

pub fn compute_strategy_scorecard_summaries(
    input: &StrategyScorecardSummariesInput,
) -> StrategyScorecardSummariesOutput {
    let mut by_id = std::collections::BTreeMap::<String, StrategyScorecardSummaryItemOutput>::new();
    for row in &input.summaries {
        let Some(obj) = row.as_object() else {
            continue;
        };
        let id = obj
            .get("strategy_id")
            .map(js_like_string)
            .map(|v| v.trim().to_string())
            .unwrap_or_default();
        if id.is_empty() {
            continue;
        }
        let metrics = obj.get("metrics").and_then(|v| v.as_object());
        let score = metrics
            .and_then(|v| v.get("score"))
            .and_then(|v| js_like_number(Some(v)))
            .unwrap_or(0.0);
        let confidence = metrics
            .and_then(|v| v.get("confidence"))
            .and_then(|v| js_like_number(Some(v)))
            .unwrap_or(0.0);
        let stage = obj
            .get("stage")
            .map(js_like_string)
            .map(|v| v.trim().to_ascii_lowercase())
            .filter(|v| !v.is_empty());
        by_id.insert(
            id,
            StrategyScorecardSummaryItemOutput {
                score,
                confidence,
                stage,
            },
        );
    }

    StrategyScorecardSummariesOutput {
        path: input
            .path
            .as_ref()
            .map(|v| v.trim().to_string())
            .unwrap_or_default(),
        ts: input
            .ts
            .as_ref()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty()),
        by_id,
    }
}

pub fn compute_outcome_fitness_policy(
    input: &OutcomeFitnessPolicyInput,
) -> OutcomeFitnessPolicyOutput {
    let policy = input
        .policy
        .as_ref()
        .filter(|value| value.is_object())
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    OutcomeFitnessPolicyOutput { policy }
}

pub fn compute_load_eyes_map(input: &LoadEyesMapInput) -> LoadEyesMapOutput {
    let mut rows: Vec<serde_json::Map<String, serde_json::Value>> = Vec::new();
    let mut idx_by_id = std::collections::HashMap::<String, usize>::new();

    for row in &input.cfg_eyes {
        let Some(obj) = row.as_object() else {
            continue;
        };
        let id = row
            .as_object()
            .and_then(|m| m.get("id"))
            .map(js_like_string)
            .map(|v| v.trim().to_string())
            .unwrap_or_default();
        if id.is_empty() {
            continue;
        }
        if let Some(index) = idx_by_id.get(&id).copied() {
            rows[index] = obj.clone();
        } else {
            idx_by_id.insert(id, rows.len());
            rows.push(obj.clone());
        }
    }

    for row in &input.state_eyes {
        let Some(obj) = row.as_object() else {
            continue;
        };
        let id = row
            .as_object()
            .and_then(|m| m.get("id"))
            .map(js_like_string)
            .map(|v| v.trim().to_string())
            .unwrap_or_default();
        if id.is_empty() {
            continue;
        }
        if let Some(index) = idx_by_id.get(&id).copied() {
            let merged = &mut rows[index];
            for (key, value) in obj {
                merged.insert(key.clone(), value.clone());
            }
        } else {
            idx_by_id.insert(id, rows.len());
            rows.push(obj.clone());
        }
    }

    LoadEyesMapOutput {
        eyes: rows
            .into_iter()
            .map(serde_json::Value::Object)
            .collect::<Vec<_>>(),
    }
}

pub fn compute_fallback_directive_objective_ids(
    input: &FallbackDirectiveObjectiveIdsInput,
) -> FallbackDirectiveObjectiveIdsOutput {
    let mut ids = std::collections::BTreeSet::<String>::new();
    for raw in &input.directive_ids {
        let id = sanitize_directive_objective_id(raw);
        if !id.is_empty() {
            ids.insert(id);
        }
    }
    FallbackDirectiveObjectiveIdsOutput {
        ids: ids.into_iter().collect(),
    }
}
