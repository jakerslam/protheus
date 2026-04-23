
fn capability_switchboard_verify_chain(chain_path: &Path) -> Value {
    let rows = capability_switchboard_chain_rows(chain_path);
    let mut prev_hash = "GENESIS".to_string();
    for (index, row) in rows.iter().enumerate() {
        let expected_prev = row
            .get("prev_hash")
            .and_then(Value::as_str)
            .map(|v| clean_text(v, 140))
            .unwrap_or_else(|| "GENESIS".to_string());
        if expected_prev != prev_hash {
            return json!({
                "ok": false,
                "entries": rows.len(),
                "error": "chain_prev_hash_mismatch",
                "index": index,
                "expected_prev_hash": prev_hash,
                "actual_prev_hash": expected_prev
            });
        }
        let mut payload = row.clone();
        if let Some(obj) = payload.as_object_mut() {
            obj.remove("hash");
        }
        let calc = sha256_hex(&stable_json_string(&payload));
        let stored = row
            .get("hash")
            .and_then(Value::as_str)
            .map(|v| clean_text(v, 140))
            .unwrap_or_default();
        if calc != stored {
            return json!({
                "ok": false,
                "entries": rows.len(),
                "error": "chain_hash_mismatch",
                "index": index,
                "expected_hash": calc,
                "actual_hash": stored
            });
        }
        prev_hash = stored;
    }
    json!({
        "ok": true,
        "entries": rows.len(),
        "tip_hash": prev_hash
    })
}

fn capability_switchboard_resolve_switch_id<'a>(
    switches: &'a serde_json::Map<String, Value>,
    requested: &'a str,
) -> Option<&'a str> {
    if switches.contains_key(requested) {
        return Some(requested);
    }
    switches
        .keys()
        .find(|key| key.eq_ignore_ascii_case(requested))
        .map(|key| key.as_str())
}

fn capability_switchboard_run_policy_root(
    script_path: &Path,
    scope: &str,
    target: &str,
    approval_note: &str,
    lease_token: Option<&str>,
    source: &str,
) -> Value {
    if !script_path.exists() {
        return json!({
            "ok": false,
            "decision": "DENY",
            "reason": "policy_root_script_missing"
        });
    }
    let safe_scope = normalize_token(scope, 160);
    let safe_target = normalize_token(target, 160);
    let safe_source = normalize_token(source, 120);
    if safe_scope.is_empty() || safe_target.is_empty() || safe_source.is_empty() {
        return json!({
            "ok": false,
            "decision": "DENY",
            "reason": "policy_root_invalid_scope_target_or_source"
        });
    }

    let node = std::env::var("INFRING_NODE_BINARY")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| "node".to_string());
    let mut args = vec![
        script_path.to_string_lossy().to_string(),
        "authorize".to_string(),
        format!("--scope={safe_scope}"),
        format!("--target={safe_target}"),
        format!("--approval-note={}", clean_text(approval_note, 360)),
        format!("--source={safe_source}"),
    ];
    if let Some(token) = lease_token {
        let clean = clean_text(token, 260);
        if !clean.is_empty() {
            args.push(format!("--lease-token={clean}"));
        }
    }

    let run = Command::new(node)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();
    let output = match run {
        Ok(v) => v,
        Err(err) => {
            return json!({
                "ok": false,
                "decision": "DENY",
                "reason": format!("policy_root_spawn_failed:{err}")
            })
        }
    };

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let payload = parse_json_from_stdout(&stdout).unwrap_or_else(|| {
        json!({
            "ok": false,
            "decision": "DENY",
            "reason": "policy_root_invalid_payload"
        })
    });
    let decision = payload
        .get("decision")
        .and_then(Value::as_str)
        .unwrap_or("DENY")
        .to_ascii_uppercase();
    let ok = output.status.success()
        && payload.get("ok").and_then(Value::as_bool).unwrap_or(false)
        && decision == "ALLOW";
    json!({
        "ok": ok,
        "decision": decision,
        "raw": payload,
        "code": output.status.code().unwrap_or(1),
        "stderr": clean_text(stderr, 320),
        "stdout": clean_text(stdout, 320)
    })
}

pub fn run_capability_switchboard(repo_root: &Path, argv: &[String]) -> (Value, i32) {
    let args = parse_cli_args(argv);
    let cmd = args
        .positional
        .first()
        .map(|v| normalize_token(v, 80))
        .unwrap_or_else(|| "status".to_string());
    let (policy_path, state_path, audit_path, policy_root_script, chain_path) =
        capability_switchboard_paths(repo_root);
    let policy = capability_switchboard_load_policy(&policy_path);
    let state = capability_switchboard_load_state(&state_path);
    let effective = capability_switchboard_effective_switches(&policy, &state);

    if cmd == "status" {
        let chain = capability_switchboard_verify_chain(&chain_path);
        return (
            json!({
                "ok": true,
                "type": "capability_switchboard_status",
                "ts": now_iso(),
                "policy_version": policy.get("version").cloned().unwrap_or(Value::String("1.0".to_string())),
                "switches": effective,
                "hash_chain": chain
            }),
            0,
        );
    }

    if cmd == "verify-chain" {
        let chain = capability_switchboard_verify_chain(&chain_path);
        let ok = chain.get("ok").and_then(Value::as_bool).unwrap_or(false);
        return (
            json!({
                "ok": ok,
                "type": "capability_switchboard_chain_verify",
                "ts": now_iso(),
                "chain_path": normalize_rel_path(chain_path.display().to_string()),
                "chain": chain
            }),
            if ok { 0 } else { 1 },
        );
    }

    if cmd == "evaluate" {
        let switch_id = normalize_token(args.flags.get("switch").cloned().unwrap_or_default(), 120);
        if switch_id.is_empty() {
            return (
                json!({
                    "ok": false,
                    "type": "capability_switchboard_evaluate",
                    "reason": "missing_switch"
                }),
                2,
            );
        }
        let switch_row = effective
            .iter()
            .find(|row| {
                row.get("id")
                    .and_then(Value::as_str)
                    .map(|v| v.eq_ignore_ascii_case(&switch_id))
                    .unwrap_or(false)
            })
            .cloned();
        match switch_row {
            Some(row) => (
                json!({
                    "ok": true,
                    "type": "capability_switchboard_evaluate",
                    "switch": switch_id,
                    "enabled": row.get("enabled").and_then(Value::as_bool).unwrap_or(true),
                    "switch_row": row
                }),
                0,
            ),
            None => (
                json!({
                    "ok": false,
                    "type": "capability_switchboard_evaluate",
                    "switch": switch_id,
                    "reason": "unknown_switch"
                }),
                1,
            ),
        }
    } else if cmd == "set" {
        let switch_id = normalize_token(args.flags.get("switch").cloned().unwrap_or_default(), 120);
        if switch_id.is_empty() {
            return (
                json!({
                    "ok": false,
                    "type": "capability_switchboard_set",
                    "reason": "missing_switch"
                }),
                2,
            );
        }
        let requested_state = bool_state(args.flags.get("state").map(String::as_str));
        if requested_state.is_none() {
            return (
                json!({
                    "ok": false,
                    "type": "capability_switchboard_set",
                    "switch": switch_id,
                    "reason": "invalid_state"
                }),
                2,
            );
        }
        let target_enabled = requested_state.unwrap_or(true);

        let switches = policy.get("switches").and_then(Value::as_object);
        let resolved_switch_id = switches
            .and_then(|rows| capability_switchboard_resolve_switch_id(rows, &switch_id))
            .map(|id| id.to_string());
        if resolved_switch_id.is_none() {
            return (
                json!({
                    "ok": false,
                    "type": "capability_switchboard_set",
                    "switch": switch_id,
                    "reason": "unknown_switch"
                }),
                1,
            );
        }
        let resolved_switch_id = resolved_switch_id.unwrap_or_else(|| switch_id.clone());
        let switch_policy = switches
            .and_then(|rows| rows.get(&resolved_switch_id))
            .cloned()
            .unwrap_or_else(|| json!({}));
        let security_locked = switch_policy
            .get("security_locked")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if security_locked && !target_enabled {
            return (
                json!({
                    "ok": false,
                    "type": "capability_switchboard_set",
                    "switch": resolved_switch_id,
                    "reason": "security_locked_non_deactivatable"
                }),
                1,
            );
        }

        let require_dual = policy
            .get("require_dual_control")
            .and_then(Value::as_bool)
            .unwrap_or(true);
        let min_note = number_i64(policy.get("dual_control_min_note_len"), 12, 8, 4096) as usize;
        let approver_id = normalize_token(
            args.flags.get("approver-id").cloned().unwrap_or_default(),
            120,
        );
        let second_approver_id = normalize_token(
            args.flags
                .get("second-approver-id")
                .cloned()
                .unwrap_or_default(),
            120,
        );
        let approval_note = clean_text(
            args.flags.get("approval-note").cloned().unwrap_or_default(),
            720,
        );
        let second_approval_note = clean_text(
            args.flags
                .get("second-approval-note")
                .cloned()
                .unwrap_or_default(),
            720,
        );
        if require_dual {
            if approver_id.is_empty() || second_approver_id.is_empty() {
                return (
                    json!({
                        "ok": false,
                        "type": "capability_switchboard_set",
                        "switch": switch_id,
                        "reason": "dual_control_approver_missing"
                    }),
                    1,
                );
            }
            if approver_id.eq_ignore_ascii_case(&second_approver_id) {
                return (
                    json!({
                        "ok": false,
                        "type": "capability_switchboard_set",
                        "switch": resolved_switch_id,
                        "reason": "dual_control_approver_must_differ"
                    }),
                    1,
                );
            }
            if approval_note.len() < min_note || second_approval_note.len() < min_note {
                return (
                    json!({
                        "ok": false,
                        "type": "capability_switchboard_set",
                        "switch": switch_id,
                        "reason": "approval_note_too_short"
                    }),
                    1,
                );
            }
        }

        let require_policy_root = switch_policy
            .get("require_policy_root")
            .and_then(Value::as_bool)
            .unwrap_or(true);
        let policy_root_required = policy
            .get("policy_root")
            .and_then(|row| row.get("required"))
            .and_then(Value::as_bool)
            .unwrap_or(true);
        let policy_root = if require_policy_root || policy_root_required {
            let scope = policy
                .get("policy_root")
                .and_then(|row| row.get("scope"))
                .and_then(Value::as_str)
                .unwrap_or("capability_switchboard_toggle");
            let lease_token = args.flags.get("lease-token").map(String::as_str);
            let source = args
                .flags
                .get("source")
                .map(String::as_str)
                .unwrap_or("capability_switchboard");
            capability_switchboard_run_policy_root(
                &policy_root_script,
                scope,
                &resolved_switch_id,
                &approval_note,
                lease_token,
                source,
            )
        } else {
            json!({
                "ok": true,
                "decision": "ALLOW",
                "reason": "policy_root_not_required"
            })
        };
        if !policy_root
            .get("ok")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            return (
                json!({
                    "ok": false,
                    "type": "capability_switchboard_set",
                    "switch": resolved_switch_id,
                    "reason": "policy_root_denied",
                    "policy_root": policy_root
                }),
                1,
            );
        }

        let mut next_state = capability_switchboard_load_state(&state_path);
        let mut switches = next_state
            .get("switches")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        switches.insert(
            resolved_switch_id.clone(),
            json!({
                "enabled": target_enabled,
                "updated_at": now_iso(),
                "updated_by": approver_id,
                "reason": approval_note
            }),
        );
        next_state["switches"] = Value::Object(switches);
        next_state["updated_at"] = Value::String(now_iso());
        if let Err(err) = write_json_atomic(&state_path, &next_state) {
            return (
                json!({
                    "ok": false,
                    "type": "capability_switchboard_set",
                    "switch": resolved_switch_id,
                    "reason": format!("state_write_failed:{err}")
                }),
                1,
            );
        }

        let audit_row = json!({
            "ts": now_iso(),
            "type": "capability_switchboard_set",
            "switch": resolved_switch_id.clone(),
            "enabled": target_enabled,
            "approver_id": approver_id.clone(),
            "second_approver_id": second_approver_id.clone(),
            "reason": approval_note.clone(),
            "policy_root": policy_root
        });
        let _ = append_jsonl(&audit_path, &audit_row);

        let prev_hash = capability_switchboard_chain_tip(&chain_path);
        let mut chain_row = json!({
            "ts": now_iso(),
            "type": "capability_switchboard_chain_event",
            "action": if target_enabled { "grant" } else { "revoke" },
            "switch": resolved_switch_id.clone(),
            "enabled": target_enabled,
            "approver_id": approver_id.clone(),
            "second_approver_id": second_approver_id.clone(),
            "reason": approval_note.clone(),
            "policy_scope": policy
                .get("policy_root")
                .and_then(|row| row.get("scope"))
                .and_then(Value::as_str)
                .unwrap_or("capability_switchboard_toggle"),
            "prev_hash": prev_hash
        });
        let hash = sha256_hex(&stable_json_string(&chain_row));
        chain_row["hash"] = Value::String(hash.clone());
        if let Err(err) = append_jsonl(&chain_path, &chain_row) {
            return (
                json!({
                    "ok": false,
                    "type": "capability_switchboard_set",
                    "switch": resolved_switch_id,
                    "reason": format!("hash_chain_append_failed:{err}")
                }),
                1,
            );
        }

        (
            json!({
                "ok": true,
                "type": "capability_switchboard_set",
                "switch": resolved_switch_id,
                "enabled": target_enabled,
                "policy_root": policy_root,
                "hash_chain": {
                    "path": normalize_rel_path(chain_path.display().to_string()),
                    "last_hash": hash
                }
            }),
            0,
        )
    } else {
        (
            json!({
                "ok": false,
                "type": "capability_switchboard_error",
                "reason": format!("unknown_command:{cmd}"),
                "usage": [
                    "infring-ops security-plane capability-switchboard status",
                    "infring-ops security-plane capability-switchboard verify-chain",
                    "infring-ops security-plane capability-switchboard evaluate --switch=<id>",
                    "infring-ops security-plane capability-switchboard set --switch=<id> --state=on|off --approver-id=<id> --approval-note=... --second-approver-id=<id> --second-approval-note=..."
                ]
            }),
            2,
        )
    }
}

// -------------------------------------------------------------------------------------------------
// Black Box Ledger
// -------------------------------------------------------------------------------------------------
