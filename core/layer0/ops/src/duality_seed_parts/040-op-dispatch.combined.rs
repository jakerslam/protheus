// SRS coverage marker: V4-DUAL-VOI-001

fn as_f64_or(value: Option<&Value>, fallback: f64) -> f64 {
    as_f64(value).unwrap_or(fallback)
}

fn evaluate_dual_voice_signal(
    policy: &Value,
    codex: &Value,
    state: &Value,
    context: &Value,
    left: &Value,
    right: &Value,
    opts: &Value,
) -> Value {
    let mut left_context = context.as_object().cloned().unwrap_or_default();
    left_context.insert("voice".to_string(), Value::String("left".to_string()));
    left_context.insert(
        "voice_role".to_string(),
        Value::String("structured_reasoning".to_string()),
    );
    if let Some(obj) = left.as_object() {
        for (k, v) in obj {
            left_context.insert(k.clone(), v.clone());
        }
    }
    let mut right_context = context.as_object().cloned().unwrap_or_default();
    right_context.insert("voice".to_string(), Value::String("right".to_string()));
    right_context.insert(
        "voice_role".to_string(),
        Value::String("creative_inversion".to_string()),
    );
    if let Some(obj) = right.as_object() {
        for (k, v) in obj {
            right_context.insert(k.clone(), v.clone());
        }
    }

    let left_signal = evaluate_signal(
        policy,
        codex,
        state,
        &Value::Object(left_context),
        &json!({
            "lane": "belief_formation",
            "source": "dual_voice_left",
            "run_id": as_str(context.get("run_id"))
        }),
    );
    let right_signal = evaluate_signal(
        policy,
        codex,
        state,
        &Value::Object(right_context),
        &json!({
            "lane": "weaver_arbitration",
            "source": "dual_voice_right",
            "run_id": as_str(context.get("run_id"))
        }),
    );

    let left_trit = normalize_trit(left_signal.get("score_trit"));
    let right_trit = normalize_trit(right_signal.get("score_trit"));
    let left_confidence = clamp_f64(as_f64_or(left_signal.get("confidence"), 0.0), 0.0, 1.0);
    let right_confidence = clamp_f64(as_f64_or(right_signal.get("confidence"), 0.0), 0.0, 1.0);
    let left_harmony = clamp_f64(
        as_f64_or(left_signal.get("zero_point_harmony_potential"), 0.0),
        0.0,
        1.0,
    );
    let right_harmony = clamp_f64(
        as_f64_or(right_signal.get("zero_point_harmony_potential"), 0.0),
        0.0,
        1.0,
    );

    let voice_alignment = if left_trit != TRIT_UNKNOWN && left_trit == right_trit {
        1.0
    } else if left_trit == TRIT_UNKNOWN || right_trit == TRIT_UNKNOWN {
        0.5
    } else {
        0.0
    };
    let harmony = clamp_f64(
        (((left_harmony + right_harmony) * 0.5) * 0.7) + (voice_alignment * 0.3),
        0.0,
        1.0,
    );
    let min_harmony = clamp_f64(
        as_f64_or(
            policy
                .get("dual_voice")
                .and_then(Value::as_object)
                .and_then(|obj| obj.get("min_harmony")),
            0.42,
        ),
        0.0,
        1.0,
    );
    let min_voice_conf = clamp_f64(
        as_f64_or(
            policy
                .get("dual_voice")
                .and_then(Value::as_object)
                .and_then(|obj| obj.get("minimum_voice_confidence")),
            0.3,
        ),
        0.0,
        1.0,
    );
    let dual_voice_enabled = as_bool(
        policy
            .get("dual_voice")
            .and_then(Value::as_object)
            .and_then(|obj| obj.get("enabled")),
        true,
    );
    let min_confidence = left_confidence.min(right_confidence);
    let pass = dual_voice_enabled && harmony >= min_harmony && min_confidence >= min_voice_conf;
    let score_trit = if pass {
        TRIT_OK
    } else if harmony < (min_harmony * 0.65) {
        TRIT_PAIN
    } else {
        TRIT_UNKNOWN
    };

    let mut out = json!({
        "type": "duality_dual_voice_evaluation",
        "enabled": dual_voice_enabled,
        "score_trit": score_trit,
        "score_label": trit_label(score_trit),
        "harmony": (harmony * 1_000_000.0).round() / 1_000_000.0,
        "voice_alignment": (voice_alignment * 1_000_000.0).round() / 1_000_000.0,
        "minimum_harmony": min_harmony,
        "minimum_voice_confidence": min_voice_conf,
        "minimum_observed_confidence": (min_confidence * 1_000_000.0).round() / 1_000_000.0,
        "pass": pass,
        "recommended_adjustment": if pass {
            "hold_balance_near_zero_point"
        } else if left_trit == TRIT_PAIN || right_trit == TRIT_PAIN {
            "decrease_extreme_voice_and_rebalance"
        } else {
            "increase_cross_voice_harmony"
        },
        "left_voice": left_signal,
        "right_voice": right_signal
    });

    if let Some(run_id) = context.get("run_id").and_then(Value::as_str) {
        if !run_id.trim().is_empty() {
            out["run_id"] = Value::String(clean_text(run_id, 160));
        }
    }
    if let Some(source) = opts.get("source").and_then(Value::as_str) {
        if !source.trim().is_empty() {
            out["source"] = Value::String(normalize_token(source, 120));
        }
    }
    out
}

fn compute_toll_from_signal(policy: &Value, state: &Value, signal: &Value) -> Value {
    let toll_enabled = as_bool(policy.get("toll_enabled"), true);
    let debt_before = clamp_f64(as_f64_or(state.get("toll_debt"), 0.0), 0.0, 100.0);
    let score_trit = normalize_trit(signal.get("score_trit"));
    let balance_score = as_f64_or(signal.get("balance_score"), 0.0);
    let harmony = clamp_f64(
        as_f64_or(signal.get("zero_point_harmony_potential"), 0.0),
        0.0,
        1.0,
    );
    let trigger_negative = clamp_f64(
        as_f64_or(policy.get("toll_trigger_negative_threshold"), -0.2),
        -1.0,
        1.0,
    );
    let debt_step = clamp_f64(as_f64_or(policy.get("toll_debt_step"), 0.2), 0.0001, 10.0);
    let recovery_step = clamp_f64(
        as_f64_or(policy.get("toll_recovery_step"), 0.08),
        0.0001,
        10.0,
    );
    let hard_block_threshold = clamp_f64(
        as_f64_or(policy.get("toll_hard_block_threshold"), 1.0),
        0.1,
        100.0,
    );

    let mut debt_after = debt_before;
    if toll_enabled {
        if score_trit == TRIT_PAIN || balance_score <= trigger_negative {
            let severity = (balance_score.abs() + (1.0 - harmony)).max(0.15);
            debt_after = (debt_before + (debt_step * severity)).min(100.0);
        } else {
            let recovery = recovery_step * (1.0 + harmony);
            debt_after = (debt_before - recovery).max(0.0);
        }
    }
    let hard_block = toll_enabled && debt_after >= hard_block_threshold;

    json!({
        "enabled": toll_enabled,
        "debt_before": (debt_before * 1_000_000.0).round() / 1_000_000.0,
        "debt_after": (debt_after * 1_000_000.0).round() / 1_000_000.0,
        "trigger_negative_threshold": trigger_negative,
        "hard_block_threshold": hard_block_threshold,
        "hard_block": hard_block,
        "score_trit": score_trit,
        "score_label": trit_label(score_trit),
        "harmony": (harmony * 1_000_000.0).round() / 1_000_000.0,
        "balance_score": (balance_score * 1_000_000.0).round() / 1_000_000.0
    })
}

fn duality_memory_tags_for_content(policy: &Value, signal: &Value) -> Value {
    let tagging_enabled = as_bool(
        policy
            .get("memory")
            .and_then(Value::as_object)
            .and_then(|obj| obj.get("tagging_enabled")),
        true,
    );
    let high_recall_threshold = clamp_f64(
        as_f64_or(
            policy
                .get("memory")
                .and_then(Value::as_object)
                .and_then(|obj| obj.get("high_recall_threshold")),
            0.65,
        ),
        0.0,
        1.0,
    );
    let inversion_flag_threshold = clamp_f64(
        as_f64_or(
            policy
                .get("memory")
                .and_then(Value::as_object)
                .and_then(|obj| obj.get("inversion_flag_threshold")),
            0.35,
        ),
        0.0,
        1.0,
    );
    let score_trit = normalize_trit(signal.get("score_trit"));
    let balance_score = as_f64_or(signal.get("balance_score"), 0.0);
    let harmony = clamp_f64(
        as_f64_or(signal.get("zero_point_harmony_potential"), 0.0),
        0.0,
        1.0,
    );
    let high_recall_priority =
        tagging_enabled && score_trit == TRIT_OK && harmony >= high_recall_threshold;
    let inversion_review_flag = tagging_enabled
        && (score_trit == TRIT_PAIN
            || harmony <= inversion_flag_threshold
            || balance_score <= -inversion_flag_threshold);
    json!({
        "enabled": tagging_enabled,
        "score_trit": score_trit,
        "score_label": trit_label(score_trit),
        "balance_score": (balance_score * 1_000_000.0).round() / 1_000_000.0,
        "zero_point_harmony_potential": (harmony * 1_000_000.0).round() / 1_000_000.0,
        "high_recall_priority": high_recall_priority,
        "inversion_review_flag": inversion_review_flag,
        "recommended_adjustment": signal.get("recommended_adjustment").cloned().unwrap_or(Value::Null)
    })
}

fn op_dispatch(root: &Path, op: &str, args: Option<&Value>) -> Result<Value, String> {
    let args_obj = args.and_then(Value::as_object).cloned().unwrap_or_default();
    let policy_path = as_str(args_obj.get("policy_path"));
    let policy = load_policy(
        root,
        if policy_path.is_empty() {
            None
        } else {
            Some(policy_path.as_str())
        },
    );

    match op {
        "loadDualityPolicy" => Ok(policy),
        "parseDualityCodexText" => {
            let text = as_str(args_obj.get("text"));
            Ok(parse_codex_text(&text))
        }
        "loadDualityCodex" => Ok(load_codex(&policy)),
        "loadDualityState" => Ok(load_state(&policy)),
        "evaluateDualitySignal" | "duality_evaluate" => {
            let state = load_state(&policy);
            let opts = args_obj.get("opts").cloned().unwrap_or_else(|| json!({}));
            let skip_validation = as_bool(opts.get("skip_validation"), false);
            let state_after_validation = if skip_validation {
                state.clone()
            } else {
                maybe_run_self_validation(
                    &policy,
                    &state,
                    if policy_path.is_empty() {
                        None
                    } else {
                        Some(policy_path.as_str())
                    },
                )?
            };
            let context = args_obj
                .get("context")
                .cloned()
                .unwrap_or_else(|| json!({}));
            let out = evaluate_signal(
                &policy,
                &load_codex(&policy),
                &state_after_validation,
                &context,
                &opts,
            );

            if as_bool(opts.get("persist"), false)
                && as_bool(
                    policy
                        .get("outputs")
                        .and_then(Value::as_object)
                        .and_then(|obj| obj.get("persist_shadow_receipts")),
                    true,
                )
            {
                let history_path = policy
                    .get("state")
                    .and_then(Value::as_object)
                    .and_then(|obj| obj.get("history_path"))
                    .map(|v| PathBuf::from(as_str(Some(v))))
                    .unwrap_or_else(|| PathBuf::from(DEFAULT_HISTORY_REL));
                append_jsonl(
                    &history_path,
                    &json!({
                        "ts": now_iso(),
                        "type": "duality_evaluation",
                        "lane": out.get("lane").cloned().unwrap_or(Value::Null),
                        "run_id": out.get("run_id").cloned().unwrap_or(Value::Null),
                        "source": out
                            .get("diagnostics")
                            .and_then(Value::as_object)
                            .and_then(|obj| obj.get("source"))
                            .cloned()
                            .unwrap_or(Value::Null),
                        "score_trit": out.get("score_trit").cloned().unwrap_or(Value::Null),
                        "balance_score": out.get("balance_score").cloned().unwrap_or(Value::Null),
                        "zero_point_harmony_potential": out
                            .get("zero_point_harmony_potential")
                            .cloned()
                            .unwrap_or(Value::Null),
                        "confidence": out.get("confidence").cloned().unwrap_or(Value::Null),
                        "effective_weight": out.get("effective_weight").cloned().unwrap_or(Value::Null),
                        "recommended_adjustment": out
                            .get("recommended_adjustment")
                            .cloned()
                            .unwrap_or(Value::Null)
                    }),
                )?;
            }

            Ok(out)
        }
        "dualVoiceEvaluate" | "dual_voice_evaluate" => {
            let state = load_state(&policy);
            let opts = args_obj.get("opts").cloned().unwrap_or_else(|| json!({}));
            let skip_validation = as_bool(opts.get("skip_validation"), false);
            let state_after_validation = if skip_validation {
                state.clone()
            } else {
                maybe_run_self_validation(
                    &policy,
                    &state,
                    if policy_path.is_empty() {
                        None
                    } else {
                        Some(policy_path.as_str())
                    },
                )?
            };
            let context = args_obj
                .get("context")
                .cloned()
                .unwrap_or_else(|| json!({}));
            let left = args_obj
                .get("left")
                .or_else(|| args_obj.get("left_context"))
                .cloned()
                .unwrap_or_else(|| json!({}));
            let right = args_obj
                .get("right")
                .or_else(|| args_obj.get("right_context"))
                .cloned()
                .unwrap_or_else(|| json!({}));
            let out = evaluate_dual_voice_signal(
                &policy,
                &load_codex(&policy),
                &state_after_validation,
                &context,
                &left,
                &right,
                &opts,
            );

            if as_bool(opts.get("persist"), false)
                && as_bool(
                    policy
                        .get("outputs")
                        .and_then(Value::as_object)
                        .and_then(|obj| obj.get("persist_shadow_receipts")),
                    true,
                )
            {
                let history_path = policy
                    .get("state")
                    .and_then(Value::as_object)
                    .and_then(|obj| obj.get("history_path"))
                    .map(|v| PathBuf::from(as_str(Some(v))))
                    .unwrap_or_else(|| PathBuf::from(DEFAULT_HISTORY_REL));
                append_jsonl(
                    &history_path,
                    &json!({
                        "ts": now_iso(),
                        "type": "duality_dual_voice_evaluation",
                        "run_id": out.get("run_id").cloned().unwrap_or(Value::Null),
                        "source": out.get("source").cloned().unwrap_or(Value::Null),
                        "score_trit": out.get("score_trit").cloned().unwrap_or(Value::Null),
                        "harmony": out.get("harmony").cloned().unwrap_or(Value::Null),
                        "pass": out.get("pass").cloned().unwrap_or(Value::Bool(false))
                    }),
                )?;
            }

            Ok(out)
        }
        "computeDualityToll" | "duality_toll" | "duality_toll_update" => {
            let state = load_state(&policy);
            let opts = args_obj.get("opts").cloned().unwrap_or_else(|| json!({}));
            let context = args_obj
                .get("context")
                .cloned()
                .unwrap_or_else(|| json!({}));
            let signal = args_obj.get("signal").cloned().unwrap_or_else(|| {
                evaluate_signal(
                    &policy,
                    &load_codex(&policy),
                    &state,
                    &context,
                    &json!({
                        "lane": "weaver_arbitration",
                        "source": "duality_toll",
                        "run_id": as_str(context.get("run_id"))
                    }),
                )
            });
            let toll = compute_toll_from_signal(&policy, &state, &signal);
            let debt_after = clamp_f64(as_f64_or(toll.get("debt_after"), 0.0), 0.0, 100.0);
            let mut next = state.as_object().cloned().unwrap_or_default();
            next.insert(
                "toll_debt".to_string(),
                json!((debt_after * 1_000_000.0).round() / 1_000_000.0),
            );
            next.insert(
                "toll_events_total".to_string(),
                json!(as_i64(state.get("toll_events_total")).unwrap_or(0) + 1),
            );
            next.insert("last_toll_update_ts".to_string(), Value::String(now_iso()));
            let persisted = if as_bool(opts.get("persist"), true) {
                persist_state(&policy, &Value::Object(next))?
            } else {
                Value::Object(next)
            };
            let out = json!({
                "ok": true,
                "type": "duality_toll_update",
                "signal": signal,
                "toll": toll,
                "state": {
                    "toll_debt": persisted.get("toll_debt").cloned().unwrap_or(Value::Null),
                    "toll_events_total": persisted.get("toll_events_total").cloned().unwrap_or(Value::Null),
                    "last_toll_update_ts": persisted.get("last_toll_update_ts").cloned().unwrap_or(Value::Null)
                }
            });
            if as_bool(opts.get("persist"), true)
                && as_bool(
                    policy
                        .get("outputs")
                        .and_then(Value::as_object)
                        .and_then(|obj| obj.get("persist_shadow_receipts")),
                    true,
                )
            {
                let history_path = policy
                    .get("state")
                    .and_then(Value::as_object)
                    .and_then(|obj| obj.get("history_path"))
                    .map(|v| PathBuf::from(as_str(Some(v))))
                    .unwrap_or_else(|| PathBuf::from(DEFAULT_HISTORY_REL));
                append_jsonl(
                    &history_path,
                    &json!({
                        "ts": now_iso(),
                        "type": "duality_toll_update",
                        "run_id": context.get("run_id").cloned().unwrap_or(Value::Null),
                        "lane": signal.get("lane").cloned().unwrap_or(Value::Null),
                        "debt_after": toll.get("debt_after").cloned().unwrap_or(Value::Null),
                        "hard_block": toll.get("hard_block").cloned().unwrap_or(Value::Bool(false)),
                        "score_trit": toll.get("score_trit").cloned().unwrap_or(Value::Null)
                    }),
                )?;
            }
            Ok(out)
        }
        "tagDualityMemoryNode" | "duality_memory_tag" | "tagDualityMemoryNodes" => {
            let state = load_state(&policy);
            let opts = args_obj.get("opts").cloned().unwrap_or_else(|| json!({}));
            let run_id = as_str(opts.get("run_id"));
            let source = {
                let v = normalize_token(&as_str(opts.get("source")), 120);
                if v.is_empty() {
                    "duality_memory_tag".to_string()
                } else {
                    v
                }
            };
            let nodes = args_obj
                .get("nodes")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_else(|| {
                    vec![json!({
                        "key": as_str(args_obj.get("key")),
                        "value": args_obj.get("value").cloned().unwrap_or(Value::Null)
                    })]
                });
            let mut tagged = Vec::<Value>::new();
            for node in nodes {
                let key = clean_text(&as_str(node.get("key")), 160);
                let value = node.get("value").cloned().unwrap_or(Value::Null);
                let context = json!({
                    "lane": "task_decomposition",
                    "run_id": run_id,
                    "source": source,
                    "memory_key": key,
                    "memory_value": value
                });
                let signal = node
                    .get("signal")
                    .filter(|candidate| candidate.is_object())
                    .cloned()
                    .unwrap_or_else(|| {
                        evaluate_signal(
                            &policy,
                            &load_codex(&policy),
                            &state,
                            &context,
                            &json!({
                                "lane": "task_decomposition",
                                "source": "duality_memory_tag",
                                "run_id": run_id
                            }),
                        )
                    });
                let tags = duality_memory_tags_for_content(&policy, &signal);
                tagged.push(json!({
                    "key": key,
                    "value": value,
                    "signal": signal,
                    "duality_tags": tags
                }));
            }
            Ok(json!({
                "ok": true,
                "type": "duality_memory_tag",
                "nodes": tagged
            }))
        }
        "registerDualityObservation" => {
            let state = load_state(&policy);
            let input = args_obj.get("input").cloned().unwrap_or_else(|| json!({}));
            let input_obj = input.as_object().cloned().unwrap_or_default();

            let predicted = normalize_trit(input_obj.get("predicted_trit"));
            let observed = normalize_trit(input_obj.get("observed_trit"));
            let lane = normalize_token(&as_str(input_obj.get("lane")), 120);
            let lane = if lane.is_empty() {
                "unknown_lane".to_string()
            } else {
                lane
            };
            let run_id = {
                let v = as_str(input_obj.get("run_id"));
                if v.is_empty() {
                    Value::Null
                } else {
                    Value::String(v)
                }
            };
            let source = {
                let v = normalize_token(&as_str(input_obj.get("source")), 120);
                if v.is_empty() {
                    "runtime".to_string()
                } else {
                    v
                }
            };

            let contradiction = predicted != 0 && observed != 0 && predicted != observed;
            let support = predicted != 0 && observed != 0 && predicted == observed;
            let neutral = !contradiction && !support;

            let min_seed_confidence = clamp_f64(
                as_f64(policy.get("minimum_seed_confidence")).unwrap_or(0.25),
                0.0,
                1.0,
            );
            let decay_step = clamp_f64(
                as_f64(policy.get("contradiction_decay_step")).unwrap_or(0.04),
                0.0001,
                1.0,
            );
            let recovery_step = clamp_f64(
                as_f64(policy.get("support_recovery_step")).unwrap_or(0.01),
                0.0001,
                1.0,
            );

            let mut seed_confidence = clamp_f64(
                as_f64(state.get("seed_confidence")).unwrap_or(1.0),
                0.0,
                1.0,
            );
            let mut consecutive_contradictions =
                as_i64(state.get("consecutive_contradictions")).unwrap_or(0);
            let mut consecutive_supports = as_i64(state.get("consecutive_supports")).unwrap_or(0);

            if contradiction {
                consecutive_contradictions += 1;
                consecutive_supports = 0;
                let dynamic =
                    decay_step * (1.0 + ((consecutive_contradictions.min(12) as f64) * 0.12));
                seed_confidence = (seed_confidence - dynamic).max(min_seed_confidence);
            } else if support {
                consecutive_supports += 1;
                consecutive_contradictions = 0;
                let dynamic =
                    recovery_step * (1.0 + ((consecutive_supports.min(12) as f64) * 0.06));
                seed_confidence = (seed_confidence + dynamic).min(1.0);
            } else {
                consecutive_contradictions = 0;
                consecutive_supports = 0;
            }

            let ts = now_iso();
            let observation = json!({
                "ts": ts,
                "lane": lane,
                "run_id": run_id,
                "source": source,
                "predicted_trit": predicted,
                "observed_trit": observed,
                "contradiction": contradiction,
                "support": support,
                "neutral": neutral
            });

            let max_window = clamp_i64(
                as_i64(policy.get("max_observation_window")).unwrap_or(200),
                10,
                20_000,
            ) as usize;
            let mut window = state
                .get("observation_window")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            if window.len() >= max_window {
                let keep = max_window.saturating_sub(1);
                let trim = window.len().saturating_sub(keep);
                if trim > 0 {
                    window.drain(0..trim);
                }
            }
            window.push(observation.clone());

            let mut next = state.as_object().cloned().unwrap_or_default();
            next.insert(
                "seed_confidence".to_string(),
                json!((seed_confidence * 1_000_000.0).round() / 1_000_000.0),
            );
            next.insert(
                "observations_total".to_string(),
                json!(as_i64(state.get("observations_total")).unwrap_or(0) + 1),
            );
            next.insert(
                "contradictions_total".to_string(),
                json!(
                    as_i64(state.get("contradictions_total")).unwrap_or(0)
                        + if contradiction { 1 } else { 0 }
                ),
            );
            next.insert(
                "supports_total".to_string(),
                json!(
                    as_i64(state.get("supports_total")).unwrap_or(0) + if support { 1 } else { 0 }
                ),
            );
            next.insert(
                "neutral_total".to_string(),
                json!(
                    as_i64(state.get("neutral_total")).unwrap_or(0) + if neutral { 1 } else { 0 }
                ),
            );
            next.insert(
                "consecutive_contradictions".to_string(),
                json!(consecutive_contradictions),
            );
            next.insert(
                "consecutive_supports".to_string(),
                json!(consecutive_supports),
            );
            next.insert("observation_window".to_string(), Value::Array(window));

            let persisted = persist_state(&policy, &Value::Object(next))?;

            if as_bool(
                policy
                    .get("outputs")
                    .and_then(Value::as_object)
                    .and_then(|obj| obj.get("persist_observations")),
                true,
            ) {
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
                        "type": "duality_observation",
                        "lane": lane,
                        "run_id": observation.get("run_id").cloned().unwrap_or(Value::Null),
                        "source": source,
                        "predicted_trit": predicted,
                        "observed_trit": observed,
                        "contradiction": contradiction,
                        "support": support,
                        "seed_confidence": persisted.get("seed_confidence").cloned().unwrap_or(Value::Null)
                    }),
                )?;
            }

            Ok(json!({
                "ok": true,
                "type": "duality_observation",
                "lane": lane,
                "contradiction": contradiction,
                "support": support,
                "neutral": neutral,
                "seed_confidence": persisted.get("seed_confidence").cloned().unwrap_or(Value::Null),
                "observations_total": persisted.get("observations_total").cloned().unwrap_or(Value::Null),
                "contradictions_total": persisted.get("contradictions_total").cloned().unwrap_or(Value::Null)
            }))
        }
        "quarantineDualitySeed" => {
            let state = load_state(&policy);
            let input = args_obj.get("input").cloned().unwrap_or_else(|| json!({}));
            let input_obj = input.as_object().cloned().unwrap_or_default();
            let reason = {
                let value = as_str(input_obj.get("reason"));
                if value.is_empty() {
                    "quarantine_requested".to_string()
                } else {
                    clean_text(&value, 220)
                }
            };
            let actor = {
                let value = normalize_token(&as_str(input_obj.get("actor")), 120);
                if value.is_empty() {
                    "unknown_actor".to_string()
                } else {
                    value
                }
            };
            let min_seed = as_f64(policy.get("minimum_seed_confidence")).unwrap_or(0.25);
            let requested_seed = as_f64(input_obj.get("seed_confidence")).unwrap_or(min_seed);
            let ts = now_iso();

            let mut next = state.as_object().cloned().unwrap_or_default();
            next.insert(
                "seed_confidence".to_string(),
                json!(clamp_f64(requested_seed, 0.0, 1.0)),
            );
            next.insert(
                "quarantine".to_string(),
                json!({
                    "active": true,
                    "ts": ts,
                    "reason": reason,
                    "actor": actor
                }),
            );

            let persisted = persist_state(&policy, &Value::Object(next))?;
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
                    "type": "duality_seed_quarantine",
                    "reason": reason,
                    "actor": actor,
                    "seed_confidence": persisted.get("seed_confidence").cloned().unwrap_or(Value::Null)
                }),
            )?;

            Ok(json!({
                "ok": true,
                "type": "duality_seed_quarantine",
                "ts": ts,
                "reason": reason,
                "actor": actor,
                "seed_confidence": persisted.get("seed_confidence").cloned().unwrap_or(Value::Null)
            }))
        }
        "maybeRunSelfValidation" => {
            let state = load_state(&policy);
            let out = maybe_run_self_validation(
                &policy,
                &state,
                if policy_path.is_empty() {
                    None
                } else {
                    Some(policy_path.as_str())
                },
            )?;
            Ok(out)
        }
        _ => Err(format!("duality_seed_unknown_op:{op}")),
    }
}


pub fn invoke(root: &Path, op: &str, args: Option<&Value>) -> Result<Value, String> {
    op_dispatch(root, op, args)
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let cmd = argv
        .first()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());

    if matches!(cmd.as_str(), "help" | "--help" | "-h") {
        println!("Usage:");
        println!("  protheus-ops duality-seed status");
        println!("  protheus-ops duality-seed invoke --payload=<json>");
        return 0;
    }

    if cmd == "status" {
        let mut out = json!({
            "ok": true,
            "type": "duality_seed_status",
            "authority": "core/layer2/autonomy",
            "commands": ["status", "invoke"],
            "default_policy_path": DEFAULT_POLICY_REL,
            "default_codex_path": DEFAULT_CODEX_REL,
            "default_latest_state_path": DEFAULT_LATEST_REL,
            "default_history_path": DEFAULT_HISTORY_REL,
            "ts": now_iso(),
            "root": clean(root.display(), 280)
        });
        out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
        print_json_line(&out);
        return 0;
    }

    if cmd != "invoke" {
        let mut out = json!({
            "ok": false,
            "type": "duality_seed_cli_error",
            "authority": "core/layer2/autonomy",
            "command": cmd,
            "error": "unknown_command",
            "ts": now_iso(),
            "exit_code": 2
        });
        out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
        print_json_line(&out);
        return 2;
    }

    let payload = match load_payload(argv) {
        Ok(value) => value,
        Err(err) => {
            let mut out = json!({
                "ok": false,
                "type": "duality_seed_cli_error",
                "authority": "core/layer2/autonomy",
                "command": "invoke",
                "error": err,
                "ts": now_iso(),
                "exit_code": 2
            });
            out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
            print_json_line(&out);
            return 2;
        }
    };

    let op = payload
        .get("op")
        .map(|v| as_str(Some(v)))
        .filter(|v| !v.is_empty())
        .unwrap_or_default();

    let result = op_dispatch(root, op.as_str(), payload.get("args"));
    match result {
        Ok(result_value) => {
            let mut out = json!({
                "ok": true,
                "type": "duality_seed",
                "authority": "core/layer2/autonomy",
                "command": "invoke",
                "op": op,
                "result": result_value,
                "ts": now_iso(),
                "root": clean(root.display(), 280)
            });
            out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
            print_json_line(&out);
            0
        }
        Err(err) => {
            let mut out = json!({
                "ok": false,
                "type": "duality_seed",
                "authority": "core/layer2/autonomy",
                "command": "invoke",
                "op": op,
                "error": err,
                "ts": now_iso(),
                "exit_code": 2
            });
            out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
            print_json_line(&out);
            2
        }
    }
}

#[cfg(test)]
mod duality_v4_tests {
    use super::*;

    #[test]
    fn dual_voice_evaluate_emits_harmony_contract() {
        let root = tempfile::tempdir().expect("tempdir");
        let out = op_dispatch(
            root.path(),
            "dual_voice_evaluate",
            Some(&json!({
                "context": {
                    "run_id": "dual-voice-test",
                    "objective": "maintain order and creativity in balance"
                },
                "left": {
                    "objective": "structured planning and safety discipline"
                },
                "right": {
                    "objective": "creative adaptation and inversion exploration"
                }
            })),
        )
        .expect("dual voice");
        assert_eq!(
            out.get("type").and_then(Value::as_str),
            Some("duality_dual_voice_evaluation")
        );
        assert!(out.get("harmony").and_then(Value::as_f64).is_some());
        assert!(out.get("left_voice").is_some());
        assert!(out.get("right_voice").is_some());
    }

    #[test]
    fn duality_toll_update_increases_debt_for_negative_signal() {
        let root = tempfile::tempdir().expect("tempdir");
        let out = op_dispatch(
            root.path(),
            "duality_toll",
            Some(&json!({
                "signal": {
                    "score_trit": -1,
                    "balance_score": -0.72,
                    "zero_point_harmony_potential": 0.08,
                    "lane": "spine"
                },
                "context": {
                    "run_id": "toll-test"
                },
                "opts": {
                    "persist": true
                }
            })),
        )
        .expect("toll");
        let debt_before = out
            .pointer("/toll/debt_before")
            .and_then(Value::as_f64)
            .unwrap_or(0.0);
        let debt_after = out
            .pointer("/toll/debt_after")
            .and_then(Value::as_f64)
            .unwrap_or(0.0);
        assert!(debt_after >= debt_before);
        let state = op_dispatch(root.path(), "loadDualityState", None).expect("state");
        assert!(
            state
                .get("toll_debt")
                .and_then(Value::as_f64)
                .unwrap_or(0.0)
                >= debt_after - 0.000001
        );
    }

    #[test]
    fn duality_toll_update_recovers_debt_for_balanced_signal() {
        let root = tempfile::tempdir().expect("tempdir");
        let _ = op_dispatch(
            root.path(),
            "duality_toll",
            Some(&json!({
                "signal": {
                    "score_trit": -1,
                    "balance_score": -0.81,
                    "zero_point_harmony_potential": 0.05
                },
                "opts": {"persist": true}
            })),
        )
        .expect("seed debt");

        let out = op_dispatch(
            root.path(),
            "duality_toll",
            Some(&json!({
                "signal": {
                    "score_trit": 1,
                    "balance_score": 0.88,
                    "zero_point_harmony_potential": 0.92
                },
                "opts": {"persist": true}
            })),
        )
        .expect("recover debt");
        let debt_before = out
            .pointer("/toll/debt_before")
            .and_then(Value::as_f64)
            .unwrap_or(0.0);
        let debt_after = out
            .pointer("/toll/debt_after")
            .and_then(Value::as_f64)
            .unwrap_or(0.0);
        assert!(debt_after <= debt_before);
    }

    #[test]
    fn duality_memory_tag_marks_extremes_for_review() {
        let root = tempfile::tempdir().expect("tempdir");
        let out = op_dispatch(
            root.path(),
            "duality_memory_tag",
            Some(&json!({
                "nodes": [
                    {
                        "key": "focus.mode",
                        "value": "maximize rigid structure and strict control without adaptation",
                        "signal": {
                            "score_trit": -1,
                            "balance_score": -0.78,
                            "zero_point_harmony_potential": 0.09,
                            "recommended_adjustment": "increase_yin_order"
                        }
                    }
                ]
            })),
        )
        .expect("memory tag");
        let first = out
            .get("nodes")
            .and_then(Value::as_array)
            .and_then(|rows| rows.first())
            .cloned()
            .unwrap_or_else(|| json!({}));
        assert_eq!(
            first
                .pointer("/duality_tags/inversion_review_flag")
                .and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn parse_codex_text_dedupes_flow_values_while_preserving_order() {
        let parsed = parse_codex_text(
            r#"
            [flow_values]
            observe/reflect, fetch/parse; observe/reflect
            "#,
        );
        assert_eq!(
            parsed.get("flow_values").cloned().unwrap_or(Value::Null),
            json!(["observe/reflect", "fetch/parse"])
        );
    }
}

