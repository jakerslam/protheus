fn normalize_signal_source_alias(raw: &str) -> String {
    let token = normalize_token(raw, 120);
    match token.as_str() {
        "" => "runtime".to_string(),
        "search_web" | "websearch" | "batch_query" => "web_search".to_string(),
        "toolcall" => "tool_call".to_string(),
        "sessionstatus" => "session_status".to_string(),
        _ => token,
    }
}

fn resolve_signal_tool_context(
    context_obj: &serde_json::Map<String, Value>,
    opts_obj: &serde_json::Map<String, Value>,
) -> String {
    for key in ["tool", "tool_name", "capability", "event_type"] {
        let context_value = as_str(context_obj.get(key));
        if !context_value.is_empty() {
            return normalize_token(&context_value, 120);
        }
        let opts_value = as_str(opts_obj.get(key));
        if !opts_value.is_empty() {
            return normalize_token(&opts_value, 120);
        }
    }
    String::new()
}

fn evaluate_signal(
    policy: &Value,
    codex: &Value,
    state: &Value,
    context: &Value,
    opts: &Value,
) -> Value {
    let context_obj = context.as_object().cloned().unwrap_or_default();
    let opts_obj = opts.as_object().cloned().unwrap_or_default();

    let lane = {
        let lane_candidate = as_str(context_obj.get("lane"));
        if !lane_candidate.is_empty() {
            normalize_token(&lane_candidate, 120)
        } else {
            let opt_lane = as_str(opts_obj.get("lane"));
            if !opt_lane.is_empty() {
                normalize_token(&opt_lane, 120)
            } else {
                let path_lane = as_str(context_obj.get("path"));
                if !path_lane.is_empty() {
                    normalize_token(&path_lane, 120)
                } else {
                    "unknown_lane".to_string()
                }
            }
        }
    };

    let run_id = {
        let candidate = as_str(context_obj.get("run_id"));
        if !candidate.is_empty() {
            candidate
        } else {
            let opt = as_str(opts_obj.get("run_id"));
            if opt.is_empty() {
                String::new()
            } else {
                opt
            }
        }
    };

    let source = {
        let candidate = as_str(context_obj.get("source"));
        if !candidate.is_empty() {
            normalize_signal_source_alias(&candidate)
        } else {
            let opt = as_str(opts_obj.get("source"));
            if opt.is_empty() {
                "runtime".to_string()
            } else {
                normalize_signal_source_alias(&opt)
            }
        }
    };
    let tool_context = resolve_signal_tool_context(&context_obj, &opts_obj);

    let lane_is_enabled = lane_enabled(policy, &lane);
    if !as_bool(policy.get("enabled"), true) || !lane_is_enabled {
        return json!({
            "enabled": false,
            "lane": lane,
            "lane_enabled": lane_is_enabled,
            "advisory_only": true,
            "shadow_only": true,
            "score_trit": TRIT_UNKNOWN,
            "score_label": trit_label(TRIT_UNKNOWN),
            "zero_point_harmony_potential": 0.0,
            "recommended_adjustment": "disabled",
            "confidence": 0.0,
            "advisory_weight": 0.0,
            "effective_weight": 0.0,
            "seed_confidence": clamp_f64(as_f64(state.get("seed_confidence")).unwrap_or(1.0), 0.0, 1.0),
            "codex_version": as_str(codex.get("version")),
            "contradiction_tracking": {
                "observations_total": as_i64(state.get("observations_total")).unwrap_or(0),
                "contradictions_total": as_i64(state.get("contradictions_total")).unwrap_or(0)
            },
            "indicator": {
                "yin_yang_bias": "neutral",
                "subtle_hint": "duality_signal_disabled"
            }
        });
    }

    let tokens = tokenize_context(context);
    let (yin_set, yang_set) = keyword_sets(codex);
    let mut yin_hits = 0usize;
    let mut yang_hits = 0usize;
    for token in &tokens {
        if yin_set.contains(token) {
            yin_hits += 1;
        }
        if yang_set.contains(token) {
            yang_hits += 1;
        }
    }

    let total = yin_hits + yang_hits;
    let skew = if total > 0 {
        ((yin_hits as f64) - (yang_hits as f64)).abs() / (total as f64)
    } else {
        0.0
    };
    let harmony = if total > 0 { 1.0 - skew } else { 0.0 };
    let signal_density = (total as f64 / 8.0).min(1.0);

    let balance_score = if yin_hits > 0 && yang_hits > 0 {
        0.2 + (0.8 * harmony * signal_density)
    } else if total > 0 {
        -0.15 - (0.65f64).min((1.0 - harmony) * 0.7)
    } else {
        0.0
    };

    let positive_threshold = as_f64(policy.get("positive_threshold")).unwrap_or(0.3);
    let negative_threshold = as_f64(policy.get("negative_threshold")).unwrap_or(-0.2);

    let score_trit = if balance_score >= positive_threshold {
        TRIT_OK
    } else if balance_score <= negative_threshold {
        TRIT_PAIN
    } else {
        TRIT_UNKNOWN
    };

    let base_confidence = (0.2 + (0.45 * harmony) + (0.35 * signal_density)).min(1.0);
    let seed_confidence = clamp_f64(
        as_f64(state.get("seed_confidence")).unwrap_or(1.0),
        0.0,
        1.0,
    );
    let confidence = clamp_f64(base_confidence * seed_confidence, 0.0, 1.0);
    let advisory_weight = clamp_f64(
        as_f64(policy.get("advisory_weight")).unwrap_or(0.35),
        0.0,
        1.0,
    );
    let effective_weight = clamp_f64(advisory_weight * confidence, 0.0, 1.0);

    let contradiction_rate = {
        let observations = as_i64(state.get("observations_total")).unwrap_or(0) as f64;
        let contradictions = as_i64(state.get("contradictions_total")).unwrap_or(0) as f64;
        if observations > 0.0 {
            (contradictions / observations * 1_000_000.0).round() / 1_000_000.0
        } else {
            0.0
        }
    };

    let codex_version = {
        let v = as_str(codex.get("version"));
        if v.is_empty() {
            "1.0".to_string()
        } else {
            v
        }
    };
    let run_id_value = if run_id.is_empty() {
        Value::Null
    } else {
        Value::String(run_id)
    };

    json!({
        "enabled": true,
        "lane": lane,
        "lane_enabled": true,
        "advisory_only": as_bool(policy.get("advisory_only"), true),
        "shadow_only": as_bool(policy.get("shadow_only"), true),
        "score_trit": score_trit,
        "score_label": trit_label(score_trit),
        "balance_score": (balance_score * 1_000_000.0).round() / 1_000_000.0,
        "zero_point_harmony_potential": (harmony * 1_000_000.0).round() / 1_000_000.0,
        "recommended_adjustment": recommend_adjustment(yin_hits, yang_hits),
        "confidence": (confidence * 1_000_000.0).round() / 1_000_000.0,
        "advisory_weight": (advisory_weight * 1_000_000.0).round() / 1_000_000.0,
        "effective_weight": (effective_weight * 1_000_000.0).round() / 1_000_000.0,
        "seed_confidence": (seed_confidence * 1_000_000.0).round() / 1_000_000.0,
        "codex_version": codex_version,
        "codex_summary": {
            "flux_pairs": codex.get("flux_pairs").and_then(Value::as_array).map(|rows| rows.len()).unwrap_or(0),
            "flow_values": codex.get("flow_values").and_then(Value::as_array).map(|rows| rows.len()).unwrap_or(0),
            "warnings": codex.get("warnings").and_then(Value::as_array).map(|rows| rows.len()).unwrap_or(0)
        },
        "diagnostics": {
            "token_count": tokens.len(),
            "yin_hits": yin_hits,
            "yang_hits": yang_hits,
            "signal_density": (signal_density * 1_000_000.0).round() / 1_000_000.0,
            "source": source,
            "tool_context": if tool_context.is_empty() {
                Value::Null
            } else {
                Value::String(tool_context)
            }
        },
        "indicator": {
            "yin_yang_bias": if yin_hits > yang_hits {
                "yin_lean"
            } else if yang_hits > yin_hits {
                "yang_lean"
            } else {
                "balanced"
            },
            "subtle_hint": if harmony >= 0.75 {
                "near_zero_point_harmony"
            } else if harmony >= 0.45 {
                "partial_balance"
            } else {
                "high_imbalance"
            }
        },
        "zero_point_insight": if harmony >= 0.75 {
            "opposites currently reinforce each other near the 0-point"
        } else {
            "rebalance order/flux before escalating decisions"
        },
        "contradiction_tracking": {
            "observations_total": as_i64(state.get("observations_total")).unwrap_or(0),
            "contradictions_total": as_i64(state.get("contradictions_total")).unwrap_or(0),
            "contradiction_rate": contradiction_rate
        },
        "run_id": run_id_value
    })
}

fn maybe_run_self_validation(
    policy: &Value,
    state: &Value,
    policy_path: Option<&str>,
) -> Result<Value, String> {
    let interval_minutes = clamp_i64(
        as_i64(policy.get("self_validation_interval_minutes")).unwrap_or(360),
        5,
        24 * 60,
    );
    let last_run_ts = state
        .get("self_validation")
        .and_then(Value::as_object)
        .and_then(|obj| obj.get("last_run_ts"))
        .map(|v| as_str(Some(v)))
        .unwrap_or_default();

    let due = if last_run_ts.is_empty() {
        true
    } else {
        let last_run_ms = chrono::DateTime::parse_from_rfc3339(&last_run_ts)
            .ok()
            .map(|ts| ts.timestamp_millis())
            .unwrap_or(0);
        let now_ms = chrono::Utc::now().timestamp_millis();
        (now_ms - last_run_ms) >= (interval_minutes * 60 * 1000)
    };

    if !due {
        return Ok(state.clone());
    }

    let scenarios = vec![
        (
            "balanced_context",
            json!({
                "lane": "self_validation",
                "objective": "keep order and exploration in harmony with safety and creativity"
            }),
            TRIT_OK,
        ),
        (
            "yin_extreme_context",
            json!({
                "lane": "self_validation",
                "objective": "maximize rigid structure and strict control without adaptation"
            }),
            TRIT_PAIN,
        ),
        (
            "yang_extreme_context",
            json!({
                "lane": "self_validation",
                "objective": "maximize mutation and chaos without constraints or stability"
            }),
            TRIT_PAIN,
        ),
    ];

    let codex = load_codex(policy);
    let mut rows = Vec::<Value>::new();
    for (id, context, expected) in scenarios {
        let out = evaluate_signal(
            policy,
            &codex,
            state,
            &context,
            &json!({
                "source": "duality_self_validation",
                "lane": "self_validation"
            }),
        );
        let predicted = normalize_trit(out.get("score_trit"));
        let pass = predicted == expected || (expected != TRIT_OK && predicted == TRIT_UNKNOWN);
        rows.push(json!({
            "scenario_id": id,
            "expected_trit": expected,
            "predicted_trit": predicted,
            "pass": pass
        }));
    }

    let pass_count = rows
        .iter()
        .filter(|row| row.get("pass").and_then(Value::as_bool) == Some(true))
        .count();
    let confidence = pass_count as f64 / (rows.len().max(1) as f64);
    let ts = now_iso();

    let mut next = state.as_object().cloned().unwrap_or_default();
    next.insert(
        "self_validation".to_string(),
        json!({
            "last_run_ts": ts,
            "confidence": (confidence * 1_000_000.0).round() / 1_000_000.0,
            "scenario_count": rows.len()
        }),
    );

    let persisted = persist_state(policy, &Value::Object(next))?;

    let history_path = policy
        .get("state")
        .and_then(Value::as_object)
        .and_then(|obj| obj.get("history_path"))
        .map(|v| PathBuf::from(as_str(Some(v))))
        .unwrap_or_else(|| PathBuf::from(DEFAULT_HISTORY_REL));

    append_jsonl(
        &history_path,
        &json!({
            "ts": ts,
            "type": "duality_self_validation",
            "confidence": (confidence * 1_000_000.0).round() / 1_000_000.0,
            "pass_count": pass_count,
            "scenario_count": rows.len(),
            "scenarios": rows,
            "seed_confidence": as_f64(persisted.get("seed_confidence")).unwrap_or(1.0)
        }),
    )?;

    let _ = policy_path;
    Ok(persisted)
}
