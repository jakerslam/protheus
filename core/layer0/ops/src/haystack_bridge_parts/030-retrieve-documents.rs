fn retrieve_documents(state: &mut Value, payload: &Map<String, Value>) -> Result<Value, String> {
    let store_id = clean_token(payload.get("store_id").and_then(Value::as_str), "");
    if store_id.is_empty() {
        return Err("haystack_store_id_required".to_string());
    }
    let query = clean_text(payload.get("query").and_then(Value::as_str), 240);
    if query.is_empty() {
        return Err("haystack_query_required".to_string());
    }
    let mode = clean_token(payload.get("mode").and_then(Value::as_str), "hybrid");
    let profile = clean_token(payload.get("profile").and_then(Value::as_str), "rich");
    let store = state
        .get("document_stores")
        .and_then(Value::as_object)
        .and_then(|rows| rows.get(&store_id))
        .cloned()
        .ok_or_else(|| format!("unknown_haystack_document_store:{store_id}"))?;
    let supported_profiles = store
        .get("supported_profiles")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if !supported_profiles
        .iter()
        .filter_map(Value::as_str)
        .any(|row| row == profile)
    {
        return Err(format!(
            "haystack_document_store_profile_unsupported:{profile}"
        ));
    }
    let supported_mode_rows = store
        .get("retrieval_modes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let supported_modes = supported_mode_rows
        .iter()
        .filter_map(Value::as_str)
        .collect::<BTreeSet<_>>();
    if !supported_modes.contains(mode.as_str()) {
        return Err(format!("haystack_retrieval_mode_unsupported:{mode}"));
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
        "retrieval_id": stable_id("hayret", &json!({"store_id": store_id, "query": query, "mode": mode})),
        "store_id": store_id,
        "query": query,
        "mode": mode,
        "profile": profile,
        "degraded": top_k != requested_top_k,
        "reason_code": if top_k != requested_top_k { "profile_context_budget_limited" } else { "retrieval_ok" },
        "results": results,
        "context_budget": context_budget,
        "recorded_at": now_iso(),
    });
    let retrieval_id = retrieval
        .get("retrieval_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "retrieval_runs").insert(retrieval_id, retrieval.clone());
    Ok(json!({
        "ok": true,
        "retrieval": retrieval,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-012.4", haystack_claim("V6-WORKFLOW-012.4")),
    }))
}

fn route_and_rank(state: &mut Value, payload: &Map<String, Value>) -> Result<Value, String> {
    let name = clean_token(
        payload.get("name").and_then(Value::as_str),
        "haystack-router",
    );
    let routes = payload
        .get("routes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let candidates = payload
        .get("candidates")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if routes.is_empty() {
        return Err("haystack_routes_required".to_string());
    }
    let context = payload
        .get("context")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let query = clean_text(payload.get("query").and_then(Value::as_str), 240);
    let terms = query_terms(&query);
    let mut best_route = None::<(i64, Value)>;
    for route in routes {
        let obj = route.as_object().cloned().unwrap_or_default();
        let mut score = parse_u64_value(obj.get("weight"), 0, 0, 100) as i64;
        if let Some(field) = obj.get("field").and_then(Value::as_str) {
            if let Some(expected) = obj.get("equals") {
                if context.get(field) == Some(expected) {
                    score += 10;
                }
            }
        }
        if let Some(tag) = obj.get("contains").and_then(Value::as_str) {
            if query
                .to_ascii_lowercase()
                .contains(&tag.to_ascii_lowercase())
            {
                score += 4;
            }
        }
        if best_route
            .as_ref()
            .map(|(current, _)| score > *current)
            .unwrap_or(true)
        {
            best_route = Some((score, Value::Object(obj)));
        }
    }
    let (_, route) = best_route.ok_or_else(|| "haystack_route_selection_failed".to_string())?;
    let route_obj = route.as_object().cloned().unwrap_or_default();
    let mut ranked = candidates
        .into_iter()
        .map(|candidate| {
            let metadata_boost = route_obj
                .get("metadata_key")
                .and_then(Value::as_str)
                .and_then(|key| candidate.get("metadata").and_then(|row| row.get(key)))
                .map(|_| 2)
                .unwrap_or(0);
            let score = retrieval_score(&candidate, &terms, "ranker") + metadata_boost;
            (score, candidate)
        })
        .collect::<Vec<_>>();
    ranked.sort_by(|a, b| b.0.cmp(&a.0));
    let record = json!({
        "route_id": stable_id("hayroute", &json!({"name": name, "query": query})),
        "name": name,
        "selected_route": {
            "id": clean_token(route_obj.get("id").and_then(Value::as_str), "route"),
            "reason": clean_text(route_obj.get("reason").and_then(Value::as_str), 160),
        },
        "ranked": ranked.into_iter().take(4).map(|(score, candidate)| json!({
            "score": score,
            "text": candidate.get("text").cloned().unwrap_or(Value::Null),
            "metadata": candidate.get("metadata").cloned().unwrap_or(Value::Null),
        })).collect::<Vec<_>>(),
        "context": context,
        "query": query,
        "recorded_at": now_iso(),
    });
    let route_id = record
        .get("route_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "routes").insert(route_id, record.clone());
    Ok(json!({
        "ok": true,
        "route": record,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-012.5", haystack_claim("V6-WORKFLOW-012.5")),
    }))
}

fn record_multimodal_eval(
    root: &Path,
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let name = clean_token(payload.get("name").and_then(Value::as_str), "haystack-eval");
    let artifacts = payload
        .get("artifacts")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if artifacts.is_empty() {
        return Err("haystack_eval_artifacts_required".to_string());
    }
    let profile = clean_token(payload.get("profile").and_then(Value::as_str), "rich");
    let multimodal = artifacts
        .iter()
        .filter_map(|row| row.get("media_type").and_then(Value::as_str))
        .any(|kind| kind != "text/plain");
    let degraded = multimodal && matches!(profile.as_str(), "pure" | "tiny-max");
    let record = json!({
        "evaluation_id": stable_id("hayeval", &json!({"name": name, "artifact_count": artifacts.len()})),
        "name": name,
        "artifact_count": artifacts.len(),
        "artifacts": artifacts,
        "metrics": payload.get("metrics").cloned().unwrap_or_else(|| json!({})),
        "profile": profile,
        "degraded": degraded,
        "reason_code": if degraded { "multimodal_evaluation_profile_limited" } else { "evaluation_ok" },
        "recorded_at": now_iso(),
    });
    let evaluation_id = record
        .get("evaluation_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    emit_native_trace(
        root,
        &evaluation_id,
        "haystack_eval",
        &format!(
            "name={name} artifacts={}",
            record
                .get("artifact_count")
                .and_then(Value::as_u64)
                .unwrap_or(0)
        ),
    )?;
    as_object_mut(state, "evaluations").insert(evaluation_id, record.clone());
    Ok(json!({
        "ok": true,
        "evaluation": record,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-012.6", haystack_claim("V6-WORKFLOW-012.6")),
    }))
}

fn trace_run(
    root: &Path,
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let trace_id = clean_token(
        payload.get("trace_id").and_then(Value::as_str),
        "haystack-trace",
    );
    let steps = payload
        .get("steps")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if steps.is_empty() {
        return Err("haystack_trace_steps_required".to_string());
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
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-012.7", haystack_claim("V6-WORKFLOW-012.7")),
    }))
}

fn import_connector(
    root: &Path,
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let name = clean_token(
        payload.get("name").and_then(Value::as_str),
        "haystack-connector",
    );
    let connector_type = clean_token(payload.get("connector_type").and_then(Value::as_str), "mcp");
    if !allowed_connector_type(&connector_type) {
        return Err(format!(
            "haystack_connector_type_unsupported:{connector_type}"
        ));
    }
    let bridge_path = normalize_bridge_path(
        root,
        payload
            .get("bridge_path")
            .and_then(Value::as_str)
            .unwrap_or("adapters/protocol/haystack_connector_bridge.ts"),
    )?;
    let record = json!({
        "connector_id": stable_id("hayconn", &json!({"name": name, "connector_type": connector_type, "bridge_path": bridge_path})),
        "name": name,
        "connector_type": connector_type,
        "bridge_path": bridge_path,
        "assets": payload.get("assets").cloned().unwrap_or_else(|| json!([])),
        "supported_profiles": payload.get("supported_profiles").cloned().unwrap_or_else(|| json!(["rich", "pure"])),
        "imported_at": now_iso(),
    });
    let connector_id = record
        .get("connector_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "connectors").insert(connector_id, record.clone());
    Ok(json!({
        "ok": true,
        "connector": record,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-012.8", haystack_claim("V6-WORKFLOW-012.8")),
    }))
}

fn assimilate_intake(
    root: &Path,
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let output_dir = normalize_shell_path(
        root,
        payload
            .get("output_dir")
            .and_then(Value::as_str)
            .unwrap_or("client/runtime/local/state/haystack-shell"),
    )?;
    let full = repo_path(root, &output_dir);
    let src_dir = full.join("src");
    let template_dir = full.join("templates");
    fs::create_dir_all(&src_dir)
        .map_err(|err| format!("haystack_intake_src_dir_create_failed:{err}"))?;
    fs::create_dir_all(&template_dir)
        .map_err(|err| format!("haystack_intake_template_dir_create_failed:{err}"))?;
    let package_json = json!({
        "name": clean_token(payload.get("package_name").and_then(Value::as_str), "haystack-shell"),
        "private": true,
        "scripts": {
            "start": "node src/haystack.pipeline.ts"
        }
    });
    let pipeline_source = "export const haystackPipeline = { components: [\n  { id: 'retrieve', stage_type: 'retriever', input_type: 'query', output_type: 'documents' },\n  { id: 'rank', stage_type: 'ranker', input_type: 'documents', output_type: 'documents' },\n  { id: 'answer', stage_type: 'generator', input_type: 'documents', output_type: 'answer', spawn: true }\n] };\n";
    let readme = "# Haystack Shell\n\nThin generated shell over `core://haystack-bridge`.\n";
    let prompt_template = "Answer the question: {{question}}\nUse only the supplied context.\n";
    fs::write(
        full.join("package.json"),
        serde_json::to_string_pretty(&package_json).unwrap(),
    )
    .map_err(|err| format!("haystack_intake_package_write_failed:{err}"))?;
    fs::write(src_dir.join("haystack.pipeline.ts"), pipeline_source)
        .map_err(|err| format!("haystack_intake_pipeline_write_failed:{err}"))?;
    fs::write(template_dir.join("prompt.jinja"), prompt_template)
        .map_err(|err| format!("haystack_intake_template_write_failed:{err}"))?;
    fs::write(full.join("README.md"), readme)
        .map_err(|err| format!("haystack_intake_readme_write_failed:{err}"))?;
    let record = json!({
        "intake_id": stable_id("hayintake", &json!({"output_dir": output_dir})),
        "output_dir": output_dir,
        "files": [
            format!("{}/package.json", rel(root, &full)),
            format!("{}/src/haystack.pipeline.ts", rel(root, &full)),
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
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-012.8", haystack_claim("V6-WORKFLOW-012.8")),
    }))
}

