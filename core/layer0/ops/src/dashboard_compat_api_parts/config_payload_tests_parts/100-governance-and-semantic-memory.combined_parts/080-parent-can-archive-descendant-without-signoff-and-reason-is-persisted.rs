fn parent_can_archive_descendant_without_signoff_and_reason_is_persisted() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();

    let parent_create = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"parent-gate-test","role":"operator"}"#,
        &snapshot,
    )
    .expect("create parent");
    let parent_id = clean_agent_id(
        parent_create
            .payload
            .get("agent_id")
            .or_else(|| parent_create.payload.get("id"))
            .and_then(Value::as_str)
            .unwrap_or(""),
    );
    assert!(!parent_id.is_empty());

    let child_payload = serde_json::to_vec(&json!({
        "name": "child-gate-test",
        "role": "analyst",
        "parent_agent_id": parent_id
    }))
    .expect("serialize child create");
    let child_create = handle(
        root.path(),
        "POST",
        "/api/agents",
        &child_payload,
        &snapshot,
    )
    .expect("create child");
    let child_id = clean_agent_id(
        child_create
            .payload
            .get("agent_id")
            .or_else(|| child_create.payload.get("id"))
            .and_then(Value::as_str)
            .unwrap_or(""),
    );
    assert!(!child_id.is_empty());

    let archived = execute_tool_call_by_name(
        root.path(),
        &snapshot,
        &parent_id,
        None,
        "manage_agent",
        &json!({"action":"archive", "agent_id": child_id}),
    );
    assert_eq!(archived.get("ok").and_then(Value::as_bool), Some(true));
    assert_ne!(
        archived.get("error").and_then(Value::as_str),
        Some("tool_explicit_signoff_required")
    );
    assert_eq!(
        archived.get("reason").and_then(Value::as_str),
        Some("Archived by parent agent")
    );

    let archived_state = read_json_loose(
        &root
            .path()
            .join("client/runtime/local/state/ui/infring_dashboard/archived_agents.json"),
    )
    .unwrap_or_else(|| json!({}));
    let reason = archived_state
        .pointer(&format!("/agents/{child_id}/reason"))
        .and_then(Value::as_str)
        .unwrap_or("");
    assert_eq!(reason, "Archived by parent agent");

    let blocked = execute_tool_call_by_name(
        root.path(),
        &snapshot,
        "agent-unrelated",
        None,
        "manage_agent",
        &json!({"action":"archive", "agent_id": child_id}),
    );
    assert_eq!(
        blocked.get("error").and_then(Value::as_str),
        Some("tool_explicit_signoff_required")
    );
}

#[test]
