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
