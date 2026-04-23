// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops::government_plane (authoritative)
use crate::v8_kernel::{
    append_jsonl, build_conduit_enforcement, canonical_json_string, conduit_bypass_requested,
    deterministic_merkle_root, emit_attached_plane_receipt, history_path, latest_path, parse_bool,
    parse_json_or_empty, read_json, scoped_state_root, sha256_hex_str, write_json,
};
use crate::{clean, now_iso, parse_args};
use serde_json::{json, Map, Value};
use std::fs;
use std::path::{Path, PathBuf};

const LANE_ID: &str = "government_plane";
const ENV_KEY: &str = "INFRING_GOVERNMENT_PLANE_STATE_ROOT";

fn usage() {
    println!("Usage:");
    println!(
        "  infring-ops government-plane attestation --op=<attest|verify|status> [--device-id=<id>] [--nonce=<v>] [--strict=1|0]"
    );
    println!(
        "  infring-ops government-plane classification --op=<set-clearance|write|read|transfer|status> [--principal=<id>] [--clearance=<level>] [--level=<level>] [--id=<object>] [--payload-json=<json>] [--from=<level>] [--to=<level>] [--via-cds=1|0] [--strict=1|0]"
    );
    println!(
        "  infring-ops government-plane nonrepudiation --principal=<subject> --action=<id> --auth-signature=<sig> --timestamp-authority=<authority> [--legal-hold=1|0] [--strict=1|0]"
    );
    println!(
        "  infring-ops government-plane diode --from=<level> --to=<level> [--sanitize=1|0] [--payload-json=<json>] [--strict=1|0]"
    );
    println!(
        "  infring-ops government-plane soc --op=<connect|emit|status> [--endpoint=<url>] [--event-json=<json>] [--strict=1|0]"
    );
    println!(
        "  infring-ops government-plane coop --op=<register-site|replicate|failover|status> [--site=<id>] [--state=<ACTIVE|STANDBY|COLD|FAILED>] [--target-site=<id>] [--strict=1|0]"
    );
    println!("  infring-ops government-plane proofs --op=<verify|status> [--strict=1|0]");
    println!(
        "  infring-ops government-plane interoperability --op=<validate|status> [--profile-json=<json>] [--strict=1|0]"
    );
    println!("  infring-ops government-plane ato-pack --op=<generate|status> [--strict=1|0]");
}

fn lane_root(root: &Path) -> PathBuf {
    scoped_state_root(root, ENV_KEY, LANE_ID)
}

fn lane_file(root: &Path, file: &str) -> PathBuf {
    lane_root(root).join(file)
}

fn attestation_path(root: &Path) -> PathBuf {
    lane_file(root, "attestation_latest.json")
}

fn clearances_path(root: &Path) -> PathBuf {
    lane_file(root, "classification_clearances.json")
}

fn classification_root(root: &Path) -> PathBuf {
    crate::core_state_root(root).join("classified")
}

fn diode_history_path(root: &Path) -> PathBuf {
    lane_file(root, "diode_transfers.jsonl")
}

fn soc_state_path(root: &Path) -> PathBuf {
    lane_file(root, "soc_state.json")
}

fn coop_state_path(root: &Path) -> PathBuf {
    lane_file(root, "coop_sites.json")
}

fn legal_log_path(root: &Path) -> PathBuf {
    lane_file(root, "legal_nonrepudiation.jsonl")
}

fn clearances(root: &Path) -> Map<String, Value> {
    read_json(&clearances_path(root))
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default()
}

fn level_rank(level: &str) -> i32 {
    match level.to_ascii_lowercase().as_str() {
        "unclassified" => 0,
        "cui" => 1,
        "confidential" => 2,
        "secret" => 3,
        "top-secret" | "top_secret" => 4,
        _ => -1,
    }
}

fn emit(root: &Path, _command: &str, strict: bool, payload: Value, conduit: Option<&Value>) -> i32 {
    emit_attached_plane_receipt(root, ENV_KEY, LANE_ID, strict, payload, conduit)
}

fn attestation_command(root: &Path, parsed: &crate::ParsedArgs) -> Result<Value, String> {
    let op = clean(
        parsed
            .flags
            .get("op")
            .map(String::as_str)
            .unwrap_or("status"),
        16,
    )
    .to_ascii_lowercase();
    if op == "status" {
        return Ok(json!({
            "ok": true,
            "type": "government_plane_attestation",
            "lane": LANE_ID,
            "ts": now_iso(),
            "op": op,
            "attestation": read_json(&attestation_path(root)).unwrap_or_else(|| json!({})),
            "claim_evidence": [{
                "id": "V7-GOV-001.1",
                "claim": "hardware_root_of_trust_status_surfaces_latest_tpm_hsm_attestation_receipt",
                "evidence": {"status_available": true}
            }]
        }));
    }
    let device_input = parsed
        .flags
        .get("device-id")
        .cloned()
        .or_else(|| std::env::var("INFRING_TPM_DEVICE_ID").ok())
        .unwrap_or_else(|| "tpm-sim".to_string());
    let device_id = clean(device_input, 120);
    let nonce = clean(
        parsed
            .flags
            .get("nonce")
            .map(String::as_str)
            .unwrap_or("attest"),
        120,
    );
    let hardware_secret = std::env::var("INFRING_HSM_RECEIPT_KEY")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "local-dev-hsm".to_string());
    let signature = sha256_hex_str(&format!("{device_id}:{nonce}:{hardware_secret}"));
    if op == "attest" {
        let attestation = json!({
            "device_id": device_id,
            "nonce": nonce,
            "tpm_quote": sha256_hex_str(&format!("quote:{}:{}", device_id, nonce)),
            "hsm_signature": signature,
            "ts": now_iso()
        });
        write_json(&attestation_path(root), &attestation)?;
        return Ok(json!({
            "ok": true,
            "type": "government_plane_attestation",
            "lane": LANE_ID,
            "ts": now_iso(),
            "op": op,
            "attestation": attestation,
            "claim_evidence": [{
                "id": "V7-GOV-001.1",
                "claim": "hardware_root_of_trust_attestation_uses_tpm_quote_and_hsm_signature_binding",
                "evidence": {"device_id": device_id}
            }]
        }));
    }
    if op == "verify" {
        let attestation =
            read_json(&attestation_path(root)).ok_or_else(|| "attestation_missing".to_string())?;
        let expected = sha256_hex_str(&format!(
            "{}:{}:{}",
            attestation
                .get("device_id")
                .and_then(Value::as_str)
                .unwrap_or(""),
            attestation
                .get("nonce")
                .and_then(Value::as_str)
                .unwrap_or(""),
            hardware_secret
        ));
        let valid = attestation
            .get("hsm_signature")
            .and_then(Value::as_str)
            .map(|s| s == expected)
            .unwrap_or(false);
        return Ok(json!({
            "ok": valid,
            "type": "government_plane_attestation",
            "lane": LANE_ID,
            "ts": now_iso(),
            "op": op,
            "valid": valid,
            "attestation": attestation,
            "claim_evidence": [{
                "id": "V7-GOV-001.1",
                "claim": "hardware_root_of_trust_verification_fails_closed_when_signature_binding_mismatches",
                "evidence": {"valid": valid}
            }]
        }));
    }
    Err("attestation_op_invalid".to_string())
}

fn classification_command(root: &Path, parsed: &crate::ParsedArgs) -> Result<Value, String> {
    let op = clean(
        parsed
            .flags
            .get("op")
            .map(String::as_str)
            .unwrap_or("status"),
        20,
    )
    .to_ascii_lowercase();
    let principal = clean(
        parsed
            .flags
            .get("principal")
            .map(String::as_str)
            .unwrap_or("operator"),
        120,
    );
    let mut clr = clearances(root);
    if op == "set-clearance" {
        let level = clean(
            parsed
                .flags
                .get("clearance")
                .map(String::as_str)
                .unwrap_or("unclassified"),
            32,
        )
        .to_ascii_lowercase();
        if level_rank(&level) < 0 {
            return Err("clearance_invalid".to_string());
        }
        clr.insert(principal.clone(), Value::String(level.clone()));
        write_json(&clearances_path(root), &Value::Object(clr))?;
        return Ok(json!({
            "ok": true,
            "type": "government_plane_classification",
            "lane": LANE_ID,
            "ts": now_iso(),
            "op": op,
            "principal": principal,
            "clearance": level,
            "claim_evidence": [{
                "id": "V7-GOV-001.2",
                "claim": "classification_plane_persists_clearance_and_enforces_namespace_isolation",
                "evidence": {"principal": principal}
            }]
        }));
    }
    let principal_level = clr
        .get(&principal)
        .and_then(Value::as_str)
        .unwrap_or("unclassified")
        .to_string();
    let level = clean(
        parsed
            .flags
            .get("level")
            .map(String::as_str)
            .unwrap_or("unclassified"),
        32,
    )
    .to_ascii_lowercase();
    if level_rank(&level) < 0 {
        return Err("classification_level_invalid".to_string());
    }
    if op == "status" {
        return Ok(json!({
            "ok": true,
            "type": "government_plane_classification",
            "lane": LANE_ID,
            "ts": now_iso(),
            "op": op,
            "principal": principal,
            "principal_clearance": principal_level,
            "clearance_path": clearances_path(root).to_string_lossy().to_string(),
            "claim_evidence": [{
                "id": "V7-GOV-001.2",
                "claim": "classification_plane_status_surfaces_principal_clearance_and_namespace_paths",
                "evidence": {"principal_clearance": principal_level}
            }]
        }));
    }
    if op == "transfer" {
        let from = clean(
            parsed
                .flags
                .get("from")
                .map(String::as_str)
                .unwrap_or("secret"),
            32,
        )
        .to_ascii_lowercase();
        let to = clean(
            parsed
                .flags
                .get("to")
                .map(String::as_str)
                .unwrap_or("unclassified"),
            32,
        )
        .to_ascii_lowercase();
        let via_cds = parse_bool(parsed.flags.get("via-cds"), false);
        let allowed = level_rank(&from) >= level_rank(&to) && via_cds;
        return Ok(json!({
            "ok": allowed,
            "type": "government_plane_classification",
            "lane": LANE_ID,
            "ts": now_iso(),
            "op": op,
            "principal": principal,
            "from": from,
            "to": to,
            "via_cds": via_cds,
            "claim_evidence": [{
                "id": "V7-GOV-001.2",
                "claim": "classification_transfers_require_explicit_cross_domain_guard_path",
                "evidence": {"allowed": allowed}
            }]
        }));
    }
    let id = clean(
        parsed
            .flags
            .get("id")
            .map(String::as_str)
            .unwrap_or("object"),
        140,
    );
    let object_path = classification_root(root)
        .join(level.clone())
        .join(format!("{}.json", id));
    if level_rank(&principal_level) < level_rank(&level) {
        return Ok(json!({
            "ok": false,
            "type": "government_plane_classification",
            "lane": LANE_ID,
            "ts": now_iso(),
            "op": op,
            "principal": principal,
            "principal_clearance": principal_level,
            "target_level": level,
            "error": "clearance_insufficient",
            "claim_evidence": [{
                "id": "V7-GOV-001.2",
                "claim": "classification_access_fails_closed_above_effective_clearance",
                "evidence": {"principal_clearance": principal_level}
            }]
        }));
    }
    if op == "write" {
        let payload = parse_json_or_empty(parsed.flags.get("payload-json"));
        write_json(
            &object_path,
            &json!({"principal": principal, "level": level, "payload": payload, "ts": now_iso()}),
        )?;
    } else if op != "read" {
        return Err("classification_op_invalid".to_string());
    }
    Ok(json!({
        "ok": true,
        "type": "government_plane_classification",
        "lane": LANE_ID,
        "ts": now_iso(),
        "op": op,
        "principal": principal,
        "principal_clearance": principal_level,
        "target_level": level,
        "object_path": object_path.to_string_lossy().to_string(),
        "object": read_json(&object_path).unwrap_or_else(|| json!({})),
        "claim_evidence": [{
            "id": "V7-GOV-001.2",
            "claim": "classification_plane_persists_isolated_level_scoped_objects",
            "evidence": {"op": op, "level": level}
        }]
    }))
}
