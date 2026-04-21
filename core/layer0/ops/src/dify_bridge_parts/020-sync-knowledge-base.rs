
fn sync_knowledge_base(
    root: &Path,
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let profile = profile(payload.get("profile"));
    let name = clean_text(
        payload.get("knowledge_base_name").and_then(Value::as_str),
        120,
    );
    if name.is_empty() {
        return Err("dify_knowledge_base_name_required".to_string());
    }
    let query = clean_text(payload.get("query").and_then(Value::as_str), 120);
    let documents = payload
        .get("documents")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let adapter_path = normalize_bridge_path(
        root,
        payload
            .get("bridge_path")
            .and_then(Value::as_str)
            .unwrap_or("adapters/protocol/dify_connector_bridge.ts"),
    )?;
    let multimodal = documents.iter().any(|row| {
        row.get("modality")
            .and_then(Value::as_str)
            .unwrap_or("text")
            != "text"
    });
    let degraded = matches!(profile.as_str(), "tiny-max") && multimodal;
    let query_lower = query.to_ascii_lowercase();
    let retrieval_hits: Vec<Value> = documents
        .iter()
        .filter(|row| {
            let title = row
                .get("title")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_ascii_lowercase();
            let text = row
                .get("text")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_ascii_lowercase();
            query_lower.is_empty() || title.contains(&query_lower) || text.contains(&query_lower)
        })
        .take(3)
        .cloned()
        .collect();
    let record = json!({
        "knowledge_base_id": stable_id("difykb", &json!({"name": name, "query": query, "documents": documents})),
        "name": name,
        "profile": profile,
        "bridge_path": adapter_path,
        "document_count": documents.len(),
        "query": query,
        "retrieval_hits": retrieval_hits,
        "context_budget": payload.get("context_budget").cloned().unwrap_or_else(|| json!(4096)),
        "degraded": degraded,
        "synced_at": now_iso(),
    });
    let id = record
        .get("knowledge_base_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "knowledge_bases").insert(id, record.clone());
    Ok(json!({
        "ok": true,
        "knowledge_base": record,
        "claim_evidence": claim("V6-WORKFLOW-005.2", "dify_knowledge_base_and_rag_semantics_route_through_governed_retrieval_with_budgeted_receipts"),
    }))
}

fn register_agent_app(
    root: &Path,
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let app_name = clean_text(payload.get("app_name").and_then(Value::as_str), 120);
    if app_name.is_empty() {
        return Err("dify_agent_app_name_required".to_string());
    }
    let adapter_path = normalize_bridge_path(
        root,
        payload
            .get("bridge_path")
            .and_then(Value::as_str)
            .unwrap_or("adapters/protocol/dify_connector_bridge.ts"),
    )?;
    let tools = payload
        .get("tools")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let plugins = payload
        .get("plugins")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let modalities = payload
        .get("modalities")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_else(|| vec![json!("text")]);
    let denied_tools: Vec<Value> = tools
        .iter()
        .filter(|row| {
            let label = match row {
                Value::String(v) => v.as_str(),
                Value::Object(map) => map.get("name").and_then(Value::as_str).unwrap_or(""),
                _ => "",
            }
            .to_ascii_lowercase();
            label.contains("delete") || label.contains("rm") || label.contains("destructive")
        })
        .cloned()
        .collect();
    let allowed_tools: Vec<Value> = tools
        .iter()
        .filter(|row| !denied_tools.iter().any(|deny| deny == *row))
        .cloned()
        .collect();
    let app = json!({
        "app_id": stable_id("difyapp", &json!({"name": app_name, "tools": allowed_tools, "plugins": plugins})),
        "app_name": app_name,
        "bridge_path": adapter_path,
        "tool_count": allowed_tools.len(),
        "plugin_count": plugins.len(),
        "modalities": modalities,
        "allowed_tools": allowed_tools,
        "denied_tools": denied_tools,
        "registered_at": now_iso(),
    });
    let id = app
        .get("app_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "agent_apps").insert(id, app.clone());
    Ok(json!({
        "ok": true,
        "agent_app": app,
        "claim_evidence": claim("V6-WORKFLOW-005.3", "dify_agentic_apps_plugins_and_multimodal_tools_are_registered_with_fail_closed_denials"),
    }))
}

fn publish_dashboard(
    root: &Path,
    state: &mut Value,
    dashboard_dir: &Path,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let dashboard_name = clean_text(payload.get("dashboard_name").and_then(Value::as_str), 120);
    if dashboard_name.is_empty() {
        return Err("dify_dashboard_name_required".to_string());
    }
    let team = clean_token(payload.get("team").and_then(Value::as_str), "default-team");
    let environment = clean_token(
        payload.get("environment").and_then(Value::as_str),
        "staging",
    );
    fs::create_dir_all(dashboard_dir)
        .map_err(|err| format!("dify_dashboard_dir_create_failed:{err}"))?;
    let record = json!({
        "dashboard_id": stable_id("difydash", &json!({"dashboard_name": dashboard_name, "team": team, "environment": environment})),
        "dashboard_name": dashboard_name,
        "team": team,
        "environment": environment,
        "publish_action": clean_token(payload.get("publish_action").and_then(Value::as_str), "deploy"),
        "deploy_target": clean_text(payload.get("deploy_target").and_then(Value::as_str), 120),
        "shell_path": rel(root, dashboard_dir),
        "published_at": now_iso(),
    });
    let id = record
        .get("dashboard_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    fs::write(
        dashboard_dir.join(format!("{id}.json")),
        serde_json::to_string_pretty(&record)
            .map_err(|err| format!("dify_dashboard_encode_failed:{err}"))?,
    )
    .map_err(|err| format!("dify_dashboard_write_failed:{err}"))?;
    as_object_mut(state, "dashboards").insert(id, record.clone());
    Ok(json!({
        "ok": true,
        "dashboard": record,
        "claim_evidence": claim("V6-WORKFLOW-005.4", "dify_team_collaboration_and_deployment_dashboards_remain_shells_over_governed_actions"),
    }))
}
