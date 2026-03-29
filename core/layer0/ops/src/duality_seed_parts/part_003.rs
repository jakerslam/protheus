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
                window = window
                    .into_iter()
                    .rev()
                    .take(keep)
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .collect();
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

