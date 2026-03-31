fn register_vector_connector(
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let name = clean_token(
        payload.get("name").and_then(Value::as_str),
        "semantic-vector",
    );
    let provider = clean_token(
        payload.get("provider").and_then(Value::as_str),
        "memory-plane",
    );
    if !supported_vector_provider(&provider) {
        return Err("semantic_kernel_vector_provider_invalid".to_string());
    }
    let connector = json!({
        "connector_id": stable_id("skvec", &json!({"name": name, "provider": provider})),
        "name": name,
        "provider": provider,
        "context_budget_tokens": parse_u64_value(payload.get("context_budget_tokens"), 512, 32, 4096),
        "min_profile": if provider == "memory-plane" { "tiny-max" } else { "rich" },
        "documents": payload.get("documents").cloned().filter(Value::is_array).unwrap_or_else(|| json!([])),
        "registered_at": now_iso(),
    });
    let connector_id = connector
        .get("connector_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "vector_connectors").insert(connector_id.clone(), connector.clone());
    Ok(json!({
        "ok": true,
        "connector": connector,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-008.5", semantic_claim("V6-WORKFLOW-008.5")),
    }))
}

fn lexical_score(query: &str, text: &str) -> u64 {
    let query_lc = query.to_ascii_lowercase();
    let query_tokens = query_lc
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|token| !token.is_empty())
        .collect::<BTreeSet<_>>();
    let text_lc = text.to_ascii_lowercase();
    query_tokens
        .into_iter()
        .map(|token| text_lc.matches(token).count() as u64)
        .sum()
}

fn retrieve(state: &mut Value, payload: &Map<String, Value>) -> Result<Value, String> {
    let connector_id = clean_token(payload.get("connector_id").and_then(Value::as_str), "");
    let query = clean_text(payload.get("query").and_then(Value::as_str), 240);
    if connector_id.is_empty() || query.is_empty() {
        return Err("semantic_kernel_retrieve_connector_and_query_required".to_string());
    }
    let connectors = as_object_mut(state, "vector_connectors");
    let connector = connectors
        .get(&connector_id)
        .and_then(Value::as_object)
        .ok_or_else(|| "semantic_kernel_vector_connector_not_found".to_string())?;
    let provider = connector
        .get("provider")
        .and_then(Value::as_str)
        .unwrap_or("memory-plane");
    let profile = normalized_profile(
        payload
            .get("profile")
            .and_then(Value::as_str)
            .unwrap_or("rich"),
    );
    let min_profile = connector
        .get("min_profile")
        .and_then(Value::as_str)
        .unwrap_or("rich");
    if min_profile == "rich" && profile != "rich" {
        return Err(format!(
            "semantic_kernel_vector_connector_degraded_profile:{provider}:{profile}"
        ));
    }
    let top_k = parse_u64_value(payload.get("top_k"), 3, 1, 12) as usize;
    let budget = connector
        .get("context_budget_tokens")
        .and_then(Value::as_u64)
        .unwrap_or(512);
    let docs = connector
        .get("documents")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut ranked = docs
        .into_iter()
        .filter_map(|row| {
            let text = row
                .get("text")
                .and_then(Value::as_str)
                .map(ToString::to_string)
                .unwrap_or_else(|| row.to_string());
            let score = lexical_score(&query, &text);
            (score > 0).then(|| {
                json!({
                    "text": text,
                    "score": score,
                    "token_estimate": approx_token_count(&text),
                    "metadata": row.get("metadata").cloned().unwrap_or(Value::Null),
                })
            })
        })
        .collect::<Vec<_>>();
    ranked.sort_by(|a, b| {
        b.get("score")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            .cmp(&a.get("score").and_then(Value::as_u64).unwrap_or(0))
    });
    let mut used = 0_u64;
    let mut results = Vec::new();
    for row in ranked.into_iter().take(top_k) {
        let tokens = row
            .get("token_estimate")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        if used.saturating_add(tokens) > budget {
            break;
        }
        used = used.saturating_add(tokens);
        results.push(row);
    }
    Ok(json!({
        "ok": true,
        "connector_id": connector_id,
        "provider": provider,
        "profile": profile,
        "results": results,
        "used_tokens": used,
        "budget_tokens": budget,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-008.5", semantic_claim("V6-WORKFLOW-008.5")),
    }))
}

fn supported_llm_provider(provider: &str) -> bool {
    matches!(
        provider,
        "azure-openai" | "ollama" | "hugging-face" | "nvidia" | "openai-compatible"
    )
}

fn register_llm_connector(
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let name = clean_token(payload.get("name").and_then(Value::as_str), "semantic-llm");
    let provider = clean_token(
        payload.get("provider").and_then(Value::as_str),
        "openai-compatible",
    );
    if !supported_llm_provider(&provider) {
        return Err("semantic_kernel_llm_provider_invalid".to_string());
    }
    let modalities = payload
        .get("modalities")
        .cloned()
        .filter(Value::is_array)
        .unwrap_or_else(|| json!(["text"]));
    let connector = json!({
        "connector_id": stable_id("skllm", &json!({"name": name, "provider": provider, "modalities": modalities})),
        "name": name,
        "provider": provider,
        "model": clean_text(payload.get("model").and_then(Value::as_str), 120),
        "modalities": modalities,
        "registered_at": now_iso(),
    });
    let connector_id = connector
        .get("connector_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "llm_connectors").insert(connector_id.clone(), connector.clone());
    Ok(json!({
        "ok": true,
        "connector": connector,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-008.6", semantic_claim("V6-WORKFLOW-008.6")),
    }))
}

fn route_llm(state: &mut Value, payload: &Map<String, Value>) -> Result<Value, String> {
    let connector_id = clean_token(payload.get("connector_id").and_then(Value::as_str), "");
    if connector_id.is_empty() {
        return Err("semantic_kernel_llm_connector_required".to_string());
    }
    let connectors = as_object_mut(state, "llm_connectors");
    let connector = connectors
        .get(&connector_id)
        .and_then(Value::as_object)
        .ok_or_else(|| "semantic_kernel_llm_connector_not_found".to_string())?;
    let modality = clean_token(payload.get("modality").and_then(Value::as_str), "text");
    let connector_modalities = connector
        .get("modalities")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let supports_modality = connector_modalities
        .iter()
        .any(|row| row.as_str() == Some(modality.as_str()));
    if !supports_modality {
        return Err(format!(
            "semantic_kernel_llm_modality_unsupported:{modality}"
        ));
    }
    let profile = normalized_profile(
        payload
            .get("profile")
            .and_then(Value::as_str)
            .unwrap_or("rich"),
    );
    if modality != "text" && profile != "rich" {
        return Err(format!(
            "semantic_kernel_llm_multimodal_profile_blocked:{profile}:{modality}"
        ));
    }
    Ok(json!({
        "ok": true,
        "route": {
            "connector_id": connector_id,
            "provider": connector.get("provider").cloned().unwrap_or(Value::Null),
            "model": connector.get("model").cloned().unwrap_or(Value::Null),
            "modality": modality,
            "prompt_tokens_estimate": approx_token_count(payload.get("prompt").and_then(Value::as_str).unwrap_or("")),
            "profile": profile,
        },
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-008.6", semantic_claim("V6-WORKFLOW-008.6")),
    }))
}

fn schema_type_matches(expected: &str, value: &Value) -> bool {
    match expected {
        "object" => value.is_object(),
        "array" => value.is_array(),
        "string" => value.is_string(),
        "number" => value.is_number(),
        "boolean" => value.is_boolean(),
        "null" => value.is_null(),
        _ => true,
    }
}

fn validate_json_schema(schema: &Value, value: &Value, path: &str, violations: &mut Vec<String>) {
    let expected_type = schema.get("type").and_then(Value::as_str).unwrap_or("");
    if !expected_type.is_empty() && !schema_type_matches(expected_type, value) {
        violations.push(format!("type_mismatch:{}:{}", path, expected_type));
        return;
    }
    if let Some(required) = schema.get("required").and_then(Value::as_array) {
        if let Some(map) = value.as_object() {
            for field in required.iter().filter_map(Value::as_str) {
                if !map.contains_key(field) {
                    violations.push(format!("missing_required:{}:{}", path, field));
                }
            }
        }
    }
    if let Some(properties) = schema.get("properties").and_then(Value::as_object) {
        if let Some(map) = value.as_object() {
            for (key, child_schema) in properties {
                if let Some(child_value) = map.get(key) {
                    let child_path = if path == "$" {
                        format!("$.{}", key)
                    } else {
                        format!("{}.{}", path, key)
                    };
                    validate_json_schema(child_schema, child_value, &child_path, violations);
                }
            }
        }
    }
    if let Some(items) = schema.get("items") {
        if let Some(rows) = value.as_array() {
            for (index, row) in rows.iter().enumerate() {
                validate_json_schema(items, row, &format!("{}[{}]", path, index), violations);
            }
        }
    }
    if let Some(options) = schema.get("enum").and_then(Value::as_array) {
        if !options.iter().any(|row| row == value) {
            violations.push(format!("enum_violation:{}", path));
        }
    }
}

fn validate_process_graph(process: &Value) -> Result<Value, String> {
    let steps = process
        .get("steps")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if steps.is_empty() {
        return Err("semantic_kernel_process_steps_required".to_string());
    }
    let mut ids = BTreeSet::new();
    for step in &steps {
        let step_id = clean_token(step.get("id").and_then(Value::as_str), "");
        if step_id.is_empty() {
            return Err("semantic_kernel_process_step_id_required".to_string());
        }
        ids.insert(step_id);
    }
    for step in &steps {
        if let Some(next) = step.get("next").and_then(Value::as_str) {
            let next_id = clean_token(Some(next), "");
            if !next_id.is_empty() && !ids.contains(&next_id) {
                return Err(format!("semantic_kernel_process_missing_next:{next_id}"));
            }
        }
    }
    Ok(json!({
        "step_count": steps.len(),
        "validated": true,
    }))
}

fn validate_structured_output(
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let schema = payload.get("schema").cloned().unwrap_or_else(|| json!({}));
    let output = payload.get("output").cloned().unwrap_or(Value::Null);
    let mut violations = Vec::new();
    validate_json_schema(&schema, &output, "$", &mut violations);
    let process_report = if let Some(process) = payload.get("process") {
        Some(validate_process_graph(process)?)
    } else {
        None
    };
    if !violations.is_empty() {
        return Err(format!(
            "semantic_kernel_structured_output_invalid:{}",
            violations.join(",")
        ));
    }
    let record = json!({
        "record_id": stable_id("skproc", &json!({"schema": schema, "output": output, "process": payload.get("process")})),
        "schema": schema,
        "output": output,
        "process_report": process_report,
        "validated_at": now_iso(),
    });
    let record_id = record
        .get("record_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "structured_processes").insert(record_id.clone(), record.clone());
    Ok(json!({
        "ok": true,
        "record": record,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-008.7", semantic_claim("V6-WORKFLOW-008.7")),
    }))
}

fn emit_enterprise_event(state: &mut Value, payload: &Map<String, Value>) -> Result<Value, String> {
    let sink = clean_token(payload.get("sink").and_then(Value::as_str), "otel");
    let cloud = clean_token(payload.get("cloud").and_then(Value::as_str), "azure");
    let endpoint = clean_text(payload.get("endpoint").and_then(Value::as_str), 200);
    if !endpoint.is_empty() && !endpoint.starts_with("https://") {
        return Err("semantic_kernel_enterprise_endpoint_must_be_https".to_string());
    }
    let event = json!({
        "event_id": stable_id("skevt", &json!({"sink": sink, "cloud": cloud, "event_type": payload.get("event_type")})),
        "event_type": clean_token(payload.get("event_type").and_then(Value::as_str), "semantic-kernel-observability"),
        "sink": sink,
        "cloud": cloud,
        "endpoint": endpoint,
        "tags": payload.get("tags").cloned().filter(Value::is_object).unwrap_or_else(|| json!({})),
        "deployment": payload.get("deployment").cloned().filter(Value::is_object).unwrap_or_else(|| json!({})),
        "recorded_at": now_iso(),
    });
    as_array_mut(state, "enterprise_events").push(event.clone());
    Ok(json!({
        "ok": true,
        "event": event,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-008.8", semantic_claim("V6-WORKFLOW-008.8")),
    }))
}

fn register_dotnet_bridge(
    root: &Path,
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let name = clean_token(
        payload.get("name").and_then(Value::as_str),
        "semantic-kernel-dotnet",
    );
    let bridge_path = normalize_bridge_path(
        root,
        payload
            .get("bridge_path")
            .and_then(Value::as_str)
            .unwrap_or("adapters/polyglot/semantic_kernel_dotnet_bridge.ts"),
    )?;
    if !bridge_path.starts_with("adapters/") {
        return Err("semantic_kernel_dotnet_bridge_must_live_in_adapters".to_string());
    }
    let bridge = json!({
        "bridge_id": stable_id("skdotnet", &json!({"name": name, "bridge_path": bridge_path})),
        "name": name,
        "bridge_path": bridge_path,
        "command": clean_text(payload.get("command").and_then(Value::as_str), 160),
        "command_args": payload.get("command_args").cloned().filter(Value::is_array).unwrap_or_else(|| json!([])),
        "capabilities": payload.get("capabilities").cloned().filter(Value::is_array).unwrap_or_else(|| json!([])),
        "registered_at": now_iso(),
    });
    let bridge_id = bridge
        .get("bridge_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "dotnet_bridges").insert(bridge_id.clone(), bridge.clone());
    Ok(json!({
        "ok": true,
        "bridge": bridge,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-008.9", semantic_claim("V6-WORKFLOW-008.9")),
    }))
}

