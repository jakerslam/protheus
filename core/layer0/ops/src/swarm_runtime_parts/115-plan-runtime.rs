fn next_plan_id(state: &SwarmState, goal: &str) -> String {
    let digest = deterministic_receipt_hash(&json!({
        "goal": goal,
        "ts": now_epoch_ms(),
        "plans": state.plan_registry.len(),
    }));
    format!("plan-{}", &digest[..12])
}

fn next_plan_node_id(plan: &SwarmPlanGraph, seed: &str, depth: u8) -> String {
    let digest = deterministic_receipt_hash(&json!({
        "plan_id": plan.plan_id,
        "seed": seed,
        "depth": depth,
        "nodes": plan.nodes.len(),
        "ts": now_epoch_ms(),
    }));
    format!("node-{}", &digest[..12])
}

fn decompose_goal_tasks(goal: &str) -> Vec<String> {
    let mut tasks = goal
        .split([',', ';', '\n'])
        .map(str::trim)
        .filter(|row| !row.is_empty())
        .map(|row| clean_text(row, 120))
        .collect::<Vec<_>>();
    if tasks.is_empty() {
        tasks = vec![
            format!("analyze goal constraints: {}", clean_text(goal, 80)),
            format!("execute primary objective: {}", clean_text(goal, 80)),
            "synthesize completion summary".to_string(),
        ];
    }
    if tasks.len() > 8 {
        tasks.truncate(8);
    }
    tasks
}

fn ensure_supervisor_session_for_plan(
    state: &mut SwarmState,
    supervisor_session_id: Option<String>,
    goal: &str,
    max_depth: u8,
) -> Result<String, String> {
    if let Some(existing) = supervisor_session_id {
        if state.sessions.contains_key(&existing) {
            return Ok(existing);
        }
        return Err(format!("unknown_supervisor_session:{existing}"));
    }
    let mut options = default_spawn_options();
    options.role = Some("coordinator".to_string());
    options.capabilities = vec![
        "delegate".to_string(),
        "audit".to_string(),
        "summarize".to_string(),
    ];
    options.agent_label = Some("swarm-plan-supervisor".to_string());
    options.verify = false;
    let spawned = spawn_single(
        state,
        None,
        &format!("plan-supervisor: {}", clean_text(goal, 120)),
        max_depth.max(2),
        &options,
    )?;
    spawned
        .get("session_id")
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| "missing_supervisor_session_id".to_string())
}

fn plans_start(state: &mut SwarmState, argv: &[String]) -> Result<Value, String> {
    let goal = parse_flag(argv, "goal")
        .map(|row| clean_text(&row, 220))
        .filter(|row| !row.is_empty())
        .ok_or_else(|| "plan_goal_required".to_string())?;
    let supervisor = ensure_supervisor_session_for_plan(
        state,
        parse_flag(argv, "supervisor-session-id").or_else(|| parse_flag(argv, "session-id")),
        &goal,
        parse_u8_flag(argv, "max-depth", 8),
    )?;
    let recursion_depth_limit = parse_u8_flag(argv, "plan-max-depth", 5).max(2);
    let plan_id = next_plan_id(state, &goal);
    let now = now_iso();
    let root_node_id = format!("{plan_id}-root");

    let mut nodes = BTreeMap::new();
    nodes.insert(
        root_node_id.clone(),
        SwarmPlanNode {
            node_id: root_node_id.clone(),
            parent_id: None,
            task: goal.clone(),
            status: "ready".to_string(),
            depth: 0,
            assignee_session_id: Some(supervisor.clone()),
            children: Vec::new(),
            summary: None,
            checkpoint_id: None,
            branch_state: None,
            updated_at: now.clone(),
        },
    );

    let mut plan = SwarmPlanGraph {
        plan_id: plan_id.clone(),
        goal: goal.clone(),
        supervisor_session_id: supervisor.clone(),
        root_node_id: root_node_id.clone(),
        status: "running".to_string(),
        recursion_depth_limit,
        created_at: now.clone(),
        updated_at: now.clone(),
        active_node_id: Some(root_node_id.clone()),
        nodes,
        checkpoints: BTreeMap::new(),
        branch_gates: BTreeMap::new(),
        merge_history: Vec::new(),
        speaker_stats: BTreeMap::new(),
    };

    let decomposed = decompose_goal_tasks(&goal);
    let mut root_children = Vec::new();
    for task in decomposed {
        let child_id = next_plan_node_id(&plan, &task, 1);
        plan.nodes.insert(
            child_id.clone(),
            SwarmPlanNode {
                node_id: child_id.clone(),
                parent_id: Some(root_node_id.clone()),
                task,
                status: "pending".to_string(),
                depth: 1,
                assignee_session_id: None,
                children: Vec::new(),
                summary: None,
                checkpoint_id: None,
                branch_state: None,
                updated_at: now.clone(),
            },
        );
        root_children.push(child_id);
    }
    if let Some(root) = plan.nodes.get_mut(&root_node_id) {
        root.children = root_children;
    }
    plan.merge_history.push(json!({
        "type": "plan_initialized",
        "node_count": plan.nodes.len(),
        "timestamp": now,
    }));

    state.plan_registry.insert(plan_id.clone(), plan.clone());
    append_event(
        state,
        json!({
            "type": "swarm_plan_started",
            "plan_id": plan_id,
            "goal": goal,
            "supervisor_session_id": supervisor,
            "lineage_parent_id": parse_flag(argv, "session-id"),
            "node_count": plan.nodes.len(),
            "timestamp": now_iso(),
        }),
    );

    Ok(json!({
        "ok": true,
        "type": "swarm_runtime_plan_start",
        "plan": plan,
    }))
}

fn plans_advance(state: &mut SwarmState, argv: &[String]) -> Result<Value, String> {
    let plan_id = parse_flag(argv, "plan-id")
        .filter(|row| !row.trim().is_empty())
        .ok_or_else(|| "plan_id_required".to_string())?;
    let max_steps = parse_u64_flag(argv, "max-steps", 3).max(1);
    let allow_replan = parse_bool_flag(argv, "allow-replan", true);
    let simulate_blocked = parse_bool_flag(argv, "simulate-blocked", false);

    if !state.plan_registry.contains_key(&plan_id) {
        return Err(format!("unknown_plan:{plan_id}"));
    }

    let mut steps = 0u64;
    let mut delegated = Vec::new();
    let mut replan_count = 0u64;
    while steps < max_steps {
        let snapshot = match state.plan_registry.get(&plan_id).cloned() {
            Some(value) => value,
            None => break,
        };
        let next_node = snapshot
            .nodes
            .values()
            .filter(|node| matches!(node.status.as_str(), "pending" | "ready"))
            .filter(|node| node.depth <= snapshot.recursion_depth_limit)
            .min_by_key(|node| (node.depth, node.node_id.clone()))
            .cloned();

        let Some(node) = next_node else {
            break;
        };

        let mut options = default_spawn_options();
        options.role = Some("worker".to_string());
        options.capabilities = vec!["execute".to_string(), "report".to_string()];
        options.verify = false;
        options.agent_label = Some(format!("plan-worker-{}", &plan_id[..8]));
        let spawned = spawn_single(
            state,
            Some(&snapshot.supervisor_session_id),
            &node.task,
            snapshot.recursion_depth_limit.saturating_add(2),
            &options,
        )?;
        let assignee = spawned
            .get("session_id")
            .and_then(Value::as_str)
            .map(ToString::to_string)
            .ok_or_else(|| "plan_spawn_missing_session_id".to_string())?;

        let mut summary = format!("delegated `{}` to {}", clean_text(&node.task, 80), assignee);
        if simulate_blocked && steps == 0 {
            summary.push_str(" (blocked_on_dependency)");
        }

        if let Some(plan) = state.plan_registry.get_mut(&plan_id) {
            if let Some(mut_node) = plan.nodes.get_mut(&node.node_id) {
                mut_node.assignee_session_id = Some(assignee.clone());
                mut_node.status = "merged".to_string();
                mut_node.summary = Some(summary.clone());
                mut_node.updated_at = now_iso();
            }
            plan.active_node_id = Some(node.node_id.clone());
            plan.updated_at = now_iso();
            plan.merge_history.push(json!({
                "type": "delegate_merge",
                "node_id": node.node_id,
                "task": node.task,
                "assignee_session_id": assignee,
                "summary": summary,
                "timestamp": now_iso(),
            }));

            let should_replan = allow_replan
                && node.depth < plan.recursion_depth_limit
                && summary.contains("blocked");
            if should_replan {
                for suffix in ["investigate blocker", "execute fallback"] {
                    let task = format!("{} :: {}", clean_text(&node.task, 90), suffix);
                    let child_id = next_plan_node_id(plan, &task, node.depth.saturating_add(1));
                    plan.nodes.insert(
                        child_id.clone(),
                        SwarmPlanNode {
                            node_id: child_id.clone(),
                            parent_id: Some(node.node_id.clone()),
                            task,
                            status: "pending".to_string(),
                            depth: node.depth.saturating_add(1),
                            assignee_session_id: None,
                            children: Vec::new(),
                            summary: None,
                            checkpoint_id: None,
                            branch_state: None,
                            updated_at: now_iso(),
                        },
                    );
                    if let Some(parent) = plan.nodes.get_mut(&node.node_id) {
                        parent.children.push(child_id);
                    }
                }
                replan_count = replan_count.saturating_add(1);
            }
        }

        delegated.push(spawned);
        steps = steps.saturating_add(1);
    }

    if let Some(plan) = state.plan_registry.get_mut(&plan_id) {
        let remaining = plan
            .nodes
            .values()
            .filter(|node| matches!(node.status.as_str(), "pending" | "ready"))
            .count();
        if remaining == 0 {
            plan.status = "completed".to_string();
        }
    }

    let snapshot = state
        .plan_registry
        .get(&plan_id)
        .cloned()
        .ok_or_else(|| format!("unknown_plan:{plan_id}"))?;
    Ok(json!({
        "ok": true,
        "type": "swarm_runtime_plan_advance",
        "plan_id": plan_id,
        "steps_executed": steps,
        "replan_count": replan_count,
        "delegated": delegated,
        "plan_status": snapshot.status,
        "active_node_id": snapshot.active_node_id,
    }))
}

fn plans_checkpoint(state: &mut SwarmState, argv: &[String]) -> Result<Value, String> {
    let plan_id = parse_flag(argv, "plan-id")
        .filter(|row| !row.trim().is_empty())
        .ok_or_else(|| "plan_id_required".to_string())?;
    let checkpoint_id = parse_flag(argv, "checkpoint-id");
    let Some(plan) = state.plan_registry.get_mut(&plan_id) else {
        return Err(format!("unknown_plan:{plan_id}"));
    };

    if let Some(checkpoint_id) = checkpoint_id {
        let Some(checkpoint) = plan.checkpoints.get(&checkpoint_id).cloned() else {
            return Err(format!("unknown_checkpoint:{checkpoint_id}"));
        };
        if let Some(node) = plan.nodes.get_mut(&checkpoint.node_id) {
            node.status = "resumed".to_string();
            node.checkpoint_id = Some(checkpoint_id.clone());
            node.updated_at = now_iso();
        }
        plan.updated_at = now_iso();
        return Ok(json!({
            "ok": true,
            "type": "swarm_runtime_plan_checkpoint_resume",
            "plan_id": plan_id,
            "checkpoint": checkpoint,
        }));
    }

    let node_id = parse_flag(argv, "node-id")
        .filter(|row| !row.trim().is_empty())
        .ok_or_else(|| "node_id_required".to_string())?;
    if !plan.nodes.contains_key(&node_id) {
        return Err(format!("unknown_plan_node:{node_id}"));
    }
    let state_payload = parse_json_flag(argv, "state-json").unwrap_or_else(|| {
        json!({
            "snapshot": "implicit",
            "node_id": node_id,
        })
    });
    let checkpoint_id = format!(
        "chk-{}",
        &deterministic_receipt_hash(&json!({
            "plan_id": plan_id,
            "node_id": node_id,
            "ts": now_epoch_ms(),
            "state": state_payload,
        }))[..12]
    );
    let checkpoint = PlanCheckpoint {
        checkpoint_id: checkpoint_id.clone(),
        node_id: node_id.clone(),
        state: state_payload,
        created_at: now_iso(),
        resumable: true,
        version: "swarm_plan_checkpoint_v1".to_string(),
    };
    plan.checkpoints
        .insert(checkpoint_id.clone(), checkpoint.clone());
    if let Some(node) = plan.nodes.get_mut(&node_id) {
        node.checkpoint_id = Some(checkpoint_id.clone());
        node.updated_at = now_iso();
    }
    plan.updated_at = now_iso();
    Ok(json!({
        "ok": true,
        "type": "swarm_runtime_plan_checkpoint",
        "plan_id": plan_id,
        "checkpoint": checkpoint,
    }))
}

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

fn plans_status(state: &SwarmState, argv: &[String]) -> Value {
    if let Some(plan_id) = parse_flag(argv, "plan-id").filter(|row| !row.trim().is_empty()) {
        if let Some(plan) = state.plan_registry.get(&plan_id) {
            return json!({
                "ok": true,
                "type": "swarm_runtime_plan_status",
                "plan": plan,
            });
        }
        return json!({
            "ok": false,
            "type": "swarm_runtime_plan_status",
            "error": format!("unknown_plan:{plan_id}"),
        });
    }
    let plans = state
        .plan_registry
        .values()
        .map(|plan| {
            json!({
                "plan_id": plan.plan_id,
                "goal": plan.goal,
                "status": plan.status,
                "node_count": plan.nodes.len(),
                "checkpoint_count": plan.checkpoints.len(),
                "updated_at": plan.updated_at,
            })
        })
        .collect::<Vec<_>>();
    json!({
        "ok": true,
        "type": "swarm_runtime_plan_status",
        "plan_count": plans.len(),
        "plans": plans,
    })
}

fn run_plans_command(state: &mut SwarmState, argv: &[String]) -> Result<Value, String> {
    let sub = argv
        .get(1)
        .map(|row| row.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());
    match sub.as_str() {
        "start" => plans_start(state, argv),
        "advance" => plans_advance(state, argv),
        "checkpoint" => plans_checkpoint(state, argv),
        "branch-gate" => plans_branch_gate(state, argv),
        "speaker-select" => plans_speaker_select(state, argv),
        "status" => Ok(plans_status(state, argv)),
        _ => Err(format!("unknown_plans_subcommand:{sub}")),
    }
}

