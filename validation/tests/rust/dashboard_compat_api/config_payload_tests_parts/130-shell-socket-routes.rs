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
                    {
                        "id": "m2",
                        "role": "assistant",
                        "text": "hello projection",
                        "ts": "2026-05-02T00:00:02Z",
                        "tools": [{
                            "name": "probe_tool",
                            "summary": "bounded tool summary",
                            "input": "SECRET_RAW_INPUT_TERM",
                            "result": "SECRET_RAW_RESULT_TERM"
                        }]
                    }
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
                    | "decision_trace" | "execution_observation" | "raw_tool_result" | "raw_tool_input"
            ) || contains_forbidden_socket_field(child)
        }),
        Value::Array(rows) => rows.iter().any(contains_forbidden_socket_field),
        _ => false,
    }
}

fn socket_payload_contains_text(value: &Value, needle: &str) -> bool {
    match value {
        Value::String(text) => text.contains(needle),
        Value::Array(rows) => rows.iter().any(|row| socket_payload_contains_text(row, needle)),
        Value::Object(map) => map.values().any(|child| socket_payload_contains_text(child, needle)),
        _ => false,
    }
}

#[test]
fn shell_socket_search_returns_bounded_message_projections_without_raw_tool_payload_search() {
    let root = shell_socket_fixture_root();
    let snapshot = json!({"ok": true});
    let search = handle(
        root.path(),
        "GET",
        "/api/shell-socket/search?agent_id=probe&q=projection&limit=10",
        &[],
        &snapshot,
    )
    .expect("message search");
    assert_eq!(search.status, 200);
    assert_eq!(
        search.payload.get("type").and_then(Value::as_str),
        Some("shell_socket_message_search_projection_v1")
    );
    assert_eq!(
        search
            .payload
            .get("hits")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(1)
    );
    let hit = search.payload.pointer("/hits/0").expect("hit");
    assert_eq!(hit.get("kind").and_then(Value::as_str), Some("message"));
    assert_eq!(hit.get("message_id").and_then(Value::as_str), Some("m2"));
    assert!(hit.get("detail_ref").and_then(Value::as_str).unwrap_or("").contains("/details/message/m2"));
    assert!(!contains_forbidden_socket_field(&search.payload));

    let raw_input_search = handle(
        root.path(),
        "GET",
        "/api/shell-socket/search?agent_id=probe&q=SECRET_RAW_INPUT_TERM&limit=10",
        &[],
        &snapshot,
    )
    .expect("raw input search");
    assert_eq!(
        raw_input_search
            .payload
            .pointer("/counts/total_hits")
            .and_then(Value::as_u64),
        Some(0)
    );

    let raw_result_search = handle(
        root.path(),
        "GET",
        "/api/shell-socket/search?agent_id=probe&q=SECRET_RAW_RESULT_TERM&limit=10",
        &[],
        &snapshot,
    )
    .expect("raw result search");
    assert_eq!(
        raw_result_search
            .payload
            .pointer("/counts/total_hits")
            .and_then(Value::as_u64),
        Some(0)
    );
}

#[test]
fn shell_socket_default_message_rows_do_not_expose_raw_tool_payloads() {
    let root = shell_socket_fixture_root();
    let snapshot = json!({"ok": true});
    let messages = handle(
        root.path(),
        "GET",
        "/api/shell-socket/sessions/probe%3A%3Adefault/messages?limit=10",
        &[],
        &snapshot,
    )
    .expect("messages");
    assert_eq!(messages.status, 200);
    assert!(!contains_forbidden_socket_field(&messages.payload));
    assert!(
        !socket_payload_contains_text(&messages.payload, "SECRET_RAW_INPUT_TERM"),
        "default shell message rows must not expose raw tool input"
    );
    assert!(
        !socket_payload_contains_text(&messages.payload, "SECRET_RAW_RESULT_TERM"),
        "default shell message rows must not expose raw tool result"
    );
    let detail_ref = messages
        .payload
        .pointer("/message_window/rows/1/tools/0/detail_ref")
        .and_then(Value::as_str)
        .unwrap_or("");
    assert!(detail_ref.starts_with("/api/agents/probe/details/tool-result/"));
    assert!(detail_ref.contains("session_artifact:"));

    let detail_path = format!("/api/shell-socket/details/{}?view=summary", urlencoding::encode(detail_ref));
    let detail = handle(root.path(), "GET", &detail_path, &[], &snapshot).expect("tool detail");
    assert_eq!(detail.status, 200);
    assert!(
        socket_payload_contains_text(&detail.payload, "SECRET_RAW_RESULT_TERM"),
        "raw tool result should be available only through explicit detail fetch"
    );
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

#[test]
fn shell_socket_approval_decision_uses_kernel_approval_queue() {
    let root = shell_socket_fixture_root();
    let queue_path = root
        .path()
        .join("client/runtime/local/state/approvals_queue.yaml");
    std::fs::create_dir_all(queue_path.parent().expect("queue parent")).expect("create queue dir");
    std::fs::write(
        &queue_path,
        r#"pending:
- action_id: socket-approval-1
  timestamp: "2026-05-02T00:00:00Z"
  directive_id: T0_invariants
  type: shell_socket_probe
  summary: Approve socket probe
  reason: needs operator decision
  status: PENDING
  payload_pointer: socket-approval-1
approved: []
denied: []
history: []
"#,
    )
    .expect("write queue");

    let approved = handle(
        root.path(),
        "POST",
        "/api/shell-socket/approvals/socket-approval-1/decision",
        br#"{"decision":"approve"}"#,
        &json!({"ok": true}),
    )
    .expect("approval decision");
    assert_eq!(approved.status, 202);
    assert_eq!(
        approved.payload.get("accepted").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        approved.payload.get("reason_code").and_then(Value::as_str),
        Some("accepted")
    );

    let saved = std::fs::read_to_string(&queue_path).expect("read queue");
    assert!(saved.contains("status: APPROVED"));
    assert!(saved.contains("action: approved"));
}
