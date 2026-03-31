pub fn detect_communication_fast_path(
    cfg: &Value,
    risk: &str,
    complexity: &str,
    intent: &str,
    task: &str,
    mode: &str,
    allow_generic_medium: bool,
) -> CommunicationFastPathResult {
    let policy = communication_fast_path_policy(cfg);

    let make_nomatch =
        |reason: &str, blocked_pattern: Option<String>| CommunicationFastPathResult {
            matched: false,
            reason: reason.to_string(),
            policy: policy.clone(),
            blocked_pattern,
            matched_pattern: None,
            text: None,
            slot: None,
            prefer_model: None,
            fallback_slot: None,
            skip_outcome_scan: None,
        };

    if !policy.enabled {
        return make_nomatch("disabled", None);
    }

    let m = normalize_key(if mode.is_empty() { "normal" } else { mode });
    if m == "deep-thinker" || m == "deep_thinker" || m == "hyper-creative" || m == "hyper_creative"
    {
        return make_nomatch("mode_disallowed", None);
    }

    if !allow_generic_medium {
        if normalize_key(risk) != "low" {
            return make_nomatch("risk_not_low", None);
        }
        let cx = normalize_key(if complexity.is_empty() {
            "medium"
        } else {
            complexity
        });
        if !(cx == "low" || cx == "medium") {
            return make_nomatch("complexity_not_eligible", None);
        }
    }

    let raw_text = if !task.is_empty() { task } else { intent }.to_string();
    let newline_count = raw_text.matches('\n').count() as i64;
    if newline_count > policy.max_newlines {
        return make_nomatch("too_many_newlines", None);
    }

    let text = raw_text.split_whitespace().collect::<Vec<_>>().join(" ");
    if text.is_empty() {
        return make_nomatch("empty_text", None);
    }

    let words = text.split(' ').filter(|row| !row.is_empty()).count() as i64;
    if text.len() as i64 > policy.max_chars {
        return make_nomatch("text_too_long", None);
    }
    if words > policy.max_words {
        return make_nomatch("word_count_too_high", None);
    }

    for raw in &policy.disallow_regexes {
        if pattern_match_ci(raw, &text, &raw_text) {
            return make_nomatch("contains_structured_intent", Some(raw.clone()));
        }
    }

    let structural_role = infer_role(&text, &text);
    if matches!(
        normalize_key(&structural_role).as_str(),
        "coding" | "tools" | "swarm" | "planning" | "logic"
    ) {
        return make_nomatch("role_not_chat_like", None);
    }

    let match_mode = normalize_key(&policy.match_mode);
    if match_mode == "patterns" {
        for raw in &policy.patterns {
            if pattern_match_ci(raw, &text, &raw_text) {
                return CommunicationFastPathResult {
                    matched: true,
                    reason: "communication_fast_path_pattern".to_string(),
                    policy: policy.clone(),
                    blocked_pattern: None,
                    matched_pattern: Some(raw.clone()),
                    text: Some(text),
                    slot: Some(policy.slot.clone()),
                    prefer_model: Some(policy.prefer_model.clone()),
                    fallback_slot: Some(policy.fallback_slot.clone()),
                    skip_outcome_scan: Some(policy.skip_outcome_scan),
                };
            }
        }
        return make_nomatch("no_pattern_match", None);
    }

    CommunicationFastPathResult {
        matched: true,
        reason: "communication_fast_path_heuristic".to_string(),
        policy: policy.clone(),
        blocked_pattern: None,
        matched_pattern: None,
        text: Some(text),
        slot: Some(policy.slot.clone()),
        prefer_model: Some(policy.prefer_model.clone()),
        fallback_slot: Some(policy.fallback_slot.clone()),
        skip_outcome_scan: Some(policy.skip_outcome_scan),
    }
}

fn normalized_optional_string(value: Option<&Value>) -> Option<String> {
    value
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(ToString::to_string)
}

fn contains_code_like_markers(raw_text: &str) -> bool {
    if raw_text.contains("```") {
        return true;
    }
    if raw_text.chars().any(|ch| {
        matches!(
            ch,
            '`' | '{' | '}' | '[' | ']' | '<' | '>' | '$' | ';' | '='
        )
    }) {
        return true;
    }
    if contains_cli_flag(raw_text) {
        return true;
    }
    let tokens = tokenize(raw_text);
    [
        "node", "npm", "pnpm", "yarn", "git", "curl", "python", "bash", "zsh", "ollama",
    ]
    .iter()
    .any(|token| tokens.contains(*token))
}

pub fn fallback_classification_policy(cfg: &Value) -> FallbackClassificationPolicy {
    let src = cfg
        .as_object()
        .and_then(|v| v.get("routing"))
        .and_then(Value::as_object)
        .and_then(|v| v.get("fallback_classification_policy"))
        .and_then(Value::as_object);

    FallbackClassificationPolicy {
        enabled: to_bool_like_value(src.and_then(|v| v.get("enabled")), true),
        only_when_medium_medium: to_bool_like_value(
            src.and_then(|v| v.get("only_when_medium_medium")),
            true,
        ),
        prefer_chat_fast_path: to_bool_like_value(
            src.and_then(|v| v.get("prefer_chat_fast_path")),
            true,
        ),
        low_chars_max: to_bounded_number_like_f64(
            src.and_then(|v| v.get("low_chars_max")),
            220.0,
            32.0,
            600.0,
        ),
        low_newlines_max: to_bounded_number_like_f64(
            src.and_then(|v| v.get("low_newlines_max")),
            1.0,
            0.0,
            6.0,
        ),
        high_chars_min: to_bounded_number_like_f64(
            src.and_then(|v| v.get("high_chars_min")),
            1200.0,
            240.0,
            12_000.0,
        ),
        high_newlines_min: to_bounded_number_like_f64(
            src.and_then(|v| v.get("high_newlines_min")),
            8.0,
            1.0,
            80.0,
        ),
        high_tokens_min: to_bounded_number_like_f64(
            src.and_then(|v| v.get("high_tokens_min")),
            2200.0,
            200.0,
            30_000.0,
        ),
    }
}

pub fn fallback_route_classification(
    input: FallbackRouteClassificationInput<'_>,
) -> FallbackRouteClassification {
    let policy = fallback_classification_policy(input.cfg);
    let base_risk = normalize_risk_level(input.requested_risk);
    let base_complexity = normalize_complexity_level(input.requested_complexity);
    let fallback = FallbackRouteClassification {
        enabled: policy.enabled,
        applied: false,
        reason: "disabled".to_string(),
        risk: base_risk.clone(),
        complexity: base_complexity.clone(),
        role: {
            let role_key = normalize_key(if input.role.is_empty() {
                "general"
            } else {
                input.role
            });
            if role_key.is_empty() {
                "general".to_string()
            } else {
                role_key
            }
        },
    };
    if !policy.enabled {
        return fallback;
    }
    if let Some(class_policy) = input.class_policy {
        if class_policy.force_risk.is_some()
            || class_policy.force_complexity.is_some()
            || !class_policy.force_role.is_empty()
        {
            return FallbackRouteClassification {
                reason: "route_class_forced".to_string(),
                ..fallback
            };
        }
    }
    if policy.only_when_medium_medium && !(base_risk == "medium" && base_complexity == "medium") {
        return FallbackRouteClassification {
            reason: "not_generic_medium".to_string(),
            ..fallback
        };
    }

    let inferred_role = {
        let candidate = if input.role.is_empty() {
            infer_role(input.intent, input.task)
        } else {
            input.role.to_string()
        };
        let normalized = normalize_key(&candidate);
        if normalized.is_empty() {
            "general".to_string()
        } else {
            normalized
        }
    };

    let raw_text = format!("{} {}", input.intent, input.task);
    let raw_text = raw_text.trim().to_string();
    let char_count = raw_text.chars().count() as f64;
    let newline_count = input.task.matches('\n').count() as f64;
    let code_like = contains_code_like_markers(&raw_text);
    let token_count = input.tokens_est.filter(|value| value.is_finite());

    if policy.prefer_chat_fast_path {
        let candidate = detect_communication_fast_path(
            input.cfg,
            &base_risk,
            &base_complexity,
            input.intent,
            input.task,
            input.mode,
            true,
        );
        if candidate.matched {
            return FallbackRouteClassification {
                enabled: fallback.enabled,
                applied: true,
                reason: "generic_medium_fast_path".to_string(),
                risk: "low".to_string(),
                complexity: "low".to_string(),
                role: "chat".to_string(),
            };
        }
    }

    if token_count
        .map(|value| value >= policy.high_tokens_min)
        .unwrap_or(false)
        || char_count >= policy.high_chars_min
        || newline_count >= policy.high_newlines_min
    {
        return FallbackRouteClassification {
            enabled: fallback.enabled,
            applied: true,
            reason: "generic_medium_complexity_escalation".to_string(),
            risk: "medium".to_string(),
            complexity: "high".to_string(),
            role: if inferred_role == "chat" {
                "general".to_string()
            } else {
                inferred_role
            },
        };
    }

    if !code_like
        && char_count <= policy.low_chars_max
        && newline_count <= policy.low_newlines_max
        && (inferred_role == "chat" || inferred_role == "general")
    {
        return FallbackRouteClassification {
            enabled: fallback.enabled,
            applied: true,
            reason: "generic_medium_short_text".to_string(),
            risk: "low".to_string(),
            complexity: "low".to_string(),
            role: "chat".to_string(),
        };
    }

    FallbackRouteClassification {
        reason: "no_override".to_string(),
        ..fallback
    }
}

fn is_budget_date(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes[..4].iter().all(|ch| ch.is_ascii_digit())
        && bytes[5..7].iter().all(|ch| ch.is_ascii_digit())
        && bytes[8..10].iter().all(|ch| ch.is_ascii_digit())
}

fn default_class_token_multipliers() -> Map<String, Value> {
    let mut out = Map::<String, Value>::new();
    out.insert("cheap_local".to_string(), json!(0.42));
    out.insert("local".to_string(), json!(0.55));
    out.insert("cloud_anchor".to_string(), json!(1.15));
    out.insert("cloud_specialist".to_string(), json!(1.35));
    out.insert("cloud".to_string(), json!(1.2));
    out.insert("default".to_string(), json!(1.0));
    out
}

fn rounded_4(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
}

fn number_value(value: f64) -> Value {
    serde_json::Number::from_f64(value)
        .map(Value::Number)
        .unwrap_or(Value::Null)
}

pub fn router_budget_policy(
    cfg: &Value,
    repo_root: &Path,
    default_state_dir: &str,
) -> RouterBudgetPolicy {
    let src = cfg
        .as_object()
        .and_then(|v| v.get("routing"))
        .and_then(Value::as_object)
        .and_then(|v| v.get("router_budget_policy"))
        .and_then(Value::as_object);

    let dir_raw = string_or(src.and_then(|v| v.get("state_dir")), default_state_dir);
    let state_dir = {
        let path = Path::new(&dir_raw);
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            repo_root.join(path)
        }
    };

    let model_token_multipliers =
        object_or_empty(src.and_then(|v| v.get("model_token_multipliers")));
    let class_token_source = object_or_empty(src.and_then(|v| v.get("class_token_multipliers")));
    let mut class_token_multipliers = default_class_token_multipliers();
    for (key, value) in class_token_source {
        class_token_multipliers.insert(key, value);
    }

    RouterBudgetPolicy {
        enabled: to_bool_like_value(src.and_then(|v| v.get("enabled")), true),
        state_dir: state_dir.to_string_lossy().to_string(),
        allow_strategy_override: to_bool_like_value(
            src.and_then(|v| v.get("allow_strategy_override")),
            true,
        ),
        soft_ratio: to_bounded_number_like_f64(
            src.and_then(|v| v.get("soft_ratio")),
            0.75,
            0.2,
            0.98,
        ),
        hard_ratio: to_bounded_number_like_f64(
            src.and_then(|v| v.get("hard_ratio")),
            0.92,
            0.3,
            0.995,
        ),
        enforce_hard_cap: to_bool_like_value(src.and_then(|v| v.get("enforce_hard_cap")), true),
        escalate_on_no_local_fallback: to_bool_like_value(
            src.and_then(|v| v.get("escalate_on_no_local_fallback")),
            true,
        ),
        cloud_penalty_soft: to_bounded_number_like_f64(
            src.and_then(|v| v.get("cloud_penalty_soft")),
            4.0,
            0.0,
            40.0,
        ),
        cloud_penalty_hard: to_bounded_number_like_f64(
            src.and_then(|v| v.get("cloud_penalty_hard")),
            10.0,
            0.0,
            60.0,
        ),
        cheap_local_bonus_soft: to_bounded_number_like_f64(
            src.and_then(|v| v.get("cheap_local_bonus_soft")),
            3.0,
            0.0,
            40.0,
        ),
        cheap_local_bonus_hard: to_bounded_number_like_f64(
            src.and_then(|v| v.get("cheap_local_bonus_hard")),
            7.0,
            0.0,
            60.0,
        ),
        model_token_multipliers,
        class_token_multipliers,
    }
}

pub fn budget_date_str(today_override: &str, now_iso: &str) -> String {
    if is_budget_date(today_override) {
        return today_override.to_string();
    }
    now_iso.chars().take(10).collect::<String>()
}

