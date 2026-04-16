pub fn compute_proposal_semantic_fingerprint(
    input: &ProposalSemanticFingerprintInput,
) -> ProposalSemanticFingerprintOutput {
    let proposal_id_raw = normalize_spaces(input.proposal_id.as_deref().unwrap_or(""));
    let proposal_id = if proposal_id_raw.is_empty() {
        None
    } else {
        Some(proposal_id_raw)
    };
    let proposal_type =
        normalize_spaces(input.proposal_type.as_deref().unwrap_or("")).to_ascii_lowercase();
    let proposal_type = if proposal_type.is_empty() {
        "unknown".to_string()
    } else {
        proposal_type
    };
    let source_eye_raw =
        normalize_spaces(input.source_eye.as_deref().unwrap_or("")).to_ascii_lowercase();
    let source_eye = if source_eye_raw.is_empty() {
        None
    } else {
        Some(source_eye_raw)
    };
    let objective_id_raw = normalize_spaces(input.objective_id.as_deref().unwrap_or(""));
    let objective_id = if objective_id_raw.is_empty() {
        None
    } else {
        Some(objective_id_raw)
    };

    let text_blob = normalize_spaces(input.text_blob.as_deref().unwrap_or(""));
    let tokenized = compute_tokenize_directive_text(&TokenizeDirectiveTextInput {
        text: Some(text_blob),
        stopwords: input.stopwords.clone(),
    });
    let mut stems = std::collections::BTreeSet::new();
    for token in tokenized.tokens {
        let stem = compute_to_stem(&ToStemInput { token: Some(token) }).stem;
        if !stem.is_empty() {
            stems.insert(stem);
        }
    }
    let token_stems: Vec<String> = stems.into_iter().collect();
    let token_count = token_stems.len() as u32;
    let min_tokens_raw = input.min_tokens.unwrap_or(4.0);
    let min_tokens = if min_tokens_raw.is_finite() {
        min_tokens_raw.max(0.0)
    } else {
        4.0
    };
    let eligible = (token_count as f64) >= min_tokens;

    ProposalSemanticFingerprintOutput {
        proposal_id,
        proposal_type,
        source_eye,
        objective_id,
        token_stems,
        token_count,
        eligible,
    }
}

pub fn compute_semantic_token_similarity(
    input: &SemanticTokenSimilarityInput,
) -> SemanticTokenSimilarityOutput {
    let norm = |row: &String| -> Option<String> {
        let token = row.trim();
        if token.is_empty() {
            return None;
        }
        Some(token.to_ascii_lowercase())
    };
    let left: std::collections::HashSet<String> =
        input.left_tokens.iter().filter_map(norm).collect();
    let right: std::collections::HashSet<String> =
        input.right_tokens.iter().filter_map(norm).collect();
    if left.is_empty() || right.is_empty() {
        return SemanticTokenSimilarityOutput { similarity: 0.0 };
    }
    let intersection = left.iter().filter(|token| right.contains(*token)).count() as f64;
    let union = (left.len() + right.len()) as f64 - intersection;
    if union <= 0.0 {
        return SemanticTokenSimilarityOutput { similarity: 0.0 };
    }
    let similarity = ((intersection / union) * 1_000_000.0).round() / 1_000_000.0;
    SemanticTokenSimilarityOutput {
        similarity: similarity.clamp(0.0, 1.0),
    }
}

pub fn compute_semantic_context_comparable(
    input: &SemanticContextComparableInput,
) -> SemanticContextComparableOutput {
    let left_type = input
        .left_proposal_type
        .as_deref()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    let right_type = input
        .right_proposal_type
        .as_deref()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    if input.require_same_type
        && !left_type.is_empty()
        && !right_type.is_empty()
        && left_type != right_type
    {
        return SemanticContextComparableOutput { comparable: false };
    }
    if !input.require_shared_context {
        return SemanticContextComparableOutput { comparable: true };
    }
    let left_eye = input
        .left_source_eye
        .as_deref()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    let right_eye = input
        .right_source_eye
        .as_deref()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    if !left_eye.is_empty() && !right_eye.is_empty() && left_eye == right_eye {
        return SemanticContextComparableOutput { comparable: true };
    }
    let left_objective = input.left_objective_id.as_deref().unwrap_or("").trim();
    let left_objective = left_objective.to_ascii_lowercase();
    let right_objective = input
        .right_objective_id
        .as_deref()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    if !left_objective.is_empty()
        && !right_objective.is_empty()
        && left_objective == right_objective
    {
        return SemanticContextComparableOutput { comparable: true };
    }
    SemanticContextComparableOutput { comparable: false }
}

fn semantic_token_set(tokens: &[String]) -> std::collections::HashSet<String> {
    tokens
        .iter()
        .map(|token| token.trim())
        .filter(|token| !token.is_empty())
        .map(|token| token.to_ascii_lowercase())
        .collect()
}

fn semantic_jaccard_similarity(
    left_tokens: &[String],
    right_tokens: &[String],
) -> SemanticTokenSimilarityOutput {
    let left = semantic_token_set(left_tokens);
    let right = semantic_token_set(right_tokens);
    if left.is_empty() || right.is_empty() {
        return SemanticTokenSimilarityOutput { similarity: 0.0 };
    }
    let intersection = left.iter().filter(|token| right.contains(*token)).count() as f64;
    let union = (left.len() + right.len()) as f64 - intersection;
    if union <= 0.0 {
        return SemanticTokenSimilarityOutput { similarity: 0.0 };
    }
    let similarity = ((intersection / union) * 1_000_000.0).round() / 1_000_000.0;
    SemanticTokenSimilarityOutput {
        similarity: similarity.clamp(0.0, 1.0),
    }
}

fn semantic_context_comparable_for_fingerprints(
    left: &SemanticNearDuplicateFingerprintInput,
    right: &SemanticNearDuplicateFingerprintInput,
    require_same_type: bool,
    require_shared_context: bool,
) -> bool {
    let input = SemanticContextComparableInput {
        left_proposal_type: left.proposal_type.clone(),
        right_proposal_type: right.proposal_type.clone(),
        left_source_eye: left.source_eye.clone(),
        right_source_eye: right.source_eye.clone(),
        left_objective_id: left.objective_id.clone(),
        right_objective_id: right.objective_id.clone(),
        require_same_type,
        require_shared_context,
    };
    compute_semantic_context_comparable(&input).comparable
}

pub fn compute_semantic_near_duplicate_match(
    input: &SemanticNearDuplicateMatchInput,
) -> SemanticNearDuplicateMatchOutput {
    let min_similarity = if input.min_similarity.is_finite() {
        input.min_similarity.clamp(0.0, 1.0)
    } else {
        0.0
    };
    if !input.fingerprint.eligible {
        return SemanticNearDuplicateMatchOutput {
            matched: false,
            similarity: 0.0,
            proposal_id: None,
            proposal_type: None,
            source_eye: None,
            objective_id: None,
        };
    }
    let mut best: Option<SemanticNearDuplicateMatchOutput> = None;
    for candidate in &input.seen_fingerprints {
        if !candidate.eligible {
            continue;
        }
        if !semantic_context_comparable_for_fingerprints(
            &input.fingerprint,
            candidate,
            input.require_same_type,
            input.require_shared_context,
        ) {
            continue;
        }
        let similarity =
            semantic_jaccard_similarity(&input.fingerprint.token_stems, &candidate.token_stems)
                .similarity;
        if similarity < min_similarity {
            continue;
        }
        let candidate_id = candidate
            .proposal_id
            .as_deref()
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase();
        let should_replace = match &best {
            None => true,
            Some(existing) if similarity > existing.similarity => true,
            Some(existing) if (similarity - existing.similarity).abs() <= f64::EPSILON => {
                let existing_id = existing
                    .proposal_id
                    .as_deref()
                    .unwrap_or("")
                    .trim()
                    .to_ascii_lowercase();
                if existing_id.is_empty() {
                    !candidate_id.is_empty()
                } else {
                    !candidate_id.is_empty() && candidate_id < existing_id
                }
            }
            _ => false,
        };
        if should_replace {
            best = Some(SemanticNearDuplicateMatchOutput {
                matched: true,
                similarity,
                proposal_id: candidate.proposal_id.clone(),
                proposal_type: candidate.proposal_type.clone(),
                source_eye: candidate.source_eye.clone(),
                objective_id: candidate.objective_id.clone(),
            });
        }
    }

    best.unwrap_or(SemanticNearDuplicateMatchOutput {
        matched: false,
        similarity: 0.0,
        proposal_id: None,
        proposal_type: None,
        source_eye: None,
        objective_id: None,
    })
}

pub fn compute_strategy_rank_score(input: &StrategyRankScoreInput) -> StrategyRankScoreOutput {
    let raw = (input.composite_weight * input.composite)
        + (input.actionability_weight * input.actionability)
        + (input.directive_fit_weight * input.directive_fit)
        + (input.signal_quality_weight * input.signal_quality)
        + (input.expected_value_weight * input.expected_value)
        + (input.value_density_weight * input.value_density)
        - (input.risk_penalty_weight * input.risk_penalty)
        + (input.time_to_value_weight * input.time_to_value)
        - input.non_yield_penalty
        - input.collective_shadow_penalty
        + input.collective_shadow_bonus;
    StrategyRankScoreOutput {
        score: (raw * 1000.0).round() / 1000.0,
    }
}

pub fn compute_strategy_rank_adjusted(
    input: &StrategyRankAdjustedInput,
) -> StrategyRankAdjustedOutput {
    let to_fixed3 = |value: f64| -> f64 { format!("{value:.3}").parse::<f64>().unwrap_or(value) };
    let pulse_score = input.pulse_score.clamp(0.0, 100.0);
    let pulse_weight = input.pulse_weight.clamp(0.0, 1.0);
    let objective_allocation_score = input.objective_allocation_score.clamp(0.0, 100.0);
    let base_objective_weight = input.base_objective_weight.clamp(0.0, 1.0);
    let objective_weight = if input.canary_mode {
        base_objective_weight
    } else {
        to_fixed3(base_objective_weight * 0.35)
    };
    let pulse_bonus = pulse_weight * pulse_score;
    let objective_bonus = objective_weight * objective_allocation_score;
    let total = to_fixed3(pulse_bonus + objective_bonus);
    let adjusted = to_fixed3(input.base + total);

    StrategyRankAdjustedOutput {
        adjusted,
        bonus: StrategyRankAdjustedBonus {
            pulse_weight,
            pulse_score,
            objective_weight,
            objective_allocation_score,
            total,
        },
    }
}

pub fn compute_trit_shadow_rank_score(
    input: &TritShadowRankScoreInput,
) -> TritShadowRankScoreOutput {
    let to_fixed3 = |value: f64| -> f64 { format!("{value:.3}").parse::<f64>().unwrap_or(value) };
    let score = input.score.clamp(-1.0, 1.0);
    let confidence = input.confidence.clamp(0.0, 1.0);
    let normalized = ((score + 1.0) * 50.0) + (confidence * 10.0);
    let clamped = normalized.clamp(0.0, 100.0);
    TritShadowRankScoreOutput {
        score: to_fixed3(clamped),
    }
}

pub fn compute_strategy_circuit_cooldown(
    input: &StrategyCircuitCooldownInput,
) -> StrategyCircuitCooldownOutput {
    let mut err = input
        .last_error_code
        .as_deref()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    if err.is_empty() {
        err = input
            .last_error
            .as_deref()
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase();
    }
    if err.is_empty() {
        return StrategyCircuitCooldownOutput {
            cooldown_hours: 0.0,
        };
    }

    if err.contains("429") || err.contains("rate_limit") {
        return StrategyCircuitCooldownOutput {
            cooldown_hours: input.http_429_cooldown_hours,
        };
    }
    let has_5xx_code = err.as_bytes().windows(3).any(|window| {
        window[0] == b'5' && window[1].is_ascii_digit() && window[2].is_ascii_digit()
    });
    if err.contains("5xx") || err.contains("server_error") || has_5xx_code {
        return StrategyCircuitCooldownOutput {
            cooldown_hours: input.http_5xx_cooldown_hours,
        };
    }
    if err.contains("dns") || err.contains("enotfound") || err.contains("unreachable") {
        return StrategyCircuitCooldownOutput {
            cooldown_hours: input.dns_error_cooldown_hours,
        };
    }

    StrategyCircuitCooldownOutput {
        cooldown_hours: 0.0,
    }
}

pub fn compute_strategy_trit_shadow_adjusted(
    input: &StrategyTritShadowAdjustedInput,
) -> StrategyTritShadowAdjustedOutput {
    let to_fixed3 = |value: f64| -> f64 { format!("{value:.3}").parse::<f64>().unwrap_or(value) };
    let bonus_applied = to_fixed3(input.bonus_raw * input.bonus_blend);
    let adjusted_score = to_fixed3(input.base_score + bonus_applied);
    StrategyTritShadowAdjustedOutput {
        adjusted_score,
        bonus_applied,
    }
}

pub fn compute_non_yield_penalty_score(
    input: &NonYieldPenaltyScoreInput,
) -> NonYieldPenaltyScoreOutput {
    let to_fixed3 = |value: f64| -> f64 { format!("{value:.3}").parse::<f64>().unwrap_or(value) };
    let raw = (input.policy_hold_rate * input.policy_hold_weight)
        + (input.no_progress_rate * input.no_progress_weight)
        + (input.stop_rate * input.stop_weight)
        - (input.shipped_rate * input.shipped_relief_weight);
    let penalty = raw.clamp(0.0, input.max_penalty.max(0.0));
    NonYieldPenaltyScoreOutput {
        penalty: to_fixed3(penalty),
    }
}

pub fn compute_collective_shadow_adjustments(
    input: &CollectiveShadowAdjustmentsInput,
) -> CollectiveShadowAdjustmentsOutput {
    let to_fixed3 = |value: f64| -> f64 { format!("{value:.3}").parse::<f64>().unwrap_or(value) };
    CollectiveShadowAdjustmentsOutput {
        penalty: to_fixed3(input.penalty_raw.clamp(0.0, input.max_penalty.max(0.0))),
        bonus: to_fixed3(input.bonus_raw.clamp(0.0, input.max_bonus.max(0.0))),
    }
}

pub fn compute_strategy_trit_shadow_ranking_summary(
    input: &StrategyTritShadowRankingSummaryInput,
) -> StrategyTritShadowRankingSummaryOutput {
    let mut ranked = input.rows.clone();
    ranked.sort_by(|a, b| {
        b.trit_rank
            .partial_cmp(&a.trit_rank)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                b.legacy_rank
                    .partial_cmp(&a.legacy_rank)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| a.proposal_id.cmp(&b.proposal_id))
    });

    let legacy_top = input
        .rows
        .first()
        .map(|row| row.proposal_id.trim().to_string())
        .filter(|s| !s.is_empty());
    let trit_top = ranked
        .first()
        .map(|row| row.proposal_id.trim().to_string())
        .filter(|s| !s.is_empty());
    let selected = input
        .selected_proposal_id
        .as_deref()
        .unwrap_or("")
        .trim()
        .to_string();
    let selected_opt = if selected.is_empty() {
        None
    } else {
        Some(selected)
    };
    let mode = input
        .selection_mode
        .as_deref()
        .unwrap_or("")
        .trim()
        .to_string();
    let mode_opt = if mode.is_empty() { None } else { Some(mode) };

    let top_k = input.top_k.max(1) as usize;
    let top = ranked.into_iter().take(top_k).collect::<Vec<_>>();
    StrategyTritShadowRankingSummaryOutput {
        considered: input.rows.len() as u32,
        selection_mode: mode_opt,
        selected_proposal_id: selected_opt.clone(),
        legacy_top_proposal_id: legacy_top.clone(),
        trit_top_proposal_id: trit_top.clone(),
        diverged_from_legacy_top: match (&legacy_top, &trit_top) {
            (Some(a), Some(b)) => a != b,
            _ => false,
        },
        diverged_from_selected: match (&selected_opt, &trit_top) {
            (Some(a), Some(b)) => a != b,
            _ => false,
        },
        top,
    }
}
