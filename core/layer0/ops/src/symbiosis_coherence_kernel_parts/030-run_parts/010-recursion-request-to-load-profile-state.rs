fn recursion_request(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let signal = if let Some(signal) = payload.get("signal") {
        signal.clone()
    } else {
        load_signal(root, payload)?
    };
    let (requested_depth, parsed_unbounded) = parse_depth_request(
        payload
            .get("requested_depth")
            .or_else(|| payload.get("requestedDepth")),
    );
    let require_unbounded = bool_value(payload.get("require_unbounded"), false) || parsed_unbounded;
    let allowed_depth = signal
        .get("recursion_gate")
        .and_then(|v| v.get("allowed_depth"))
        .and_then(|v| {
            if v.is_null() {
                None
            } else {
                v.as_i64()
                    .or_else(|| v.as_u64().and_then(|u| i64::try_from(u).ok()))
            }
        });
    let unbounded_allowed = signal
        .get("recursion_gate")
        .and_then(|v| v.get("unbounded_allowed"))
        .and_then(Value::as_bool)
        .unwrap_or(false);

    let mut reasons = Vec::new();
    let mut blocked = false;
    if signal.get("available").and_then(Value::as_bool) != Some(true) {
        reasons.push(Value::String("symbiosis_signal_unavailable".to_string()));
    } else {
        if require_unbounded && !unbounded_allowed {
            blocked = true;
            reasons.push(Value::String("symbiosis_unbounded_not_allowed".to_string()));
        }
        if let (Some(requested), Some(allowed)) = (requested_depth, allowed_depth) {
            if requested > allowed {
                blocked = true;
                reasons.push(Value::String("symbiosis_depth_exceeds_allowed".to_string()));
            }
        }
    }

    let shadow_only = if payload.contains_key("shadow_only_override") {
        bool_value(payload.get("shadow_only_override"), true)
    } else {
        signal
            .get("shadow_only")
            .and_then(Value::as_bool)
            .unwrap_or(true)
    };
    let blocked_hard = blocked && !shadow_only;

    Ok(json!({
        "ok": !blocked_hard,
        "available": signal.get("available").and_then(Value::as_bool).unwrap_or(false),
        "blocked": blocked,
        "blocked_hard": blocked_hard,
        "shadow_violation": blocked && shadow_only,
        "shadow_only": shadow_only,
        "reason_codes": reasons,
        "requested_depth": requested_depth,
        "requested_unbounded": require_unbounded,
        "allowed_depth": allowed_depth,
        "unbounded_allowed": unbounded_allowed,
        "coherence_score": signal.get("coherence_score").and_then(Value::as_f64),
        "coherence_tier": signal.get("coherence_tier").cloned().unwrap_or(Value::Null),
        "sustained_high_samples": signal
            .get("recursion_gate")
            .and_then(|v| v.get("sustained_high_samples"))
            .and_then(Value::as_i64),
        "latest_path_rel": signal.get("latest_path_rel").cloned().unwrap_or_else(|| {
            signal
                .get("source_paths")
                .and_then(|v| v.get("latest_path"))
                .cloned()
                .unwrap_or(Value::Null)
        })
    }))
}

fn profile_state_path(root: &Path, payload: &Map<String, Value>) -> PathBuf {
    resolve_path(
        root,
        payload.get("profile_path"),
        "local/state/symbiosis/coherence/profile_state.json",
    )
}

fn default_profile_state() -> Value {
    json!({
        "version": "1.0",
        "updated_at": now_iso(),
        "settings": {
            "tone": "collaborative",
            "depth": 0.62,
            "initiative": 0.54,
            "tool_aggressiveness": 0.34,
            "response_style": "balanced",
            "detail_level": "standard",
            "proactivity_tolerance": "medium",
            "risk_appetite": "balanced"
        },
        "edit_later_path": profile_edit_later_path(),
        "deltas": []
    })
}

fn profile_edit_later_path() -> &'static str {
    "protheus-ops symbiosis-coherence-kernel profile-update --payload='{\"response_style\":\"direct\",\"detail_level\":\"detailed\",\"proactivity_tolerance\":\"high\",\"risk_appetite\":\"cautious\"}'"
}

fn normalize_choice(raw: Option<&Value>, fallback: &str, allowed: &[&str]) -> String {
    let token = normalize_token(raw, 32);
    if token.is_empty() {
        return fallback.to_string();
    }
    if allowed.iter().any(|item| *item == token) {
        token
    } else {
        fallback.to_string()
    }
}

fn normalize_response_style(raw: Option<&Value>, fallback: &str) -> String {
    normalize_choice(raw, fallback, &["direct", "balanced", "coaching"])
}

fn normalize_detail_level(raw: Option<&Value>, fallback: &str) -> String {
    normalize_choice(raw, fallback, &["concise", "standard", "detailed"])
}

fn normalize_proactivity_tolerance(raw: Option<&Value>, fallback: &str) -> String {
    normalize_choice(raw, fallback, &["low", "medium", "high"])
}

fn normalize_risk_appetite(raw: Option<&Value>, fallback: &str) -> String {
    normalize_choice(raw, fallback, &["cautious", "balanced", "aggressive"])
}

fn profile_checklist(settings: &Value) -> Value {
    let response_style = normalize_response_style(settings.get("response_style"), "balanced");
    let detail_level = normalize_detail_level(settings.get("detail_level"), "standard");
    let proactivity_tolerance =
        normalize_proactivity_tolerance(settings.get("proactivity_tolerance"), "medium");
    let risk_appetite = normalize_risk_appetite(settings.get("risk_appetite"), "balanced");
    json!([
        {
            "key": "response_style",
            "label": "Response style",
            "current": response_style,
            "options": ["direct", "balanced", "coaching"],
            "description": "Controls concise-vs-guided response tone."
        },
        {
            "key": "detail_level",
            "label": "Detail level",
            "current": detail_level,
            "options": ["concise", "standard", "detailed"],
            "description": "Controls default answer depth."
        },
        {
            "key": "proactivity_tolerance",
            "label": "Proactivity tolerance",
            "current": proactivity_tolerance,
            "options": ["low", "medium", "high"],
            "description": "Controls how often proactive suggestions are surfaced."
        },
        {
            "key": "risk_appetite",
            "label": "Risk appetite",
            "current": risk_appetite,
            "options": ["cautious", "balanced", "aggressive"],
            "description": "Controls conservative vs bold default suggestions."
        }
    ])
}

fn normalize_tone(raw: Option<&Value>, fallback: &str) -> String {
    let token = normalize_token(raw, 32);
    if token.is_empty() {
        return fallback.to_string();
    }
    match token.as_str() {
        "direct" | "concise" => "direct".to_string(),
        "neutral" | "balanced" => "neutral".to_string(),
        "collaborative" | "supportive" => "collaborative".to_string(),
        _ => fallback.to_string(),
    }
}

fn signed_delta(value: Option<&Value>, fallback: f64) -> f64 {
    as_f64(value).unwrap_or(fallback).clamp(-0.35, 0.35)
}

fn load_profile_state(path: &Path) -> Value {
    let loaded = read_json_value(path, default_profile_state());
    if loaded.is_object() {
        loaded
    } else {
        default_profile_state()
    }
}
