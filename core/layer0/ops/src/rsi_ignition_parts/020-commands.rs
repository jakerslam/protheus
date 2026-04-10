fn emit_state_write_error(root: &Path, event_type: &str, err: String) -> i32 {
    emit(
        root,
        json!({
            "ok": false,
            "type": event_type,
            "lane": "core/layer0/ops",
            "error": clean(err, 220)
        }),
    )
}

fn command_ignite(root: &Path, parsed: &crate::ParsedArgs) -> i32 {
    let proposal = clean(
        parsed
            .flags
            .get("proposal")
            .cloned()
            .unwrap_or_else(|| "optimize_runtime_loop".to_string()),
        280,
    );
    let module = clean(
        parsed
            .flags
            .get("module")
            .cloned()
            .unwrap_or_else(|| "conduit".to_string()),
        120,
    )
    .to_ascii_lowercase();
    let apply = parse_bool(parsed.flags.get("apply"), true);
    let canary_pass = parse_bool(parsed.flags.get("canary-pass"), true);
    let sim_regression = parsed
        .flags
        .get("sim-regression")
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or_else(|| simulate_regression(&proposal, &module))
        .max(0.0);
    let threshold = parse_f64(parsed.flags.get("max-regression"), 0.05).max(0.0);
    let gate_action = format!("rsi:ignite:{module}");
    let gate_eval = directive_kernel::evaluate_action(root, &gate_action);
    let gate_ok = gate_eval
        .get("allowed")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let mut allowed = gate_ok && canary_pass && sim_regression <= threshold;
    let mut mutation_exit = 0i32;
    let mut reward = Value::Null;

    let mut state = load_loop_state(root);
    let state_obj = loop_obj_mut(&mut state);
    state_obj.insert("active".to_string(), Value::Bool(apply && allowed));

    if apply && allowed {
        mutation_exit = binary_blob_runtime::run(
            root,
            &[
                "mutate".to_string(),
                format!("--module={module}"),
                format!("--proposal={proposal}"),
                "--apply=1".to_string(),
                format!("--canary-pass={}", if canary_pass { 1 } else { 0 }),
                format!("--sim-regression={sim_regression:.4}"),
            ],
        );
        if mutation_exit != 0 {
            allowed = false;
        }
    }

    if apply {
        let _ = append_jsonl(
            &recursive_loop_path(root),
            &json!({
                "ts": now_iso(),
                "proposal": proposal,
                "module": module,
                "gate_action": gate_action,
                "gate_eval": gate_eval,
                "canary_pass": canary_pass,
                "sim_regression": sim_regression,
                "threshold": threshold,
                "mutation_exit": mutation_exit,
                "result": if allowed { "merge" } else { "rollback" }
            }),
        );
    }

    if apply && allowed {
        let next = state_obj
            .get("merge_count")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            + 1;
        state_obj.insert("merge_count".to_string(), Value::from(next));
        state_obj.insert(
            "last_merge".to_string(),
            json!({
                "ts": now_iso(),
                "proposal": proposal,
                "module": module,
                "sim_regression": sim_regression
            }),
        );
        reward = maybe_token_reward(root, "organism:global", 1.0, "tokenomics");
    } else if apply {
        let next = state_obj
            .get("rollback_count")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            + 1;
        state_obj.insert("rollback_count".to_string(), Value::from(next));
        state_obj.insert(
            "last_rollback".to_string(),
            json!({
                "ts": now_iso(),
                "proposal": proposal,
                "module": module,
                "gate_ok": gate_ok,
                "canary_pass": canary_pass,
                "sim_regression": sim_regression,
                "mutation_exit": mutation_exit
            }),
        );
    }

    if let Err(err) = store_loop_state(root, &state) {
        return emit_state_write_error(root, "rsi_ignition_activate", err);
    }

    emit(
        root,
        json!({
            "ok": allowed,
            "type": "rsi_ignition_activate",
            "lane": "core/layer0/ops",
            "proposal": proposal,
            "module": module,
            "apply": apply,
            "gate_ok": gate_ok,
            "canary_pass": canary_pass,
            "sim_regression": sim_regression,
            "max_regression": threshold,
            "mutation_exit": mutation_exit,
            "token_reward": reward,
            "pipeline": ["propose", "simulate", "canary", "merge_or_rollback"],
            "claim_evidence": [
                {
                    "id": "V8-RSI-IGNITION-001",
                    "claim": "recursive_self_modification_is_inversion_gated_with_merge_and_rollback_paths",
                    "evidence": {
                        "gate_action": gate_action,
                        "gate_allowed": gate_ok,
                        "canary_pass": canary_pass,
                        "sim_regression": sim_regression,
                        "mutation_exit": mutation_exit,
                        "result": if allowed { "merge" } else { "rollback" }
                    }
                }
            ]
        }),
    )
}

fn command_reflect(root: &Path, parsed: &crate::ParsedArgs) -> i32 {
    let mut state = load_loop_state(root);
    let observed_failure_rate = estimate_recent_failure_rate(root);
    let drift = parsed
        .flags
        .get("drift")
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or((0.1 + observed_failure_rate * 0.8).clamp(0.0, 1.0))
        .clamp(0.0, 1.0);
    let exploration = parsed
        .flags
        .get("exploration")
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or_else(|| {
            let prior = state
                .get("exploration_drive")
                .and_then(Value::as_f64)
                .unwrap_or(0.6);
            if drift > 0.5 {
                (prior - 0.15).clamp(0.05, 1.0)
            } else {
                (prior + 0.05).clamp(0.05, 1.0)
            }
        })
        .clamp(0.0, 1.0);

    {
        let obj = loop_obj_mut(&mut state);
        obj.insert("drift_score".to_string(), Value::from(drift));
        obj.insert("exploration_drive".to_string(), Value::from(exploration));
        obj.insert(
            "last_reflection".to_string(),
            json!({
                "ts": now_iso(),
                "drift_score": drift,
                "exploration_drive": exploration,
                "observed_failure_rate": observed_failure_rate,
                "action": if drift > 0.5 { "self_correct" } else { "continue_explore" }
            }),
        );
    }
    if let Err(err) = store_loop_state(root, &state) {
        return emit_state_write_error(root, "rsi_ignition_reflection", err);
    }

    let _ = append_jsonl(
        &metacognition_journal_path(root),
        &json!({
            "ts": now_iso(),
            "drift_score": drift,
            "exploration_drive": exploration,
            "observed_failure_rate": observed_failure_rate,
            "strategy_adjustment": if drift > 0.5 { "stabilize" } else { "explore" }
        }),
    );

    emit(
        root,
        json!({
            "ok": true,
            "type": "rsi_ignition_reflection",
            "lane": "core/layer0/ops",
            "drift_score": drift,
            "exploration_drive": exploration,
            "observed_failure_rate": observed_failure_rate,
            "action": if drift > 0.5 { "self_correct" } else { "continue_explore" },
            "claim_evidence": [
                {
                    "id": "V8-RSI-IGNITION-002",
                    "claim": "metacognitive_reflection_tracks_goal_drift_and_adjusts_exploration_drive",
                    "evidence": {
                        "drift_score": drift,
                        "exploration_drive": exploration,
                        "observed_failure_rate": observed_failure_rate
                    }
                }
            ]
        }),
    )
}

fn command_swarm(root: &Path, parsed: &crate::ParsedArgs) -> i32 {
    let nodes = parse_f64(parsed.flags.get("nodes"), 8.0).max(1.0) as u64;
    let share_rate = parse_f64(parsed.flags.get("share-rate"), 0.55).clamp(0.0, 1.0);
    let apply = parse_bool(parsed.flags.get("apply"), true);
    let gate_ok = directive_kernel::action_allowed(root, "rsi:swarm");
    let convergence = ((share_rate * 0.75) + ((nodes as f64).ln() / 10.0)).clamp(0.0, 1.0);
    let allowed = gate_ok && convergence > 0.1;
    let mut reward = Value::Null;

    let mut state = load_loop_state(root);
    if apply && allowed {
        let obj = loop_obj_mut(&mut state);
        obj.insert(
            "swarm".to_string(),
            json!({
                "nodes": nodes,
                "share_rate": share_rate,
                "convergence_score": convergence,
                "updated_at": now_iso()
            }),
        );
        reward = maybe_token_reward(
            root,
            "organism:swarm",
            (nodes as f64) * share_rate * 0.1,
            "tokenomics",
        );
        let _ = append_jsonl(
            &network_symbiosis_path(root),
            &json!({
                "ts": now_iso(),
                "nodes": nodes,
                "share_rate": share_rate,
                "convergence_score": convergence,
                "resource_allocation": {
                    "token_reward_attempted": reward.get("attempted").and_then(Value::as_bool).unwrap_or(false),
                    "token_reward_ok": reward.get("ok").and_then(Value::as_bool).unwrap_or(false)
                }
            }),
        );
    }
    if let Err(err) = store_loop_state(root, &state) {
        return emit_state_write_error(root, "rsi_ignition_swarm", err);
    }

    emit(
        root,
        json!({
            "ok": allowed,
            "type": "rsi_ignition_swarm",
            "lane": "core/layer0/ops",
            "nodes": nodes,
            "share_rate": share_rate,
            "convergence_score": convergence,
            "apply": apply,
            "gate_ok": gate_ok,
            "swarm_reward": reward,
            "claim_evidence": [
                {
                    "id": "V8-RSI-IGNITION-003",
                    "claim": "network_level_sub_swarms_share_improvements_with_policy_bounded_resource_allocation",
                    "evidence": {
                        "nodes": nodes,
                        "share_rate": share_rate,
                        "convergence_score": convergence
                    }
                }
            ]
        }),
    )
}

fn command_evolve(root: &Path, parsed: &crate::ParsedArgs) -> i32 {
    let mut state = load_loop_state(root);
    let insight = clean(
        parsed.flags.get("insight").cloned().unwrap_or_else(|| {
            "I found a lower-cost planning strategy with stable quality.".to_string()
        }),
        360,
    );
    let module = clean(
        parsed
            .flags
            .get("module")
            .cloned()
            .unwrap_or_else(|| "conduit".to_string()),
        120,
    )
    .to_ascii_lowercase();
    let apply = parse_bool(parsed.flags.get("apply"), false);
    let ignite_apply = parse_bool(parsed.flags.get("ignite-apply"), false);
    let night_cycle = parse_bool(parsed.flags.get("night-cycle"), true);
    let gate_ok = directive_kernel::action_allowed(root, &format!("rsi:evolve:{module}"));
    let proactive_message = format!(
        "night-cycle insight: {} | suggested module={} | apply={}",
        insight, module, ignite_apply
    );

    let mut ignite_exit = 0i32;
    if apply && gate_ok {
        ignite_exit = command_ignite(
            root,
            &parse_args(&[
                "ignite".to_string(),
                format!("--proposal={insight}"),
                format!("--module={module}"),
                format!("--apply={}", if ignite_apply { 1 } else { 0 }),
            ]),
        );
        if night_cycle {
            let _ = append_jsonl(
                &proactive_evolution_path(root),
                &json!({
                    "ts": now_iso(),
                    "insight": insight,
                    "module": module,
                    "directive_safe": gate_ok,
                    "proactive_message": proactive_message,
                    "morning_surface": true
                }),
            );
        }
    }

    {
        let obj = loop_obj_mut(&mut state);
        let next = obj
            .get("proactive_evolution_count")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            + 1;
        obj.insert("proactive_evolution_count".to_string(), Value::from(next));
        obj.insert(
            "last_evolution".to_string(),
            json!({
                "ts": now_iso(),
                "insight": insight,
                "module": module,
                "apply": apply,
                "ignite_apply": ignite_apply,
                "ignite_exit": ignite_exit
            }),
        );
    }
    if let Err(err) = store_loop_state(root, &state) {
        return emit_state_write_error(root, "rsi_ignition_evolve", err);
    }

    emit(
        root,
        json!({
            "ok": gate_ok,
            "type": "rsi_ignition_evolve",
            "lane": "core/layer0/ops",
            "insight": insight,
            "module": module,
            "apply": apply,
            "ignite_apply": ignite_apply,
            "ignite_exit": ignite_exit,
            "night_cycle": night_cycle,
            "proactive_message": proactive_message,
            "directive_safe": gate_ok,
            "claim_evidence": [
                {
                    "id": "V8-RSI-IGNITION-004",
                    "claim": "proactive_evolution_surfaces_night_cycle_insights_with_directive_safe_receipts",
                    "evidence": {
                        "directive_safe": gate_ok,
                        "night_cycle": night_cycle,
                        "ignite_exit": ignite_exit
                    }
                }
            ]
        }),
    )
}
