fn persist_autonomy_run_row(
    root: &Path,
    argv: &[String],
    receipt: &Value,
) -> Result<Value, String> {
    let ts = receipt
        .get("ts")
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(now_iso);
    let day = today_ymd(&ts);
    let max_actions = parse_flag(argv, "max-actions")
        .and_then(|v| v.parse::<i64>().ok())
        .map(|v| v.clamp(1, 100))
        .unwrap_or(1);
    let objective_id = parse_flag(argv, "objective").unwrap_or_else(|| "default".to_string());
    let result = parse_flag(argv, "result").unwrap_or_else(|| "executed".to_string());
    let outcome = parse_flag(argv, "outcome").unwrap_or_else(|| {
        if result.eq_ignore_ascii_case("executed") {
            "no_change".to_string()
        } else {
            "blocked".to_string()
        }
    });
    let policy_hold_reason = parse_flag(argv, "policy-hold-reason");
    let route_block_reason = parse_flag(argv, "route-block-reason");
    let policy_hold = parse_bool(parse_flag(argv, "policy-hold").as_deref(), false)
        || result
            .to_ascii_lowercase()
            .starts_with("no_candidates_policy_");
    let duality = receipt
        .get("duality")
        .and_then(Value::as_object)
        .map(|bundle| {
            json!({
                "toll": bundle.get("toll").cloned().unwrap_or(Value::Null),
                "dual_voice": bundle.get("dual_voice").cloned().unwrap_or(Value::Null),
                "fractal_balance_score": bundle
                    .get("fractal_balance_score")
                    .cloned()
                    .unwrap_or(Value::Null)
            })
        })
        .unwrap_or_else(|| {
            json!({
                "toll": Value::Null,
                "dual_voice": Value::Null,
                "fractal_balance_score": Value::Null
            })
        });
    let row = json!({
        "ts": ts,
        "type": "autonomy_run",
        "lane": LANE_ID,
        "command": "run",
        "objective_id": objective_id,
        "max_actions": max_actions,
        "result": result,
        "outcome": outcome,
        "policy_hold": policy_hold,
        "policy_hold_reason": policy_hold_reason,
        "route_block_reason": route_block_reason,
        "receipt_hash": receipt.get("receipt_hash").cloned().unwrap_or(Value::Null),
        "duality": duality
    });
    append_jsonl(&autonomy_runs_path(root, &day), &row)?;
    Ok(row)
}

fn load_domain_constraints(root: &Path) -> Value {
    read_json(
        &root
            .join("client")
            .join("runtime")
            .join("config")
            .join("agent_domain_constraints.json"),
    )
    .unwrap_or_else(|| {
        json!({
            "allowed_domains": ["general", "finance", "healthcare", "enterprise", "research"],
            "deny_without_policy": true
        })
    })
}

fn load_provider_policy(root: &Path) -> Value {
    read_json(
        &root
            .join("client")
            .join("runtime")
            .join("config")
            .join("hand_provider_policy.json"),
    )
    .unwrap_or_else(|| {
        json!({
            "allowed_providers": ["bitnet", "openai", "frontier_provider", "local-moe"],
            "default_provider": "bitnet",
            "max_cost_per_cycle_usd": 0.50
        })
    })
}

fn as_f64(value: Option<&Value>, fallback: f64) -> f64 {
    value.and_then(Value::as_f64).unwrap_or(fallback)
}

fn autonomy_duality_clearance_tier(toll: &Value, harmony: f64) -> i64 {
    let hard_block = toll
        .get("hard_block")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if hard_block {
        return 1;
    }
    let debt_after = as_f64(toll.get("debt_after"), 0.0).clamp(0.0, 100.0);
    if debt_after >= 0.75 {
        2
    } else if debt_after <= 0.2 && harmony >= 0.85 {
        4
    } else {
        3
    }
}

fn autonomy_duality_bundle(
    root: &Path,
    lane: &str,
    source: &str,
    run_id: &str,
    context: &Value,
    persist: bool,
) -> Value {
    let mut base_context = serde_json::Map::new();
    base_context.insert("lane".to_string(), Value::String(lane.to_string()));
    base_context.insert("source".to_string(), Value::String(source.to_string()));
    base_context.insert("run_id".to_string(), Value::String(run_id.to_string()));
    if let Some(obj) = context.as_object() {
        for (k, v) in obj {
            base_context.insert(k.clone(), v.clone());
        }
    }

    let evaluation = match crate::duality_seed::invoke(
        root,
        "duality_evaluate",
        Some(&json!({
            "context": Value::Object(base_context.clone()),
            "opts": {
                "persist": persist,
                "lane": lane,
                "source": source,
                "run_id": run_id
            }
        })),
    ) {
        Ok(value) => value,
        Err(err) => {
            return json!({
                "ok": false,
                "type": "autonomy_duality_bundle",
                "error": format!("duality_evaluate_failed:{err}")
            });
        }
    };

    let dual_voice = crate::duality_seed::invoke(
        root,
        "dual_voice_evaluate",
        Some(&json!({
            "context": Value::Object(base_context.clone()),
            "left": {
                "policy_lens": "guardian",
                "focus": "structured_reasoning"
            },
            "right": {
                "policy_lens": "strategist",
                "focus": "creative_inversion"
            },
            "opts": {
                "persist": persist,
                "source": source,
                "run_id": run_id
            }
        })),
    )
    .unwrap_or_else(|_| json!({"ok": false, "type": "duality_dual_voice_evaluation"}));

    let toll_update = match crate::duality_seed::invoke(
        root,
        "duality_toll_update",
        Some(&json!({
            "context": Value::Object(base_context),
            "signal": evaluation.clone(),
            "opts": {
                "persist": persist,
                "source": source,
                "run_id": run_id
            }
        })),
    ) {
        Ok(value) => value,
        Err(err) => {
            return json!({
                "ok": false,
                "type": "autonomy_duality_bundle",
                "evaluation": evaluation,
                "dual_voice": dual_voice,
                "error": format!("duality_toll_update_failed:{err}")
            });
        }
    };

    let toll = toll_update
        .get("toll")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let harmony = as_f64(
        dual_voice.get("harmony"),
        as_f64(evaluation.get("zero_point_harmony_potential"), 0.0),
    )
    .clamp(0.0, 1.0);
    let debt_after = as_f64(toll.get("debt_after"), 0.0).clamp(0.0, 100.0);
    let hard_block = toll
        .get("hard_block")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let recommended_clearance_tier = autonomy_duality_clearance_tier(&toll, harmony);

    json!({
        "ok": true,
        "type": "autonomy_duality_bundle",
        "lane": lane,
        "source": source,
        "run_id": run_id,
        "evaluation": evaluation,
        "dual_voice": dual_voice,
        "toll": toll,
        "state": toll_update.get("state").cloned().unwrap_or(Value::Null),
        "hard_block": hard_block,
        "recommended_clearance_tier": recommended_clearance_tier,
        "fractal_balance_score": ((harmony * (1.0 - debt_after.min(1.0))) * 1_000_000.0).round() / 1_000_000.0
    })
}

fn autonomy_duality_hard_block(duality: &Value) -> bool {
    duality
        .get("hard_block")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn conduit_guard(argv: &[String], strict: bool) -> Option<Value> {
    if strict && parse_bool(parse_flag(argv, "bypass").as_deref(), false) {
        Some(json!({
            "ok": false,
            "type": "autonomy_controller_conduit_gate",
            "lane": LANE_ID,
            "strict": strict,
            "error": "conduit_bypass_rejected",
            "claim_evidence": [
                {
                    "id": "V8-AGENT-ERA-001.5",
                    "claim": "all_ephemeral_and_hand_operations_route_through_conduit_with_fail_closed_boundary",
                    "evidence": {"bypass_requested": true}
                }
            ]
        }))
    } else {
        None
    }
}

fn emit_receipt(root: &Path, value: &mut Value) -> i32 {
    if let Some(map) = value.as_object_mut() {
        map.remove("receipt_hash");
    }
    value["receipt_hash"] = Value::String(receipt_hash(value));
    match write_receipt(root, STATE_ENV, STATE_SCOPE, value.clone()) {
        Ok(out) => {
            let ok = out.get("ok").and_then(Value::as_bool).unwrap_or(false);
            print_json_line(&out);
            if ok {
                0
            } else {
                1
            }
        }
        Err(err) => {
            let mut out = json!({
                "ok": false,
                "type": "autonomy_controller_error",
                "lane": LANE_ID,
                "error": err
            });
            out["receipt_hash"] = Value::String(receipt_hash(&out));
            print_json_line(&out);
            1
        }
    }
}
