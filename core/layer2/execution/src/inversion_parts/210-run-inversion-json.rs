// FILE_SIZE_EXCEPTION: reason=Single dispatch function with dense branch graph; split deferred pending semantic extraction; owner=jay; expires=2026-04-12
pub fn run_inversion_json(payload_json: &str) -> Result<String, String> {
    let payload: Value = serde_json::from_str(payload_json)
        .map_err(|e| format!("inversion_payload_parse_failed:{e}"))?;
    let mode = payload
        .get("mode")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_lowercase())
        .unwrap_or_default();
    if mode.is_empty() {
        return Err("inversion_mode_missing".to_string());
    }
    if mode == "normalize_impact" {
        let input: NormalizeImpactInput = decode_input(&payload, "normalize_impact_input")?;
        let out = compute_normalize_impact(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "normalize_impact",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_normalize_impact_failed:{e}"));
    }
    if mode == "normalize_mode" {
        let input: NormalizeModeInput = decode_input(&payload, "normalize_mode_input")?;
        let out = compute_normalize_mode(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "normalize_mode",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_normalize_mode_failed:{e}"));
    }
    if mode == "normalize_target" {
        let input: NormalizeTargetInput = decode_input(&payload, "normalize_target_input")?;
        let out = compute_normalize_target(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "normalize_target",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_normalize_target_failed:{e}"));
    }
    if mode == "normalize_result" {
        let input: NormalizeResultInput = decode_input(&payload, "normalize_result_input")?;
        let out = compute_normalize_result(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "normalize_result",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_normalize_result_failed:{e}"));
    }
    if mode == "objective_id_valid" {
        let input: ObjectiveIdValidInput = decode_input(&payload, "objective_id_valid_input")?;
        let out = compute_objective_id_valid(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "objective_id_valid",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_objective_id_valid_failed:{e}"));
    }
    if mode == "trit_vector_from_input" {
        let input: TritVectorFromInputInput =
            decode_input(&payload, "trit_vector_from_input_input")?;
        let out = compute_trit_vector_from_input(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "trit_vector_from_input",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_trit_vector_from_input_failed:{e}"));
    }
    if mode == "jaccard_similarity" {
        let input: JaccardSimilarityInput = decode_input(&payload, "jaccard_similarity_input")?;
        let out = compute_jaccard_similarity(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "jaccard_similarity",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_jaccard_similarity_failed:{e}"));
    }
    if mode == "trit_similarity" {
        let input: TritSimilarityInput = decode_input(&payload, "trit_similarity_input")?;
        let out = compute_trit_similarity(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "trit_similarity",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_trit_similarity_failed:{e}"));
    }
    if mode == "certainty_threshold" {
        let input: CertaintyThresholdInput = decode_input(&payload, "certainty_threshold_input")?;
        let out = compute_certainty_threshold(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "certainty_threshold",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_certainty_threshold_failed:{e}"));
    }
    if mode == "max_target_rank" {
        let input: MaxTargetRankInput = decode_input(&payload, "max_target_rank_input")?;
        let out = compute_max_target_rank(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "max_target_rank",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_max_target_rank_failed:{e}"));
    }
    if mode == "creative_penalty" {
        let input: CreativePenaltyInput = decode_input(&payload, "creative_penalty_input")?;
        let out = compute_creative_penalty(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "creative_penalty",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_creative_penalty_failed:{e}"));
    }
    if mode == "extract_bullets" {
        let input: ExtractBulletsInput = decode_input(&payload, "extract_bullets_input")?;
        let out = compute_extract_bullets(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "extract_bullets",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_extract_bullets_failed:{e}"));
    }
    if mode == "extract_list_items" {
        let input: ExtractListItemsInput = decode_input(&payload, "extract_list_items_input")?;
        let out = compute_extract_list_items(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "extract_list_items",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_extract_list_items_failed:{e}"));
    }
    if mode == "parse_system_internal_permission" {
        let input: ParseSystemInternalPermissionInput =
            decode_input(&payload, "parse_system_internal_permission_input")?;
        let out = compute_parse_system_internal_permission(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "parse_system_internal_permission",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_parse_system_internal_permission_failed:{e}"));
    }
    if mode == "parse_soul_token_data_pass_rules" {
        let input: ParseSoulTokenDataPassRulesInput =
            decode_input(&payload, "parse_soul_token_data_pass_rules_input")?;
        let out = compute_parse_soul_token_data_pass_rules(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "parse_soul_token_data_pass_rules",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_parse_soul_token_data_pass_rules_failed:{e}"));
    }
    if mode == "ensure_system_passed_section" {
        let input: EnsureSystemPassedSectionInput =
            decode_input(&payload, "ensure_system_passed_section_input")?;
        let out = compute_ensure_system_passed_section(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "ensure_system_passed_section",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_ensure_system_passed_section_failed:{e}"));
    }
    if mode == "system_passed_payload_hash" {
        let input: SystemPassedPayloadHashInput =
            decode_input(&payload, "system_passed_payload_hash_input")?;
        let out = compute_system_passed_payload_hash(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "system_passed_payload_hash",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_system_passed_payload_hash_failed:{e}"));
    }
    if mode == "build_lens_position" {
        let input: BuildLensPositionInput = decode_input(&payload, "build_lens_position_input")?;
        let out = compute_build_lens_position(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "build_lens_position",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_build_lens_position_failed:{e}"));
    }
    if mode == "build_conclave_proposal_summary" {
        let input: BuildConclaveProposalSummaryInput =
            decode_input(&payload, "build_conclave_proposal_summary_input")?;
        let out = compute_build_conclave_proposal_summary(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "build_conclave_proposal_summary",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_build_conclave_proposal_summary_failed:{e}"));
    }
    if mode == "conclave_high_risk_flags" {
        let input: ConclaveHighRiskFlagsInput =
            decode_input(&payload, "conclave_high_risk_flags_input")?;
        let out = compute_conclave_high_risk_flags(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "conclave_high_risk_flags",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_conclave_high_risk_flags_failed:{e}"));
    }
    if mode == "tokenize_text" {
        let input: TokenizeTextInput = decode_input(&payload, "tokenize_text_input")?;
        let out = compute_tokenize_text(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "tokenize_text",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_tokenize_text_failed:{e}"));
    }
    if mode == "normalize_list" {
        let input: NormalizeListInput = decode_input(&payload, "normalize_list_input")?;
        let out = compute_normalize_list(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "normalize_list",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_normalize_list_failed:{e}"));
    }
    if mode == "normalize_text_list" {
        let input: NormalizeTextListInput = decode_input(&payload, "normalize_text_list_input")?;
        let out = compute_normalize_text_list(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "normalize_text_list",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_normalize_text_list_failed:{e}"));
    }
    if mode == "parse_json_from_stdout" {
        let input: ParseJsonFromStdoutInput =
            decode_input(&payload, "parse_json_from_stdout_input")?;
        let out = compute_parse_json_from_stdout(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "parse_json_from_stdout",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_parse_json_from_stdout_failed:{e}"));
    }
    if mode == "parse_args" {
        let input: ParseArgsInput = decode_input(&payload, "parse_args_input")?;
        let out = compute_parse_args(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "parse_args",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_parse_args_failed:{e}"));
    }
    if mode == "parse_lane_decision" {
        let input: ParseLaneDecisionInput = decode_input(&payload, "parse_lane_decision_input")?;
        let out = compute_parse_lane_decision(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "parse_lane_decision",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_parse_lane_decision_failed:{e}"));
    }
    if mode == "library_match_score" {
        let input: LibraryMatchScoreInput = decode_input(&payload, "library_match_score_input")?;
        let out = compute_library_match_score(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "library_match_score",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_library_match_score_failed:{e}"));
    }
    if mode == "known_failure_pressure" {
        let input: KnownFailurePressureInput =
            decode_input(&payload, "known_failure_pressure_input")?;
        let out = compute_known_failure_pressure(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "known_failure_pressure",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_known_failure_pressure_failed:{e}"));
    }
    if mode == "select_library_candidates" {
        let input: SelectLibraryCandidatesInput =
            decode_input(&payload, "select_library_candidates_input")?;
        let out = compute_select_library_candidates(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "select_library_candidates",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_select_library_candidates_failed:{e}"));
    }
    if mode == "has_signal_term_match" {
        let input: HasSignalTermMatchInput = decode_input(&payload, "has_signal_term_match_input")?;
        let out = compute_has_signal_term_match(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "has_signal_term_match",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_has_signal_term_match_failed:{e}"));
    }
    if mode == "count_axiom_signal_groups" {
        let input: CountAxiomSignalGroupsInput =
            decode_input(&payload, "count_axiom_signal_groups_input")?;
        let out = compute_count_axiom_signal_groups(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "count_axiom_signal_groups",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_count_axiom_signal_groups_failed:{e}"));
    }
    if mode == "effective_first_n_human_veto_uses" {
        let input: EffectiveFirstNHumanVetoUsesInput =
            decode_input(&payload, "effective_first_n_human_veto_uses_input")?;
        let out = compute_effective_first_n_human_veto_uses(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "effective_first_n_human_veto_uses",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_effective_first_n_human_veto_uses_failed:{e}"));
    }
    if mode == "normalize_band_map" {
        let input: NormalizeBandMapInput = decode_input(&payload, "normalize_band_map_input")?;
        let out = compute_normalize_band_map(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "normalize_band_map",
            "payload": out
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
        let input: NormalizeTokenInput = decode_input(&payload, "normalize_token_input")?;
        let out = compute_normalize_token(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "normalize_token",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_normalize_token_failed:{e}"));
    }
    if mode == "normalize_word_token" {
        let input: NormalizeWordTokenInput = decode_input(&payload, "normalize_word_token_input")?;
        let out = compute_normalize_word_token(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "normalize_word_token",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_normalize_word_token_failed:{e}"));
    }
    if mode == "band_to_index" {
        let input: BandToIndexInput = decode_input(&payload, "band_to_index_input")?;
        let out = compute_band_to_index(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "band_to_index",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_band_to_index_failed:{e}"));
    }
    if mode == "escape_regex" {
        let input: EscapeRegexInput = decode_input(&payload, "escape_regex_input")?;
        let out = compute_escape_regex(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "escape_regex",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_escape_regex_failed:{e}"));
    }
    if mode == "pattern_to_word_regex" {
        let input: PatternToWordRegexInput = decode_input(&payload, "pattern_to_word_regex_input")?;
        let out = compute_pattern_to_word_regex(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "pattern_to_word_regex",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_pattern_to_word_regex_failed:{e}"));
    }
    if mode == "stable_id" {
        let input: StableIdInput = decode_input(&payload, "stable_id_input")?;
        let out = compute_stable_id(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "stable_id",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_stable_id_failed:{e}"));
    }
    if mode == "rel_path" {
        let input: RelPathInput = decode_input(&payload, "rel_path_input")?;
        let out = compute_rel_path(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "rel_path",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_rel_path_failed:{e}"));
    }
    if mode == "normalize_axiom_pattern" {
        let input: NormalizeAxiomPatternInput =
            decode_input(&payload, "normalize_axiom_pattern_input")?;
        let out = compute_normalize_axiom_pattern(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "normalize_axiom_pattern",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_normalize_axiom_pattern_failed:{e}"));
    }
    if mode == "normalize_axiom_signal_terms" {
        let input: NormalizeAxiomSignalTermsInput =
            decode_input(&payload, "normalize_axiom_signal_terms_input")?;
        let out = compute_normalize_axiom_signal_terms(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "normalize_axiom_signal_terms",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_normalize_axiom_signal_terms_failed:{e}"));
    }
    if mode == "normalize_observer_id" {
        let input: NormalizeObserverIdInput =
            decode_input(&payload, "normalize_observer_id_input")?;
        let out = compute_normalize_observer_id(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "normalize_observer_id",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_normalize_observer_id_failed:{e}"));
    }
    if mode == "extract_numeric" {
        let input: ExtractNumericInput = decode_input(&payload, "extract_numeric_input")?;
        let out = compute_extract_numeric(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "extract_numeric",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_extract_numeric_failed:{e}"));
    }
    if mode == "pick_first_numeric" {
        let input: PickFirstNumericInput = decode_input(&payload, "pick_first_numeric_input")?;
        let out = compute_pick_first_numeric(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "pick_first_numeric",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_pick_first_numeric_failed:{e}"));
    }
    if mode == "safe_rel_path" {
        let input: SafeRelPathInput = decode_input(&payload, "safe_rel_path_input")?;
        let out = compute_safe_rel_path(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "safe_rel_path",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_safe_rel_path_failed:{e}"));
    }
    if mode == "now_iso" {
        let input: NowIsoInput = decode_input(&payload, "now_iso_input")?;
        let out = compute_now_iso(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "now_iso",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_now_iso_failed:{e}"));
    }
    if mode == "default_tier_event_map" {
        let input: DefaultTierEventMapInput =
            decode_input(&payload, "default_tier_event_map_input")?;
        let out = compute_default_tier_event_map(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "default_tier_event_map",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_default_tier_event_map_failed:{e}"));
    }
    if mode == "coerce_tier_event_map" {
        let input: CoerceTierEventMapInput = decode_input(&payload, "coerce_tier_event_map_input")?;
        let out = compute_coerce_tier_event_map(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "coerce_tier_event_map",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_coerce_tier_event_map_failed:{e}"));
    }
    if mode == "get_tier_scope" {
        let input: GetTierScopeInput = decode_input(&payload, "get_tier_scope_input")?;
        let out = compute_get_tier_scope(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "get_tier_scope",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_get_tier_scope_failed:{e}"));
    }
    if mode == "default_harness_state" {
        let input: DefaultHarnessStateInput =
            decode_input(&payload, "default_harness_state_input")?;
        let out = compute_default_harness_state(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "default_harness_state",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_default_harness_state_failed:{e}"));
    }
    if mode == "default_first_principle_lock_state" {
        let input: DefaultFirstPrincipleLockStateInput =
            decode_input(&payload, "default_first_principle_lock_state_input")?;
        let out = compute_default_first_principle_lock_state(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "default_first_principle_lock_state",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_default_first_principle_lock_state_failed:{e}"));
    }
    if mode == "default_maturity_state" {
        let input: DefaultMaturityStateInput =
            decode_input(&payload, "default_maturity_state_input")?;
        let out = compute_default_maturity_state(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "default_maturity_state",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_default_maturity_state_failed:{e}"));
    }
    if mode == "principle_key_for_session" {
        let input: PrincipleKeyForSessionInput =
            decode_input(&payload, "principle_key_for_session_input")?;
        let out = compute_principle_key_for_session(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "principle_key_for_session",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_principle_key_for_session_failed:{e}"));
    }
    if mode == "normalize_objective_arg" {
        let input: NormalizeObjectiveArgInput =
            decode_input(&payload, "normalize_objective_arg_input")?;
        let out = compute_normalize_objective_arg(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "normalize_objective_arg",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_normalize_objective_arg_failed:{e}"));
    }
    if mode == "maturity_band_order" {
        let input: MaturityBandOrderInput = decode_input(&payload, "maturity_band_order_input")?;
        let out = compute_maturity_band_order(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "maturity_band_order",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_maturity_band_order_failed:{e}"));
    }
    if mode == "current_runtime_mode" {
        let input: CurrentRuntimeModeInput = decode_input(&payload, "current_runtime_mode_input")?;
        let out = compute_current_runtime_mode(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "current_runtime_mode",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_current_runtime_mode_failed:{e}"));
    }
    if mode == "read_drift_from_state_file" {
        let input: ReadDriftFromStateFileInput =
            decode_input(&payload, "read_drift_from_state_file_input")?;
        let out = compute_read_drift_from_state_file(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "read_drift_from_state_file",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_read_drift_from_state_file_failed:{e}"));
    }
    if mode == "resolve_lens_gate_drift" {
        let input: ResolveLensGateDriftInput =
            decode_input(&payload, "resolve_lens_gate_drift_input")?;
        let out = compute_resolve_lens_gate_drift(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "resolve_lens_gate_drift",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_resolve_lens_gate_drift_failed:{e}"));
    }
    if mode == "resolve_parity_confidence" {
        let input: ResolveParityConfidenceInput =
            decode_input(&payload, "resolve_parity_confidence_input")?;
        let out = compute_resolve_parity_confidence(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "resolve_parity_confidence",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_resolve_parity_confidence_failed:{e}"));
    }
    if mode == "compute_attractor_score" {
        let input: ComputeAttractorScoreInput =
            decode_input(&payload, "compute_attractor_score_input")?;
        let out = compute_attractor_score(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "compute_attractor_score",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_compute_attractor_score_failed:{e}"));
    }
    if mode == "detect_immutable_axiom_violation" {
        let input: DetectImmutableAxiomViolationInput =
            decode_input(&payload, "detect_immutable_axiom_violation_input")?;
        let out = compute_detect_immutable_axiom_violation(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "detect_immutable_axiom_violation",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_detect_immutable_axiom_violation_failed:{e}"));
    }
    if mode == "compute_maturity_score" {
        let input: ComputeMaturityScoreInput =
            decode_input(&payload, "compute_maturity_score_input")?;
        let out = compute_maturity_score(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "compute_maturity_score",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_compute_maturity_score_failed:{e}"));
    }
    if mode == "build_output_interfaces" {
        let input: BuildOutputInterfacesInput =
            decode_input(&payload, "build_output_interfaces_input")?;
        let out = compute_build_output_interfaces(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "build_output_interfaces",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_build_output_interfaces_failed:{e}"));
    }
    if mode == "build_code_change_proposal_draft" {
        let input: BuildCodeChangeProposalDraftInput =
            decode_input(&payload, "build_code_change_proposal_draft_input")?;
        let out = compute_build_code_change_proposal_draft(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "build_code_change_proposal_draft",
            "payload": out.proposal
        }))
        .map_err(|e| format!("inversion_encode_build_code_change_proposal_draft_failed:{e}"));
    }
    if mode == "normalize_library_row" {
        let input: NormalizeLibraryRowInput =
            decode_input(&payload, "normalize_library_row_input")?;
        let out = compute_normalize_library_row(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "normalize_library_row",
            "payload": out.row
        }))
        .map_err(|e| format!("inversion_encode_normalize_library_row_failed:{e}"));
    }
    if mode == "ensure_dir" {
        let input: EnsureDirInput = decode_input(&payload, "ensure_dir_input")?;
        let out = compute_ensure_dir(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "ensure_dir",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_ensure_dir_failed:{e}"));
    }
    if mode == "read_json" {
        let input: ReadJsonInput = decode_input(&payload, "read_json_input")?;
        let out = compute_read_json(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "read_json",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_read_json_failed:{e}"));
    }
    if mode == "read_jsonl" {
        let input: ReadJsonlInput = decode_input(&payload, "read_jsonl_input")?;
        let out = compute_read_jsonl(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "read_jsonl",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_read_jsonl_failed:{e}"));
    }
    if mode == "write_json_atomic" {
        let input: WriteJsonAtomicInput = decode_input(&payload, "write_json_atomic_input")?;
        let out = compute_write_json_atomic(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "write_json_atomic",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_write_json_atomic_failed:{e}"));
    }
    if mode == "append_jsonl" {
        let input: AppendJsonlInput = decode_input(&payload, "append_jsonl_input")?;
        let out = compute_append_jsonl(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "append_jsonl",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_append_jsonl_failed:{e}"));
    }
    if mode == "read_text" {
        let input: ReadTextInput = decode_input(&payload, "read_text_input")?;
        let out = compute_read_text(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "read_text",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_read_text_failed:{e}"));
    }
    if mode == "latest_json_file_in_dir" {
        let input: LatestJsonFileInDirInput =
            decode_input(&payload, "latest_json_file_in_dir_input")?;
        let out = compute_latest_json_file_in_dir(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "latest_json_file_in_dir",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_latest_json_file_in_dir_failed:{e}"));
    }
    if mode == "normalize_output_channel" {
        let input: NormalizeOutputChannelInput =
            decode_input(&payload, "normalize_output_channel_input")?;
        let out = compute_normalize_output_channel(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "normalize_output_channel",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_normalize_output_channel_failed:{e}"));
    }
    if mode == "normalize_repo_path" {
        let input: NormalizeRepoPathInput = decode_input(&payload, "normalize_repo_path_input")?;
        let out = compute_normalize_repo_path(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "normalize_repo_path",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_normalize_repo_path_failed:{e}"));
    }
    if mode == "runtime_paths" {
        let input: RuntimePathsInput = decode_input(&payload, "runtime_paths_input")?;
        let out = compute_runtime_paths(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "runtime_paths",
            "payload": out.paths
        }))
        .map_err(|e| format!("inversion_encode_runtime_paths_failed:{e}"));
    }
    if mode == "normalize_axiom_list" {
        let input: NormalizeAxiomListInput = decode_input(&payload, "normalize_axiom_list_input")?;
        let out = compute_normalize_axiom_list(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "normalize_axiom_list",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_normalize_axiom_list_failed:{e}"));
    }
    if mode == "normalize_harness_suite" {
        let input: NormalizeHarnessSuiteInput =
            decode_input(&payload, "normalize_harness_suite_input")?;
        let out = compute_normalize_harness_suite(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "normalize_harness_suite",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_normalize_harness_suite_failed:{e}"));
    }
    if mode == "load_harness_state" {
        let input: LoadHarnessStateInput = decode_input(&payload, "load_harness_state_input")?;
        let out = compute_load_harness_state(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "load_harness_state",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_load_harness_state_failed:{e}"));
    }
    if mode == "save_harness_state" {
        let input: SaveHarnessStateInput = decode_input(&payload, "save_harness_state_input")?;
        let out = compute_save_harness_state(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "save_harness_state",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_save_harness_state_failed:{e}"));
    }
    if mode == "load_first_principle_lock_state" {
        let input: LoadFirstPrincipleLockStateInput =
            decode_input(&payload, "load_first_principle_lock_state_input")?;
        let out = compute_load_first_principle_lock_state(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "load_first_principle_lock_state",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_load_first_principle_lock_state_failed:{e}"));
    }
    if mode == "save_first_principle_lock_state" {
        let input: SaveFirstPrincipleLockStateInput =
            decode_input(&payload, "save_first_principle_lock_state_input")?;
        let out = compute_save_first_principle_lock_state(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "save_first_principle_lock_state",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_save_first_principle_lock_state_failed:{e}"));
    }
    if mode == "check_first_principle_downgrade" {
        let input: CheckFirstPrincipleDowngradeInput =
            decode_input(&payload, "check_first_principle_downgrade_input")?;
        let out = compute_check_first_principle_downgrade(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "check_first_principle_downgrade",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_check_first_principle_downgrade_failed:{e}"));
    }
    if mode == "upsert_first_principle_lock" {
        let input: UpsertFirstPrincipleLockInput =
            decode_input(&payload, "upsert_first_principle_lock_input")?;
        let out = compute_upsert_first_principle_lock(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "upsert_first_principle_lock",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_upsert_first_principle_lock_failed:{e}"));
    }
    if mode == "load_observer_approvals" {
        let input: LoadObserverApprovalsInput =
            decode_input(&payload, "load_observer_approvals_input")?;
        let out = compute_load_observer_approvals(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "load_observer_approvals",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_load_observer_approvals_failed:{e}"));
    }
    if mode == "append_observer_approval" {
        let input: AppendObserverApprovalInput =
            decode_input(&payload, "append_observer_approval_input")?;
        let out = compute_append_observer_approval(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "append_observer_approval",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_append_observer_approval_failed:{e}"));
    }
    if mode == "count_observer_approvals" {
        let input: CountObserverApprovalsInput =
            decode_input(&payload, "count_observer_approvals_input")?;
        let out = compute_count_observer_approvals(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "count_observer_approvals",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_count_observer_approvals_failed:{e}"));
    }
    if mode == "ensure_correspondence_file" {
        let input: EnsureCorrespondenceFileInput =
            decode_input(&payload, "ensure_correspondence_file_input")?;
        let out = compute_ensure_correspondence_file(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "ensure_correspondence_file",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_ensure_correspondence_file_failed:{e}"));
    }
    if mode == "load_maturity_state" {
        let input: LoadMaturityStateInput = decode_input(&payload, "load_maturity_state_input")?;
        let out = compute_load_maturity_state(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "load_maturity_state",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_load_maturity_state_failed:{e}"));
    }
    if mode == "save_maturity_state" {
        let input: SaveMaturityStateInput = decode_input(&payload, "save_maturity_state_input")?;
        let out = compute_save_maturity_state(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "save_maturity_state",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_save_maturity_state_failed:{e}"));
    }
    if mode == "load_active_sessions" {
        let input: LoadActiveSessionsInput = decode_input(&payload, "load_active_sessions_input")?;
        let out = compute_load_active_sessions(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "load_active_sessions",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_load_active_sessions_failed:{e}"));
    }
    if mode == "save_active_sessions" {
        let input: SaveActiveSessionsInput = decode_input(&payload, "save_active_sessions_input")?;
        let out = compute_save_active_sessions(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "save_active_sessions",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_save_active_sessions_failed:{e}"));
    }
    if mode == "sweep_expired_sessions" {
        let input: SweepExpiredSessionsInput =
            decode_input(&payload, "sweep_expired_sessions_input")?;
        let out = compute_sweep_expired_sessions(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "sweep_expired_sessions",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_sweep_expired_sessions_failed:{e}"));
    }
    if mode == "emit_event" {
        let input: EmitEventInput = decode_input(&payload, "emit_event_input")?;
        let out = compute_emit_event(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "emit_event",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_emit_event_failed:{e}"));
    }
    if mode == "append_persona_lens_gate_receipt" {
        let input: AppendPersonaLensGateReceiptInput =
            decode_input(&payload, "append_persona_lens_gate_receipt_input")?;
        let out = compute_append_persona_lens_gate_receipt(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "append_persona_lens_gate_receipt",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_append_persona_lens_gate_receipt_failed:{e}"));
    }
    if mode == "append_conclave_correspondence" {
        let input: AppendConclaveCorrespondenceInput =
            decode_input(&payload, "append_conclave_correspondence_input")?;
        let out = compute_append_conclave_correspondence(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "append_conclave_correspondence",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_append_conclave_correspondence_failed:{e}"));
    }
    if mode == "persist_decision" {
        let input: PersistDecisionInput = decode_input(&payload, "persist_decision_input")?;
        let out = compute_persist_decision(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "persist_decision",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_persist_decision_failed:{e}"));
    }
    if mode == "persist_interface_envelope" {
        let input: PersistInterfaceEnvelopeInput =
            decode_input(&payload, "persist_interface_envelope_input")?;
        let out = compute_persist_interface_envelope(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "persist_interface_envelope",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_persist_interface_envelope_failed:{e}"));
    }
    if mode == "trim_library" {
        let input: TrimLibraryInput = decode_input(&payload, "trim_library_input")?;
        let out = compute_trim_library(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "trim_library",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_trim_library_failed:{e}"));
    }
    if mode == "load_impossibility_signals" {
        let input: LoadImpossibilitySignalsInput =
            decode_input(&payload, "load_impossibility_signals_input")?;
        let out = compute_load_impossibility_signals(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "load_impossibility_signals",
            "payload": out.signals
        }))
        .map_err(|e| format!("inversion_encode_load_impossibility_signals_failed:{e}"));
    }
    if mode == "evaluate_impossibility_trigger" {
        let input: EvaluateImpossibilityTriggerInput =
            decode_input(&payload, "evaluate_impossibility_trigger_input")?;
        let out = compute_evaluate_impossibility_trigger(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "evaluate_impossibility_trigger",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_evaluate_impossibility_trigger_failed:{e}"));
    }
    if mode == "extract_first_principle" {
        let input: ExtractFirstPrincipleInput =
            decode_input(&payload, "extract_first_principle_input")?;
        let out = compute_extract_first_principle(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "extract_first_principle",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_extract_first_principle_failed:{e}"));
    }
    if mode == "extract_failure_cluster_principle" {
        let input: ExtractFailureClusterPrincipleInput =
            decode_input(&payload, "extract_failure_cluster_principle_input")?;
        let out = compute_extract_failure_cluster_principle(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "extract_failure_cluster_principle",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_extract_failure_cluster_principle_failed:{e}"));
    }
    if mode == "persist_first_principle" {
        let input: PersistFirstPrincipleInput =
            decode_input(&payload, "persist_first_principle_input")?;
        let out = compute_persist_first_principle(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "persist_first_principle",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_persist_first_principle_failed:{e}"));
    }
    Err(format!("inversion_mode_unsupported:{mode}"))
}
