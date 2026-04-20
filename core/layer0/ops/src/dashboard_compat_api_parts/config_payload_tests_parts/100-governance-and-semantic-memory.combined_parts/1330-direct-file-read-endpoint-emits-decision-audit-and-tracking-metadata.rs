fn direct_file_read_endpoint_emits_decision_audit_and_tracking_metadata() {
    let root = governance_temp_root();
    init_git_repo(root.path());
    std::fs::create_dir_all(root.path().join("notes")).expect("mkdir");
    std::fs::write(root.path().join("notes/plan.txt"), "ship it").expect("write");
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"File Audit Agent","role":"operator"}"#,
        &governance_ok_snapshot(),
    )
    .expect("create");
    let agent_id = clean_agent_id(
        created
            .payload
            .get("agent_id")
            .and_then(Value::as_str)
            .unwrap_or(""),
    );
    assert!(!agent_id.is_empty());
    let out = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/file/read"),
        br#"{"path":"notes/plan.txt"}"#,
        &governance_ok_snapshot(),
    )
    .expect("file read");
    assert_eq!(out.status, 200);
    assert_eq!(
        out.payload.pointer("/nexus_connection/source")
            .and_then(Value::as_str),
        Some("client_ingress")
    );
    assert!(out.payload.get("decision_audit_receipt").is_some());
    assert!(out.payload.get("turn_loop_tracking").is_some());
    assert_eq!(
        out.payload
            .pointer("/recovery_strategy")
            .and_then(Value::as_str),
        Some("none")
    );
}
