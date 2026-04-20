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
