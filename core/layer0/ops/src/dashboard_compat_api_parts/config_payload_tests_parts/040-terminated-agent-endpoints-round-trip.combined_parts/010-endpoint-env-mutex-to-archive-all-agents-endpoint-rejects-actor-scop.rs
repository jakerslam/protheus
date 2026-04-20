static WEB_ENDPOINT_ENV_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn terminated_temp_root() -> tempfile::TempDir {
    tempfile::tempdir().expect("tempdir")
}

fn terminated_ok_snapshot() -> Value {
    json!({"ok": true})
}

#[test]
fn terminated_agent_endpoints_round_trip() {
    let root = terminated_temp_root();
    let _ = crate::dashboard_agent_state::upsert_contract(
        root.path(),
        "agent-a",
        &json!({
            "created_at": "2000-01-01T00:00:00Z",
            "expiry_seconds": 1,
            "status": "active"
        }),
    );
    let _ = crate::dashboard_agent_state::enforce_expired_contracts(root.path());

    let listed = handle(
        root.path(),
        "GET",
        "/api/agents/terminated",
        &[],
        &terminated_ok_snapshot(),
    )
    .expect("terminated list");
    let rows = listed
        .payload
        .get("entries")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert!(!rows.is_empty());

    let revived = handle(
        root.path(),
        "POST",
        "/api/agents/agent-a/revive",
        br#"{"role":"analyst"}"#,
        &terminated_ok_snapshot(),
    )
    .expect("revive");
    assert_eq!(
        revived.payload.get("ok").and_then(Value::as_bool),
        Some(true)
    );

    let after_revive = handle(
        root.path(),
        "GET",
        "/api/agents/terminated",
        &[],
        &terminated_ok_snapshot(),
    )
    .expect("terminated list after revive");
    let rows_after = after_revive
        .payload
        .get("entries")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert!(rows_after.is_empty());

    let _ = crate::dashboard_agent_state::upsert_contract(
        root.path(),
        "agent-a",
        &json!({
            "created_at": "2000-01-01T00:00:00Z",
            "expiry_seconds": 1,
            "status": "active"
        }),
    );
    let _ = crate::dashboard_agent_state::enforce_expired_contracts(root.path());
    let deleted = handle(
        root.path(),
        "DELETE",
        "/api/agents/terminated/agent-a",
        &[],
        &terminated_ok_snapshot(),
    )
    .expect("delete terminated");
    assert!(
        deleted
            .payload
            .get("removed_history_entries")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            >= 1
    );
}

#[test]
fn archive_all_agents_endpoint_archives_visible_roster() {
    let root = terminated_temp_root();
    let created_a = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"Alpha","role":"operator"}"#,
        &terminated_ok_snapshot(),
    )
    .expect("create alpha");
    let created_b = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"Beta","role":"analyst"}"#,
        &terminated_ok_snapshot(),
    )
    .expect("create beta");
    let alpha_id = clean_text(
        created_a
            .payload
            .get("agent_id")
            .and_then(Value::as_str)
            .unwrap_or(""),
        180,
    );
    let beta_id = clean_text(
        created_b
            .payload
            .get("agent_id")
            .and_then(Value::as_str)
            .unwrap_or(""),
        180,
    );
    assert!(!alpha_id.is_empty());
    assert!(!beta_id.is_empty());

    let archived = handle(
        root.path(),
        "POST",
        "/api/agents/archive-all",
        br#"{"reason":"test_archive_all"}"#,
        &terminated_ok_snapshot(),
    )
    .expect("archive all");
    assert_eq!(archived.status, 200);
    assert_eq!(
        archived.payload.get("ok").and_then(Value::as_bool),
        Some(true)
    );
    let archived_ids = archived
        .payload
        .get("archived_agent_ids")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let archived_labels: Vec<String> = archived_ids
        .iter()
        .filter_map(|row| row.as_str())
        .map(|row| row.to_string())
        .collect();
    assert!(archived_labels.contains(&alpha_id));
    assert!(archived_labels.contains(&beta_id));

    let listed =
        handle(root.path(), "GET", "/api/agents", &[], &terminated_ok_snapshot()).expect("list active");
    let rows = listed.payload.as_array().cloned().unwrap_or_default();
    assert!(rows
        .iter()
        .all(
            |row| clean_text(row.get("id").and_then(Value::as_str).unwrap_or(""), 180) != alpha_id
        ));
    assert!(rows
        .iter()
        .all(
            |row| clean_text(row.get("id").and_then(Value::as_str).unwrap_or(""), 180) != beta_id
        ));
}

#[test]
fn archive_all_agents_endpoint_rejects_actor_scoped_requests() {
    let root = terminated_temp_root();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"Parent","role":"operator"}"#,
        &terminated_ok_snapshot(),
    )
    .expect("create parent");
    let actor_id = clean_text(
        created
            .payload
            .get("agent_id")
            .and_then(Value::as_str)
            .unwrap_or(""),
        180,
    );
    assert!(!actor_id.is_empty());

    let denied = handle_with_headers(
        root.path(),
        "POST",
        "/api/agents/archive-all",
        br#"{}"#,
        &[("X-Actor-Agent-Id", actor_id.as_str())],
        &terminated_ok_snapshot(),
    )
    .expect("archive-all denied");
    assert_eq!(denied.status, 403);
    assert_eq!(
        denied.payload.get("error").and_then(Value::as_str),
        Some("agent_manage_forbidden")
    );
}
