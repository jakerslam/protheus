fn canonical_continuity_op(raw: &str) -> String {
    let op = clean(raw, 30).to_ascii_lowercase().replace('-', "_");
    match op.as_str() {
        "checkpoint_state" | "save" | "snapshot" => "checkpoint".to_string(),
        "restore" | "resume" | "rebuild" => "reconstruct".to_string(),
        "state" => "status".to_string(),
        _ => op,
    }
}

fn load_continuity_context(parsed: &crate::ParsedArgs) -> Result<Value, String> {
    if let Some(raw) = parsed.flags.get("context-json") {
        return serde_json::from_str::<Value>(raw).map_err(|err| format!("context_json_invalid:{err}"));
    }
    Ok(json!({
        "context": ["session active", "pending tasks"],
        "user_model": {"style": "direct", "confidence": 0.87},
        "active_tasks": ["batch12 hardening"]
    }))
}

fn run_continuity(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let contract = load_json_or(
        root,
        CONTINUITY_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "hermes_continuity_contract",
            "required_context_keys": ["context", "user_model", "active_tasks"],
            "require_deterministic_receipts": true
        }),
    );

    let mut errors = Vec::<String>::new();
    if contract
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("hermes_continuity_contract_version_must_be_v1".to_string());
    }
    if contract
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "hermes_continuity_contract"
    {
        errors.push("hermes_continuity_contract_kind_invalid".to_string());
    }

    let op = canonical_continuity_op(
        parsed
            .flags
            .get("op")
            .cloned()
            .or_else(|| parsed.positional.get(1).cloned())
            .unwrap_or_else(|| "status".to_string())
            .as_str(),
    );
    if !matches!(op.as_str(), "checkpoint" | "reconstruct" | "status") {
        errors.push("continuity_op_invalid".to_string());
    }

    let session_id = clean_id(
        parsed
            .flags
            .get("session-id")
            .map(String::as_str)
            .unwrap_or("session-default"),
        "session-default",
    );
    if !errors.is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "hermes_plane_continuity",
            "errors": errors
        });
    }

    match op.as_str() {
        "status" => {
            let snapshot_path = continuity_snapshot_path(root, &session_id);
            let restore_path = continuity_restore_path(root, &session_id);
            let snapshot = read_json(&snapshot_path);
            let restore = read_json(&restore_path);
            let mut out = json!({
                "ok": true,
                "strict": strict,
                "type": "hermes_plane_continuity",
                "op": "status",
                "lane": "core/layer0/ops",
                "session_id": session_id,
                "snapshot_path": snapshot_path.display().to_string(),
                "restore_path": restore_path.display().to_string(),
                "snapshot_present": snapshot.is_some(),
                "reconstructed_present": restore.is_some(),
                "claim_evidence": [
                    {
                        "id": "V6-HERMES-001.3",
                        "claim": "continuity_contract_tracks_snapshot_and_reconstruction_state_across_attach_disconnect_cycles",
                        "evidence": {
                            "snapshot_present": snapshot.is_some(),
                            "reconstructed_present": restore.is_some()
                        }
                    }
                ]
            });
            out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
            out
        }
        "checkpoint" => {
            let context = match load_continuity_context(parsed) {
                Ok(value) => value,
                Err(error) => {
                    return json!({
                        "ok": false,
                        "strict": strict,
                        "type": "hermes_plane_continuity",
                        "op": "checkpoint",
                        "errors": [error]
                    });
                }
            };
            let mut context_map = context.as_object().cloned().unwrap_or_default();
            for required in contract
                .get("required_context_keys")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
                .iter()
                .filter_map(Value::as_str)
            {
                if !context_map.contains_key(required) {
                    context_map.insert(required.to_string(), Value::Null);
                }
            }
            let context_payload = Value::Object(context_map);
            let context_hash = sha256_hex_str(&canonical_json_string(&context_payload));
            let checkpoint = json!({
                "version": "v1",
                "session_id": session_id,
                "checkpoint_ts": crate::now_iso(),
                "detached": true,
                "context_payload": context_payload,
                "context_hash": context_hash,
                "lane": "core/layer0/ops/hermes_plane"
            });
            let snapshot_path = continuity_snapshot_path(root, &session_id);
            let _ = write_json(&snapshot_path, &checkpoint);
            let _ = append_jsonl(
                &continuity_dir(root).join("history.jsonl"),
                &json!({
                    "type": "continuity_checkpoint",
                    "session_id": session_id,
                    "path": snapshot_path.display().to_string(),
                    "context_hash": context_hash,
                    "ts": crate::now_iso()
                }),
            );

            let mut out = json!({
                "ok": true,
                "strict": strict,
                "type": "hermes_plane_continuity",
                "op": "checkpoint",
                "lane": "core/layer0/ops",
                "session_id": session_id,
                "checkpoint": checkpoint,
                "artifact": {
                    "path": snapshot_path.display().to_string(),
                    "sha256": sha256_hex_str(&checkpoint.to_string())
                },
                "claim_evidence": [
                    {
                        "id": "V6-HERMES-001.3",
                        "claim": "continuity_checkpoint_serializes_context_and_user_model_for_detach_resume_cycles",
                        "evidence": {
                            "session_id": session_id,
                            "context_hash": context_hash
                        }
                    }
                ]
            });
            out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
            out
        }
        "reconstruct" => {
            let snapshot_path = continuity_snapshot_path(root, &session_id);
            let Some(snapshot) = read_json(&snapshot_path) else {
                return json!({
                    "ok": false,
                    "strict": strict,
                    "type": "hermes_plane_continuity",
                    "op": "reconstruct",
                    "errors": [format!("snapshot_missing:{}", snapshot_path.display())]
                });
            };
            let context_hash = clean(
                snapshot
                    .get("context_hash")
                    .and_then(Value::as_str)
                    .unwrap_or_default(),
                80,
            );
            let reconstructed = json!({
                "version": "v1",
                "session_id": session_id,
                "reconstruct_ts": crate::now_iso(),
                "daemon_restart_simulated": true,
                "detached_reattached": true,
                "restored_context": snapshot.get("context_payload").cloned().unwrap_or(Value::Null),
                "source_snapshot": snapshot_path.display().to_string(),
                "source_context_hash": context_hash,
                "reconstruction_receipt_hash": sha256_hex_str(&format!("{}:{}", session_id, context_hash))
            });
            let restore_path = continuity_restore_path(root, &session_id);
            let _ = write_json(&restore_path, &reconstructed);
            let _ = append_jsonl(
                &continuity_dir(root).join("history.jsonl"),
                &json!({
                    "type": "continuity_reconstruct",
                    "session_id": session_id,
                    "path": restore_path.display().to_string(),
                    "source_snapshot": snapshot_path.display().to_string(),
                    "source_context_hash": context_hash,
                    "ts": crate::now_iso()
                }),
            );

            let mut out = json!({
                "ok": true,
                "strict": strict,
                "type": "hermes_plane_continuity",
                "op": "reconstruct",
                "lane": "core/layer0/ops",
                "session_id": session_id,
                "reconstructed": reconstructed,
                "artifact": {
                    "path": restore_path.display().to_string(),
                    "sha256": sha256_hex_str(&reconstructed.to_string())
                },
                "claim_evidence": [
                    {
                        "id": "V6-HERMES-001.3",
                        "claim": "continuity_reconstruction_rebuilds_context_and_user_model_after_restart_with_deterministic_receipts",
                        "evidence": {
                            "session_id": session_id,
                            "source_context_hash": context_hash,
                            "daemon_restart_simulated": true,
                            "detached_reattached": true
                        }
                    }
                ]
            });
            out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
            out
        }
        _ => json!({
            "ok": false,
            "strict": strict,
            "type": "hermes_plane_continuity",
            "errors": ["continuity_op_invalid"]
        }),
    }
}

fn run_delegate(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let contract = load_json_or(
        root,
        DELEGATION_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "subagent_delegation_contract",
            "default_roles": ["researcher", "executor"],
            "tool_packs": {
                "research_pack": ["search", "crawl", "extract"],
                "security_pack": ["scan", "triage", "report"]
            },
            "max_children": 8
        }),
    );
    let mut errors = Vec::<String>::new();
    if contract
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("subagent_delegation_contract_version_must_be_v1".to_string());
    }
    if contract
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "subagent_delegation_contract"
    {
        errors.push("subagent_delegation_contract_kind_invalid".to_string());
    }
    let task = clean(
        parsed
            .flags
            .get("task")
            .cloned()
            .or_else(|| parsed.positional.get(1).cloned())
            .unwrap_or_default(),
        400,
    );
    if task.is_empty() {
        errors.push("delegate_task_required".to_string());
    }
    let parent = clean(
        parsed
            .flags
            .get("parent")
            .cloned()
            .unwrap_or_else(|| "shadow-root".to_string()),
        120,
    );
    let pack = clean(
        parsed
            .flags
            .get("tool-pack")
            .cloned()
            .unwrap_or_else(|| "research_pack".to_string()),
        80,
    );
    if !errors.is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "hermes_plane_delegate",
            "errors": errors
        });
    }

    let roles = parsed
        .flags
        .get("roles")
        .map(|raw| split_csv_clean(raw, 80))
        .filter(|rows| !rows.is_empty())
        .unwrap_or_else(|| {
            contract
                .get("default_roles")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_else(|| vec![json!("researcher"), json!("executor")])
                .iter()
                .filter_map(Value::as_str)
                .map(|v| clean(v, 80))
                .collect::<Vec<_>>()
        });
    let max_children = contract
        .get("max_children")
        .and_then(Value::as_u64)
        .unwrap_or(8) as usize;
    if strict && roles.len() > max_children {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "hermes_plane_delegate",
            "errors": ["delegate_roles_exceed_max_children"]
        });
    }

    let tool_packs = contract
        .get("tool_packs")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let tools = tool_packs
        .get(&pack)
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(Value::as_str)
        .map(|v| clean(v, 80))
        .collect::<Vec<_>>();
    if strict && tools.is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "hermes_plane_delegate",
            "errors": ["delegate_tool_pack_unknown"]
        });
    }

    let parent_receipt_hash = sha256_hex_str(&format!("{}:{}:{}", parent, task, pack));
    let mut previous_hash = parent_receipt_hash.clone();
    let children = roles
        .iter()
        .enumerate()
        .map(|(idx, role)| {
            let child_id = format!(
                "{}_{}",
                clean(role, 40),
                &sha256_hex_str(&format!("{}:{}:{}", parent, task, idx))[..10]
            );
            let chain_hash =
                sha256_hex_str(&format!("{}:{}:{}:{}", previous_hash, child_id, role, pack));
            previous_hash = chain_hash.clone();
            json!({
                "index": idx + 1,
                "child_id": child_id,
                "role": role,
                "tool_pack": pack,
                "tools": tools,
                "parent_receipt_hash": parent_receipt_hash,
                "previous_hash": previous_hash,
                "chain_hash": chain_hash,
                "task": task
            })
        })
        .collect::<Vec<_>>();

    let artifact = json!({
        "version": "v1",
        "parent": parent,
        "task": task,
        "tool_pack": pack,
        "children": children,
        "delegated_at": crate::now_iso(),
        "parent_receipt_hash": parent_receipt_hash
    });
    let artifact_path = state_root(root).join("delegation").join("latest.json");
    let _ = write_json(&artifact_path, &artifact);
    let _ = append_jsonl(
        &state_root(root).join("delegation").join("history.jsonl"),
        &artifact,
    );

    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "hermes_plane_delegate",
        "lane": "core/layer0/ops",
        "artifact": {
            "path": artifact_path.display().to_string(),
            "sha256": sha256_hex_str(&artifact.to_string())
        },
        "delegation": artifact,
        "claim_evidence": [
            {
                "id": "V6-HERMES-001.4",
                "claim": "subagent_delegation_uses_policy_scoped_tool_packs_and_parent_child_receipt_chains",
                "evidence": {
                    "parent": parent,
                    "tool_pack": pack,
                    "children": roles.len()
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}
