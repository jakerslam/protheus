
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn write_contract_receipt_increments_chain() {
        let tmp = tempdir().expect("tempdir");
        let file_path = tmp.path().join("receipts.jsonl");
        let payload = json!({
            "file_path": file_path,
            "record": { "type": "unit" },
            "attempted": true,
            "verified": false
        });
        let first = write_contract_receipt_value(tmp.path(), payload_obj(&payload)).expect("first");
        let first_seq = first
            .get("record")
            .and_then(Value::as_object)
            .and_then(|row| row.get("receipt_contract"))
            .and_then(Value::as_object)
            .and_then(|row| row.get("integrity"))
            .and_then(Value::as_object)
            .and_then(|row| row.get("seq"))
            .and_then(Value::as_u64);
        assert_eq!(first_seq, Some(1));

        let second =
            write_contract_receipt_value(tmp.path(), payload_obj(&payload)).expect("second");
        let second_seq = second
            .get("record")
            .and_then(Value::as_object)
            .and_then(|row| row.get("receipt_contract"))
            .and_then(Value::as_object)
            .and_then(|row| row.get("integrity"))
            .and_then(Value::as_object)
            .and_then(|row| row.get("seq"))
            .and_then(Value::as_u64);
        assert_eq!(second_seq, Some(2));
        assert!(chain_state_path(&file_path).exists());
    }

    #[test]
    fn replay_task_lineage_reconstructs_end_to_end_chain() {
        let tmp = tempdir().expect("tempdir");
        let task_id = "task-123";
        let trace_id = "trace-abc";

        let task_receipts = tmp
            .path()
            .join("local/state/runtime/task_runtime/verity_receipts.jsonl");
        append_jsonl(
            &task_receipts,
            &json!({
                "type": "task_verity_receipt",
                "event_type": "task_result",
                "receipt_hash": "r-task",
                "payload": {"task_id": task_id, "status":"done"}
            }),
        )
        .expect("append task receipt");

        let actions_history = tmp
            .path()
            .join("client/runtime/local/state/ui/infring_dashboard/actions/history.jsonl");
        append_jsonl(
            &actions_history,
            &json!({
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
                            "evidence_id":"ev-1",
                            "task_id": task_id,
                            "trace_id": trace_id,
                            "summary":"snippet"
                        }],
                        "claim_bundle": {
                            "task_id": task_id,
                            "claims": [{
                                "claim_id":"claim-1",
                                "text":"found",
                                "evidence_ids":["ev-1"],
                                "status":"supported"
                            }]
                        }
                    }
                }
            }),
        )
        .expect("append action history");

        let memory_history = tmp.path().join("local/state/ops/memory/history.jsonl");
        append_jsonl(
            &memory_history,
            &json!({
                "type":"memory_write",
                "task_id": task_id,
                "receipt_hash":"r-mem",
                "payload":{"object_id":"o-1","version_id":"v-1"}
            }),
        )
        .expect("append memory history");

        let assimilation_steps = tmp
            .path()
            .join("local/state/ops/runtime_systems/assimilate/protocol_step_receipts.jsonl");
        append_jsonl(
            &assimilation_steps,
            &json!({
                "type":"assimilation_protocol_step",
                "task_id": task_id,
                "step_id":"step-1",
                "receipt_hash":"r-assim"
            }),
        )
        .expect("append assimilation steps");

        let out = run_command(
            tmp.path(),
            "replay-task-lineage",
            payload_obj(&json!({"task_id": task_id, "trace_id": trace_id})),
        )
        .expect("replay lineage");
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            out.pointer("/lineage/tool_call")
                .and_then(Value::as_array)
                .map(|rows| rows.len()),
            Some(1)
        );
        assert_eq!(
            out.pointer("/lineage/evidence")
                .and_then(Value::as_array)
                .map(|rows| rows.len()),
            Some(1)
        );
        assert_eq!(
            out.pointer("/lineage/claim")
                .and_then(Value::as_array)
                .map(|rows| rows.len()),
            Some(1)
        );
        assert_eq!(
            out.pointer("/lineage/memory_mutation")
                .and_then(Value::as_array)
                .map(|rows| rows.len()),
            Some(1)
        );
        assert_eq!(
            out.pointer("/lineage/assimilation_step")
                .and_then(Value::as_array)
                .map(|rows| rows.len()),
            Some(1)
        );
        assert_eq!(
            out.pointer("/validation/claim_evidence_integrity_ok")
                .and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn replay_task_lineage_requires_task_id() {
        let tmp = tempdir().expect("tempdir");
        let err = run_command(tmp.path(), "replay-task-lineage", payload_obj(&json!({})))
            .expect_err("expected missing task id error");
        assert_eq!(err, "task_id_required");
    }
}

