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
