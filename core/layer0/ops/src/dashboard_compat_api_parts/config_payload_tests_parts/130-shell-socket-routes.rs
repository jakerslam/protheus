// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)
fn shell_socket_fixture_root() -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("tempdir");
    write_json(
        &state_path(root.path(), AGENT_PROFILES_REL),
        &json!({
            "agents": {
                "probe": {
                    "agent_id": "probe",
                    "name": "Probe",
                    "role": "analyst",
                    "state": "Running",
                    "updated_at": "2026-05-02T00:00:00Z"
                }
            }
        }),
    );
    save_session_state(
        root.path(),
        "probe",
        &json!({
            "agent_id": "probe",
            "active_session_id": "default",
            "sessions": [{
                "session_id": "default",
                "label": "Session",
                "created_at": "2026-05-02T00:00:00Z",
                "updated_at": "2026-05-02T00:01:00Z",
                "messages": [
                    {"id": "m1", "role": "user", "text": "hello socket", "ts": "2026-05-02T00:00:01Z"},
                    {"id": "m2", "role": "assistant", "text": "hello projection", "ts": "2026-05-02T00:00:02Z"}
                ]
            }]
        }),
    );
    root
}

fn contains_forbidden_socket_field(value: &Value) -> bool {
    match value {
        Value::Object(map) => map.iter().any(|(key, child)| {
            matches!(
                key.as_str(),
                "raw" | "root" | "raw_payload" | "raw_runtime_state" | "all_messages" | "conversation_tree" | "tool_result" | "trace_body" | "workflow_graph"
            ) || contains_forbidden_socket_field(child)
        }),
        Value::Array(rows) => rows.iter().any(contains_forbidden_socket_field),
        _ => false,
    }
}

#[test]
fn shell_socket_read_routes_return_bounded_gateway_projections() {
    let root = shell_socket_fixture_root();
    let snapshot = json!({"ok": true, "receipt_hash": "receipt-a"});
    let status = handle_with_headers(
        root.path(),
        "GET",
        "/api/shell-socket/runtime-status",
        &[],
        &[("Host", "127.0.0.1:4173")],
        &snapshot,
    )
    .expect("status");
    assert_eq!(status.status, 200);
    assert_eq!(status.payload.get("state").and_then(Value::as_str), Some("ready"));

    let agents = handle(
        root.path(),
        "GET",
        "/api/shell-socket/agents?limit=10",
        &[],
        &snapshot,
    )
    .expect("agents");
    assert!(agents.payload.get("agents").and_then(Value::as_array).unwrap().len() <= 10);
    assert!(!contains_forbidden_socket_field(&agents.payload));

    let sessions = handle(
        root.path(),
        "GET",
        "/api/shell-socket/agents/probe/sessions?limit=10",
        &[],
        &snapshot,
    )
    .expect("sessions");
    assert_eq!(
        sessions.payload.get("active_session_id").and_then(Value::as_str),
        Some("probe::default")
    );
    assert!(!contains_forbidden_socket_field(&sessions.payload));

    let messages = handle(
        root.path(),
        "GET",
        "/api/shell-socket/sessions/probe%3A%3Adefault/messages?limit=1",
        &[],
        &snapshot,
    )
    .expect("messages");
    assert_eq!(messages.payload.get("total_count").and_then(Value::as_i64), Some(2));
    assert_eq!(
        messages
            .payload
            .pointer("/message_window/rows")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(1)
    );
    assert!(!contains_forbidden_socket_field(&messages.payload));

    let detail = handle(
        root.path(),
        "GET",
        "/api/shell-socket/details/%2Fapi%2Fagents%2Fprobe%2Fdetails%2Fmessage%2Fm1?view=summary",
        &[],
        &snapshot,
    )
    .expect("detail");
    assert_eq!(detail.payload.get("detail_id").and_then(Value::as_str), Some("m1"));
    assert!(detail.payload.get("detail_projection").is_some());
}

#[test]
fn shell_socket_ingress_routes_fail_closed_when_required_fields_are_missing() {
    let root = shell_socket_fixture_root();
    let rejected = handle(
        root.path(),
        "POST",
        "/api/shell-socket/input",
        br#"{"message":"missing agent"}"#,
        &json!({"ok": true}),
    )
    .expect("submit input");
    assert_eq!(rejected.status, 400);
    assert_eq!(rejected.payload.get("accepted").and_then(Value::as_bool), Some(false));
    assert_eq!(rejected.payload.get("rejected").and_then(Value::as_bool), Some(true));
    assert_eq!(
        rejected.payload.get("reason_code").and_then(Value::as_str),
        Some("agent_id_and_message_required")
    );
}
