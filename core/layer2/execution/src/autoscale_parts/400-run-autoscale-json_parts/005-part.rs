        let input = request
            .criteria_pattern_keys_input
            .ok_or_else(|| "autoscale_missing_criteria_pattern_keys_input".to_string())?;
        let out = compute_criteria_pattern_keys(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "criteria_pattern_keys",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_criteria_pattern_keys_encode_failed:{e}"));
    }
    if mode == "success_criteria_requirement" {
        let input = request
            .success_criteria_requirement_input
            .ok_or_else(|| "autoscale_missing_success_criteria_requirement_input".to_string())?;
        let out = compute_success_criteria_requirement(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "success_criteria_requirement",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_success_criteria_requirement_encode_failed:{e}"));
    }
    if mode == "success_criteria_policy_for_proposal" {
        let input = request
            .success_criteria_policy_for_proposal_input
            .ok_or_else(|| {
                "autoscale_missing_success_criteria_policy_for_proposal_input".to_string()
            })?;
        let out = compute_success_criteria_policy_for_proposal(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "success_criteria_policy_for_proposal",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_success_criteria_policy_for_proposal_encode_failed:{e}"));
    }
    if mode == "capability_descriptor" {
        let input = request
            .capability_descriptor_input
            .ok_or_else(|| "autoscale_missing_capability_descriptor_input".to_string())?;
        let out = compute_capability_descriptor(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "capability_descriptor",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_capability_descriptor_encode_failed:{e}"));
    }
    if mode == "normalize_token_usage_shape" {
        let input = request
            .normalize_token_usage_shape_input
            .ok_or_else(|| "autoscale_missing_normalize_token_usage_shape_input".to_string())?;
        let out = compute_normalize_token_usage_shape(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "normalize_token_usage_shape",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_normalize_token_usage_shape_encode_failed:{e}"));
    }
    if mode == "directive_pulse_context" {
        let input = request
            .directive_pulse_context_input
            .ok_or_else(|| "autoscale_missing_directive_pulse_context_input".to_string())?;
        let out = compute_directive_pulse_context(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "directive_pulse_context",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_directive_pulse_context_encode_failed:{e}"));
    }
    if mode == "is_directive_clarification_proposal" {
        let input = request
            .is_directive_clarification_proposal_input
            .ok_or_else(|| {
                "autoscale_missing_is_directive_clarification_proposal_input".to_string()
            })?;
        let out = compute_is_directive_clarification_proposal(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "is_directive_clarification_proposal",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_is_directive_clarification_proposal_encode_failed:{e}"));
    }
    if mode == "is_directive_decomposition_proposal" {
        let input = request
            .is_directive_decomposition_proposal_input
            .ok_or_else(|| {
                "autoscale_missing_is_directive_decomposition_proposal_input".to_string()
            })?;
        let out = compute_is_directive_decomposition_proposal(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "is_directive_decomposition_proposal",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_is_directive_decomposition_proposal_encode_failed:{e}"));
    }
    if mode == "sanitize_directive_objective_id" {
        let input = request
            .sanitize_directive_objective_id_input
            .ok_or_else(|| "autoscale_missing_sanitize_directive_objective_id_input".to_string())?;
        let out = compute_sanitize_directive_objective_id(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "sanitize_directive_objective_id",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_sanitize_directive_objective_id_encode_failed:{e}"));
    }
    if mode == "sanitized_directive_id_list" {
        let input = request
            .sanitized_directive_id_list_input
            .ok_or_else(|| "autoscale_missing_sanitized_directive_id_list_input".to_string())?;
        let out = compute_sanitized_directive_id_list(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "sanitized_directive_id_list",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_sanitized_directive_id_list_encode_failed:{e}"));
    }
    if mode == "parse_first_json_line" {
        let input = request
            .parse_first_json_line_input
            .ok_or_else(|| "autoscale_missing_parse_first_json_line_input".to_string())?;
        let out = compute_parse_first_json_line(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "parse_first_json_line",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_parse_first_json_line_encode_failed:{e}"));
    }
    if mode == "parse_json_objects_from_text" {
        let input = request
            .parse_json_objects_from_text_input
            .ok_or_else(|| "autoscale_missing_parse_json_objects_from_text_input".to_string())?;
        let out = compute_parse_json_objects_from_text(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "parse_json_objects_from_text",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_parse_json_objects_from_text_encode_failed:{e}"));
    }
    if mode == "read_path_value" {
        let input = request
            .read_path_value_input
            .ok_or_else(|| "autoscale_missing_read_path_value_input".to_string())?;
        let out = compute_read_path_value(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "read_path_value",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_read_path_value_encode_failed:{e}"));
    }
    if mode == "number_or_null" {
        let input = request
            .number_or_null_input
            .ok_or_else(|| "autoscale_missing_number_or_null_input".to_string())?;
        let out = compute_number_or_null(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "number_or_null",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_number_or_null_encode_failed:{e}"));
    }
    if mode == "choose_evidence_selection_mode" {
        let input = request
            .choose_evidence_selection_mode_input
            .ok_or_else(|| "autoscale_missing_choose_evidence_selection_mode_input".to_string())?;
        let out = compute_choose_evidence_selection_mode(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "choose_evidence_selection_mode",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_choose_evidence_selection_mode_encode_failed:{e}"));
    }
    if mode == "truthy_flag" {
        let input = request
            .truthy_flag_input
            .ok_or_else(|| "autoscale_missing_truthy_flag_input".to_string())?;
        let out = compute_truthy_flag(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "truthy_flag",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_truthy_flag_encode_failed:{e}"));
    }
    if mode == "falsey_flag" {
        let input = request
            .falsey_flag_input
            .ok_or_else(|| "autoscale_missing_falsey_flag_input".to_string())?;
        let out = compute_falsey_flag(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "falsey_flag",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_falsey_flag_encode_failed:{e}"));
    }
    if mode == "stable_selection_index" {
        let input = request
            .stable_selection_index_input
            .ok_or_else(|| "autoscale_missing_stable_selection_index_input".to_string())?;
        let out = compute_stable_selection_index(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "stable_selection_index",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_stable_selection_index_encode_failed:{e}"));
    }
    if mode == "as_string_array" {
        let input = request
            .as_string_array_input
            .ok_or_else(|| "autoscale_missing_as_string_array_input".to_string())?;
        let out = compute_as_string_array(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "as_string_array",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_as_string_array_encode_failed:{e}"));
    }
    if mode == "uniq_sorted" {
        let input = request
            .uniq_sorted_input
            .ok_or_else(|| "autoscale_missing_uniq_sorted_input".to_string())?;
        let out = compute_uniq_sorted(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "uniq_sorted",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_uniq_sorted_encode_failed:{e}"));
    }
    if mode == "normalize_model_ids" {
        let input = request
            .normalize_model_ids_input
            .ok_or_else(|| "autoscale_missing_normalize_model_ids_input".to_string())?;
        let out = compute_normalize_model_ids(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "normalize_model_ids",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_normalize_model_ids_encode_failed:{e}"));
    }
    if mode == "selected_model_from_run_event" {
        let input = request
            .selected_model_from_run_event_input
            .ok_or_else(|| "autoscale_missing_selected_model_from_run_event_input".to_string())?;
        let out = compute_selected_model_from_run_event(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "selected_model_from_run_event",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_selected_model_from_run_event_encode_failed:{e}"));
    }
    if mode == "read_first_numeric_metric" {
        let input = request
            .read_first_numeric_metric_input
            .ok_or_else(|| "autoscale_missing_read_first_numeric_metric_input".to_string())?;
        let out = compute_read_first_numeric_metric(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "read_first_numeric_metric",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_read_first_numeric_metric_encode_failed:{e}"));
    }
    if mode == "parse_arg" {
        let input = request
            .parse_arg_input
            .ok_or_else(|| "autoscale_missing_parse_arg_input".to_string())?;
        let out = compute_parse_arg(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "parse_arg",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_parse_arg_encode_failed:{e}"));
    }
    if mode == "date_arg_or_today" {
        let input = request
            .date_arg_or_today_input
            .ok_or_else(|| "autoscale_missing_date_arg_or_today_input".to_string())?;
        let out = compute_date_arg_or_today(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "date_arg_or_today",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_date_arg_or_today_encode_failed:{e}"));
    }
    if mode == "has_env_numeric_override" {
        let input = request
            .has_env_numeric_override_input
            .ok_or_else(|| "autoscale_missing_has_env_numeric_override_input".to_string())?;
        let out = compute_has_env_numeric_override(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "has_env_numeric_override",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_has_env_numeric_override_encode_failed:{e}"));
    }
    if mode == "coalesce_numeric" {
        let input = request
            .coalesce_numeric_input
            .ok_or_else(|| "autoscale_missing_coalesce_numeric_input".to_string())?;
        let out = compute_coalesce_numeric(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "coalesce_numeric",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_coalesce_numeric_encode_failed:{e}"));
    }
    if mode == "clamp_number" {
        let input = request
            .clamp_number_input
            .ok_or_else(|| "autoscale_missing_clamp_number_input".to_string())?;
        let out = compute_clamp_number(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "clamp_number",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_clamp_number_encode_failed:{e}"));
    }
    if mode == "list_proposal_files" {
        let input = request
            .list_proposal_files_input
            .ok_or_else(|| "autoscale_missing_list_proposal_files_input".to_string())?;
        let out = compute_list_proposal_files(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "list_proposal_files",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_list_proposal_files_encode_failed:{e}"));
    }
    if mode == "latest_proposal_date" {
        let input = request
            .latest_proposal_date_input
            .ok_or_else(|| "autoscale_missing_latest_proposal_date_input".to_string())?;
        let out = compute_latest_proposal_date(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "latest_proposal_date",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_latest_proposal_date_encode_failed:{e}"));
    }
    if mode == "now_iso" {
        let input = request
            .now_iso_input
            .ok_or_else(|| "autoscale_missing_now_iso_input".to_string())?;
        let out = compute_now_iso(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "now_iso",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_now_iso_encode_failed:{e}"));
    }
    if mode == "today_str" {
        let input = request
            .today_str_input
            .ok_or_else(|| "autoscale_missing_today_str_input".to_string())?;
        let out = compute_today_str(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "today_str",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_today_str_encode_failed:{e}"));
    }
    if mode == "human_canary_override_approval_phrase" {
        let input = request
            .human_canary_override_approval_phrase_input
            .ok_or_else(|| {
                "autoscale_missing_human_canary_override_approval_phrase_input".to_string()
            })?;
        let out = compute_human_canary_override_approval_phrase(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "human_canary_override_approval_phrase",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_human_canary_override_approval_phrase_encode_failed:{e}"));
    }
    if mode == "parse_human_canary_override_state" {
        let input = request
            .parse_human_canary_override_state_input
            .ok_or_else(|| {
                "autoscale_missing_parse_human_canary_override_state_input".to_string()
            })?;
        let out = compute_parse_human_canary_override_state(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "parse_human_canary_override_state",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_parse_human_canary_override_state_encode_failed:{e}"));
    }
    if mode == "daily_budget_path" {
        let input = request
            .daily_budget_path_input
            .ok_or_else(|| "autoscale_missing_daily_budget_path_input".to_string())?;
        let out = compute_daily_budget_path(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "daily_budget_path",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_daily_budget_path_encode_failed:{e}"));
    }
    if mode == "runs_path_for" {
        let input = request
            .runs_path_for_input
            .ok_or_else(|| "autoscale_missing_runs_path_for_input".to_string())?;
        let out = compute_runs_path_for(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "runs_path_for",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_runs_path_for_encode_failed:{e}"));
    }
    if mode == "effective_tier1_policy" {
        let input = request
            .effective_tier1_policy_input
            .ok_or_else(|| "autoscale_missing_effective_tier1_policy_input".to_string())?;
        let out = compute_effective_tier1_policy(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "effective_tier1_policy",
            "payload": out
