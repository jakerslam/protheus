
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

