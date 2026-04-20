    if mode == "consecutive_gate_exhausted_attempts" {
        let input = request
            .consecutive_gate_exhausted_attempts_input
            .ok_or_else(|| {
                "autoscale_missing_consecutive_gate_exhausted_attempts_input".to_string()
            })?;
        let out = compute_consecutive_gate_exhausted_attempts(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "consecutive_gate_exhausted_attempts",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_consecutive_gate_exhausted_attempts_encode_failed:{e}"));
    }
    if mode == "runs_since_reset_index" {
        let input = request
            .runs_since_reset_index_input
            .ok_or_else(|| "autoscale_missing_runs_since_reset_index_input".to_string())?;
        let out = compute_runs_since_reset_index(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "runs_since_reset_index",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_runs_since_reset_index_encode_failed:{e}"));
    }
    if mode == "attempt_event_indices" {
        let input = request
            .attempt_event_indices_input
            .ok_or_else(|| "autoscale_missing_attempt_event_indices_input".to_string())?;
        let out = compute_attempt_event_indices(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "attempt_event_indices",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_attempt_event_indices_encode_failed:{e}"));
    }
    if mode == "capacity_counted_attempt_indices" {
        let input = request
            .capacity_counted_attempt_indices_input
            .ok_or_else(|| {
                "autoscale_missing_capacity_counted_attempt_indices_input".to_string()
            })?;
        let out = compute_capacity_counted_attempt_indices(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "capacity_counted_attempt_indices",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_capacity_counted_attempt_indices_encode_failed:{e}"));
    }
    if mode == "consecutive_no_progress_runs" {
        let input = request
            .consecutive_no_progress_runs_input
            .ok_or_else(|| "autoscale_missing_consecutive_no_progress_runs_input".to_string())?;
        let out = compute_consecutive_no_progress_runs(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "consecutive_no_progress_runs",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_consecutive_no_progress_runs_encode_failed:{e}"));
    }
    if mode == "shipped_count" {
        let input = request
            .shipped_count_input
            .ok_or_else(|| "autoscale_missing_shipped_count_input".to_string())?;
        let out = compute_shipped_count(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "shipped_count",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_shipped_count_encode_failed:{e}"));
    }
    if mode == "executed_count_by_risk" {
        let input = request
            .executed_count_by_risk_input
            .ok_or_else(|| "autoscale_missing_executed_count_by_risk_input".to_string())?;
        let out = compute_executed_count_by_risk(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "executed_count_by_risk",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_executed_count_by_risk_encode_failed:{e}"));
    }
    if mode == "run_result_tally" {
        let input = request
            .run_result_tally_input
            .ok_or_else(|| "autoscale_missing_run_result_tally_input".to_string())?;
        let out = compute_run_result_tally(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "run_result_tally",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_run_result_tally_encode_failed:{e}"));
    }
    if mode == "qos_lane_weights" {
        let input = request
            .qos_lane_weights_input
            .ok_or_else(|| "autoscale_missing_qos_lane_weights_input".to_string())?;
        let out = compute_qos_lane_weights(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "qos_lane_weights",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_qos_lane_weights_encode_failed:{e}"));
    }
    if mode == "qos_lane_usage" {
        let input = request
            .qos_lane_usage_input
            .ok_or_else(|| "autoscale_missing_qos_lane_usage_input".to_string())?;
        let out = compute_qos_lane_usage(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "qos_lane_usage",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_qos_lane_usage_encode_failed:{e}"));
    }
    if mode == "qos_lane_share_cap_exceeded" {
        let input = request
            .qos_lane_share_cap_exceeded_input
            .ok_or_else(|| "autoscale_missing_qos_lane_share_cap_exceeded_input".to_string())?;
        let out = compute_qos_lane_share_cap_exceeded(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "qos_lane_share_cap_exceeded",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_qos_lane_share_cap_exceeded_encode_failed:{e}"));
    }
    if mode == "qos_lane_from_candidate" {
        let input = request
            .qos_lane_from_candidate_input
            .ok_or_else(|| "autoscale_missing_qos_lane_from_candidate_input".to_string())?;
        let out = compute_qos_lane_from_candidate(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "qos_lane_from_candidate",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_qos_lane_from_candidate_encode_failed:{e}"));
    }
    if mode == "eye_outcome_count_window" {
        let input = request
            .eye_outcome_count_window_input
            .ok_or_else(|| "autoscale_missing_eye_outcome_count_window_input".to_string())?;
        let out = compute_eye_outcome_count_window(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "eye_outcome_count_window",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_eye_outcome_count_window_encode_failed:{e}"));
    }
    if mode == "eye_outcome_count_last_hours" {
        let input = request
            .eye_outcome_count_last_hours_input
            .ok_or_else(|| "autoscale_missing_eye_outcome_count_last_hours_input".to_string())?;
        let out = compute_eye_outcome_count_last_hours(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "eye_outcome_count_last_hours",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_eye_outcome_count_last_hours_encode_failed:{e}"));
    }
    if mode == "sorted_counts" {
        let input = request
            .sorted_counts_input
            .ok_or_else(|| "autoscale_missing_sorted_counts_input".to_string())?;
        let out = compute_sorted_counts(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "sorted_counts",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_sorted_counts_encode_failed:{e}"));
    }
    if mode == "normalize_proposal_status" {
        let input = request
            .normalize_proposal_status_input
            .ok_or_else(|| "autoscale_missing_normalize_proposal_status_input".to_string())?;
        let out = compute_normalize_proposal_status(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "normalize_proposal_status",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_normalize_proposal_status_encode_failed:{e}"));
    }
    if mode == "proposal_status" {
        let input = request
            .proposal_status_input
            .ok_or_else(|| "autoscale_missing_proposal_status_input".to_string())?;
        let out = compute_proposal_status(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "proposal_status",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_proposal_status_encode_failed:{e}"));
    }
    if mode == "proposal_outcome_status" {
        let input = request
            .proposal_outcome_status_input
            .ok_or_else(|| "autoscale_missing_proposal_outcome_status_input".to_string())?;
        let out = compute_proposal_outcome_status(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "proposal_outcome_status",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_proposal_outcome_status_encode_failed:{e}"));
    }
    if mode == "queue_underflow_backfill" {
        let input = request
            .queue_underflow_backfill_input
            .ok_or_else(|| "autoscale_missing_queue_underflow_backfill_input".to_string())?;
        let out = compute_queue_underflow_backfill(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "queue_underflow_backfill",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_queue_underflow_backfill_encode_failed:{e}"));
    }
    if mode == "proposal_risk_score" {
        let input = request
            .proposal_risk_score_input
            .ok_or_else(|| "autoscale_missing_proposal_risk_score_input".to_string())?;
        let out = compute_proposal_risk_score(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "proposal_risk_score",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_proposal_risk_score_encode_failed:{e}"));
    }
    if mode == "proposal_score" {
        let input = request
            .proposal_score_input
            .ok_or_else(|| "autoscale_missing_proposal_score_input".to_string())?;
        let out = compute_proposal_score(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "proposal_score",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_proposal_score_encode_failed:{e}"));
    }
    if mode == "proposal_admission_preview" {
        let input = request
            .proposal_admission_preview_input
            .ok_or_else(|| "autoscale_missing_proposal_admission_preview_input".to_string())?;
        let out = compute_proposal_admission_preview(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "proposal_admission_preview",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_proposal_admission_preview_encode_failed:{e}"));
    }
    if mode == "impact_weight" {
        let input = request
            .impact_weight_input
            .ok_or_else(|| "autoscale_missing_impact_weight_input".to_string())?;
        let out = compute_impact_weight(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "impact_weight",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_impact_weight_encode_failed:{e}"));
    }
    if mode == "risk_penalty" {
        let input = request
            .risk_penalty_input
            .ok_or_else(|| "autoscale_missing_risk_penalty_input".to_string())?;
        let out = compute_risk_penalty(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "risk_penalty",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_risk_penalty_encode_failed:{e}"));
    }
    if mode == "estimate_tokens" {
        let input = request
            .estimate_tokens_input
            .ok_or_else(|| "autoscale_missing_estimate_tokens_input".to_string())?;
        let out = compute_estimate_tokens(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "estimate_tokens",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_estimate_tokens_encode_failed:{e}"));
    }
    if mode == "proposal_remediation_depth" {
        let input = request
            .proposal_remediation_depth_input
            .ok_or_else(|| "autoscale_missing_proposal_remediation_depth_input".to_string())?;
        let out = compute_proposal_remediation_depth(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "proposal_remediation_depth",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_proposal_remediation_depth_encode_failed:{e}"));
    }
    if mode == "proposal_dedup_key" {
        let input = request
            .proposal_dedup_key_input
            .ok_or_else(|| "autoscale_missing_proposal_dedup_key_input".to_string())?;
        let out = compute_proposal_dedup_key(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "proposal_dedup_key",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_proposal_dedup_key_encode_failed:{e}"));
    }
    if mode == "proposal_semantic_fingerprint" {
        let input = request
            .proposal_semantic_fingerprint_input
            .ok_or_else(|| "autoscale_missing_proposal_semantic_fingerprint_input".to_string())?;
        let out = compute_proposal_semantic_fingerprint(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "proposal_semantic_fingerprint",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_proposal_semantic_fingerprint_encode_failed:{e}"));
    }
    if mode == "semantic_token_similarity" {
        let input = request
            .semantic_token_similarity_input
            .ok_or_else(|| "autoscale_missing_semantic_token_similarity_input".to_string())?;
        let out = compute_semantic_token_similarity(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "semantic_token_similarity",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_semantic_token_similarity_encode_failed:{e}"));
    }
    if mode == "semantic_context_comparable" {
        let input = request
            .semantic_context_comparable_input
            .ok_or_else(|| "autoscale_missing_semantic_context_comparable_input".to_string())?;
        let out = compute_semantic_context_comparable(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "semantic_context_comparable",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_semantic_context_comparable_encode_failed:{e}"));
    }
    if mode == "semantic_near_duplicate_match" {
        let input = request
            .semantic_near_duplicate_match_input
            .ok_or_else(|| "autoscale_missing_semantic_near_duplicate_match_input".to_string())?;
        let out = compute_semantic_near_duplicate_match(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "semantic_near_duplicate_match",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_semantic_near_duplicate_match_encode_failed:{e}"));
    }
