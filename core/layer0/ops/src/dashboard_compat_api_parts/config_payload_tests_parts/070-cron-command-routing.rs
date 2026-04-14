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

#[test]
fn comms_tasks_endpoint_supports_status_filter_and_summary() {
    let root = tempfile::tempdir().expect("tempdir");
    let snapshot = json!({"ok": true});
    let created = handle(
        root.path(),
        "POST",
        "/api/comms/task",
        br#"{"title":"Ship web tooling contract","description":"run queued wave"}"#,
        &snapshot,
    )
    .expect("create task");
    assert_eq!(created.status, 200);
    let task_id = created
        .payload
        .pointer("/task/id")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    assert!(!task_id.is_empty());

    let complete = handle(
        root.path(),
        "POST",
        &format!("/api/comms/task/{task_id}/complete"),
        br#"{"result_summary":"done"}"#,
        &snapshot,
    )
    .expect("complete task");
    assert_eq!(complete.status, 200);

    let filtered = handle(
        root.path(),
        "GET",
        "/api/comms/tasks?status=completed&limit=10",
        &[],
        &snapshot,
    )
    .expect("filtered tasks");
    assert_eq!(filtered.status, 200);
    assert_eq!(
        filtered
            .payload
            .get("contract_version")
            .and_then(Value::as_str),
        Some("dashboard_comms_v1")
    );
    let rows = filtered
        .payload
        .get("tasks")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert!(!rows.is_empty());
    assert!(rows.iter().all(|row| {
        row.get("status")
            .and_then(Value::as_str)
            .map(|status| status.eq_ignore_ascii_case("completed"))
            .unwrap_or(false)
    }));
    assert_eq!(
        filtered
            .payload
            .get("total_count")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            >= rows.len() as u64,
        true
    );
}
