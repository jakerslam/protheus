    if mode == "extract_eye_from_evidence_ref" {
        let input = request
            .extract_eye_from_evidence_ref_input
            .ok_or_else(|| "autoscale_missing_extract_eye_from_evidence_ref_input".to_string())?;
        let out = compute_extract_eye_from_evidence_ref(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "extract_eye_from_evidence_ref",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_extract_eye_from_evidence_ref_encode_failed:{e}"));
    }
    if mode == "total_outcomes" {
        let input = request
            .total_outcomes_input
            .ok_or_else(|| "autoscale_missing_total_outcomes_input".to_string())?;
        let out = compute_total_outcomes(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "total_outcomes",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_total_outcomes_encode_failed:{e}"));
    }
    if mode == "derive_entity_bias" {
        let input = request
            .derive_entity_bias_input
            .ok_or_else(|| "autoscale_missing_derive_entity_bias_input".to_string())?;
        let out = compute_derive_entity_bias(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "derive_entity_bias",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_derive_entity_bias_encode_failed:{e}"));
    }
    if mode == "strategy_profile" {
        let input = request
            .strategy_profile_input
            .ok_or_else(|| "autoscale_missing_strategy_profile_input".to_string())?;
        let out = compute_strategy_profile(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "strategy_profile",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_strategy_profile_encode_failed:{e}"));
    }
    if mode == "active_strategy_variants" {
        let input = request
            .active_strategy_variants_input
            .ok_or_else(|| "autoscale_missing_active_strategy_variants_input".to_string())?;
        let out = compute_active_strategy_variants(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "active_strategy_variants",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_active_strategy_variants_encode_failed:{e}"));
    }
    if mode == "strategy_scorecard_summaries" {
        let input = request
            .strategy_scorecard_summaries_input
            .ok_or_else(|| "autoscale_missing_strategy_scorecard_summaries_input".to_string())?;
        let out = compute_strategy_scorecard_summaries(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "strategy_scorecard_summaries",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_strategy_scorecard_summaries_encode_failed:{e}"));
    }
    if mode == "outcome_fitness_policy" {
        let input = request
            .outcome_fitness_policy_input
            .ok_or_else(|| "autoscale_missing_outcome_fitness_policy_input".to_string())?;
        let out = compute_outcome_fitness_policy(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "outcome_fitness_policy",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_outcome_fitness_policy_encode_failed:{e}"));
    }
    if mode == "load_eyes_map" {
        let input = request
            .load_eyes_map_input
            .ok_or_else(|| "autoscale_missing_load_eyes_map_input".to_string())?;
        let out = compute_load_eyes_map(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "load_eyes_map",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_load_eyes_map_encode_failed:{e}"));
    }
    if mode == "fallback_directive_objective_ids" {
        let input = request
            .fallback_directive_objective_ids_input
            .ok_or_else(|| {
                "autoscale_missing_fallback_directive_objective_ids_input".to_string()
            })?;
        let out = compute_fallback_directive_objective_ids(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "fallback_directive_objective_ids",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_fallback_directive_objective_ids_encode_failed:{e}"));
    }
    if mode == "queue_pressure_snapshot" {
        let input = request
            .queue_pressure_snapshot_input
            .ok_or_else(|| "autoscale_missing_queue_pressure_snapshot_input".to_string())?;
        let out = compute_queue_pressure_snapshot(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "queue_pressure_snapshot",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_queue_pressure_snapshot_encode_failed:{e}"));
    }
    if mode == "parse_success_criteria_rows" {
        let input = request
            .parse_success_criteria_rows_input
            .ok_or_else(|| "autoscale_missing_parse_success_criteria_rows_input".to_string())?;
        let out = compute_parse_success_criteria_rows(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "parse_success_criteria_rows",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_parse_success_criteria_rows_encode_failed:{e}"));
    }
    if mode == "collect_outcome_stats" {
        let input = request
            .collect_outcome_stats_input
            .ok_or_else(|| "autoscale_missing_collect_outcome_stats_input".to_string())?;
        let out = compute_collect_outcome_stats(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "collect_outcome_stats",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_collect_outcome_stats_encode_failed:{e}"));
    }
    if mode == "subdirective_v2_signals" {
        let input = request
            .subdirective_v2_signals_input
            .ok_or_else(|| "autoscale_missing_subdirective_v2_signals_input".to_string())?;
        let out = compute_subdirective_v2_signals(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "subdirective_v2_signals",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_subdirective_v2_signals_encode_failed:{e}"));
    }
    if mode == "build_overlay" {
        let input = request
            .build_overlay_input
            .ok_or_else(|| "autoscale_missing_build_overlay_input".to_string())?;
        let out = compute_build_overlay(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "build_overlay",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_build_overlay_encode_failed:{e}"));
    }
    if mode == "has_adaptive_mutation_signal" {
        let input = request
            .has_adaptive_mutation_signal_input
            .ok_or_else(|| "autoscale_missing_has_adaptive_mutation_signal_input".to_string())?;
        let out = compute_has_adaptive_mutation_signal(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "has_adaptive_mutation_signal",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_has_adaptive_mutation_signal_encode_failed:{e}"));
    }
    if mode == "adaptive_mutation_execution_guard" {
        let input = request
            .adaptive_mutation_execution_guard_input
            .ok_or_else(|| {
                "autoscale_missing_adaptive_mutation_execution_guard_input".to_string()
            })?;
        let out = compute_adaptive_mutation_execution_guard(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "adaptive_mutation_execution_guard",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_adaptive_mutation_execution_guard_encode_failed:{e}"));
    }
    if mode == "strategy_selection" {
        let input = request
            .strategy_selection_input
            .ok_or_else(|| "autoscale_missing_strategy_selection_input".to_string())?;
        let out = compute_strategy_selection(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "strategy_selection",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_strategy_selection_encode_failed:{e}"));
    }
    if mode == "calibration_deltas" {
        let input = request
            .calibration_deltas_input
            .ok_or_else(|| "autoscale_missing_calibration_deltas_input".to_string())?;
        let out = compute_calibration_deltas(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "calibration_deltas",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_calibration_deltas_encode_failed:{e}"));
    }
    if mode == "strategy_admission_decision" {
        let input = request
            .strategy_admission_decision_input
            .ok_or_else(|| "autoscale_missing_strategy_admission_decision_input".to_string())?;
        let out = compute_strategy_admission_decision(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "strategy_admission_decision",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_strategy_admission_decision_encode_failed:{e}"));
    }
    if mode == "expected_value_score" {
        let input = request
            .expected_value_score_input
            .ok_or_else(|| "autoscale_missing_expected_value_score_input".to_string())?;
        let out = compute_expected_value_score(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "expected_value_score",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_expected_value_score_encode_failed:{e}"));
    }
    if mode == "suggest_run_batch_max" {
        let input = request
            .suggest_run_batch_max_input
            .ok_or_else(|| "autoscale_missing_suggest_run_batch_max_input".to_string())?;
        let out = compute_suggest_run_batch_max(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "suggest_run_batch_max",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_suggest_run_batch_max_encode_failed:{e}"));
    }
    if mode == "backlog_autoscale_snapshot" {
        let input = request
            .backlog_autoscale_snapshot_input
            .ok_or_else(|| "autoscale_missing_backlog_autoscale_snapshot_input".to_string())?;
        let out = compute_backlog_autoscale_snapshot(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "backlog_autoscale_snapshot",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_backlog_autoscale_snapshot_encode_failed:{e}"));
    }
    if mode == "admission_summary" {
        let input = request
            .admission_summary_input
            .ok_or_else(|| "autoscale_missing_admission_summary_input".to_string())?;
        let out = compute_admission_summary(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "admission_summary",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_admission_summary_encode_failed:{e}"));
    }
    if mode == "unknown_type_quarantine_decision" {
        let input = request
            .unknown_type_quarantine_decision_input
            .ok_or_else(|| {
                "autoscale_missing_unknown_type_quarantine_decision_input".to_string()
            })?;
        let out = compute_unknown_type_quarantine_decision(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "unknown_type_quarantine_decision",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_unknown_type_quarantine_decision_encode_failed:{e}"));
    }
    if mode == "infer_optimization_delta" {
        let input = request
            .infer_optimization_delta_input
            .ok_or_else(|| "autoscale_missing_infer_optimization_delta_input".to_string())?;
        let out = compute_infer_optimization_delta(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "infer_optimization_delta",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_infer_optimization_delta_encode_failed:{e}"));
    }
    if mode == "optimization_intent_proposal" {
        let input = request
            .optimization_intent_proposal_input
            .ok_or_else(|| "autoscale_missing_optimization_intent_proposal_input".to_string())?;
        let out = compute_optimization_intent_proposal(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "optimization_intent_proposal",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_optimization_intent_proposal_encode_failed:{e}"));
    }
    if mode == "unlinked_optimization_admission" {
        let input = request
            .unlinked_optimization_admission_input
            .ok_or_else(|| "autoscale_missing_unlinked_optimization_admission_input".to_string())?;
        let out = compute_unlinked_optimization_admission(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "unlinked_optimization_admission",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_unlinked_optimization_admission_encode_failed:{e}"));
    }
    if mode == "optimization_good_enough" {
        let input = request
            .optimization_good_enough_input
            .ok_or_else(|| "autoscale_missing_optimization_good_enough_input".to_string())?;
        let out = compute_optimization_good_enough(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "optimization_good_enough",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_optimization_good_enough_encode_failed:{e}"));
    }
    if mode == "proposal_dependency_summary" {
        let input = request
            .proposal_dependency_summary_input
            .ok_or_else(|| "autoscale_missing_proposal_dependency_summary_input".to_string())?;
        let out = compute_proposal_dependency_summary(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "proposal_dependency_summary",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_proposal_dependency_summary_encode_failed:{e}"));
    }
    if mode == "choose_selection_mode" {
        let input = request
            .choose_selection_mode_input
            .ok_or_else(|| "autoscale_missing_choose_selection_mode_input".to_string())?;
        let out = compute_choose_selection_mode(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "choose_selection_mode",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_choose_selection_mode_encode_failed:{e}"));
    }
    if mode == "explore_quota_for_day" {
        let input = request
            .explore_quota_for_day_input
            .ok_or_else(|| "autoscale_missing_explore_quota_for_day_input".to_string())?;
        let out = compute_explore_quota_for_day(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "explore_quota_for_day",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_explore_quota_for_day_encode_failed:{e}"));
    }
