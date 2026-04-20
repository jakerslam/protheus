fn autoreason_state_path(root: &Path) -> PathBuf {
    state_root(root).join("autoreason").join("state.json")
}

fn autoreason_run_log_path(root: &Path, run_id: &str) -> PathBuf {
    state_root(root)
        .join("autoreason")
        .join("runs")
        .join(format!("{run_id}.jsonl"))
}

fn autoreason_default_state() -> Value {
    json!({
        "version": "v1",
        "total_runs": 0u64,
        "last_run": Value::Null,
        "runs": {},
        "updated_at": now_iso()
    })
}

fn ensure_autoreason_state_shape(state: &mut Value) {
    if !state.is_object() {
        *state = autoreason_default_state();
    }
    if !state.get("runs").map(Value::is_object).unwrap_or(false) {
        state["runs"] = Value::Object(serde_json::Map::new());
    }
}

fn autoreason_seed_candidate(task: &str, style: &str) -> String {
    let clipped = task.trim();
    match style {
        "counterfactual" => format!(
            "Approach: counterfactual-first.\nThesis: {clipped}\nPlan: enumerate strongest objection first, then reconcile trade-offs with one decisive recommendation."
        ),
        _ => format!(
            "Approach: direct-first.\nThesis: {clipped}\nPlan: deliver concise argument, explicit trade-offs, and one measurable next step."
        ),
    }
}

fn autoreason_score(text: &str) -> f64 {
    let normalized = text.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return 0.0;
    }
    let words = normalized
        .split_whitespace()
        .map(|w| w.trim_matches(|c: char| !c.is_ascii_alphanumeric()))
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>();
    if words.is_empty() {
        return 0.0;
    }
    let unique = words
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>()
        .len() as f64;
    let uniq_ratio = (unique / words.len() as f64).clamp(0.0, 1.0);
    let repetition_ratio = (1.0 - uniq_ratio).clamp(0.0, 1.0);
    let action_hits = words
        .iter()
        .filter(|w| {
            matches!(
                **w,
                "build"
                    | "test"
                    | "measure"
                    | "ship"
                    | "validate"
                    | "compare"
                    | "decide"
                    | "execute"
                    | "improve"
                    | "monitor"
            )
        })
        .count() as f64;
    let action_score = (action_hits / 8.0).clamp(0.0, 1.0);
    let has_tradeoff = normalized.contains("trade-off") || normalized.contains("tradeoff");
    let has_recommendation = normalized.contains("recommend") || normalized.contains("next step");
    let has_measure = normalized.contains("measure")
        || normalized.contains("metric")
        || normalized.contains("baseline");
    let structure_score = if has_tradeoff && has_recommendation && has_measure {
        1.0
    } else if (has_tradeoff && has_recommendation) || has_measure {
        0.6
    } else {
        0.25
    };
    let length_score = ((words.len() as f64) / 120.0).clamp(0.1, 1.0);
    let redundancy_penalty = if repetition_ratio > 0.45 {
        (repetition_ratio - 0.45).clamp(0.0, 0.35)
    } else {
        0.0
    };
    let raw = 0.33 * uniq_ratio + 0.22 * action_score + 0.26 * structure_score + 0.19 * length_score;
    (raw - (0.35 * redundancy_penalty)).clamp(0.0, 1.0)
}

fn autoreason_critique(task: &str, candidate: &str) -> Value {
    let score = autoreason_score(candidate);
    let mut strengths = Vec::new();
    let mut weaknesses = Vec::new();
    if score >= 0.65 {
        strengths.push("clear structure and actionable framing".to_string());
    } else {
        weaknesses.push("needs tighter structure and stronger actionability".to_string());
    }
    if candidate.to_ascii_lowercase().contains("trade-off")
        || candidate.to_ascii_lowercase().contains("tradeoff")
    {
        strengths.push("acknowledges trade-offs".to_string());
    } else {
        weaknesses.push("missing explicit trade-off discussion".to_string());
    }
    if candidate.to_ascii_lowercase().contains("recommend")
        || candidate.to_ascii_lowercase().contains("next step")
    {
        strengths.push("includes recommendation".to_string());
    } else {
        weaknesses.push("no explicit recommendation".to_string());
    }
    json!({
        "task": task,
        "subjective_fitness": score,
        "strengths": strengths,
        "weaknesses": weaknesses,
        "revision_directive": if score < 0.70 {
            "sharpen thesis, surface one core trade-off, and end with measurable next step"
        } else {
            "retain core structure and increase precision"
        }
    })
}

fn autoreason_revise(task: &str, candidate: &str, critique: &Value) -> String {
    let directive = critique
        .get("revision_directive")
        .and_then(Value::as_str)
        .unwrap_or("improve clarity");
    format!(
        "Task: {task}\nCandidate:\n{candidate}\nRevision:\n{directive}. Recommendation: execute one small measurable step and review signal quality after implementation."
    )
}

fn autoreason_synthesize(task: &str, a: &str, b: &str) -> String {
    format!(
        "Synthesis for task: {task}\nCombined thesis: keep direct execution speed while preserving counterfactual risk checks.\nOption A signal: {}\nOption B signal: {}\nRecommendation: apply A as default path and B as adversarial guardrail.",
        a.lines().next().unwrap_or_default(),
        b.lines().next().unwrap_or_default()
    )
}

fn deterministic_noise(seed: &Value, modulo: u64) -> u64 {
    let hash = receipt_hash(seed);
    let n = u64::from_str_radix(hash.get(0..8).unwrap_or("0"), 16).unwrap_or(0);
    if modulo == 0 {
        0
    } else {
        n % modulo
    }
}

fn autoreason_blind_evaluate(
    run_id: &str,
    iteration: u64,
    candidates: &[(String, String)],
    judges: u64,
) -> Value {
    let mut blinded = candidates
        .iter()
        .map(|(candidate_id, text)| {
            let sort_key = receipt_hash(
                &json!({"run_id": run_id, "iteration": iteration, "candidate_id": candidate_id}),
            );
            (sort_key, candidate_id.clone(), text.clone())
        })
        .collect::<Vec<_>>();
    blinded.sort_by(|a, b| a.0.cmp(&b.0));

    let blinded_index = blinded
        .iter()
        .enumerate()
        .map(|(idx, (_, candidate_id, text))| {
            let alias = format!("c{:02}", idx + 1);
            let preview = text.chars().take(140).collect::<String>();
            (alias, candidate_id.clone(), text.clone(), preview)
        })
        .collect::<Vec<_>>();

    let mut tallies = std::collections::BTreeMap::<String, (u64, f64)>::new();
    let mut votes = Vec::new();
    for judge in 0..judges {
        let mut scored = blinded_index
            .iter()
            .map(|(alias, candidate_id, text, _)| {
                let base = autoreason_score(text);
                let noise = deterministic_noise(
                    &json!({"run_id": run_id, "iteration": iteration, "judge": judge, "alias": alias}),
                    97,
                ) as f64
                    / 1000.0;
                (alias.clone(), candidate_id.clone(), (base + noise).clamp(0.0, 1.0))
            })
            .collect::<Vec<_>>();
        scored.sort_by(|a, b| {
            b.2.partial_cmp(&a.2)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.0.cmp(&b.0))
        });
        if let Some((alias, candidate_id, score)) = scored.first().cloned() {
            let entry = tallies.entry(candidate_id.clone()).or_insert((0, 0.0));
            entry.0 = entry.0.saturating_add(1);
            entry.1 += score;
            votes.push(json!({
                "judge": format!("j{:02}", judge + 1),
                "selected_alias": alias,
                "selected_candidate": candidate_id,
                "score": ((score * 1_000_000.0).round() / 1_000_000.0)
            }));
        }
    }

    let mut tally_rows = tallies
        .iter()
        .map(|(candidate_id, (votes, cumulative_score))| {
            let mean_score = if *votes == 0 {
                0.0
            } else {
                *cumulative_score / (*votes as f64)
            };
            json!({
                "candidate_id": candidate_id,
                "votes": votes,
                "mean_score": ((mean_score * 1_000_000.0).round() / 1_000_000.0)
            })
        })
        .collect::<Vec<_>>();
    tally_rows.sort_by(|a, b| {
        let av = a.get("votes").and_then(Value::as_u64).unwrap_or(0);
        let bv = b.get("votes").and_then(Value::as_u64).unwrap_or(0);
        let am = a.get("mean_score").and_then(Value::as_f64).unwrap_or(0.0);
        let bm = b.get("mean_score").and_then(Value::as_f64).unwrap_or(0.0);
        bv.cmp(&av)
            .then_with(|| bm.partial_cmp(&am).unwrap_or(std::cmp::Ordering::Equal))
            .then_with(|| {
                let aid = a
                    .get("candidate_id")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                let bid = b
                    .get("candidate_id")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                aid.cmp(bid)
            })
    });
    let winner_id = tally_rows
        .first()
        .and_then(|row| row.get("candidate_id"))
        .and_then(Value::as_str)
        .unwrap_or("ab_synth")
        .to_string();
    let winner_votes = tally_rows
        .first()
        .and_then(|row| row.get("votes"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let second_votes = tally_rows
        .get(1)
        .and_then(|row| row.get("votes"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let winner_vote_margin = winner_votes.saturating_sub(second_votes);

    json!({
        "blinded_candidates": blinded_index
            .iter()
            .map(|(alias, _, _, preview)| json!({"alias": alias, "preview": preview}))
            .collect::<Vec<_>>(),
        "votes": votes,
        "tally": tally_rows,
        "winner_id": winner_id,
        "winner_vote_margin": winner_vote_margin
    })
}

fn run_autoreason(root: &Path, argv: &[String]) -> i32 {
    let strict = parse_bool(parse_flag(argv, "strict").as_deref(), true);
    if let Some(mut denied) = conduit_guard(argv, strict) {
        return emit_receipt(root, &mut denied);
    }
    let action = clean_id(
        parse_flag(argv, "action").or_else(|| parse_positional(argv, 1)),
        "status",
    );
    let mut state =
        read_json(&autoreason_state_path(root)).unwrap_or_else(autoreason_default_state);
    ensure_autoreason_state_shape(&mut state);

    if action == "status" {
        state["updated_at"] = json!(now_iso());
        let mut out = json!({
            "ok": true,
            "type": "autonomy_autoreason",
            "lane": LANE_ID,
            "strict": strict,
            "action": action,
            "state": state,
            "claim_evidence": [
                {"id":"V6-COGNITION-013.1","claim":"autoreason_status_surfaces_governed_subjective_refinement_runtime_state"}
            ]
        });
        return emit_receipt(root, &mut out);
    }

    if action != "run" {
        let mut out = cli_error_receipt(argv, "autoreason_action_invalid", 2);
        out["type"] = json!("autonomy_autoreason");
        return emit_receipt(root, &mut out);
    }

    let task = parse_flag(argv, "task")
        .or_else(|| parse_positional(argv, 2))
        .unwrap_or_else(|| "subjective refinement task".to_string());
    let convergence = parse_u64(parse_flag(argv, "convergence").as_deref(), 3, 1, 9);
    let max_iters = parse_u64(parse_flag(argv, "max-iters").as_deref(), 6, 1, 24);
    let mut judges = parse_u64(parse_flag(argv, "judges").as_deref(), 3, 1, 11);
    if judges % 2 == 0 {
        judges = (judges + 1).min(11);
    }
    let run_id = clean_id(
        parse_flag(argv, "run-id"),
        &format!(
            "ar-{}",
            &receipt_hash(&json!({"task": task, "ts": now_iso()}))[..10]
        ),
    );
    if strict
        && state
            .pointer(&format!("/runs/{run_id}"))
            .map(Value::is_object)
            .unwrap_or(false)
    {
        let mut out = cli_error_receipt(argv, "autoreason_run_id_exists", 2);
        out["type"] = json!("autonomy_autoreason");
        return emit_receipt(root, &mut out);
    }

    let mut candidate_a = autoreason_seed_candidate(&task, "direct");
    let mut candidate_b = autoreason_seed_candidate(&task, "counterfactual");
    let mut winner_streak = 0u64;
    let mut last_winner = String::new();
    let mut converged = false;
    let mut final_winner = "ab_synth".to_string();
    let mut final_text = String::new();
    let mut iterations = Vec::new();

    for iteration in 1..=max_iters {
        let critique_a = autoreason_critique(&task, &candidate_a);
        let critique_b = autoreason_critique(&task, &candidate_b);
        let revised_a = autoreason_revise(&task, &candidate_a, &critique_a);
        let revised_b = autoreason_revise(&task, &candidate_b, &critique_b);
        let synthesized = autoreason_synthesize(&task, &revised_a, &revised_b);

        let candidates = vec![
            ("a_revised".to_string(), revised_a.clone()),
            ("b_revised".to_string(), revised_b.clone()),
            ("ab_synth".to_string(), synthesized.clone()),
        ];
        let blind_eval = autoreason_blind_evaluate(&run_id, iteration, &candidates, judges);
        let winner_id = blind_eval
            .get("winner_id")
            .and_then(Value::as_str)
            .unwrap_or("ab_synth")
            .to_string();

        if winner_id == last_winner {
            winner_streak = winner_streak.saturating_add(1);
        } else {
            winner_streak = 1;
            last_winner = winner_id.clone();
        }
        converged = winner_streak >= convergence;
        final_winner = winner_id.clone();
        final_text = candidates
            .iter()
            .find(|(id, _)| id == &winner_id)
            .map(|(_, text)| text.clone())
            .unwrap_or_else(|| synthesized.clone());

        let row = json!({
            "type": "autoreason_iteration",
            "run_id": run_id,
            "iteration": iteration,
            "task": task,
            "candidates": {
                "a_revised": revised_a,
                "b_revised": revised_b,
                "ab_synth": synthesized
            },
            "critiques": {
                "a": critique_a,
                "b": critique_b
            },
            "blind_eval": blind_eval,
            "winner_id": winner_id,
            "winner_streak": winner_streak,
            "convergence_target": convergence,
            "converged": converged,
            "ts": now_iso()
        });
        if let Err(err) = append_jsonl(&autoreason_run_log_path(root, &run_id), &row) {
            let mut out = cli_error_receipt(argv, &format!("autoreason_append_failed:{err}"), 2);
            out["type"] = json!("autonomy_autoreason");
            return emit_receipt(root, &mut out);
        }
        iterations.push(row);

        match final_winner.as_str() {
            "a_revised" => {
                candidate_a = final_text.clone();
                candidate_b = autoreason_synthesize(&task, &candidate_b, &final_text);
            }
            "b_revised" => {
                candidate_a = autoreason_synthesize(&task, &candidate_a, &final_text);
                candidate_b = final_text.clone();
            }
            _ => {
                candidate_a = autoreason_revise(
                    &task,
                    &candidate_a,
                    &json!({"revision_directive":"align with synthesized winner"}),
                );
                candidate_b = autoreason_revise(
                    &task,
                    &candidate_b,
                    &json!({"revision_directive":"align with synthesized winner"}),
                );
            }
        }

        if converged {
            break;
        }
    }

    let run_summary = json!({
        "run_id": run_id,
        "task": task,
        "max_iters": max_iters,
        "convergence_target": convergence,
        "judges": judges,
        "iterations": iterations.len(),
        "converged": converged,
        "winner_id": final_winner,
        "winner_text": final_text,
        "updated_at": now_iso()
    });
    state["runs"][&run_id] = run_summary.clone();
    state["last_run"] = run_summary.clone();
    state["total_runs"] = json!(state.get("total_runs").and_then(Value::as_u64).unwrap_or(0) + 1);
    state["updated_at"] = json!(now_iso());
    if let Err(err) = write_json(&autoreason_state_path(root), &state) {
        let mut out = cli_error_receipt(argv, &format!("autoreason_state_write_failed:{err}"), 2);
        out["type"] = json!("autonomy_autoreason");
        return emit_receipt(root, &mut out);
    }
    if strict {
        let persisted = read_json(&autoreason_state_path(root)).unwrap_or(Value::Null);
        let ok = persisted
            .pointer("/last_run/run_id")
            .and_then(Value::as_str)
            == Some(&run_id);
        if !ok {
            let mut out = cli_error_receipt(argv, "autoreason_state_verify_failed", 2);
            out["type"] = json!("autonomy_autoreason");
            return emit_receipt(root, &mut out);
        }
    }

    let mut out = json!({
        "ok": true,
        "type": "autonomy_autoreason",
        "lane": LANE_ID,
        "strict": strict,
        "action": action,
        "run": run_summary,
        "claim_evidence": [
            {"id":"V6-COGNITION-013.1","claim":"autoreason_role_orchestration_executes_generator_critic_reviser_synthesizer_and_blind_judges"},
            {"id":"V6-COGNITION-013.2","claim":"autoreason_blind_randomized_evaluation_is_deterministically_receipted"},
            {"id":"V6-COGNITION-013.3","claim":"autoreason_iterative_loop_stops_on_convergence_threshold"}
        ]
    });
    emit_receipt(root, &mut out)
}