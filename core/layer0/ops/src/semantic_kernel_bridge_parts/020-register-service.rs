fn register_service(state: &mut Value, payload: &Map<String, Value>) -> Result<Value, String> {
    let name = clean_token(
        payload.get("name").and_then(Value::as_str),
        "semantic-kernel-service",
    );
    let role = clean_token(payload.get("role").and_then(Value::as_str), "orchestrator");
    let execution_surface = clean_token(
        payload.get("execution_surface").and_then(Value::as_str),
        "workflow-executor",
    );
    if !allowed_service_surface(&execution_surface) {
        return Err("semantic_kernel_service_surface_invalid".to_string());
    }
    let service = json!({
        "service_id": stable_id("sksvc", &json!({"name": name, "role": role, "surface": execution_surface})),
        "name": name,
        "role": role,
        "execution_surface": execution_surface,
        "description": clean_text(payload.get("description").and_then(Value::as_str), 240),
        "default_budget": parse_u64_value(payload.get("default_budget"), 512, 32, 8192),
        "capabilities": payload.get("capabilities").cloned().filter(Value::is_array).unwrap_or_else(|| json!([])),
        "registered_at": now_iso(),
    });
    let service_id = service
        .get("service_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "services").insert(service_id.clone(), service.clone());
    Ok(json!({
        "ok": true,
        "service": service,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-008.1", semantic_claim("V6-WORKFLOW-008.1")),
    }))
}

fn allowed_plugin_kind(kind: &str) -> bool {
    matches!(kind, "native" | "prompt" | "openapi" | "mcp")
}

fn register_plugin(
    root: &Path,
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let service_id = clean_token(payload.get("service_id").and_then(Value::as_str), "");
    if service_id.is_empty() || !as_object_mut(state, "services").contains_key(&service_id) {
        return Err("semantic_kernel_plugin_service_not_found".to_string());
    }
    let plugin_name = clean_token(
        payload.get("plugin_name").and_then(Value::as_str),
        "semantic-kernel-plugin",
    );
    let plugin_kind = clean_token(payload.get("plugin_kind").and_then(Value::as_str), "native");
    if !allowed_plugin_kind(&plugin_kind) {
        return Err("semantic_kernel_plugin_kind_invalid".to_string());
    }
    let bridge_path = normalize_bridge_path(
        root,
        payload
            .get("bridge_path")
            .and_then(Value::as_str)
            .unwrap_or("adapters/cognition/skills/mcp/mcp_gateway.ts"),
    )?;
    let entrypoint = clean_token(payload.get("entrypoint").and_then(Value::as_str), "invoke");
    let openapi_url = clean_text(payload.get("openapi_url").and_then(Value::as_str), 200);
    if plugin_kind == "openapi"
        && !(openapi_url.starts_with("https://") || openapi_url.ends_with("openapi.json"))
    {
        return Err("semantic_kernel_openapi_url_invalid".to_string());
    }
    let template = clean_text(payload.get("prompt_template").and_then(Value::as_str), 400);
    let plugin = json!({
        "plugin_id": stable_id("skplug", &json!({"service_id": service_id, "plugin_name": plugin_name, "plugin_kind": plugin_kind, "bridge_path": bridge_path})),
        "service_id": service_id,
        "plugin_name": plugin_name,
        "plugin_kind": plugin_kind,
        "bridge_path": bridge_path,
        "entrypoint": entrypoint,
        "openapi_url": openapi_url,
        "prompt_template": template,
        "schema": payload.get("schema").cloned().unwrap_or(Value::Null),
        "registered_at": now_iso(),
        "invocation_count": 0,
        "fail_closed": true,
    });
    let plugin_id = plugin
        .get("plugin_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "plugins").insert(plugin_id.clone(), plugin.clone());
    Ok(json!({
        "ok": true,
        "plugin": plugin,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-008.2", semantic_claim("V6-WORKFLOW-008.2")),
    }))
}

fn replace_template(template: &str, args: &Map<String, Value>) -> String {
    let mut out = template.to_string();
    for (key, value) in args {
        let replacement = value
            .as_str()
            .map(ToString::to_string)
            .unwrap_or_else(|| value.to_string());
        out = out.replace(&format!("{{{{{key}}}}}"), &replacement);
    }
    out
}

fn invoke_plugin(state: &mut Value, payload: &Map<String, Value>) -> Result<Value, String> {
    let plugin_id = clean_token(payload.get("plugin_id").and_then(Value::as_str), "");
    let args = payload
        .get("args")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    if plugin_id.is_empty() {
        return Err("semantic_kernel_plugin_id_required".to_string());
    }
    let plugins = as_object_mut(state, "plugins");
    let plugin = plugins
        .get_mut(&plugin_id)
        .and_then(Value::as_object_mut)
        .ok_or_else(|| "semantic_kernel_plugin_not_found".to_string())?;
    let plugin_kind = plugin
        .get("plugin_kind")
        .and_then(Value::as_str)
        .unwrap_or("native");
    let rendered = if plugin_kind == "prompt" {
        replace_template(
            plugin
                .get("prompt_template")
                .and_then(Value::as_str)
                .unwrap_or(""),
            &args,
        )
    } else {
        String::new()
    };
    let invocation = match plugin_kind {
        "prompt" => json!({
            "mode": "prompt_render",
            "rendered": rendered,
        }),
        "openapi" => json!({
            "mode": "openapi_request",
            "target": plugin.get("openapi_url").cloned().unwrap_or(Value::Null),
            "method": payload.get("method").cloned().unwrap_or_else(|| json!("POST")),
            "path": payload.get("path").cloned().unwrap_or_else(|| json!("/invoke")),
            "body": Value::Object(args.clone()),
        }),
        "mcp" => json!({
            "mode": "mcp_tool_call",
            "tool": payload.get("tool").cloned().unwrap_or_else(|| json!(plugin.get("plugin_name").cloned().unwrap_or_else(|| json!("tool")))),
            "arguments": Value::Object(args.clone()),
        }),
        _ => json!({
            "mode": "native_function",
            "entrypoint": plugin.get("entrypoint").cloned().unwrap_or_else(|| json!("invoke")),
            "arguments": Value::Object(args.clone()),
        }),
    };
    let invocation_count = plugin
        .get("invocation_count")
        .and_then(Value::as_u64)
        .unwrap_or(0)
        .saturating_add(1);
    plugin.insert("invocation_count".to_string(), json!(invocation_count));
    plugin.insert("last_invoked_at".to_string(), json!(now_iso()));
    Ok(json!({
        "ok": true,
        "plugin_id": plugin_id,
        "invocation": invocation,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-008.2", semantic_claim("V6-WORKFLOW-008.2")),
    }))
}

fn collaborate(
    root: &Path,
    argv: &[String],
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let collaboration_name = clean_token(
        payload.get("name").and_then(Value::as_str),
        "semantic-kernel-collaboration",
    );
    let agents = payload
        .get("agents")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if agents.is_empty() {
        return Err("semantic_kernel_collaboration_agents_required".to_string());
    }
    let swarm_state_path = semantic_swarm_state_path(root, argv, payload);
    let mut session_ids = BTreeMap::new();
    let coordinator_task = format!("semantic-kernel:{}:coordinator", collaboration_name);
    let child_budget_sum = agents
        .iter()
        .filter_map(|row| row.get("budget").and_then(Value::as_u64))
        .sum::<u64>();
    let edge_count = payload
        .get("edges")
        .and_then(Value::as_array)
        .map(|rows| rows.len() as u64)
        .unwrap_or(0);
    let max_budget = child_budget_sum
        .saturating_add(edge_count.saturating_mul(96))
        .saturating_add(2048)
        .clamp(2048, 16384);
    let coordinator_exit = crate::swarm_runtime::run(
        root,
        &[
            "spawn".to_string(),
            format!("--task={coordinator_task}"),
            format!("--max-tokens={max_budget}"),
            "--agent-label=semantic-kernel-coordinator".to_string(),
            format!("--state-path={}", swarm_state_path.display()),
        ],
    );
    if coordinator_exit != 0 {
        return Err("semantic_kernel_collaboration_coordinator_spawn_failed".to_string());
    }
    let swarm_state = read_swarm_state(&swarm_state_path);
    let coordinator_id = find_swarm_session_id_by_task(&swarm_state, &coordinator_task)
        .ok_or_else(|| "semantic_kernel_collaboration_coordinator_missing".to_string())?;
    session_ids.insert("coordinator".to_string(), coordinator_id.clone());

    let mut node_specs = Vec::new();
    for agent in &agents {
        let label = clean_token(agent.get("label").and_then(Value::as_str), "agent");
        let role = clean_token(agent.get("role").and_then(Value::as_str), "specialist");
        let task = format!(
            "semantic-kernel:{}:{}:{}",
            collaboration_name,
            label,
            clean_text(agent.get("task").and_then(Value::as_str), 80)
        );
        let budget = parse_u64_value(agent.get("budget"), 256, 32, 4096);
        node_specs.push(json!({
            "label": label,
            "role": role,
            "task": task,
            "token_budget": budget,
            "context": {
                "semantic_kernel_collaboration": collaboration_name,
                "semantic_kernel_role": role,
            }
        }));
    }

    let mut edge_specs = Vec::new();
    for edge in payload
        .get("edges")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
    {
        let from = clean_token(edge.get("from").and_then(Value::as_str), "");
        let to = clean_token(edge.get("to").and_then(Value::as_str), "");
        if from.is_empty() || to.is_empty() {
            continue;
        }
        let reason = clean_text(edge.get("reason").and_then(Value::as_str), 120);
        edge_specs.push(json!({
            "from": from,
            "to": to,
            "relation": edge.get("relation").cloned().unwrap_or_else(|| json!("handoff")),
            "importance": parse_f64_value(edge.get("importance"), 0.8, 0.0, 1.0),
            "auto_handoff": true,
            "reason": reason,
            "context": {
                "semantic_kernel_reason": reason,
                "semantic_kernel_collaboration": collaboration_name,
            },
        }));
    }

    let network_name = format!("semantic-kernel-{}", collaboration_name);
    let network_spec = json!({
        "name": network_name,
        "nodes": node_specs,
        "edges": edge_specs,
    });
    let network_exit = crate::swarm_runtime::run(
        root,
        &[
            "networks".to_string(),
            "create".to_string(),
            format!("--session-id={coordinator_id}"),
            format!("--spec-json={}", encode_json_arg(&network_spec)?),
            format!("--state-path={}", swarm_state_path.display()),
        ],
    );
    if network_exit != 0 {
        return Err("semantic_kernel_network_create_failed".to_string());
    }
    let final_swarm_state = read_swarm_state(&swarm_state_path);
    let network_id = find_swarm_network_id_by_name(&final_swarm_state, &network_name)
        .ok_or_else(|| "semantic_kernel_network_missing".to_string())?;
    let network = find_swarm_network_by_name(&final_swarm_state, &network_name)
        .ok_or_else(|| "semantic_kernel_network_receipt_missing".to_string())?;
    if let Some(nodes) = network.get("nodes").and_then(Value::as_array) {
        for node in nodes {
            let Some(label) = node.get("label").and_then(Value::as_str) else {
                continue;
            };
            let Some(session_id) = node.get("session_id").and_then(Value::as_str) else {
                continue;
            };
            session_ids.insert(label.to_string(), session_id.to_string());
        }
    }
    let collaboration = json!({
        "collaboration_id": stable_id("skcollab", &json!({"name": collaboration_name, "network": network_id})),
        "name": collaboration_name,
        "coordinator_session_id": coordinator_id,
        "session_ids": session_ids,
        "swarm_state_path": rel(root, &swarm_state_path),
        "network_id": network_id,
        "registered_at": now_iso(),
    });
    let collaboration_id = collaboration
        .get("collaboration_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "collaborations").insert(collaboration_id, collaboration.clone());
    Ok(json!({
        "ok": true,
        "collaboration": collaboration,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-008.3", semantic_claim("V6-WORKFLOW-008.3")),
    }))
}

fn parse_function_specs(payload: &Map<String, Value>) -> Vec<Map<String, Value>> {
    payload
        .get("functions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| row.as_object().cloned())
        .collect()
}

fn plan(state: &mut Value, payload: &Map<String, Value>) -> Result<Value, String> {
    let objective = clean_text(payload.get("objective").and_then(Value::as_str), 200);
    if objective.is_empty() {
        return Err("semantic_kernel_planner_objective_required".to_string());
    }
    let service_id = clean_token(payload.get("service_id").and_then(Value::as_str), "");
    if service_id.is_empty() || !as_object_mut(state, "services").contains_key(&service_id) {
        return Err("semantic_kernel_planner_service_not_found".to_string());
    }
    let mut functions = parse_function_specs(payload);
    let objective_lc = objective.to_ascii_lowercase();
    functions.sort_by(|a, b| {
        let a_name = clean_token(a.get("name").and_then(Value::as_str), "fn");
        let b_name = clean_token(b.get("name").and_then(Value::as_str), "fn");
        let a_score = parse_f64_value(a.get("score"), 0.5, 0.0, 1.0)
            + if has_token(&objective_lc, &a_name) {
                0.25
            } else {
                0.0
            };
        let b_score = parse_f64_value(b.get("score"), 0.5, 0.0, 1.0)
            + if has_token(&objective_lc, &b_name) {
                0.25
            } else {
                0.0
            };
        b_score
            .partial_cmp(&a_score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a_name.cmp(&b_name))
    });
    let max_steps = parse_u64_value(payload.get("max_steps"), 4, 1, 16) as usize;
    let selected = functions.into_iter().take(max_steps).collect::<Vec<_>>();
    let plan_steps = selected
        .iter()
        .enumerate()
        .map(|(index, row)| {
            let name = clean_token(row.get("name").and_then(Value::as_str), "function");
            json!({
                "step_id": format!("step-{}", index + 1),
                "function_name": name,
                "description": clean_text(row.get("description").and_then(Value::as_str), 160),
                "checkpoint_key": format!("workflow.{}.{}", service_id, name),
                "execution_surface": "workflow-executor",
                "function_selection_score": parse_f64_value(row.get("score"), 0.5, 0.0, 1.0),
            })
        })
        .collect::<Vec<_>>();
    let plan = json!({
        "plan_id": stable_id("skplan", &json!({"service_id": service_id, "objective": objective, "steps": plan_steps})),
        "service_id": service_id,
        "objective": objective,
        "routing_mode": clean_token(payload.get("routing_mode").and_then(Value::as_str), "sequential"),
        "steps": plan_steps,
        "registered_at": now_iso(),
    });
    let plan_id = plan
        .get("plan_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "plans").insert(plan_id.clone(), plan.clone());
    Ok(json!({
        "ok": true,
        "plan": plan,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-008.4", semantic_claim("V6-WORKFLOW-008.4")),
    }))
}

fn supported_vector_provider(provider: &str) -> bool {
    matches!(
        provider,
        "azure-ai-search" | "chroma" | "elasticsearch" | "memory-plane"
    )
}

