#[test]
fn receipts_lineage_http_route_reconstructs_chain() {
    let root = tempfile::tempdir().expect("tempdir");
    let task_id = "task-lineage-001";
    let trace_id = "trace-lineage-001";

    let task_receipts = root
        .path()
        .join("local/state/runtime/task_runtime/verity_receipts.jsonl");
    let task_receipt_row = json!({
        "type": "task_verity_receipt",
        "event_type": "task_result",
        "receipt_hash": "r-task",
        "payload": {
            "task_id": task_id,
            "status": "done"
        }
    });
    let task_receipt_raw = serde_json::to_string(&task_receipt_row).expect("encode task row");
    if let Some(parent) = task_receipts.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(&task_receipts, format!("{task_receipt_raw}\n"));

    let actions_history = root
        .path()
        .join("client/runtime/local/state/ui/infring_dashboard/actions/history.jsonl");
    let tool_row = json!({
        "type": "dashboard_tool_result",
        "receipt_hash": "r-tool",
        "payload": {
            "tool_pipeline": {
                "normalized_result": {
                    "result_id": "res-1",
                    "task_id": task_id,
                    "trace_id": trace_id,
                    "tool_name": "web_search"
                },
                "evidence_cards": [{
                    "evidence_id": "ev-1",
                    "task_id": task_id,
                    "trace_id": trace_id,
                    "summary": "retrieved snippet"
                }],
                "claim_bundle": {
                    "task_id": task_id,
                    "claims": [{
                        "claim_id": "claim-1",
                        "text": "supported finding",
                        "evidence_ids": ["ev-1"],
                        "status": "supported"
                    }]
                }
            }
        }
    });
    let tool_row_other_trace = json!({
        "type": "dashboard_tool_result",
        "receipt_hash": "r-tool-other-trace",
        "payload": {
            "tool_pipeline": {
                "normalized_result": {
                    "result_id": "res-2",
                    "task_id": "task-lineage-other",
                    "trace_id": "trace-lineage-other",
                    "tool_name": "web_search"
                },
                "evidence_cards": [{
                    "evidence_id": "ev-2",
                    "task_id": "task-lineage-other",
                    "trace_id": "trace-lineage-other",
                    "summary": "should be filtered by trace_id"
                }]
            }
        }
    });
    let tool_raw = serde_json::to_string(&tool_row).expect("encode tool row");
    let tool_other_raw =
        serde_json::to_string(&tool_row_other_trace).expect("encode tool row other trace");
    if let Some(parent) = actions_history.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(&actions_history, format!("{tool_raw}\n{tool_other_raw}\n"));

    let memory_history = root.path().join("local/state/ops/memory/history.jsonl");
    let memory_row = json!({
        "type": "memory_write",
        "task_id": task_id,
        "receipt_hash": "r-mem",
        "payload": {
            "object_id": "memory-object-1",
            "version_id": "memory-version-1"
        }
    });
    let memory_raw = serde_json::to_string(&memory_row).expect("encode memory row");
    if let Some(parent) = memory_history.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(&memory_history, format!("{memory_raw}\n"));

    let assimilation_steps = root
        .path()
        .join("local/state/ops/runtime_systems/assimilate/protocol_step_receipts.jsonl");
    let assimilation_row = json!({
        "type": "assimilation_protocol_step",
        "task_id": task_id,
        "step_id": "step-1",
        "receipt_hash": "r-assim"
    });
    let assimilation_raw =
        serde_json::to_string(&assimilation_row).expect("encode assimilation row");
    if let Some(parent) = assimilation_steps.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(&assimilation_steps, format!("{assimilation_raw}\n"));

    let response = handle(
        root.path(),
        "GET",
        &format!("/api/receipts/lineage?task_id={task_id}&trace_id={trace_id}&limit=200"),
        &[],
        &json!({"ok": true}),
    )
    .expect("lineage response");
    assert_eq!(response.status, 200);
    assert_eq!(
        response.payload.get("ok").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        response
            .payload
            .pointer("/lineage/task")
            .and_then(Value::as_array)
            .map(|rows| rows.len()),
        Some(1)
    );
    assert_eq!(
        response
            .payload
            .pointer("/lineage/tool_call")
            .and_then(Value::as_array)
            .map(|rows| rows.len()),
        Some(1)
    );
    assert_eq!(
        response
            .payload
            .pointer("/lineage/tool_call/0/trace_id")
            .and_then(Value::as_str),
        Some(trace_id)
    );
    assert_eq!(
        response
            .payload
            .pointer("/lineage/evidence")
            .and_then(Value::as_array)
            .map(|rows| rows.len()),
        Some(1)
    );
    assert_eq!(
        response
            .payload
            .pointer("/lineage/claim")
            .and_then(Value::as_array)
            .map(|rows| rows.len()),
        Some(1)
    );
    assert_eq!(
        response
            .payload
            .pointer("/lineage/memory_mutation")
            .and_then(Value::as_array)
            .map(|rows| rows.len()),
        Some(1)
    );
    assert_eq!(
        response
            .payload
            .pointer("/lineage/assimilation_step")
            .and_then(Value::as_array)
            .map(|rows| rows.len()),
        Some(1)
    );
    assert_eq!(
        response
            .payload
            .pointer("/validation/claim_evidence_integrity_ok")
            .and_then(Value::as_bool),
        Some(true)
    );
}

#[test]
fn receipts_lineage_http_route_requires_task_id() {
    let root = tempfile::tempdir().expect("tempdir");
    let response = handle(
        root.path(),
        "GET",
        "/api/receipts/lineage",
        &[],
        &json!({"ok": true}),
    )
    .expect("lineage response");
    assert_eq!(response.status, 400);
    assert_eq!(
        response.payload.get("error").and_then(Value::as_str),
        Some("task_id_required")
    );
}

#[test]
fn receipts_lineage_http_route_supports_sources_override() {
    let root = tempfile::tempdir().expect("tempdir");
    let task_id = "task-lineage-sources-001";

    let default_path = root
        .path()
        .join("local/state/runtime/task_runtime/verity_receipts.jsonl");
    let default_row = json!({
        "type": "task_verity_receipt",
        "event_type": "task_result",
        "receipt_hash": "r-default",
        "payload": {"task_id": task_id, "status": "from_default"}
    });
    if let Some(parent) = default_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(
        &default_path,
        format!(
            "{}\n{}\n",
            serde_json::to_string(&default_row).expect("encode default row 1"),
            serde_json::to_string(&default_row).expect("encode default row 2")
        ),
    );

    let override_rel = "local/state/custom/lineage_override.jsonl";
    let override_path = root.path().join(override_rel);
    let override_row = json!({
        "type": "task_verity_receipt",
        "event_type": "task_result",
        "receipt_hash": "r-override",
        "payload": {"task_id": task_id, "status": "from_override"}
    });
    if let Some(parent) = override_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(
        &override_path,
        format!(
            "{}\n",
            serde_json::to_string(&override_row).expect("encode override row")
        ),
    );

    let response = handle(
        root.path(),
        "GET",
        &format!("/api/receipts/lineage?task_id={task_id}&sources={override_rel}"),
        &[],
        &json!({"ok": true}),
    )
    .expect("lineage response");
    assert_eq!(response.status, 200);
    assert_eq!(
        response
            .payload
            .pointer("/stats/sources_override")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        response
            .payload
            .pointer("/stats/scanned_files")
            .and_then(Value::as_u64),
        Some(1)
    );
    assert_eq!(
        response
            .payload
            .pointer("/lineage/task")
            .and_then(Value::as_array)
            .map(|rows| rows.len()),
        Some(1)
    );
    assert_eq!(
        response
            .payload
            .pointer("/lineage/task/0/receipt_hash")
            .and_then(Value::as_str),
        Some("r-override")
    );
}
