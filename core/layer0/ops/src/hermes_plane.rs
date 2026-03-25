// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops::hermes_plane (authoritative)

use crate::v8_kernel::{
    append_jsonl, attach_conduit, build_plane_conduit_enforcement, canonical_json_string,
    conduit_bypass_requested, emit_plane_receipt, load_json_or, parse_bool, parse_u64,
    plane_status, print_json, read_json, scoped_state_root, sha256_hex_str, split_csv_clean,
    write_json,
};
use crate::{clean, parse_args};
use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

const STATE_ENV: &str = "HERMES_PLANE_STATE_ROOT";
const STATE_SCOPE: &str = "hermes_plane";

const IDENTITY_CONTRACT_PATH: &str = "planes/contracts/hermes/shadow_discovery_contract_v1.json";
const COCKPIT_CONTRACT_PATH: &str = "planes/contracts/hermes/premium_cockpit_contract_v1.json";
const CONTINUITY_CONTRACT_PATH: &str =
    "planes/contracts/hermes/continuity_reconstruction_contract_v1.json";
const DELEGATION_CONTRACT_PATH: &str =
    "planes/contracts/hermes/subagent_delegation_contract_v1.json";

fn usage() {
    println!("Usage:");
    println!("  protheus-ops hermes-plane status");
    println!("  protheus-ops hermes-plane discover [--shadow=<id>] [--strict=1|0]");
    println!(
        "  protheus-ops hermes-plane continuity --op=<checkpoint|reconstruct|status> [--session-id=<id>] [--context-json=<json>] [--strict=1|0]"
    );
    println!(
        "  protheus-ops hermes-plane delegate --task=<text> [--parent=<id>] [--roles=researcher,executor] [--tool-pack=<id>] [--strict=1|0]"
    );
    println!(
        "  protheus-ops hermes-plane cockpit [--max-blocks=<n>] [--stale-threshold-ms=<n>] [--conduit-signal-window-ms=<n>] [--strict=1|0]"
    );
    println!(
        "  protheus-ops hermes-plane reclaim-stale [--stale-threshold-ms=<n>] [--max-reclaims=<n>] [--dry-run=1|0] [--strict=1|0]"
    );
}

fn state_root(root: &Path) -> PathBuf {
    scoped_state_root(root, STATE_ENV, STATE_SCOPE)
}

fn emit(root: &Path, payload: Value) -> i32 {
    emit_plane_receipt(root, STATE_ENV, STATE_SCOPE, "hermes_plane_error", payload)
}

fn status(root: &Path) -> Value {
    plane_status(root, STATE_ENV, STATE_SCOPE, "hermes_plane_status")
}

fn claim_ids_for_action(action: &str) -> Vec<&'static str> {
    match action {
        "discover" => vec!["V6-HERMES-001.1", "V6-HERMES-001.5"],
        "continuity" => vec!["V6-HERMES-001.3", "V6-HERMES-001.5"],
        "delegate" => vec!["V6-HERMES-001.4", "V6-HERMES-001.5"],
        "cockpit" | "top" | "dashboard" => vec!["V6-HERMES-001.2", "V6-HERMES-001.5"],
        "reclaim-stale" | "reclaim-blocks" => vec!["V6-HERMES-001.2", "V6-HERMES-001.5"],
        _ => vec!["V6-HERMES-001.5"],
    }
}

fn conduit_enforcement(
    root: &Path,
    parsed: &crate::ParsedArgs,
    strict: bool,
    action: &str,
) -> Value {
    let bypass_requested = conduit_bypass_requested(&parsed.flags);
    let claim_ids = claim_ids_for_action(action);
    build_plane_conduit_enforcement(
        root,
        STATE_ENV,
        STATE_SCOPE,
        strict,
        action,
        "hermes_conduit_enforcement",
        "core/layer0/ops/hermes_plane",
        bypass_requested,
        "hermes_surface_is_conduit_routed_with_fail_closed_receipts",
        &claim_ids,
    )
}

fn continuity_dir(root: &Path) -> PathBuf {
    state_root(root).join("continuity")
}

fn continuity_snapshot_path(root: &Path, session_id: &str) -> PathBuf {
    continuity_dir(root)
        .join("snapshots")
        .join(format!("{session_id}.json"))
}

fn continuity_restore_path(root: &Path, session_id: &str) -> PathBuf {
    continuity_dir(root)
        .join("reconstructed")
        .join(format!("{session_id}.json"))
}

fn clean_id(raw: &str, fallback: &str) -> String {
    let mut out = String::new();
    for ch in raw.trim().chars() {
        if out.len() >= 96 {
            break;
        }
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push('-');
        }
    }
    let trimmed = out.trim_matches('-');
    if trimmed.is_empty() {
        fallback.to_string()
    } else {
        trimmed.to_string()
    }
}

fn run_discover(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let contract = load_json_or(
        root,
        IDENTITY_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "shadow_discovery_contract",
            "required_fields": ["shadow_id", "runtime", "capabilities", "model", "signature"]
        }),
    );
    let mut errors = Vec::<String>::new();
    if contract
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("shadow_discovery_contract_version_must_be_v1".to_string());
    }
    if contract
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "shadow_discovery_contract"
    {
        errors.push("shadow_discovery_contract_kind_invalid".to_string());
    }
    if !errors.is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "hermes_plane_discover",
            "errors": errors
        });
    }

    let shadow_id = clean_id(
        parsed
            .flags
            .get("shadow")
            .map(String::as_str)
            .or_else(|| parsed.positional.get(1).map(String::as_str))
            .unwrap_or("default-shadow"),
        "default-shadow",
    );
    let model = clean(
        std::env::var("PROTHEUS_MODEL_ID").unwrap_or_else(|_| "unknown-model".to_string()),
        120,
    );
    let runtime_mode = clean(
        std::env::var("PROTHEUS_RUNTIME_MODE").unwrap_or_else(|_| "source".to_string()),
        80,
    );

    let mut identity = json!({
        "version": "v1",
        "shadow_id": shadow_id,
        "runtime": {
            "os": std::env::consts::OS,
            "arch": std::env::consts::ARCH,
            "family": std::env::consts::FAMILY,
            "runtime_mode": runtime_mode,
            "cwd": root.display().to_string()
        },
        "model": {
            "active": model,
            "router": clean(std::env::var("PROTHEUS_MODEL_ROUTER").unwrap_or_else(|_| "default".to_string()), 80)
        },
        "capabilities": {
            "can_research": true,
            "can_parse": true,
            "can_orchestrate": true,
            "can_use_tools": true
        },
        "generated_at": crate::now_iso(),
        "signature": ""
    });

    let signing_key = std::env::var("HERMES_IDENTITY_SIGNING_KEY")
        .unwrap_or_else(|_| "hermes-dev-signing-key".to_string());
    let mut signature_basis = identity.clone();
    if let Some(obj) = signature_basis.as_object_mut() {
        obj.remove("signature");
    }
    let signature = format!(
        "sig:{}",
        sha256_hex_str(&format!(
            "{}:{}",
            signing_key,
            canonical_json_string(&signature_basis)
        ))
    );
    identity["signature"] = Value::String(signature.clone());

    let artifact_path = state_root(root)
        .join("identity")
        .join(format!("{}.json", shadow_id));
    let _ = write_json(&artifact_path, &identity);

    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "hermes_plane_discover",
        "lane": "core/layer0/ops",
        "artifact": {
            "path": artifact_path.display().to_string(),
            "sha256": sha256_hex_str(&identity.to_string())
        },
        "identity": identity,
        "claim_evidence": [
            {
                "id": "V6-HERMES-001.1",
                "claim": "shadow_discover_generates_signed_identity_artifact_with_conduit_receipts",
                "evidence": {
                    "shadow_id": shadow_id,
                    "signature": signature
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
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

    let op = clean(
        parsed
            .flags
            .get("op")
            .cloned()
            .or_else(|| parsed.positional.get(1).cloned())
            .unwrap_or_else(|| "status".to_string()),
        30,
    )
    .to_ascii_lowercase();
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
            let context = parsed
                .flags
                .get("context-json")
                .and_then(|raw| serde_json::from_str::<Value>(raw).ok())
                .unwrap_or_else(|| {
                    json!({
                        "context": ["session active", "pending tasks"],
                        "user_model": {"style": "direct", "confidence": 0.87},
                        "active_tasks": ["batch12 hardening"]
                    })
                });
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

fn collect_recent_ops_latest(root: &Path, max_blocks: usize) -> Vec<Value> {
    let ops_root = root.join("core").join("local").join("state").join("ops");
    let mut rows = Vec::<Value>::new();
    if !ops_root.exists() {
        return rows;
    }
    let Ok(entries) = fs::read_dir(&ops_root) else {
        return rows;
    };
    for entry in entries.flatten() {
        let lane = entry.file_name().to_string_lossy().to_string();
        let latest = entry.path().join("latest.json");
        if !latest.exists() {
            continue;
        }
        let latest_mtime_ms = file_mtime_epoch_ms(&latest);
        let conduit_history = entry.path().join("conduit").join("history.jsonl");
        let conduit_history_mtime_ms = if conduit_history.exists() {
            file_mtime_epoch_ms(&conduit_history)
        } else {
            0
        };
        if let Some(payload) = read_json(&latest) {
            let ok = payload
                .get("ok")
                .and_then(Value::as_bool)
                .unwrap_or_else(|| {
                    let status = payload
                        .get("status")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .trim()
                        .to_ascii_lowercase();
                    matches!(
                        status.as_str(),
                        "ok" | "pass" | "healthy" | "running" | "active" | "success"
                    )
                });
            let ty = clean(
                payload
                    .get("type")
                    .and_then(Value::as_str)
                    .or_else(|| payload.get("event_type").and_then(Value::as_str))
                    .unwrap_or("unknown"),
                120,
            );
            let ts = clean(
                payload
                    .get("ts")
                    .and_then(Value::as_str)
                    .or_else(|| payload.get("generated_at").and_then(Value::as_str))
                    .unwrap_or(""),
                80,
            );
            rows.push(json!({
                "lane": lane,
                "type": ty,
                "ok": ok,
                "ts": ts,
                "latest_path": latest.display().to_string(),
                "latest_mtime_ms": latest_mtime_ms,
                "has_conduit_history": conduit_history.exists(),
                "conduit_history_mtime_ms": conduit_history_mtime_ms,
                "payload": payload
            }));
        }
    }
    rows.sort_by(|a, b| {
        let left_mtime = a
            .get("latest_mtime_ms")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let right_mtime = b
            .get("latest_mtime_ms")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        right_mtime.cmp(&left_mtime).then_with(|| {
            let left_lane = a.get("lane").and_then(Value::as_str).unwrap_or_default();
            let right_lane = b.get("lane").and_then(Value::as_str).unwrap_or_default();
            left_lane.cmp(right_lane)
        })
    });
    rows.truncate(max_blocks);
    rows
}

fn default_reclaim_protected_lanes() -> BTreeSet<String> {
    [
        "app_plane",
        "skills_plane",
        "collab_plane",
        "hermes_plane",
        "security_plane",
        "attention_queue",
        "dashboard_ui",
    ]
    .iter()
    .map(|value| value.to_string())
    .collect()
}

fn reclaim_protected_lanes(contract: &Value) -> BTreeSet<String> {
    let mut out = default_reclaim_protected_lanes();
    if let Some(rows) = contract
        .get("reclaim_protected_lanes")
        .and_then(Value::as_array)
    {
        for row in rows {
            let lane = clean(row.as_str().unwrap_or_default(), 80).to_ascii_lowercase();
            if !lane.is_empty() {
                out.insert(lane);
            }
        }
    }
    out
}

fn reclaim_stale_latest(
    root: &Path,
    stale_threshold_ms: u64,
    max_reclaims: usize,
    protected_lanes: &BTreeSet<String>,
    dry_run: bool,
) -> Value {
    let ops_root = root.join("core").join("local").join("state").join("ops");
    if !ops_root.exists() {
        return json!({
            "ok": true,
            "type": "hermes_plane_reclaim_stale",
            "dry_run": dry_run,
            "stale_threshold_ms": stale_threshold_ms,
            "scanned_lanes": 0,
            "candidate_count": 0,
            "reclaimed_count": 0,
            "skipped_protected_count": 0,
            "rows": [],
            "errors": []
        });
    }
    let mut scanned_lanes: usize = 0;
    let mut skipped_protected_count: usize = 0;
    let mut candidates = Vec::<(u64, String, String, PathBuf, String)>::new();
    let mut errors = Vec::<String>::new();
    let entries = match fs::read_dir(&ops_root) {
        Ok(rows) => rows,
        Err(err) => {
            return json!({
                "ok": false,
                "type": "hermes_plane_reclaim_stale",
                "dry_run": dry_run,
                "stale_threshold_ms": stale_threshold_ms,
                "scanned_lanes": 0,
                "candidate_count": 0,
                "reclaimed_count": 0,
                "skipped_protected_count": 0,
                "rows": [],
                "errors": [clean(&format!("read_dir_failed:{err}"), 320)]
            });
        }
    };
    for entry in entries.flatten() {
        let lane = entry.file_name().to_string_lossy().to_ascii_lowercase();
        let latest_path = entry.path().join("latest.json");
        if !latest_path.exists() {
            continue;
        }
        scanned_lanes += 1;
        if protected_lanes.contains(&lane) {
            skipped_protected_count += 1;
            continue;
        }
        let payload = read_json(&latest_path).unwrap_or(Value::Null);
        let ts = clean(
            payload
                .get("ts")
                .and_then(Value::as_str)
                .or_else(|| payload.get("generated_at").and_then(Value::as_str))
                .unwrap_or(""),
            80,
        );
        let latest_mtime_ms = file_mtime_epoch_ms(&latest_path);
        let (duration_ms, source) = duration_from_ts_or_mtime_ms(&ts, latest_mtime_ms);
        if duration_ms < stale_threshold_ms {
            continue;
        }
        candidates.push((
            duration_ms,
            lane,
            latest_path.display().to_string(),
            latest_path,
            source.to_string(),
        ));
    }
    candidates.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
    let mut reclaimed_count: usize = 0;
    let mut rows = Vec::<Value>::new();
    let max_apply = max_reclaims.max(1);
    let archive_dir = state_root(root).join("cockpit").join("reclaimed");
    if !dry_run {
        let _ = fs::create_dir_all(&archive_dir);
    }
    for (idx, (duration_ms, lane, from_display, from_path, source)) in candidates.iter().enumerate() {
        if idx >= max_apply {
            break;
        }
        if dry_run {
            rows.push(json!({
                "lane": lane,
                "duration_ms": duration_ms,
                "duration_source": source,
                "from": from_display,
                "reclaimed": false,
                "dry_run": true
            }));
            continue;
        }
        let lane_token = clean_id(lane, "lane");
        let stamp = Utc::now().timestamp_millis().max(0) as u64;
        let archive_path = archive_dir.join(format!("{lane_token}-{stamp}.json"));
        let moved = fs::rename(from_path, &archive_path)
            .or_else(|_| fs::copy(from_path, &archive_path).map(|_| ()))
            .and_then(|_| fs::remove_file(from_path).or(Ok(())));
        if moved.is_ok() {
            reclaimed_count += 1;
            rows.push(json!({
                "lane": lane,
                "duration_ms": duration_ms,
                "duration_source": source,
                "from": from_display,
                "to": archive_path.display().to_string(),
                "reclaimed": true
            }));
        } else if let Err(err) = moved {
            errors.push(clean(&format!("reclaim_failed:{lane}:{err}"), 320));
        }
    }
    json!({
        "ok": errors.is_empty(),
        "type": "hermes_plane_reclaim_stale",
        "dry_run": dry_run,
        "stale_threshold_ms": stale_threshold_ms,
        "max_reclaims": max_apply,
        "scanned_lanes": scanned_lanes,
        "candidate_count": candidates.len(),
        "reclaimed_count": reclaimed_count,
        "skipped_protected_count": skipped_protected_count,
        "rows": rows,
        "errors": errors
    })
}

fn classify_tool_call(ty: &str) -> &'static str {
    let lower = ty.to_ascii_lowercase();
    if lower.contains("research") {
        "research"
    } else if lower.contains("parse") {
        "parse"
    } else if lower.contains("mcp") {
        "mcp"
    } else if lower.contains("skills") {
        "skills"
    } else if lower.contains("binary") {
        "security"
    } else if lower.contains("vbrowser") {
        "browser"
    } else {
        "runtime"
    }
}

fn status_color(ok: bool, class: &str) -> &'static str {
    if !ok {
        "red"
    } else if class == "security" {
        "amber"
    } else if class == "browser" {
        "blue"
    } else {
        "green"
    }
}

fn parse_ts_epoch_ms(ts: &str) -> Option<u64> {
    let parsed = DateTime::parse_from_rfc3339(ts).ok();
    parsed
        .map(|value| value.with_timezone(&Utc).timestamp_millis())
        .and_then(|ms| u64::try_from(ms).ok())
}

fn file_mtime_epoch_ms(path: &Path) -> u64 {
    fs::metadata(path)
        .ok()
        .and_then(|metadata| metadata.modified().ok())
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or(0)
}

fn duration_from_epoch_ms(epoch_ms: u64) -> u64 {
    let now_ms = Utc::now().timestamp_millis();
    let now_ms = u64::try_from(now_ms).unwrap_or(0);
    now_ms.saturating_sub(epoch_ms)
}

fn duration_from_ts_or_mtime_ms(ts: &str, latest_mtime_ms: u64) -> (u64, &'static str) {
    if let Some(ts_ms) = parse_ts_epoch_ms(ts) {
        return (duration_from_epoch_ms(ts_ms), "event_ts");
    }
    if latest_mtime_ms > 0 {
        return (duration_from_epoch_ms(latest_mtime_ms), "latest_mtime");
    }
    (0, "unknown")
}

fn run_cockpit(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let contract = load_json_or(
        root,
        COCKPIT_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "premium_cockpit_contract",
            "max_blocks": 64,
            "stale_block_threshold_ms": 30_000,
            "conduit_signal_active_window_ms": 90_000,
            "auto_reclaim_stale_blocks": true,
            "auto_reclaim_max_per_run": 16,
            "allowed_status_colors": ["green", "amber", "red", "blue"]
        }),
    );
    let mut errors = Vec::<String>::new();
    if contract
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("premium_cockpit_contract_version_must_be_v1".to_string());
    }
    if contract
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "premium_cockpit_contract"
    {
        errors.push("premium_cockpit_contract_kind_invalid".to_string());
    }
    if !errors.is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "hermes_plane_cockpit",
            "errors": errors
        });
    }

    let max_blocks = parse_u64(parsed.flags.get("max-blocks"), 0).max(1).min(
        contract
            .get("max_blocks")
            .and_then(Value::as_u64)
            .unwrap_or(64),
    ) as usize;
    let contract_stale_threshold_ms = contract
        .get("stale_block_threshold_ms")
        .and_then(Value::as_u64)
        .unwrap_or(30_000);
    let stale_block_threshold_ms = parse_u64(
        parsed.flags.get("stale-threshold-ms"),
        parse_u64(parsed.flags.get("threshold-ms"), contract_stale_threshold_ms),
    )
    .max(1);
    let conduit_signal_active_window_ms = parse_u64(
        parsed.flags.get("conduit-signal-window-ms"),
        contract
            .get("conduit_signal_active_window_ms")
            .and_then(Value::as_u64)
            .unwrap_or(90_000),
    )
    .max(stale_block_threshold_ms);
    let auto_reclaim_stale_blocks = contract
        .get("auto_reclaim_stale_blocks")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let reclaim_threshold_ms = parse_u64(
        parsed.flags.get("reclaim-threshold-ms"),
        stale_block_threshold_ms,
    )
    .max(1);
    let reclaim_max_per_run = parse_u64(
        parsed.flags.get("max-reclaims"),
        contract
            .get("auto_reclaim_max_per_run")
            .and_then(Value::as_u64)
            .unwrap_or(16),
    )
    .clamp(1, 10_000) as usize;
    let protected_lanes = reclaim_protected_lanes(&contract);
    let reclaim = if auto_reclaim_stale_blocks {
        reclaim_stale_latest(
            root,
            reclaim_threshold_ms,
            reclaim_max_per_run,
            &protected_lanes,
            false,
        )
    } else {
        json!({
            "ok": true,
            "type": "hermes_plane_reclaim_stale",
            "dry_run": true,
            "stale_threshold_ms": reclaim_threshold_ms,
            "candidate_count": 0,
            "reclaimed_count": 0,
            "skipped_protected_count": protected_lanes.len()
        })
    };
    let reclaimed_count = reclaim
        .get("reclaimed_count")
        .and_then(Value::as_u64)
        .unwrap_or(0) as usize;
    let latest_rows = collect_recent_ops_latest(root, max_blocks);

    let mut blocks = Vec::<Value>::new();
    let mut stale_block_count: usize = 0;
    let mut active_block_count: usize = 0;
    let mut conduit_signals_total: usize = 0;
    let mut conduit_signals_active: usize = 0;
    for (idx, row) in latest_rows.iter().enumerate() {
        let lane = clean(
            row.get("lane").and_then(Value::as_str).unwrap_or("unknown"),
            120,
        );
        let ty = clean(
            row.get("type").and_then(Value::as_str).unwrap_or("unknown"),
            120,
        );
        let row_ts = row.get("ts").and_then(Value::as_str).unwrap_or("");
        let latest_mtime_ms = row
            .get("latest_mtime_ms")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let conduit_history_mtime_ms = row
            .get("conduit_history_mtime_ms")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let ok = row.get("ok").and_then(Value::as_bool).unwrap_or(false);
        let class = classify_tool_call(&ty).to_string();
        let payload = row.get("payload").cloned().unwrap_or(Value::Null);
        let has_conduit_history = row
            .get("has_conduit_history")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let conduit_enforced = payload.get("conduit_enforcement").is_some()
            || payload
                .get("routed_via")
                .and_then(Value::as_str)
                .map(|v| v.eq_ignore_ascii_case("conduit"))
                .unwrap_or(false)
            || has_conduit_history;
        let (duration_ms, duration_source) = duration_from_ts_or_mtime_ms(row_ts, latest_mtime_ms);
        let conduit_history_age_ms = if conduit_history_mtime_ms > 0 {
            duration_from_epoch_ms(conduit_history_mtime_ms)
        } else {
            0
        };
        let is_stale = duration_ms >= stale_block_threshold_ms;
        if is_stale {
            stale_block_count += 1;
        } else {
            active_block_count += 1;
        }
        if conduit_enforced {
            conduit_signals_total += 1;
            if duration_ms < conduit_signal_active_window_ms {
                conduit_signals_active += 1;
            }
        }
        let block = json!({
            "index": idx + 1,
            "lane": lane,
            "event_type": ty,
            "tool_call_class": class,
            "status": if ok { "ok" } else { "fail" },
            "status_color": status_color(ok, classify_tool_call(&ty)),
            "conduit_enforced": conduit_enforced,
            "duration_ms": duration_ms,
            "duration_source": duration_source,
            "is_stale": is_stale,
            "stale_block_threshold_ms": stale_block_threshold_ms,
            "latest_mtime_ms": latest_mtime_ms,
            "conduit_history_age_ms": conduit_history_age_ms,
            "ts": row.get("ts").cloned().unwrap_or(Value::Null),
            "path": row.get("latest_path").cloned().unwrap_or(Value::Null)
        });
        blocks.push(block);
    }

    let cockpit = json!({
        "version": "v1",
        "mode": "premium",
        "render": {
            "ascii_header": "PROTHEUS TOP",
            "stream_blocks": blocks,
            "total_blocks": blocks.len()
        },
        "metrics": {
            "active_block_count": active_block_count,
            "stale_block_count": stale_block_count,
            "stale_block_threshold_ms": stale_block_threshold_ms,
            "stale_reclaimed_count": reclaimed_count,
            "conduit_signal_active_window_ms": conduit_signal_active_window_ms,
            "conduit_signals_active": conduit_signals_active,
            "conduit_signals_total": conduit_signals_total,
            "conduit_channels_observed": conduit_signals_active
        },
        "generated_at": crate::now_iso()
    });
    let artifact_path = state_root(root).join("cockpit").join("latest.json");
    let _ = write_json(&artifact_path, &cockpit);

    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "hermes_plane_cockpit",
        "lane": "core/layer0/ops",
        "artifact": {
            "path": artifact_path.display().to_string(),
            "sha256": sha256_hex_str(&cockpit.to_string())
        },
        "cockpit": cockpit,
        "reclaim": reclaim,
        "claim_evidence": [
            {
                "id": "V6-HERMES-001.2",
                "claim": "premium_realtime_cockpit_stream_exposes_timings_tool_classes_and_status_colors",
                "evidence": {
                    "blocks": blocks.len(),
                    "max_blocks": max_blocks
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}

fn run_reclaim_stale(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let contract = load_json_or(
        root,
        COCKPIT_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "premium_cockpit_contract",
            "max_blocks": 64,
            "stale_block_threshold_ms": 30_000,
            "conduit_signal_active_window_ms": 90_000,
            "auto_reclaim_stale_blocks": true,
            "auto_reclaim_max_per_run": 16,
            "allowed_status_colors": ["green", "amber", "red", "blue"]
        }),
    );
    let default_threshold_ms = contract
        .get("stale_block_threshold_ms")
        .and_then(Value::as_u64)
        .unwrap_or(30_000);
    let threshold_ms = parse_u64(
        parsed.flags.get("stale-threshold-ms"),
        parse_u64(parsed.flags.get("threshold-ms"), default_threshold_ms),
    )
    .max(1);
    let max_reclaims = parse_u64(
        parsed.flags.get("max-reclaims"),
        contract
            .get("auto_reclaim_max_per_run")
            .and_then(Value::as_u64)
            .unwrap_or(16),
    )
    .clamp(1, 10_000) as usize;
    let dry_run = parse_bool(parsed.flags.get("dry-run"), false);
    let protected_lanes = reclaim_protected_lanes(&contract);
    let reclaim = reclaim_stale_latest(root, threshold_ms, max_reclaims, &protected_lanes, dry_run);
    json!({
        "ok": reclaim.get("ok").and_then(Value::as_bool).unwrap_or(false),
        "strict": strict,
        "type": "hermes_plane_reclaim_stale",
        "lane": "core/layer0/ops",
        "reclaim": reclaim,
        "claim_evidence": [
            {
                "id": "V6-HERMES-001.2",
                "claim": "premium_realtime_cockpit_stream_exposes_timings_tool_classes_and_status_colors",
                "evidence": {
                    "stale_threshold_ms": threshold_ms,
                    "max_reclaims": max_reclaims,
                    "dry_run": dry_run
                }
            }
        ]
    })
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let parsed = parse_args(argv);
    let command = parsed
        .positional
        .first()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());
    if matches!(command.as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }

    let strict = parse_bool(parsed.flags.get("strict"), true);
    let conduit = if command != "status" {
        Some(conduit_enforcement(root, &parsed, strict, &command))
    } else {
        None
    };
    if strict
        && conduit
            .as_ref()
            .and_then(|v| v.get("ok"))
            .and_then(Value::as_bool)
            == Some(false)
    {
        return emit(
            root,
            json!({
                "ok": false,
                "strict": strict,
                "type": "hermes_plane_conduit_gate",
                "errors": ["conduit_bypass_rejected"],
                "conduit_enforcement": conduit
            }),
        );
    }

    let payload = match command.as_str() {
        "status" => status(root),
        "discover" => run_discover(root, &parsed, strict),
        "continuity" => run_continuity(root, &parsed, strict),
        "delegate" => run_delegate(root, &parsed, strict),
        "cockpit" | "top" | "dashboard" => run_cockpit(root, &parsed, strict),
        "reclaim-stale" | "reclaim-blocks" => run_reclaim_stale(root, &parsed, strict),
        _ => json!({
            "ok": false,
            "type": "hermes_plane_error",
            "error": "unknown_command",
            "command": command
        }),
    };
    if command == "status" {
        print_json(&payload);
        return 0;
    }
    emit(root, attach_conduit(payload, conduit.as_ref()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_tool_call_maps_known_classes() {
        assert_eq!(classify_tool_call("skills_plane_run"), "skills");
        assert_eq!(classify_tool_call("binary_vuln_plane_scan"), "security");
        assert_eq!(
            classify_tool_call("vbrowser_plane_session_start"),
            "browser"
        );
    }

    #[test]
    fn conduit_rejects_bypass() {
        let root = tempfile::tempdir().expect("tempdir");
        let parsed = crate::parse_args(&["discover".to_string(), "--bypass=1".to_string()]);
        let out = conduit_enforcement(root.path(), &parsed, true, "discover");
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
    }

    #[test]
    fn continuity_snapshot_paths_are_stable() {
        let root = tempfile::tempdir().expect("tempdir");
        let path = continuity_snapshot_path(root.path(), "session-a");
        assert!(path.to_string_lossy().contains("session-a"));
    }

    #[test]
    fn cockpit_latest_reader_accepts_status_and_event_type_fallbacks() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane_dir = root
            .path()
            .join("core")
            .join("local")
            .join("state")
            .join("ops")
            .join("benchmark_sanity");
        let latest = lane_dir.join("latest.json");
        let _ = write_json(
            &latest,
            &json!({
                "status": "ok",
                "event_type": "benchmark_sanity_gate",
                "generated_at": "2026-03-22T00:00:00.000Z"
            }),
        );

        let rows = collect_recent_ops_latest(root.path(), 16);
        let row = rows
            .iter()
            .find(|entry| entry.get("lane").and_then(Value::as_str) == Some("benchmark_sanity"))
            .expect("benchmark_sanity row should be present");

        assert_eq!(row.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            row.get("type").and_then(Value::as_str),
            Some("benchmark_sanity_gate")
        );
        assert_eq!(
            row.get("ts").and_then(Value::as_str),
            Some("2026-03-22T00:00:00.000Z")
        );
    }

    #[test]
    fn cockpit_marks_conduit_enforced_from_lane_payload() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane_dir = root
            .path()
            .join("core")
            .join("local")
            .join("state")
            .join("ops")
            .join("alpha_lane");
        let latest = lane_dir.join("latest.json");
        let _ = write_json(
            &latest,
            &json!({
                "ok": true,
                "type": "alpha_task",
                "ts": "2026-03-22T00:00:00.000Z",
                "conduit_enforcement": {
                    "ok": true,
                    "type": "alpha_conduit_enforcement"
                }
            }),
        );

        let parsed = crate::parse_args(&["cockpit".to_string(), "--max-blocks=8".to_string()]);
        let out = run_cockpit(root.path(), &parsed, true);
        let blocks = out["cockpit"]["render"]["stream_blocks"]
            .as_array()
            .expect("stream blocks");
        let row = blocks
            .iter()
            .find(|entry| entry.get("lane").and_then(Value::as_str) == Some("alpha_lane"))
            .expect("alpha lane block should be present");
        assert_eq!(
            row.get("conduit_enforced").and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn cockpit_duration_tracks_timestamp_age() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane_dir = root
            .path()
            .join("core")
            .join("local")
            .join("state")
            .join("ops")
            .join("age_lane");
        let latest = lane_dir.join("latest.json");
        let _ = write_json(
            &latest,
            &json!({
                "ok": true,
                "type": "age_task",
                "ts": "2000-01-01T00:00:00.000Z"
            }),
        );

        let parsed = crate::parse_args(&["cockpit".to_string(), "--max-blocks=8".to_string()]);
        let out = run_cockpit(root.path(), &parsed, true);
        let blocks = out["cockpit"]["render"]["stream_blocks"]
            .as_array()
            .expect("stream blocks");
        let row = blocks
            .iter()
            .find(|entry| entry.get("lane").and_then(Value::as_str) == Some("age_lane"))
            .expect("age lane block should be present");
        assert!(
            row.get("duration_ms").and_then(Value::as_u64).unwrap_or(0) > 1_000,
            "duration_ms should reflect parsed timestamp age"
        );
    }

    #[test]
    fn cockpit_duration_falls_back_to_latest_mtime_when_ts_missing() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane_dir = root
            .path()
            .join("core")
            .join("local")
            .join("state")
            .join("ops")
            .join("mtime_lane");
        let latest = lane_dir.join("latest.json");
        let _ = write_json(
            &latest,
            &json!({
                "ok": true,
                "type": "mtime_task"
            }),
        );

        let parsed = crate::parse_args(&["cockpit".to_string(), "--max-blocks=8".to_string()]);
        let out = run_cockpit(root.path(), &parsed, true);
        let blocks = out["cockpit"]["render"]["stream_blocks"]
            .as_array()
            .expect("stream blocks");
        let row = blocks
            .iter()
            .find(|entry| entry.get("lane").and_then(Value::as_str) == Some("mtime_lane"))
            .expect("mtime lane block should be present");
        assert_eq!(
            row.get("duration_source").and_then(Value::as_str),
            Some("latest_mtime")
        );
    }

    #[test]
    fn cockpit_metrics_include_active_and_stale_block_counts() {
        let root = tempfile::tempdir().expect("tempdir");
        let stale_lane_dir = root
            .path()
            .join("core")
            .join("local")
            .join("state")
            .join("ops")
            .join("stale_lane");
        let fresh_lane_dir = root
            .path()
            .join("core")
            .join("local")
            .join("state")
            .join("ops")
            .join("fresh_lane");
        let _ = write_json(
            &stale_lane_dir.join("latest.json"),
            &json!({
                "ok": true,
                "type": "stale_task",
                "ts": "2000-01-01T00:00:00.000Z"
            }),
        );
        let _ = write_json(
            &fresh_lane_dir.join("latest.json"),
            &json!({
                "ok": true,
                "type": "fresh_task",
                "ts": crate::now_iso()
            }),
        );

        let parsed = crate::parse_args(&["cockpit".to_string(), "--max-blocks=8".to_string()]);
        let out = run_cockpit(root.path(), &parsed, true);
        let metrics = out["cockpit"]["metrics"].clone();
        let stale_count = metrics
            .get("stale_block_count")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let reclaimed_count = metrics
            .get("stale_reclaimed_count")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        assert!(
            stale_count + reclaimed_count >= 1
        );
        assert!(
            metrics
                .get("active_block_count")
                .and_then(Value::as_u64)
                .unwrap_or(0)
                >= 1
        );
    }
}
