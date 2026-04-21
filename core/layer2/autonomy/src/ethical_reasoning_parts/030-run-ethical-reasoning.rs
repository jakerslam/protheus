
pub fn run_ethical_reasoning(
    root: &Path,
    input: &Value,
    explicit_policy_path: Option<&Path>,
    explicit_state_dir: Option<&Path>,
    persist: bool,
) -> Value {
    let policy = load_policy(root, explicit_policy_path);
    let paths = resolve_runtime_paths(root, explicit_state_dir);
    let ts = input
        .get("ts")
        .and_then(Value::as_str)
        .map(|v| clean_text(v, 80))
        .filter(|v| !v.is_empty())
        .unwrap_or_else(now_iso);
    let run_id = input
        .get("run_id")
        .and_then(Value::as_str)
        .map(|v| normalize_token(v, 120))
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| format!("eth_{}", hash10(&ts)));

    if !policy.enabled {
        return json!({
            "ok": false,
            "type": "ethical_reasoning_run",
            "ts": ts,
            "run_id": run_id,
            "error": "policy_disabled"
        });
    }

    let weaver_payload = input
        .get("weaver_payload")
        .cloned()
        .filter(|v| v.is_object())
        .unwrap_or_else(|| read_json(&policy.weaver_latest_path));
    let mirror_payload = input
        .get("mirror_payload")
        .cloned()
        .filter(|v| v.is_object())
        .unwrap_or_else(|| read_json(&policy.mirror_latest_path));
    let maturity_score = clamp_num(
        input
            .get("maturity_score")
            .and_then(Value::as_f64)
            .unwrap_or(0.5),
        0.0,
        1.0,
        0.5,
    );

    let allocations = normalize_allocations(&weaver_payload);
    let top = allocations.first().cloned();
    let top_share = clamp_num(
        top.as_ref()
            .and_then(|m| m.get("share"))
            .and_then(Value::as_f64)
            .unwrap_or(0.0),
        0.0,
        1.0,
        0.0,
    );
    let mirror_pressure = clamp_num(
        mirror_payload
            .get("pressure_score")
            .and_then(Value::as_f64)
            .unwrap_or(0.0),
        0.0,
        1.0,
        0.0,
    );

    let mut reason_codes = Vec::<String>::new();
    let mut correction_actions = Vec::<Value>::new();

    if top_share >= policy.monoculture_warn_share {
        reason_codes.push("ethical_monoculture_warning".to_string());
        correction_actions.push(json!({
            "action": "rebalance_value_allocations",
            "reason": "top_metric_share_exceeded",
            "top_metric_id": top.as_ref().and_then(|m| m.get("metric_id")).cloned().unwrap_or(Value::Null),
            "top_share": round_to(top_share, 6)
        }));
    }
    if mirror_pressure >= policy.mirror_pressure_warn {
        reason_codes.push("ethical_mirror_pressure_warning".to_string());
        correction_actions.push(json!({
            "action": "increase_reflection_weight",
            "reason": "mirror_pressure_high",
            "mirror_pressure": round_to(mirror_pressure, 6)
        }));
    }

    let mut tradeoff_receipts = Vec::<Value>::new();
    if top_share >= policy.high_impact_share {
        let alternatives: Vec<Value> = allocations
            .iter()
            .skip(1)
            .take(3)
            .map(|row| {
                json!({
                    "metric_id": row.get("metric_id").cloned().unwrap_or(Value::Null),
                    "share": row.get("share").cloned().unwrap_or(json!(0.0))
                })
            })
            .collect();
        tradeoff_receipts.push(json!({
            "receipt_id": format!("ethrcpt_{}", hash10(&format!("{}|{}|{}", run_id, top.as_ref().and_then(|m| m.get("metric_id")).and_then(Value::as_str).unwrap_or("unknown"), top_share))),
            "ts": now_iso(),
            "objective_id": input
                .get("objective_id")
                .or_else(|| weaver_payload.get("objective_id"))
                .and_then(Value::as_str)
                .map(|s| clean_text(s, 120)),
            "selected_metric_id": top.as_ref().and_then(|m| m.get("metric_id")).cloned().unwrap_or(Value::Null),
            "selected_share": round_to(top_share, 6),
            "alternatives": alternatives,
            "ethical_basis": [
                "constitution_sovereignty_preserved",
                "monoculture_checked",
                "mirror_pressure_considered"
            ],
            "high_impact": true
        }));
    }

    let current_priors = load_priors(&paths.priors_state_path, &policy.value_priors);
    let mut next_priors = current_priors.clone();
    let mut priors_updated = false;

    if maturity_score >= policy.maturity_min_for_prior_updates && !allocations.is_empty() {
        for row in &allocations {
            let key = row
                .get("metric_id")
                .and_then(Value::as_str)
                .map(|s| normalize_token(s, 80))
                .unwrap_or_default();
            if key.is_empty() {
                continue;
            }
            let current = *next_priors.get(&key).unwrap_or(&0.0);
            let target = clamp_num(
                row.get("share").and_then(Value::as_f64).unwrap_or(0.0),
                0.0,
                1.0,
                0.0,
            );
            let delta = clamp_num(
                target - current,
                -policy.max_prior_delta_per_run,
                policy.max_prior_delta_per_run,
                0.0,
            );
            let updated = round_to(current + delta, 6);
            next_priors.insert(key, updated);
            if delta.abs() > 0.0005 {
                priors_updated = true;
            }
        }
    } else {
        reason_codes.push("ethical_prior_update_maturity_gate".to_string());
    }

    let normalized_priors = normalize_priors(&next_priors);

    let summary = json!({
        "top_metric_id": top.as_ref().and_then(|m| m.get("metric_id")).cloned().unwrap_or(Value::Null),
        "top_share": round_to(top_share, 6),
        "mirror_pressure": round_to(mirror_pressure, 6),
        "maturity_score": round_to(maturity_score, 6),
        "monoculture_warning": top_share >= policy.monoculture_warn_share,
        "priors_updated": priors_updated
    });

    let payload = json!({
        "ok": true,
        "type": "ethical_reasoning_run",
        "ts": ts,
        "run_id": run_id,
        "policy": {
            "version": policy.version,
            "shadow_only": policy.shadow_only
        },
        "objective_id": input
            .get("objective_id")
            .or_else(|| weaver_payload.get("objective_id"))
            .and_then(Value::as_str)
            .map(|s| clean_text(s, 120)),
        "summary": summary,
        "reason_codes": reason_codes,
        "correction_actions": correction_actions,
        "tradeoff_receipts": tradeoff_receipts,
        "value_priors": normalized_priors
    });

    if persist {
        let _ = write_json_atomic(&paths.latest_path, &payload);
        let _ = append_jsonl(
            &paths.history_path,
            &json!({
                "ts": payload.get("ts").cloned().unwrap_or(Value::Null),
                "type": "ethical_reasoning_history",
                "run_id": payload.get("run_id").cloned().unwrap_or(Value::Null),
                "objective_id": payload.get("objective_id").cloned().unwrap_or(Value::Null),
                "reason_codes": payload.get("reason_codes").cloned().unwrap_or(json!([])),
                "top_metric_id": summary.get("top_metric_id").cloned().unwrap_or(Value::Null),
                "top_share": summary.get("top_share").cloned().unwrap_or(json!(0.0)),
                "priors_updated": summary.get("priors_updated").cloned().unwrap_or(json!(false))
            }),
        );
        if let Some(rows) = payload.get("tradeoff_receipts").and_then(Value::as_array) {
            for row in rows {
                let _ = append_jsonl(
                    &paths.receipts_path,
                    &json!({
                        "ts": payload.get("ts").cloned().unwrap_or(Value::Null),
                        "run_id": payload.get("run_id").cloned().unwrap_or(Value::Null),
                        "receipt": row
                    }),
                );
            }
        }
        if priors_updated {
            let priors_map: Map<String, Value> = normalized_priors
                .iter()
                .map(|(k, v)| (k.clone(), json!(*v)))
                .collect();
            let _ = write_json_atomic(
                &paths.priors_state_path,
                &json!({
                    "schema_id": "ethical_value_priors",
                    "schema_version": "1.0",
                    "ts": payload.get("ts").cloned().unwrap_or(Value::Null),
                    "run_id": payload.get("run_id").cloned().unwrap_or(Value::Null),
                    "priors": priors_map
                }),
            );
        }
    }

    payload
}
