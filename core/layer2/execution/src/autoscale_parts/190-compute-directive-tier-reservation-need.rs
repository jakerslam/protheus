fn normalize_directive_token(raw: &str) -> String {
    let mut out = String::new();
    for ch in raw.trim().to_ascii_lowercase().chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            out.push(ch);
        }
    }
    out
}

fn stem_directive_token(raw: &str) -> String {
    let token = normalize_directive_token(raw);
    if token.len() <= 5 {
        token
    } else {
        token[..5].to_string()
    }
}

pub fn compute_directive_tier_reservation_need(
    input: &DirectiveTierReservationNeedInput,
) -> DirectiveTierReservationNeedOutput {
    let attempts_today = if input.attempts_today.is_finite() {
        input.attempts_today.max(0.0)
    } else {
        0.0
    };
    if !input.enabled || !input.available {
        return DirectiveTierReservationNeedOutput {
            reserve: false,
            tier: None,
            min_share: None,
            attempts_today,
            current_tier_attempts: None,
            required_after_next: None,
            candidate_count: None,
        };
    }

    let clamp_ratio = |value: f64| -> f64 {
        if !value.is_finite() {
            0.0
        } else {
            value.clamp(0.0, 1.0)
        }
    };
    let normalize_tier = |raw: f64, fallback: f64| -> f64 {
        let source = if raw.is_finite() { raw } else { fallback };
        source.round().max(1.0)
    };
    let candidate_tiers = input
        .candidate_tiers
        .iter()
        .map(|raw| normalize_tier(*raw, 99.0))
        .collect::<Vec<_>>();
    for tier in [1.0_f64, 2.0_f64] {
        let min_share = if tier <= 1.0 {
            clamp_ratio(input.tier1_min_share)
        } else {
            clamp_ratio(input.tier2_min_share)
        };
        if min_share <= 0.0 {
            continue;
        }
        let current = if tier <= 1.0 {
            input.tier1_attempts
        } else {
            input.tier2_attempts
        };
        let current = if current.is_finite() {
            current.max(0.0)
        } else {
            0.0
        };
        let required_after_next = ((attempts_today + 1.0) * min_share).ceil();
        if current >= required_after_next {
            continue;
        }
        let candidate_count = candidate_tiers
            .iter()
            .filter(|value| (**value - tier).abs() < 0.000001)
            .count() as u32;
        return DirectiveTierReservationNeedOutput {
            reserve: true,
            tier: Some(tier as u32),
            min_share: Some(min_share),
            attempts_today,
            current_tier_attempts: Some(current),
            required_after_next: Some(required_after_next),
            candidate_count: Some(candidate_count),
        };
    }
    DirectiveTierReservationNeedOutput {
        reserve: false,
        tier: None,
        min_share: None,
        attempts_today,
        current_tier_attempts: None,
        required_after_next: None,
        candidate_count: None,
    }
}

pub fn compute_pulse_objective_cooldown_active(
    input: &PulseObjectiveCooldownActiveInput,
) -> PulseObjectiveCooldownActiveOutput {
    let streak = input.no_progress_streak;
    if !streak.is_finite() {
        return PulseObjectiveCooldownActiveOutput { active: false };
    }
    let limit = if input.no_progress_limit.is_finite() {
        input.no_progress_limit
    } else {
        0.0
    };
    if streak < limit.max(1.0) {
        return PulseObjectiveCooldownActiveOutput { active: false };
    }
    let Some(last_attempt_ts) = input.last_attempt_ts.as_ref() else {
        return PulseObjectiveCooldownActiveOutput { active: false };
    };
    let Some(last_ms) = parse_rfc3339_ts_ms(last_attempt_ts.trim()) else {
        return PulseObjectiveCooldownActiveOutput { active: false };
    };
    let now_ms = input
        .now_ms
        .filter(|v| v.is_finite())
        .unwrap_or_else(|| Utc::now().timestamp_millis() as f64);
    let age_hours = (now_ms - (last_ms as f64)) / (1000.0 * 60.0 * 60.0);
    let cooldown = if input.cooldown_hours.is_finite() {
        input.cooldown_hours
    } else {
        0.0
    };
    PulseObjectiveCooldownActiveOutput {
        active: age_hours < cooldown.max(1.0),
    }
}

pub fn compute_directive_token_hits(input: &DirectiveTokenHitsInput) -> DirectiveTokenHitsOutput {
    let text_tokens = input
        .text_tokens
        .iter()
        .map(|token| normalize_directive_token(token))
        .filter(|token| !token.is_empty())
        .collect::<std::collections::BTreeSet<_>>();
    let text_stems = input
        .text_stems
        .iter()
        .map(|token| normalize_directive_token(token))
        .filter(|token| !token.is_empty())
        .collect::<std::collections::BTreeSet<_>>();
    let mut seen = std::collections::BTreeSet::new();
    let mut hits = Vec::new();
    for token in &input.directive_tokens {
        let token = normalize_directive_token(token);
        if token.is_empty() || !seen.insert(token.clone()) {
            continue;
        }
        if text_tokens.contains(&token) {
            hits.push(token);
            continue;
        }
        let stem = stem_directive_token(&token);
        if !stem.is_empty() && text_stems.contains(&stem) {
            hits.push(token);
        }
    }
    DirectiveTokenHitsOutput { hits }
}

pub fn compute_to_stem(input: &ToStemInput) -> ToStemOutput {
    let stem = input
        .token
        .as_ref()
        .map(|token| stem_directive_token(token))
        .unwrap_or_default();
    ToStemOutput { stem }
}

pub fn compute_normalize_directive_text(
    input: &NormalizeDirectiveTextInput,
) -> NormalizeDirectiveTextOutput {
    let text = input.text.as_deref().unwrap_or("");
    let lowered = text.to_ascii_lowercase();
    let mut scrubbed = String::with_capacity(lowered.len());
    for ch in lowered.chars() {
        if ch.is_ascii_alphanumeric() {
            scrubbed.push(ch);
        } else {
            scrubbed.push(' ');
        }
    }
    let normalized = scrubbed.split_whitespace().collect::<Vec<_>>().join(" ");
    NormalizeDirectiveTextOutput { normalized }
}

pub fn compute_tokenize_directive_text(
    input: &TokenizeDirectiveTextInput,
) -> TokenizeDirectiveTextOutput {
    let normalized = compute_normalize_directive_text(&NormalizeDirectiveTextInput {
        text: input.text.clone(),
    })
    .normalized;
    if normalized.is_empty() {
        return TokenizeDirectiveTextOutput { tokens: Vec::new() };
    }
    let stopwords = input
        .stopwords
        .iter()
        .map(|word| word.trim().to_string())
        .filter(|word| !word.is_empty())
        .collect::<std::collections::BTreeSet<_>>();

    let tokens = normalized
        .split(' ')
        .filter(|token| token.len() >= 3)
        .filter(|token| !token.chars().all(|ch| ch.is_ascii_digit()))
        .filter(|token| !stopwords.contains(*token))
        .map(|token| token.to_string())
        .collect::<Vec<_>>();
    TokenizeDirectiveTextOutput { tokens }
}

pub fn compute_normalize_spaces(input: &NormalizeSpacesInput) -> NormalizeSpacesOutput {
    let text = input.text.as_deref().unwrap_or("");
    let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
    NormalizeSpacesOutput { normalized }
}

pub fn compute_parse_lower_list(input: &ParseLowerListInput) -> ParseLowerListOutput {
    let items = if !input.list.is_empty() {
        input
            .list
            .iter()
            .map(|value| value.trim().to_ascii_lowercase())
            .filter(|value| !value.is_empty())
            .collect::<Vec<_>>()
    } else {
        input
            .csv
            .as_deref()
            .unwrap_or("")
            .split(|ch| matches!(ch, ',' | '\n' | '\r' | ';'))
            .map(|value| value.trim().to_ascii_lowercase())
            .filter(|value| !value.is_empty())
            .collect::<Vec<_>>()
    };
    ParseLowerListOutput { items }
}

pub fn compute_canary_failed_checks_allowed(
    input: &CanaryFailedChecksAllowedInput,
) -> CanaryFailedChecksAllowedOutput {
    let failed = input
        .failed_checks
        .iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    let allowed = input
        .allowed_checks
        .iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect::<std::collections::BTreeSet<_>>();

    if failed.is_empty() || allowed.is_empty() {
        return CanaryFailedChecksAllowedOutput { allowed: false };
    }
    for check in &failed {
        if !allowed.contains(check) {
            return CanaryFailedChecksAllowedOutput { allowed: false };
        }
    }
    CanaryFailedChecksAllowedOutput { allowed: true }
}

pub fn compute_proposal_text_blob(input: &ProposalTextBlobInput) -> ProposalTextBlobOutput {
    let mut parts = vec![
        input.title.as_deref().unwrap_or("").trim().to_string(),
        input.summary.as_deref().unwrap_or("").trim().to_string(),
        input
            .suggested_next_command
            .as_deref()
            .unwrap_or("")
            .trim()
            .to_string(),
        input
            .suggested_command
            .as_deref()
            .unwrap_or("")
            .trim()
            .to_string(),
        input.notes.as_deref().unwrap_or("").trim().to_string(),
    ];
    for ev in &input.evidence {
        let evidence_ref = ev.evidence_ref.as_deref().unwrap_or("").trim().to_string();
        let path = ev.path.as_deref().unwrap_or("").trim().to_string();
        let title = ev.title.as_deref().unwrap_or("").trim().to_string();
        if !evidence_ref.is_empty() {
            parts.push(evidence_ref);
        }
        if !path.is_empty() {
            parts.push(path);
        }
        if !title.is_empty() {
            parts.push(title);
        }
    }
    parts.retain(|value| !value.is_empty());
    let joined = parts.join(" | ");
    let normalized = compute_normalize_spaces(&NormalizeSpacesInput { text: Some(joined) })
        .normalized
        .to_ascii_lowercase();
    ProposalTextBlobOutput { blob: normalized }
}

pub fn compute_percent_mentions_from_text(
    input: &PercentMentionsFromTextInput,
) -> PercentMentionsFromTextOutput {
    let text = input.text.as_deref().unwrap_or("");
    if text.is_empty() {
        return PercentMentionsFromTextOutput { values: Vec::new() };
    }
    let regex = Regex::new(r"(-?\d+(?:\.\d+)?)\s*%").expect("valid percent regex");
    let mut values = Vec::new();
    for capture in regex.captures_iter(text) {
        let raw = capture
            .get(1)
            .and_then(|value| value.as_str().parse::<f64>().ok());
        let Some(raw) = raw else {
            continue;
        };
        if !raw.is_finite() || raw <= 0.0 {
            continue;
        }
        values.push(raw.clamp(0.0, 100.0));
    }
    PercentMentionsFromTextOutput { values }
}

pub fn compute_optimization_min_delta_percent(
    input: &OptimizationMinDeltaPercentInput,
) -> OptimizationMinDeltaPercentOutput {
    let min_delta_percent = if input.high_accuracy_mode {
        input.high_accuracy_value
    } else {
        input.base_value
    };
    OptimizationMinDeltaPercentOutput { min_delta_percent }
}

pub fn compute_source_eye_ref(input: &SourceEyeRefInput) -> SourceEyeRefOutput {
    let meta_eye = input
        .meta_source_eye
        .as_deref()
        .unwrap_or("")
        .trim()
        .to_string();
    if !meta_eye.is_empty() {
        return SourceEyeRefOutput {
            eye_ref: format!("eye:{meta_eye}"),
        };
    }
    let evidence_ref = input
        .first_evidence_ref
        .as_deref()
        .unwrap_or("")
        .trim()
        .to_string();
    if evidence_ref.starts_with("eye:") {
        return SourceEyeRefOutput {
            eye_ref: evidence_ref,
        };
    }
    SourceEyeRefOutput {
        eye_ref: "eye:unknown_eye".to_string(),
    }
}

pub fn compute_normalized_risk(input: &NormalizedRiskInput) -> NormalizedRiskOutput {
    let risk = input
        .risk
        .as_deref()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    let normalized = if risk == "high" || risk == "medium" || risk == "low" {
        risk
    } else {
        "low".to_string()
    };
    NormalizedRiskOutput { risk: normalized }
}

pub fn compute_parse_iso_ts(input: &ParseIsoTsInput) -> ParseIsoTsOutput {
    let ts = input.ts.as_deref().unwrap_or("").trim();
    let timestamp_ms = parse_rfc3339_ts_ms(ts).map(|value| value as f64);
    ParseIsoTsOutput { timestamp_ms }
}

pub fn compute_extract_objective_id_token(
    input: &ExtractObjectiveIdTokenInput,
) -> ExtractObjectiveIdTokenOutput {
    let text = compute_normalize_spaces(&NormalizeSpacesInput {
        text: input.value.clone(),
    })
    .normalized;
    if text.is_empty() {
        return ExtractObjectiveIdTokenOutput { objective_id: None };
    }
    let direct = Regex::new(r"^T[0-9]+_[A-Za-z0-9_]+$").expect("valid direct objective regex");
    if direct.is_match(&text) {
        return ExtractObjectiveIdTokenOutput {
            objective_id: Some(text),
        };
    }
    let token = Regex::new(r"\b(T[0-9]+_[A-Za-z0-9_]+)\b").expect("valid token objective regex");
    let objective_id = token
        .captures(&text)
        .and_then(|capture| capture.get(1))
        .map(|match_| match_.as_str().to_string());
    ExtractObjectiveIdTokenOutput { objective_id }
}

fn normalize_value_currency_token_with_allowed(raw: &str, allowed_keys: &[String]) -> String {
    let token = raw.trim().to_ascii_lowercase();
    if token.is_empty() {
        return String::new();
    }
    if allowed_keys.is_empty() {
        return token;
    }
    if allowed_keys
        .iter()
        .any(|key| key.trim().eq_ignore_ascii_case(&token))
    {
        return token;
    }
    String::new()
}

pub fn compute_normalize_value_currency_token(
    input: &NormalizeValueCurrencyTokenInput,
) -> NormalizeValueCurrencyTokenOutput {
    let token = normalize_value_currency_token_with_allowed(
        input.value.as_deref().unwrap_or(""),
        &input.allowed_keys,
    );
    NormalizeValueCurrencyTokenOutput { token }
}
