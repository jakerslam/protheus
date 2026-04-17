fn execute_spawn_subagents_tool(
    root: &Path,
    snapshot: &Value,
    actor: &str,
    input: &Value,
    headers: &[(&str, &str)],
) -> Value {
    let spawn_policy = spawn_guard_policy(root);
    let max_per_spawn = spawn_policy
        .get("max_per_spawn")
        .and_then(Value::as_i64)
        .unwrap_or(8)
        .clamp(1, 64) as usize;
    let max_descendants_per_parent = spawn_policy
        .get("max_descendants_per_parent")
        .and_then(Value::as_i64)
        .unwrap_or(24)
        .clamp(1, 4096) as usize;
    let depth_limit = spawn_policy
        .get("max_depth")
        .and_then(Value::as_i64)
        .unwrap_or(4)
        .clamp(1, 32) as usize;
    let per_child_budget_default = spawn_policy
        .get("per_child_budget_default")
        .and_then(Value::as_i64)
        .unwrap_or(800)
        .clamp(64, 200_000);
    let per_child_budget_max = spawn_policy
        .get("per_child_budget_max")
        .and_then(Value::as_i64)
        .unwrap_or(5000)
        .clamp(per_child_budget_default, 2_000_000);
    let spawn_budget_cap = spawn_policy
        .get("spawn_budget_cap")
        .and_then(Value::as_i64)
        .unwrap_or(per_child_budget_max.saturating_mul(max_per_spawn as i64))
        .clamp(per_child_budget_max, 20_000_000);

    let requested_count_raw = input
        .get("count")
        .or_else(|| input.get("team_size"))
        .or_else(|| input.get("agents"))
        .and_then(Value::as_i64)
        .unwrap_or(3);
    let requested_count_raw_pos = requested_count_raw.max(1) as usize;
    let requested_count = requested_count_raw_pos.min(max_per_spawn);
    let expiry_seconds = input
        .get("expiry_seconds")
        .or_else(|| input.get("lifespan_sec"))
        .and_then(Value::as_i64)
        .unwrap_or(3600)
        .clamp(60, 172_800);
    let budget_tokens_requested_raw = input
        .get("budget_tokens")
        .or_else(|| input.get("token_budget"))
        .and_then(Value::as_i64)
        .unwrap_or(per_child_budget_default);
    let budget_tokens = budget_tokens_requested_raw.clamp(64, per_child_budget_max);
    let budget_tokens_for_capacity = budget_tokens_requested_raw.clamp(64, spawn_budget_cap);
    let objective = clean_text(
        input
            .get("objective")
            .or_else(|| input.get("task"))
            .or_else(|| input.get("message"))
            .and_then(Value::as_str)
            .unwrap_or("Parallel child task requested by parent directive."),
        800,
    );
    let explicit_initial_prompt = clean_text(
        input
            .get("initial_prompt")
            .or_else(|| input.get("system_prompt"))
            .or_else(|| input.get("prompt"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        12_000,
    );
    let contract_lifespan = match clean_text(
        input
            .get("lifespan")
            .and_then(Value::as_str)
            .unwrap_or(""),
        40,
    )
    .to_ascii_lowercase()
    .as_str()
    {
        "task" => "task",
        "permanent" | "indefinite" => "permanent",
        _ => "ephemeral",
    }
    .to_string();
    let parent_row = agent_row_by_id(root, snapshot, actor);
    let parent_permissions_manifest = parent_row
        .as_ref()
        .and_then(permissions_manifest_from_agent_row)
        .unwrap_or_else(default_permissions_manifest);
    let requested_permissions_manifest = input
        .get("permissions")
        .or_else(|| input.get("permissions_manifest"))
        .and_then(parse_permissions_payload)
        .map(|value| normalize_permissions_manifest(&value));
    let child_permissions_manifest = requested_permissions_manifest
        .map(|requested| clamp_child_permissions_manifest(&parent_permissions_manifest, &requested))
        .unwrap_or_else(|| parent_permissions_manifest.clone());
    let merge_strategy = match clean_text(
        input
            .get("merge_strategy")
            .or_else(|| input.get("merge"))
            .and_then(Value::as_str)
            .unwrap_or("reduce"),
        40,
    )
    .to_ascii_lowercase()
    .as_str()
    {
        "voting" | "vote" => "voting",
        "concat" | "concatenate" => "concatenate",
        _ => "reduce",
    }
    .to_string();
    let mut role_plan = input
        .get("roles")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|row| clean_text(row, 60))
                .filter(|row| !row.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let role_hint = clean_text(
        input
            .get("role")
            .or_else(|| input.get("default_role"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        60,
    );
    if !role_hint.is_empty() && role_plan.is_empty() {
        role_plan.push(role_hint);
    }
    if role_plan.is_empty() {
        role_plan = vec![
            "analyst".to_string(),
            "researcher".to_string(),
            "builder".to_string(),
            "reviewer".to_string(),
        ];
    }
    let base_name = clean_text(
        input
            .get("base_name")
            .or_else(|| input.get("name_prefix"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        80,
    );
    let parent_map = agent_parent_map(root, snapshot);
    let current_depth = agent_depth_from_parent_map(&parent_map, actor);
    if current_depth + 1 > depth_limit {
        return json!({
            "ok": false,
            "error": "spawn_depth_limit_exceeded",
            "parent_agent_id": actor,
            "current_depth": current_depth,
            "max_depth": depth_limit
        });
    }
    let existing_descendants = descendant_count(&parent_map, actor);
    if existing_descendants >= max_descendants_per_parent {
        return json!({
            "ok": false,
            "error": "spawn_descendant_limit_exceeded",
            "parent_agent_id": actor,
            "existing_descendants": existing_descendants,
            "max_descendants_per_parent": max_descendants_per_parent
        });
    }
    let remaining_capacity = max_descendants_per_parent.saturating_sub(existing_descendants);
    let budget_limited_count = ((spawn_budget_cap / budget_tokens_for_capacity.max(1)) as usize).max(1);
    let effective_count = requested_count
        .min(remaining_capacity.max(1))
        .min(budget_limited_count.max(1));
    if effective_count == 0 {
        return json!({
            "ok": false,
            "error": "spawn_budget_exceeded",
            "parent_agent_id": actor,
            "spawn_budget_cap": spawn_budget_cap,
            "requested_budget_tokens": budget_tokens
        });
    }
    let context_slice = subagent_context_slice(root, actor, &objective);
    let directive_receipt = crate::deterministic_receipt_hash(&json!({
        "type": "agent_spawn_directive",
        "actor_agent_id": actor,
        "requested_count_raw": requested_count_raw,
        "requested_count": requested_count,
        "effective_count": effective_count,
        "objective": objective,
        "merge_strategy": merge_strategy,
        "budget_tokens": budget_tokens,
        "budget_tokens_requested_raw": budget_tokens_requested_raw,
        "budget_tokens_for_capacity": budget_tokens_for_capacity,
        "requested_at": crate::now_iso()
    }));

    let mut created = Vec::<Value>::new();
    let mut errors = Vec::<Value>::new();
    for idx in 0..effective_count {
        let role = role_plan
            .get(idx % role_plan.len())
            .cloned()
            .unwrap_or_else(|| "analyst".to_string());
        let child_initial_prompt = if explicit_initial_prompt.is_empty() {
            clean_text(
                &format!(
                    "You are a delegated subagent for parent {actor}. Objective: {objective}. Keep updates concise, evidence-backed, and escalate blockers early."
                ),
                12_000,
            )
        } else {
            explicit_initial_prompt.clone()
        };
        let mut request_body = json!({
            "role": role,
            "parent_agent_id": actor,
            "system_prompt": child_initial_prompt.clone(),
            "contract": {
                "owner": "descendant_auto_spawn",
                "mission": if objective.is_empty() {
                    format!("Parallel subtask for parent {}", actor)
                } else {
                    format!("Parallel subtask for parent {}: {}", actor, objective)
                },
                "initial_prompt": child_initial_prompt,
                "permissions_manifest": child_permissions_manifest.clone(),
                "lifespan": contract_lifespan.clone(),
                "termination_condition": "task_or_timeout",
                "expiry_seconds": expiry_seconds,
                "auto_terminate_allowed": true,
                "budget_tokens": budget_tokens,
                "merge_strategy": merge_strategy,
                "context_slice": context_slice,
                "source_user_directive": objective,
                "source_user_directive_receipt": directive_receipt,
                "spawn_guard": {
                    "max_depth": depth_limit,
                    "max_descendants_per_parent": max_descendants_per_parent,
                    "spawn_budget_cap": spawn_budget_cap
                }
            }
        });
        if !base_name.is_empty() {
            request_body["name"] = json!(format!("{base_name}-{}", idx + 1));
        }
        let body_bytes = serde_json::to_vec(&request_body).unwrap_or_default();
        let spawned = handle_with_headers(root, "POST", "/api/agents", &body_bytes, headers, snapshot)
            .map(|response| response.payload)
            .unwrap_or_else(|| json!({"ok": false, "error": "tool_route_not_found"}));
        if spawned.get("ok").and_then(Value::as_bool).unwrap_or(false) {
            created.push(json!({
                "agent_id": clean_agent_id(
                    spawned
                        .get("agent_id")
                        .or_else(|| spawned.get("id"))
                        .and_then(Value::as_str)
                        .unwrap_or("")
                ),
                "name": clean_text(spawned.get("name").and_then(Value::as_str).unwrap_or(""), 120),
                "role": role
            }));
        } else {
            errors.push(json!({
                "role": role,
                "error": clean_text(spawned.get("error").and_then(Value::as_str).unwrap_or("spawn_failed"), 160)
            }));
        }
    }
    let mut out = json!({
        "ok": !created.is_empty(),
        "type": "spawn_subagents",
        "parent_agent_id": actor,
        "requested_count_raw": requested_count_raw,
        "requested_count": requested_count,
        "effective_count": effective_count,
        "created_count": created.len(),
        "failed_count": errors.len(),
        "directive": {
            "objective": objective,
            "receipt": directive_receipt,
            "merge_strategy": merge_strategy,
            "budget_tokens": budget_tokens
        },
        "circuit_breakers": {
            "max_depth": depth_limit,
            "current_depth": current_depth,
            "existing_descendants": existing_descendants,
            "max_descendants_per_parent": max_descendants_per_parent,
            "spawn_budget_cap": spawn_budget_cap,
            "remaining_capacity": remaining_capacity,
            "degraded": effective_count < requested_count_raw_pos
        },
        "children": created,
        "errors": errors
    });
    out["receipt_hash"] = json!(crate::deterministic_receipt_hash(&out));
    out
}
