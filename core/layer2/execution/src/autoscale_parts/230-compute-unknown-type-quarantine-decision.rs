fn normalize_quarantine_token(raw: &str) -> String {
    raw.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
        .replace('-', "_")
}

fn canonical_proposal_type(raw: &str) -> String {
    let token = normalize_quarantine_token(raw);
    match token.as_str() {
        "directive clarification" => "directive_clarification".to_string(),
        "directive decomposition" => "directive_decomposition".to_string(),
        "actuation optimization" => "actuation_optimization".to_string(),
        _ => token.replace(' ', "_"),
    }
}

pub fn compute_unknown_type_quarantine_decision(
    input: &UnknownTypeQuarantineDecisionInput,
) -> UnknownTypeQuarantineDecisionOutput {
    let proposal_type = input
        .proposal_type
        .as_ref()
        .map(|v| canonical_proposal_type(v))
        .filter(|v| !v.is_empty());
    if !input.enabled {
        return UnknownTypeQuarantineDecisionOutput {
            block: false,
            proposal_type,
            reason: None,
            objective_id: None,
        };
    }
    if !input.type_in_quarantine_set {
        return UnknownTypeQuarantineDecisionOutput {
            block: false,
            proposal_type,
            reason: None,
            objective_id: None,
        };
    }
    let is_directive = matches!(
        proposal_type.as_deref(),
        Some("directive_clarification") | Some("directive_decomposition")
    );
    if input.allow_directive && is_directive {
        return UnknownTypeQuarantineDecisionOutput {
            block: false,
            proposal_type,
            reason: Some("directive_exempt".to_string()),
            objective_id: None,
        };
    }
    let objective_id = input
        .objective_id
        .as_ref()
        .map(|v| sanitize_directive_objective_id(v))
        .filter(|v| !v.is_empty());
    if input.allow_tier1 && input.tier1_objective {
        return UnknownTypeQuarantineDecisionOutput {
            block: false,
            proposal_type,
            reason: Some("tier1_objective_exempt".to_string()),
            objective_id,
        };
    }
    UnknownTypeQuarantineDecisionOutput {
        block: true,
        proposal_type,
        reason: Some("unknown_type_quarantine".to_string()),
        objective_id,
    }
}

pub fn compute_infer_optimization_delta(
    input: &InferOptimizationDeltaInput,
) -> InferOptimizationDeltaOutput {
    let direct_keys = [
        (
            input.optimization_delta_percent,
            "meta:optimization_delta_percent",
        ),
        (
            input.expected_optimization_percent,
            "meta:expected_optimization_percent",
        ),
        (input.expected_delta_percent, "meta:expected_delta_percent"),
        (
            input.estimated_improvement_percent,
            "meta:estimated_improvement_percent",
        ),
        (
            input.target_improvement_percent,
            "meta:target_improvement_percent",
        ),
        (
            input.performance_gain_percent,
            "meta:performance_gain_percent",
        ),
    ];
    for (value, source) in direct_keys {
        let Some(raw) = value else {
            continue;
        };
        if raw.is_finite() && raw > 0.0 {
            return InferOptimizationDeltaOutput {
                delta_percent: Some(round3(raw.clamp(0.0, 100.0))),
                delta_source: Some(source.to_string()),
            };
        }
    }
    let values = compute_percent_mentions_from_text(&PercentMentionsFromTextInput {
        text: input.text_blob.clone(),
    })
    .values;
    if values.is_empty() {
        return InferOptimizationDeltaOutput {
            delta_percent: None,
            delta_source: None,
        };
    }
    let max_val = values
        .into_iter()
        .fold(0.0_f64, |acc, v| if v > acc { v } else { acc });
    InferOptimizationDeltaOutput {
        delta_percent: Some(round3(max_val)),
        delta_source: Some("text:%".to_string()),
    }
}

pub fn compute_optimization_intent_proposal(
    input: &OptimizationIntentProposalInput,
) -> OptimizationIntentProposalOutput {
    let proposal_type = input
        .proposal_type
        .as_ref()
        .map(|v| canonical_proposal_type(v))
        .unwrap_or_default();
    let blob = input.blob.as_deref().unwrap_or("");
    let canary_smoke_re = Regex::new(r"(?i)\bcanary\b|\bsmoke\s*test\b").expect("valid regex");
    let type_is_actuation = proposal_type.starts_with("actuation_")
        || proposal_type == "actuation"
        || input.has_actuation_meta;
    if type_is_actuation && canary_smoke_re.is_match(blob) {
        return OptimizationIntentProposalOutput { intent: false };
    }
    let intent_re = Regex::new(
        r"(?i)\b(optimi[sz]e|optimization|improv(?:e|ement)|tune|polish|streamlin|efficien(?:cy|t)|latency|throughput|cost|token(?:s)?|performance)\b",
    )
    .expect("valid regex");
    let has_intent = intent_re.is_match(&proposal_type) || intent_re.is_match(blob);
    if !has_intent {
        return OptimizationIntentProposalOutput { intent: false };
    }
    let exempt_re = Regex::new(
        r"(?i)\b(fail(?:ure)?|error|outage|broken|incident|security|integrity|violation|breach|timeout|rate\s*limit|dns|connection|recover|restore|rollback|revert|remediation)\b",
    )
    .expect("valid regex");
    if exempt_re.is_match(&proposal_type) || exempt_re.is_match(blob) {
        return OptimizationIntentProposalOutput { intent: false };
    }
    let opportunity_re = Regex::new(
        r"(?i)\b(opportunity|freelance|job|jobs|hiring|contract|contractor|gig|client|rfp|request for proposal|seeking|looking for)\b",
    )
    .expect("valid regex");
    if opportunity_re.is_match(blob) {
        return OptimizationIntentProposalOutput { intent: false };
    }
    OptimizationIntentProposalOutput { intent: true }
}

pub fn compute_unlinked_optimization_admission(
    input: &UnlinkedOptimizationAdmissionInput,
) -> UnlinkedOptimizationAdmissionOutput {
    if !input.optimization_intent {
        return UnlinkedOptimizationAdmissionOutput {
            applies: false,
            linked: true,
            penalty: 0.0,
            block: false,
            reason: None,
        };
    }
    let proposal_type = input
        .proposal_type
        .as_ref()
        .map(|v| v.trim().to_lowercase())
        .unwrap_or_default();
    let exempt: std::collections::BTreeSet<String> = input
        .exempt_types
        .iter()
        .map(|v| v.trim().to_lowercase())
        .filter(|v| !v.is_empty())
        .collect();
    if !proposal_type.is_empty() && exempt.contains(&proposal_type) {
        return UnlinkedOptimizationAdmissionOutput {
            applies: true,
            linked: true,
            penalty: 0.0,
            block: false,
            reason: Some("optimization_exempt_type".to_string()),
        };
    }
    if input.linked {
        return UnlinkedOptimizationAdmissionOutput {
            applies: true,
            linked: true,
            penalty: 0.0,
            block: false,
            reason: None,
        };
    }
    let normalized_risk = input
        .normalized_risk
        .as_ref()
        .map(|v| v.trim().to_lowercase())
        .unwrap_or_else(|| "low".to_string());
    let high_risk_block = input.hard_block_high_risk && normalized_risk == "high";
    UnlinkedOptimizationAdmissionOutput {
        applies: true,
        linked: false,
        penalty: if input.penalty.is_finite() {
            input.penalty
        } else {
            0.0
        },
        block: high_risk_block,
        reason: Some(if high_risk_block {
            "optimization_unlinked_objective_high_risk_block".to_string()
        } else {
            "optimization_unlinked_objective_penalty".to_string()
        }),
    }
}

pub fn compute_optimization_good_enough(
    input: &OptimizationGoodEnoughInput,
) -> OptimizationGoodEnoughOutput {
    let mode = if input.high_accuracy_mode {
        "high_accuracy".to_string()
    } else {
        "default".to_string()
    };
    let risk = input
        .normalized_risk
        .as_ref()
        .map(|v| v.trim().to_lowercase())
        .filter(|v| v == "high" || v == "medium" || v == "low")
        .unwrap_or_else(|| "low".to_string());
    if !input.applies {
        return OptimizationGoodEnoughOutput {
            applies: false,
            pass: true,
            reason: None,
            delta_percent: None,
            delta_source: None,
            min_delta_percent: input.min_delta_percent,
            require_delta: input.require_delta,
            mode,
            risk,
        };
    }
    let delta_percent = input.delta_percent.filter(|v| v.is_finite());
    if delta_percent.is_none() && input.require_delta {
        return OptimizationGoodEnoughOutput {
            applies: true,
            pass: false,
            reason: Some("optimization_delta_missing".to_string()),
            delta_percent: None,
            delta_source: None,
            min_delta_percent: input.min_delta_percent,
            require_delta: true,
            mode,
            risk,
        };
    }
    if let Some(delta) = delta_percent {
        if delta < input.min_delta_percent {
            return OptimizationGoodEnoughOutput {
                applies: true,
                pass: false,
                reason: Some("optimization_good_enough".to_string()),
                delta_percent: Some(delta),
                delta_source: input.delta_source.clone(),
                min_delta_percent: input.min_delta_percent,
                require_delta: input.require_delta,
                mode,
                risk,
            };
        }
    }
    OptimizationGoodEnoughOutput {
        applies: true,
        pass: true,
        reason: None,
        delta_percent,
        delta_source: input.delta_source.clone(),
        min_delta_percent: input.min_delta_percent,
        require_delta: input.require_delta,
        mode,
        risk,
    }
}

pub fn compute_proposal_dependency_summary(
    input: &ProposalDependencySummaryInput,
) -> ProposalDependencySummaryOutput {
    let proposal_id = input
        .proposal_id
        .as_ref()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());
    let decision = input
        .decision
        .as_ref()
        .map(|v| {
            v.split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
                .to_uppercase()
        })
        .unwrap_or_default();
    let source = input
        .source
        .as_ref()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());
    let parent = input
        .parent_objective_id
        .as_ref()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());

    let mut child_ids = Vec::new();
    let mut seen = std::collections::BTreeSet::<String>::new();
    for raw in &input.created_ids {
        let id = raw.trim().to_string();
        if id.is_empty() || !seen.insert(id.clone()) {
            continue;
        }
        child_ids.push(id);
        if child_ids.len() >= 16 {
            break;
        }
    }

    let mut nodes = Vec::<ProposalDependencySummaryNode>::new();
    let mut edges = Vec::<ProposalDependencySummaryEdge>::new();
    if let Some(parent_id) = parent.clone() {
        nodes.push(ProposalDependencySummaryNode {
            id: parent_id.clone(),
            kind: "directive".to_string(),
            role: "parent".to_string(),
        });
        for child_id in &child_ids {
            nodes.push(ProposalDependencySummaryNode {
                id: child_id.clone(),
                kind: "directive".to_string(),
                role: "child".to_string(),
            });
            edges.push(ProposalDependencySummaryEdge {
                from: parent_id.clone(),
                to: child_id.clone(),
                relation: "parent_child".to_string(),
            });
        }
    } else {
        for child_id in &child_ids {
            nodes.push(ProposalDependencySummaryNode {
                id: child_id.clone(),
                kind: "directive".to_string(),
                role: "child".to_string(),
            });
        }
    }

    let chain = if let Some(parent_id) = parent.clone() {
        let mut out = vec![parent_id];
        out.extend(child_ids.clone());
        out
    } else {
        child_ids.clone()
    };

    ProposalDependencySummaryOutput {
        proposal_id,
        decision,
        source,
        parent_objective_id: parent,
        child_objective_ids: child_ids.clone(),
        edge_count: edges.len() as u32,
        nodes: nodes.into_iter().take(20).collect(),
        edges: edges.into_iter().take(20).collect(),
        chain,
        dry_run: input.dry_run,
        created_count: input
            .created_count
            .filter(|v| v.is_finite() && *v >= 0.0)
            .unwrap_or(child_ids.len() as f64),
        quality_ok: input.quality_ok,
        reason: input
            .reason
            .as_ref()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty()),
    }
}

pub fn compute_choose_selection_mode(
    input: &ChooseSelectionModeInput,
) -> ChooseSelectionModeOutput {
    let mut mode = "exploit".to_string();
    let mut index: u32 = 0;
    let eligible_len = input.eligible_len;
    let min_eligible = input.min_eligible.max(2);
    let every_n = input.every_n.max(1);
    if eligible_len >= min_eligible
        && input.explore_used < input.explore_quota
        && input.executed_count > 0
        && input.executed_count.is_multiple_of(every_n)
    {
        mode = "explore".to_string();
        let middle = ((eligible_len as f64) / 2.0).floor() as u32;
        index = middle.clamp(1, eligible_len.saturating_sub(1));
    }
    ChooseSelectionModeOutput {
        mode,
        index,
        explore_used: input.explore_used,
        explore_quota: input.explore_quota,
        exploit_used: input.exploit_used,
    }
}

pub fn compute_explore_quota_for_day(input: &ExploreQuotaForDayInput) -> ExploreQuotaForDayOutput {
    let max_runs = input
        .daily_runs_cap
        .filter(|v| v.is_finite())
        .unwrap_or(input.default_max_runs);
    let clamped_max = max_runs.max(1.0);
    let frac = input
        .explore_fraction
        .filter(|v| v.is_finite())
        .unwrap_or(0.2)
        .clamp(0.05, 0.8);
    let quota = (clamped_max * frac).floor().max(1.0);
    ExploreQuotaForDayOutput {
        quota: quota as u32,
    }
}
