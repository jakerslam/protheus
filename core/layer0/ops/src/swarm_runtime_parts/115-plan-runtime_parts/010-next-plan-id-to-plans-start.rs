fn next_plan_id(state: &SwarmState, goal: &str) -> String {
    let digest = crate::deterministic_receipt_hash(&json!({
        "goal": goal,
        "ts": now_epoch_ms(),
        "plans": state.plan_registry.len(),
    }));
    format!("plan-{}", &digest[..12])
}

fn next_plan_node_id(plan: &SwarmPlanGraph, seed: &str, depth: u8) -> String {
    let digest = crate::deterministic_receipt_hash(&json!({
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
