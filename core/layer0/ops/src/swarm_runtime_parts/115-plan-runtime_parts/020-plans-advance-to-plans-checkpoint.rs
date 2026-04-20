
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
