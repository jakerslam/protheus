
#[test]
fn agent_create_cua_rejects_unsupported_execution_features() {
    let root = agent_create_temp_root();
    init_git_repo(root.path());
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"mode":"cua","stream":true,"messages":[{"role":"user","content":"hi"}],"excludeTools":["web_search"],"output":{"type":"json"},"variables":{"city":"SF"}}"#,
        &agent_create_ok_snapshot(),
    )
    .expect("create cua agent");
    assert_eq!(created.status, 400);
    assert_eq!(
        created.payload.get("error").and_then(Value::as_str),
        Some("cua_unsupported_features")
    );
    assert_eq!(
        created
            .payload
            .get("validation_contract_version")
            .and_then(Value::as_str),
        Some("v1")
    );
    let unsupported = created
        .payload
        .get("unsupported_features")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| row.as_str().map(ToString::to_string))
        .collect::<Vec<_>>();
    assert!(unsupported.iter().any(|row| row == "streaming"));
    assert!(unsupported.iter().any(|row| row == "message continuation"));
    assert!(unsupported.iter().any(|row| row == "excludeTools"));
    assert!(unsupported.iter().any(|row| row == "output schema"));
    assert!(unsupported.iter().any(|row| row == "variables"));
    assert_eq!(
        created
            .payload
            .get("unsupported_features_count")
            .and_then(Value::as_u64),
        Some(unsupported.len() as u64)
    );
    let signature = created
        .payload
        .get("unsupported_features_signature")
        .and_then(Value::as_str)
        .unwrap_or("");
    assert!(signature.contains("streaming"));
    assert!(signature.contains("variables"));
}

#[test]
fn agent_create_cua_accepts_minimal_payload() {
    let root = agent_create_temp_root();
    init_git_repo(root.path());
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"mode":"cua","role":"analyst"}"#,
        &agent_create_ok_snapshot(),
    )
    .expect("create cua agent");
    assert_eq!(created.status, 200);
    assert_eq!(created.payload.get("ok").and_then(Value::as_bool), Some(true));
}

#[test]
fn agent_create_cua_rejects_unsupported_execution_feature_aliases() {
    let root = agent_create_temp_root();
    init_git_repo(root.path());
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"mode":"cua","streaming":true,"abortSignal":{"id":"sig-1"},"messageContinuation":[{"role":"user","content":"hi"}],"outputSchema":{"type":"json"},"responseFormat":{"type":"json_schema"},"variablesJson":{"city":"SF"}}"#,
        &agent_create_ok_snapshot(),
    )
    .expect("create cua agent");
    assert_eq!(created.status, 400);
    assert_eq!(
        created.payload.get("error").and_then(Value::as_str),
        Some("cua_unsupported_features")
    );
    let unsupported = created
        .payload
        .get("unsupported_features")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| row.as_str().map(ToString::to_string))
        .collect::<Vec<_>>();
    assert!(unsupported.iter().any(|row| row == "streaming"));
    assert!(unsupported.iter().any(|row| row == "abort signal"));
    assert!(unsupported.iter().any(|row| row == "message continuation"));
    assert!(unsupported.iter().any(|row| row == "output schema"));
    assert!(unsupported.iter().any(|row| row == "variables"));
}

#[test]
fn agent_create_cua_rejects_malformed_scalar_unsupported_feature_forms() {
    let root = agent_create_temp_root();
    init_git_repo(root.path());
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"mode":"cua","excludeTools":"web_search","variables":"city=SF","exclude_tools":"fallback_tool"}"#,
        &agent_create_ok_snapshot(),
    )
    .expect("create cua agent");
    assert_eq!(created.status, 400);
    assert_eq!(
        created.payload.get("error").and_then(Value::as_str),
        Some("cua_unsupported_features")
    );
    let unsupported = created
        .payload
        .get("unsupported_features")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| row.as_str().map(ToString::to_string))
        .collect::<Vec<_>>();
    assert!(unsupported.iter().any(|row| row == "excludeTools"));
    assert!(unsupported.iter().any(|row| row == "variables"));
}

#[test]
fn agent_create_cua_rejects_malformed_stream_flag_forms() {
    let root = agent_create_temp_root();
    init_git_repo(root.path());
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"mode":"cua","stream":"true","streaming":{"enabled":true}}"#,
        &agent_create_ok_snapshot(),
    )
    .expect("create cua agent");
    assert_eq!(created.status, 400);
    assert_eq!(
        created.payload.get("error").and_then(Value::as_str),
        Some("cua_unsupported_features")
    );
    let unsupported = created
        .payload
        .get("unsupported_features")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| row.as_str().map(ToString::to_string))
        .collect::<Vec<_>>();
    assert!(unsupported.iter().any(|row| row == "streaming"));
    let signature = created
        .payload
        .get("unsupported_features_signature")
        .and_then(Value::as_str)
        .unwrap_or("");
    assert!(signature.contains("streaming"));
}

#[test]
fn repeated_default_agent_creation_avoids_double_agent_prefix() {
    let root = agent_create_temp_root();
    init_git_repo(root.path());

    let first = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"role":"analyst"}"#,
        &agent_create_ok_snapshot(),
    )
    .expect("create first agent");
    assert_eq!(first.status, 200);

    let second = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"role":"analyst"}"#,
        &agent_create_ok_snapshot(),
    )
    .expect("create second agent");
    assert_eq!(second.status, 200);

    let second_agent_id = clean_text(
        second
            .payload
            .get("agent_id")
            .and_then(Value::as_str)
            .unwrap_or(""),
        180,
    );
    let second_name = clean_text(
        second
            .payload
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or(""),
        120,
    );
    assert!(
        !second_agent_id.starts_with("agent-agent-"),
        "agent_id should not duplicate the agent prefix: {second_agent_id}"
    );
    assert!(
        !second_name.starts_with("agent-agent-"),
        "display name should not duplicate the agent prefix: {second_name}"
    );
    assert!(
        second_agent_id.starts_with("agent-"),
        "agent_id should keep the default agent prefix: {second_agent_id}"
    );
    assert_eq!(second_name, second_agent_id);
}

#[test]
fn permanent_lifespan_agents_do_not_auto_expire() {
    let root = agent_create_temp_root();
    init_git_repo(root.path());
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"Permanent","role":"analyst","contract":{"lifespan":"permanent"}}"#,
        &agent_create_ok_snapshot(),
    )
    .expect("create permanent agent");
    assert_eq!(created.status, 200);
    let agent_id = clean_text(
        created
            .payload
            .get("agent_id")
            .and_then(Value::as_str)
            .unwrap_or(""),
        180,
    );
    assert!(!agent_id.is_empty());

    let details = handle(
        root.path(),
        "GET",
        &format!("/api/agents/{agent_id}"),
        &[],
        &agent_create_ok_snapshot(),
    )
    .expect("agent details");
    assert_eq!(
        details
            .payload
            .pointer("/contract/termination_condition")
            .and_then(Value::as_str)
            .map(|value| value.to_ascii_lowercase()),
        Some("manual".to_string())
    );
    assert_eq!(
        details
            .payload
            .pointer("/contract/auto_terminate_allowed")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        details
            .payload
            .pointer("/contract/idle_terminate_allowed")
            .and_then(Value::as_bool),
        Some(false)
    );
    let enforcement = crate::dashboard_agent_state::enforce_expired_contracts(root.path());
    let terminated = enforcement
        .get("terminated")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert!(
        !terminated.iter().any(|row| {
            clean_text(row.get("agent_id").and_then(Value::as_str).unwrap_or(""), 180) == agent_id
        }),
        "permanent agent should not be terminated by expiry enforcement"
    );
}

#[test]
fn identity_hydration_prompt_uses_agent_metadata() {
    let row = json!({
        "id": "agent-lucas",
        "name": "Lucas",
        "role": "engineer",
        "identity": {
            "archetype": "coder",
            "vibe": "friendly"
        },
        "system_prompt": "Stay practical and calm. Keep responses concise."
    });
    let prompt = agent_identity_hydration_prompt(&row);
    assert!(prompt.contains("name=Lucas"), "prompt should carry agent name");
    assert!(
        prompt.contains("role=engineer"),
        "prompt should carry agent role"
    );
    assert!(
        prompt.contains("archetype=coder"),
        "prompt should carry agent archetype"
    );
    assert!(prompt.contains("vibe=friendly"), "prompt should carry agent vibe");
    assert!(
        prompt.contains("Personality directive: Stay practical and calm."),
        "prompt should carry a brief personality hydration sentence"
    );
}
