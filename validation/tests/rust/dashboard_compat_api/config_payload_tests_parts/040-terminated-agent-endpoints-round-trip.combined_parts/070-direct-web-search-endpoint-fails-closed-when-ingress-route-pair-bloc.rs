
#[test]
fn direct_web_search_endpoint_fails_closed_when_ingress_route_pair_blocked() {
    let _guard = WEB_ENDPOINT_ENV_MUTEX.lock().expect("lock");
    std::env::set_var("INFRING_HIERARCHICAL_NEXUS_BLOCK_CLIENT_INGRESS_ROUTE", "1");
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
    std::env::remove_var("INFRING_HIERARCHICAL_NEXUS_BLOCK_CLIENT_INGRESS_ROUTE");
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

#[test]
fn direct_web_media_host_post_and_get_delivery_route_round_trip() {
    let root = terminated_temp_root();
    init_git_repo(root.path());
    let target = root.path().join("tiny.png");
    let png = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/woAAn8B9FD5fHAAAAAASUVORK5CYII=",
    )
    .expect("png");
    fs::write(&target, png).expect("write");

    let hosted = handle(
        root.path(),
        "POST",
        "/api/web/media-host",
        format!(
            "{{\"path\":\"{}\",\"local_roots\":\"any\"}}",
            target.display()
        )
        .as_bytes(),
        &terminated_ok_snapshot(),
    )
    .expect("host");
    assert_eq!(hosted.status, 200);
    let hosted_id = clean_text(
        hosted.payload.get("id").and_then(Value::as_str).unwrap_or(""),
        220,
    );
    assert!(!hosted_id.is_empty());

    let delivered = handle(
        root.path(),
        "GET",
        &format!("/api/web/media/{hosted_id}"),
        &[],
        &terminated_ok_snapshot(),
    )
    .expect("deliver");
    assert_eq!(delivered.status, 200);
    assert_eq!(
        delivered.payload.get("content_type").and_then(Value::as_str),
        Some("image/png")
    );
    assert!(delivered
        .payload
        .get("data_url")
        .and_then(Value::as_str)
        .unwrap_or("")
        .starts_with("data:image/png;base64,"));

    let removed = handle(
        root.path(),
        "GET",
        &format!("/api/web/media/{hosted_id}"),
        &[],
        &terminated_ok_snapshot(),
    )
    .expect("removed");
    assert_eq!(removed.status, 404);
}

