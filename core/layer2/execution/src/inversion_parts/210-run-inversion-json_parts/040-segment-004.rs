        .map_err(|e| format!("inversion_encode_prune_tier_scope_events_failed:{e}"));
    }
    if mode == "load_tier_governance_state" {
        let input: LoadTierGovernanceStateInput =
            decode_input(&payload, "load_tier_governance_state_input")?;
        let out = compute_load_tier_governance_state(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "load_tier_governance_state",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_load_tier_governance_state_failed:{e}"));
    }
    if mode == "save_tier_governance_state" {
        let input: SaveTierGovernanceStateInput =
            decode_input(&payload, "save_tier_governance_state_input")?;
        let out = compute_save_tier_governance_state(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "save_tier_governance_state",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_save_tier_governance_state_failed:{e}"));
    }
    if mode == "push_tier_event" {
        let input: PushTierEventInput = decode_input(&payload, "push_tier_event_input")?;
        let out = compute_push_tier_event(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "push_tier_event",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_push_tier_event_failed:{e}"));
    }
    if mode == "add_tier_event" {
        let input: AddTierEventInput = decode_input(&payload, "add_tier_event_input")?;
        let out = compute_add_tier_event(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "add_tier_event",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_add_tier_event_failed:{e}"));
    }
    if mode == "increment_live_apply_attempt" {
        let input: IncrementLiveApplyAttemptInput =
            decode_input(&payload, "increment_live_apply_attempt_input")?;
        let out = compute_increment_live_apply_attempt(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "increment_live_apply_attempt",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_increment_live_apply_attempt_failed:{e}"));
    }
    if mode == "increment_live_apply_success" {
        let input: IncrementLiveApplySuccessInput =
            decode_input(&payload, "increment_live_apply_success_input")?;
        let out = compute_increment_live_apply_success(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "increment_live_apply_success",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_increment_live_apply_success_failed:{e}"));
    }
    if mode == "increment_live_apply_safe_abort" {
        let input: IncrementLiveApplySafeAbortInput =
            decode_input(&payload, "increment_live_apply_safe_abort_input")?;
        let out = compute_increment_live_apply_safe_abort(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "increment_live_apply_safe_abort",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_increment_live_apply_safe_abort_failed:{e}"));
    }
    if mode == "update_shadow_trial_counters" {
        let input: UpdateShadowTrialCountersInput =
            decode_input(&payload, "update_shadow_trial_counters_input")?;
        let out = compute_update_shadow_trial_counters(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "update_shadow_trial_counters",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_update_shadow_trial_counters_failed:{e}"));
    }
    if mode == "count_tier_events" {
        let input: CountTierEventsInput = decode_input(&payload, "count_tier_events_input")?;
        let out = compute_count_tier_events(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "count_tier_events",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_count_tier_events_failed:{e}"));
    }
    if mode == "effective_window_days_for_target" {
        let input: EffectiveWindowDaysForTargetInput =
            decode_input(&payload, "effective_window_days_for_target_input")?;
        let out = compute_effective_window_days_for_target(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "effective_window_days_for_target",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_effective_window_days_for_target_failed:{e}"));
    }
    if mode == "to_date" {
        let input: ToDateInput = decode_input(&payload, "to_date_input")?;
        let out = compute_to_date(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "to_date",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_to_date_failed:{e}"));
    }
    if mode == "parse_ts_ms" {
        let input: ParseTsMsInput = decode_input(&payload, "parse_ts_ms_input")?;
        let out = compute_parse_ts_ms(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "parse_ts_ms",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_parse_ts_ms_failed:{e}"));
    }
    if mode == "add_minutes" {
        let input: AddMinutesInput = decode_input(&payload, "add_minutes_input")?;
        let out = compute_add_minutes(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "add_minutes",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_add_minutes_failed:{e}"));
    }
    if mode == "clamp_int" {
        let input: ClampIntInput = decode_input(&payload, "clamp_int_input")?;
        let out = compute_clamp_int(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "clamp_int",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_clamp_int_failed:{e}"));
    }
    if mode == "clamp_number" {
        let input: ClampNumberInput = decode_input(&payload, "clamp_number_input")?;
        let out = compute_clamp_number(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "clamp_number",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_clamp_number_failed:{e}"));
    }
    if mode == "to_bool" {
        let input: ToBoolInput = decode_input(&payload, "to_bool_input")?;
        let out = compute_to_bool(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "to_bool",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_to_bool_failed:{e}"));
    }
    if mode == "clean_text" {
        let input: CleanTextInput = decode_input(&payload, "clean_text_input")?;
        let out = compute_clean_text(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "clean_text",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_clean_text_failed:{e}"));
    }
    if mode == "normalize_token" {
