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
        }))
        .map_err(|e| format!("autoscale_effective_tier1_policy_encode_failed:{e}"));
    }
    if mode == "compact_tier1_exception" {
        let input = request
            .compact_tier1_exception_input
            .ok_or_else(|| "autoscale_missing_compact_tier1_exception_input".to_string())?;
        let out = compute_compact_tier1_exception(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "compact_tier1_exception",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_compact_tier1_exception_encode_failed:{e}"));
    }
    if mode == "next_human_escalation_clear_at" {
        let input = request
            .next_human_escalation_clear_at_input
            .ok_or_else(|| "autoscale_missing_next_human_escalation_clear_at_input".to_string())?;
        let out = compute_next_human_escalation_clear_at(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "next_human_escalation_clear_at",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_next_human_escalation_clear_at_encode_failed:{e}"));
    }
    if mode == "model_catalog_canary_thresholds" {
        let input = request
            .model_catalog_canary_thresholds_input
            .ok_or_else(|| "autoscale_missing_model_catalog_canary_thresholds_input".to_string())?;
        let out = compute_model_catalog_canary_thresholds(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "model_catalog_canary_thresholds",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_model_catalog_canary_thresholds_encode_failed:{e}"));
    }
    if mode == "parse_directive_file_arg" {
        let input = request
            .parse_directive_file_arg_input
            .ok_or_else(|| "autoscale_missing_parse_directive_file_arg_input".to_string())?;
        let out = compute_parse_directive_file_arg(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "parse_directive_file_arg",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_parse_directive_file_arg_encode_failed:{e}"));
    }
    if mode == "parse_directive_objective_arg" {
        let input = request
            .parse_directive_objective_arg_input
            .ok_or_else(|| "autoscale_missing_parse_directive_objective_arg_input".to_string())?;
        let out = compute_parse_directive_objective_arg(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "parse_directive_objective_arg",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_parse_directive_objective_arg_encode_failed:{e}"));
    }
    if mode == "directive_clarification_exec_spec" {
        let input = request
            .directive_clarification_exec_spec_input
            .ok_or_else(|| {
                "autoscale_missing_directive_clarification_exec_spec_input".to_string()
            })?;
        let out = compute_directive_clarification_exec_spec(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "directive_clarification_exec_spec",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_directive_clarification_exec_spec_encode_failed:{e}"));
    }
    if mode == "directive_decomposition_exec_spec" {
        let input = request
            .directive_decomposition_exec_spec_input
            .ok_or_else(|| {
                "autoscale_missing_directive_decomposition_exec_spec_input".to_string()
            })?;
        let out = compute_directive_decomposition_exec_spec(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "directive_decomposition_exec_spec",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_directive_decomposition_exec_spec_encode_failed:{e}"));
    }
    if mode == "parse_actuation_spec" {
        let input = request
            .parse_actuation_spec_input
            .ok_or_else(|| "autoscale_missing_parse_actuation_spec_input".to_string())?;
        let out = compute_parse_actuation_spec(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "parse_actuation_spec",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_parse_actuation_spec_encode_failed:{e}"));
    }
    if mode == "task_from_proposal" {
        let input = request
            .task_from_proposal_input
            .ok_or_else(|| "autoscale_missing_task_from_proposal_input".to_string())?;
        let out = compute_task_from_proposal(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "task_from_proposal",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_task_from_proposal_encode_failed:{e}"));
    }
    if mode == "parse_objective_id_from_evidence_refs" {
        let input = request
            .parse_objective_id_from_evidence_refs_input
            .ok_or_else(|| {
                "autoscale_missing_parse_objective_id_from_evidence_refs_input".to_string()
            })?;
        let out = compute_parse_objective_id_from_evidence_refs(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "parse_objective_id_from_evidence_refs",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_parse_objective_id_from_evidence_refs_encode_failed:{e}"));
    }
    if mode == "parse_objective_id_from_command" {
        let input = request
            .parse_objective_id_from_command_input
            .ok_or_else(|| "autoscale_missing_parse_objective_id_from_command_input".to_string())?;
        let out = compute_parse_objective_id_from_command(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "parse_objective_id_from_command",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_parse_objective_id_from_command_encode_failed:{e}"));
    }
    if mode == "objective_id_for_execution" {
        let input = request
            .objective_id_for_execution_input
            .ok_or_else(|| "autoscale_missing_objective_id_for_execution_input".to_string())?;
        let out = compute_objective_id_for_execution(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "objective_id_for_execution",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_objective_id_for_execution_encode_failed:{e}"));
    }
    if mode == "short_text" {
        let input = request
            .short_text_input
            .ok_or_else(|| "autoscale_missing_short_text_input".to_string())?;
        let out = compute_short_text(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "short_text",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_short_text_encode_failed:{e}"));
    }
    if mode == "normalized_signal_status" {
        let input = request
            .normalized_signal_status_input
            .ok_or_else(|| "autoscale_missing_normalized_signal_status_input".to_string())?;
        let out = compute_normalized_signal_status(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "normalized_signal_status",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_normalized_signal_status_encode_failed:{e}"));
    }
    if mode == "execution_reserve_snapshot" {
        let input = request
            .execution_reserve_snapshot_input
            .ok_or_else(|| "autoscale_missing_execution_reserve_snapshot_input".to_string())?;
        let out = compute_execution_reserve_snapshot(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "execution_reserve_snapshot",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_execution_reserve_snapshot_encode_failed:{e}"));
    }
    if mode == "budget_pacing_gate" {
        let input = request
            .budget_pacing_gate_input
            .ok_or_else(|| "autoscale_missing_budget_pacing_gate_input".to_string())?;
        let out = compute_budget_pacing_gate(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "budget_pacing_gate",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_budget_pacing_gate_encode_failed:{e}"));
    }
    if mode == "capability_cap" {
        let input = request
            .capability_cap_input
            .ok_or_else(|| "autoscale_missing_capability_cap_input".to_string())?;
        let out = compute_capability_cap(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "capability_cap",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_capability_cap_encode_failed:{e}"));
    }
    if mode == "estimate_tokens_for_candidate" {
        let input = request
            .estimate_tokens_for_candidate_input
            .ok_or_else(|| "autoscale_missing_estimate_tokens_for_candidate_input".to_string())?;
        let out = compute_estimate_tokens_for_candidate(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "estimate_tokens_for_candidate",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_estimate_tokens_for_candidate_encode_failed:{e}"));
    }
    if mode == "proposal_status_for_queue_pressure" {
        let input = request
            .proposal_status_for_queue_pressure_input
            .ok_or_else(|| {
                "autoscale_missing_proposal_status_for_queue_pressure_input".to_string()
            })?;
        let out = compute_proposal_status_for_queue_pressure(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "proposal_status_for_queue_pressure",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_proposal_status_for_queue_pressure_encode_failed:{e}"));
    }
    if mode == "minutes_since_ts" {
        let input = request
            .minutes_since_ts_input
            .ok_or_else(|| "autoscale_missing_minutes_since_ts_input".to_string())?;
        let out = compute_minutes_since_ts(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "minutes_since_ts",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_minutes_since_ts_encode_failed:{e}"));
    }
    if mode == "date_window" {
        let input = request
            .date_window_input
            .ok_or_else(|| "autoscale_missing_date_window_input".to_string())?;
        let out = compute_date_window(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "date_window",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_date_window_encode_failed:{e}"));
    }
    if mode == "in_window" {
        let input = request
            .in_window_input
            .ok_or_else(|| "autoscale_missing_in_window_input".to_string())?;
        let out = compute_in_window(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "in_window",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_in_window_encode_failed:{e}"));
    }
    if mode == "exec_window_match" {
        let input = request
            .exec_window_match_input
            .ok_or_else(|| "autoscale_missing_exec_window_match_input".to_string())?;
        let out = compute_exec_window_match(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "exec_window_match",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_exec_window_match_encode_failed:{e}"));
    }
