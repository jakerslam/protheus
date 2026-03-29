pub fn compute_directive_pulse_stats(
    input: &DirectivePulseStatsInput,
) -> DirectivePulseStatsOutput {
    let date_str = input
        .date_str
        .as_ref()
        .map(|v| v.trim().to_string())
        .unwrap_or_default();

    let mut tier_attempts_today = std::collections::BTreeMap::<String, f64>::new();
    let mut attempts_today = 0.0_f64;
    let mut objective_stats_by_id =
        std::collections::BTreeMap::<String, DirectivePulseContextObjectiveStatOutput>::new();

    for evt in &input.events {
        let event_type = evt
            .event_type
            .as_ref()
            .map(|v| v.trim().to_string())
            .unwrap_or_default();
        let result = evt
            .result
            .as_ref()
            .map(|v| v.trim().to_string())
            .unwrap_or_default();
        let outcome = evt
            .outcome
            .as_ref()
            .map(|v| v.trim().to_string())
            .unwrap_or_default();
        let day = evt
            .day
            .as_ref()
            .map(|v| v.trim().to_string())
            .unwrap_or_default();
        let objective_id = evt
            .objective_id
            .as_ref()
            .map(|v| v.trim().to_string())
            .unwrap_or_default();
        let pulse_tier = compute_normalize_directive_tier(&NormalizeDirectiveTierInput {
            raw_tier: evt.tier,
            fallback: Some(3.0),
        })
        .tier;

        let is_attempt = compute_attempt_run_event(&AttemptRunEventInput {
            event_type: Some(event_type.clone()),
            result: Some(result.clone()),
        })
        .is_attempt;
        if !is_attempt {
            continue;
        }

        if !date_str.is_empty() && day == date_str {
            let tier_key = pulse_tier.to_string();
            let next = tier_attempts_today.get(&tier_key).copied().unwrap_or(0.0) + 1.0;
            tier_attempts_today.insert(tier_key, next);
            attempts_today += 1.0;
        }

        if objective_id.is_empty() {
            continue;
        }

        let is_no_progress = compute_no_progress_result(&NoProgressResultInput {
            event_type: Some(event_type),
            result: Some(result.clone()),
            outcome: Some(outcome.clone()),
        })
        .is_no_progress;

        let entry = objective_stats_by_id
            .entry(objective_id.clone())
            .or_insert_with(|| DirectivePulseContextObjectiveStatOutput {
                objective_id: objective_id.clone(),
                tier: pulse_tier,
                attempts: 0,
                shipped: 0,
                no_change: 0,
                reverted: 0,
                no_progress_streak: 0,
                last_attempt_ts: None,
                last_shipped_ts: None,
            });

        entry.attempts += 1;
        entry.tier = pulse_tier;
        if let Some(ts) = evt
            .ts
            .as_ref()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
        {
            entry.last_attempt_ts = Some(ts.clone());
        }

        let shipped = result == "executed" && outcome == "shipped";
        if shipped {
            entry.shipped += 1;
            entry.no_progress_streak = 0;
            if let Some(ts) = evt
                .ts
                .as_ref()
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
            {
                entry.last_shipped_ts = Some(ts);
            }
        } else {
            if result == "executed" && outcome == "no_change" {
                entry.no_change += 1;
            }
            if result == "executed" && outcome == "reverted" {
                entry.reverted += 1;
            }
            if is_no_progress {
                entry.no_progress_streak += 1;
            }
        }
    }

    DirectivePulseStatsOutput {
        tier_attempts_today,
        attempts_today,
        objective_stats: objective_stats_by_id.into_values().collect(),
    }
}

fn json_path<'a>(root: &'a serde_json::Value, path: &[&str]) -> Option<&'a serde_json::Value> {
    let mut current = root;
    for key in path {
        current = current.as_object()?.get(*key)?;
    }
    Some(current)
}

fn js_like_string(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => String::new(),
        serde_json::Value::Bool(v) => {
            if *v {
                "true".to_string()
            } else {
                "false".to_string()
            }
        }
        serde_json::Value::Number(v) => v.to_string(),
        serde_json::Value::String(v) => v.clone(),
        serde_json::Value::Array(values) => values
            .iter()
            .map(js_like_string)
            .collect::<Vec<_>>()
            .join(","),
        serde_json::Value::Object(_) => "[object Object]".to_string(),
    }
}

fn js_like_string_array(value: Option<&serde_json::Value>) -> Vec<String> {
    match value {
        Some(serde_json::Value::Array(rows)) => rows
            .iter()
            .map(js_like_string)
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .collect(),
        Some(serde_json::Value::String(v)) if !v.trim().is_empty() => vec![v.trim().to_string()],
        _ => Vec::new(),
    }
}

fn js_array_to_strings(value: Option<&serde_json::Value>) -> Vec<String> {
    match value {
        Some(serde_json::Value::Array(rows)) => rows
            .iter()
            .map(js_like_string)
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .collect(),
        _ => Vec::new(),
    }
}

fn js_like_number(value: Option<&serde_json::Value>) -> Option<f64> {
    let v = value?;
    match v {
        serde_json::Value::Number(n) => n.as_f64(),
        serde_json::Value::String(s) => s.trim().parse::<f64>().ok(),
        serde_json::Value::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
        serde_json::Value::Null => None,
        _ => None,
    }
}

pub fn compute_compile_directive_pulse_objectives(
    input: &CompileDirectivePulseObjectivesInput,
) -> CompileDirectivePulseObjectivesOutput {
    let mut out = Vec::<CompileDirectivePulseObjectiveOutput>::new();
    let mut seen = std::collections::BTreeSet::<String>::new();

    for directive in &input.directives {
        let id_from_metadata =
            json_path(directive, &["data", "metadata", "id"]).map(js_like_string);
        let id_from_root = json_path(directive, &["id"]).map(js_like_string);
        let id = id_from_metadata
            .or(id_from_root)
            .map(|v| v.trim().to_string())
            .unwrap_or_default();
        if id.is_empty() || !seen.insert(id.clone()) {
            continue;
        }
        let id_upper = id.to_ascii_uppercase();
        if id_upper == "T0" || id_upper.starts_with("T0_") || id_upper.starts_with("T0-") {
            continue;
        }

        let tier_raw = js_like_number(json_path(directive, &["tier"])).or(js_like_number(
            json_path(directive, &["data", "metadata", "tier"]),
        ));
        let tier = compute_normalize_directive_tier(&NormalizeDirectiveTierInput {
            raw_tier: tier_raw,
            fallback: Some(3.0),
        })
        .tier;

        let mut phrases_raw = Vec::<String>::new();
        phrases_raw.extend(js_like_string_array(json_path(
            directive,
            &["data", "metadata", "description"],
        )));
        phrases_raw.extend(js_like_string_array(json_path(
            directive,
            &["data", "intent", "primary"],
        )));
        phrases_raw.extend(js_like_string_array(json_path(
            directive,
            &["data", "scope", "included"],
        )));
        phrases_raw.extend(js_like_string_array(json_path(
            directive,
            &["data", "success_metrics", "leading"],
        )));
        phrases_raw.extend(js_like_string_array(json_path(
            directive,
            &["data", "success_metrics", "lagging"],
        )));

        let mut phrase_set = std::collections::BTreeSet::<String>::new();
        for phrase in &phrases_raw {
            let normalized = compute_normalize_directive_text(&NormalizeDirectiveTextInput {
                text: Some(phrase.clone()),
            })
            .normalized;
            if !normalized.is_empty() && normalized.len() >= 6 {
                phrase_set.insert(normalized);
            }
        }
        let mut phrases = phrase_set.into_iter().collect::<Vec<_>>();
        if phrases.len() > 16 {
            phrases.truncate(16);
        }

        let mut token_set = std::collections::BTreeSet::<String>::new();
        for phrase in &phrases {
            let tokens = compute_tokenize_directive_text(&TokenizeDirectiveTextInput {
                text: Some(phrase.clone()),
                stopwords: input.stopwords.clone(),
            })
            .tokens;
            for token in tokens {
                let clean = token.trim();
                if !clean.is_empty() {
                    token_set.insert(clean.to_string());
                }
            }
        }
        let mut tokens = token_set.into_iter().collect::<Vec<_>>();
        if tokens.len() > 64 {
            tokens.truncate(64);
        }

        let mut explicit_rows = Vec::<String>::new();
        explicit_rows.extend(js_like_string_array(json_path(
            directive,
            &["data", "metadata", "value_currency"],
        )));
        explicit_rows.extend(js_like_string_array(json_path(
            directive,
            &["data", "metadata", "value_currencies"],
        )));
        explicit_rows.extend(js_like_string_array(json_path(
            directive,
            &["data", "value_currency"],
        )));
        explicit_rows.extend(js_like_string_array(json_path(
            directive,
            &["data", "value_currencies"],
        )));
        explicit_rows.extend(js_like_string_array(json_path(
            directive,
            &["data", "intent", "value_currency"],
        )));
        explicit_rows.extend(js_like_string_array(json_path(
            directive,
            &["data", "intent", "value_currencies"],
        )));

        let explicit_currencies = compute_list_value_currencies(&ListValueCurrenciesInput {
            value_list: explicit_rows,
            value_csv: None,
            allowed_keys: input.allowed_value_keys.clone(),
        })
        .currencies;

        let mut inference_bits = Vec::<String>::new();
        inference_bits.push(id.clone());
        inference_bits.extend(phrases_raw.iter().cloned());
        inference_bits.extend(phrases.iter().cloned());
        inference_bits.extend(tokens.iter().cloned());

        let inferred_currencies = compute_infer_value_currencies_from_directive_bits(
            &InferValueCurrenciesFromDirectiveBitsInput {
                bits: inference_bits,
                allowed_keys: input.allowed_value_keys.clone(),
            },
        )
        .currencies;

        let value_currencies = if explicit_currencies.is_empty() {
            inferred_currencies
        } else {
            let mut merged = explicit_currencies;
            merged.extend(inferred_currencies);
            compute_list_value_currencies(&ListValueCurrenciesInput {
                value_list: merged,
                value_csv: None,
                allowed_keys: input.allowed_value_keys.clone(),
            })
            .currencies
        };
        let primary_currency = value_currencies.first().cloned();

        let title_primary =
            js_like_string_array(json_path(directive, &["data", "intent", "primary"]));
        let title_description =
            js_like_string_array(json_path(directive, &["data", "metadata", "description"]));
        let title = title_primary
            .first()
            .cloned()
            .or_else(|| title_description.first().cloned())
            .unwrap_or_else(|| id.clone());

        let tier_weight = compute_directive_tier_weight(&DirectiveTierWeightInput {
            tier: Some(tier as f64),
            fallback: Some(3.0),
        })
        .weight;
        let min_share = compute_directive_tier_min_share(&DirectiveTierMinShareInput {
            tier: Some(tier as f64),
            fallback: Some(3.0),
            t1_min_share: input.t1_min_share.unwrap_or(0.5),
            t2_min_share: input.t2_min_share.unwrap_or(0.25),
        })
        .min_share;

        out.push(CompileDirectivePulseObjectiveOutput {
            id,
            tier,
            title,
            tier_weight,
            min_share,
            phrases,
            tokens,
            value_currencies,
            primary_currency,
        });
    }

    out.sort_by(|a, b| a.tier.cmp(&b.tier).then_with(|| a.id.cmp(&b.id)));
    CompileDirectivePulseObjectivesOutput { objectives: out }
}

pub fn compute_directive_pulse_objectives_profile(
    input: &DirectivePulseObjectivesProfileInput,
) -> DirectivePulseObjectivesProfileOutput {
    if !input.enabled {
        return DirectivePulseObjectivesProfileOutput {
            enabled: false,
            available: false,
            objectives: Vec::new(),
            error: Some("directive_pulse_disabled".to_string()),
        };
    }
    let load_error = input
        .load_error
        .as_ref()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .map(|v| v.chars().take(200).collect::<String>());
    if let Some(err) = load_error {
        return DirectivePulseObjectivesProfileOutput {
            enabled: true,
            available: false,
            objectives: Vec::new(),
            error: Some(err),
        };
    }
    let objectives = input.objectives.clone();
    let available = !objectives.is_empty();
    DirectivePulseObjectivesProfileOutput {
        enabled: true,
        available,
        objectives,
        error: if available {
            None
        } else {
            Some("no_objectives".to_string())
        },
    }
}
