    let request: AutoscaleRequest = serde_json::from_str(payload_json)
        .map_err(|e| format!("autoscale_request_parse_failed:{e}"))?;
    let mode_raw = request.mode.trim().to_ascii_lowercase();
    if mode_raw.is_empty() {
        return Err("autoscale_mode_missing".to_string());
    }
    let mode = mode_raw.replace('-', "_").replace(' ', "_");
    let mode = match mode.as_str() {
        "scale_plan" | "plan_scale" => "plan".to_string(),
        "batchmax" => "batch_max".to_string(),
        "dynamiccaps" => "dynamic_caps".to_string(),
        "normalizequeue" => "normalize_queue".to_string(),
        "tokenusage" => "token_usage".to_string(),
        "criteria" | "criteria_check" => "criteria_gate".to_string(),
        _ => mode,
    };
    if mode == "default_criteria_pattern_memory" {
        let input = request
            .default_criteria_pattern_memory_input
            .ok_or_else(|| "autoscale_missing_default_criteria_pattern_memory_input".to_string())?;
        let out = compute_default_criteria_pattern_memory(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "default_criteria_pattern_memory",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_default_criteria_pattern_memory_encode_failed:{e}"));
    }
    if mode == "strategy_execution_mode_effective" {
        let input = request
            .strategy_execution_mode_effective_input
            .ok_or_else(|| {
                "autoscale_missing_strategy_execution_mode_effective_input".to_string()
            })?;
        let out = compute_strategy_execution_mode_effective(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "strategy_execution_mode_effective",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_strategy_execution_mode_effective_encode_failed:{e}"));
    }
    if mode == "strategy_canary_exec_limit_effective" {
        let input = request
            .strategy_canary_exec_limit_effective_input
            .ok_or_else(|| {
                "autoscale_missing_strategy_canary_exec_limit_effective_input".to_string()
            })?;
        let out = compute_strategy_canary_exec_limit_effective(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "strategy_canary_exec_limit_effective",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_strategy_canary_exec_limit_effective_encode_failed:{e}"));
    }
    if mode == "strategy_exploration_effective" {
        let input = request
            .strategy_exploration_effective_input
            .ok_or_else(|| "autoscale_missing_strategy_exploration_effective_input".to_string())?;
        let out = compute_strategy_exploration_effective(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "strategy_exploration_effective",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_strategy_exploration_effective_encode_failed:{e}"));
    }
    if mode == "strategy_budget_effective" {
        let input = request
            .strategy_budget_effective_input
            .ok_or_else(|| "autoscale_missing_strategy_budget_effective_input".to_string())?;
        let out = compute_strategy_budget_effective(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "strategy_budget_effective",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_strategy_budget_effective_encode_failed:{e}"));
    }
    if mode == "preexec_verdict_from_signals" {
        let input = request
            .preexec_verdict_from_signals_input
            .ok_or_else(|| "autoscale_missing_preexec_verdict_from_signals_input".to_string())?;
        let out = compute_preexec_verdict_from_signals(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "preexec_verdict_from_signals",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_preexec_verdict_from_signals_encode_failed:{e}"));
    }
    if mode == "score_only_proposal_churn" {
        let input = request
            .score_only_proposal_churn_input
            .ok_or_else(|| "autoscale_missing_score_only_proposal_churn_input".to_string())?;
        let out = compute_score_only_proposal_churn(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "score_only_proposal_churn",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_score_only_proposal_churn_encode_failed:{e}"));
    }
    if mode == "success_criteria_quality_audit" {
        let input = request
            .success_criteria_quality_audit_input
            .ok_or_else(|| "autoscale_missing_success_criteria_quality_audit_input".to_string())?;
        let out = compute_success_criteria_quality_audit(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "success_criteria_quality_audit",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_success_criteria_quality_audit_encode_failed:{e}"));
    }
    if mode == "detect_eyes_terminology_drift" {
        let input = request
            .detect_eyes_terminology_drift_input
            .ok_or_else(|| "autoscale_missing_detect_eyes_terminology_drift_input".to_string())?;
        let out = compute_detect_eyes_terminology_drift(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "detect_eyes_terminology_drift",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_detect_eyes_terminology_drift_encode_failed:{e}"));
    }
    if mode == "normalize_stored_proposal_row" {
        let input = request
            .normalize_stored_proposal_row_input
            .ok_or_else(|| "autoscale_missing_normalize_stored_proposal_row_input".to_string())?;
        let out = compute_normalize_stored_proposal_row(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "normalize_stored_proposal_row",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_normalize_stored_proposal_row_encode_failed:{e}"));
    }
    if mode == "default_backlog_autoscale_state" {
        let input = request
            .default_backlog_autoscale_state_input
            .ok_or_else(|| "autoscale_missing_default_backlog_autoscale_state_input".to_string())?;
        let out = compute_default_backlog_autoscale_state(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "default_backlog_autoscale_state",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_default_backlog_autoscale_state_encode_failed:{e}"));
    }
    if mode == "normalize_backlog_autoscale_state" {
        let input = request
            .normalize_backlog_autoscale_state_input
            .ok_or_else(|| {
                "autoscale_missing_normalize_backlog_autoscale_state_input".to_string()
            })?;
        let out = compute_normalize_backlog_autoscale_state(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "normalize_backlog_autoscale_state",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_normalize_backlog_autoscale_state_encode_failed:{e}"));
    }
    if mode == "spawn_allocated_cells" {
        let input = request
            .spawn_allocated_cells_input
            .ok_or_else(|| "autoscale_missing_spawn_allocated_cells_input".to_string())?;
        let out = compute_spawn_allocated_cells(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "spawn_allocated_cells",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_spawn_allocated_cells_encode_failed:{e}"));
    }
    if mode == "spawn_capacity_boost_snapshot" {
        let input = request
            .spawn_capacity_boost_snapshot_input
            .ok_or_else(|| "autoscale_missing_spawn_capacity_boost_snapshot_input".to_string())?;
        let out = compute_spawn_capacity_boost_snapshot(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "spawn_capacity_boost_snapshot",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_spawn_capacity_boost_snapshot_encode_failed:{e}"));
    }
    if mode == "inversion_maturity_score" {
        let input = request
            .inversion_maturity_score_input
            .ok_or_else(|| "autoscale_missing_inversion_maturity_score_input".to_string())?;
        let out = compute_inversion_maturity_score(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "inversion_maturity_score",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_inversion_maturity_score_encode_failed:{e}"));
    }
    if mode == "plan" {
        let input = request
            .plan_input
            .ok_or_else(|| "autoscale_missing_plan_input".to_string())?;
        let out = compute_plan(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "plan",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_plan_encode_failed:{e}"));
    }
    if mode == "batch_max" {
        let input = request
            .batch_input
            .ok_or_else(|| "autoscale_missing_batch_input".to_string())?;
        let out = compute_batch_max(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "batch_max",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_batch_encode_failed:{e}"));
    }
    if mode == "dynamic_caps" {
        let input = request
            .dynamic_caps_input
            .ok_or_else(|| "autoscale_missing_dynamic_caps_input".to_string())?;
        let out = compute_dynamic_caps(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "dynamic_caps",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_dynamic_caps_encode_failed:{e}"));
    }
    if mode == "token_usage" {
        let input = request
            .token_usage_input
            .ok_or_else(|| "autoscale_missing_token_usage_input".to_string())?;
        let out = compute_token_usage(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "token_usage",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_token_usage_encode_failed:{e}"));
    }
    if mode == "normalize_queue" {
        let input = request
            .normalize_queue_input
            .ok_or_else(|| "autoscale_missing_normalize_queue_input".to_string())?;
        let out = compute_normalize_queue(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "normalize_queue",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_normalize_queue_encode_failed:{e}"));
    }
    if mode == "criteria_gate" {
        let input = request
            .criteria_gate_input
            .ok_or_else(|| "autoscale_missing_criteria_gate_input".to_string())?;
        let out = compute_criteria_gate(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "criteria_gate",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_criteria_gate_encode_failed:{e}"));
    }
    if mode == "structural_preview_criteria_failure" {
        let input = request
            .structural_preview_criteria_failure_input
            .ok_or_else(|| {
                "autoscale_missing_structural_preview_criteria_failure_input".to_string()
            })?;
        let out = compute_structural_preview_criteria_failure(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "structural_preview_criteria_failure",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_structural_preview_criteria_failure_encode_failed:{e}"));
    }
    if mode == "policy_hold" {
        let input = request
            .policy_hold_input
            .ok_or_else(|| "autoscale_missing_policy_hold_input".to_string())?;
        let out = compute_policy_hold(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "policy_hold",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_policy_hold_encode_failed:{e}"));
    }
    if mode == "policy_hold_result" {
        let input = request
            .policy_hold_result_input
            .ok_or_else(|| "autoscale_missing_policy_hold_result_input".to_string())?;
        let out = compute_policy_hold_result(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "policy_hold_result",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_policy_hold_result_encode_failed:{e}"));
    }
    if mode == "policy_hold_run_event" {
        let input = request
            .policy_hold_run_event_input
            .ok_or_else(|| "autoscale_missing_policy_hold_run_event_input".to_string())?;
        let out = compute_policy_hold_run_event(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "policy_hold_run_event",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_policy_hold_run_event_encode_failed:{e}"));
    }
    if mode == "dod_evidence_diff" {
        let input = request
            .dod_evidence_diff_input
            .ok_or_else(|| "autoscale_missing_dod_evidence_diff_input".to_string())?;
        let out = compute_dod_evidence_diff(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "dod_evidence_diff",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_dod_evidence_diff_encode_failed:{e}"));
    }
    if mode == "score_only_result" {
        let input = request
            .score_only_result_input
            .ok_or_else(|| "autoscale_missing_score_only_result_input".to_string())?;
        let out = compute_score_only_result(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "score_only_result",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_score_only_result_encode_failed:{e}"));
    }
    if mode == "score_only_failure_like" {
        let input = request
            .score_only_failure_like_input
            .ok_or_else(|| "autoscale_missing_score_only_failure_like_input".to_string())?;
        let out = compute_score_only_failure_like(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "score_only_failure_like",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_score_only_failure_like_encode_failed:{e}"));
    }
    if mode == "gate_exhausted_attempt" {
        let input = request
            .gate_exhausted_attempt_input
            .ok_or_else(|| "autoscale_missing_gate_exhausted_attempt_input".to_string())?;
        let out = compute_gate_exhausted_attempt(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "gate_exhausted_attempt",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_gate_exhausted_attempt_encode_failed:{e}"));
    }
