fn workflow_gate_stability_update_version_ring(
    root: &Path,
    workflow_id: &str,
    version_hash: &str,
    workflow_snapshot: &Value,
    rows: &[Value],
    ts: &str,
) -> Value {
    let path = root.join("local/state/ops/workflow_gate_stability/versions_ring.json");
    let snapshots_dir = root.join("local/state/ops/workflow_gate_stability/workflow_versions");
    let snapshot_path = snapshots_dir.join(format!("{version_hash}.workflow.json"));
    write_json_pretty(&snapshot_path, workflow_snapshot);
    let snapshot_rel_path = format!(
        "local/state/ops/workflow_gate_stability/workflow_versions/{version_hash}.workflow.json"
    );
    let existing = read_json_loose(&path).unwrap_or_else(|| json!({}));
    let mut versions = existing
        .get("versions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    versions.retain(|row| {
        row.get("workflow_json")
            .filter(|snapshot| snapshot.is_object())
            .is_some()
    });
    let mut version = versions
        .iter()
        .position(|row| {
            row.get("workflow_version_hash").and_then(Value::as_str) == Some(version_hash)
        })
        .map(|index| versions.remove(index))
        .unwrap_or_else(|| {
            json!({
                "workflow_version_hash": version_hash,
                "workflow_snapshot_path": snapshot_rel_path.clone(),
                "workflow_json": workflow_snapshot,
                "first_seen": ts,
                "turn_count": 0,
                "event_count": 0,
                "per_gate": []
            })
        });
    version["workflow_snapshot_path"] = Value::String(snapshot_rel_path.clone());
    version["workflow_json"] = workflow_snapshot.clone();
    let mut counts = HashMap::<String, (usize, usize, usize)>::new();
    for gate_row in version
        .get("per_gate")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
    {
        let gate = clean_text(
            gate_row.get("gate").and_then(Value::as_str).unwrap_or(""),
            120,
        );
        if gate.is_empty() {
            continue;
        }
        counts.insert(
            gate,
            (
                gate_row.get("passed").and_then(Value::as_u64).unwrap_or(0) as usize,
                gate_row.get("failed").and_then(Value::as_u64).unwrap_or(0) as usize,
                gate_row.get("other").and_then(Value::as_u64).unwrap_or(0) as usize,
            ),
        );
    }
    for row in rows {
        let gate = clean_text(row.get("gate").and_then(Value::as_str).unwrap_or(""), 120);
        if gate.is_empty() {
            continue;
        }
        let entry = counts.entry(gate).or_insert((0, 0, 0));
        match row.get("status").and_then(Value::as_str).unwrap_or("") {
            "passed" => entry.0 += 1,
            "failed" => entry.1 += 1,
            _ => entry.2 += 1,
        }
    }
    version["last_seen"] = Value::String(ts.to_string());
    version["turn_count"] = json!(
        version
            .get("turn_count")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            + 1
    );
    version["event_count"] = json!(
        version
            .get("event_count")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            + rows.len() as u64
    );
    version["per_gate"] = Value::Array(workflow_gate_stability_per_gate_rollup_from_counts(counts));
    versions.insert(0, version);
    versions.truncate(3);
    let retained_hashes = versions
        .iter()
        .filter_map(|row| row.get("workflow_version_hash").and_then(Value::as_str))
        .map(str::to_string)
        .collect::<std::collections::BTreeSet<_>>();
    if let Ok(entries) = std::fs::read_dir(&snapshots_dir) {
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            let Some(file_name) = path.file_name().and_then(|value| value.to_str()) else {
                continue;
            };
            if !file_name.ends_with(".workflow.json") {
                continue;
            }
            let hash = file_name.trim_end_matches(".workflow.json");
            if !retained_hashes.contains(hash) {
                let _ = std::fs::remove_file(path);
            }
        }
    }
    let ring = json!({
        "type": "workflow_gate_stability_version_ring",
        "updated_at": ts,
        "workflow_id": workflow_id,
        "ring_size": 3,
        "current_version_hash": version_hash,
        "workflow_snapshots_dir": "local/state/ops/workflow_gate_stability/workflow_versions",
        "versions": versions
    });
    write_json_pretty(&path, &ring);
    ring
}

fn finalize_workflow_gate_stability(root: &Path, mut workflow: Value, message: &str) -> Value {
    let contract = workflow_gate_stability_contract(&workflow);
    let rows = workflow_gate_stability_rows(&workflow);
    let summary = workflow_gate_stability_summary(&rows);
    let workflow_id = clean_text(
        workflow
            .pointer("/selected_workflow/name")
            .or_else(|| workflow.pointer("/selected_workflow/id"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        120,
    );
    let message_hash = crate::deterministic_receipt_hash(&json!({
        "type": "workflow_gate_stability_message",
        "workflow_id": workflow_id,
        "message": clean_text(message, 2_000)
    }));
    let ts = crate::now_iso();
    let workflow_snapshot =
        workflow_gate_stability_current_workflow_snapshot(&workflow_id, &contract);
    let version_hash = workflow_gate_stability_version_hash(&workflow_id, &workflow_snapshot);
    let stream_path = root.join("local/state/ops/workflow_gate_stability/events.jsonl");
    for row in &rows {
        append_jsonl_row(
            &stream_path,
            &json!({
                "type": "workflow_gate_stability_event",
                "ts": ts,
                "workflow_id": workflow_id,
                "message_hash": message_hash,
                "gate": row.get("gate").cloned().unwrap_or_else(|| json!("")),
                "status": row.get("status").cloned().unwrap_or_else(|| json!("")),
                "failure_class": row.get("failure_class").cloned().unwrap_or_else(|| json!("")),
                "stage_status": row.get("stage_status").cloned().unwrap_or_else(|| json!("")),
                "missing_artifacts": row.get("missing_artifacts").cloned().unwrap_or_else(|| json!([]))
            }),
        );
    }
    let rolling = workflow_gate_stability_latest_rollup(root, &workflow_id);
    let version_ring = workflow_gate_stability_update_version_ring(
        root,
        &workflow_id,
        &version_hash,
        &workflow_snapshot,
        &rows,
        &ts,
    );
    write_json_pretty(
        &root.join("local/state/ops/workflow_gate_stability/latest.json"),
        &json!({
            "updated_at": ts,
            "workflow_id": workflow_id,
            "workflow_version_hash": version_hash,
            "summary": rolling,
            "workflow_snapshot_path": format!("local/state/ops/workflow_gate_stability/workflow_versions/{version_hash}.workflow.json"),
            "versions_ring_path": "local/state/ops/workflow_gate_stability/versions_ring.json"
        }),
    );
    workflow["gate_stability"] = json!({
        "contract": contract,
        "workflow_version_hash": version_hash,
        "workflow_snapshot_path": format!("local/state/ops/workflow_gate_stability/workflow_versions/{version_hash}.workflow.json"),
        "rows": rows,
        "summary": summary,
        "version_ring": version_ring,
        "rolling_summary_path": "local/state/ops/workflow_gate_stability/latest.json",
        "versions_ring_path": "local/state/ops/workflow_gate_stability/versions_ring.json",
        "event_stream_path": "local/state/ops/workflow_gate_stability/events.jsonl"
    });
    workflow
}
