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
