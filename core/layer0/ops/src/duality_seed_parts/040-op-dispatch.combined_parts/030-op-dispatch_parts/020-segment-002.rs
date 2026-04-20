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

