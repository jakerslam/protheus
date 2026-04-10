#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FallbackRouteClassification {
    pub enabled: bool,
    pub applied: bool,
    pub reason: String,
    pub risk: String,
    pub complexity: String,
    pub role: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RouterBudgetPolicy {
    pub enabled: bool,
    pub state_dir: String,
    pub allow_strategy_override: bool,
    pub soft_ratio: f64,
    pub hard_ratio: f64,
    pub enforce_hard_cap: bool,
    pub escalate_on_no_local_fallback: bool,
    pub cloud_penalty_soft: f64,
    pub cloud_penalty_hard: f64,
    pub cheap_local_bonus_soft: f64,
    pub cheap_local_bonus_hard: f64,
    pub model_token_multipliers: Map<String, Value>,
    pub class_token_multipliers: Map<String, Value>,
}

#[derive(Debug, Clone, Copy)]
pub struct RouterBudgetStateInput<'a> {
    pub cfg: &'a Value,
    pub repo_root: &'a Path,
    pub default_state_dir: &'a str,
    pub today_override: &'a str,
    pub now_iso: &'a str,
    pub budget_state: Option<&'a Value>,
    pub oracle_signal: Option<&'a Value>,
    pub default_oracle_source_path: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouterBudgetAutopauseState {
    pub active: bool,
    pub source: Option<String>,
    pub reason: Option<String>,
    pub until: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RouterGlobalBudgetGateResult {
    pub enabled: bool,
    pub blocked: bool,
    pub deferred: bool,
    pub bypassed: bool,
    pub reason: Option<String>,
    pub autopause_active: bool,
    pub autopause: RouterBudgetAutopauseState,
    pub guard: Option<Value>,
    pub oracle: Option<Value>,
}

#[derive(Debug, Clone, Copy)]
pub struct RouterGlobalBudgetGateInput<'a> {
    pub request_tokens_est: Option<f64>,
    pub dry_run: Option<&'a Value>,
    pub execution_intent: Option<&'a Value>,
    pub enforce_execution_only: bool,
    pub nonexec_max_tokens: i64,
    pub autopause: Option<&'a Value>,
    pub oracle: Option<&'a Value>,
    pub guard: Option<&'a Value>,
}

#[derive(Debug, Clone, Copy)]
pub struct FallbackRouteClassificationInput<'a> {
    pub cfg: &'a Value,
    pub requested_risk: &'a str,
    pub requested_complexity: &'a str,
    pub intent: &'a str,
    pub task: &'a str,
    pub mode: &'a str,
    pub role: &'a str,
    pub tokens_est: Option<f64>,
    pub class_policy: Option<&'a RouteClassPolicy>,
}

fn js_truthy_value(value: &Value) -> bool {
    js_truthy(Some(value))
}

fn first_truthy_value<'a>(candidates: &[Option<&'a Value>]) -> Option<&'a Value> {
    candidates
        .iter()
        .flatten()
        .copied()
        .find(|value| js_truthy_value(value))
}

fn object_or_empty(value: Option<&Value>) -> Map<String, Value> {
    value
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_else(Map::new)
}

fn number_or_default(value: Option<&Value>, fallback: i64) -> i64 {
    finite_number(value).map_or(fallback, |v| v as i64)
}

fn js_number_with_or_zero(value: Option<&Value>) -> f64 {
    if !js_truthy(value) {
        return 0.0;
    }
    finite_number(value).unwrap_or(f64::NAN)
}

fn positive_multiplier(value: Option<&Value>) -> Option<f64> {
    let multiplier = finite_number(value).unwrap_or(f64::NAN);
    if multiplier.is_finite() && multiplier > 0.0 {
        Some(multiplier)
    } else {
        None
    }
}

pub fn resolve_model_token_multiplier(
    model_id: &str,
    profile_class: &str,
    policy: &Value,
) -> ModelTokenMultiplier {
    let key = normalize_key(model_id);
    let by_model = policy
        .as_object()
        .and_then(|obj| obj.get("model_token_multipliers"))
        .and_then(Value::as_object);

    if let Some(by_model_map) = by_model {
        for (model, raw_multiplier) in by_model_map {
            if normalize_key(model) != key {
                continue;
            }
            if let Some(multiplier) = positive_multiplier(Some(raw_multiplier)) {
                return ModelTokenMultiplier {
                    multiplier,
                    source: "model",
                };
            }
        }
    }

    let class_multipliers = policy
        .as_object()
        .and_then(|obj| obj.get("class_token_multipliers"))
        .and_then(Value::as_object);
    let class_key = normalize_key(profile_class);
    let fallback_class = if is_local_ollama_model(model_id) {
        "local"
    } else {
        "cloud"
    };
    let selected = class_multipliers.and_then(|map| {
        first_truthy_value(&[
            map.get(&class_key),
            map.get(fallback_class),
            map.get("default"),
        ])
    });
    if let Some(class_value) = positive_multiplier(selected) {
        return ModelTokenMultiplier {
            multiplier: class_value,
            source: "class",
        };
    }

    ModelTokenMultiplier {
        multiplier: 1.0,
        source: "default",
    }
}

pub fn estimate_model_request_tokens(
    model_id: &str,
    request_tokens: Option<f64>,
    profile_class: &str,
    policy: &Value,
) -> ModelTokenEstimate {
    let req = request_tokens.unwrap_or(f64::NAN);
    if !req.is_finite() || req <= 0.0 {
        return ModelTokenEstimate {
            tokens_est: None,
            multiplier: None,
            source: "none",
        };
    }

    let detail = resolve_model_token_multiplier(model_id, profile_class, policy);
    let est = clamp_request_tokens((req * detail.multiplier).round() as i64);
    let rounded_multiplier = ((detail.multiplier * 10_000.0).round()) / 10_000.0;
    ModelTokenEstimate {
        tokens_est: Some(est),
        multiplier: Some(rounded_multiplier),
        source: detail.source,
    }
}

pub fn normalize_probe_blocked_record(rec: Option<&Value>) -> ProbeBlockedNormalization {
    let mut row = match rec.and_then(Value::as_object).cloned() {
        Some(value) => value,
        None => {
            return ProbeBlockedNormalization {
                rec: None,
                changed: false,
            };
        }
    };

    let txt = format!(
        "{} {}",
        row.get("reason")
            .and_then(Value::as_str)
            .unwrap_or_default(),
        row.get("stderr")
            .and_then(Value::as_str)
            .unwrap_or_default()
    );
    let blocked = matches!(row.get("probe_blocked"), Some(Value::Bool(true)))
        || is_env_probe_blocked_text(&txt);
    if !blocked {
        return ProbeBlockedNormalization {
            rec: Some(Value::Object(row)),
            changed: false,
        };
    }

    let mut changed = false;
    if !matches!(row.get("probe_blocked"), Some(Value::Bool(true))) {
        row.insert("probe_blocked".to_string(), Value::Bool(true));
        changed = true;
    }
    if !matches!(row.get("reason"), Some(Value::String(reason)) if reason == "env_probe_blocked") {
        row.insert(
            "reason".to_string(),
            Value::String("env_probe_blocked".to_string()),
        );
        changed = true;
    }
    if !matches!(row.get("available"), Some(Value::Null)) {
        row.insert("available".to_string(), Value::Null);
        changed = true;
    }

    ProbeBlockedNormalization {
        rec: Some(Value::Object(row)),
        changed,
    }
}

pub fn suppression_active(rec: Option<&Value>, now_ms: i64) -> bool {
    let until = js_number_with_or_zero(
        rec.and_then(Value::as_object)
            .and_then(|row| row.get("suppressed_until_ms")),
    );
    until.is_finite() && until > now_ms as f64
}

pub fn apply_probe_health_stabilizer(
    previous: Option<&Value>,
    current: Option<&Value>,
    now_ms: i64,
    policy: &ProbeHealthStabilizerPolicy,
) -> Value {
    let prev = object_or_empty(previous);
    let mut rec = object_or_empty(current);

    let prev_timeout_streak = number_or_default(prev.get("timeout_streak"), 0);
    let timeout_streak = if matches!(rec.get("timeout"), Some(Value::Bool(true))) {
        prev_timeout_streak + 1
    } else {
        0
    };
    rec.insert(
        "timeout_streak".to_string(),
        Value::Number(serde_json::Number::from(timeout_streak)),
    );

    let prev_rehab_success = number_or_default(prev.get("rehab_success_streak"), 0).max(0);
    let rehab_success_streak = if matches!(rec.get("timeout"), Some(Value::Bool(true))) {
        0
    } else if matches!(rec.get("available"), Some(Value::Bool(true))) {
        prev_rehab_success + 1
    } else {
        prev_rehab_success
    };
    rec.insert(
        "rehab_success_streak".to_string(),
        Value::Number(serde_json::Number::from(rehab_success_streak)),
    );

    if policy.suppression_enabled
        && matches!(rec.get("timeout"), Some(Value::Bool(true)))
        && timeout_streak >= policy.suppression_timeout_streak
    {
        let until = now_ms + (policy.suppression_minutes * 60 * 1000);
        rec.insert(
            "suppressed_until_ms".to_string(),
            Value::Number(serde_json::Number::from(until)),
        );
        rec.insert(
            "suppressed_reason".to_string(),
            Value::String("timeout_streak".to_string()),
        );
        rec.insert("available".to_string(), Value::Bool(false));
    }

    if matches!(rec.get("available"), Some(Value::Bool(true))) {
        let prev_suppressed_until = js_number_with_or_zero(prev.get("suppressed_until_ms"));
        if rehab_success_streak >= policy.rehab_success_threshold
            || (prev_suppressed_until > 0.0 && prev_suppressed_until <= now_ms as f64)
        {
            rec.remove("suppressed_until_ms");
            rec.remove("suppressed_reason");
            rec.remove("suppressed_at_ms");
        }
    }

    if suppression_active(Some(&Value::Object(rec.clone())), now_ms) {
        let existing = rec.get("suppressed_at_ms");
        let suppressed_at = if js_truthy(existing) {
            finite_number(existing).unwrap_or(now_ms as f64)
        } else {
            now_ms as f64
        };
        let suppressed_at_number = serde_json::Number::from_f64(suppressed_at)
            .unwrap_or_else(|| serde_json::Number::from(now_ms));
        rec.insert(
            "suppressed_at_ms".to_string(),
            Value::Number(suppressed_at_number),
        );
        rec.insert(
            "reason".to_string(),
            Value::String("probe_suppressed_timeout_rehab".to_string()),
        );
        rec.insert("available".to_string(), Value::Bool(false));
    }

    Value::Object(rec)
}

pub fn communication_fast_path_policy(cfg: &Value) -> CommunicationFastPathPolicy {
    let src = cfg
        .as_object()
        .and_then(|v| v.get("routing"))
        .and_then(Value::as_object)
        .and_then(|v| v.get("communication_fast_path"))
        .and_then(Value::as_object);

    let patterns = value_string_array(src.and_then(|v| v.get("patterns")));
    let disallow_regexes = value_string_array(src.and_then(|v| v.get("disallow_regexes")));
    let disallow_regexes = if disallow_regexes.is_empty() {
        DEFAULT_FAST_PATH_DISALLOW_REGEXES
            .iter()
            .map(|row| row.to_string())
            .collect::<Vec<_>>()
    } else {
        disallow_regexes
    };

    CommunicationFastPathPolicy {
        enabled: to_bool_like_value(src.and_then(|v| v.get("enabled")), true),
        match_mode: string_or(src.and_then(|v| v.get("match_mode")), "heuristic"),
        max_chars: to_bounded_number_like(src.and_then(|v| v.get("max_chars")), 48, 8, 220),
        max_words: to_bounded_number_like(src.and_then(|v| v.get("max_words")), 8, 1, 32),
        max_newlines: to_bounded_number_like(src.and_then(|v| v.get("max_newlines")), 0, 0, 8),
        patterns,
        disallow_regexes,
        slot: string_or(src.and_then(|v| v.get("slot")), "grunt"),
        prefer_model: string_or(
            src.and_then(|v| v.get("prefer_model")),
            "ollama/smallthinker",
        ),
        fallback_slot: string_or(src.and_then(|v| v.get("fallback_slot")), "fallback"),
        skip_outcome_scan: to_bool_like_value(src.and_then(|v| v.get("skip_outcome_scan")), true),
    }
}
