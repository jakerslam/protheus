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

