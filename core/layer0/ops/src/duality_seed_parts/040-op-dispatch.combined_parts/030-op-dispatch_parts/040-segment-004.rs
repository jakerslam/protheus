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
