
fn run_cycle(root: &Path, argv: &[String]) -> Result<Value, String> {
    let digest_id = parse_flag(argv, "id").unwrap_or_else(|| "stomach-default".to_string());
    let source_root = parse_flag(argv, "source-root")
        .map(PathBuf::from)
        .ok_or_else(|| "stomach_missing_source_root".to_string())?;
    let origin = parse_flag(argv, "origin")
        .unwrap_or_else(|| "https://github.com/protheuslabs/InfRing".to_string());
    let commit = parse_flag(argv, "commit").unwrap_or_else(|| "unknown".to_string());
    let refs = csv_list(parse_flag(argv, "refs"));
    let spdx = parse_flag(argv, "spdx");
    let transform = parse_transform(argv);
    let scoring_targets = if !transform.target_paths.is_empty() {
        transform.target_paths.clone()
    } else {
        Vec::<String>::new()
    };

    let state_root = stomach_state_root(root);
    ensure_state_dirs(&state_root)?;
    let mut scoring_rows = scored_candidate_rows(&source_root, scoring_targets.as_slice())?;
    let scoring_ledger_path =
        write_scoring_gate_ledger(root, &digest_id, &source_root, &scoring_rows, "preflight_scored")?;

    let out = run_stomach_cycle(
        &state_root,
        &digest_id,
        &source_root,
        &origin,
        &commit,
        &refs,
        spdx.as_deref(),
        &transform,
        &StomachConfig::default(),
    )?;

    write_json(
        &state_root.join("fetch").join(format!("{digest_id}.json")),
        &serde_json::to_value(&out.fetch)
            .map_err(|e| format!("stomach_fetch_encode_failed:{e}"))?,
    )?;
    write_json(
        &state_root
            .join("snapshots")
            .join(format!("{digest_id}.json")),
        &serde_json::to_value(&out.snapshot)
            .map_err(|e| format!("stomach_snapshot_encode_failed:{e}"))?,
    )?;
    write_json(
        &state_root
            .join("provenance")
            .join(format!("{digest_id}.json")),
        &serde_json::to_value(&out.provenance)
            .map_err(|e| format!("stomach_provenance_encode_failed:{e}"))?,
    )?;
    write_json(
        &state_root
            .join("analysis")
            .join(format!("{digest_id}.json")),
        &serde_json::to_value(&out.analysis)
            .map_err(|e| format!("stomach_analysis_encode_failed:{e}"))?,
    )?;
    write_json(
        &state_root
            .join("proposals")
            .join(format!("{digest_id}.json")),
        &serde_json::to_value(&out.proposal)
            .map_err(|e| format!("stomach_proposal_encode_failed:{e}"))?,
    )?;
    write_json(
        &state_root.join("state").join(format!("{digest_id}.json")),
        &serde_json::to_value(&out.state)
            .map_err(|e| format!("stomach_state_encode_failed:{e}"))?,
    )?;
    let evidence_pointer = state_root
        .join("receipts.jsonl")
        .to_string_lossy()
        .to_string();
    advance_scoring_rows(&mut scoring_rows, &evidence_pointer, None);
    let _ = write_scoring_gate_ledger(
        root,
        &digest_id,
        &source_root,
        &scoring_rows,
        "completed",
    )?;
    let scoring_report_path =
        write_scoring_gate_markdown_report(root, &digest_id, &scoring_rows)?;

    let receipt_payload = json!({
      "digest_id": digest_id,
      "snapshot_id": out.snapshot.snapshot_id,
      "proposal_id": out.proposal.proposal_id,
      "execution_status": out.execution.status,
      "state_status": out.state.status,
      "cycle_hash": stable_hash(&out),
      "scoring_gate": {
        "mandatory": true,
        "ledger_path": scoring_ledger_path.to_string_lossy().to_string(),
        "report_path": scoring_report_path.to_string_lossy().to_string(),
        "row_count": scoring_rows.len()
      }
    });
    let receipt = json_receipt("stomach_kernel_run", receipt_payload);
    append_jsonl(&state_root.join("receipts.jsonl"), &receipt)?;
    Ok(receipt)
}

fn score_cycle(root: &Path, argv: &[String]) -> Result<Value, String> {
    let digest_id = parse_flag(argv, "id").unwrap_or_else(|| "stomach-default".to_string());
    let source_root = parse_flag(argv, "source-root")
        .map(PathBuf::from)
        .ok_or_else(|| "stomach_missing_source_root".to_string())?;
    let targets = csv_list(parse_flag(argv, "targets"));
    let mut rows = scored_candidate_rows(&source_root, targets.as_slice())?;
    let evidence_pointer = stomach_state_root(root)
        .join("receipts.jsonl")
        .to_string_lossy()
        .to_string();
    advance_scoring_rows(&mut rows, &evidence_pointer, Some("score_only_mode"));
    let ledger_path = write_scoring_gate_ledger(root, &digest_id, &source_root, &rows, "score_only")?;
    let report_path = write_scoring_gate_markdown_report(root, &digest_id, &rows)?;
    Ok(json_receipt(
        "stomach_kernel_score",
        json!({
            "digest_id": digest_id,
            "mandatory_scoring_gate": true,
            "ledger_path": ledger_path.to_string_lossy().to_string(),
            "report_path": report_path.to_string_lossy().to_string(),
            "row_count": rows.len()
        }),
    ))
}

fn status_cycle(root: &Path, argv: &[String]) -> Result<Value, String> {
    let digest_id = parse_flag(argv, "id").ok_or_else(|| "stomach_missing_id".to_string())?;
    let state_root = stomach_state_root(root);
    let state = read_json(&state_root.join("state").join(format!("{digest_id}.json")))?;
    let proposal = read_json(
        &state_root
            .join("proposals")
            .join(format!("{digest_id}.json")),
    )
    .ok();
    Ok(json_receipt(
        "stomach_kernel_status",
        json!({
          "digest_id": digest_id,
          "state": state,
          "proposal": proposal
        }),
    ))
}

fn rollback_cycle(root: &Path, argv: &[String]) -> Result<Value, String> {
    let digest_id = parse_flag(argv, "id").ok_or_else(|| "stomach_missing_id".to_string())?;
    let receipt =
        parse_flag(argv, "receipt").ok_or_else(|| "stomach_missing_receipt".to_string())?;
    let reason = parse_flag(argv, "reason").unwrap_or_else(|| "manual_rollback".to_string());
    let state_root = stomach_state_root(root);
    let state_path = state_root.join("state").join(format!("{digest_id}.json"));
    let raw = read_json(&state_path)?;
    let mut state: DigestState =
        serde_json::from_value(raw).map_err(|e| format!("stomach_state_decode_failed:{e}"))?;
    let rollback = rollback_by_receipt(&mut state, &receipt, &reason)?;
    write_json(
        &state_path,
        &serde_json::to_value(&state).map_err(|e| format!("stomach_state_encode_failed:{e}"))?,
    )?;
    let out = json_receipt(
        "stomach_kernel_rollback",
        json!({
          "digest_id": digest_id,
          "rollback": rollback,
          "state_status": state.status
        }),
    );
    append_jsonl(&state_root.join("receipts.jsonl"), &out)?;
    Ok(out)
}

fn purge_cycle(root: &Path, argv: &[String]) -> Result<Value, String> {
    let digest_id = parse_flag(argv, "id").ok_or_else(|| "stomach_missing_id".to_string())?;
    let state_root = stomach_state_root(root);
    let state_path = state_root.join("state").join(format!("{digest_id}.json"));
    let raw = read_json(&state_path)?;
    let mut state: DigestState =
        serde_json::from_value(raw).map_err(|e| format!("stomach_state_decode_failed:{e}"))?;
    if matches!(
        state.status,
        DigestStatus::Proposed | DigestStatus::Verified | DigestStatus::Assimilated
    ) && state.retention.explicit_purge_approval_receipt.is_none()
    {
        return Err(
            "stomach_purge_explicit_approval_required_for_proposed_or_assimilated".to_string(),
        );
    }
    let now_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let quarantine_dir = state_root.join("quarantine").join(&digest_id);
    purge_artifact_path(&quarantine_dir, &mut state.retention, now_secs)?;
    write_json(
        &state_path,
        &serde_json::to_value(&state).map_err(|e| format!("stomach_state_encode_failed:{e}"))?,
    )?;
    let out = json_receipt(
        "stomach_kernel_purge",
        json!({
          "digest_id": digest_id,
          "retention_state": state.retention_state()
        }),
    );
    append_jsonl(&state_root.join("receipts.jsonl"), &out)?;
    Ok(out)
}
