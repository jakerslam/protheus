fn value_array_has_text(rows: &[Value], wanted: &str) -> bool {
    rows.iter()
        .filter_map(Value::as_str)
        .any(|row| row == wanted)
}
fn recall_memory(state: &mut Value, payload: &Map<String, Value>) -> Result<Value, String> {
    let store_id = clean_token(payload.get("memory_id").and_then(Value::as_str).or_else(|| payload.get("store_id").and_then(Value::as_str)), "");
    if store_id.is_empty() {
        return Err("workflow_chain_memory_id_required".to_string());
    }
    let query = clean_text(payload.get("query").and_then(Value::as_str), 240);
    if query.is_empty() {
        return Err("workflow_chain_query_required".to_string());
    }
    let mode = clean_token(payload.get("mode").and_then(Value::as_str), "hybrid");
    let profile = clean_token(payload.get("profile").and_then(Value::as_str), "rich");
    let store = state
        .get("memory_bridges")
        .and_then(Value::as_object)
        .and_then(|rows| rows.get(&store_id))
        .cloned()
        .ok_or_else(|| format!("unknown_workflow_chain_memory_bridge:{store_id}"))?;
    let supported_profiles = store.get("supported_profiles").and_then(Value::as_array).cloned().unwrap_or_default();
    if !value_array_has_text(&supported_profiles, &profile) {
        return Err(format!(
            "workflow_chain_memory_bridge_profile_unsupported:{profile}"
        ));
    }
    let supported_mode_rows = store.get("retrieval_modes").and_then(Value::as_array).cloned().unwrap_or_default();
    let supported_modes = supported_mode_rows
        .iter()
        .filter_map(Value::as_str)
        .collect::<BTreeSet<_>>();
    if !supported_modes.contains(mode.as_str()) {
        return Err(format!("workflow_chain_retrieval_mode_unsupported:{mode}"));
    }
    let requested_top_k = parse_u64_value(payload.get("top_k"), 3, 1, 12) as usize;
    let top_k = if matches!(profile.as_str(), "pure" | "tiny-max") {
        requested_top_k.min(2)
    } else {
        requested_top_k
    };
    let terms = query_terms(&query);
    let mut ranked = store
        .get("documents")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|doc| (retrieval_score(&doc, &terms, &mode), doc))
        .filter(|(score, _)| *score > 0)
        .collect::<Vec<_>>();
    ranked.sort_by(|a, b| b.0.cmp(&a.0));
    let context_budget = parse_u64_value(
        payload.get("context_budget"),
        store
            .get("context_budget")
            .and_then(Value::as_u64)
            .unwrap_or(512),
        64,
        4096,
    );
    let mut consumed = 0usize;
    let context_limit = (context_budget as usize) * 4;
    let mut results = Vec::new();
    for (score, doc) in ranked.into_iter().take(top_k) {
        let text = clean_text(doc.get("text").and_then(Value::as_str), 4000);
        if !results.is_empty() && consumed + text.len() > context_limit {
            break;
        }
        consumed += text.len();
        results.push(json!({
            "score": score,
            "text": text,
            "metadata": doc.get("metadata").cloned().unwrap_or(Value::Null),
        }));
    }
    let retrieval = json!({
        "recall_id": stable_id("langrecall", &json!({"memory_id": store_id, "query": query, "mode": mode})),
        "memory_id": store_id,
        "query": query,
        "mode": mode,
        "profile": profile,
        "degraded": top_k != requested_top_k,
        "reason_code": if top_k != requested_top_k { "profile_context_budget_limited" } else { "recall_ok" },
        "results": results,
        "context_budget": context_budget,
        "recorded_at": now_iso(),
    });
    let retrieval_id = retrieval
        .get("recall_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "memory_queries").insert(retrieval_id, retrieval.clone());
    Ok(json!({
        "ok": true,
        "recall": retrieval,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-014.3", workflow_chain_claim("V6-WORKFLOW-014.3")),
    }))
}
fn route_prompt(state: &mut Value, payload: &Map<String, Value>) -> Result<Value, String> {
    let name = clean_token(payload.get("name").and_then(Value::as_str), "workflow_chain-prompt");
    let template = clean_text(payload.get("template").and_then(Value::as_str).or_else(|| payload.get("prompt").and_then(Value::as_str)), 4000);
    if template.is_empty() {
        return Err("workflow_chain_prompt_template_required".to_string());
    }
    let variables = payload
        .get("variables")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let provider = clean_token(payload.get("provider").and_then(Value::as_str), "openai-compatible");
    let fallback_provider = clean_token(payload.get("fallback_provider").and_then(Value::as_str), &provider);
    let model = clean_token(payload.get("model").and_then(Value::as_str), "gpt-5-mini");
    let fallback_model = clean_token(payload.get("fallback_model").and_then(Value::as_str), &model);
    let profile = clean_token(payload.get("profile").and_then(Value::as_str), "rich");
    let supported_providers = payload.get("supported_providers").and_then(Value::as_array).cloned().unwrap_or_else(|| vec![json!("openai-compatible"), json!("local")]);
    if !value_array_has_text(&supported_providers, &provider) {
        return Err(format!("workflow_chain_provider_unsupported:{provider}"));
    }
    let local_capable = matches!(provider.as_str(), "local" | "openai-compatible");
    let constrained_profile = matches!(profile.as_str(), "pure" | "tiny-max");
    let selected_provider = if constrained_profile && !local_capable && supported_providers.iter().filter_map(Value::as_str).any(|row| row == fallback_provider) {
        fallback_provider.clone()
    } else {
        provider.clone()
    };
    let selected_model = if selected_provider != provider {
        fallback_model
    } else {
        model.clone()
    };
    let degraded = constrained_profile && selected_provider != provider;
    let rendered_prompt = render_template_text(&template, &variables);
    let record = json!({
        "route_id": stable_id("langprompt", &json!({"name": name, "provider": provider, "model": model})),
        "name": name,
        "provider": provider,
        "model": model,
        "selected_provider": selected_provider,
        "selected_model": selected_model,
        "profile": profile,
        "rendered_prompt": rendered_prompt,
        "template_variables": Value::Object(variables),
        "degraded": degraded,
        "reason_code": if degraded { "profile_provider_limited" } else { "prompt_route_ok" },
        "recorded_at": now_iso(),
    });
    let route_id = record
        .get("route_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "prompt_routes").insert(route_id, record.clone());
    Ok(json!({
        "ok": true,
        "route": record,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-014.5", workflow_chain_claim("V6-WORKFLOW-014.5")),
    }))
}
fn value_matches_structured_type(value: &Value, wanted: &str) -> bool {
    match wanted {
        "any" => true,
        "string" => value.is_string(),
        "number" | "float" => value.is_number(),
        "integer" | "int" => value.as_i64().is_some() || value.as_u64().is_some(),
        "boolean" | "bool" => value.is_boolean(),
        "array" => value.is_array(),
        "object" => value.is_object(),
        "null" => value.is_null(),
        _ => false,
    }
}
fn parse_structured_output(
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let parser_name = clean_token(payload.get("name").and_then(Value::as_str), "workflow_chain-structured-output");
    let schema = payload
        .get("schema")
        .and_then(Value::as_object)
        .cloned()
        .ok_or_else(|| "workflow_chain_structured_schema_required".to_string())?;
    let required_fields = schema
        .get("required_fields")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| row.as_str().map(ToString::to_string))
        .collect::<Vec<_>>();
    if required_fields.is_empty() {
        return Err("workflow_chain_structured_schema_required_fields_missing".to_string());
    }
    let parsed = if let Some(obj) = payload.get("output_json").and_then(Value::as_object) {
        Value::Object(obj.clone())
    } else {
        let raw = clean_text(payload.get("output_text").and_then(Value::as_str), 12000);
        if raw.is_empty() {
            return Err("workflow_chain_structured_output_missing".to_string());
        }
        serde_json::from_str::<Value>(&raw)
            .map_err(|err| format!("workflow_chain_structured_output_decode_failed:{err}"))?
    };
    let parsed_obj = parsed
        .as_object()
        .ok_or_else(|| "workflow_chain_structured_output_must_be_object".to_string())?;
    let field_types = schema
        .get("field_types")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let mut missing_fields = Vec::new();
    let mut invalid_fields = Vec::new();
    for field in &required_fields {
        let Some(value) = parsed_obj.get(field) else {
            missing_fields.push(field.to_string());
            continue;
        };
        let wanted = field_types
            .get(field)
            .and_then(Value::as_str)
            .unwrap_or("any");
        if !value_matches_structured_type(value, wanted) {
            invalid_fields.push(format!("{field}:{wanted}"));
        }
    }
    if !missing_fields.is_empty() || !invalid_fields.is_empty() {
        return Err(format!(
            "workflow_chain_structured_output_validation_failed:missing={}:invalid={}",
            missing_fields.join(","),
            invalid_fields.join(",")
        ));
    }
    let record = json!({
        "parse_id": stable_id("langparse", &json!({"name": parser_name, "required_fields": required_fields})),
        "name": parser_name,
        "schema": Value::Object(schema),
        "validated_output": Value::Object(parsed_obj.clone()),
        "recorded_at": now_iso(),
    });
    let parse_id = record.get("parse_id").and_then(Value::as_str).unwrap().to_string();
    as_object_mut(state, "structured_outputs").insert(parse_id, record.clone());
    Ok(json!({
        "ok": true,
        "structured_output": record,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-014.8", workflow_chain_claim("V6-WORKFLOW-014.8")),
    }))
}
fn checkpoint_run(
    root: &Path,
    argv: &[String],
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let chain_id = clean_token(payload.get("chain_id").and_then(Value::as_str), "");
    if chain_id.is_empty() {
        return Err("workflow_chain_checkpoint_chain_id_required".to_string());
    }
    let chain = state.get("chains").and_then(Value::as_object).and_then(|rows| rows.get(&chain_id)).cloned().ok_or_else(|| format!("unknown_workflow_chain_chain:{chain_id}"))?;
    let runnables = chain
        .get("runnables")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let profile = clean_token(payload.get("profile").and_then(Value::as_str), "rich");
    let parallel_count = runnables
        .iter()
        .filter(|row| row.get("parallel").and_then(Value::as_bool) == Some(true))
        .count();
    let degraded = matches!(profile.as_str(), "pure" | "tiny-max") && parallel_count > 1;
    let swarm_state_path = swarm_state_path(root, argv, payload);
    let root_session_id = if runnables.iter().any(|row| row.get("spawn").and_then(Value::as_bool) == Some(true)) {
        Some(ensure_session_for_task(
            root,
            &swarm_state_path,
            &format!("workflow_chain:checkpoint:{chain_id}"),
            "workflow_chain-checkpoint",
            Some("checkpoint"),
            None,
            parse_u64_value(payload.get("budget"), 768, 96, 12288),
        )?)
    } else {
        None
    };
    let state_snapshot = payload.get("state_snapshot").cloned().unwrap_or_else(|| json!({}));
    let record = json!({
        "checkpoint_id": stable_id("langcheckpoint", &json!({"chain_id": chain_id, "profile": profile})),
        "chain_id": chain_id,
        "prototype_label": clean_token(payload.get("prototype_label").and_then(Value::as_str), "workflow_chain-prototype"),
        "profile": profile,
        "state_snapshot": state_snapshot,
        "resume_token": stable_id("langresume", &json!({"profile": profile, "parallel_count": parallel_count})),
        "root_session_id": root_session_id,
        "runnable_count": runnables.len(),
        "degraded": degraded,
        "reason_code": if degraded { "parallel_chain_profile_limited" } else { "checkpoint_ready" },
        "checkpointed_at": now_iso(),
    });
    let checkpoint_id = record
        .get("checkpoint_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "checkpoints").insert(checkpoint_id, record.clone());
    Ok(json!({
        "ok": true,
        "checkpoint": record,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-014.7", workflow_chain_claim("V6-WORKFLOW-014.7")),
    }))
}
fn record_trace(
    root: &Path,
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let trace_id = clean_token(payload.get("trace_id").and_then(Value::as_str), "workflow_chain-trace");
    let steps = payload.get("steps").and_then(Value::as_array).cloned().unwrap_or_default();
    if steps.is_empty() {
        return Err("workflow_chain_trace_steps_required".to_string());
    }
    for step in &steps {
        let label = clean_token(step.get("stage").and_then(Value::as_str), "step");
        let message = clean_text(step.get("message").and_then(Value::as_str), 160);
        emit_native_trace(root, &trace_id, &label, &message)?;
    }
    let record = json!({
        "trace_id": trace_id,
        "steps": steps,
        "recorded_at": now_iso(),
    });
    as_array_mut(state, "traces").push(record.clone());
    Ok(json!({
        "ok": true,
        "trace": record,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-014.6", workflow_chain_claim("V6-WORKFLOW-014.6")),
    }))
}
fn import_integration(
    root: &Path,
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let name = clean_token(payload.get("name").and_then(Value::as_str), "workflow_chain-integration");
    let connector_type = clean_token(payload.get("integration_type").and_then(Value::as_str).or_else(|| payload.get("connector_type").and_then(Value::as_str)), "tool");
    if !allowed_connector_type(&connector_type) {
        return Err(format!(
            "workflow_chain_connector_type_unsupported:{connector_type}"
        ));
    }
    let bridge_path = normalize_bridge_path(
        root,
        payload
            .get("bridge_path")
            .and_then(Value::as_str)
            .unwrap_or("adapters/protocol/workflow_chain_connector_bridge.ts"),
    )?;
    let record = json!({
        "integration_id": stable_id("langint", &json!({"name": name, "connector_type": connector_type, "bridge_path": bridge_path})),
        "name": name,
        "integration_type": connector_type,
        "bridge_path": bridge_path,
        "assets": payload.get("assets").cloned().unwrap_or_else(|| json!([])),
        "supported_profiles": payload.get("supported_profiles").cloned().unwrap_or_else(|| json!(["rich", "pure"])),
        "imported_at": now_iso(),
    });
    let connector_id = record
        .get("integration_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "integrations").insert(connector_id, record.clone());
    Ok(json!({
        "ok": true,
        "integration": record,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-014.4", workflow_chain_claim("V6-WORKFLOW-014.4")),
    }))
}
fn assimilate_intake(
    root: &Path,
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let output_dir = normalize_shell_path(root, payload.get("output_dir").and_then(Value::as_str).unwrap_or("client/runtime/local/state/workflow_chain-shell"))?;
    let full = repo_path(root, &output_dir);
    let src_dir = full.join("src");
    let template_dir = full.join("templates");
    fs::create_dir_all(&src_dir)
        .map_err(|err| format!("workflow_chain_intake_src_dir_create_failed:{err}"))?;
    fs::create_dir_all(&template_dir)
        .map_err(|err| format!("workflow_chain_intake_template_dir_create_failed:{err}"))?;
    let package_json = json!({
        "name": clean_token(payload.get("package_name").and_then(Value::as_str), "workflow_chain-shell"),
        "private": true,
        "scripts": {
            "start": "node src/workflow_chain.pipeline.ts"
        }
    });
    let pipeline_source = "export const workflow_chainChain = { runnables: [\n  { id: 'retrieve', runnable_type: 'retriever', input_type: 'query', output_type: 'documents' },\n  { id: 'route', runnable_type: 'prompt', input_type: 'documents', output_type: 'prompt' },\n  { id: 'answer', runnable_type: 'llm', input_type: 'prompt', output_type: 'answer', spawn: true }\n] };\n";
    let readme =
        "# Workflow Chain Shell\n\nThin generated shell over `core://workflow_chain-bridge`.\n";
    let prompt_template = "Answer the question: {{question}}\nUse only the supplied context.\n";
    fs::write(
        full.join("package.json"),
        serde_json::to_string_pretty(&package_json).unwrap(),
    )
    .map_err(|err| format!("workflow_chain_intake_package_write_failed:{err}"))?;
    fs::write(src_dir.join("workflow_chain.pipeline.ts"), pipeline_source)
        .map_err(|err| format!("workflow_chain_intake_pipeline_write_failed:{err}"))?;
    fs::write(template_dir.join("prompt.jinja"), prompt_template)
        .map_err(|err| format!("workflow_chain_intake_template_write_failed:{err}"))?;
    fs::write(full.join("README.md"), readme)
        .map_err(|err| format!("workflow_chain_intake_readme_write_failed:{err}"))?;
    let record = json!({
        "intake_id": stable_id("langintake", &json!({"output_dir": output_dir})),
        "output_dir": output_dir,
        "files": [
            format!("{}/package.json", rel(root, &full)),
            format!("{}/src/workflow_chain.pipeline.ts", rel(root, &full)),
            format!("{}/templates/prompt.jinja", rel(root, &full)),
            format!("{}/README.md", rel(root, &full)),
        ],
        "created_at": now_iso(),
    });
    let intake_id = record
        .get("intake_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "intakes").insert(intake_id, record.clone());
    Ok(json!({
        "ok": true,
        "intake": record,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-014.4", workflow_chain_claim("V6-WORKFLOW-014.4")),
    }))
}
