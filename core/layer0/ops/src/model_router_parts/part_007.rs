pub fn route_class_policy(cfg: &Value, route_class_raw: &str) -> RouteClassPolicy {
    let id = {
        let normalized = normalize_key(if route_class_raw.is_empty() {
            "default"
        } else {
            route_class_raw
        });
        if normalized.is_empty() {
            "default".to_string()
        } else {
            normalized
        }
    };

    let classes = cfg
        .as_object()
        .and_then(|v| v.get("routing"))
        .and_then(Value::as_object)
        .and_then(|v| v.get("route_classes"))
        .and_then(Value::as_object);
    let src = classes
        .and_then(|map| map.get(&id))
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    let mut merged = Map::<String, Value>::new();
    if id == "reflex" {
        merged.insert("force_risk".to_string(), Value::String("low".to_string()));
        merged.insert(
            "force_complexity".to_string(),
            Value::String("low".to_string()),
        );
        merged.insert(
            "force_role".to_string(),
            Value::String("reflex".to_string()),
        );
        merged.insert(
            "prefer_slot".to_string(),
            Value::String("grunt".to_string()),
        );
        merged.insert(
            "prefer_model".to_string(),
            Value::String("ollama/smallthinker".to_string()),
        );
        merged.insert(
            "fallback_slot".to_string(),
            Value::String("fallback".to_string()),
        );
        merged.insert("disable_fast_path".to_string(), Value::Bool(true));
        merged.insert(
            "max_tokens_est".to_string(),
            Value::Number(serde_json::Number::from(420)),
        );
    }
    for (k, v) in src {
        merged.insert(k, v);
    }

    let force_risk_raw = normalize_key(
        merged
            .get("force_risk")
            .and_then(Value::as_str)
            .unwrap_or_default(),
    );
    let force_complexity_raw = normalize_key(
        merged
            .get("force_complexity")
            .and_then(Value::as_str)
            .unwrap_or_default(),
    );

    let max_tokens = finite_number(merged.get("max_tokens_est"));
    RouteClassPolicy {
        id,
        force_risk: match force_risk_raw.as_str() {
            "low" | "medium" | "high" => Some(force_risk_raw),
            _ => None,
        },
        force_complexity: match force_complexity_raw.as_str() {
            "low" | "medium" | "high" => Some(force_complexity_raw),
            _ => None,
        },
        force_role: normalize_key(
            merged
                .get("force_role")
                .and_then(Value::as_str)
                .unwrap_or_default(),
        ),
        prefer_slot: normalized_optional_string(merged.get("prefer_slot")),
        prefer_model: normalized_optional_string(merged.get("prefer_model")),
        fallback_slot: normalized_optional_string(merged.get("fallback_slot")),
        disable_fast_path: to_bool_like_value(merged.get("disable_fast_path"), false),
        max_tokens_est: max_tokens.and_then(|value| {
            if value.is_finite() && value > 0.0 {
                Some((value.round() as i64).clamp(50, 12_000))
            } else {
                None
            }
        }),
    }
}

pub fn prompt_cache_lane_for_route(
    route_class_id: &str,
    mode: &str,
    execution_intent: bool,
) -> String {
    let route_class = normalize_key(route_class_id);
    let mode_key = normalize_key(mode);
    if route_class == "reflex" {
        return "reflex".to_string();
    }
    if mode_key.contains("dream") {
        return "dream".to_string();
    }
    if execution_intent {
        return "autonomy".to_string();
    }
    "autonomy".to_string()
}

fn tier_alias_to_adjustment(tier_alias: &str, base: &ModeAdjustment) -> ModeAdjustment {
    let key = normalize_key(tier_alias);
    if key == "tier1_governance" {
        return ModeAdjustment {
            risk: "high".to_string(),
            complexity: "high".to_string(),
            role: "logic".to_string(),
            mode_adjusted: true,
            mode_reason: Some("tier1_governance".to_string()),
            ..base.clone()
        };
    }
    if key == "tier2_build" {
        return ModeAdjustment {
            risk: "medium".to_string(),
            complexity: "medium".to_string(),
            role: "coding".to_string(),
            mode_adjusted: true,
            mode_reason: Some("tier2_build".to_string()),
            ..base.clone()
        };
    }
    if key == "tier3_grunt" {
        return ModeAdjustment {
            risk: "low".to_string(),
            complexity: "low".to_string(),
            role: "chat".to_string(),
            mode_adjusted: true,
            mode_reason: Some("tier3_grunt".to_string()),
            ..base.clone()
        };
    }
    ModeAdjustment {
        mode_adjusted: false,
        mode_reason: None,
        ..base.clone()
    }
}

pub fn apply_mode_adjustments(
    mode: &str,
    base: &ModeAdjustmentInput,
    adapters: &Value,
) -> ModeAdjustment {
    let m = normalize_key(if mode.is_empty() { "normal" } else { mode });
    let out = ModeAdjustment {
        risk: base.risk.clone(),
        complexity: base.complexity.clone(),
        role: base.role.clone(),
        mode: m.clone(),
        mode_adjusted: false,
        mode_reason: None,
        mode_policy_source: "fallback".to_string(),
    };

    let mode_routing = adapters
        .as_object()
        .and_then(|v| v.get("mode_routing"))
        .and_then(Value::as_object);
    if let Some(routing) = mode_routing {
        let has_explicit = routing.contains_key(&m);
        let allow_default = !(m == "normal" || m == "default");
        let alias = if has_explicit {
            routing.get(&m).and_then(Value::as_str)
        } else if allow_default {
            routing.get("default").and_then(Value::as_str)
        } else {
            None
        };
        if let Some(alias) = alias {
            let mut mapped = tier_alias_to_adjustment(alias, &out);
            mapped.mode = m.clone();
            mapped.mode_policy_source = "client/runtime/config/model_adapters.json".to_string();
            if m == "deep-thinker" || m == "deep_thinker" {
                mapped.risk = "high".to_string();
                mapped.complexity = "high".to_string();
                mapped.role = "logic".to_string();
                mapped.mode_adjusted = true;
                mapped.mode_reason = Some("deep_thinker_forces_high_logic".to_string());
            }
            return mapped;
        }
    }

    if m == "deep-thinker" || m == "deep_thinker" {
        return ModeAdjustment {
            risk: "high".to_string(),
            complexity: "high".to_string(),
            role: "logic".to_string(),
            mode_adjusted: true,
            mode_reason: Some("deep_thinker_forces_high_logic".to_string()),
            ..out
        };
    }
    if m == "hyper-creative" || m == "hyper_creative" {
        let next_complexity = if out.complexity == "low" {
            "medium".to_string()
        } else {
            out.complexity.clone()
        };
        return ModeAdjustment {
            complexity: next_complexity,
            role: "planning".to_string(),
            mode_adjusted: true,
            mode_reason: Some("hyper_creative_bias_planning".to_string()),
            ..out
        };
    }
    if m == "creative" || m == "narrative" {
        return ModeAdjustment {
            role: "chat".to_string(),
            mode_adjusted: true,
            mode_reason: Some(format!("{m}_bias_chat")),
            ..out
        };
    }
    out
}

fn finite_number(value: Option<&Value>) -> Option<f64> {
    let raw = value?;
    match raw {
        Value::Number(n) => n.as_f64().filter(|v| v.is_finite()),
        Value::String(s) => s.trim().parse::<f64>().ok().filter(|v| v.is_finite()),
        Value::Bool(true) => Some(1.0),
        Value::Bool(false) => Some(0.0),
        _ => None,
    }
}

fn js_truthy(value: Option<&Value>) -> bool {
    match value {
        Some(Value::Null) | None => false,
        Some(Value::Bool(v)) => *v,
        Some(Value::Number(n)) => n.as_f64().is_some_and(|v| v != 0.0),
        Some(Value::String(s)) => !s.is_empty(),
        Some(Value::Array(_)) | Some(Value::Object(_)) => true,
    }
}

fn js_number_from_truthy_or(default: f64, value: Option<&Value>) -> f64 {
    if !js_truthy(value) {
        return default;
    }
    finite_number(value).unwrap_or(default)
}

fn object_field<'a>(obj: &'a Map<String, Value>, key: &str) -> Option<&'a Value> {
    obj.get(key)
}

fn string_or_null(value: Option<&Value>) -> Value {
    value
        .and_then(Value::as_str)
        .map(|v| Value::String(v.to_string()))
        .unwrap_or(Value::Null)
}

pub fn build_handoff_packet(decision: &Value) -> Value {
    let Some(obj) = decision.as_object() else {
        return json!({
            "selected_model": null,
            "previous_model": null,
            "model_changed": false,
            "reason": null,
            "tier": 2,
            "role": null,
            "route_class": "default",
            "mode": null,
            "slot": null,
            "escalation_chain": []
        });
    };

    // Keep JS `Number(d.tier || 2)` behavior: numeric zero defaults to 2.
    let tier_num = js_number_from_truthy_or(2.0, object_field(obj, "tier"));
    let tier = if tier_num.is_finite() {
        tier_num.round() as i64
    } else {
        2
    };
    let role = normalize_key(
        object_field(obj, "role")
            .and_then(Value::as_str)
            .unwrap_or_default(),
    );

    let escalation_limit = (tier + 1).clamp(2, 4) as usize;
    let escalation_chain = object_field(obj, "escalation_chain")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .take(escalation_limit)
                .cloned()
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let mut out = json!({
        "selected_model": string_or_null(object_field(obj, "selected_model")),
        "previous_model": string_or_null(object_field(obj, "previous_model")),
        "model_changed": object_field(obj, "model_changed").and_then(Value::as_bool).unwrap_or(false),
        "reason": string_or_null(object_field(obj, "reason")),
        "tier": tier,
        "role": if role.is_empty() { Value::Null } else { Value::String(role.clone()) },
        "route_class": object_field(obj, "route_class").and_then(Value::as_str).unwrap_or("default"),
        "mode": string_or_null(object_field(obj, "mode")),
        "slot": string_or_null(object_field(obj, "slot")),
        "escalation_chain": escalation_chain
    });

    let out_obj = out
        .as_object_mut()
        .expect("handoff packet root should always be an object");

    if object_field(obj, "fast_path")
        .and_then(Value::as_object)
        .and_then(|v| v.get("matched"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        out_obj.insert(
            "fast_path".to_string(),
            Value::String("communication".to_string()),
        );
    }

    if let Some(budget) = object_field(obj, "budget").and_then(Value::as_object) {
        let pressure = budget
            .get("pressure")
            .and_then(Value::as_str)
            .unwrap_or("none");
        let projected_pressure = budget
            .get("projected_pressure")
            .and_then(Value::as_str)
            .or_else(|| budget.get("pressure").and_then(Value::as_str))
            .unwrap_or("none");
        let request_tokens_est = finite_number(budget.get("request_tokens_est"))
            .and_then(serde_json::Number::from_f64)
            .map(Value::Number)
            .unwrap_or(Value::Null);

        out_obj.insert(
            "budget".to_string(),
            json!({
                "pressure": pressure,
                "projected_pressure": projected_pressure,
                "request_tokens_est": request_tokens_est
            }),
        );
    }

    let role_with_capability = matches!(
        role.as_str(),
        "coding" | "tools" | "swarm" | "planning" | "logic"
    );
    if tier >= 2 || role_with_capability {
        out_obj.insert(
            "capability".to_string(),
            string_or_null(object_field(obj, "capability")),
        );
        out_obj.insert(
            "fallback_slot".to_string(),
            string_or_null(object_field(obj, "fallback_slot")),
        );
    }

    if tier >= 3 {
        out_obj.insert(
            "guardrails".to_string(),
            json!({
                "deep_thinker": js_truthy(object_field(obj, "deep_thinker")),
                "verification_required": true
            }),
        );
        if js_truthy(object_field(obj, "post_task_return_model")) {
            out_obj.insert(
                "post_task_return_model".to_string(),
                object_field(obj, "post_task_return_model")
                    .cloned()
                    .unwrap_or(Value::Null),
            );
        }
    }

    if let Some(budget_enforcement) =
        object_field(obj, "budget_enforcement").and_then(Value::as_object)
    {
        out_obj.insert(
            "budget_enforcement".to_string(),
            json!({
                "action": string_or_null(budget_enforcement.get("action")),
                "reason": string_or_null(budget_enforcement.get("reason")),
                "blocked": matches!(budget_enforcement.get("blocked"), Some(Value::Bool(true)))
            }),
        );
    }

    out
}

#[cfg(test)]
#[path = "../model_router_tests_part1.rs"]
mod model_router_tests_part1;
#[cfg(test)]
#[path = "../model_router_tests_part2.rs"]
mod model_router_tests_part2;
#[cfg(test)]
#[path = "../model_router_tests_part3.rs"]
mod model_router_tests_part3;

