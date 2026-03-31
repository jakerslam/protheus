fn doc_token_set(doc: &Value) -> BTreeSet<String> {
    clean_text(doc.get("text").and_then(Value::as_str), 4096)
        .to_ascii_lowercase()
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|row| !row.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn query_terms(query: &str) -> Vec<String> {
    clean_text(Some(query), 240)
        .to_ascii_lowercase()
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|row| !row.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn retrieval_score(doc: &Value, terms: &[String], mode: &str) -> i64 {
    let tokens = doc_token_set(doc);
    let mut score = 0i64;
    for term in terms {
        if tokens.contains(term) {
            score += match mode {
                "ranker" => 4,
                "vector" => 3,
                _ => 2,
            };
        }
    }
    if mode == "hybrid"
        && doc
            .get("metadata")
            .and_then(|row| row.get("kind"))
            .and_then(Value::as_str)
            == Some("graph")
    {
        score += 2;
    }
    score
}

fn render_template_text(template: &str, variables: &Map<String, Value>) -> String {
    let mut out = template.to_string();
    for (key, value) in variables {
        let replacement = value
            .as_str()
            .map(|row| clean_text(Some(row), 4000))
            .unwrap_or_else(|| value.to_string());
        out = out.replace(&format!("{{{{{key}}}}}"), &replacement);
    }
    out
}

fn allowed_connector_type(kind: &str) -> bool {
    matches!(
        kind,
        "mcp"
            | "openapi"
            | "filesystem"
            | "pgvector"
            | "qdrant"
            | "weaviate"
            | "elasticsearch"
            | "opensearch"
            | "s3"
            | "http"
    )
}

fn register_pipeline(state: &mut Value, payload: &Map<String, Value>) -> Result<Value, String> {
    let name = clean_token(
        payload.get("name").and_then(Value::as_str),
        "haystack-pipeline",
    );
    let components = payload
        .get("components")
        .or_else(|| payload.get("stages"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if components.is_empty() {
        return Err("haystack_pipeline_components_required".to_string());
    }
    let normalized = components
        .into_iter()
        .map(|component| {
            let obj = component.as_object().cloned().unwrap_or_default();
            json!({
                "id": clean_token(obj.get("id").and_then(Value::as_str), "stage"),
                "stage_type": clean_token(obj.get("stage_type").and_then(Value::as_str).or_else(|| obj.get("type").and_then(Value::as_str)), "generator"),
                "input_type": clean_token(obj.get("input_type").and_then(Value::as_str), "text"),
                "output_type": clean_token(obj.get("output_type").and_then(Value::as_str), "text"),
                "parallel": parse_bool_value(obj.get("parallel"), false),
                "spawn": parse_bool_value(obj.get("spawn"), false),
                "budget": parse_u64_value(obj.get("budget"), 192, 32, 4096),
            })
        })
        .collect::<Vec<_>>();
    let pipeline = json!({
        "pipeline_id": stable_id("haypipe", &json!({"name": name, "components": normalized.len()})),
        "name": name,
        "components": normalized,
        "registered_at": now_iso(),
    });
    let pipeline_id = pipeline
        .get("pipeline_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "pipelines").insert(pipeline_id, pipeline.clone());
    Ok(json!({
        "ok": true,
        "pipeline": pipeline,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-012.1", haystack_claim("V6-WORKFLOW-012.1")),
    }))
}

fn run_pipeline(
    root: &Path,
    argv: &[String],
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let pipeline_id = clean_token(payload.get("pipeline_id").and_then(Value::as_str), "");
    if pipeline_id.is_empty() {
        return Err("haystack_pipeline_id_required".to_string());
    }
    let profile = clean_token(payload.get("profile").and_then(Value::as_str), "rich");
    let pipeline = state
        .get("pipelines")
        .and_then(Value::as_object)
        .and_then(|rows| rows.get(&pipeline_id))
        .cloned()
        .ok_or_else(|| format!("unknown_haystack_pipeline:{pipeline_id}"))?;
    let components = pipeline
        .get("components")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let parallel_count = components
        .iter()
        .filter(|row| row.get("parallel").and_then(Value::as_bool) == Some(true))
        .count();
    let degraded = matches!(profile.as_str(), "pure" | "tiny-max") && parallel_count > 1;
    let swarm_state_path = swarm_state_path(root, argv, payload);
    let root_session_id = if components.iter().any(|row| {
        row.get("spawn").and_then(Value::as_bool) == Some(true)
            || matches!(
                row.get("stage_type").and_then(Value::as_str),
                Some("generator" | "tool" | "agent")
            )
    }) {
        Some(ensure_session_for_task(
            root,
            &swarm_state_path,
            &format!(
                "haystack:pipeline:{}",
                clean_token(pipeline.get("name").and_then(Value::as_str), "pipeline")
            ),
            "haystack-pipeline",
            Some("pipeline"),
            None,
            parse_u64_value(payload.get("budget"), 896, 96, 12288),
        )?)
    } else {
        None
    };
    let mut selected_parallel = 0usize;
    let visited = components
        .into_iter()
        .map(|component| {
            let is_parallel = component.get("parallel").and_then(Value::as_bool) == Some(true);
            let selected = if degraded && is_parallel {
                selected_parallel += 1;
                selected_parallel == 1
            } else {
                true
            };
            json!({
                "stage_id": component.get("id").cloned().unwrap_or(Value::Null),
                "stage_type": component.get("stage_type").cloned().unwrap_or(Value::Null),
                "parallel": is_parallel,
                "selected": selected,
                "session_id": if selected { root_session_id.clone().map(Value::String).unwrap_or(Value::Null) } else { Value::Null },
            })
        })
        .collect::<Vec<_>>();
    let run = json!({
        "run_id": stable_id("hayrun", &json!({"pipeline_id": pipeline_id, "profile": profile})),
        "pipeline_id": pipeline_id,
        "profile": profile,
        "visited": visited,
        "degraded": degraded,
        "reason_code": if degraded { "parallel_pipeline_profile_limited" } else { "pipeline_ok" },
        "root_session_id": root_session_id,
        "executed_at": now_iso(),
    });
    let run_id = run
        .get("run_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "pipeline_runs").insert(run_id, run.clone());
    Ok(json!({
        "ok": true,
        "run": run,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-012.1", haystack_claim("V6-WORKFLOW-012.1")),
    }))
}

fn run_agent_toolset(
    root: &Path,
    argv: &[String],
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let name = clean_token(
        payload.get("name").and_then(Value::as_str),
        "haystack-agent",
    );
    let goal = clean_text(payload.get("goal").and_then(Value::as_str), 240);
    if goal.is_empty() {
        return Err("haystack_agent_goal_required".to_string());
    }
    let tools = payload
        .get("tools")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if tools.is_empty() {
        return Err("haystack_agent_tools_required".to_string());
    }
    let terms = query_terms(&goal);
    let search_limit = parse_u64_value(payload.get("search_limit"), 3, 1, 12) as usize;
    let mut ranked = tools
        .into_iter()
        .map(|tool| {
            let hay = format!(
                "{} {} {}",
                clean_text(tool.get("name").and_then(Value::as_str), 120),
                clean_text(tool.get("description").and_then(Value::as_str), 240),
                tool.get("tags").cloned().unwrap_or_else(|| json!([]))
            )
            .to_ascii_lowercase();
            let score = terms
                .iter()
                .filter(|term| hay.contains(term.as_str()))
                .count() as i64;
            (score, tool)
        })
        .collect::<Vec<_>>();
    ranked.sort_by(|a, b| b.0.cmp(&a.0));
    let selected_tools = ranked
        .iter()
        .filter(|(score, _)| *score > 0)
        .take(search_limit)
        .map(|(_, tool)| tool.clone())
        .collect::<Vec<_>>();
    let selected_tools = if selected_tools.is_empty() {
        vec![ranked
            .first()
            .map(|(_, tool)| tool.clone())
            .ok_or_else(|| "haystack_agent_tool_selection_failed".to_string())?]
    } else {
        selected_tools
    };
    let swarm_state_path = swarm_state_path(root, argv, payload);
    let session_id = ensure_session_for_task(
        root,
        &swarm_state_path,
        &format!("haystack:agent:{name}:{goal}"),
        &name,
        Some("tool-agent"),
        None,
        parse_u64_value(payload.get("budget"), 640, 96, 12288),
    )?;
    let run = json!({
        "agent_run_id": stable_id("hayagent", &json!({"name": name, "goal": goal})),
        "name": name,
        "goal": goal,
        "session_id": session_id,
        "search_terms": terms,
        "selected_tools": selected_tools,
        "discarded_tool_count": ranked.len().saturating_sub(selected_tools.len()),
        "executed_at": now_iso(),
    });
    let run_id = run
        .get("agent_run_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "agent_runs").insert(run_id, run.clone());
    Ok(json!({
        "ok": true,
        "agent": run,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-012.2", haystack_claim("V6-WORKFLOW-012.2")),
    }))
}

fn register_template(state: &mut Value, payload: &Map<String, Value>) -> Result<Value, String> {
    let name = clean_token(
        payload.get("name").and_then(Value::as_str),
        "haystack-template",
    );
    let template = clean_text(payload.get("template").and_then(Value::as_str), 4000);
    if template.is_empty() {
        return Err("haystack_template_body_required".to_string());
    }
    let record = json!({
        "template_id": stable_id("haytpl", &json!({"name": name, "template": template})),
        "name": name,
        "template": template,
        "asset_kind": clean_token(payload.get("asset_kind").and_then(Value::as_str), "prompt"),
        "version": 1,
        "registered_at": now_iso(),
    });
    let template_id = record
        .get("template_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "templates").insert(template_id, record.clone());
    Ok(json!({
        "ok": true,
        "template": record,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-012.3", haystack_claim("V6-WORKFLOW-012.3")),
    }))
}

fn render_template(state: &mut Value, payload: &Map<String, Value>) -> Result<Value, String> {
    let template_id = clean_token(payload.get("template_id").and_then(Value::as_str), "");
    if template_id.is_empty() {
        return Err("haystack_render_template_id_required".to_string());
    }
    let template = state
        .get("templates")
        .and_then(Value::as_object)
        .and_then(|rows| rows.get(&template_id))
        .cloned()
        .ok_or_else(|| format!("unknown_haystack_template:{template_id}"))?;
    let variables = payload
        .get("variables")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let body = render_template_text(
        template
            .get("template")
            .and_then(Value::as_str)
            .unwrap_or_default(),
        &variables,
    );
    let render = json!({
        "render_id": stable_id("hayrender", &json!({"template_id": template_id, "variables": variables})),
        "template_id": template_id,
        "source_template_id": template.get("template_id").cloned().unwrap_or(Value::Null),
        "output": body,
        "rendered_at": now_iso(),
    });
    let render_id = render
        .get("render_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "template_renders").insert(render_id, render.clone());
    Ok(json!({
        "ok": true,
        "render": render,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-012.3", haystack_claim("V6-WORKFLOW-012.3")),
    }))
}

fn register_document_store(
    root: &Path,
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let name = clean_token(
        payload.get("name").and_then(Value::as_str),
        "haystack-store",
    );
    let bridge_path = normalize_bridge_path(
        root,
        payload
            .get("bridge_path")
            .and_then(Value::as_str)
            .unwrap_or("adapters/protocol/haystack_connector_bridge.ts"),
    )?;
    let documents = payload
        .get("documents")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if documents.is_empty() {
        return Err("haystack_document_store_documents_required".to_string());
    }
    let retrieval_modes = payload
        .get("retrieval_modes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_else(|| vec![json!("hybrid"), json!("vector"), json!("ranker")]);
    let supported_profiles = payload
        .get("supported_profiles")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_else(|| vec![json!("rich"), json!("pure")]);
    let store = json!({
        "store_id": stable_id("haystore", &json!({"name": name, "bridge_path": bridge_path})),
        "name": name,
        "bridge_path": bridge_path,
        "documents": documents,
        "retrieval_modes": retrieval_modes,
        "supported_profiles": supported_profiles,
        "context_budget": parse_u64_value(payload.get("context_budget"), 512, 64, 4096),
        "registered_at": now_iso(),
    });
    let store_id = store
        .get("store_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "document_stores").insert(store_id, store.clone());
    Ok(json!({
        "ok": true,
        "document_store": store,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-012.4", haystack_claim("V6-WORKFLOW-012.4")),
    }))
}

