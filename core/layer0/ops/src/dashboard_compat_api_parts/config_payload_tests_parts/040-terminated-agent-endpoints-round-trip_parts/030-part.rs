#[test]
fn active_collab_agent_is_not_hidden_by_stale_terminated_contract() {
    let root = terminated_temp_root();
    let _ = crate::dashboard_agent_state::upsert_profile(
        root.path(),
        "agent-live",
        &json!({"name":"Jarvis","role":"analyst","updated_at":"2026-03-28T00:00:00Z"}),
    );
    let _ = crate::dashboard_agent_state::upsert_contract(
        root.path(),
        "agent-live",
        &json!({
            "status": "terminated",
            "created_at": "2026-03-28T00:00:00Z",
            "updated_at": "2026-03-28T00:00:00Z"
        }),
    );
    let snapshot = json!({
        "ok": true,
        "collab": {
            "dashboard": {
                "agents": [
                    {
                        "shadow": "agent-live",
                        "status": "active",
                        "role": "analyst",
                        "activated_at": "2026-03-29T00:00:00Z"
                    }
                ]
            }
        }
    });

    let listed = handle(root.path(), "GET", "/api/agents", &[], &snapshot).expect("agents");
    assert!(listed
        .payload
        .as_array()
        .map(|rows| {
            rows.iter().any(|row| {
                row.get("id")
                    .and_then(Value::as_str)
                    .map(|value| value == "agent-live")
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false));
}

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

#[test]
fn direct_file_read_endpoint_emits_nexus_connection_metadata() {
    let root = terminated_temp_root();
    init_git_repo(root.path());
    let _ = fs::create_dir_all(root.path().join("notes"));
    let _ = fs::write(root.path().join("notes/plan.txt"), "ship it");

    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"File Agent","role":"operator"}"#,
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
        &format!("/api/agents/{agent_id}/file/read"),
        br#"{"path":"notes/plan.txt"}"#,
        &terminated_ok_snapshot(),
    )
    .expect("file read");
    assert_eq!(out.status, 200);
    assert_eq!(out.payload.get("ok").and_then(Value::as_bool), Some(true));
    assert_eq!(
        out.payload.pointer("/file/content").and_then(Value::as_str),
        Some("ship it")
    );
    assert_eq!(
        out.payload.pointer("/nexus_connection/source")
            .and_then(Value::as_str),
        Some("client_ingress")
    );
    assert_eq!(
        out.payload.pointer("/nexus_connection/target")
            .and_then(Value::as_str),
        Some("context_stacks")
    );
    assert_eq!(
        out.payload
            .pointer("/tool_pipeline/normalized_result/tool_name")
            .and_then(Value::as_str),
        Some("file_read")
    );
    assert_eq!(
        out.payload
            .pointer("/tool_pipeline/worker_output/status")
            .and_then(Value::as_str),
        Some("completed")
    );
}

#[test]
fn direct_file_read_many_endpoint_emits_nexus_connection_metadata() {
    let root = terminated_temp_root();
    init_git_repo(root.path());
    let _ = fs::create_dir_all(root.path().join("notes"));
    let _ = fs::write(root.path().join("notes/plan-a.txt"), "a");
    let _ = fs::write(root.path().join("notes/plan-b.txt"), "b");

    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"File Agent 2","role":"operator"}"#,
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
        &format!("/api/agents/{agent_id}/file/read-many"),
        br#"{"paths":["notes/plan-a.txt","notes/plan-b.txt"]}"#,
        &terminated_ok_snapshot(),
    )
    .expect("file read many");
    assert_eq!(out.status, 200);
    assert_eq!(out.payload.get("ok").and_then(Value::as_bool), Some(true));
    assert_eq!(
        out.payload
            .pointer("/counts/ok")
            .and_then(Value::as_u64),
        Some(2)
    );
    assert_eq!(
        out.payload.pointer("/nexus_connection/source")
            .and_then(Value::as_str),
        Some("client_ingress")
    );
    assert_eq!(
        out.payload.pointer("/nexus_connection/target")
            .and_then(Value::as_str),
        Some("context_stacks")
    );
    assert_eq!(
        out.payload
            .pointer("/tool_pipeline/normalized_result/tool_name")
            .and_then(Value::as_str),
        Some("file_read_many")
    );
}

#[test]
fn direct_search_slash_routes_through_web_search_tool() {
    let root = terminated_temp_root();
    init_git_repo(root.path());

    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"Search Agent","role":"researcher"}"#,
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
        br#"{"message":"/search infringing runtime architecture"}"#,
        &terminated_ok_snapshot(),
    )
    .expect("search slash tool");
    assert!(matches!(out.status, 200 | 400));
    assert!(out
        .payload
        .get("tools")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter().any(|row| {
                row.get("name")
                    .and_then(Value::as_str)
                    .map(|name| name == "web_search" || name == "batch_query")
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false));
}

#[test]
fn direct_web_search_get_endpoint_emits_nexus_connection_metadata() {
    let _guard = WEB_ENDPOINT_ENV_MUTEX.lock().expect("lock");
    std::env::remove_var("INFRING_HIERARCHICAL_NEXUS_BLOCK_CLIENT_INGRESS_ROUTE");
    let root = terminated_temp_root();
    init_git_repo(root.path());

    let out = handle(
        root.path(),
        "GET",
        "/api/web/search?q=runtime",
        &[],
        &terminated_ok_snapshot(),
    )
    .expect("web search get");
    assert_eq!(out.status, 200);
    assert_eq!(
        out.payload.pointer("/nexus_connection/source")
            .and_then(Value::as_str),
        Some("client_ingress")
    );
    assert_eq!(
        out.payload.pointer("/nexus_connection/target")
            .and_then(Value::as_str),
        Some("context_stacks")
    );
    assert_eq!(
        out.payload
            .pointer("/tool_pipeline/normalized_result/tool_name")
            .and_then(Value::as_str),
        Some("web_search")
    );
}

include!("030-part.tail.rs");
