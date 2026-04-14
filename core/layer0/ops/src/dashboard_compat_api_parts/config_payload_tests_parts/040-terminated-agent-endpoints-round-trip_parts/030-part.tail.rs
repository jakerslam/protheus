#[test]
fn direct_web_search_post_endpoint_emits_nexus_connection_metadata() {
    let _guard = WEB_ENDPOINT_ENV_MUTEX.lock().expect("lock");
    std::env::remove_var("PROTHEUS_HIERARCHICAL_NEXUS_BLOCK_CLIENT_INGRESS_ROUTE");
    let root = terminated_temp_root();
    init_git_repo(root.path());

    let out = handle(
        root.path(),
        "POST",
        "/api/web/search",
        br#"{"query":""}"#,
        &terminated_ok_snapshot(),
    )
    .expect("web search post");
    assert_eq!(out.status, 400);
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
}

#[test]
fn direct_web_fetch_post_endpoint_emits_nexus_connection_metadata() {
    let _guard = WEB_ENDPOINT_ENV_MUTEX.lock().expect("lock");
    std::env::remove_var("PROTHEUS_HIERARCHICAL_NEXUS_BLOCK_CLIENT_INGRESS_ROUTE");
    let root = terminated_temp_root();
    init_git_repo(root.path());

    let out = handle(
        root.path(),
        "POST",
        "/api/web/fetch",
        br#"{"url":""}"#,
        &terminated_ok_snapshot(),
    )
    .expect("web fetch post");
    assert_eq!(out.status, 400);
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
}
