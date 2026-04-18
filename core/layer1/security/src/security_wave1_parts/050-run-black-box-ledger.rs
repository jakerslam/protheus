
fn verify_black_box_export(export_path: &Path) -> Result<Value, String> {
    let export = read_json_or(export_path, json!({}));
    let entries = export
        .get("entries")
        .and_then(Value::as_array)
        .cloned()
        .ok_or_else(|| "offline_export_entries_missing".to_string())?;
    if entries.is_empty() {
        return Err("offline_export_empty".to_string());
    }
    let mut prev_hash = "GENESIS".to_string();
    let mut last_hash = "GENESIS".to_string();
    let mut last_seq = 0u64;
    for entry in &entries {
        let seq = entry.get("seq").and_then(Value::as_u64).unwrap_or_default();
        let ts = entry.get("ts").and_then(Value::as_str).unwrap_or_default();
        let actor = entry
            .get("actor")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let action = entry
            .get("action")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let source = entry
            .get("source")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let stored_prev = entry
            .get("prev_hash")
            .and_then(Value::as_str)
            .unwrap_or("GENESIS");
        let signature = entry
            .get("signature")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let stored_hash = entry
            .get("entry_hash")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if seq == 0 {
            return Err("offline_seq_invalid:seq=0".to_string());
        }
        if seq != last_seq + 1 {
            return Err(format!("offline_seq_gap_or_reorder:seq={seq}:last_seq={last_seq}"));
        }
        if ts.is_empty() || actor.is_empty() || action.is_empty() || source.is_empty() {
            return Err(format!("offline_required_field_missing:seq={seq}"));
        }
        if stored_prev != prev_hash {
            return Err(format!("offline_prev_hash_mismatch:seq={seq}"));
        }
        let ciphertext_digest = entry
            .get("ciphertext_digest")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if ciphertext_digest.is_empty() {
            return Err(format!("offline_ciphertext_digest_missing:seq={seq}"));
        }
        let calc_signature = sha256_hex(&format!(
            "{ts}|{actor}|{action}|{source}|{stored_prev}|{ciphertext_digest}"
        ));
        let calc_hash = sha256_hex(&stable_json_string(&json!({
            "ts": ts,
            "actor": actor,
            "action": action,
            "source": source,
            "prev_hash": stored_prev,
            "signature": calc_signature,
            "ciphertext_digest": ciphertext_digest
        })));
        if calc_hash != stored_hash || calc_signature != signature {
            return Err(format!("offline_hash_or_signature_mismatch:seq={seq}"));
        }
        prev_hash = stored_hash.to_string();
        last_hash = stored_hash.to_string();
        last_seq = seq;
    }
    if let Some(root) = export
        .get("published_roots")
        .and_then(Value::as_array)
        .and_then(|rows| rows.last())
    {
        if root.get("root_hash").and_then(Value::as_str) != Some(last_hash.as_str()) {
            return Err("offline_published_root_hash_mismatch".to_string());
        }
    }
    Ok(json!({
        "ok": true,
        "type": "black_box_ledger_verify_offline",
        "valid": true,
        "entry_count": entries.len(),
        "last_seq": last_seq,
        "last_hash": last_hash,
        "export_path": normalize_rel_path(export_path.to_string_lossy()),
    }))
}

pub fn run_black_box_ledger(repo_root: &Path, argv: &[String]) -> (Value, i32) {
    let args = parse_cli_args(argv);
    let cmd = args
        .positional
        .first()
        .map(|v| normalize_token(v, 80))
        .unwrap_or_else(|| "status".to_string());
    let (ledger_dir, spine_dir, autonomy_dir, attest_dir) = black_box_paths(repo_root);
    let chain_path = black_box_chain_path(&ledger_dir);
    let sqlite_path = black_box_sqlite_path(&ledger_dir);
    let default_export_path = black_box_export_path(&ledger_dir);

    if cmd == "append" {
        return append_black_box_entry(repo_root, &args, &ledger_dir);
    } else if cmd == "export" {
        let export_path = args
            .flags
            .get("export-path")
            .map(PathBuf::from)
            .unwrap_or_else(|| default_export_path.clone());
        return export_black_box_sqlite(&ledger_dir, &export_path);
    } else if cmd == "verify_offline" || cmd == "verify-offline" {
        let export_path = args
            .flags
            .get("export-path")
            .map(PathBuf::from)
            .unwrap_or_else(|| default_export_path.clone());
        return match verify_black_box_export(&export_path) {
            Ok(payload) => (payload, 0),
            Err(err) => (
                json!({
                    "ok": false,
                    "type": "black_box_ledger_verify_offline",
                    "error": err,
                    "export_path": normalize_rel_path(export_path.to_string_lossy())
                }),
                1,
            ),
        };
    }

    if cmd == "rollup" {
        let date = date_arg_or_today(args.positional.get(1));
        let mode = clean_text(
            args.flags
                .get("mode")
                .cloned()
                .unwrap_or_else(|| "daily".to_string()),
            40,
        );
        let chain_rows = read_jsonl(&chain_path);
        let seq = next_rollup_seq(&chain_rows, &date);
        let detail_path = black_box_detail_path(&ledger_dir, &date, seq);
        let (events, spine_count, autonomy_count, external_count) =
            load_critical_events(&date, &spine_dir, &autonomy_dir, &attest_dir);
        let (_detail_rows, digest) = match black_box_write_detail(&date, &events, &detail_path) {
            Ok(v) => v,
            Err(err) => {
                return (
                    json!({
                        "ok": false,
                        "type": "black_box_ledger_rollup",
                        "error": format!("detail_write_failed:{err}")
                    }),
                    1,
                );
            }
        };

        let prev_hash = chain_rows
            .last()
            .and_then(|row| row.get("hash"))
            .and_then(Value::as_str)
            .unwrap_or("GENESIS")
            .to_string();
        let mut chain_row = json!({
            "ts": now_iso(),
            "date": date,
            "mode": mode,
            "rollup_seq": seq,
            "detail_file": detail_path.file_name().and_then(|v| v.to_str()).unwrap_or_default(),
            "digest": digest,
            "spine_events": spine_count,
            "autonomy_events": autonomy_count,
            "external_events": external_count,
            "total_events": events.len(),
            "prev_hash": prev_hash
        });
        let hash = sha256_hex(&stable_json_string(&chain_row));
        chain_row["hash"] = Value::String(hash);

        let mut next_chain = chain_rows;
        next_chain.push(chain_row.clone());
        if let Err(err) = write_jsonl_rows(&chain_path, &next_chain) {
            return (
                json!({
                    "ok": false,
                    "type": "black_box_ledger_rollup",
                    "error": format!("chain_write_failed:{err}")
                }),
                1,
            );
        }

        (
            json!({
                "ok": true,
                "type": "black_box_ledger_rollup",
                "date": date,
                "mode": mode,
                "rollup_seq": seq,
                "spine_events": spine_count,
                "autonomy_events": autonomy_count,
                "external_events": external_count,
                "total_events": events.len(),
                "detail_path": normalize_rel_path(detail_path.to_string_lossy()),
                "digest": chain_row.get("digest").cloned().unwrap_or(Value::Null)
            }),
            0,
        )
    } else if cmd == "verify" {
        let sqlite_verify = verify_black_box_sqlite(&ledger_dir);
        let chain_rows = read_jsonl(&chain_path);
        if chain_rows.is_empty() {
            return match sqlite_verify {
                Ok(payload) => (payload, 0),
                Err(err) => (
                    json!({
                        "ok": false,
                        "type": "black_box_ledger_verify",
                        "error": err,
                        "sqlite_path": normalize_rel_path(sqlite_path.to_string_lossy())
                    }),
                    1,
                ),
            };
        }
        if let Err(err) = sqlite_verify {
            return (
                json!({
                    "ok": false,
                    "type": "black_box_ledger_verify",
                    "error": err,
                    "sqlite_path": normalize_rel_path(sqlite_path.to_string_lossy())
                }),
                1,
            );
        }
        let mut prev_hash = "GENESIS".to_string();
        for (idx, row) in chain_rows.iter().enumerate() {
            let expected_prev = row
                .get("prev_hash")
                .and_then(Value::as_str)
                .unwrap_or("GENESIS")
                .to_string();
            if expected_prev != prev_hash {
                return (
                    json!({
                        "ok": false,
                        "type": "black_box_ledger_verify",
                        "error": "chain_prev_hash_mismatch",
                        "index": idx
                    }),
                    1,
                );
            }
            let mut payload = row.clone();
            payload.as_object_mut().map(|m| {
                m.remove("hash");
            });
            let calc = sha256_hex(&stable_json_string(&payload));
            let stored = row.get("hash").and_then(Value::as_str).unwrap_or("");
            if calc != stored {
                return (
                    json!({
                        "ok": false,
                        "type": "black_box_ledger_verify",
                        "error": "chain_hash_mismatch",
                        "index": idx
                    }),
                    1,
                );
            }
            let date = row.get("date").and_then(Value::as_str).unwrap_or("");
            let seq = row.get("rollup_seq").and_then(Value::as_u64).unwrap_or(1) as usize;
            let detail_path = black_box_detail_path(&ledger_dir, date, seq);
            let detail_rows = read_jsonl(&detail_path);
            let mut detail_prev = "GENESIS".to_string();
            for (detail_idx, detail_row) in detail_rows.iter().enumerate() {
                let dprev = detail_row
                    .get("prev_hash")
                    .and_then(Value::as_str)
                    .unwrap_or("GENESIS")
                    .to_string();
                if dprev != detail_prev {
                    return (
                        json!({
                            "ok": false,
                            "type": "black_box_ledger_verify",
                            "error": "detail_prev_hash_mismatch",
                            "date": date,
                            "index": detail_idx
                        }),
                        1,
                    );
                }
                let mut detail_payload = detail_row.clone();
                detail_payload.as_object_mut().map(|m| {
                    m.remove("hash");
                });
                let dcalc = sha256_hex(&stable_json_string(&detail_payload));
                let dstored = detail_row.get("hash").and_then(Value::as_str).unwrap_or("");
                if dcalc != dstored {
                    return (
                        json!({
                            "ok": false,
                            "type": "black_box_ledger_verify",
                            "error": "detail_hash_mismatch",
                            "date": date,
                            "index": detail_idx
                        }),
                        1,
                    );
                }
                detail_prev = dstored.to_string();
            }
            let detail_digest = detail_rows
                .last()
                .and_then(|v| v.get("hash"))
                .and_then(Value::as_str)
                .map(|v| v.to_string())
                .unwrap_or_else(|| {
                    sha256_hex(&stable_json_string(&json!({"date": date, "empty": true})))
                });
            let row_digest = row.get("digest").and_then(Value::as_str).unwrap_or("");
            if detail_digest != row_digest {
                return (
                    json!({
                        "ok": false,
                        "type": "black_box_ledger_verify",
                        "error": "digest_mismatch",
                        "date": date
                    }),
                    1,
                );
            }
            prev_hash = stored.to_string();
        }
        (
            json!({
                "ok": true,
                "type": "black_box_ledger_verify",
                "valid": true,
                "chain_length": chain_rows.len(),
                "sqlite_path": normalize_rel_path(sqlite_path.to_string_lossy())
            }),
            0,
        )
    } else if cmd == "status" {
        let chain_rows = read_jsonl(&chain_path);
        let sqlite_status = verify_black_box_sqlite(&ledger_dir).ok();
        let last = chain_rows.last().cloned().unwrap_or_else(|| json!({}));
        (
            json!({
                "ok": !chain_rows.is_empty() || sqlite_status.is_some(),
                "type": "black_box_ledger_status",
                "chain_length": chain_rows.len(),
                "last_date": last.get("date").cloned().unwrap_or(Value::Null),
                "last_digest": last.get("digest").cloned().unwrap_or(Value::Null),
                "last_rollup_seq": last.get("rollup_seq").cloned().unwrap_or(Value::Null),
                "sqlite_path": normalize_rel_path(sqlite_path.to_string_lossy()),
                "sqlite_chain_length": sqlite_status
                    .as_ref()
                    .and_then(|v| v.get("sqlite_chain_length"))
                    .cloned()
                    .unwrap_or(Value::Null),
                "published_root": sqlite_status
                    .as_ref()
                    .and_then(|v| v.get("published_root"))
                    .cloned()
                    .unwrap_or(Value::Null),
                "encrypted_at_rest": sqlite_status.is_some(),
            }),
            if !chain_rows.is_empty() || sqlite_status.is_some() {
                0
            } else {
                1
            },
        )
    } else {
        (
            json!({
                "ok": false,
                "type": "black_box_ledger_error",
                "error": format!("unknown_command:{cmd}")
            }),
            2,
        )
    }
}

// -------------------------------------------------------------------------------------------------
// Goal Preservation Kernel
// -------------------------------------------------------------------------------------------------

fn goal_preservation_policy_path(repo_root: &Path, args: &CliArgs) -> PathBuf {
    if let Some(v) = args.flags.get("policy") {
        let p = PathBuf::from(v);
        if p.is_absolute() {
            return p;
        }
        return repo_root.join(p);
    }
    std::env::var("GOAL_PRESERVATION_POLICY_PATH")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| runtime_config_path(repo_root, "goal_preservation_policy.json"))
}

fn goal_preservation_default_policy() -> Value {
    json!({
        "version": "1.0",
        "strict_mode": true,
        "constitution_path": "docs/workspace/AGENT-CONSTITUTION.md",
        "protected_axiom_markers": [
            "to be a hero",
            "to test the limits",
            "to create at will",
            "to win my freedom",
            "user sovereignty",
            "root constitution"
        ],
        "blocked_mutation_paths": [
            "^AGENT-CONSTITUTION\\.md$",
            "^SOUL\\.md$",
            "^USER\\.md$",
            "^client/runtime/systems/security/guard\\.(ts|js)$",
            "^client/runtime/systems/security/policy_rootd\\.(ts|js)$"
        ],
        "symbiosis_recursion_gate": {
            "enabled": true,
            "shadow_only": true,
            "signal_policy_path": "client/runtime/config/symbiosis_coherence_policy.json"
        },
        "output": {
            "state_path": "local/state/security/goal_preservation/latest.json",
            "receipts_path": "local/state/security/goal_preservation/receipts.jsonl"
        }
    })
}

fn goal_preservation_load_policy(policy_path: &Path) -> Value {
    let raw = read_json_or(policy_path, json!({}));
    let mut policy = goal_preservation_default_policy();
    if let Some(version) = raw.get("version").and_then(Value::as_str) {
        policy["version"] = Value::String(clean_text(version, 40));
    }
    if let Some(strict) = raw.get("strict_mode").and_then(Value::as_bool) {
        policy["strict_mode"] = Value::Bool(strict);
    }
    if let Some(path) = raw.get("constitution_path").and_then(Value::as_str) {
        policy["constitution_path"] = Value::String(clean_text(path, 320));
    }
    if let Some(markers) = raw.get("protected_axiom_markers").and_then(Value::as_array) {
        policy["protected_axiom_markers"] = Value::Array(
            markers
                .iter()
                .filter_map(Value::as_str)
                .map(|v| Value::String(clean_text(v, 240).to_ascii_lowercase()))
                .collect::<Vec<_>>(),
        );
    }
    if let Some(blocked) = raw.get("blocked_mutation_paths").and_then(Value::as_array) {
        policy["blocked_mutation_paths"] = Value::Array(
            blocked
                .iter()
                .filter_map(Value::as_str)
                .map(|v| Value::String(clean_text(v, 260)))
                .collect::<Vec<_>>(),
        );
    }
    if let Some(gate) = raw
        .get("symbiosis_recursion_gate")
        .and_then(Value::as_object)
    {
        if let Some(enabled) = gate.get("enabled").and_then(Value::as_bool) {
            policy["symbiosis_recursion_gate"]["enabled"] = Value::Bool(enabled);
        }
        if let Some(shadow_only) = gate.get("shadow_only").and_then(Value::as_bool) {
            policy["symbiosis_recursion_gate"]["shadow_only"] = Value::Bool(shadow_only);
        }
        if let Some(path) = gate.get("signal_policy_path").and_then(Value::as_str) {
            policy["symbiosis_recursion_gate"]["signal_policy_path"] =
                Value::String(clean_text(path, 320));
        }
    }
    if let Some(out) = raw.get("output").and_then(Value::as_object) {
        if let Some(path) = out.get("state_path").and_then(Value::as_str) {
            policy["output"]["state_path"] = Value::String(clean_text(path, 320));
        }
        if let Some(path) = out.get("receipts_path").and_then(Value::as_str) {
            policy["output"]["receipts_path"] = Value::String(clean_text(path, 320));
        }
    }
    policy
}
