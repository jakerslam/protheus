// SPDX-License-Identifier: Apache-2.0
#[path = "../enterprise_moat_extensions.rs"]
mod enterprise_moat_extensions;

use crate::{deterministic_receipt_hash, now_iso, parse_args};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

const DEFAULT_POLICY_REL: &str = "client/runtime/config/f100_enterprise_hardening_policy.json";
const DEFAULT_IDENTITY_POLICY_REL: &str = "client/runtime/config/identity_federation_policy.json";
const DEFAULT_ACCESS_POLICY_REL: &str = "client/runtime/config/enterprise_access_policy.json";
const DEFAULT_ABAC_POLICY_REL: &str = "client/runtime/config/abac_policy_plane.json";
const DEFAULT_SIEM_POLICY_REL: &str = "client/runtime/config/siem_bridge_policy.json";
const DEFAULT_MULTI_TENANT_CONTRACT_REL: &str =
    "client/runtime/config/multi_tenant_isolation_contract.json";
const DEFAULT_SECRET_KMS_POLICY_REL: &str = "client/runtime/config/enterprise_secret_kms_policy.json";
const DEFAULT_SIGNED_RECEIPT_POLICY_REL: &str = "client/runtime/config/signed_receipt_policy.json";
const DEFAULT_RETENTION_POLICY_PACK_REL: &str = "client/runtime/config/retention_policy_pack.json";
const DEFAULT_RUNTIME_RETENTION_POLICY_REL: &str = "client/runtime/config/runtime_retention_policy.json";
const DEFAULT_COMPLIANCE_RETENTION_POLICY_REL: &str =
    "client/runtime/config/compliance_retention_policy.json";
const DEFAULT_AUDIT_EXPORT_POLICY_REL: &str = "client/runtime/config/audit_log_export_policy.json";
const DEFAULT_EVIDENCE_AUDIT_POLICY_REL: &str =
    "client/runtime/config/evidence_audit_dashboard_policy.json";
const DEFAULT_DEPLOYMENT_PACKAGING_DOC_REL: &str = "docs/client/DEPLOYMENT_PACKAGING.md";
const DEFAULT_SCALE_POLICY_REL: &str = "client/runtime/config/scale_readiness_program_policy.json";
const DEFAULT_BEDROCK_POLICY_REL: &str =
    "planes/contracts/enterprise/bedrock_proxy_contract_v1.json";
const DEFAULT_THIN_WRAPPER_SCAN_ROOT_REL: &str = "client/runtime/systems";
const DEFAULT_DOC_FREEZE_TAG: &str = "genesis-candidate";
const DEFAULT_INSTALLER_PROFILE: &str = "standard";
const ALLOWED_DELIVERY_CHANNELS: &[&str] = &[
    "last",
    "main",
    "inbox",
    "discord",
    "slack",
    "email",
    "pagerduty",
    "stdout",
    "stderr",
    "sms",
];

fn usage() {
    println!("Usage:");
    println!("  protheus-ops enterprise-hardening run [--strict=1|0] [--policy=<path>]");
    println!("  protheus-ops enterprise-hardening status [--policy=<path>]");
    println!(
        "  protheus-ops enterprise-hardening export-compliance [--profile=<internal|customer|auditor>] [--strict=1|0] [--policy=<path>]"
    );
    println!(
        "  protheus-ops enterprise-hardening identity-surface [--provider=<id>] [--token-issuer=<url>] [--scopes=a,b] [--roles=r1,r2] [--strict=1|0]"
    );
    println!(
        "  protheus-ops enterprise-hardening regulated-readiness [--strict=1|0] [--multi-tenant-policy=<path>] [--access-policy=<path>] [--abac-policy=<path>] [--secret-kms-policy=<path>] [--signed-receipt-policy=<path>] [--retention-pack-policy=<path>] [--runtime-retention-policy=<path>] [--compliance-retention-policy=<path>] [--audit-export-policy=<path>] [--evidence-audit-policy=<path>] [--deployment-doc=<path>]"
    );
    println!(
        "  protheus-ops enterprise-hardening certify-scale [--target-nodes=<n>] [--samples=<n>] [--strict=1|0] [--scale-policy=<path>]"
    );
    println!(
        "  protheus-ops enterprise-hardening enable-bedrock [--strict=1|0] [--region=<aws-region>] [--vpc=<id>] [--subnet=<id>] [--ssm-path=<path>] [--policy=<path>]"
    );
    println!(
        "  protheus-ops enterprise-hardening moat-license [--strict=1|0] [--primitives=a,b] [--license=<id>] [--reviewer=<id>]"
    );
    println!(
        "  protheus-ops enterprise-hardening moat-contrast [--strict=1|0] [--narrative=<short-text>]"
    );
    println!(
        "  protheus-ops enterprise-hardening moat-launch-sim [--strict=1|0] [--contributors=<n>] [--events=<n>]"
    );
    println!(
        "  protheus-ops enterprise-hardening genesis-truth-gate [--strict=1|0] [--regression-pass=1|0] [--dod-pass=1|0] [--verify-pass=1|0]"
    );
    println!(
        "  protheus-ops enterprise-hardening genesis-thin-wrapper-audit [--strict=1|0] [--scan-root=<rel-path>]"
    );
    println!(
        "  protheus-ops enterprise-hardening genesis-doc-freeze [--strict=1|0] [--release-tag=<tag>]"
    );
    println!(
        "  protheus-ops enterprise-hardening genesis-bootstrap [--strict=1|0] [--profile=<id>]"
    );
    println!(
        "  protheus-ops enterprise-hardening genesis-installer-sim [--strict=1|0] [--profile=<standard|airgap|enterprise>]"
    );
    println!(
        "  protheus-ops enterprise-hardening zero-trust-profile [--issuer=<url>] [--cmek-key=<kms://...>] [--private-link=<id>] [--egress=deny|restricted] [--strict=1|0]"
    );
    println!("  protheus-ops enterprise-hardening ops-bridge [--providers=a,b] [--strict=1|0]");
    println!(
        "  protheus-ops enterprise-hardening scale-ha-certify [--regions=<n>] [--airgap-agents=<n>] [--cold-start-ms=<n>] [--strict=1|0]"
    );
    println!(
        "  protheus-ops enterprise-hardening deploy-modules [--profile=<enterprise|airgap>] [--strict=1|0]"
    );
    println!("  protheus-ops enterprise-hardening super-gate [--strict=1|0]");
    println!(
        "  protheus-ops enterprise-hardening adoption-bootstrap [--profile=<id>] [--strict=1|0]"
    );
    println!(
        "  protheus-ops enterprise-hardening replay [--at=<rfc3339> | --receipt-hash=<hash>] [--strict=1|0]"
    );
    println!("  protheus-ops enterprise-hardening explore [--strict=1|0]");
    println!(
        "  protheus-ops enterprise-hardening ai [--model=<ollama/...>] [--prompt=<text>] [--local-only=1|0] [--strict=1|0]"
    );
    println!("  protheus-ops enterprise-hardening sync [--peer-roots=a,b] [--strict=1|0]");
    println!(
        "  protheus-ops enterprise-hardening energy-cert [--agents=<n>] [--idle-watts=<n>] [--task-watts=<n>] [--strict=1|0]"
    );
    println!(
        "  protheus-ops enterprise-hardening migrate-ecosystem [--from=infring|openhands|agent-os] --payload-file=<path> [--strict=1|0]"
    );
    println!(
        "  protheus-ops enterprise-hardening chaos-run [--agents=<n>] [--suite=general|isolate] [--attacks=a,b] [--strict=1|0]"
    );
    println!(
        "  protheus-ops enterprise-hardening assistant-mode [--topic=<id>] [--hand=<id>] [--workspace=<path>] [--strict=1|0]"
    );
    println!("  protheus-ops enterprise-hardening dashboard");
}

fn bool_flag(raw: Option<&str>, fallback: bool) -> bool {
    match raw.map(|v| v.trim().to_ascii_lowercase()) {
        Some(v) if matches!(v.as_str(), "1" | "true" | "yes" | "on") => true,
        Some(v) if matches!(v.as_str(), "0" | "false" | "no" | "off") => false,
        _ => fallback,
    }
}

fn read_json(path: &Path) -> Result<Value, String> {
    let raw = fs::read_to_string(path)
        .map_err(|err| format!("read_json_failed:{}:{err}", path.display()))?;
    serde_json::from_str::<Value>(&raw)
        .map_err(|err| format!("parse_json_failed:{}:{err}", path.display()))
}

fn split_csv(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(|token| token.trim().to_ascii_lowercase())
        .filter(|token| !token.is_empty())
        .collect()
}

fn enterprise_state_root(root: &Path) -> PathBuf {
    crate::core_state_root(root)
        .join("ops")
        .join("enterprise_hardening")
}

fn enterprise_latest_path(root: &Path) -> PathBuf {
    enterprise_state_root(root).join("latest.json")
}

fn enterprise_history_path(root: &Path) -> PathBuf {
    enterprise_state_root(root).join("history.jsonl")
}

fn write_json(path: &Path, payload: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("create_parent_failed:{}:{err}", parent.to_string_lossy()))?;
    }
    let encoded =
        serde_json::to_string_pretty(payload).map_err(|err| format!("encode_json_failed:{err}"))?;
    fs::write(path, format!("{encoded}\n"))
        .map_err(|err| format!("write_json_failed:{}:{err}", path.display()))
}

fn append_jsonl(path: &Path, payload: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("create_parent_failed:{}:{err}", parent.to_string_lossy()))?;
    }
    let mut row =
        serde_json::to_string(payload).map_err(|err| format!("encode_jsonl_failed:{err}"))?;
    row.push('\n');
    fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .and_then(|mut file| std::io::Write::write_all(&mut file, row.as_bytes()))
        .map_err(|err| format!("append_jsonl_failed:{}:{err}", path.display()))
}

fn with_receipt_hash(mut payload: Value) -> Value {
    payload["receipt_hash"] = Value::String(deterministic_receipt_hash(&payload));
    payload
}

fn persist_enterprise_receipt(root: &Path, payload: &Value) -> Result<(), String> {
    write_json(&enterprise_latest_path(root), payload)?;
    append_jsonl(&enterprise_history_path(root), payload)
}

fn file_sha256(path: &Path) -> Result<String, String> {
    let bytes =
        fs::read(path).map_err(|err| format!("read_bytes_failed:{}:{err}", path.display()))?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    Ok(hex::encode(hasher.finalize()))
}

fn manifest_entry(root: &Path, rel: &str) -> Value {
    let path = root.join(rel);
    if !path.exists() {
        return json!({
            "path": rel,
            "exists": false
        });
    }
    let sha256 = file_sha256(&path).unwrap_or_else(|_| String::new());
    let size = fs::metadata(&path).map(|meta| meta.len()).unwrap_or(0);
    json!({
        "path": rel,
        "exists": true,
        "size_bytes": size,
        "sha256": sha256
    })
}

fn collect_files_with_extension(
    dir: &Path,
    extension: &str,
    out: &mut Vec<PathBuf>,
) -> Result<(), String> {
    if !dir.exists() {
        return Ok(());
    }
    let entries =
        fs::read_dir(dir).map_err(|err| format!("read_dir_failed:{}:{err}", dir.display()))?;
    for entry in entries {
        let entry =
            entry.map_err(|err| format!("read_dir_entry_failed:{}:{err}", dir.display()))?;
        let path = entry.path();
        if path.is_dir() {
            collect_files_with_extension(&path, extension, out)?;
        } else if path
            .extension()
            .and_then(|v| v.to_str())
            .map(|v| v.eq_ignore_ascii_case(extension))
            .unwrap_or(false)
        {
            out.push(path);
        }
    }
    Ok(())
}

fn resolve_json_path<'a>(value: &'a Value, dotted_path: &str) -> Option<&'a Value> {
    let mut cur = value;
    for part in dotted_path.split('.') {
        if part.trim().is_empty() {
            return None;
        }
        cur = cur.get(part)?;
    }
    Some(cur)
}

fn file_contains_all(path: &Path, required_tokens: &[String]) -> Result<Vec<String>, String> {
    let body = fs::read_to_string(path)
        .map_err(|err| format!("read_text_failed:{}:{err}", path.display()))?;
    let missing = required_tokens
        .iter()
        .filter(|token| !body.contains(token.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    Ok(missing)
}

fn check_cron_delivery_integrity(root: &Path, path_rel: &str) -> Result<(bool, Value), String> {
    let path = root.join(path_rel);
    let payload = read_json(&path)?;
    let jobs = payload
        .get("jobs")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let mut issues = Vec::<Value>::new();
    let mut enabled_jobs = 0usize;
    for job in jobs {
        let enabled = job.get("enabled").and_then(Value::as_bool).unwrap_or(true);
        if !enabled {
            continue;
        }
        enabled_jobs += 1;
        let name = job.get("name").and_then(Value::as_str).unwrap_or("unknown");
        let id = job.get("id").and_then(Value::as_str).unwrap_or("unknown");
        let target = job
            .get("sessionTarget")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_ascii_lowercase();
        let delivery = job.get("delivery").and_then(Value::as_object);

        if delivery.is_none() {
            issues.push(json!({
                "id": id,
                "name": name,
                "reason": "missing_delivery_for_enabled_job",
                "session_target": target
            }));
            continue;
        }

        let Some(delivery) = delivery else {
            continue;
        };

        let mode = delivery
            .get("mode")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase();
        let channel = delivery
            .get("channel")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase();

        if mode == "none" {
            issues.push(json!({
                "id": id,
                "name": name,
                "reason": "delivery_mode_none_forbidden"
            }));
            continue;
        }

        if mode == "announce" {
            if channel.is_empty() {
                issues.push(json!({
                    "id": id,
                    "name": name,
                    "reason": "announce_channel_missing"
                }));
                continue;
            }
            if !ALLOWED_DELIVERY_CHANNELS.contains(&channel.as_str()) {
                issues.push(json!({
                    "id": id,
                    "name": name,
                    "reason": "announce_channel_invalid",
                    "channel": channel
                }));
            }
        }

        if target == "isolated" && mode != "announce" {
            issues.push(json!({
                "id": id,
                "name": name,
                "reason": "isolated_requires_announce"
            }));
        }
    }

    Ok((
        issues.is_empty(),
        json!({
            "enabled_jobs": enabled_jobs,
            "issues": issues,
            "allowed_channels": ALLOWED_DELIVERY_CHANNELS
        }),
    ))
}
