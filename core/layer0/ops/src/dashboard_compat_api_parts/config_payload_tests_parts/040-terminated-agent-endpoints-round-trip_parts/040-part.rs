#[test]
fn direct_web_search_endpoint_fails_closed_when_ingress_route_pair_blocked() {
    let _guard = WEB_ENDPOINT_ENV_MUTEX.lock().expect("lock");
    std::env::set_var("PROTHEUS_HIERARCHICAL_NEXUS_BLOCK_CLIENT_INGRESS_ROUTE", "1");
    let root = terminated_temp_root();
    init_git_repo(root.path());

    let out = handle(
        root.path(),
        "POST",
        "/api/web/search",
        br#"{"query":"nexus deny check"}"#,
        &terminated_ok_snapshot(),
    )
    .expect("web search denied");
    std::env::remove_var("PROTHEUS_HIERARCHICAL_NEXUS_BLOCK_CLIENT_INGRESS_ROUTE");
    assert_eq!(out.status, 403);
    assert_eq!(
        out.payload.get("error").and_then(Value::as_str),
        Some("web_search_nexus_delivery_denied")
    );
}

#[test]
fn natural_language_web_search_intent_routes_without_slash() {
    let root = terminated_temp_root();
    init_git_repo(root.path());

    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"Search Agent 2","role":"researcher"}"#,
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
        br#"{"message":"search the web for robust websocket reconnect patterns"}"#,
        &terminated_ok_snapshot(),
    )
    .expect("natural search tool");
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
    let response = out
        .payload
        .get("response")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_ascii_lowercase();
    assert!(!response.contains("couldn't extract usable findings"));
    assert!(!response.contains("search response came from"));
}

#[test]
fn message_turns_capture_memory_and_attention_receipt() {
    let root = terminated_temp_root();
    init_git_repo(root.path());

    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"Memory Agent","role":"analyst"}"#,
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
        br#"{"message":"Remember this exactly: launch codename is aurora-7"}"#,
        &terminated_ok_snapshot(),
    )
    .expect("message");
    assert_eq!(out.status, 200);
    let memory_capture_ok = out
        .payload
        .pointer("/memory_capture/ok")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    assert!(memory_capture_ok);
    let queued = out
        .payload
        .pointer("/attention_queue/queued")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let staged = out
        .payload
        .pointer("/attention_queue/staged")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    assert!(queued || staged);
}
