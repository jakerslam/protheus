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
    if mode == "source_eye_ref" {
        let input = request
            .source_eye_ref_input
            .ok_or_else(|| "autoscale_missing_source_eye_ref_input".to_string())?;
        let out = compute_source_eye_ref(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "source_eye_ref",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_source_eye_ref_encode_failed:{e}"));
    }
    if mode == "normalized_risk" {
        let input = request
            .normalized_risk_input
            .ok_or_else(|| "autoscale_missing_normalized_risk_input".to_string())?;
        let out = compute_normalized_risk(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "normalized_risk",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_normalized_risk_encode_failed:{e}"));
    }
    if mode == "parse_iso_ts" {
        let input = request
            .parse_iso_ts_input
            .ok_or_else(|| "autoscale_missing_parse_iso_ts_input".to_string())?;
        let out = compute_parse_iso_ts(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "parse_iso_ts",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_parse_iso_ts_encode_failed:{e}"));
    }
    if mode == "extract_objective_id_token" {
        let input = request
            .extract_objective_id_token_input
            .ok_or_else(|| "autoscale_missing_extract_objective_id_token_input".to_string())?;
        let out = compute_extract_objective_id_token(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "extract_objective_id_token",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_extract_objective_id_token_encode_failed:{e}"));
    }
    if mode == "normalize_value_currency_token" {
        let input = request
            .normalize_value_currency_token_input
            .ok_or_else(|| "autoscale_missing_normalize_value_currency_token_input".to_string())?;
        let out = compute_normalize_value_currency_token(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "normalize_value_currency_token",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_normalize_value_currency_token_encode_failed:{e}"));
    }
    if mode == "list_value_currencies" {
        let input = request
            .list_value_currencies_input
            .ok_or_else(|| "autoscale_missing_list_value_currencies_input".to_string())?;
        let out = compute_list_value_currencies(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "list_value_currencies",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_list_value_currencies_encode_failed:{e}"));
    }
    if mode == "infer_value_currencies_from_directive_bits" {
        let input = request
            .infer_value_currencies_from_directive_bits_input
            .ok_or_else(|| {
                "autoscale_missing_infer_value_currencies_from_directive_bits_input".to_string()
            })?;
        let out = compute_infer_value_currencies_from_directive_bits(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "infer_value_currencies_from_directive_bits",
            "payload": out
        }))
        .map_err(|e| {
            format!("autoscale_infer_value_currencies_from_directive_bits_encode_failed:{e}")
        });
    }
    if mode == "has_linked_objective_entry" {
        let input = request
            .has_linked_objective_entry_input
            .ok_or_else(|| "autoscale_missing_has_linked_objective_entry_input".to_string())?;
        let out = compute_has_linked_objective_entry(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "has_linked_objective_entry",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_has_linked_objective_entry_encode_failed:{e}"));
    }
    if mode == "verified_entry_outcome" {
        let input = request
            .verified_entry_outcome_input
            .ok_or_else(|| "autoscale_missing_verified_entry_outcome_input".to_string())?;
        let out = compute_verified_entry_outcome(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "verified_entry_outcome",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_verified_entry_outcome_encode_failed:{e}"));
    }
    if mode == "verified_revenue_action" {
        let input = request
            .verified_revenue_action_input
            .ok_or_else(|| "autoscale_missing_verified_revenue_action_input".to_string())?;
        let out = compute_verified_revenue_action(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "verified_revenue_action",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_verified_revenue_action_encode_failed:{e}"));
    }
    if mode == "minutes_until_next_utc_day" {
        let input = request
            .minutes_until_next_utc_day_input
            .ok_or_else(|| "autoscale_missing_minutes_until_next_utc_day_input".to_string())?;
        let out = compute_minutes_until_next_utc_day(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "minutes_until_next_utc_day",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_minutes_until_next_utc_day_encode_failed:{e}"));
    }
    if mode == "age_hours" {
        let input = request
            .age_hours_input
            .ok_or_else(|| "autoscale_missing_age_hours_input".to_string())?;
        let out = compute_age_hours(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "age_hours",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_age_hours_encode_failed:{e}"));
    }
    if mode == "url_domain" {
        let input = request
            .url_domain_input
            .ok_or_else(|| "autoscale_missing_url_domain_input".to_string())?;
        let out = compute_url_domain(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "url_domain",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_url_domain_encode_failed:{e}"));
    }
    if mode == "domain_allowed" {
        let input = request
            .domain_allowed_input
            .ok_or_else(|| "autoscale_missing_domain_allowed_input".to_string())?;
        let out = compute_domain_allowed(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "domain_allowed",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_domain_allowed_encode_failed:{e}"));
    }
    if mode == "is_execute_mode" {
        let input = request
            .is_execute_mode_input
            .ok_or_else(|| "autoscale_missing_is_execute_mode_input".to_string())?;
        let out = compute_is_execute_mode(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "is_execute_mode",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_is_execute_mode_encode_failed:{e}"));
    }
    if mode == "execution_allowed_by_feature_flag" {
        let input = request
            .execution_allowed_by_feature_flag_input
            .ok_or_else(|| {
                "autoscale_missing_execution_allowed_by_feature_flag_input".to_string()
            })?;
        let out = compute_execution_allowed_by_feature_flag(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "execution_allowed_by_feature_flag",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_execution_allowed_by_feature_flag_encode_failed:{e}"));
    }
    if mode == "is_tier1_objective_id" {
        let input = request
            .is_tier1_objective_id_input
            .ok_or_else(|| "autoscale_missing_is_tier1_objective_id_input".to_string())?;
        let out = compute_is_tier1_objective_id(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "is_tier1_objective_id",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_is_tier1_objective_id_encode_failed:{e}"));
    }
    if mode == "is_tier1_candidate_objective" {
        let input = request
            .is_tier1_candidate_objective_input
            .ok_or_else(|| "autoscale_missing_is_tier1_candidate_objective_input".to_string())?;
        let out = compute_is_tier1_candidate_objective(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "is_tier1_candidate_objective",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_is_tier1_candidate_objective_encode_failed:{e}"));
    }
    if mode == "needs_execution_quota" {
        let input = request
            .needs_execution_quota_input
            .ok_or_else(|| "autoscale_missing_needs_execution_quota_input".to_string())?;
        let out = compute_needs_execution_quota(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "needs_execution_quota",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_needs_execution_quota_encode_failed:{e}"));
    }
    if mode == "normalize_criteria_metric" {
        let input = request
            .normalize_criteria_metric_input
            .ok_or_else(|| "autoscale_missing_normalize_criteria_metric_input".to_string())?;
        let out = compute_normalize_criteria_metric(&input);
