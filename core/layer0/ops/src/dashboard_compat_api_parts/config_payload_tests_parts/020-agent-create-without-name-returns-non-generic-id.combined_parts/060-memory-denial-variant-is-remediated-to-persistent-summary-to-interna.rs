
#[test]
fn memory_denial_variant_is_remediated_to_persistent_summary() {
    let root = agent_create_temp_root();
    init_git_repo(root.path());
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"Memory Probe","role":"analyst"}"#,
        &agent_create_ok_snapshot(),
    )
    .expect("create agent");
    let agent_id = clean_text(
        created
            .payload
            .get("agent_id")
            .and_then(Value::as_str)
            .unwrap_or(""),
        180,
    );
    assert!(!agent_id.is_empty());

    let seeded = handle(
            root.path(),
            "POST",
            &format!("/api/agents/{agent_id}/message"),
            br#"{"message":"Remember this exactly: favorite animal is octopus and codename aurora-7."}"#,
            &agent_create_ok_snapshot(),
        )
        .expect("seed memory");
    assert_eq!(seeded.status, 200);

    let second = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/sessions"),
        br#"{"label":"Session 2"}"#,
        &agent_create_ok_snapshot(),
    )
    .expect("create second session");
    let sid = clean_text(
        second
            .payload
            .get("active_session_id")
            .and_then(Value::as_str)
            .unwrap_or(""),
        180,
    );
    assert!(!sid.is_empty());
    let switched = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/sessions/{sid}/switch"),
        &[],
        &agent_create_ok_snapshot(),
    )
    .expect("switch second session");
    assert_eq!(switched.status, 200);

    let denial_variant = handle(
            root.path(),
            "POST",
            &format!("/api/agents/{agent_id}/message"),
            br#"{"message":"I still do not see any stored memory context from earlier in this session. I do not retain information between exchanges unless you explicitly use a memory conduit, and I can only work with what is in the current message."}"#,
            &agent_create_ok_snapshot(),
        )
        .expect("denial variant message");
    assert_eq!(denial_variant.status, 200);
    let response = denial_variant
        .payload
        .get("response")
        .and_then(Value::as_str)
        .unwrap_or("");
    assert!(
        response.contains("Here's what I remember from earlier:")
            || response.contains("octopus")
            || response.contains("aurora-7"),
        "memory denial variant should be remediated to user-facing memory recall"
    );
    assert!(
        !response
            .to_ascii_lowercase()
            .contains("do not retain information between exchanges"),
        "raw denial text should not leak back to caller"
    );
}

#[test]
fn internal_recalled_context_metadata_is_not_echoed_to_user() {
    let root = agent_create_temp_root();
    init_git_repo(root.path());
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"Context Leak Guard","role":"analyst"}"#,
        &agent_create_ok_snapshot(),
    )
    .expect("create agent");
    let agent_id = clean_text(
        created
            .payload
            .get("agent_id")
            .and_then(Value::as_str)
            .unwrap_or(""),
        180,
    );
    assert!(!agent_id.is_empty());

    let message = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"Persistent memory is enabled for this agent across 1 session(s) with 12 stored messages. Recalled context: alpha | beta | gamma"}"#,
        &agent_create_ok_snapshot(),
    )
    .expect("metadata dump probe");
    assert_eq!(message.status, 200);
    let response = message
        .payload
        .get("response")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_ascii_lowercase();
    assert!(
        !response.contains("persistent memory is enabled for this agent across"),
        "internal metadata banner must never be returned as user-visible output"
    );
    assert!(
        !response.contains("recalled context:"),
        "recalled-context scaffolding must never be user-visible output"
    );
}

