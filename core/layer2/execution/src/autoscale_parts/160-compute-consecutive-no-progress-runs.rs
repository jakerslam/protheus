pub fn compute_consecutive_no_progress_runs(
    input: &ConsecutiveNoProgressRunsInput,
) -> ConsecutiveNoProgressRunsOutput {
    let mut count: u32 = 0;
    for evt in input.events.iter().rev() {
        let event_type = evt
            .event_type
            .as_ref()
            .map(|v| v.trim().to_ascii_lowercase())
            .unwrap_or_default();
        if event_type != "autonomy_run" {
            continue;
        }

        let result = evt
            .result
            .as_ref()
            .map(|v| v.trim().to_string())
            .unwrap_or_default();
        let outcome = evt
            .outcome
            .as_ref()
            .map(|v| v.trim().to_string())
            .unwrap_or_default();
        if result == "executed" && outcome == "shipped" {
            break;
        }
        let is_no_progress = compute_no_progress_result(&NoProgressResultInput {
            event_type: Some("autonomy_run".to_string()),
            result: Some(result),
            outcome: Some(outcome),
        })
        .is_no_progress;
        if !is_no_progress {
            break;
        }
        count += 1;
    }
    ConsecutiveNoProgressRunsOutput { count }
}

pub fn compute_shipped_count(input: &ShippedCountInput) -> ShippedCountOutput {
    let mut count: u32 = 0;
    for evt in &input.events {
        let event_type = evt
            .event_type
            .as_ref()
            .map(|v| v.trim().to_ascii_lowercase())
            .unwrap_or_default();
        let result = evt
            .result
            .as_ref()
            .map(|v| v.trim().to_ascii_lowercase())
            .unwrap_or_default();
        let outcome = evt
            .outcome
            .as_ref()
            .map(|v| v.trim().to_ascii_lowercase())
            .unwrap_or_default();
        if event_type == "autonomy_run" && result == "executed" && outcome == "shipped" {
            count += 1;
        }
    }
    ShippedCountOutput { count }
}

fn normalize_risk_level(raw: &str) -> String {
    let level = raw.trim().to_ascii_lowercase();
    if level == "high" || level == "medium" || level == "low" {
        level
    } else {
        "low".to_string()
    }
}

pub fn compute_executed_count_by_risk(
    input: &ExecutedCountByRiskInput,
) -> ExecutedCountByRiskOutput {
    let target = normalize_risk_level(input.risk.as_deref().unwrap_or(""));
    let mut count: u32 = 0;
    for evt in &input.events {
        let event_type = evt
            .event_type
            .as_ref()
            .map(|v| v.trim().to_ascii_lowercase())
            .unwrap_or_default();
        if event_type != "autonomy_run" {
            continue;
        }
        let result = evt
            .result
            .as_ref()
            .map(|v| v.trim().to_ascii_lowercase())
            .unwrap_or_default();
        if result != "executed" {
            continue;
        }
        let run_risk = if let Some(risk) = evt.risk.as_ref() {
            normalize_risk_level(risk)
        } else {
            normalize_risk_level(evt.proposal_risk.as_deref().unwrap_or(""))
        };
        if run_risk == target {
            count += 1;
        }
    }
    ExecutedCountByRiskOutput { count }
}

pub fn compute_run_result_tally(input: &RunResultTallyInput) -> RunResultTallyOutput {
    let mut counts: std::collections::BTreeMap<String, u32> = std::collections::BTreeMap::new();
    for evt in &input.events {
        let event_type = evt
            .event_type
            .as_ref()
            .map(|v| v.trim().to_ascii_lowercase())
            .unwrap_or_default();
        if event_type != "autonomy_run" {
            continue;
        }
        let key = evt
            .result
            .as_ref()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| "unknown".to_string());
        let next = counts.get(&key).copied().unwrap_or(0).saturating_add(1);
        counts.insert(key, next);
    }
    RunResultTallyOutput { counts }
}

pub fn compute_sorted_counts(input: &SortedCountsInput) -> SortedCountsOutput {
    let mut items = input
        .counts
        .iter()
        .map(|(result, count)| SortedCountItem {
            result: result.to_string(),
            count: if count.is_finite() && *count > 0.0 {
                count.round() as u32
            } else {
                0
            },
        })
        .collect::<Vec<_>>();
    items.sort_by(|a, b| {
        if b.count != a.count {
            b.count.cmp(&a.count)
        } else {
            a.result.cmp(&b.result)
        }
    });
    SortedCountsOutput { items }
}

pub fn compute_normalize_proposal_status(
    input: &NormalizeProposalStatusInput,
) -> NormalizeProposalStatusOutput {
    let base = input
        .fallback
        .as_ref()
        .map(|v| v.trim().to_ascii_lowercase())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "pending".to_string());
    let status = input
        .raw_status
        .as_ref()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_default();
    let normalized_status = if status.is_empty()
        || status == "unknown"
        || status == "new"
        || status == "queued"
        || status == "open"
        || status == "admitted"
    {
        base
    } else if status == "closed_won" || status == "won" || status == "paid" || status == "verified"
    {
        "closed".to_string()
    } else {
        status
    };
    NormalizeProposalStatusOutput { normalized_status }
}

pub fn compute_proposal_status(input: &ProposalStatusInput) -> ProposalStatusOutput {
    let overlay_decision = input
        .overlay_decision
        .as_ref()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_default();
    let status = if overlay_decision == "accept" {
        "accepted".to_string()
    } else if overlay_decision == "reject" {
        "rejected".to_string()
    } else if overlay_decision == "park" {
        "parked".to_string()
    } else {
        "pending".to_string()
    };
    ProposalStatusOutput { status }
}

pub fn compute_proposal_outcome_status(
    input: &ProposalOutcomeStatusInput,
) -> ProposalOutcomeStatusOutput {
    let outcome = input
        .overlay_outcome
        .as_ref()
        .map(|v| v.trim().to_ascii_lowercase())
        .filter(|v| !v.is_empty());
    ProposalOutcomeStatusOutput { outcome }
}

pub fn compute_queue_underflow_backfill(
    input: &QueueUnderflowBackfillInput,
) -> QueueUnderflowBackfillOutput {
    if input.underflow_backfill_max <= 0.0 {
        return QueueUnderflowBackfillOutput { allow: false };
    }
    let status = input
        .status
        .as_ref()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_default();
    if status != "accepted" {
        return QueueUnderflowBackfillOutput { allow: false };
    }
    let out = compute_proposal_outcome_status(&ProposalOutcomeStatusInput {
        overlay_outcome: input.overlay_outcome.clone(),
    })
    .outcome;
    QueueUnderflowBackfillOutput {
        allow: out.is_none(),
    }
}

pub fn compute_proposal_risk_score(input: &ProposalRiskScoreInput) -> ProposalRiskScoreOutput {
    if let Some(explicit) = input.explicit_risk_score {
        if explicit.is_finite() {
            let rounded = explicit.round();
            if rounded <= 0.0 {
                return ProposalRiskScoreOutput { risk_score: 0 };
            }
            if rounded >= 100.0 {
                return ProposalRiskScoreOutput { risk_score: 100 };
            }
            return ProposalRiskScoreOutput {
                risk_score: rounded as u32,
            };
        }
    }
    let risk = normalize_risk_level(input.risk.as_deref().unwrap_or(""));
    let risk_score = if risk == "high" {
        90
    } else if risk == "medium" {
        60
    } else {
        25
    };
    ProposalRiskScoreOutput { risk_score }
}

pub fn compute_proposal_score(input: &ProposalScoreInput) -> ProposalScoreOutput {
    let age_penalty = (input.age_hours / 24.0) * 0.6;
    let stub_penalty = if input.is_stub { 2.5 } else { 0.0 };
    let no_change_penalty = input.no_change_count * 1.5;
    let reverted_penalty = input.reverted_count * 3.0;
    ProposalScoreOutput {
        score: (input.impact_weight * 2.0)
            - (input.risk_penalty * 1.0)
            - age_penalty
            - stub_penalty
            - no_change_penalty
            - reverted_penalty,
    }
}

pub fn compute_proposal_admission_preview(
    input: &ProposalAdmissionPreviewInput,
) -> ProposalAdmissionPreviewOutput {
    let preview = input
        .admission_preview
        .as_ref()
        .filter(|v| v.is_object() || v.is_array())
        .cloned();
    ProposalAdmissionPreviewOutput { preview }
}

pub fn compute_impact_weight(input: &ImpactWeightInput) -> ImpactWeightOutput {
    let impact = input
        .expected_impact
        .as_ref()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_default();
    let weight = if impact == "high" {
        3
    } else if impact == "medium" {
        2
    } else {
        1
    };
    ImpactWeightOutput { weight }
}

pub fn compute_risk_penalty(input: &RiskPenaltyInput) -> RiskPenaltyOutput {
    let risk = input
        .risk
        .as_ref()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_default();
    let penalty = if risk == "high" {
        2
    } else if risk == "medium" {
        1
    } else {
        0
    };
    RiskPenaltyOutput { penalty }
}

pub fn compute_estimate_tokens(input: &EstimateTokensInput) -> EstimateTokensOutput {
    let impact = input
        .expected_impact
        .as_ref()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_default();
    let est_tokens = if impact == "high" {
        1400
    } else if impact == "medium" {
        800
    } else {
        300
    };
    EstimateTokensOutput { est_tokens }
}

pub fn compute_proposal_remediation_depth(
    input: &ProposalRemediationDepthInput,
) -> ProposalRemediationDepthOutput {
    if let Some(raw) = input.remediation_depth {
        if raw.is_finite() && raw >= 0.0 {
            return ProposalRemediationDepthOutput {
                depth: raw.round().clamp(0.0, u32::MAX as f64) as u32,
            };
        }
    }
    let trigger = input
        .trigger
        .as_deref()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    let depth = if trigger == "consecutive_failures" || trigger == "multi_eye_transport_failure" {
        1
    } else {
        0
    };
    ProposalRemediationDepthOutput { depth }
}

pub fn compute_proposal_dedup_key(input: &ProposalDedupKeyInput) -> ProposalDedupKeyOutput {
    let proposal_type = input
        .proposal_type
        .as_deref()
        .unwrap_or("unknown")
        .trim()
        .to_ascii_lowercase();
    let proposal_type = if proposal_type.is_empty() {
        "unknown".to_string()
    } else {
        proposal_type
    };
    let source_eye_id = input.source_eye_id.as_deref().unwrap_or("").trim();
    let remediation_kind = input
        .remediation_kind
        .as_deref()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    let proposal_id = input.proposal_id.as_deref().unwrap_or("unknown").trim();
    let dedup_key = if proposal_type.contains("remediation") {
        format!(
            "{}|{}|{}",
            proposal_type,
            source_eye_id,
            if remediation_kind.is_empty() {
                "none"
            } else {
                remediation_kind.as_str()
            }
        )
    } else {
        format!(
            "{}|{}|{}",
            proposal_type,
            source_eye_id,
            if proposal_id.is_empty() {
                "unknown"
            } else {
                proposal_id
            }
        )
    };
    ProposalDedupKeyOutput { dedup_key }
}
