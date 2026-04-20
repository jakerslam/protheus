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
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "normalize_criteria_metric",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_normalize_criteria_metric_encode_failed:{e}"));
    }
    if mode == "escape_reg_exp" {
        let input = request
            .escape_reg_exp_input
            .ok_or_else(|| "autoscale_missing_escape_reg_exp_input".to_string())?;
        let out = compute_escape_reg_exp(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "escape_reg_exp",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_escape_reg_exp_encode_failed:{e}"));
    }
    if mode == "tool_token_mentioned" {
        let input = request
            .tool_token_mentioned_input
            .ok_or_else(|| "autoscale_missing_tool_token_mentioned_input".to_string())?;
        let out = compute_tool_token_mentioned(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "tool_token_mentioned",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_tool_token_mentioned_encode_failed:{e}"));
    }
    if mode == "policy_hold_reason_from_event" {
        let input = request
            .policy_hold_reason_from_event_input
            .ok_or_else(|| "autoscale_missing_policy_hold_reason_from_event_input".to_string())?;
        let out = compute_policy_hold_reason_from_event(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "policy_hold_reason_from_event",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_policy_hold_reason_from_event_encode_failed:{e}"));
    }
    if mode == "strategy_marker_tokens" {
        let input = request
            .strategy_marker_tokens_input
            .ok_or_else(|| "autoscale_missing_strategy_marker_tokens_input".to_string())?;
        let out = compute_strategy_marker_tokens(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "strategy_marker_tokens",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_strategy_marker_tokens_encode_failed:{e}"));
    }
    if mode == "capability_cooldown_key" {
        let input = request
            .capability_cooldown_key_input
            .ok_or_else(|| "autoscale_missing_capability_cooldown_key_input".to_string())?;
        let out = compute_capability_cooldown_key(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "capability_cooldown_key",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_capability_cooldown_key_encode_failed:{e}"));
    }
    if mode == "readiness_retry_cooldown_key" {
        let input = request
            .readiness_retry_cooldown_key_input
            .ok_or_else(|| "autoscale_missing_readiness_retry_cooldown_key_input".to_string())?;
        let out = compute_readiness_retry_cooldown_key(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "readiness_retry_cooldown_key",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_readiness_retry_cooldown_key_encode_failed:{e}"));
    }
    if mode == "source_eye_id" {
        let input = request
            .source_eye_id_input
            .ok_or_else(|| "autoscale_missing_source_eye_id_input".to_string())?;
        let out = compute_source_eye_id(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "source_eye_id",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_source_eye_id_encode_failed:{e}"));
    }
    if mode == "deprioritized_source_proposal" {
        let input = request
            .deprioritized_source_proposal_input
            .ok_or_else(|| "autoscale_missing_deprioritized_source_proposal_input".to_string())?;
        let out = compute_deprioritized_source_proposal(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "deprioritized_source_proposal",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_deprioritized_source_proposal_encode_failed:{e}"));
    }
    if mode == "composite_eligibility_min" {
        let input = request
            .composite_eligibility_min_input
            .ok_or_else(|| "autoscale_missing_composite_eligibility_min_input".to_string())?;
        let out = compute_composite_eligibility_min(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "composite_eligibility_min",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_composite_eligibility_min_encode_failed:{e}"));
    }
    if mode == "clamp_threshold" {
        let input = request
            .clamp_threshold_input
            .ok_or_else(|| "autoscale_missing_clamp_threshold_input".to_string())?;
        let out = compute_clamp_threshold(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "clamp_threshold",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_clamp_threshold_encode_failed:{e}"));
    }
    if mode == "applied_thresholds" {
        let input = request
            .applied_thresholds_input
            .ok_or_else(|| "autoscale_missing_applied_thresholds_input".to_string())?;
        let out = compute_applied_thresholds(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "applied_thresholds",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_applied_thresholds_encode_failed:{e}"));
    }
