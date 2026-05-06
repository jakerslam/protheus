
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
