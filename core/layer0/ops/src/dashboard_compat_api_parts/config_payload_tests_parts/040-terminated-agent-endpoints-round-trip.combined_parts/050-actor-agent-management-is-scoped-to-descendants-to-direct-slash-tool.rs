
#[test]
fn actor_agent_management_is_scoped_to_descendants() {
    let root = terminated_temp_root();
    init_git_repo(root.path());

    let parent = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"Parent","role":"director"}"#,
        &terminated_ok_snapshot(),
    )
    .expect("create parent");
    let parent_id = clean_text(
        parent
            .payload
            .get("agent_id")
            .and_then(Value::as_str)
            .unwrap_or(""),
        180,
    );
    assert!(!parent_id.is_empty());

    let child = handle(
        root.path(),
        "POST",
        "/api/agents",
        format!(
            "{{\"name\":\"Child\",\"role\":\"analyst\",\"parent_agent_id\":\"{}\"}}",
            parent_id
        )
        .as_bytes(),
        &terminated_ok_snapshot(),
    )
    .expect("create child");
    let child_id = clean_text(
        child
            .payload
            .get("agent_id")
            .and_then(Value::as_str)
            .unwrap_or(""),
        180,
    );
    assert!(!child_id.is_empty());

    let sibling = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"Sibling","role":"analyst"}"#,
        &terminated_ok_snapshot(),
    )
    .expect("create sibling");
    let sibling_id = clean_text(
        sibling
            .payload
            .get("agent_id")
            .and_then(Value::as_str)
            .unwrap_or(""),
        180,
    );
    assert!(!sibling_id.is_empty());

    let allowed = handle_with_headers(
        root.path(),
        "POST",
        &format!("/api/agents/{child_id}/stop"),
        br#"{}"#,
        &[("X-Actor-Agent-Id", parent_id.as_str())],
        &terminated_ok_snapshot(),
    )
    .expect("parent manages child");
    assert_eq!(allowed.status, 200);
    assert_eq!(
        allowed.payload.get("ok").and_then(Value::as_bool),
        Some(true)
    );

    let denied = handle_with_headers(
        root.path(),
        "POST",
        &format!("/api/agents/{child_id}/start"),
        br#"{}"#,
        &[("X-Actor-Agent-Id", sibling_id.as_str())],
        &terminated_ok_snapshot(),
    )
    .expect("sibling denied");
    assert_eq!(denied.status, 403);
    assert_eq!(
        denied.payload.get("error").and_then(Value::as_str),
        Some("agent_manage_forbidden")
    );
}

#[test]
fn lineage_agents_can_message_parent_and_child_without_manage_rights() {
    let root = terminated_temp_root();
    init_git_repo(root.path());

    let parent = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"Parent","role":"director"}"#,
        &terminated_ok_snapshot(),
    )
    .expect("create parent");
    let parent_id = clean_text(
        parent
            .payload
            .get("agent_id")
            .and_then(Value::as_str)
            .unwrap_or(""),
        180,
    );
    let child = handle(
        root.path(),
        "POST",
        "/api/agents",
        format!(
            "{{\"name\":\"Child\",\"role\":\"analyst\",\"parent_agent_id\":\"{}\"}}",
            parent_id
        )
        .as_bytes(),
        &terminated_ok_snapshot(),
    )
    .expect("create child");
    let child_id = clean_text(
        child.payload.get("agent_id").and_then(Value::as_str).unwrap_or(""),
        180,
    );

    let child_to_parent = handle_with_headers(
        root.path(),
        "POST",
        &format!("/api/agents/{parent_id}/message"),
        br#"{"message":"status update"}"#,
        &[("X-Actor-Agent-Id", child_id.as_str())],
        &terminated_ok_snapshot(),
    )
    .expect("child messages parent");
    assert_eq!(child_to_parent.status, 200);

    let parent_to_child = handle_with_headers(
        root.path(),
        "POST",
        &format!("/api/agents/{child_id}/message"),
        br#"{"message":"new directive"}"#,
        &[("X-Actor-Agent-Id", parent_id.as_str())],
        &terminated_ok_snapshot(),
    )
    .expect("parent messages child");
    assert_eq!(parent_to_child.status, 200);
}

#[test]
fn direct_slash_tool_routes_through_agent_message() {
    let root = terminated_temp_root();
    init_git_repo(root.path());
    let _ = fs::create_dir_all(root.path().join("notes"));
    let _ = fs::write(root.path().join("notes/plan.txt"), "ship it");

    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"Tool Agent","role":"operator"}"#,
        &terminated_ok_snapshot(),
    )
    .expect("create");
    let agent_id = clean_text(
        created
            .payload
            .get("agent_id")
            .and_then(Value::as_str)
            .unwrap_or(""),
        180,
    );
    assert!(!agent_id.is_empty());

    let out = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"/file notes/plan.txt"}"#,
        &terminated_ok_snapshot(),
    )
    .expect("slash tool");
    assert_eq!(out.status, 200);
    assert_eq!(out.payload.get("ok").and_then(Value::as_bool), Some(true));
    assert!(out
        .payload
        .get("response")
        .and_then(Value::as_str)
        .unwrap_or("")
        .contains("ship it"));
    assert!(out
        .payload
        .get("tools")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter().any(|row| {
                row.get("name")
                    .and_then(Value::as_str)
                    .map(|name| name == "file_read")
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false));
}
