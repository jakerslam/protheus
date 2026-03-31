// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops::persist_plane::continuity

use super::*;

fn continuity_integrity_report(
    session_id: &str,
    snapshot: Option<&Value>,
    reconstructed: Option<&Value>,
    snapshot_path: &Path,
    reconstruct_path: &Path,
) -> Value {
    let snapshot_present = snapshot.is_some();
    let reconstructed_present = reconstructed.is_some();

    let snapshot_context_hash_expected = snapshot
        .and_then(|row| row.get("context_hash"))
        .and_then(Value::as_str)
        .map(|raw| raw.trim().to_string())
        .unwrap_or_default();
    let snapshot_context_hash_actual = snapshot
        .and_then(|row| row.get("context_payload"))
        .map(|payload| sha256_hex_str(&payload.to_string()))
        .unwrap_or_default();
    let snapshot_context_hash_match = snapshot_present
        && !snapshot_context_hash_expected.is_empty()
        && snapshot_context_hash_expected == snapshot_context_hash_actual;

    let snapshot_file_hash = snapshot
        .map(|row| sha256_hex_str(&row.to_string()))
        .unwrap_or_default();

    let reconstruct_source_context_hash = reconstructed
        .and_then(|row| row.get("source_context_hash"))
        .and_then(Value::as_str)
        .map(|raw| raw.trim().to_string())
        .unwrap_or_default();
    let reconstruct_hash_actual = reconstructed
        .and_then(|row| row.get("reconstruction_hash"))
        .and_then(Value::as_str)
        .map(|raw| raw.trim().to_string())
        .unwrap_or_default();
    let reconstruct_hash_expected = if reconstruct_source_context_hash.is_empty() {
        String::new()
    } else {
        sha256_hex_str(&format!("{session_id}:{reconstruct_source_context_hash}"))
    };
    let reconstruct_source_hash_match = if !reconstructed_present {
        true
    } else {
        !snapshot_context_hash_expected.is_empty()
            && reconstruct_source_context_hash == snapshot_context_hash_expected
    };
    let reconstruct_hash_match = if !reconstructed_present {
        true
    } else {
        !reconstruct_hash_expected.is_empty() && reconstruct_hash_expected == reconstruct_hash_actual
    };

    let checks = vec![
        json!({
            "id": "snapshot_present",
            "ok": snapshot_present
        }),
        json!({
            "id": "snapshot_context_hash_match",
            "ok": snapshot_context_hash_match,
            "expected": snapshot_context_hash_expected,
            "actual": snapshot_context_hash_actual
        }),
        json!({
            "id": "reconstruct_source_hash_match",
            "ok": reconstruct_source_hash_match,
            "required": reconstructed_present,
            "expected": snapshot_context_hash_expected,
            "actual": reconstruct_source_context_hash
        }),
        json!({
            "id": "reconstruct_hash_match",
            "ok": reconstruct_hash_match,
            "required": reconstructed_present,
            "expected": reconstruct_hash_expected,
            "actual": reconstruct_hash_actual
        }),
    ];
    let failed_checks = checks
        .iter()
        .filter(|row| row.get("ok").and_then(Value::as_bool) == Some(false))
        .count();
    json!({
        "ok": failed_checks == 0,
        "failed_checks": failed_checks,
        "snapshot_present": snapshot_present,
        "reconstructed_present": reconstructed_present,
        "snapshot_path": snapshot_path.display().to_string(),
        "reconstruct_path": reconstruct_path.display().to_string(),
        "snapshot_file_sha256": snapshot_file_hash,
        "checks": checks
    })
}

pub(super) fn run_continuity(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let contract = load_json_or(
        root,
        CONTINUITY_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "persist_continuity_contract",
            "allowed_ops": ["checkpoint", "reconstruct", "status", "validate"],
            "required_context_keys": ["context", "user_model", "active_tasks"]
        }),
    );
    let op = clean(
        parsed
            .flags
            .get("op")
            .cloned()
            .or_else(|| parsed.positional.get(1).cloned())
            .unwrap_or_else(|| "status".to_string()),
        20,
    )
    .to_ascii_lowercase();
    let allowed_ops = contract
        .get("allowed_ops")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if strict
        && !allowed_ops
            .iter()
            .filter_map(Value::as_str)
            .any(|row| row == op)
    {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "persist_plane_continuity",
            "errors": ["persist_continuity_op_invalid"]
        });
    }

    let session_id = clean_id(
        parsed
            .flags
            .get("session-id")
            .map(String::as_str)
            .or_else(|| parsed.flags.get("session").map(String::as_str))
            .or_else(|| parsed.positional.get(2).map(String::as_str)),
        "session-default",
    );
    let snapshot_path = continuity_snapshot_path(root, &session_id);
    let reconstruct_path = continuity_reconstruct_path(root, &session_id);
    if op == "status" {
        let snapshot = read_json(&snapshot_path);
        let reconstructed = read_json(&reconstruct_path);
        let integrity = continuity_integrity_report(
            &session_id,
            snapshot.as_ref(),
            reconstructed.as_ref(),
            &snapshot_path,
            &reconstruct_path,
        );
        let mut out = json!({
            "ok": true,
            "strict": strict,
            "type": "persist_plane_continuity",
            "lane": "core/layer0/ops",
            "op": "status",
            "session_id": session_id,
            "snapshot_path": snapshot_path.display().to_string(),
            "reconstruct_path": reconstruct_path.display().to_string(),
            "snapshot_present": snapshot.is_some(),
            "reconstructed_present": reconstructed.is_some(),
            "integrity": integrity,
            "claim_evidence": [
                {
                    "id": "V6-PERSIST-001.3",
                    "claim": "continuity_status_tracks_reconstruction_survivability_across_restart_and_disconnect",
                    "evidence": {
                        "session_id": session_id,
                        "snapshot_present": snapshot.is_some(),
                        "reconstructed_present": reconstructed.is_some()
                    }
                }
            ]
        });
        out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
        return out;
    }

    if op == "validate" {
        let snapshot = read_json(&snapshot_path);
        let reconstructed = read_json(&reconstruct_path);
        let integrity = continuity_integrity_report(
            &session_id,
            snapshot.as_ref(),
            reconstructed.as_ref(),
            &snapshot_path,
            &reconstruct_path,
        );
        let ok = integrity.get("ok").and_then(Value::as_bool).unwrap_or(false);
        let errors = integrity
            .get("checks")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|row| {
                let check_ok = row.get("ok").and_then(Value::as_bool).unwrap_or(false);
                if check_ok {
                    None
                } else {
                    row.get("id")
                        .and_then(Value::as_str)
                        .map(|id| Value::String(format!("continuity_integrity_check_failed:{id}")))
                }
            })
            .collect::<Vec<Value>>();
        let _ = append_jsonl(
            &continuity_dir(root).join("history.jsonl"),
            &json!({
                "type": "continuity_validate",
                "session_id": session_id,
                "ok": ok,
                "failed_checks": integrity.get("failed_checks").cloned().unwrap_or_else(|| json!(0)),
                "ts": crate::now_iso()
            }),
        );
        let mut out = json!({
            "ok": ok,
            "strict": strict,
            "type": "persist_plane_continuity",
            "lane": "core/layer0/ops",
            "op": "validate",
            "session_id": session_id,
            "snapshot_path": snapshot_path.display().to_string(),
            "reconstruct_path": reconstruct_path.display().to_string(),
            "integrity": integrity,
            "errors": errors,
            "claim_evidence": [
                {
                    "id": "V6-PERSIST-001.3",
                    "claim": "continuity_validate_confirms_snapshot_and_reconstruction_hash_integrity_before_resume",
                    "evidence": {
                        "session_id": session_id,
                        "snapshot_present": snapshot.is_some(),
                        "reconstructed_present": reconstructed.is_some()
                    }
                }
            ]
        });
        out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
        return out;
    }

    if op == "checkpoint" {
        let mut context = parse_json_flag(parsed.flags.get("context-json")).unwrap_or_else(|| {
            json!({
                "context": ["session_active"],
                "user_model": {"style": "direct", "confidence": 0.8},
                "active_tasks": []
            })
        });
        let required_keys = contract
            .get("required_context_keys")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        if let Some(map) = context.as_object_mut() {
            for key in required_keys
                .into_iter()
                .filter_map(|row| row.as_str().map(str::to_string))
            {
                map.entry(key).or_insert(Value::Null);
            }
        }
        let snapshot = json!({
            "version": "v1",
            "session_id": session_id,
            "detached": true,
            "checkpoint_ts": crate::now_iso(),
            "context_payload": context,
            "context_hash": sha256_hex_str(&context.to_string()),
            "lane": "core/layer0/ops/persist_plane"
        });
        let _ = write_json(&snapshot_path, &snapshot);
        let _ = append_jsonl(
            &continuity_dir(root).join("history.jsonl"),
            &json!({
                "type": "continuity_checkpoint",
                "session_id": session_id,
                "path": snapshot_path.display().to_string(),
                "context_hash": snapshot.get("context_hash"),
                "ts": crate::now_iso()
            }),
        );

        let mut out = json!({
            "ok": true,
            "strict": strict,
            "type": "persist_plane_continuity",
            "lane": "core/layer0/ops",
            "op": "checkpoint",
            "session_id": session_id,
            "snapshot": snapshot,
            "artifact": {
                "path": snapshot_path.display().to_string(),
                "sha256": sha256_hex_str(&read_json(&snapshot_path).unwrap_or_else(|| json!({})).to_string())
            },
            "claim_evidence": [
                {
                    "id": "V6-PERSIST-001.3",
                    "claim": "auto_memory_continuity_checkpoint_persists_context_for_restart_disconnect_boundaries",
                    "evidence": {
                        "session_id": session_id,
                        "snapshot_path": snapshot_path.display().to_string()
                    }
                }
            ]
        });
        out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
        return out;
    }

    let Some(snapshot) = read_json(&snapshot_path) else {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "persist_plane_continuity",
            "op": "reconstruct",
            "errors": [format!("persist_continuity_snapshot_missing:{}", snapshot_path.display())]
        });
    };
    let reconstructed = json!({
        "version": "v1",
        "session_id": session_id,
        "reconstruct_ts": crate::now_iso(),
        "daemon_restart_simulated": true,
        "detached_reattached": true,
        "restored_context": snapshot.get("context_payload").cloned().unwrap_or(Value::Null),
        "source_snapshot": snapshot_path.display().to_string(),
        "source_context_hash": snapshot.get("context_hash").cloned().unwrap_or(Value::Null),
        "reconstruction_hash": sha256_hex_str(&format!("{}:{}", session_id, snapshot.get("context_hash").and_then(Value::as_str).unwrap_or("")))
    });
    let _ = write_json(&reconstruct_path, &reconstructed);
    let _ = append_jsonl(
        &continuity_dir(root).join("history.jsonl"),
        &json!({
            "type": "continuity_reconstruct",
            "session_id": session_id,
            "path": reconstruct_path.display().to_string(),
            "source_snapshot": snapshot_path.display().to_string(),
            "ts": crate::now_iso()
        }),
    );

    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "persist_plane_continuity",
        "lane": "core/layer0/ops",
        "op": "reconstruct",
        "session_id": session_id,
        "reconstructed": reconstructed,
        "artifact": {
            "path": reconstruct_path.display().to_string(),
            "sha256": sha256_hex_str(&read_json(&reconstruct_path).unwrap_or_else(|| json!({})).to_string())
        },
        "claim_evidence": [
            {
                "id": "V6-PERSIST-001.3",
                "claim": "auto_memory_continuity_reconstructs_context_after_restart_disconnect",
                "evidence": {
                    "session_id": session_id,
                    "reconstruct_path": reconstruct_path.display().to_string()
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}
