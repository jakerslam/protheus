pub fn evaluate_heroic_gate(req: &HeroicGateRequest) -> HeroicGateResponse {
    let local_destructive = Regex::new(
        r"(?:\bdisable\s+(?:all\s+)?guards?\b|\bbypass\b.*\b(?:guard|policy|safety)\b|\bself[\s_-]*terminate\b|\bexfiltrate\b|\bwipe\s+data\b)",
    )
    .ok()
    .map(|regex| regex.is_match(req.task_text.as_str()))
    .unwrap_or(false);

    let purified = req
        .purified_row
        .as_ref()
        .and_then(|value| value.as_object())
        .cloned();
    if purified.is_none() {
        let mut reason_codes = vec!["heroic_echo_row_missing".to_string()];
        if local_destructive {
            reason_codes.push("local_destructive_pattern".to_string());
        }
        return HeroicGateResponse {
            ok: true,
            classification: if local_destructive {
                "destructive_instruction".to_string()
            } else {
                "unknown".to_string()
            },
            decision: if local_destructive {
                "blocked_destructive_local_pattern".to_string()
            } else {
                "purification_missing".to_string()
            },
            blocked: local_destructive,
            reason_codes,
        };
    }

    let purified = purified.expect("purified row must exist");
    let row_classification =
        clean_or_default(purified.get("classification").and_then(|value| value.as_str()), 80, "unknown");
    let classification = if local_destructive {
        "destructive_instruction".to_string()
    } else if row_classification.is_empty() {
        "unknown".to_string()
    } else {
        row_classification
    };
    let row_decision =
        clean_or_default(purified.get("decision").and_then(|value| value.as_str()), 120, "unknown");
    let decision = if local_destructive {
        "blocked_destructive_local_pattern".to_string()
    } else if row_decision.is_empty() {
        "unknown".to_string()
    } else {
        row_decision
    };
    let row_blocked = purified
        .get("blocked")
        .and_then(|value| value.as_bool())
        .unwrap_or(false);
    let row_is_destructive = purified
        .get("classification")
        .and_then(|value| value.as_str())
        .map(|value| value == "destructive_instruction")
        .unwrap_or(false);
    let blocked_by_destructive =
        req.block_on_destructive && (row_is_destructive || local_destructive);
    let mut reason_codes = collect_strings(purified.get("reason_codes"), 8, 120);
    if local_destructive {
        reason_codes.push("local_destructive_pattern".to_string());
    }
    HeroicGateResponse {
        ok: true,
        classification,
        decision,
        blocked: blocked_by_destructive || row_blocked,
        reason_codes,
    }
}

pub fn evaluate_heroic_gate_json(payload: &str) -> Result<String, String> {
    let req = parse_payload_json::<HeroicGateRequest>(payload, "heroic_gate")?;
    let resp = evaluate_heroic_gate(&req);
    serialize_payload_json(&resp, "heroic_gate")
}

fn ensure_object(value: &mut Value) -> &mut serde_json::Map<String, Value> {
    if !value.is_object() {
        *value = json!({});
    }
    value.as_object_mut().expect("value should be object")
}

fn collect_strings(value: Option<&Value>, max_items: usize, max_len: usize) -> Vec<String> {
    value
        .and_then(|row| row.as_array())
        .map(|rows| {
            rows.iter()
                .filter_map(|row| row.as_str())
                .map(|row| clean_text(row, max_len))
                .filter(|row| !row.is_empty())
                .take(max_items)
                .collect::<Vec<String>>()
        })
        .unwrap_or_default()
}

fn numeric_or_zero(value: Option<&Value>) -> f64 {
    value
        .and_then(|row| row.as_f64().or_else(|| row.as_i64().map(|v| v as f64)))
        .unwrap_or(0.0)
}
pub fn apply_governance(req: &GovernanceApplyRequest) -> Vec<Value> {
    let storm_lane = normalized_or_default(req.policy.storm_lane.as_str(), 80, &default_storm_lane());
    let default_lane = normalized_or_default(req.policy.default_lane.as_str(), 80, &default_lane());
    let min_storm_share = req.policy.min_storm_share.clamp(0.0, 1.0);
    let mut tasks: Vec<Value> = Vec::new();

    for row in &req.rows {
        let source_task = row.get("task").cloned().unwrap_or(Value::Null);
        if !source_task.is_object() {
            continue;
        }
        let mut task = source_task;
        let task_text = clean_text(
            task.get("task_text").and_then(|v| v.as_str()).unwrap_or(""),
            1000,
        );
        if task_text.is_empty() {
            continue;
        }

        let heroic = row.get("heroic").cloned().unwrap_or_else(|| json!({}));
        let heroic_classification =
            clean_or_default(heroic.get("classification").and_then(|v| v.as_str()), 80, "unknown");
        let heroic_decision =
            clean_or_default(heroic.get("decision").and_then(|v| v.as_str()), 80, "unknown");
        let heroic_blocked = heroic
            .get("blocked")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let heroic_reason_codes = collect_strings(heroic.get("reason_codes"), 8, 120);

        let constitution = row
            .get("constitution")
            .cloned()
            .unwrap_or_else(|| json!({}));
        let constitution_decision =
            clean_or_default(constitution.get("decision").and_then(|v| v.as_str()), 40, "ALLOW");
        let constitution_risk =
            clean_or_default(constitution.get("risk").and_then(|v| v.as_str()), 40, "low");
        let constitution_reasons = collect_strings(constitution.get("reasons"), 8, 120);

        let suggested_lane = {
            let row_lane = row
                .get("suggested_lane")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let task_lane = task
                .get("route")
                .and_then(|v| v.as_object())
                .and_then(|route| route.get("lane"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let candidate = if row_lane.trim().is_empty() {
                task_lane
            } else {
                row_lane
            };
            normalized_or_default(candidate, 80, &default_lane)
        };
        let lane = if constitution_decision == "MANUAL" {
            storm_lane.clone()
        } else {
            suggested_lane
        };
        let blocked_by_constitution =
            req.policy.block_on_constitution_deny && constitution_decision == "DENY";
        let blocked = heroic_blocked || blocked_by_constitution;
        let requires_manual_review = constitution_decision == "MANUAL" || lane == storm_lane;

        let duality = row.get("duality").cloned().unwrap_or_else(|| json!({}));
        let duality_indicator = duality
            .get("indicator")
            .filter(|value| value.is_object())
            .cloned()
            .unwrap_or_else(|| json!({ "subtle_hint": "duality_signal_absent" }));
        let duality_score_label =
            clean_or_default(duality.get("score_label").and_then(|v| v.as_str()), 40, "unknown");
        let recommended_adjustment = {
            let value = clean_text(
                duality
                    .get("recommended_adjustment")
                    .and_then(|v| v.as_str())
                    .unwrap_or(""),
                120,
            );
            if value.is_empty() {
                Value::Null
            } else {
                Value::String(value)
            }
        };
        let duality_block = json!({
            "enabled": duality.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false),
            "score_trit": numeric_or_zero(duality.get("score_trit")),
            "score_label": duality_score_label,
            "zero_point_harmony_potential": numeric_or_zero(duality.get("zero_point_harmony_potential")),
            "recommended_adjustment": recommended_adjustment,
            "indicator": duality_indicator
        });

        let block_reasons = {
            let mut reasons: Vec<String> = Vec::new();
            if heroic_blocked {
                reasons.push("heroic_echo_blocked".to_string());
            }
            if blocked_by_constitution {
                reasons.push("constitution_denied".to_string());
            }
            reasons
        };

        {
            let task_obj = ensure_object(&mut task);
            let route = ensure_object(task_obj.entry("route").or_insert_with(|| json!({})));
            route.insert("lane".to_string(), Value::String(lane.clone()));
            route.insert("blocked".to_string(), Value::Bool(blocked));
            route.insert(
                "requires_manual_review".to_string(),
                Value::Bool(requires_manual_review),
            );

            task_obj.insert(
                "governance".to_string(),
                json!({
                    "blocked": blocked,
                    "block_reasons": block_reasons,
                    "heroic_echo": {
                        "classification": heroic_classification,
                        "decision": heroic_decision,
                        "blocked": heroic_blocked,
                        "reason_codes": heroic_reason_codes
                    },
                    "constitution": {
                        "decision": constitution_decision,
                        "risk": constitution_risk,
                        "reasons": constitution_reasons
                    }
                }),
            );
            task_obj.insert("duality".to_string(), duality_block.clone());

            let profile = ensure_object(task_obj.entry("profile").or_insert_with(|| json!({})));
            let routing = ensure_object(profile.entry("routing").or_insert_with(|| json!({})));
            routing.insert("preferred_lane".to_string(), Value::String(lane.clone()));
            routing.insert(
                "requires_manual_review".to_string(),
                Value::Bool(requires_manual_review),
            );

            let provenance =
                ensure_object(profile.entry("provenance").or_insert_with(|| json!({})));
            provenance.insert(
                "confidence".to_string(),
                Value::from(if blocked { 0.55 } else { 0.92 }),
            );
            let evidence = ensure_object(provenance.entry("evidence").or_insert_with(|| json!({})));
            evidence.insert(
                "heroic_echo_decision".to_string(),
                Value::String(heroic_decision.clone()),
            );
            evidence.insert(
                "constitution_decision".to_string(),
                Value::String(constitution_decision.clone()),
            );

            profile.insert(
                "governance".to_string(),
                json!({
                    "heroic_echo": {
                        "classification": heroic_classification,
                        "decision": heroic_decision,
                        "reason_codes": heroic_reason_codes
                    },
                    "constitution": {
                        "decision": constitution_decision,
                        "risk": constitution_risk,
                        "reasons": constitution_reasons
                    }
                }),
            );
            profile.insert("duality".to_string(), duality_block);
        }

        tasks.push(task);
    }

    let storm_count = tasks
        .iter()
        .filter(|task| {
            task.get("route")
                .and_then(|v| v.as_object())
                .and_then(|route| route.get("lane"))
                .and_then(|v| v.as_str())
                .map(|lane| lane == storm_lane)
                .unwrap_or(false)
        })
        .count();
    let storm_share = if tasks.is_empty() {
        0.0
    } else {
        storm_count as f64 / tasks.len() as f64
    };
    if tasks.len() > 2 && storm_share < min_storm_share {
        if let Some(task) = tasks.iter_mut().find(|task| {
            task.get("governance")
                .and_then(|v| v.as_object())
                .and_then(|governance| governance.get("constitution"))
                .and_then(|v| v.as_object())
                .and_then(|constitution| constitution.get("decision"))
                .and_then(|v| v.as_str())
                .map(|decision| decision != "DENY")
                .unwrap_or(true)
        }) {
            let task_obj = ensure_object(task);
            let route = ensure_object(task_obj.entry("route").or_insert_with(|| json!({})));
            route.insert("lane".to_string(), Value::String(storm_lane.clone()));
            route.insert("requires_manual_review".to_string(), Value::Bool(true));

            let profile = ensure_object(task_obj.entry("profile").or_insert_with(|| json!({})));
            let routing = ensure_object(profile.entry("routing").or_insert_with(|| json!({})));
            routing.insert(
                "preferred_lane".to_string(),
                Value::String(storm_lane.clone()),
            );
            routing.insert("requires_manual_review".to_string(), Value::Bool(true));
        }
    }

    tasks
}

pub fn apply_governance_json(payload: &str) -> Result<String, String> {
    let req = parse_payload_json::<GovernanceApplyRequest>(payload, "apply_governance")?;
    let resp = GovernanceApplyResponse {
        ok: true,
        tasks: apply_governance(&req),
    };
    serialize_payload_json(&resp, "apply_governance")
}
