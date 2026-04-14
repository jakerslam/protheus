
#[test]
fn conversation_search_includes_archived_agents() {
    let root = tempfile::tempdir().expect("tempdir");
    let _ = crate::dashboard_agent_state::upsert_profile(
        root.path(),
        "agent-search-a",
        &json!({
            "name": "Search Atlas",
            "identity": {"emoji": "🛰️"}
        }),
    );
    let _ = crate::dashboard_agent_state::append_turn(
        root.path(),
        "agent-search-a",
        "Please patch websocket reconnect jitter and bottom scroll bounce",
        "I can patch reconnect jitter and scroll bounce.",
    );
    let _ = crate::dashboard_agent_state::archive_agent(root.path(), "agent-search-a", "test");
    let _ = crate::dashboard_agent_state::upsert_contract(
        root.path(),
        "agent-search-a",
        &json!({"status":"terminated","termination_reason":"user_archive"}),
    );

    let response = handle(
        root.path(),
        "GET",
        "/api/search/conversations?q=reconnect%20jitter&limit=5",
        &[],
        &json!({"ok": true}),
    )
    .expect("search response");

    assert_eq!(response.status, 200);
    let rows = response
        .payload
        .get("results")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert!(!rows.is_empty());
    let row = rows.first().cloned().unwrap_or_else(|| json!({}));
    assert_eq!(
        row.get("agent_id").and_then(Value::as_str),
        Some("agent-search-a")
    );
    assert_eq!(row.get("archived").and_then(Value::as_bool), Some(true));
}

#[test]
fn channels_endpoint_exposes_transport_contract_metadata() {
    let root = tempfile::tempdir().expect("tempdir");
    let response = handle(
        root.path(),
        "GET",
        "/api/channels",
        &[],
        &json!({"ok": true}),
    )
    .expect("channels");
    assert_eq!(response.status, 200);
    let rows = response
        .payload
        .get("channels")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert!(!rows.is_empty());

    let webchat = rows
        .iter()
        .find(|row| row.get("name").and_then(Value::as_str) == Some("webchat"))
        .cloned()
        .unwrap_or_else(|| json!({}));
    assert_eq!(
        webchat.get("transport_kind").and_then(Value::as_str),
        Some("internal")
    );
    assert_eq!(
        webchat
            .get("external_network_required")
            .and_then(Value::as_bool),
        Some(false)
    );

    let webhook = rows
        .iter()
        .find(|row| row.get("name").and_then(Value::as_str) == Some("slack_webhook"))
        .cloned()
        .unwrap_or_else(|| json!({}));
    assert_eq!(
        webhook.get("transport_kind").and_then(Value::as_str),
        Some("webhook")
    );
    assert_eq!(
        webhook
            .get("external_network_required")
            .and_then(Value::as_bool),
        Some(true)
    );
}
