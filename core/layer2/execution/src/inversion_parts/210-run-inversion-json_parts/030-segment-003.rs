        }))
        .map_err(|e| format!("inversion_encode_normalize_band_map_failed:{e}"));
    }
    if mode == "normalize_impact_map" {
        let input: NormalizeImpactMapInput = decode_input(&payload, "normalize_impact_map_input")?;
        let out = compute_normalize_impact_map(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "normalize_impact_map",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_normalize_impact_map_failed:{e}"));
    }
    if mode == "normalize_target_map" {
        let input: NormalizeTargetMapInput = decode_input(&payload, "normalize_target_map_input")?;
        let out = compute_normalize_target_map(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "normalize_target_map",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_normalize_target_map_failed:{e}"));
    }
    if mode == "normalize_target_policy" {
        let input: NormalizeTargetPolicyInput =
            decode_input(&payload, "normalize_target_policy_input")?;
        let out = compute_normalize_target_policy(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "normalize_target_policy",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_normalize_target_policy_failed:{e}"));
    }
    if mode == "window_days_for_target" {
        let input: WindowDaysForTargetInput =
            decode_input(&payload, "window_days_for_target_input")?;
        let out = compute_window_days_for_target(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "window_days_for_target",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_window_days_for_target_failed:{e}"));
    }
    if mode == "tier_retention_days" {
        let input: TierRetentionDaysInput = decode_input(&payload, "tier_retention_days_input")?;
        let out = compute_tier_retention_days(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "tier_retention_days",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_tier_retention_days_failed:{e}"));
    }
    if mode == "parse_candidate_list_from_llm_payload" {
        let input: ParseCandidateListFromLlmPayloadInput =
            decode_input(&payload, "parse_candidate_list_from_llm_payload_input")?;
        let out = compute_parse_candidate_list_from_llm_payload(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "parse_candidate_list_from_llm_payload",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_parse_candidate_list_from_llm_payload_failed:{e}"));
    }
    if mode == "heuristic_filter_candidates" {
        let input: HeuristicFilterCandidatesInput =
            decode_input(&payload, "heuristic_filter_candidates_input")?;
        let out = compute_heuristic_filter_candidates(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "heuristic_filter_candidates",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_heuristic_filter_candidates_failed:{e}"));
    }
    if mode == "score_trial" {
        let input: ScoreTrialInput = decode_input(&payload, "score_trial_input")?;
        let out = compute_score_trial(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "score_trial",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_score_trial_failed:{e}"));
    }
    if mode == "mutate_trial_candidates" {
        let input: MutateTrialCandidatesInput =
            decode_input(&payload, "mutate_trial_candidates_input")?;
        let out = compute_mutate_trial_candidates(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "mutate_trial_candidates",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_mutate_trial_candidates_failed:{e}"));
    }
    if mode == "normalize_iso_events" {
        let input: NormalizeIsoEventsInput = decode_input(&payload, "normalize_iso_events_input")?;
        let out = compute_normalize_iso_events(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "normalize_iso_events",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_normalize_iso_events_failed:{e}"));
    }
    if mode == "expand_legacy_count_to_events" {
        let input: ExpandLegacyCountToEventsInput =
            decode_input(&payload, "expand_legacy_count_to_events_input")?;
        let out = compute_expand_legacy_count_to_events(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "expand_legacy_count_to_events",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_expand_legacy_count_to_events_failed:{e}"));
    }
    if mode == "normalize_tier_event_map" {
        let input: NormalizeTierEventMapInput =
            decode_input(&payload, "normalize_tier_event_map_input")?;
        let out = compute_normalize_tier_event_map(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "normalize_tier_event_map",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_normalize_tier_event_map_failed:{e}"));
    }
    if mode == "default_tier_scope" {
        let input: DefaultTierScopeInput = decode_input(&payload, "default_tier_scope_input")?;
        let out = compute_default_tier_scope(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "default_tier_scope",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_default_tier_scope_failed:{e}"));
    }
    if mode == "normalize_tier_scope" {
        let input: NormalizeTierScopeInput = decode_input(&payload, "normalize_tier_scope_input")?;
        let out = compute_normalize_tier_scope(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "normalize_tier_scope",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_normalize_tier_scope_failed:{e}"));
    }
    if mode == "default_tier_governance_state" {
        let input: DefaultTierGovernanceStateInput =
            decode_input(&payload, "default_tier_governance_state_input")?;
        let out = compute_default_tier_governance_state(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "default_tier_governance_state",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_default_tier_governance_state_failed:{e}"));
    }
    if mode == "clone_tier_scope" {
        let input: CloneTierScopeInput = decode_input(&payload, "clone_tier_scope_input")?;
        let out = compute_clone_tier_scope(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "clone_tier_scope",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_clone_tier_scope_failed:{e}"));
    }
    if mode == "prune_tier_scope_events" {
        let input: PruneTierScopeEventsInput =
            decode_input(&payload, "prune_tier_scope_events_input")?;
        let out = compute_prune_tier_scope_events(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "prune_tier_scope_events",
            "payload": out
        }))
