fn default_tier_scope_value(legacy: Option<&Value>, legacy_ts: &str) -> Value {
    let live_apply_attempts = normalize_tier_event_map_value(
        Some(&json!({})),
        Some(&default_tier_event_map_value()),
        legacy
            .and_then(|v| v.as_object())
            .and_then(|m| m.get("live_apply_attempts"))
            .or_else(|| {
                legacy
                    .and_then(|v| v.as_object())
                    .and_then(|m| m.get("live_apply_counts"))
            }),
        legacy_ts,
    );
    let live_apply_successes = normalize_tier_event_map_value(
        Some(&json!({})),
        Some(&default_tier_event_map_value()),
        legacy
            .and_then(|v| v.as_object())
            .and_then(|m| m.get("live_apply_successes"))
            .or_else(|| {
                legacy
                    .and_then(|v| v.as_object())
                    .and_then(|m| m.get("live_apply_counts"))
            }),
        legacy_ts,
    );
    let live_apply_safe_aborts = normalize_tier_event_map_value(
        Some(&json!({})),
        Some(&default_tier_event_map_value()),
        legacy
            .and_then(|v| v.as_object())
            .and_then(|m| m.get("live_apply_safe_aborts")),
        legacy_ts,
    );
    let shadow_passes = normalize_tier_event_map_value(
        Some(&json!({})),
        Some(&default_tier_event_map_value()),
        legacy
            .and_then(|v| v.as_object())
            .and_then(|m| m.get("shadow_passes"))
            .or_else(|| {
                legacy
                    .and_then(|v| v.as_object())
                    .and_then(|m| m.get("shadow_pass_counts"))
            }),
        legacy_ts,
    );
    let shadow_critical_failures = normalize_tier_event_map_value(
        Some(&json!({})),
        Some(&default_tier_event_map_value()),
        legacy
            .and_then(|v| v.as_object())
            .and_then(|m| m.get("shadow_critical_failures")),
        legacy_ts,
    );
    json!({
        "live_apply_attempts": live_apply_attempts,
        "live_apply_successes": live_apply_successes,
        "live_apply_safe_aborts": live_apply_safe_aborts,
        "shadow_passes": shadow_passes,
        "shadow_critical_failures": shadow_critical_failures
    })
}

pub fn compute_default_tier_scope(input: &DefaultTierScopeInput) -> DefaultTierScopeOutput {
    let legacy_ts = input.legacy_ts.clone().unwrap_or_else(now_iso_runtime);
    DefaultTierScopeOutput {
        scope: default_tier_scope_value(input.legacy.as_ref(), &legacy_ts),
    }
}

fn normalize_tier_scope_value(
    scope: Option<&Value>,
    legacy: Option<&Value>,
    legacy_ts: &str,
) -> Value {
    let src = scope
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    let fallback = default_tier_scope_value(legacy, legacy_ts);
    json!({
        "live_apply_attempts": normalize_tier_event_map_value(src.get("live_apply_attempts"), value_path(Some(&fallback), &["live_apply_attempts"]), None, legacy_ts),
        "live_apply_successes": normalize_tier_event_map_value(src.get("live_apply_successes"), value_path(Some(&fallback), &["live_apply_successes"]), None, legacy_ts),
        "live_apply_safe_aborts": normalize_tier_event_map_value(src.get("live_apply_safe_aborts"), value_path(Some(&fallback), &["live_apply_safe_aborts"]), None, legacy_ts),
        "shadow_passes": normalize_tier_event_map_value(src.get("shadow_passes"), value_path(Some(&fallback), &["shadow_passes"]), None, legacy_ts),
        "shadow_critical_failures": normalize_tier_event_map_value(src.get("shadow_critical_failures"), value_path(Some(&fallback), &["shadow_critical_failures"]), None, legacy_ts)
    })
}

pub fn compute_normalize_tier_scope(input: &NormalizeTierScopeInput) -> NormalizeTierScopeOutput {
    let legacy_ts = input.legacy_ts.clone().unwrap_or_else(now_iso_runtime);
    NormalizeTierScopeOutput {
        scope: normalize_tier_scope_value(input.scope.as_ref(), input.legacy.as_ref(), &legacy_ts),
    }
}

pub fn compute_default_tier_governance_state(
    input: &DefaultTierGovernanceStateInput,
) -> DefaultTierGovernanceStateOutput {
    let safe_version =
        normalize_policy_version_runtime(input.policy_version.as_deref().unwrap_or("1.0"));
    let scope = default_tier_scope_value(None, &now_iso_runtime());
    DefaultTierGovernanceStateOutput {
        state: json!({
            "schema_id": "inversion_tier_governance_state",
            "schema_version": "1.0",
            "active_policy_version": safe_version,
            "updated_at": now_iso_runtime(),
            "scopes": {
                safe_version.clone(): scope
            }
        }),
    }
}

pub fn compute_clone_tier_scope(input: &CloneTierScopeInput) -> CloneTierScopeOutput {
    CloneTierScopeOutput {
        scope: normalize_tier_scope_value(input.scope.as_ref(), None, &now_iso_runtime()),
    }
}

pub fn compute_prune_tier_scope_events(
    input: &PruneTierScopeEventsInput,
) -> PruneTierScopeEventsOutput {
    let retention_days = input.retention_days.unwrap_or(365).clamp(1, 3650);
    let mut out = normalize_tier_scope_value(input.scope.as_ref(), None, &now_iso_runtime());
    let keep_cutoff = Utc::now().timestamp_millis() - (retention_days * 24 * 60 * 60 * 1000);
    for metric in TIER_METRICS {
        let mut map = out
            .as_object()
            .and_then(|obj| obj.get(metric))
            .cloned()
            .unwrap_or_else(default_tier_event_map_value);
        for target in TIER_TARGETS {
            let rows = map
                .as_object()
                .and_then(|m| m.get(target))
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            let filtered = rows
                .iter()
                .map(|row| value_to_string(Some(row)))
                .filter(|row| parse_ts_ms_runtime(row) >= keep_cutoff)
                .collect::<Vec<_>>();
            let kept = if filtered.len() > 10000 {
                filtered[(filtered.len() - 10000)..].to_vec()
            } else {
                filtered
            };
            if let Some(map_obj) = map.as_object_mut() {
                map_obj.insert(
                    target.to_string(),
                    Value::Array(kept.into_iter().map(Value::String).collect::<Vec<_>>()),
                );
            }
        }
        if let Some(obj) = out.as_object_mut() {
            obj.insert(metric.to_string(), map);
        }
    }
    PruneTierScopeEventsOutput { scope: out }
}

pub fn compute_count_tier_events(input: &CountTierEventsInput) -> CountTierEventsOutput {
    let metric = clean_text_runtime(input.metric.as_deref().unwrap_or(""), 80);
    let target = normalize_target_for_key(input.target.as_deref().unwrap_or("tactical"));
    let map = input
        .scope
        .as_ref()
        .and_then(|scope| scope.as_object())
        .and_then(|scope| scope.get(&metric))
        .cloned()
        .unwrap_or_else(default_tier_event_map_value);
    let rows = map
        .as_object()
        .and_then(|m| m.get(&target))
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let window_days = input.window_days.unwrap_or(90).clamp(1, 3650);
    let cutoff = Utc::now().timestamp_millis() - (window_days * 24 * 60 * 60 * 1000);
    let count = rows
        .iter()
        .filter(|row| parse_ts_ms_runtime(&value_to_string(Some(row))) >= cutoff)
        .count() as i64;
    CountTierEventsOutput { count }
}

pub fn compute_effective_window_days_for_target(
    input: &EffectiveWindowDaysForTargetInput,
) -> EffectiveWindowDaysForTargetOutput {
    let configured = compute_window_days_for_target(&WindowDaysForTargetInput {
        window_map: input.window_map.clone(),
        target: input.target.clone(),
        fallback: input.fallback,
    })
    .days;
    let minimum = compute_window_days_for_target(&WindowDaysForTargetInput {
        window_map: input.minimum_window_map.clone(),
        target: input.target.clone(),
        fallback: Some(1),
    })
    .days;
    EffectiveWindowDaysForTargetOutput {
        days: configured.max(minimum),
    }
}

pub fn compute_to_date(input: &ToDateInput) -> ToDateOutput {
    let raw = input.value.as_deref().unwrap_or("").trim().to_string();
    let valid = Regex::new(r"^\d{4}-\d{2}-\d{2}$")
        .ok()
        .map(|re| re.is_match(&raw))
        .unwrap_or(false);
    if valid && NaiveDate::parse_from_str(&raw, "%Y-%m-%d").is_ok() {
        return ToDateOutput { value: raw };
    }
    ToDateOutput {
        value: now_iso_runtime().chars().take(10).collect::<String>(),
    }
}

pub fn compute_parse_ts_ms(input: &ParseTsMsInput) -> ParseTsMsOutput {
    ParseTsMsOutput {
        ts_ms: parse_ts_ms_runtime(input.value.as_deref().unwrap_or("")),
    }
}

pub fn compute_add_minutes(input: &AddMinutesInput) -> AddMinutesOutput {
    let base = parse_ts_ms_runtime(input.iso_ts.as_deref().unwrap_or(""));
    if base <= 0 {
        return AddMinutesOutput { iso_ts: None };
    }
    let minutes = input.minutes.unwrap_or(0.0).max(0.0);
    let out_ms = base + (minutes * 60.0 * 1000.0) as i64;
    let out = Utc
        .timestamp_millis_opt(out_ms)
        .single()
        .map(|dt| dt.to_rfc3339_opts(SecondsFormat::Millis, true).to_string());
    AddMinutesOutput { iso_ts: out }
}

pub fn compute_clamp_int(input: &ClampIntInput) -> ClampIntOutput {
    let lo = input.lo.unwrap_or(i64::MIN);
    let hi = input.hi.unwrap_or(i64::MAX);
    let fallback = input.fallback.unwrap_or(0);
    ClampIntOutput {
        value: clamp_int_value(input.value.as_ref(), lo, hi, fallback),
    }
}

pub fn compute_clamp_number(input: &ClampNumberInput) -> ClampNumberOutput {
    let lo = input.lo.unwrap_or(f64::NEG_INFINITY);
    let hi = input.hi.unwrap_or(f64::INFINITY);
    let fallback = input.fallback.unwrap_or(0.0);
    let value = parse_number_like(input.value.as_ref()).unwrap_or(fallback);
    ClampNumberOutput {
        value: clamp_number(value, lo, hi),
    }
}

pub fn compute_to_bool(input: &ToBoolInput) -> ToBoolOutput {
    ToBoolOutput {
        value: to_bool_like(input.value.as_ref(), input.fallback.unwrap_or(false)),
    }
}

pub fn compute_clean_text(input: &CleanTextInput) -> CleanTextOutput {
    let max_len = input.max_len.unwrap_or(240).clamp(0, 10000) as usize;
    CleanTextOutput {
        value: clean_text_runtime(input.value.as_deref().unwrap_or(""), max_len),
    }
}

pub fn compute_normalize_token(input: &NormalizeTokenInput) -> NormalizeTokenOutput {
    let max_len = input.max_len.unwrap_or(80).clamp(1, 10000) as usize;
    NormalizeTokenOutput {
        value: normalize_token_runtime(input.value.as_deref().unwrap_or(""), max_len),
    }
}

pub fn compute_normalize_word_token(input: &NormalizeWordTokenInput) -> NormalizeWordTokenOutput {
    let max_len = input.max_len.unwrap_or(80).clamp(1, 10000) as usize;
    let src = clean_text_runtime(input.value.as_deref().unwrap_or(""), max_len).to_lowercase();
    let mut out = String::new();
    let mut prev_underscore = false;
    for ch in src.chars() {
        if ch.is_ascii_lowercase() || ch.is_ascii_digit() {
            out.push(ch);
            prev_underscore = false;
        } else if !prev_underscore {
            out.push('_');
            prev_underscore = true;
        }
    }
    NormalizeWordTokenOutput {
        value: out.trim_matches('_').to_string(),
    }
}

pub fn compute_band_to_index(input: &BandToIndexInput) -> BandToIndexOutput {
    let b = compute_normalize_token(&NormalizeTokenInput {
        value: input.band.clone(),
        max_len: Some(24),
    })
    .value;
    let index = if b == "novice" {
        0
    } else if b == "developing" {
        1
    } else if b == "mature" {
        2
    } else if b == "seasoned" {
        3
    } else {
        4
    };
    BandToIndexOutput { index }
}

pub fn compute_escape_regex(input: &EscapeRegexInput) -> EscapeRegexOutput {
    EscapeRegexOutput {
        value: regex::escape(input.value.as_deref().unwrap_or("")),
    }
}

pub fn compute_pattern_to_word_regex(input: &PatternToWordRegexInput) -> PatternToWordRegexOutput {
    let max_len = input.max_len.unwrap_or(200).clamp(1, 10000) as usize;
    let raw = clean_text_runtime(input.pattern.as_deref().unwrap_or(""), max_len);
    if raw.is_empty() {
        return PatternToWordRegexOutput { source: None };
    }
    let words = raw
        .split_whitespace()
        .map(regex::escape)
        .filter(|row| !row.is_empty())
        .collect::<Vec<_>>();
    if words.is_empty() {
        return PatternToWordRegexOutput { source: None };
    }
    PatternToWordRegexOutput {
        source: Some(format!("\\b{}\\b", words.join("\\s+"))),
    }
}

pub fn compute_stable_id(input: &StableIdInput) -> StableIdOutput {
    let seed = input.seed.as_deref().unwrap_or("");
    let prefix = clean_text_runtime(input.prefix.as_deref().unwrap_or("inv"), 80);
    let safe_prefix = if prefix.is_empty() {
        "inv".to_string()
    } else {
        prefix
    };
    StableIdOutput {
        id: stable_id_runtime(seed, &safe_prefix),
    }
}

pub fn compute_rel_path(input: &RelPathInput) -> RelPathOutput {
    RelPathOutput {
        value: rel_path_runtime(
            input.root.as_deref().unwrap_or(""),
            input.file_path.as_deref().unwrap_or(""),
        ),
    }
}

pub fn compute_normalize_axiom_pattern(
    input: &NormalizeAxiomPatternInput,
) -> NormalizeAxiomPatternOutput {
    NormalizeAxiomPatternOutput {
        value: clean_text_runtime(input.value.as_deref().unwrap_or(""), 200).to_lowercase(),
    }
}

pub fn compute_normalize_axiom_signal_terms(
    input: &NormalizeAxiomSignalTermsInput,
) -> NormalizeAxiomSignalTermsOutput {
    let mut out = input
        .terms
        .iter()
        .map(|row| {
            compute_normalize_axiom_pattern(&NormalizeAxiomPatternInput {
                value: Some(value_to_string(Some(row))),
            })
            .value
        })
        .filter(|row| !row.is_empty())
        .collect::<Vec<_>>();
    out.truncate(32);
    NormalizeAxiomSignalTermsOutput { terms: out }
}

pub fn compute_normalize_observer_id(
    input: &NormalizeObserverIdInput,
) -> NormalizeObserverIdOutput {
    NormalizeObserverIdOutput {
        value: normalize_token_runtime(input.value.as_deref().unwrap_or(""), 120),
    }
}

pub fn compute_extract_numeric(input: &ExtractNumericInput) -> ExtractNumericOutput {
    let value = js_number_for_extract(Some(&input.value)).filter(|n| n.is_finite());
    ExtractNumericOutput { value }
}

pub fn compute_pick_first_numeric(input: &PickFirstNumericInput) -> PickFirstNumericOutput {
    for candidate in &input.candidates {
        let out = compute_extract_numeric(&ExtractNumericInput {
            value: candidate.clone(),
        });
        if out.value.is_some() {
            return PickFirstNumericOutput { value: out.value };
        }
    }
    PickFirstNumericOutput { value: None }
}

pub fn compute_safe_rel_path(input: &SafeRelPathInput) -> SafeRelPathOutput {
    let rel = rel_path_runtime(
        input.root.as_deref().unwrap_or(""),
        input.file_path.as_deref().unwrap_or(""),
    );
    let value = if rel.is_empty() || rel.starts_with("..") {
        normalize_slashes(input.file_path.as_deref().unwrap_or(""))
    } else {
        rel
    };
    SafeRelPathOutput { value }
}

pub fn compute_now_iso(_input: &NowIsoInput) -> NowIsoOutput {
    NowIsoOutput {
        value: now_iso_runtime(),
    }
}

pub fn compute_default_tier_event_map(
    _input: &DefaultTierEventMapInput,
) -> DefaultTierEventMapOutput {
    DefaultTierEventMapOutput {
        map: default_tier_event_map_value(),
    }
}
fn normalize_policy_version_runtime(raw: &str) -> String {
    let mut out = String::new();
    for ch in clean_text_runtime(raw, 24).chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-') {
            out.push(ch);
        }
        if out.len() >= 24 {
            break;
        }
    }
    let trimmed = out.trim_matches(|ch: char| matches!(ch, '.' | '_' | '-'));
    if trimmed.is_empty() {
        "1.0".to_string()
    } else {
        trimmed.to_string()
    }
}
