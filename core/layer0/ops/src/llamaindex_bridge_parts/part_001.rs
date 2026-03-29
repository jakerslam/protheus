fn query_index(state: &Value, payload: &Map<String, Value>) -> Result<Value, String> {
    let index_id = clean_token(payload.get("index_id").and_then(Value::as_str), "");
    if index_id.is_empty() {
        return Err("llamaindex_query_index_id_required".to_string());
    }
    let query = clean_text(payload.get("query").and_then(Value::as_str), 240);
    if query.is_empty() {
        return Err("llamaindex_query_text_required".to_string());
    }
    let mode = clean_token(payload.get("mode").and_then(Value::as_str), "hybrid");
    let top_k = parse_u64_value(payload.get("top_k"), 3, 1, 12) as usize;
    let profile = clean_token(payload.get("profile").and_then(Value::as_str), "rich");
    let index = state
        .get("indexes")
        .and_then(Value::as_object)
        .and_then(|rows| rows.get(&index_id))
        .cloned()
        .ok_or_else(|| format!("unknown_llamaindex_index:{index_id}"))?;
    let supported = index
        .get("retrieval_modes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let supported_modes = supported
        .iter()
        .filter_map(Value::as_str)
        .collect::<BTreeSet<_>>();
    if !supported_modes.contains(mode.as_str()) {
        return Err(format!("llamaindex_query_mode_unsupported:{mode}"));
    }
    if (profile == "pure" || profile == "tiny-max") && mode == "graph" {
        return Ok(json!({
            "ok": true,
            "index_id": index_id,
            "profile": profile,
            "mode": mode,
            "degraded": true,
            "reason_code": "graph_retrieval_requires_rich_profile",
            "results": [],
            "claim_evidence": default_claim_evidence("V6-WORKFLOW-009.1", semantic_claim("V6-WORKFLOW-009.1")),
        }));
    }
    let terms = query_terms(&query);
    let mut ranked = index
        .get("documents")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|doc| {
            let score = retrieval_score(&doc, &terms, &mode);
            (score, doc)
        })
        .filter(|(score, _)| *score > 0)
        .collect::<Vec<_>>();
    ranked.sort_by(|a, b| b.0.cmp(&a.0));
    let results = ranked
        .into_iter()
        .take(top_k)
        .map(|(score, doc)| {
            json!({
                "score": score,
                "text": doc.get("text").cloned().unwrap_or(Value::Null),
                "metadata": doc.get("metadata").cloned().unwrap_or(Value::Null),
            })
        })
        .collect::<Vec<_>>();
    Ok(json!({
        "ok": true,
        "index_id": index_id,
        "query": query,
        "mode": mode,
        "profile": profile,
        "query_engine": index.get("query_engine").cloned().unwrap_or(Value::Null),
        "results": results,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-009.1", semantic_claim("V6-WORKFLOW-009.1")),
    }))
}

fn run_agent_workflow(
    root: &Path,
    argv: &[String],
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let workflow_name = clean_token(
        payload.get("name").and_then(Value::as_str),
        "llamaindex-agent-workflow",
    );
    let query = clean_text(payload.get("query").and_then(Value::as_str), 200);
    if query.is_empty() {
        return Err("llamaindex_agent_workflow_query_required".to_string());
    }
    let swarm_state_path = swarm_state_path(root, argv, payload);
    let tools = payload
        .get("tools")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let handoffs = payload
        .get("handoffs")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let child_budget_sum = handoffs
        .iter()
        .filter_map(|row| row.get("budget").and_then(Value::as_u64))
        .sum::<u64>();
    let agent_budget = parse_u64_value(payload.get("budget"), 640, 64, 4096);
    let total_budget = agent_budget
        .saturating_add(child_budget_sum)
        .saturating_add(1024)
        .clamp(1024, 16384);
    let agent_task = format!("llamaindex:{}:{}", workflow_name, query);
    let agent_label = clean_token(
        payload.get("agent_label").and_then(Value::as_str),
        "llamaindex-agent",
    );
    let spawn_exit = crate::swarm_runtime::run(
        root,
        &[
            "spawn".to_string(),
            format!("--task={agent_task}"),
            format!("--max-tokens={total_budget}"),
            format!("--agent-label={agent_label}"),
            format!("--state-path={}", swarm_state_path.display()),
        ],
    );
    if spawn_exit != 0 {
        return Err("llamaindex_agent_workflow_spawn_failed".to_string());
    }
    let swarm_state = read_swarm_state(&swarm_state_path);
    let primary_session_id = find_swarm_session_id_by_task(&swarm_state, &agent_task)
        .ok_or_else(|| "llamaindex_agent_workflow_primary_session_missing".to_string())?;

    let mut tool_calls = Vec::new();
    for tool in tools {
        let tool_obj = tool
            .as_object()
            .ok_or_else(|| "llamaindex_tool_object_required".to_string())?;
        let tool_name = clean_token(tool_obj.get("name").and_then(Value::as_str), "tool");
        let bridge_path = normalize_bridge_path(
            root,
            tool_obj
                .get("bridge_path")
                .and_then(Value::as_str)
                .unwrap_or(""),
        )?;
        let entrypoint = clean_token(tool_obj.get("entrypoint").and_then(Value::as_str), "run");
        tool_calls.push(json!({
            "tool_name": tool_name,
            "bridge_path": bridge_path,
            "entrypoint": entrypoint,
            "arguments": tool_obj.get("args").cloned().unwrap_or_else(|| json!({})),
            "mode": "governed_receipted_invocation",
        }));
    }

    let mut handoff_rows = Vec::new();
    let mut session_ids = BTreeMap::new();
    session_ids.insert(agent_label.clone(), primary_session_id.clone());
    for handoff in handoffs {
        let handoff_obj = handoff
            .as_object()
            .ok_or_else(|| "llamaindex_handoff_object_required".to_string())?;
        let label = clean_token(
            handoff_obj.get("label").and_then(Value::as_str),
            "handoff-agent",
        );
        let role = clean_token(
            handoff_obj.get("role").and_then(Value::as_str),
            "specialist",
        );
        let task = format!(
            "llamaindex:{}:{}:{}",
            workflow_name,
            label,
            clean_text(handoff_obj.get("task").and_then(Value::as_str), 120)
        );
        let budget = parse_u64_value(handoff_obj.get("budget"), 256, 32, 4096);
        let spawn_child_exit = crate::swarm_runtime::run(
            root,
            &[
                "spawn".to_string(),
                format!("--task={task}"),
                format!("--session-id={primary_session_id}"),
                format!("--max-tokens={budget}"),
                format!("--agent-label={label}"),
                format!("--role={role}"),
                format!("--state-path={}", swarm_state_path.display()),
            ],
        );
        if spawn_child_exit != 0 {
            return Err(format!("llamaindex_agent_handoff_spawn_failed:{label}"));
        }
        let updated = read_swarm_state(&swarm_state_path);
        let child_session_id = find_swarm_session_id_by_task(&updated, &task)
            .ok_or_else(|| format!("llamaindex_agent_handoff_session_missing:{label}"))?;
        let reason = clean_text(handoff_obj.get("reason").and_then(Value::as_str), 120);
        let handoff_exit = crate::swarm_runtime::run(
            root,
            &[
                "sessions".to_string(),
                "handoff".to_string(),
                format!("--session-id={primary_session_id}"),
                format!("--target-session-id={child_session_id}"),
                format!(
                    "--reason={}",
                    if reason.is_empty() {
                        "llamaindex_handoff"
                    } else {
                        &reason
                    }
                ),
                format!(
                    "--importance={:.2}",
                    parse_f64_value(handoff_obj.get("importance"), 0.75, 0.0, 1.0)
                ),
                format!("--state-path={}", swarm_state_path.display()),
            ],
        );
        if handoff_exit != 0 {
            return Err(format!("llamaindex_agent_handoff_failed:{label}"));
        }
        session_ids.insert(label.clone(), child_session_id.clone());
        handoff_rows.push(json!({
            "label": label,
            "role": role,
            "session_id": child_session_id,
            "reason": reason,
        }));
    }

    let workflow = json!({
        "workflow_id": stable_id("llxwf", &json!({"name": workflow_name, "query": query})),
        "name": workflow_name,
        "query": query,
        "primary_session_id": primary_session_id,
        "session_ids": session_ids,
        "tool_calls": tool_calls,
        "handoffs": handoff_rows,
        "swarm_state_path": rel(root, &swarm_state_path),
        "executed_at": now_iso(),
    });
    let workflow_id = workflow
        .get("workflow_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "agent_workflows").insert(workflow_id, workflow.clone());
    Ok(json!({
        "ok": true,
        "workflow": workflow,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-009.2", semantic_claim("V6-WORKFLOW-009.2")),
    }))
}

fn ingest_multimodal(
    root: &Path,
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let loader_name = clean_token(
        payload.get("loader_name").and_then(Value::as_str),
        "llamaindex-loader",
    );
    let modality = clean_token(payload.get("modality").and_then(Value::as_str), "text");
    let profile = clean_token(payload.get("profile").and_then(Value::as_str), "rich");
    let assets = payload
        .get("assets")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if assets.is_empty() {
        return Err("llamaindex_ingestion_assets_required".to_string());
    }
    let connector = payload
        .get("bridge_path")
        .and_then(Value::as_str)
        .map(|raw| normalize_bridge_path(root, raw))
        .transpose()?;
    let degraded = matches!(profile.as_str(), "pure" | "tiny-max")
        && matches!(modality.as_str(), "audio" | "video" | "image");
    let reason_code = if degraded {
        "profile_multimodal_degraded"
    } else {
        "ingestion_ok"
    };
    let record = json!({
        "ingestion_id": stable_id("llxingest", &json!({"loader": loader_name, "modality": modality})),
        "loader_name": loader_name,
        "modality": modality,
        "profile": profile,
        "bridge_path": connector,
        "asset_count": assets.len(),
        "degraded": degraded,
        "reason_code": reason_code,
        "recorded_at": now_iso(),
    });
    let ingestion_id = record
        .get("ingestion_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "ingestions").insert(ingestion_id, record.clone());
    Ok(json!({
        "ok": true,
        "ingestion": record,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-009.3", semantic_claim("V6-WORKFLOW-009.3")),
    }))
}

fn emit_native_trace(
    root: &Path,
    trace_id: &str,
    intent: &str,
    message: &str,
) -> Result<(), String> {
    let enable_exit = crate::observability_plane::run(
        root,
        &[
            "acp-provenance".to_string(),
            "--op=enable".to_string(),
            "--enabled=1".to_string(),
            "--visibility-mode=meta".to_string(),
            "--strict=1".to_string(),
        ],
    );
    if enable_exit != 0 {
        return Err("llamaindex_observability_enable_failed".to_string());
    }
    let exit = crate::observability_plane::run(
        root,
        &[
            "acp-provenance".to_string(),
            "--op=trace".to_string(),
            "--source-agent=llamaindex-bridge".to_string(),
            format!("--target-agent={}", clean_token(Some(intent), "workflow")),
            format!("--intent={}", clean_text(Some(intent), 80)),
            format!("--message={}", clean_text(Some(message), 160)),
            format!("--trace-id={trace_id}"),
            "--visibility-mode=meta".to_string(),
            "--strict=1".to_string(),
        ],
    );
    if exit != 0 {
        return Err("llamaindex_observability_trace_failed".to_string());
    }
    Ok(())
}

fn record_memory_eval(
    root: &Path,
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let memory_key = clean_token(
        payload.get("memory_key").and_then(Value::as_str),
        "llamaindex-memory",
    );
    let entries = payload
        .get("entries")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if entries.is_empty() {
        return Err("llamaindex_memory_entries_required".to_string());
    }
    let expected = payload
        .get("expected_hits")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let actual = payload
        .get("actual_hits")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let expected_set = expected
        .iter()
        .filter_map(Value::as_str)
        .collect::<BTreeSet<_>>();
    let actual_set = actual
        .iter()
        .filter_map(Value::as_str)
        .collect::<BTreeSet<_>>();
    let overlap = expected_set.intersection(&actual_set).count() as f64;
    let recall = if expected_set.is_empty() {
        1.0
    } else {
        overlap / (expected_set.len() as f64)
    };
    let eval = json!({
        "evaluation_id": stable_id("llxeval", &json!({"memory_key": memory_key, "expected": expected_set.len(), "actual": actual_set.len()})),
        "memory_key": memory_key,
        "entry_count": entries.len(),
        "expected_hits": expected,
        "actual_hits": actual,
        "recall": recall,
        "evaluated_at": now_iso(),
    });
    let eval_id = eval
        .get("evaluation_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "memory_store").insert(
        memory_key.clone(),
        json!({
            "entries": entries,
            "updated_at": now_iso(),
        }),
    );
    as_object_mut(state, "evaluations").insert(eval_id, eval.clone());
    emit_native_trace(
        root,
        eval.get("evaluation_id")
            .and_then(Value::as_str)
            .unwrap_or("llamaindex-eval"),
        "llamaindex_eval",
        &format!("memory_key={memory_key} recall={recall:.2}"),
    )?;
    Ok(json!({
        "ok": true,
        "evaluation": eval,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-009.4", semantic_claim("V6-WORKFLOW-009.4")),
    }))
}

fn condition_matches(condition: &Value, context: &Map<String, Value>) -> bool {
    let field = condition
        .get("field")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let equals = condition.get("equals");
    if field.is_empty() {
        return false;
    }
    context.get(field) == equals
}

