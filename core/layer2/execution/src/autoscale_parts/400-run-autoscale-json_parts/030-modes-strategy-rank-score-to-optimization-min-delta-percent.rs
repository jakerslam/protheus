    if mode == "strategy_rank_score" {
        let input = request
            .strategy_rank_score_input
            .ok_or_else(|| "autoscale_missing_strategy_rank_score_input".to_string())?;
        let out = compute_strategy_rank_score(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "strategy_rank_score",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_strategy_rank_score_encode_failed:{e}"));
    }
    if mode == "strategy_rank_adjusted" {
        let input = request
            .strategy_rank_adjusted_input
            .ok_or_else(|| "autoscale_missing_strategy_rank_adjusted_input".to_string())?;
        let out = compute_strategy_rank_adjusted(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "strategy_rank_adjusted",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_strategy_rank_adjusted_encode_failed:{e}"));
    }
    if mode == "trit_shadow_rank_score" {
        let input = request
            .trit_shadow_rank_score_input
            .ok_or_else(|| "autoscale_missing_trit_shadow_rank_score_input".to_string())?;
        let out = compute_trit_shadow_rank_score(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "trit_shadow_rank_score",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_trit_shadow_rank_score_encode_failed:{e}"));
    }
    if mode == "strategy_circuit_cooldown" {
        let input = request
            .strategy_circuit_cooldown_input
            .ok_or_else(|| "autoscale_missing_strategy_circuit_cooldown_input".to_string())?;
        let out = compute_strategy_circuit_cooldown(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "strategy_circuit_cooldown",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_strategy_circuit_cooldown_encode_failed:{e}"));
    }
    if mode == "strategy_trit_shadow_adjusted" {
        let input = request
            .strategy_trit_shadow_adjusted_input
            .ok_or_else(|| "autoscale_missing_strategy_trit_shadow_adjusted_input".to_string())?;
        let out = compute_strategy_trit_shadow_adjusted(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "strategy_trit_shadow_adjusted",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_strategy_trit_shadow_adjusted_encode_failed:{e}"));
    }
    if mode == "non_yield_penalty_score" {
        let input = request
            .non_yield_penalty_score_input
            .ok_or_else(|| "autoscale_missing_non_yield_penalty_score_input".to_string())?;
        let out = compute_non_yield_penalty_score(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "non_yield_penalty_score",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_non_yield_penalty_score_encode_failed:{e}"));
    }
    if mode == "collective_shadow_adjustments" {
        let input = request
            .collective_shadow_adjustments_input
            .ok_or_else(|| "autoscale_missing_collective_shadow_adjustments_input".to_string())?;
        let out = compute_collective_shadow_adjustments(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "collective_shadow_adjustments",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_collective_shadow_adjustments_encode_failed:{e}"));
    }
    if mode == "strategy_trit_shadow_ranking_summary" {
        let input = request
            .strategy_trit_shadow_ranking_summary_input
            .ok_or_else(|| {
                "autoscale_missing_strategy_trit_shadow_ranking_summary_input".to_string()
            })?;
        let out = compute_strategy_trit_shadow_ranking_summary(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "strategy_trit_shadow_ranking_summary",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_strategy_trit_shadow_ranking_summary_encode_failed:{e}"));
    }
    if mode == "shadow_scope_matches" {
        let input = request
            .shadow_scope_matches_input
            .ok_or_else(|| "autoscale_missing_shadow_scope_matches_input".to_string())?;
        let out = compute_shadow_scope_matches(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "shadow_scope_matches",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_shadow_scope_matches_encode_failed:{e}"));
    }
    if mode == "collective_shadow_aggregate" {
        let input = request
            .collective_shadow_aggregate_input
            .ok_or_else(|| "autoscale_missing_collective_shadow_aggregate_input".to_string())?;
        let out = compute_collective_shadow_aggregate(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "collective_shadow_aggregate",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_collective_shadow_aggregate_encode_failed:{e}"));
    }
    if mode == "expected_value_signal" {
        let input = request
            .expected_value_signal_input
            .ok_or_else(|| "autoscale_missing_expected_value_signal_input".to_string())?;
        let out = compute_expected_value_signal(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "expected_value_signal",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_expected_value_signal_encode_failed:{e}"));
    }
    if mode == "value_signal_score" {
        let input = request
            .value_signal_score_input
            .ok_or_else(|| "autoscale_missing_value_signal_score_input".to_string())?;
        let out = compute_value_signal_score(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "value_signal_score",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_value_signal_score_encode_failed:{e}"));
    }
    if mode == "composite_eligibility_score" {
        let input = request
            .composite_eligibility_score_input
            .ok_or_else(|| "autoscale_missing_composite_eligibility_score_input".to_string())?;
        let out = compute_composite_eligibility_score(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "composite_eligibility_score",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_composite_eligibility_score_encode_failed:{e}"));
    }
    if mode == "time_to_value_score" {
        let input = request
            .time_to_value_score_input
            .ok_or_else(|| "autoscale_missing_time_to_value_score_input".to_string())?;
        let out = compute_time_to_value_score(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "time_to_value_score",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_time_to_value_score_encode_failed:{e}"));
    }
    if mode == "value_density_score" {
        let input = request
            .value_density_score_input
            .ok_or_else(|| "autoscale_missing_value_density_score_input".to_string())?;
        let out = compute_value_density_score(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "value_density_score",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_value_density_score_encode_failed:{e}"));
    }
    if mode == "normalize_directive_tier" {
        let input = request
            .normalize_directive_tier_input
            .ok_or_else(|| "autoscale_missing_normalize_directive_tier_input".to_string())?;
        let out = compute_normalize_directive_tier(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "normalize_directive_tier",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_normalize_directive_tier_encode_failed:{e}"));
    }
    if mode == "directive_tier_weight" {
        let input = request
            .directive_tier_weight_input
            .ok_or_else(|| "autoscale_missing_directive_tier_weight_input".to_string())?;
        let out = compute_directive_tier_weight(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "directive_tier_weight",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_directive_tier_weight_encode_failed:{e}"));
    }
    if mode == "directive_tier_min_share" {
        let input = request
            .directive_tier_min_share_input
            .ok_or_else(|| "autoscale_missing_directive_tier_min_share_input".to_string())?;
        let out = compute_directive_tier_min_share(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "directive_tier_min_share",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_directive_tier_min_share_encode_failed:{e}"));
    }
    if mode == "directive_tier_coverage_bonus" {
        let input = request
            .directive_tier_coverage_bonus_input
            .ok_or_else(|| "autoscale_missing_directive_tier_coverage_bonus_input".to_string())?;
        let out = compute_directive_tier_coverage_bonus(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "directive_tier_coverage_bonus",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_directive_tier_coverage_bonus_encode_failed:{e}"));
    }
    if mode == "directive_tier_reservation_need" {
        let input = request
            .directive_tier_reservation_need_input
            .ok_or_else(|| "autoscale_missing_directive_tier_reservation_need_input".to_string())?;
        let out = compute_directive_tier_reservation_need(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "directive_tier_reservation_need",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_directive_tier_reservation_need_encode_failed:{e}"));
    }
    if mode == "pulse_objective_cooldown_active" {
        let input = request
            .pulse_objective_cooldown_active_input
            .ok_or_else(|| "autoscale_missing_pulse_objective_cooldown_active_input".to_string())?;
        let out = compute_pulse_objective_cooldown_active(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "pulse_objective_cooldown_active",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_pulse_objective_cooldown_active_encode_failed:{e}"));
    }
    if mode == "directive_token_hits" {
        let input = request
            .directive_token_hits_input
            .ok_or_else(|| "autoscale_missing_directive_token_hits_input".to_string())?;
        let out = compute_directive_token_hits(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "directive_token_hits",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_directive_token_hits_encode_failed:{e}"));
    }
    if mode == "to_stem" {
        let input = request
            .to_stem_input
            .ok_or_else(|| "autoscale_missing_to_stem_input".to_string())?;
        let out = compute_to_stem(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "to_stem",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_to_stem_encode_failed:{e}"));
    }
    if mode == "normalize_directive_text" {
        let input = request
            .normalize_directive_text_input
            .ok_or_else(|| "autoscale_missing_normalize_directive_text_input".to_string())?;
        let out = compute_normalize_directive_text(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "normalize_directive_text",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_normalize_directive_text_encode_failed:{e}"));
    }
    if mode == "tokenize_directive_text" {
        let input = request
            .tokenize_directive_text_input
            .ok_or_else(|| "autoscale_missing_tokenize_directive_text_input".to_string())?;
        let out = compute_tokenize_directive_text(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "tokenize_directive_text",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_tokenize_directive_text_encode_failed:{e}"));
    }
    if mode == "normalize_spaces" {
        let input = request
            .normalize_spaces_input
            .ok_or_else(|| "autoscale_missing_normalize_spaces_input".to_string())?;
        let out = compute_normalize_spaces(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "normalize_spaces",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_normalize_spaces_encode_failed:{e}"));
    }
    if mode == "parse_lower_list" {
        let input = request
            .parse_lower_list_input
            .ok_or_else(|| "autoscale_missing_parse_lower_list_input".to_string())?;
        let out = compute_parse_lower_list(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "parse_lower_list",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_parse_lower_list_encode_failed:{e}"));
    }
    if mode == "canary_failed_checks_allowed" {
        let input = request
            .canary_failed_checks_allowed_input
            .ok_or_else(|| "autoscale_missing_canary_failed_checks_allowed_input".to_string())?;
        let out = compute_canary_failed_checks_allowed(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "canary_failed_checks_allowed",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_canary_failed_checks_allowed_encode_failed:{e}"));
    }
    if mode == "proposal_text_blob" {
        let input = request
            .proposal_text_blob_input
            .ok_or_else(|| "autoscale_missing_proposal_text_blob_input".to_string())?;
        let out = compute_proposal_text_blob(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "proposal_text_blob",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_proposal_text_blob_encode_failed:{e}"));
    }
    if mode == "percent_mentions_from_text" {
        let input = request
            .percent_mentions_from_text_input
            .ok_or_else(|| "autoscale_missing_percent_mentions_from_text_input".to_string())?;
        let out = compute_percent_mentions_from_text(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "percent_mentions_from_text",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_percent_mentions_from_text_encode_failed:{e}"));
    }
    if mode == "optimization_min_delta_percent" {
        let input = request
            .optimization_min_delta_percent_input
            .ok_or_else(|| "autoscale_missing_optimization_min_delta_percent_input".to_string())?;
        let out = compute_optimization_min_delta_percent(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "optimization_min_delta_percent",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_optimization_min_delta_percent_encode_failed:{e}"));
    }
