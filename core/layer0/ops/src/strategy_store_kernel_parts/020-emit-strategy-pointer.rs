fn emit_strategy_pointer(root: &Path, meta: Option<&Map<String, Value>>) -> Result<(), String> {
    let rel = DEFAULT_REL_PATH;
    let uid = stable_uid(&format!("adaptive_blob|{rel}|v1"), "a", 24);
    let row = json!({
        "ts": now_iso(),
        "op": "set",
        "source": "adaptive_layer_store",
        "source_path": source_from_meta(meta),
        "reason": reason_from_meta(meta, "set_strategy_state"),
        "actor": actor_from_meta(meta),
        "kind": "adaptive_strategy_registry-json",
        "layer": "strategy",
        "uid": uid,
        "entity_id": Value::Null,
        "status": "active",
        "tags": ["adaptive", "strategy"],
        "summary": "Adaptive record: strategy/registry.json",
        "path_ref": "adaptive/strategy/registry.json",
        "created_ts": now_iso(),
        "updated_ts": now_iso(),
    });
    let key = format!(
        "{}|{}|{}|{}",
        row.get("kind").and_then(Value::as_str).unwrap_or(""),
        row.get("uid").and_then(Value::as_str).unwrap_or(""),
        row.get("path_ref").and_then(Value::as_str).unwrap_or(""),
        row.get("entity_id")
            .map(json_string)
            .unwrap_or_else(|| "null".to_string()),
    );
    let hash = hash16(&json_string(&json!({
        "uid": row.get("uid").cloned().unwrap_or(Value::Null),
        "kind": row.get("kind").cloned().unwrap_or(Value::Null),
        "path_ref": row.get("path_ref").cloned().unwrap_or(Value::Null),
        "entity_id": row.get("entity_id").cloned().unwrap_or(Value::Null),
        "tags": row.get("tags").cloned().unwrap_or(Value::Null),
        "summary": row.get("summary").cloned().unwrap_or(Value::Null),
        "status": row.get("status").cloned().unwrap_or(Value::Null),
    })));
    let mut index = pointer_index_load(root);
    if !index.get("pointers").map(Value::is_object).unwrap_or(false) {
        index["pointers"] = json!({});
    }
    if index["pointers"].get(&key).and_then(Value::as_str) == Some(hash.as_str()) {
        return Ok(());
    }
    append_jsonl(&pointers_path(root), &row)?;
    index["pointers"][&key] = Value::String(hash);
    pointer_index_save(root, &index)
}

fn default_strategy_draft(seed: Option<&Map<String, Value>>) -> Value {
    let seed = seed.unwrap_or_else(|| {
        static EMPTY: std::sync::OnceLock<Map<String, Value>> = std::sync::OnceLock::new();
        EMPTY.get_or_init(Map::new)
    });
    let base_name = {
        let candidate = clean_text(seed.get("id"), 40);
        if !candidate.is_empty() {
            candidate
        } else {
            let name = clean_text(seed.get("name"), 120);
            if !name.is_empty() {
                name
            } else {
                format!("strategy_{}", random_uid("s", 8))
            }
        }
    };
    let id = {
        let key = normalize_key(&base_name, 40);
        if key.is_empty() {
            format!("strategy_{}", hash16(&now_iso()))
        } else {
            key
        }
    };
    let name = clean_text(seed.get("name"), 120).if_empty_then(&id);
    let objective_primary = if let Some(objective) = as_object(seed.get("objective")) {
        clean_text(objective.get("primary"), 180)
    } else {
        let summary = clean_text(seed.get("summary"), 180);
        if !summary.is_empty() {
            summary
        } else {
            let prompt = clean_text(seed.get("prompt"), 180);
            if !prompt.is_empty() {
                prompt
            } else {
                format!("Improve outcomes for {name}")
            }
        }
    };
    json!({
        "version": "1.0",
        "id": id,
        "name": name,
        "status": "disabled",
        "objective": {
            "primary": objective_primary.if_empty_then(&format!("Improve outcomes for {name}")),
            "secondary": [],
            "fitness_metric": "verified_progress_rate",
            "target_window_days": 14,
        },
        "generation_policy": {
            "mode": normalize_mode(seed.get("generation_mode"), Some("hyper-creative")),
        },
        "risk_policy": {
            "allowed_risks": normalize_allowed_risks(seed.get("risk_policy").and_then(Value::as_object).and_then(|v| v.get("allowed_risks"))),
            "max_risk_per_action": clamp_number(
                seed.get("risk_policy")
                    .and_then(Value::as_object)
                    .and_then(|v| v.get("max_risk_per_action")),
                0.0,
                100.0,
                35.0,
            ),
        },
        "admission_policy": {
            "allowed_types": [],
            "blocked_types": [],
            "max_remediation_depth": 2,
            "duplicate_window_hours": 24,
        },
        "ranking_weights": {
            "composite": 0.35,
            "actionability": 0.2,
            "directive_fit": 0.15,
            "signal_quality": 0.15,
            "expected_value": 0.1,
            "time_to_value": 0.0,
            "risk_penalty": 0.05,
        },
        "budget_policy": {
            "daily_runs_cap": 4,
            "daily_token_cap": 4000,
            "max_tokens_per_action": 1600,
        },
        "exploration_policy": {
            "fraction": 0.25,
            "every_n": 3,
            "min_eligible": 3,
        },
        "stop_policy": {
            "circuit_breakers": {
                "http_429_cooldown_hours": 12,
            }
        },
        "promotion_policy": {
            "min_days": 7,
            "min_attempted": 12,
            "min_verified_rate": 0.5,
            "min_success_criteria_receipts": 2,
            "min_success_criteria_pass_rate": 0.6,
            "min_objective_coverage": 0.25,
            "max_objective_no_progress_rate": 0.9,
            "max_reverted_rate": 0.35,
            "max_stop_ratio": 0.75,
            "min_shipped": 1,
        },
        "execution_policy": {
            "mode": "score_only",
        },
        "threshold_overrides": {},
    })
}

trait StringExt {
    fn if_empty_then(self, fallback: &str) -> String;
}

impl StringExt for String {
    fn if_empty_then(self, fallback: &str) -> String {
        if self.trim().is_empty() {
            fallback.to_string()
        } else {
            self
        }
    }
}

fn normalize_mode(value: Option<&Value>, fallback: Option<&str>) -> String {
    let raw = as_str(value).to_ascii_lowercase().replace('_', "-");
    let normalized = match raw.as_str() {
        "hypercreative" | "hyper-creative" => "hyper-creative",
        "deepthinker" | "deep-thinker" => "deep-thinker",
        _ => "",
    };
    if GENERATION_MODES.contains(&normalized) {
        normalized.to_string()
    } else {
        fallback.unwrap_or("hyper-creative").to_string()
    }
}

fn normalize_execution_mode(value: Option<&Value>, fallback: Option<&str>) -> String {
    let raw = as_str(value).to_ascii_lowercase().replace('-', "_");
    let normalized = match raw.as_str() {
        "score_only" | "scoreonly" | "preview" => "score_only",
        "canary_execute" | "canary" => "canary_execute",
        "execute" | "full_execute" | "run" => "execute",
        _ => "",
    };
    if EXECUTION_MODES.contains(&normalized) {
        normalized.to_string()
    } else {
        fallback.unwrap_or("score_only").to_string()
    }
}

fn normalize_allowed_risks(raw: Option<&Value>) -> Value {
    let mut out = Vec::new();
    let values = if let Some(rows) = raw.and_then(Value::as_array) {
        rows.iter()
            .map(|row| as_str(Some(row)))
            .collect::<Vec<_>>()
    } else {
        as_str(raw).split(',').map(|row| row.to_string()).collect::<Vec<_>>()
    };
    for value in values {
        let mapped = match value.trim().to_ascii_lowercase().as_str() {
            "low" | "safe" | "minimal" | "none" => "low",
            "medium" | "med" | "moderate" => "medium",
            "high" | "critical" | "severe" => "high",
            _ => "",
        };
        if mapped.is_empty() {
            continue;
        }
        if !out.iter().any(|existing| existing == mapped) {
            out.push(mapped.to_string());
        }
    }
    if out.is_empty() {
        out.push("low".to_string());
    }
    Value::Array(out.into_iter().map(Value::String).collect())
}

fn default_strategy_state() -> Value {
    json!({
        "version": "1.0",
        "policy": {
            "max_profiles": 64,
            "max_queue": 64,
            "queue_ttl_hours": 72,
            "queue_max_attempts": 3,
            "queue_min_evidence_refs": 1,
            "queue_min_trust_score": 35,
            "gc_inactive_days": 21,
            "gc_min_uses_30d": 1,
            "gc_protect_new_days": 3,
        },
        "profiles": [],
        "intake_queue": [],
        "metrics": {
            "total_intakes": 0,
            "total_profiles_created": 0,
            "total_profiles_updated": 0,
            "total_queue_consumed": 0,
            "total_gc_deleted": 0,
            "last_gc_ts": Value::Null,
            "last_usage_sync_ts": Value::Null,
        }
    })
}

fn normalize_policy(raw: Option<&Map<String, Value>>) -> Value {
    let raw = raw.unwrap_or_else(|| {
        static EMPTY: std::sync::OnceLock<Map<String, Value>> = std::sync::OnceLock::new();
        EMPTY.get_or_init(Map::new)
    });
    let defaults = default_strategy_state();
    let base = defaults.get("policy").and_then(Value::as_object).unwrap();
    json!({
        "max_profiles": clamp_i64(raw.get("max_profiles"), 4, 512, base.get("max_profiles").and_then(Value::as_i64).unwrap_or(64)),
        "max_queue": clamp_i64(raw.get("max_queue"), 4, 512, base.get("max_queue").and_then(Value::as_i64).unwrap_or(64)),
        "queue_ttl_hours": clamp_i64(raw.get("queue_ttl_hours"), 1, 24 * 30, base.get("queue_ttl_hours").and_then(Value::as_i64).unwrap_or(72)),
        "queue_max_attempts": clamp_i64(raw.get("queue_max_attempts"), 1, 100, base.get("queue_max_attempts").and_then(Value::as_i64).unwrap_or(3)),
        "queue_min_evidence_refs": clamp_i64(raw.get("queue_min_evidence_refs"), 0, 32, base.get("queue_min_evidence_refs").and_then(Value::as_i64).unwrap_or(1)),
        "queue_min_trust_score": clamp_i64(raw.get("queue_min_trust_score"), 0, 100, base.get("queue_min_trust_score").and_then(Value::as_i64).unwrap_or(35)),
        "gc_inactive_days": clamp_i64(raw.get("gc_inactive_days"), 1, 365, base.get("gc_inactive_days").and_then(Value::as_i64).unwrap_or(21)),
        "gc_min_uses_30d": clamp_i64(raw.get("gc_min_uses_30d"), 0, 1000, base.get("gc_min_uses_30d").and_then(Value::as_i64).unwrap_or(1)),
        "gc_protect_new_days": clamp_i64(raw.get("gc_protect_new_days"), 0, 90, base.get("gc_protect_new_days").and_then(Value::as_i64).unwrap_or(3)),
    })
}

fn normalize_usage(raw: Option<&Map<String, Value>>, now_ts: &str) -> Value {
    let raw = raw.unwrap_or_else(|| {
        static EMPTY: std::sync::OnceLock<Map<String, Value>> = std::sync::OnceLock::new();
        EMPTY.get_or_init(Map::new)
    });
    let mut events = raw
        .get("use_events")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .map(|row| as_str(Some(row)))
                .filter(|row| parse_ts_ms(row).is_some())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    events.sort();
    if events.len() > 256 {
        events = events.split_off(events.len() - 256);
    }
    let cutoff = Utc::now().timestamp_millis() - (30_i64 * 24 * 60 * 60 * 1000);
    let uses_30 = events
        .iter()
        .filter(|ts| parse_ts_ms(ts).map(|ms| ms >= cutoff).unwrap_or(false))
        .count() as i64;
    json!({
        "uses_total": clamp_i64(raw.get("uses_total"), 0, 100_000_000, events.len() as i64),
        "uses_30d": clamp_i64(raw.get("uses_30d"), 0, 100_000_000, uses_30),
        "use_events": events,
        "last_used_ts": raw.get("last_used_ts").filter(|v| parse_ts_ms(&as_str(Some(v))).is_some()).cloned().unwrap_or(Value::Null),
        "last_usage_sync_ts": raw.get("last_usage_sync_ts").filter(|v| parse_ts_ms(&as_str(Some(v))).is_some()).cloned().unwrap_or_else(|| Value::String(now_ts.to_string())),
    })
}

fn ensure_work_packet(item: &Value) -> Value {
    let mode = normalize_mode(
        item.get("recommended_generation_mode")
            .or_else(|| item.get("generation_mode")),
        Some("hyper-creative"),
    );
    json!({
        "mode_hint": mode,
        "allowed_modes": ["hyper-creative", "deep-thinker"],
        "objective": "Turn this intake signal into a structured strategy profile draft.",
        "input_summary": clean_text(item.get("summary"), 220),
        "output_contract": {
            "format": "strategy_profile_json",
            "required_keys": [
                "id",
                "name",
                "objective.primary",
                "risk_policy.allowed_risks",
                "execution_policy.mode"
            ],
            "notes": "Keep output strategy-agnostic and deterministic; prefer score_only at first."
        }
    })
}

fn recommend_mode(summary: &str, raw_text: &str) -> String {
    let text = format!("{} {}", summary, raw_text).to_ascii_lowercase();
    if text.len() > 900
        || [
            "tradeoff",
            "architecture",
            "uncertain",
            "counterfactual",
            "conflict",
            "multi-step",
            "nonlinear",
            "portfolio",
            "long horizon",
            "long-horizon",
        ]
        .iter()
        .any(|needle| text.contains(needle))
    {
        "deep-thinker".to_string()
    } else {
        "hyper-creative".to_string()
    }
}

fn compute_trust_score(item: &Value) -> i64 {
    let source = as_str(item.get("source")).to_ascii_lowercase();
    let evidence = item
        .get("evidence_refs")
        .and_then(Value::as_array)
        .map(|rows| rows.len())
        .unwrap_or(0) as i64;
    let summary_len = as_str(item.get("summary")).len() as i64;
    let text_len = as_str(item.get("text")).len() as i64;
    let mut score = 20_i64;
    score += (evidence * 10).min(40);
    score += (summary_len / 20).min(20);
    score += (text_len / 300).min(10);
    if source.contains("outcome_fitness") || source.contains("strategy_scorecards") {
        score += 12;
    }
    if source.contains("cross_signal") || source.contains("sensory_trends") {
        score += 8;
    }
    if source == "manual" {
        score += 5;
    }
    score.clamp(0, 100)
}

fn queue_drop_reasons(item: &Value, policy: &Value, now_ms: i64) -> Vec<String> {
    let created_ms = item
        .get("created_ts")
        .and_then(|v| parse_ts_ms(&as_str(Some(v))));
    let ttl_hours = clamp_i64(policy.get("queue_ttl_hours"), 1, 24 * 30, 72);
    let max_attempts = clamp_i64(policy.get("queue_max_attempts"), 1, 100, 3);
    let min_evidence = clamp_i64(policy.get("queue_min_evidence_refs"), 0, 32, 1);
    let min_trust = clamp_i64(policy.get("queue_min_trust_score"), 0, 100, 35);
    let mut reasons = Vec::new();
    if created_ms
        .map(|ms| now_ms - ms > ttl_hours * 60 * 60 * 1000)
        .unwrap_or(false)
    {
        reasons.push("queue_ttl_expired".to_string());
    }
    if clamp_i64(item.get("attempts"), 0, 1000, 0) >= max_attempts {
        reasons.push("queue_max_attempts_exceeded".to_string());
    }
    let evidence_len = item
        .get("evidence_refs")
        .and_then(Value::as_array)
        .map(|rows| rows.len())
        .unwrap_or(0) as i64;
    if evidence_len < min_evidence {
        reasons.push("evidence_missing".to_string());
    }
    if clamp_i64(item.get("trust_score"), 0, 100, 0) < min_trust {
        reasons.push("trust_score_low".to_string());
    }
    if as_str(item.get("summary")).len() < 16 {
        reasons.push("summary_too_short".to_string());
    }
    reasons
}
