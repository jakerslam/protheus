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
            | "vector-store"
            | "retriever"
            | "loader"
            | "tool"
            | "model"
            | "prompt"
            | "pgvector"
            | "qdrant"
            | "weaviate"
            | "elasticsearch"
            | "opensearch"
            | "s3"
            | "http"
    )
}

fn allowed_middleware_hook(kind: &str) -> bool {
    matches!(
        kind,
        "before_model"
            | "after_model"
            | "before_tool"
            | "after_tool"
            | "before_chain"
            | "after_chain"
    )
}

fn collect_chain_middleware(state: &Value, chain_id: &str) -> Vec<Value> {
    state
        .get("middleware_hooks")
        .and_then(Value::as_object)
        .map(|rows| {
            rows.values()
                .filter(|row| {
                    row.get("chain_id")
                        .and_then(Value::as_str)
                        .map_or(true, |scope| scope == chain_id)
                })
                .cloned()
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn register_chain(state: &mut Value, payload: &Map<String, Value>) -> Result<Value, String> {
    let name = clean_token(
        payload.get("name").and_then(Value::as_str),
        "langchain-chain",
    );
    let runnables = payload
        .get("runnables")
        .or_else(|| payload.get("components"))
        .or_else(|| payload.get("stages"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if runnables.is_empty() {
        return Err("langchain_chain_runnables_required".to_string());
    }
    let normalized = runnables
        .into_iter()
        .map(|component| {
            let obj = component.as_object().cloned().unwrap_or_default();
            json!({
                "id": clean_token(obj.get("id").and_then(Value::as_str), "runnable"),
                "runnable_type": clean_token(obj.get("runnable_type").and_then(Value::as_str).or_else(|| obj.get("stage_type").and_then(Value::as_str)).or_else(|| obj.get("type").and_then(Value::as_str)), "llm"),
                "input_type": clean_token(obj.get("input_type").and_then(Value::as_str), "text"),
                "output_type": clean_token(obj.get("output_type").and_then(Value::as_str), "text"),
                "parallel": parse_bool_value(obj.get("parallel"), false),
                "spawn": parse_bool_value(obj.get("spawn"), false),
                "budget": parse_u64_value(obj.get("budget"), 192, 32, 4096),
            })
        })
        .collect::<Vec<_>>();
    let chain = json!({
        "chain_id": stable_id("langchain", &json!({"name": name, "runnables": normalized.len()})),
        "name": name,
        "runnables": normalized,
        "registered_at": now_iso(),
    });
    let chain_id = chain
        .get("chain_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "chains").insert(chain_id, chain.clone());
    Ok(json!({
        "ok": true,
        "chain": chain,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-014.1", langchain_claim("V6-WORKFLOW-014.1")),
    }))
}

fn register_middleware(state: &mut Value, payload: &Map<String, Value>) -> Result<Value, String> {
    let name = clean_token(
        payload.get("name").and_then(Value::as_str),
        "langchain-middleware",
    );
    let hook = clean_token(payload.get("hook").and_then(Value::as_str), "");
    if !allowed_middleware_hook(&hook) {
        return Err(format!("langchain_middleware_hook_unsupported:{hook}"));
    }
    let chain_id = clean_token(payload.get("chain_id").and_then(Value::as_str), "");
    if !chain_id.is_empty()
        && !state
            .get("chains")
            .and_then(Value::as_object)
            .is_some_and(|rows| rows.contains_key(&chain_id))
    {
        return Err(format!("unknown_langchain_chain:{chain_id}"));
    }
    let middleware = json!({
        "middleware_id": stable_id("langmw", &json!({"name": name, "hook": hook, "chain_id": chain_id})),
        "name": name,
        "hook": hook,
        "chain_id": if chain_id.is_empty() { Value::Null } else { Value::String(chain_id) },
        "action": clean_text(payload.get("action").and_then(Value::as_str), 200),
        "fail_closed": parse_bool_value(payload.get("fail_closed"), true),
        "registered_at": now_iso(),
    });
    let middleware_id = middleware
        .get("middleware_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "middleware_hooks").insert(middleware_id, middleware.clone());
    Ok(json!({
        "ok": true,
        "middleware": middleware,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-014.9", langchain_claim("V6-WORKFLOW-014.9")),
    }))
}

fn execute_chain(
    root: &Path,
    argv: &[String],
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let chain_id = clean_token(payload.get("chain_id").and_then(Value::as_str), "");
    if chain_id.is_empty() {
        return Err("langchain_chain_id_required".to_string());
    }
    let profile = clean_token(payload.get("profile").and_then(Value::as_str), "rich");
    let chain = state
        .get("chains")
        .and_then(Value::as_object)
        .and_then(|rows| rows.get(&chain_id))
        .cloned()
        .ok_or_else(|| format!("unknown_langchain_chain:{chain_id}"))?;
    let runnables = chain
        .get("runnables")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let applied_middleware = collect_chain_middleware(state, &chain_id);
    let middleware_count = applied_middleware.len();
    let parallel_count = runnables
        .iter()
        .filter(|row| row.get("parallel").and_then(Value::as_bool) == Some(true))
        .count();
    let degraded = matches!(profile.as_str(), "pure" | "tiny-max") && parallel_count > 1;
    let swarm_state_path = swarm_state_path(root, argv, payload);
    let root_session_id = if runnables.iter().any(|row| {
        row.get("spawn").and_then(Value::as_bool) == Some(true)
            || matches!(
                row.get("runnable_type").and_then(Value::as_str),
                Some("llm" | "tool" | "agent" | "retriever" | "ranker")
            )
    }) {
        Some(ensure_session_for_task(
            root,
            &swarm_state_path,
            &format!(
                "langchain:chain:{}",
                clean_token(chain.get("name").and_then(Value::as_str), "chain")
            ),
            "langchain-chain",
            Some("chain"),
            None,
            parse_u64_value(payload.get("budget"), 896, 96, 12288),
        )?)
    } else {
        None
    };
    let mut selected_parallel = 0usize;
    let visited = runnables
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
                "runnable_id": component.get("id").cloned().unwrap_or(Value::Null),
                "runnable_type": component.get("runnable_type").cloned().unwrap_or(Value::Null),
                "parallel": is_parallel,
                "selected": selected,
                "session_id": if selected { root_session_id.clone().map(Value::String).unwrap_or(Value::Null) } else { Value::Null },
            })
        })
        .collect::<Vec<_>>();
    let run = json!({
        "run_id": stable_id("langrun", &json!({"chain_id": chain_id, "profile": profile})),
        "chain_id": chain_id,
        "profile": profile,
        "visited": visited,
        "applied_middleware": applied_middleware,
        "middleware_count": middleware_count,
        "degraded": degraded,
        "reason_code": if degraded { "parallel_chain_profile_limited" } else { "chain_ok" },
        "root_session_id": root_session_id,
        "executed_at": now_iso(),
    });
    let run_id = run
        .get("run_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "chain_runs").insert(run_id, run.clone());
    Ok(json!({
        "ok": true,
        "run": run,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-014.1", langchain_claim("V6-WORKFLOW-014.1")),
    }))
}

fn run_deep_agent(
    root: &Path,
    argv: &[String],
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let name = clean_token(
        payload.get("name").and_then(Value::as_str),
        "langchain-agent",
    );
    let instruction = clean_text(
        payload
            .get("instruction")
            .and_then(Value::as_str)
            .or_else(|| payload.get("goal").and_then(Value::as_str)),
        240,
    );
    if instruction.is_empty() {
        return Err("langchain_agent_instruction_required".to_string());
    }
    let tools = payload
        .get("tools")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if tools.is_empty() {
        return Err("langchain_agent_tools_required".to_string());
    }
    let terms = query_terms(&instruction);
    let requested_limit = parse_u64_value(payload.get("search_limit"), 3, 1, 12) as usize;
    let search_limit = if matches!(
        clean_token(payload.get("profile").and_then(Value::as_str), "rich").as_str(),
        "pure" | "tiny-max"
    ) {
        requested_limit.min(1)
    } else {
        requested_limit
    };
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
            .ok_or_else(|| "langchain_agent_tool_selection_failed".to_string())?]
    } else {
        selected_tools
    };
    let swarm_state_path = swarm_state_path(root, argv, payload);
    let session_id = ensure_session_for_task(
        root,
        &swarm_state_path,
        &format!("langchain:agent:{name}:{instruction}"),
        &name,
        Some("deep-agent"),
        None,
        parse_u64_value(payload.get("budget"), 640, 96, 12288),
    )?;
    let run = json!({
        "agent_run_id": stable_id("langagent", &json!({"name": name, "instruction": instruction})),
        "name": name,
        "instruction": instruction,
        "session_id": session_id,
        "search_terms": terms,
        "selected_tools": selected_tools,
        "discarded_tool_count": ranked.len().saturating_sub(selected_tools.len()),
        "degraded": requested_limit != search_limit,
        "reason_code": if requested_limit != search_limit { "profile_tool_fanout_limited" } else { "deep_agent_ok" },
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
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-014.2", langchain_claim("V6-WORKFLOW-014.2")),
    }))
}

fn register_memory_bridge(
    root: &Path,
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let name = clean_token(
        payload.get("name").and_then(Value::as_str),
        "langchain-memory",
    );
    let bridge_path = normalize_bridge_path(
        root,
        payload
            .get("bridge_path")
            .and_then(Value::as_str)
            .unwrap_or("adapters/protocol/langchain_connector_bridge.ts"),
    )?;
    let documents = payload
        .get("documents")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if documents.is_empty() {
        return Err("langchain_memory_bridge_documents_required".to_string());
    }
    let retrieval_modes = payload
        .get("retrieval_modes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_else(|| vec![json!("similarity"), json!("mmr"), json!("hybrid")]);
    let supported_profiles = payload
        .get("supported_profiles")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_else(|| vec![json!("rich"), json!("pure")]);
    let store = json!({
        "memory_id": stable_id("langmem", &json!({"name": name, "bridge_path": bridge_path})),
        "name": name,
        "bridge_path": bridge_path,
        "documents": documents,
        "retrieval_modes": retrieval_modes,
        "supported_profiles": supported_profiles,
        "context_budget": parse_u64_value(payload.get("context_budget"), 512, 64, 4096),
        "registered_at": now_iso(),
    });
    let store_id = store
        .get("memory_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "memory_bridges").insert(store_id, store.clone());
    Ok(json!({
        "ok": true,
        "memory_bridge": store,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-014.3", langchain_claim("V6-WORKFLOW-014.3")),
    }))
}
