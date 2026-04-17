) -> Value {
    let policy = load_policy(root, explicit_policy_path);
    if !policy.enabled {
        return json!({
            "ok": false,
            "type": "multi_agent_debate_orchestrator",
            "error": "policy_disabled"
        });
    }

    let ts = now_iso();
    let date = parse_date_or_today(
        date_override
            .or_else(|| input.get("date").and_then(Value::as_str))
            .or_else(|| Some(&ts[..10])),
    );
    let objective_id = input
        .get("objective_id")
        .or_else(|| input.get("objectiveId"))
        .and_then(Value::as_str)
        .map(|v| normalize_token(v, 120))
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "generic_objective".to_string());
    let objective_text = input
        .get("objective")
        .or_else(|| input.get("objective_text"))
        .and_then(Value::as_str)
        .map(|v| clean_text(v, 300))
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| objective_id.clone());

    let candidates = normalize_candidates(input);
    let agents = build_agents(&policy, input);
    let rounds = policy.rounds_max.max(1);

    let mut transcript: Vec<Value> = Vec::new();
    let mut vote_totals: HashMap<String, f64> = HashMap::new();
    let mut distinct_roles: HashSet<String> = HashSet::new();
    let mut disagreement_votes: i64 = 0;
    let mut total_votes: i64 = 0;

    for round in 1..=rounds {
        for agent in &agents {
            let role = if agent.role.is_empty() {
                "orderly_executor".to_string()
            } else {
                agent.role.clone()
            };
            distinct_roles.insert(role.clone());
            let role_cfg = policy.roles.get(&role).cloned().unwrap_or(RoleCfg {
                weight: 1.0,
                bias: "delivery".to_string(),
            });

            let mut scored: Vec<(String, f64)> = candidates
                .iter()
                .map(|candidate| {
                    (
                        candidate.id.clone(),
                        score_candidate_for_role(&role_cfg, candidate),
                    )
                })
                .collect();
            scored.sort_by(|a, b| {
                b.1.partial_cmp(&a.1)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| a.0.cmp(&b.0))
            });

            let Some(top) = scored.first() else {
                continue;
            };
            let runner_up = scored.get(1);
            let gap = runner_up.map(|r| round_to(top.1 - r.1, 6)).unwrap_or(1.0);
            let contested = gap <= policy.disagreement_gap_threshold;
            let certainty = round_to(clamp_num((gap + 0.45).max(0.05), 0.0, 1.0, 0.5), 6);

            if contested {
                disagreement_votes += 1;
            }
            total_votes += 1;
            let vote_weight = round_to(top.1 * certainty, 6);
            *vote_totals.entry(top.0.clone()).or_insert(0.0) += vote_weight;

            transcript.push(json!({
                "round": round,
                "agent_id": agent.id,
                "role": role,
                "selected_candidate_id": top.0,
                "vote_score": top.1,
                "certainty": certainty,
                "contested": contested,
                "gap_to_runner_up": gap,
                "runner_up_candidate_id": runner_up.map(|r| r.0.clone())
            }));
        }
    }

    let mut ranked: Vec<(String, f64)> = vote_totals
        .iter()
        .map(|(k, v)| (k.clone(), round_to(*v, 6)))
        .collect();
    ranked.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.cmp(&b.0))
    });
    let top = ranked.first().cloned();
    let total_score: f64 = ranked.iter().map(|(_, s)| *s).sum();
    let consensus_share = if total_score > 0.0 {
        round_to(top.clone().map(|(_, s)| s).unwrap_or(0.0) / total_score, 6)
    } else {
        0.0
    };
    let disagreement_index = if total_votes > 0 {
        round_to(disagreement_votes as f64 / total_votes as f64, 6)
    } else {
        0.0
    };

    let min_agents = policy.rounds_min_agents.max(1) as usize;
    let quorum_met = agents.len() >= min_agents
        && (!policy.require_distinct_roles_for_quorum
            || distinct_roles.len() >= std::cmp::min(3usize, min_agents));
    let confidence_score = round_to(
        clamp_num(
            consensus_share * (1.0 - disagreement_index * 0.5),
            0.0,
            1.0,
            0.0,
        ),
        6,
    );

    let mut consensus = quorum_met
        && consensus_share >= policy.consensus_threshold
        && confidence_score >= policy.confidence_floor;
    let mut recommended_candidate_id = top.clone().map(|(id, _)| id);
    if !consensus {
        recommended_candidate_id = None;
    }

    let mut runoff_executed = false;
    let mut runoff_consensus = false;
    let mut runoff_recommended_candidate_id: Option<String> = None;

    if !consensus && policy.runoff_enabled && policy.max_runoff_rounds > 0 && ranked.len() >= 2 {
        runoff_executed = true;
        let runoff_candidates = vec![ranked[0].0.clone(), ranked[1].0.clone()];
        let mut runoff_totals: HashMap<String, f64> = HashMap::new();

        for round in 1..=policy.max_runoff_rounds {
            for agent in &agents {
                let role = if agent.role.is_empty() {
                    "orderly_executor".to_string()
                } else {
                    agent.role.clone()
                };
                let role_cfg = policy.roles.get(&role).cloned().unwrap_or(RoleCfg {
                    weight: 1.0,
                    bias: "delivery".to_string(),
                });

                let mut scored: Vec<(String, f64)> = runoff_candidates
                    .iter()
                    .map(|candidate_id| {
                        let source = candidates
                            .iter()
                            .find(|row| row.id == *candidate_id)
                            .cloned()
                            .unwrap_or(Candidate {
                                id: candidate_id.clone(),
                                score: 0.5,
                                confidence: 0.5,
                                risk: "medium".to_string(),
                            });
                        (
                            candidate_id.clone(),
                            score_candidate_for_role(&role_cfg, &source),
                        )
                    })
                    .collect();

                scored.sort_by(|a, b| {
                    b.1.partial_cmp(&a.1)
                        .unwrap_or(std::cmp::Ordering::Equal)
                        .then_with(|| a.0.cmp(&b.0))
                });

                let Some(pick) = scored.first() else {
                    continue;
                };
                *runoff_totals.entry(pick.0.clone()).or_insert(0.0) += pick.1;
                transcript.push(json!({
                    "round": rounds + round,
                    "phase": "runoff",
                    "agent_id": agent.id,
                    "role": role,
                    "selected_candidate_id": pick.0,
                    "vote_score": pick.1,
                    "runoff_candidates": runoff_candidates
                }));
            }
        }

        let mut runoff_ranked: Vec<(String, f64)> = runoff_totals
            .iter()
            .map(|(k, v)| (k.clone(), round_to(*v, 6)))
            .collect();
        runoff_ranked.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.0.cmp(&b.0))
        });
        let runoff_top = runoff_ranked.first().cloned();
        let runoff_total: f64 = runoff_ranked.iter().map(|(_, s)| *s).sum();
        let runoff_share = if runoff_total > 0.0 {
            round_to(
                runoff_top.clone().map(|(_, s)| s).unwrap_or(0.0) / runoff_total,
                6,
            )
        } else {
            0.0
        };
        let runoff_confidence = round_to(
            clamp_num(
                runoff_share * (1.0 - disagreement_index * 0.35),
                0.0,
                1.0,
                0.0,
            ),
            6,
        );

        runoff_consensus = quorum_met
            && runoff_share >= policy.runoff_consensus_threshold
            && runoff_confidence >= policy.confidence_floor;
        if runoff_consensus {
            consensus = true;
            runoff_recommended_candidate_id = runoff_top.clone().map(|(id, _)| id);
            recommended_candidate_id = runoff_top.map(|(id, _)| id);
        }
    }

    let ranked_json: Vec<Value> = ranked
        .iter()
        .map(|(candidate_id, score)| json!({ "candidate_id": candidate_id, "score": score }))
        .collect();

    let mut reason_codes = vec![];
    if consensus {
        if runoff_executed && runoff_consensus {
            reason_codes.push("multi_agent_consensus_reached_after_runoff".to_string());
        } else {
            reason_codes.push("multi_agent_consensus_reached".to_string());
        }
    } else {
        reason_codes.push("multi_agent_consensus_not_reached".to_string());
    }
    reason_codes.push(format!("confidence_score_{:.3}", confidence_score));

    let out = json!({
        "ok": true,
        "type": "multi_agent_debate_orchestrator",
        "ts": ts,
        "date": date,
        "shadow_only": policy.shadow_only,
        "objective_id": objective_id,
        "objective_text": objective_text,
        "rounds_executed": rounds,
        "quorum_met": quorum_met,
        "quorum_rule": {
            "min_agents": min_agents,
            "require_distinct_roles_for_quorum": policy.require_distinct_roles_for_quorum,
            "distinct_roles": distinct_roles.into_iter().collect::<Vec<_>>()
        },
        "consensus": consensus,
        "confidence_score": confidence_score,
        "confidence_floor": policy.confidence_floor,
        "consensus_share": consensus_share,
        "disagreement_index": disagreement_index,
        "disagreement_votes": disagreement_votes,
        "total_votes": total_votes,
        "recommended_candidate_id": recommended_candidate_id,
        "debate_resolution": {
            "runoff_executed": runoff_executed,
            "runoff_consensus": runoff_consensus,
            "runoff_rounds": if runoff_executed { policy.max_runoff_rounds } else { 0 },
            "runoff_recommended_candidate_id": runoff_recommended_candidate_id
        },
        "ranked_candidates": ranked_json,
        "debate_transcript": transcript,
        "reason_codes": reason_codes
    });

    if persist {
        let _ = write_json_atomic(&policy.latest_path, &out);
        let _ = append_jsonl(&policy.history_path, &out);
        let _ = append_jsonl(&policy.receipts_path, &out);
    }

    out
}

pub fn debate_status(root: &Path, explicit_policy_path: Option<&Path>, key: Option<&str>) -> Value {
    let policy = load_policy(root, explicit_policy_path);
    let key = clean_text(key.unwrap_or("latest"), 40);

    let payload = if key == "latest" {
        read_json(&policy.latest_path)
    } else {
        let day = parse_date_or_today(Some(&key));
        let rows = read_jsonl(&policy.history_path);
        rows.into_iter()
            .filter(|row| row.get("date").and_then(Value::as_str) == Some(day.as_str()))
            .last()
            .unwrap_or(Value::Null)
    };

    if !payload.is_object() {
        return json!({
            "ok": false,
            "type": "multi_agent_debate_status",
            "error": "snapshot_missing",
            "date": key
        });
    }

    json!({
        "ok": true,
        "type": "multi_agent_debate_status",
        "ts": payload.get("ts").cloned().unwrap_or(Value::Null),
        "date": payload.get("date").cloned().unwrap_or(Value::Null),
        "objective_id": payload.get("objective_id").cloned().unwrap_or(Value::Null),
        "consensus": payload.get("consensus").cloned().unwrap_or(json!(false)),
        "confidence_score": payload.get("confidence_score").cloned().unwrap_or(json!(0.0)),
        "consensus_share": payload.get("consensus_share").cloned().unwrap_or(json!(0.0)),
        "disagreement_index": payload.get("disagreement_index").cloned().unwrap_or(json!(0.0)),
        "recommended_candidate_id": payload
            .get("recommended_candidate_id")
            .cloned()
            .unwrap_or(Value::Null),
        "rounds_executed": payload.get("rounds_executed").cloned().unwrap_or(json!(0)),
        "shadow_only": payload.get("shadow_only").cloned().unwrap_or(json!(true))
    })
}

#[cfg(test)]
