#[test]
fn cron_direct_intent_parses_schedule_and_list() {
    let list = direct_tool_intent_from_user_message("/cron list").expect("cron list intent");
    assert_eq!(list.0, "cron_list");

    let schedule = direct_tool_intent_from_user_message("/cron schedule 15m check queue pressure")
        .expect("cron schedule intent");
    assert_eq!(schedule.0, "cron_schedule");
    assert_eq!(
        schedule
            .1
            .get("interval_minutes")
            .and_then(Value::as_i64)
            .unwrap_or(0),
        15
    );
    assert_eq!(
        schedule
            .1
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or(""),
        "check queue pressure"
    );
}

#[test]
fn cron_command_endpoint_schedules_agent_owned_job() {
    let root = tempfile::tempdir().expect("tempdir");
    let snapshot = json!({"ok": true});
    let create = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"Cron Agent","role":"analyst"}"#,
        &snapshot,
    )
    .expect("create agent");
    assert_eq!(create.status, 200);
    let agent_id = create
        .payload
        .get("agent_id")
        .and_then(Value::as_str)
        .unwrap_or("agent-cron")
        .to_string();

    let request = json!({
        "command": "cron",
        "args": "schedule 10m follow up on workflow completion"
    });
    let body = serde_json::to_vec(&request).expect("serialize");

    let response = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/command"),
        &body,
        &snapshot,
    )
    .expect("cron command response");
    assert_eq!(response.status, 200);
    assert_eq!(
        response
            .payload
            .get("ok")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        true
    );
    assert_eq!(
        response
            .payload
            .get("tool")
            .and_then(Value::as_str)
            .unwrap_or(""),
        "cron_schedule"
    );
    assert_eq!(
        response
            .payload
            .pointer("/result/job/agent_id")
            .and_then(Value::as_str)
            .unwrap_or(""),
        agent_id
    );

    let jobs = handle(root.path(), "GET", "/api/cron/jobs", &[], &snapshot).expect("cron jobs");
    assert_eq!(jobs.status, 200);
    let rows = jobs
        .payload
        .get("jobs")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert_eq!(rows.is_empty(), false);
    assert_eq!(
        rows[0]
            .get("agent_id")
            .and_then(Value::as_str)
            .unwrap_or(""),
        agent_id
    );
}
