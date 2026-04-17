}

fn load_proposal(paths: &ConstitutionPaths, proposal_id: &str) -> Option<Value> {
    let path = proposal_path(paths, proposal_id);
    if !path.exists() {
        return None;
    }
    Some(read_json_or(&path, Value::Null))
}

fn save_proposal(
    paths: &ConstitutionPaths,
    proposal_id: &str,
    value: &Value,
) -> Result<(), String> {
    write_json_atomic(&proposal_path(paths, proposal_id), value)
}

fn proposal_status(value: &Value) -> String {
    clean(
        value
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("unknown"),
        64,
    )
}

pub fn run_constitution_guardian(repo_root: &Path, argv: &[String]) -> (Value, i32) {
    let parsed = parse_args(argv);
    let cmd = parsed
        .positional
        .first()
        .map(|v| v.to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());
    let policy = load_constitution_policy(repo_root, &parsed);
    let paths = constitution_paths(repo_root, &policy);
    let _ = fs::create_dir_all(&paths.proposals_dir);
    let _ = fs::create_dir_all(&paths.history_dir);

    match cmd.as_str() {
        "init-genesis" => {
            if !paths.constitution.exists() {
                return (
                    json!({
                        "ok": false,
                        "type": "constitution_genesis",
                        "error": "constitution_missing",
                        "constitution_path": normalize_rel(paths.constitution.to_string_lossy())
                    }),
                    1,
                );
            }
            let force = bool_flag(&parsed, "force", false);
            if paths.genesis.exists() && !force {
                return (
                    json!({
                        "ok": true,
                        "type": "constitution_genesis",
                        "already_initialized": true,
                        "genesis_path": normalize_rel(paths.genesis.to_string_lossy())
                    }),
                    0,
                );
            }
            let constitution_sha = match sha256_hex_file(&paths.constitution) {
                Ok(v) => v,
                Err(err) => {
                    return (
                        json!({"ok": false, "type": "constitution_genesis", "error": clean(err, 220)}),
                        1,
                    )
                }
            };
            let genesis = json!({
                "type": "constitution_genesis",
                "ts": now_iso(),
                "constitution_path": normalize_rel(paths.constitution.to_string_lossy()),
                "constitution_sha256": constitution_sha,
                "genesis_id": format!("genesis_{}", &sha256_hex_bytes(now_iso().as_bytes())[0..12])
            });
            if let Err(err) = write_json_atomic(&paths.genesis, &genesis) {
                return (
                    json!({"ok": false, "type": "constitution_genesis", "error": clean(err, 220)}),
                    1,
                );
            }
            let _ = append_jsonl(
                &paths.events,
                &json!({"ts": now_iso(), "type": "constitution_genesis_initialized"}),
            );
            (
                json!({"ok": true, "type": "constitution_genesis", "genesis": genesis}),
                0,
            )
        }
        "propose-change" => {
            let candidate_file = clean(
                flag(&parsed, "candidate-file")
                    .or_else(|| flag(&parsed, "candidate_file"))
                    .unwrap_or(""),
                420,
            );
            let proposer = clean(
                flag(&parsed, "proposer-id")
                    .or_else(|| flag(&parsed, "proposer_id"))
                    .unwrap_or(""),
                120,
            );
            let reason = clean(flag(&parsed, "reason").unwrap_or(""), 400);
            if candidate_file.is_empty() || proposer.is_empty() || reason.is_empty() {
                return (
                    json!({"ok": false, "type": "constitution_propose_change", "error": "candidate_file_proposer_id_reason_required"}),
                    1,
                );
            }
            let candidate_abs = resolve_runtime_or_state(repo_root, &candidate_file);
            if !candidate_abs.exists() {
                return (
                    json!({"ok": false, "type": "constitution_propose_change", "error": "candidate_file_missing"}),
                    1,
                );
            }
            let proposal_id = clean(
                flag(&parsed, "proposal-id")
                    .or_else(|| flag(&parsed, "proposal_id"))
                    .unwrap_or(&format!(
                        "ccp_{}",
                        &sha256_hex_bytes(now_iso().as_bytes())[0..10]
                    )),
                120,
            );
            let proposal_dir = paths.proposals_dir.join(&proposal_id);
            let candidate_copy = proposal_dir.join("candidate_constitution.md");
            if let Some(parent) = candidate_copy.parent() {
                let _ = fs::create_dir_all(parent);
            }
            if let Err(err) = fs::copy(&candidate_abs, &candidate_copy) {
                return (
                    json!({"ok": false, "type": "constitution_propose_change", "error": clean(format!("copy_candidate_failed:{err}"), 220)}),
                    1,
                );
            }
            let candidate_sha = sha256_hex_file(&candidate_copy).unwrap_or_default();
            let proposal = json!({
                "proposal_id": proposal_id,
                "status": "pending_approval",
                "created_at": now_iso(),
                "proposer_id": proposer,
                "reason": reason,
                "candidate_file": normalize_rel(candidate_copy.to_string_lossy()),
                "candidate_sha256": candidate_sha,
                "approvals": [],
                "veto": null,
                "gauntlet": null,
                "activated_at": null
            });
            if let Err(err) = save_proposal(&paths, &proposal_id, &proposal) {
                return (
                    json!({"ok": false, "type": "constitution_propose_change", "error": clean(err, 220)}),
                    1,
                );
            }
            let _ = append_jsonl(
                &paths.events,
                &json!({"ts": now_iso(), "type": "constitution_proposal_created", "proposal_id": proposal_id}),
            );
            (
                json!({"ok": true, "type": "constitution_propose_change", "proposal": proposal}),
                0,
            )
        }
        "approve-change" => {
            let proposal_id = clean(
                flag(&parsed, "proposal-id")
                    .or_else(|| flag(&parsed, "proposal_id"))
                    .unwrap_or(""),
                120,
            );
            let approver_id = clean(
                flag(&parsed, "approver-id")
                    .or_else(|| flag(&parsed, "approver_id"))
                    .unwrap_or(""),
                120,
            );
            let approval_note = clean(
                flag(&parsed, "approval-note")
                    .or_else(|| flag(&parsed, "approval_note"))
                    .unwrap_or(""),
                500,
            );
            if proposal_id.is_empty()
                || approver_id.is_empty()
                || approval_note.len() < policy.min_approval_note_chars
            {
                return (
                    json!({"ok": false, "type": "constitution_approve_change", "error": "proposal_id_approver_id_and_approval_note_required"}),
                    1,
                );
            }
            let mut proposal = match load_proposal(&paths, &proposal_id) {
                Some(v) if v.is_object() => v,
                _ => {
                    return (
                        json!({"ok": false, "type": "constitution_approve_change", "error": "proposal_missing"}),
                        1,
                    )
                }
            };
            let mut approvals = proposal
                .get("approvals")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            approvals.push(json!({
                "approver_id": approver_id,
                "approval_note": approval_note,
                "ts": now_iso()
            }));
            let approved_count = approvals.len();
            let status = if policy.require_dual_approval && approved_count < 2 {
                "pending_secondary_approval"
            } else {
                "approved"
            };
            if let Some(obj) = proposal.as_object_mut() {
                obj.insert("approvals".to_string(), Value::Array(approvals));
                obj.insert("status".to_string(), Value::String(status.to_string()));
                obj.insert("updated_at".to_string(), Value::String(now_iso()));
            }
            if let Err(err) = save_proposal(&paths, &proposal_id, &proposal) {
                return (
                    json!({"ok": false, "type": "constitution_approve_change", "error": clean(err, 220)}),
                    1,
                );
            }
            let _ = append_jsonl(
                &paths.events,
                &json!({"ts": now_iso(), "type": "constitution_proposal_approved", "proposal_id": proposal_id, "status": status}),
            );
            (
                json!({"ok": true, "type": "constitution_approve_change", "proposal": proposal}),
                0,
            )
        }
        "veto-change" => {
            let proposal_id = clean(
                flag(&parsed, "proposal-id")
                    .or_else(|| flag(&parsed, "proposal_id"))
                    .unwrap_or(""),
                120,
            );
            let veto_by = clean(
                flag(&parsed, "veto-by")
                    .or_else(|| flag(&parsed, "veto_by"))
                    .unwrap_or(""),
                120,
            );
            let note = clean(flag(&parsed, "note").unwrap_or(""), 400);
            if proposal_id.is_empty() || veto_by.is_empty() || note.is_empty() {
                return (
                    json!({"ok": false, "type": "constitution_veto_change", "error": "proposal_id_veto_by_note_required"}),
                    1,
                );
            }
            let mut proposal = match load_proposal(&paths, &proposal_id) {
                Some(v) if v.is_object() => v,
                _ => {
                    return (
                        json!({"ok": false, "type": "constitution_veto_change", "error": "proposal_missing"}),
                        1,
                    )
                }
            };
            if let Some(obj) = proposal.as_object_mut() {
                obj.insert("status".to_string(), Value::String("vetoed".to_string()));
                obj.insert(
                    "veto".to_string(),
                    json!({"veto_by": veto_by, "note": note, "ts": now_iso()}),
                );
            }
            if let Err(err) = save_proposal(&paths, &proposal_id, &proposal) {
                return (
                    json!({"ok": false, "type": "constitution_veto_change", "error": clean(err, 220)}),
                    1,
                );
            }
            let _ = append_jsonl(
                &paths.events,
                &json!({"ts": now_iso(), "type": "constitution_proposal_vetoed", "proposal_id": proposal_id}),
            );
            (
                json!({"ok": true, "type": "constitution_veto_change", "proposal": proposal}),
                0,
            )
        }
        "run-gauntlet" => {
            let proposal_id = clean(
                flag(&parsed, "proposal-id")
                    .or_else(|| flag(&parsed, "proposal_id"))
                    .unwrap_or(""),
                120,
            );
            let critical_failures = flag(&parsed, "critical-failures")
                .or_else(|| flag(&parsed, "critical_failures"))
                .and_then(|v| v.parse::<i64>().ok())
                .unwrap_or(0)
                .max(0);
            if proposal_id.is_empty() {
                return (
                    json!({"ok": false, "type": "constitution_run_gauntlet", "error": "proposal_id_required"}),
                    1,
                );
            }
            let mut proposal = match load_proposal(&paths, &proposal_id) {
                Some(v) if v.is_object() => v,
                _ => {
                    return (
                        json!({"ok": false, "type": "constitution_run_gauntlet", "error": "proposal_missing"}),
                        1,
                    )
                }
            };
            let gauntlet = json!({
                "ts": now_iso(),
                "critical_failures": critical_failures,
                "evidence": clean(flag(&parsed, "evidence").unwrap_or(""), 400),
                "passed": critical_failures == 0
            });
            if let Some(obj) = proposal.as_object_mut() {
                obj.insert("gauntlet".to_string(), gauntlet.clone());
                obj.insert(
                    "status".to_string(),
                    Value::String(
                        if critical_failures == 0 {
                            "gauntlet_passed"
                        } else {
                            "gauntlet_failed"
                        }
                        .to_string(),
                    ),
                );
            }
            if let Err(err) = save_proposal(&paths, &proposal_id, &proposal) {
                return (
                    json!({"ok": false, "type": "constitution_run_gauntlet", "error": clean(err, 220)}),
                    1,
                );
            }
            let _ = append_jsonl(
                &paths.events,
                &json!({"ts": now_iso(), "type": "constitution_gauntlet", "proposal_id": proposal_id, "passed": critical_failures == 0}),
            );
            (
                json!({"ok": critical_failures == 0, "type": "constitution_run_gauntlet", "proposal": proposal}),
                if critical_failures == 0 { 0 } else { 1 },
            )
        }
        "activate-change" => {
            let proposal_id = clean(
                flag(&parsed, "proposal-id")
                    .or_else(|| flag(&parsed, "proposal_id"))
                    .unwrap_or(""),
                120,
            );
            let approver_id = clean(
                flag(&parsed, "approver-id")
                    .or_else(|| flag(&parsed, "approver_id"))
                    .unwrap_or(""),
                120,
            );
            let approval_note = clean(
                flag(&parsed, "approval-note")
                    .or_else(|| flag(&parsed, "approval_note"))
                    .unwrap_or(""),
                500,
            );
            if proposal_id.is_empty()
                || approver_id.is_empty()
                || approval_note.len() < policy.min_approval_note_chars
            {
                return (
                    json!({"ok": false, "type": "constitution_activate_change", "error": "proposal_id_approver_id_and_approval_note_required"}),
                    1,
                );
            }
            let mut proposal = match load_proposal(&paths, &proposal_id) {
                Some(v) if v.is_object() => v,
                _ => {
                    return (
                        json!({"ok": false, "type": "constitution_activate_change", "error": "proposal_missing"}),
                        1,
                    )
                }
            };
            let status = proposal_status(&proposal);
            if status != "approved" && status != "gauntlet_passed" {
                return (
                    json!({"ok": false, "type": "constitution_activate_change", "error": "proposal_not_approved", "status": status}),
                    1,
                );
            }
            let gauntlet_passed = proposal
                .get("gauntlet")
                .and_then(|v| v.get("passed"))
                .and_then(Value::as_bool)
                .unwrap_or(false);
            if !gauntlet_passed {
                return (
                    json!({"ok": false, "type": "constitution_activate_change", "error": "gauntlet_not_passed"}),
                    1,
                );
            }
            let candidate_path = proposal
                .get("candidate_file")
                .and_then(Value::as_str)
                .map(|v| resolve_runtime_or_state(repo_root, v))
                .unwrap_or_else(|| PathBuf::from(""));
            if !candidate_path.exists() {
                return (
