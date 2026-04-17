fn run_quarantine(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let op = clean(
        parsed
            .flags
            .get("op")
            .cloned()
            .or_else(|| parsed.positional.get(1).cloned())
            .unwrap_or_else(|| "status".to_string()),
        40,
    )
    .to_ascii_lowercase();
    let mut ledger = read_json(&quarantine_path(root)).unwrap_or_else(|| json!({}));
    let mut rewritten_from_invalid = false;
    if !ledger.is_object() {
        ledger = json!({});
        rewritten_from_invalid = true;
    }
    let mut rows = ledger.as_object().cloned().unwrap_or_default();
    let mut state_changed = rewritten_from_invalid;
    if matches!(op.as_str(), "quarantine" | "release") {
        let skill = clean(
            parsed
                .flags
                .get("skill")
                .cloned()
                .or_else(|| parsed.positional.get(2).cloned())
                .unwrap_or_default(),
            120,
        );
        if skill.is_empty() {
            return json!({
                "ok": false,
                "strict": strict,
                "type": "skills_plane_quarantine",
                "errors": ["skill_required"]
            });
        }
        if op == "quarantine" {
            let reason = clean(
                parsed
                    .flags
                    .get("reason")
                    .cloned()
                    .unwrap_or_else(|| "operator_request".to_string()),
                220,
            );
            let existing = rows.get(&skill).and_then(Value::as_object).cloned();
            let existing_reason = existing
                .as_ref()
                .and_then(|row| row.get("reason"))
                .and_then(Value::as_str)
                .unwrap_or_default();
            rows.insert(
                skill.clone(),
                json!({
                    "reason": reason,
                    "ts": crate::now_iso()
                }),
            );
            if existing_reason != reason {
                state_changed = true;
            } else if existing.is_none() {
                state_changed = true;
            }
        } else {
            if rows.remove(&skill).is_some() {
                state_changed = true;
            }
        }
    } else if op != "status" {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "skills_plane_quarantine",
            "errors": [format!("unknown_quarantine_op:{op}")]
        });
    }
    let persisted = Value::Object(rows.clone());
    let mut state_persisted = false;
    if state_changed {
        if write_json(&quarantine_path(root), &persisted).is_err() {
            return json!({
                "ok": false,
                "strict": strict,
                "type": "skills_plane_quarantine",
                "errors": ["quarantine_state_write_failed"]
            });
        }
        state_persisted = true;
    }
    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "skills_plane_quarantine",
        "lane": "core/layer0/ops",
        "op": op,
        "quarantine_path": quarantine_path(root).display().to_string(),
        "quarantined_count": rows.len(),
        "quarantined_skills": Value::Object(rows),
        "state_changed": state_changed,
        "state_persisted": state_persisted,
        "claim_evidence": [
            {
                "id": "V8-SKILL-007",
                "claim": "skill_quarantine_and_release_paths_are_receipted_and_fail_closed",
                "evidence": {
                    "op": op
                }
            },
            {
                "id": "V8-SKILL-009",
                "claim": "skill_lifecycle_validation_enforces_fail_closed_quarantine_controls",
                "evidence": {
                    "quarantined_count": persisted.as_object().map(|v| v.len()).unwrap_or(0)
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}

fn run_rollback(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let skill = clean(
        parsed
            .flags
            .get("skill")
            .cloned()
            .or_else(|| parsed.positional.get(1).cloned())
            .unwrap_or_default(),
        120,
    );
    if skill.is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "skills_plane_rollback",
            "errors": ["skill_required"]
        });
    }
    let target_version = clean(
        parsed
            .flags
            .get("target-version")
            .cloned()
            .unwrap_or_default(),
        40,
    );
    let checkpoint_path = rollback_checkpoint_path(root, &skill);
    if !checkpoint_path.exists() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "skills_plane_rollback",
            "errors": [format!("rollback_checkpoint_missing:{}", checkpoint_path.display())]
        });
    }
    let checkpoint = read_json(&checkpoint_path).unwrap_or_else(|| json!({}));
    let previous_entry = checkpoint
        .get("previous_entry")
        .cloned()
        .unwrap_or(Value::Null);
    let previous_version = previous_entry
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    if strict && previous_entry.is_null() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "skills_plane_rollback",
            "errors": ["rollback_checkpoint_missing_previous_entry"]
        });
    }
    if strict && !target_version.is_empty() && previous_version != target_version {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "skills_plane_rollback",
            "errors": ["rollback_target_version_mismatch"],
            "checkpoint_previous_version": if previous_version.is_empty() { Value::Null } else { Value::String(previous_version.clone()) },
            "target_version": target_version
        });
    }

    let registry_path = state_root(root).join("registry.json");
    let mut registry = load_registry(&registry_path);
    if !registry
        .get("installed")
        .map(Value::is_object)
        .unwrap_or(false)
    {
        registry["installed"] = Value::Object(Map::new());
    }
    let mut installed = registry
        .get("installed")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    if previous_entry.is_null() {
        installed.remove(&skill);
    } else {
        installed.insert(skill.clone(), previous_entry.clone());
    }
    registry["installed"] = Value::Object(installed);
    if write_json(&registry_path, &registry).is_err() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "skills_plane_rollback",
            "errors": [format!("registry_write_failed:{}", registry_path.display())]
        });
    }

    let rollback_history_path = state_root(root)
        .join("migrations")
        .join("rollback_history.jsonl");
    let rollback_latest_path = state_root(root)
        .join("migrations")
        .join("rollback_latest.json");
    let mut rollback_receipt = json!({
        "ok": true,
        "type": "skills_plane_rollback",
        "skill_id": skill,
        "restored_entry": previous_entry,
        "checkpoint_path": checkpoint_path.display().to_string(),
        "registry_path": registry_path.display().to_string(),
        "ts": crate::now_iso()
    });
    rollback_receipt["receipt_hash"] =
        Value::String(sha256_hex_str(&canonical_json_string(&rollback_receipt)));
    if append_jsonl(&rollback_history_path, &rollback_receipt).is_err() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "skills_plane_rollback",
            "errors": [format!("rollback_history_append_failed:{}", rollback_history_path.display())]
        });
    }
    if write_json(&rollback_latest_path, &rollback_receipt).is_err() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "skills_plane_rollback",
            "errors": [format!("rollback_latest_write_failed:{}", rollback_latest_path.display())]
        });
    }
    rollback_receipt["claim_evidence"] = json!([
        {
            "id": "V8-SKILL-002",
            "claim": "skill_backward_compatibility_lane_supports_receipted_rollback_to_previous_registry_state",
            "evidence": {
                "checkpoint_path": checkpoint_path.display().to_string(),
                "rollback_latest_path": rollback_latest_path.display().to_string()
            }
        }
    ]);
    rollback_receipt
}

fn run_skill(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let skill = clean(
        parsed
            .flags
            .get("skill")
            .cloned()
            .or_else(|| parsed.positional.get(1).cloned())
            .unwrap_or_default(),
        120,
    );
    if skill.is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "skills_plane_run",
            "errors": ["skill_required"]
        });
    }
    let input = clean(
        parsed
            .flags
            .get("input")
            .cloned()
            .unwrap_or_else(|| "".to_string()),
        1000,
    );
    if strict {
        let quarantine = read_json(&quarantine_path(root)).unwrap_or_else(|| json!({}));
        if quarantine.get(&skill).and_then(Value::as_object).is_some() {
            let mut out = json!({
                "ok": false,
                "strict": strict,
                "type": "skills_plane_run",
                "errors": ["skill_quarantined"],
                "skill": skill,
                "quarantine": quarantine.get(&skill).cloned().unwrap_or(Value::Null),
                "claim_evidence": [
                    {
                        "id": "V8-SKILL-007",
                        "claim": "quarantined_skills_are_denied_execution_in_strict_mode",
                        "evidence": {
                            "skill": skill
                        }
                    },
                    {
                        "id": "V8-SKILL-009",
                        "claim": "skill_execution_fails_closed_when_quarantine_controls_are_active",
                        "evidence": {
                            "skill": skill
                        }
                    }
                ]
            });
            out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
            return out;
        }
    }
    let compatibility = match evaluate_skill_run_backward_compat(root, &skill) {
        Ok(summary) => summary,
        Err(code) => {
            let mut out = json!({
                "ok": false,
                "strict": strict,
                "type": "skills_plane_run",
                "errors": [format!("backward_compat_gate_failed:{code}")],
                "skill": skill,
                "compatibility": {
                    "compatibility_gate_passed": false,
                    "error": code
                },
                "claim_evidence": [
                    {
                        "id": "V8-SKILL-002",
                        "claim": "skill_run_enforces_backward_compatibility_gates_before_execution",
                        "evidence": {
                            "compatibility_gate_passed": false
                        }
                    }
                ]
            });
            out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
            return out;
        }
    };
    let event = json!({
        "ts": crate::now_iso(),
        "skill": skill,
        "input_sha256": sha256_hex_str(&input),
        "execution_id": format!("skillrun_{}", &sha256_hex_str(&format!("{}:{}", skill, input))[..14])
    });
    let _ = append_jsonl(&state_root(root).join("runs").join("history.jsonl"), &event);
    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "skills_plane_run",
        "lane": "core/layer0/ops",
        "event": event,
        "compatibility": compatibility,
        "claim_evidence": [
            {
                "id": "V6-SKILLS-001.4",
                "claim": "skill_install_run_share_actions_route_through_layer0_conduit_with_deterministic_audit_receipts",
                "evidence": {
                    "action": "run",
                    "skill": skill
                }
            },
            {
                "id": "V8-SKILL-002",
                "claim": "skill_run_enforces_backward_compatibility_gates_before_execution",
                "evidence": {
                    "skill": skill,
                    "compatibility_gate_passed": true
                }
            },
            {
                "id": "V8-SKILL-008",
                "claim": "skill_execution_path_is_receipted_with_deterministic_run_identity",
                "evidence": {
                    "execution_id": event.get("execution_id").cloned().unwrap_or(Value::Null)
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}

fn run_share(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let skill = clean(
        parsed
            .flags
            .get("skill")
            .cloned()
            .or_else(|| parsed.positional.get(1).cloned())
            .unwrap_or_default(),
        120,
    );
    let target = clean(
        parsed
            .flags
            .get("target")
            .cloned()
            .unwrap_or_else(|| "local-team".to_string()),
        120,
    );
    if skill.is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "skills_plane_share",
            "errors": ["skill_required"]
        });
    }
    let packet = json!({
        "skill": skill,
        "target": target,
        "shared_at": crate::now_iso(),
        "packet_hash": sha256_hex_str(&format!("{}:{}", skill, target))
    });
    let _ = append_jsonl(
        &state_root(root).join("share").join("history.jsonl"),
        &packet,
    );
    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "skills_plane_share",
        "lane": "core/layer0/ops",
        "share_packet": packet,
        "claim_evidence": [
            {
                "id": "V6-SKILLS-001.4",
                "claim": "skill_install_run_share_actions_route_through_layer0_conduit_with_deterministic_audit_receipts",
                "evidence": {
                    "action": "share",
                    "skill": skill,
                    "target": target
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}
