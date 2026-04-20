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
                    json!({"ok": false, "type": "constitution_activate_change", "error": "candidate_copy_missing"}),
                    1,
                );
            }
            if paths.constitution.exists() {
                let backup_name = format!("{}_constitution.md", Utc::now().format("%Y%m%d%H%M%S"));
                let backup_path = paths.history_dir.join(backup_name);
                if let Some(parent) = backup_path.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                let _ = fs::copy(&paths.constitution, &backup_path);
            }
            if let Some(parent) = paths.constitution.parent() {
                let _ = fs::create_dir_all(parent);
            }
            if let Err(err) = fs::copy(&candidate_path, &paths.constitution) {
                return (
                    json!({"ok": false, "type": "constitution_activate_change", "error": clean(format!("activate_copy_failed:{err}"), 220)}),
                    1,
                );
            }
            if let Some(obj) = proposal.as_object_mut() {
                obj.insert("status".to_string(), Value::String("active".to_string()));
                obj.insert("activated_at".to_string(), Value::String(now_iso()));
                obj.insert(
                    "activation".to_string(),
                    json!({"approver_id": approver_id, "approval_note": approval_note}),
                );
            }
            if let Err(err) = save_proposal(&paths, &proposal_id, &proposal) {
                return (
                    json!({"ok": false, "type": "constitution_activate_change", "error": clean(err, 220)}),
                    1,
                );
            }
            if let Err(err) = write_json_atomic(
                &paths.active_state,
                &json!({
                    "active_proposal_id": proposal_id,
                    "activated_at": now_iso(),
                    "constitution_sha256": sha256_hex_file(&paths.constitution).unwrap_or_default()
                }),
            ) {
                return (
                    json!({"ok": false, "type": "constitution_activate_change", "error": clean(err, 220)}),
                    1,
                );
            }
            let _ = append_jsonl(
                &paths.events,
                &json!({"ts": now_iso(), "type": "constitution_activated", "proposal_id": proposal_id}),
            );
            (
                json!({"ok": true, "type": "constitution_activate_change", "proposal": proposal}),
                0,
            )
        }
        "enforce-inheritance" => {
            let actor = clean(flag(&parsed, "actor").unwrap_or("unknown"), 120);
            let target = clean(flag(&parsed, "target").unwrap_or("unknown"), 120);
            let locked = policy.enforce_inheritance_lock;
            let out = json!({
                "ok": true,
                "type": "constitution_enforce_inheritance",
                "actor": actor,
                "target": target,
                "inheritance_lock_enforced": locked,
                "ts": now_iso()
            });
            let _ = append_jsonl(&paths.events, &out);
            (out, 0)
        }
        "emergency-rollback" => {
            let note = clean(flag(&parsed, "note").unwrap_or(""), 400);
            if policy.emergency_rollback_requires_approval
                && note.len() < policy.min_approval_note_chars
            {
                return (
                    json!({"ok": false, "type": "constitution_emergency_rollback", "error": "approval_note_too_short"}),
                    1,
                );
            }
            let mut backups = fs::read_dir(&paths.history_dir)
                .ok()
                .into_iter()
                .flatten()
                .filter_map(Result::ok)
                .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
                .collect::<Vec<_>>();
            backups.sort_by_key(|e| e.file_name());
            let Some(entry) = backups.pop() else {
                return (
                    json!({"ok": false, "type": "constitution_emergency_rollback", "error": "no_backup_available"}),
                    1,
                );
            };
            if let Err(err) = fs::copy(entry.path(), &paths.constitution) {
                return (
                    json!({"ok": false, "type": "constitution_emergency_rollback", "error": clean(format!("rollback_copy_failed:{err}"), 220)}),
                    1,
                );
            }
            let _ = append_jsonl(
                &paths.events,
                &json!({
                    "ts": now_iso(),
                    "type": "constitution_emergency_rollback",
                    "rollback_from": normalize_rel(entry.path().to_string_lossy()),
                    "note": note
                }),
            );
            (
                json!({
                    "ok": true,
                    "type": "constitution_emergency_rollback",
                    "rollback_from": normalize_rel(entry.path().to_string_lossy())
                }),
                0,
            )
        }
        "status" => {
            let proposals = fs::read_dir(&paths.proposals_dir)
                .ok()
                .into_iter()
                .flatten()
                .filter_map(Result::ok)
                .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
                .filter_map(|e| e.file_name().into_string().ok())
                .collect::<Vec<_>>();
            (
                json!({
                    "ok": true,
                    "type": "constitution_guardian_status",
                    "ts": now_iso(),
                    "policy_version": policy.version,
                    "constitution_path": normalize_rel(paths.constitution.to_string_lossy()),
                    "genesis": read_json_or(&paths.genesis, Value::Null),
                    "active_state": read_json_or(&paths.active_state, Value::Null),
                    "proposals_count": proposals.len(),
                    "proposals": proposals.into_iter().take(25).collect::<Vec<_>>(),
                    "state_dir": normalize_rel(paths.state_dir.to_string_lossy())
                }),
                0,
            )
        }
        _ => (
            json!({
