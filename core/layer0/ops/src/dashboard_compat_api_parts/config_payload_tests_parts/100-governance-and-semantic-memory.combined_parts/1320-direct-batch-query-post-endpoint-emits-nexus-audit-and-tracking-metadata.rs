fn direct_batch_query_post_endpoint_emits_nexus_audit_and_tracking_metadata() {
    let _guard = WEB_ENDPOINT_ENV_MUTEX.lock().expect("lock");
    std::env::remove_var("PROTHEUS_HIERARCHICAL_NEXUS_BLOCK_CLIENT_INGRESS_ROUTE");
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let out = handle(
        root.path(),
        "POST",
        "/api/batch-query",
        br#"{"query":""}"#,
        &snapshot,
    )
    .expect("batch query post");
    assert!(matches!(out.status, 200 | 400));
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

#[test]
