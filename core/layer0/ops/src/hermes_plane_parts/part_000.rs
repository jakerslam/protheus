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

