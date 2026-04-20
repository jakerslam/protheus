fn transient_tool_failure_detects_request_read_failed_signature() {
    assert!(transient_tool_failure(&json!({
        "ok": false,
        "error": "request_read_failed:Resource temporarily unavailable (os error 35)"
    })));
}

#[test]
