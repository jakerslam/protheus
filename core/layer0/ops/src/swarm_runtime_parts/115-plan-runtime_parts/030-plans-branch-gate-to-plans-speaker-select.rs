
fn plans_branch_gate(state: &mut SwarmState, argv: &[String]) -> Result<Value, String> {
    let plan_id = parse_flag(argv, "plan-id")
        .filter(|row| !row.trim().is_empty())
        .ok_or_else(|| "plan_id_required".to_string())?;
    let node_id = parse_flag(argv, "node-id")
        .filter(|row| !row.trim().is_empty())
        .ok_or_else(|| "node_id_required".to_string())?;
    let requires_user = parse_bool_flag(argv, "wait-user", false);
    let timeout_ms = parse_u64_flag(argv, "timeout-ms", 120_000).max(1);
    let mut decision = parse_flag(argv, "decision")
        .unwrap_or_else(|| "auto".to_string())
        .trim()
        .to_ascii_lowercase();
    let auto_path = parse_flag(argv, "auto-path")
        .map(|row| clean_text(&row, 64))
        .filter(|row| !row.is_empty())
        .unwrap_or_else(|| "approve".to_string());
    let Some(plan) = state.plan_registry.get_mut(&plan_id) else {
        return Err(format!("unknown_plan:{plan_id}"));
    };
    let Some(node) = plan.nodes.get_mut(&node_id) else {
        return Err(format!("unknown_plan_node:{node_id}"));
    };

    let status;
    if requires_user && decision == "auto" {
        status = "waiting_user".to_string();
        node.status = "waiting_user".to_string();
    } else {
        if decision == "auto" {
            decision = auto_path.to_ascii_lowercase();
        }
        if decision == "deny" {
            status = "blocked".to_string();
            node.status = "blocked".to_string();
        } else {
            status = "approved".to_string();
            node.status = "ready".to_string();
        }
    }
    node.branch_state = Some(status.clone());
    node.updated_at = now_iso();

    let gate = BranchGateState {
        node_id: node_id.clone(),
        requires_user,
        status: status.clone(),
        decision: Some(decision.clone()),
        timeout_ms,
        auto_path,
        decided_at_ms: Some(now_epoch_ms()),
    };
    plan.branch_gates.insert(node_id.clone(), gate.clone());
    plan.updated_at = now_iso();
    Ok(json!({
        "ok": true,
        "type": "swarm_runtime_plan_branch_gate",
        "plan_id": plan_id,
        "node_id": node_id,
        "gate": gate,
    }))
}

fn plans_speaker_select(state: &mut SwarmState, argv: &[String]) -> Result<Value, String> {
    let plan_id = parse_flag(argv, "plan-id")
        .filter(|row| !row.trim().is_empty())
        .ok_or_else(|| "plan_id_required".to_string())?;
    let message = parse_flag(argv, "message")
        .map(|row| clean_text(&row, 240))
        .filter(|row| !row.is_empty())
        .ok_or_else(|| "message_required".to_string())?;
    let snapshot = state
        .plan_registry
        .get(&plan_id)
        .cloned()
        .ok_or_else(|| format!("unknown_plan:{plan_id}"))?;
    let mut candidates = parse_flag(argv, "candidate-session-ids")
        .map(|raw| {
            raw.split(',')
                .map(str::trim)
                .filter(|row| !row.is_empty())
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if candidates.is_empty() {
        candidates.push(snapshot.supervisor_session_id.clone());
        for node in snapshot.nodes.values() {
            if let Some(session_id) = node.assignee_session_id.as_ref() {
                if !candidates.iter().any(|row| row == session_id) {
                    candidates.push(session_id.clone());
                }
            }
        }
    }
    if candidates.is_empty() {
        return Err("speaker_candidates_empty".to_string());
    }
    let msg = message.to_ascii_lowercase();
    let now_ms = now_epoch_ms();

    let mut rankings = Vec::new();
    for session_id in candidates {
        if !state.sessions.contains_key(&session_id) {
            continue;
        }
        let role = state
            .sessions
            .get(&session_id)
            .and_then(|session| session.role.clone())
            .unwrap_or_else(|| "worker".to_string());
        let expertise = session_capabilities(state, &session_id);
        let expertise_hits = expertise
            .iter()
            .filter(|tag| msg.contains(&tag.to_ascii_lowercase()))
            .count() as f64;
        let role_hit = if msg.contains(&role.to_ascii_lowercase()) {
            1.0
        } else {
            0.0
        };
        let prior = snapshot.speaker_stats.get(&session_id);
        let freshness = prior
            .and_then(|row| row.last_spoke_ms)
            .map(|last| (now_ms.saturating_sub(last) > 60_000) as u8 as f64)
            .unwrap_or(0.5);
        let turn_bonus = 1.0 / (1.0 + prior.map(|row| row.turns).unwrap_or(0) as f64);
        let score = expertise_hits * 2.0 + role_hit + freshness + turn_bonus;
        rankings.push(json!({
            "session_id": session_id,
            "role": role,
            "expertise_tags": expertise,
            "score": score,
        }));
    }
    rankings.sort_by(|a, b| {
        let lhs = b.get("score").and_then(Value::as_f64).unwrap_or(0.0);
        let rhs = a.get("score").and_then(Value::as_f64).unwrap_or(0.0);
        lhs.partial_cmp(&rhs).unwrap_or(std::cmp::Ordering::Equal)
    });
    let selected = rankings
        .first()
        .cloned()
        .ok_or_else(|| "speaker_candidates_empty".to_string())?;
    let selected_id = selected
        .get("session_id")
        .and_then(Value::as_str)
        .ok_or_else(|| "selected_speaker_missing".to_string())?
        .to_string();

    if let Some(plan) = state.plan_registry.get_mut(&plan_id) {
        let role = selected
            .get("role")
            .and_then(Value::as_str)
            .unwrap_or("worker")
            .to_string();
        let expertise_tags = selected
            .get("expertise_tags")
            .and_then(Value::as_array)
            .map(|rows| {
                rows.iter()
                    .filter_map(Value::as_str)
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let score = selected.get("score").and_then(Value::as_f64).unwrap_or(0.0);
        let stat = plan
            .speaker_stats
            .entry(selected_id.clone())
            .or_insert(SpeakerStats {
                session_id: selected_id.clone(),
                role,
                expertise_tags,
                last_spoke_ms: None,
                turns: 0,
                score,
            });
        stat.turns = stat.turns.saturating_add(1);
        stat.last_spoke_ms = Some(now_ms);
        stat.score = score;
        plan.updated_at = now_iso();
    }

    Ok(json!({
        "ok": true,
        "type": "swarm_runtime_plan_speaker_select",
        "plan_id": plan_id,
        "selected": selected,
        "rankings": rankings,
    }))
}
