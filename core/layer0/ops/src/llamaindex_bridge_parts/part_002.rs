fn run_conditional_workflow(
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let workflow_name = clean_token(
        payload.get("name").and_then(Value::as_str),
        "llamaindex-conditional",
    );
    let steps = payload
        .get("steps")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if steps.is_empty() {
        return Err("llamaindex_conditional_workflow_steps_required".to_string());
    }
    let context = payload
        .get("context")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let mut current = steps.first().cloned().unwrap_or(Value::Null);
    let mut visited = Vec::new();
    for _ in 0..steps.len().saturating_add(2) {
        let step = current
            .as_object()
            .cloned()
            .ok_or_else(|| "llamaindex_conditional_step_object_required".to_string())?;
        let step_id = clean_token(step.get("id").and_then(Value::as_str), "step");
        let matched = step
            .get("condition")
            .map(|row| condition_matches(row, &context))
            .unwrap_or(true);
        let next_id = if matched {
            step.get("next").and_then(Value::as_str)
        } else {
            step.get("else").and_then(Value::as_str)
        };
        visited.push(json!({
            "step_id": step_id,
            "matched": matched,
            "checkpoint_key": step.get("checkpoint_key").cloned().unwrap_or_else(|| json!(Value::Null)),
        }));
        let Some(next_id) = next_id else {
            break;
        };
        if next_id.is_empty() {
            break;
        }
        current = steps
            .iter()
            .find(|row| row.get("id").and_then(Value::as_str) == Some(next_id))
            .cloned()
            .ok_or_else(|| format!("llamaindex_conditional_workflow_unknown_step:{next_id}"))?;
    }
    let record = json!({
        "workflow_id": stable_id("llxroute", &json!({"name": workflow_name, "context": context})),
        "name": workflow_name,
        "visited": visited,
        "context": context,
        "executed_at": now_iso(),
    });
    let workflow_id = record
        .get("workflow_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "conditional_workflows").insert(workflow_id, record.clone());
    Ok(json!({
        "ok": true,
        "workflow": record,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-009.5", semantic_claim("V6-WORKFLOW-009.5")),
    }))
}

fn emit_trace(
    root: &Path,
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let trace_id = clean_token(
        payload.get("trace_id").and_then(Value::as_str),
        "llamaindex-trace",
    );
    let stage = clean_token(payload.get("stage").and_then(Value::as_str), "query");
    let message = clean_text(payload.get("message").and_then(Value::as_str), 160);
    emit_native_trace(root, &trace_id, &stage, &message)?;
    let record = json!({
        "trace_id": trace_id,
        "stage": stage,
        "message": message,
        "data": payload.get("data").cloned().unwrap_or_else(|| json!({})),
        "recorded_at": now_iso(),
    });
    as_array_mut(state, "traces").push(record.clone());
    Ok(json!({
        "ok": true,
        "trace": record,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-009.6", semantic_claim("V6-WORKFLOW-009.6")),
    }))
}

fn register_connector(
    root: &Path,
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let name = clean_token(
        payload.get("name").and_then(Value::as_str),
        "llamaindex-connector",
    );
    let bridge_path = normalize_bridge_path(
        root,
        payload
            .get("bridge_path")
            .and_then(Value::as_str)
            .unwrap_or(""),
    )?;
    let capabilities = payload
        .get("capabilities")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let documents = payload
        .get("documents")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let supported_profiles = payload
        .get("supported_profiles")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_else(|| vec![json!("rich"), json!("pure")]);
    let connector = json!({
        "connector_id": stable_id("llxconn", &json!({"name": name, "bridge_path": bridge_path})),
        "name": name,
        "bridge_path": bridge_path,
        "capabilities": capabilities,
        "supported_profiles": supported_profiles,
        "documents": documents,
        "registered_at": now_iso(),
    });
    let connector_id = connector
        .get("connector_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "connectors").insert(connector_id, connector.clone());
    Ok(json!({
        "ok": true,
        "connector": connector,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-009.7", semantic_claim("V6-WORKFLOW-009.7")),
    }))
}

fn connector_query(state: &Value, payload: &Map<String, Value>) -> Result<Value, String> {
    let connector_id = clean_token(payload.get("connector_id").and_then(Value::as_str), "");
    if connector_id.is_empty() {
        return Err("llamaindex_connector_id_required".to_string());
    }
    let query = clean_text(payload.get("query").and_then(Value::as_str), 240);
    if query.is_empty() {
        return Err("llamaindex_connector_query_required".to_string());
    }
    let profile = clean_token(payload.get("profile").and_then(Value::as_str), "rich");
    let connector = state
        .get("connectors")
        .and_then(Value::as_object)
        .and_then(|rows| rows.get(&connector_id))
        .cloned()
        .ok_or_else(|| format!("unknown_llamaindex_connector:{connector_id}"))?;
    let supported = connector
        .get("supported_profiles")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if !supported
        .iter()
        .filter_map(Value::as_str)
        .any(|row| row == profile)
    {
        return Err(format!(
            "llamaindex_connector_profile_unsupported:{profile}"
        ));
    }
    let terms = query_terms(&query);
    let mut ranked = connector
        .get("documents")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|doc| (retrieval_score(&doc, &terms, "hybrid"), doc))
        .filter(|(score, _)| *score > 0)
        .collect::<Vec<_>>();
    ranked.sort_by(|a, b| b.0.cmp(&a.0));
    Ok(json!({
        "ok": true,
        "connector_id": connector_id,
        "query": query,
        "profile": profile,
        "results": ranked.into_iter().take(3).map(|(score, doc)| json!({
            "score": score,
            "text": doc.get("text").cloned().unwrap_or(Value::Null),
            "metadata": doc.get("metadata").cloned().unwrap_or(Value::Null),
        })).collect::<Vec<_>>(),
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-009.7", semantic_claim("V6-WORKFLOW-009.7")),
    }))
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    if argv.is_empty() || matches!(argv[0].as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }
    let command = argv[0].as_str();
    let payload = match payload_json(&argv[1..]) {
        Ok(payload) => payload,
        Err(err) => {
            print_json_line(&cli_error("llamaindex_bridge_error", &err));
            return 1;
        }
    };
    let input = payload_obj(&payload);
    let state_path = state_path(root, argv, input);
    let history_path = history_path(root, argv, input);
    let mut state = load_state(&state_path);

    let result = match command {
        "status" => Ok(json!({
            "ok": true,
            "state_path": rel(root, &state_path),
            "history_path": rel(root, &history_path),
            "indexes": as_object_mut(&mut state, "indexes").len(),
            "agent_workflows": as_object_mut(&mut state, "agent_workflows").len(),
            "ingestions": as_object_mut(&mut state, "ingestions").len(),
            "memory_store": as_object_mut(&mut state, "memory_store").len(),
            "evaluations": as_object_mut(&mut state, "evaluations").len(),
            "conditional_workflows": as_object_mut(&mut state, "conditional_workflows").len(),
            "traces": as_array_mut(&mut state, "traces").len(),
            "connectors": as_object_mut(&mut state, "connectors").len(),
            "last_receipt": state.get("last_receipt").cloned().unwrap_or(Value::Null),
        })),
        "register-index" => register_index(&mut state, input),
        "query" => query_index(&state, input),
        "run-agent-workflow" => run_agent_workflow(root, argv, &mut state, input),
        "ingest-multimodal" => ingest_multimodal(root, &mut state, input),
        "record-memory-eval" => record_memory_eval(root, &mut state, input),
        "run-conditional-workflow" => run_conditional_workflow(&mut state, input),
        "emit-trace" => emit_trace(root, &mut state, input),
        "register-connector" => register_connector(root, &mut state, input),
        "connector-query" => connector_query(&state, input),
        _ => Err(format!("unknown_llamaindex_bridge_command:{command}")),
    };

    match result {
        Ok(payload) => {
            let receipt = cli_receipt(
                &format!("llamaindex_bridge_{}", command.replace('-', "_")),
                payload,
            );
            state["last_receipt"] = receipt.clone();
            if let Err(err) = save_state(&state_path, &state)
                .and_then(|_| append_history(&history_path, &receipt))
            {
                print_json_line(&cli_error("llamaindex_bridge_error", &err));
                return 1;
            }
            print_json_line(&receipt);
            0
        }
        Err(err) => {
            print_json_line(&cli_error("llamaindex_bridge_error", &err));
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_index_ranks_matching_documents() {
        let mut state = default_state();
        let payload = json!({
            "name": "ops-index",
            "documents": [
                {"text": "llamaindex query engine supports hybrid retrieval"},
                {"text": "semantic kernel planner supports function routing"}
            ]
        });
        let _ = register_index(&mut state, payload.as_object().unwrap()).expect("register");
        let index_id = state["indexes"]
            .as_object()
            .unwrap()
            .keys()
            .next()
            .unwrap()
            .to_string();
        let query = json!({"index_id": index_id, "query": "hybrid retrieval", "mode": "hybrid"});
        let out = query_index(&state, query.as_object().unwrap()).expect("query");
        assert!(out["results"]
            .as_array()
            .map(|rows| !rows.is_empty())
            .unwrap_or(false));
    }

    #[test]
    fn conditional_workflow_routes_deterministically() {
        let mut state = default_state();
        let payload = json!({
            "name": "router",
            "context": {"intent": "support"},
            "steps": [
                {"id": "start", "condition": {"field": "intent", "equals": "support"}, "next": "support-lane", "else": "generic"},
                {"id": "support-lane"},
                {"id": "generic"}
            ]
        });
        let out =
            run_conditional_workflow(&mut state, payload.as_object().unwrap()).expect("workflow");
        assert_eq!(
            out["workflow"]["visited"][0]["matched"].as_bool(),
            Some(true)
        );
    }
}

