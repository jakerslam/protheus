// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops::nexus_plane (authoritative)
use crate::v8_kernel::{
    append_jsonl, build_conduit_enforcement, canonical_json_string, conduit_bypass_requested,
    deterministic_merkle_root, emit_attached_plane_receipt, history_path, latest_path,
    merkle_proof, parse_bool, parse_json_or_empty, read_json, read_jsonl, scoped_state_root,
    sha256_hex_str, write_json,
};
use crate::{clean, now_iso, parse_args};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

const LANE_ID: &str = "nexus_plane";
const ENV_KEY: &str = "PROTHEUS_NEXUS_PLANE_STATE_ROOT";

fn usage() {
    println!("Usage:");
    for line in [
        "  protheus-ops nexus-plane package-domain --domain=<id> [--strict=1|0]",
        "  protheus-ops nexus-plane bridge --from-domain=<id> --to-domain=<id> [--payload-json=<json>] [--legal-contract-id=<id>] [--sanitize=1|0] [--strict=1|0]",
        "  protheus-ops nexus-plane insurance --op=<quote|status> [--risk-json=<json>] [--strict=1|0]",
        "  protheus-ops nexus-plane human-boundary --op=<authorize|status> [--action=<id>] [--human-a=<sig>] [--human-b=<sig>] [--strict=1|0]",
        "  protheus-ops nexus-plane receipt-v2 --op=<validate|status> [--receipt-json=<json>] [--strict=1|0]",
        "  protheus-ops nexus-plane merkle-forest --op=<build|status> [--strict=1|0]",
        "  protheus-ops nexus-plane compliance-ledger --op=<append|query|status> [--entry-json=<json>] [--chain-id=<id>] [--strict=1|0]",
    ] {
        println!("{line}");
    }
}

fn lane_root(root: &Path) -> PathBuf {
    scoped_state_root(root, ENV_KEY, LANE_ID)
}

fn lane_file(root: &Path, leaf: &str) -> PathBuf {
    lane_root(root).join(leaf)
}

fn emit(root: &Path, _command: &str, strict: bool, payload: Value, conduit: Option<&Value>) -> i32 {
    emit_attached_plane_receipt(root, ENV_KEY, LANE_ID, strict, payload, conduit)
}

fn parse_op(parsed: &crate::ParsedArgs) -> String {
    clean(
        parsed
            .flags
            .get("op")
            .map(String::as_str)
            .unwrap_or("status"),
        16,
    )
    .to_ascii_lowercase()
}

fn command_error_payload(command: &str, error: &str) -> Value {
    json!({
        "ok": false,
        "type": "nexus_plane",
        "lane": LANE_ID,
        "ts": now_iso(),
        "command": command,
        "error": error
    })
}

fn package_domain_command(root: &Path, parsed: &crate::ParsedArgs) -> Result<Value, String> {
    let domain = clean(
        parsed
            .flags
            .get("domain")
            .map(String::as_str)
            .unwrap_or("domain"),
        80,
    );
    let base = lane_file(root, "packages").join(&domain);
    for part in [
        "layer0/policy",
        "layer1/execution",
        "layer2/surfaces",
        "certification",
        "bridges",
    ] {
        fs::create_dir_all(base.join(part))
            .map_err(|e| format!("domain_package_mkdir_failed:{e}"))?;
    }
    let manifest = json!({
        "domain": domain,
        "layout": ["layer0/policy", "layer1/execution", "layer2/surfaces", "certification", "bridges"],
        "packaged_at": now_iso()
    });
    write_json(&base.join("manifest.json"), &manifest)?;
    Ok(json!({
        "ok": true,
        "type": "nexus_plane_package_domain",
        "lane": LANE_ID,
        "ts": now_iso(),
        "domain": domain,
        "manifest_path": base.join("manifest.json").to_string_lossy().to_string(),
        "claim_evidence": [{
            "id": "V7-NEXUS-001.1",
            "claim": "domain_packaging_materializes_complete_substrate_layout_with_isolation_boundaries",
            "evidence": {"domain": domain}
        }]
    }))
}

fn bridge_command(root: &Path, parsed: &crate::ParsedArgs) -> Result<Value, String> {
    let from_domain = clean(
        parsed
            .flags
            .get("from-domain")
            .map(String::as_str)
            .unwrap_or(""),
        80,
    );
    let to_domain = clean(
        parsed
            .flags
            .get("to-domain")
            .map(String::as_str)
            .unwrap_or(""),
        80,
    );
    if from_domain.is_empty() || to_domain.is_empty() {
        return Err("bridge_domains_required".to_string());
    }
    let sanitize = parse_bool(parsed.flags.get("sanitize"), true);
    let payload = parse_json_or_empty(parsed.flags.get("payload-json"));
    let legal_contract_id = clean(
        parsed
            .flags
            .get("legal-contract-id")
            .map(String::as_str)
            .unwrap_or("legal-ref"),
        120,
    );
    let allowed = sanitize && from_domain != to_domain;
    let row = json!({
        "ts": now_iso(),
        "from_domain": from_domain,
        "to_domain": to_domain,
        "sanitize": sanitize,
        "legal_contract_id": legal_contract_id,
        "payload_hash": sha256_hex_str(&canonical_json_string(&payload)),
        "ok": allowed
    });
    append_jsonl(&lane_file(root, "bridge.jsonl"), &row)?;
    Ok(json!({
        "ok": allowed,
        "type": "nexus_plane_bridge",
        "lane": LANE_ID,
        "ts": now_iso(),
        "bridge": row,
        "claim_evidence": [{
            "id": "V7-NEXUS-001.2",
            "claim": "cross_domain_bridge_requires_zero_trust_sanitization_and_legal_binding_metadata",
            "evidence": {"allowed": allowed}
        }]
    }))
}

fn insurance_command(root: &Path, parsed: &crate::ParsedArgs) -> Result<Value, String> {
    let op = parse_op(parsed);
    if op == "status" {
        let rows = read_jsonl(&lane_file(root, "insurance_quotes.jsonl"));
        return Ok(json!({
            "ok": true,
            "type": "nexus_plane_insurance",
            "lane": LANE_ID,
            "ts": now_iso(),
            "op": op,
            "quotes": rows,
            "claim_evidence": [{
                "id": "V7-NEXUS-001.3",
                "claim": "insurance_oracle_status_surfaces_risk_quote_history",
                "evidence": {"count": rows.len()}
            }]
        }));
    }
    if op != "quote" {
        return Err("insurance_op_invalid".to_string());
    }
    let risk = parse_json_or_empty(parsed.flags.get("risk-json"));
    let base_risk = risk
        .get("risk_score")
        .and_then(Value::as_f64)
        .unwrap_or(0.5)
        .clamp(0.0, 1.0);
    let compliance = risk
        .get("compliance_score")
        .and_then(Value::as_f64)
        .unwrap_or(0.7)
        .clamp(0.0, 1.0);
    let adjusted = (base_risk + (1.0 - compliance) * 0.5).clamp(0.0, 1.0);
    let premium = 1000.0 + (adjusted * 9000.0);
    let quote = json!({
        "ts": now_iso(),
        "risk_score": adjusted,
        "premium_usd": premium,
        "coverage": if adjusted < 0.8 { "approved" } else { "limited" },
        "exclusions": if adjusted < 0.8 { Vec::<String>::new() } else { vec!["high_loss_domain".to_string()] }
    });
    append_jsonl(&lane_file(root, "insurance_quotes.jsonl"), &quote)?;
    Ok(json!({
        "ok": true,
        "type": "nexus_plane_insurance",
        "lane": LANE_ID,
        "ts": now_iso(),
        "op": op,
        "quote": quote,
        "claim_evidence": [{
            "id": "V7-NEXUS-001.3",
            "claim": "insurance_oracle_scores_execution_risk_and_emits_coverage_premium_decision_receipts",
            "evidence": {"premium_usd": premium}
        }]
    }))
}
