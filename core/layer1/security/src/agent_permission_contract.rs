use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

pub const DEFAULT_AGENT_PERMISSION_POLICY_PATH: &str =
    "core/layer1/security/config/agent_permission_contract_policy.json";
pub const DEFAULT_AGENT_PERMISSION_REPORT_PATH: &str =
    "core/local/artifacts/agent_permission_contract_guard_current.json";
pub const AGENT_PERMISSION_CONTRACT_SRS_ID: &str = "V12-AGENT-PERM-CONTRACT-001";
pub const AGENT_PERMISSION_CONTRACT_LEGACY_SRS_ID: &str = "V11-AGENT-PERM-001";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentPermissionPolicy {
    pub version: String,
    pub owner: String,
    pub enforcement_mode: String,
    pub manifests: Vec<AgentPermissionManifest>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentPermissionManifest {
    pub manifest_id: String,
    pub agent_id: String,
    pub parent_agent_id: String,
    pub parent_permission_root: String,
    #[serde(default)]
    pub allowed_patch_prefixes: Vec<String>,
    #[serde(default)]
    pub requested_patch_keys: Vec<String>,
    #[serde(default)]
    pub required_telemetry_fields: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BlockedPermissionKeyLineage {
    pub requested_key: String,
    pub agent_id: String,
    pub parent_agent_id: String,
    pub parent_permission_root: String,
    pub reason: String,
    pub nearest_allowed_prefix: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ManifestReceipt {
    pub manifest_id: String,
    pub agent_id: String,
    pub parent_agent_id: String,
    pub enforcement_mode: String,
    pub requested_count: usize,
    pub allowed_count: usize,
    pub blocked_count: usize,
    pub requested_receipt: String,
    pub manifest_receipt: String,
    pub parent_manifest_receipt: String,
    pub telemetry: Value,
    pub blocked_key_lineage: Vec<BlockedPermissionKeyLineage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPermissionContractReport {
    pub ok: bool,
    #[serde(rename = "type")]
    pub report_type: String,
    pub generated_at: String,
    pub policy_path: String,
    pub summary: Value,
    pub checks: Vec<Value>,
    pub manifest_receipts: Vec<ManifestReceipt>,
}

pub fn load_agent_permission_policy(path: &str) -> Result<AgentPermissionPolicy, String> {
    let raw = fs::read_to_string(path).map_err(|err| format!("read_policy_failed:{err}"))?;
    serde_json::from_str(&raw).map_err(|err| format!("parse_policy_failed:{err}"))
}

pub fn evaluate_manifest(
    policy: &AgentPermissionPolicy,
    manifest: &AgentPermissionManifest,
) -> ManifestReceipt {
    let allowed_keys = allowed_patch_keys(manifest);
    let blocked_key_lineage = blocked_patch_key_lineage(manifest);
    let requested_receipt = stable_receipt(json!({
        "kind": "permissions_requested_receipt",
        "manifest_id": manifest.manifest_id,
        "requested_patch_keys": manifest.requested_patch_keys
    }));
    let manifest_receipt = stable_receipt(json!({
        "kind": "permissions_manifest_receipt",
        "manifest_id": manifest.manifest_id,
        "enforcement_mode": policy.enforcement_mode,
        "allowed_patch_keys": allowed_keys,
        "blocked_key_lineage": blocked_key_lineage
    }));
    let parent_manifest_receipt = stable_receipt(json!({
        "kind": "permissions_parent_manifest_receipt",
        "parent_agent_id": manifest.parent_agent_id,
        "parent_permission_root": manifest.parent_permission_root,
        "allowed_patch_prefixes": manifest.allowed_patch_prefixes
    }));
    let blocked_permissions = blocked_key_lineage
        .iter()
        .map(|lineage| lineage.requested_key.clone())
        .collect::<Vec<_>>();
    let telemetry = json!({
        "permissions_parent_clamp_applied": true,
        "permissions_enforcement_mode": policy.enforcement_mode,
        "permissions_requested_receipt": requested_receipt,
        "permissions_manifest_receipt": manifest_receipt,
        "permissions_parent_manifest_receipt": parent_manifest_receipt,
        "permissions_widening_blocked_count": blocked_permissions.len(),
        "permissions_widening_blocked_permissions": blocked_permissions,
        "permissions_parent_agent_id": manifest.parent_agent_id,
        "permissions_blocked_key_lineage": blocked_key_lineage
    });
    ManifestReceipt {
        manifest_id: manifest.manifest_id.clone(),
        agent_id: manifest.agent_id.clone(),
        parent_agent_id: manifest.parent_agent_id.clone(),
        enforcement_mode: policy.enforcement_mode.clone(),
        requested_count: manifest.requested_patch_keys.len(),
        allowed_count: allowed_keys.len(),
        blocked_count: blocked_permissions.len(),
        requested_receipt,
        manifest_receipt,
        parent_manifest_receipt,
        telemetry,
        blocked_key_lineage,
    }
}

pub fn build_agent_permission_contract_report(
    policy_path: &str,
    policy: &AgentPermissionPolicy,
) -> AgentPermissionContractReport {
    let manifest_receipts = policy
        .manifests
        .iter()
        .map(|manifest| evaluate_manifest(policy, manifest))
        .collect::<Vec<_>>();
    let enforcement_ok = policy.enforcement_mode == "fail_closed";
    let clamp_ok = manifest_receipts.iter().all(|receipt| {
        receipt.allowed_count > 0
            && receipt.blocked_count > 0
            && receipt
                .blocked_key_lineage
                .iter()
                .all(|lineage| lineage.reason == "outside_parent_permission_root"
                    || lineage.reason == "missing_allowed_patch_prefix")
    });
    let receipt_ok = manifest_receipts.iter().all(|receipt| {
        receipt.requested_receipt.starts_with("sha256:")
            && receipt.manifest_receipt.starts_with("sha256:")
            && receipt.parent_manifest_receipt.starts_with("sha256:")
            && receipt.telemetry["permissions_enforcement_mode"] == policy.enforcement_mode
    });
    let telemetry_ok = policy.manifests.iter().zip(manifest_receipts.iter()).all(
        |(manifest, receipt)| {
            manifest.required_telemetry_fields.iter().all(|field| {
                receipt
                    .telemetry
                    .get(field)
                    .map(|value| !value.is_null())
                    .unwrap_or(false)
            })
        },
    );
    let lineage_ok = manifest_receipts.iter().all(|receipt| {
        receipt.blocked_key_lineage.iter().all(|lineage| {
            !lineage.requested_key.is_empty()
                && !lineage.parent_permission_root.is_empty()
                && lineage.parent_agent_id == receipt.parent_agent_id
        })
    });
    let checks = vec![
        check_row("agent_permission_enforcement_mode_fail_closed", enforcement_ok),
        check_row("agent_permission_parent_bounded_patch_clamp", clamp_ok),
        check_row("agent_permission_manifest_receipt_contract", receipt_ok),
        check_row("agent_permission_required_telemetry_contract", telemetry_ok),
        check_row("agent_permission_blocked_key_lineage_contract", lineage_ok),
    ];
    let ok = checks
        .iter()
        .all(|check| check.get("ok").and_then(Value::as_bool) == Some(true));
    AgentPermissionContractReport {
        ok,
        report_type: "agent_permission_contract_guard".to_string(),
        generated_at: Utc::now().to_rfc3339(),
        policy_path: policy_path.to_string(),
        summary: json!({
            "srs_id": AGENT_PERMISSION_CONTRACT_SRS_ID,
            "legacy_srs_ids": [AGENT_PERMISSION_CONTRACT_LEGACY_SRS_ID],
            "manifest_count": policy.manifests.len(),
            "requested_key_count": manifest_receipts.iter().map(|receipt| receipt.requested_count).sum::<usize>(),
            "allowed_key_count": manifest_receipts.iter().map(|receipt| receipt.allowed_count).sum::<usize>(),
            "blocked_key_count": manifest_receipts.iter().map(|receipt| receipt.blocked_count).sum::<usize>(),
            "enforcement_mode": policy.enforcement_mode,
            "pass": ok
        }),
        checks,
        manifest_receipts,
    }
}

pub fn run_agent_permission_contract_guard(
    policy_path: &str,
    out_json: &str,
    strict: bool,
) -> Result<AgentPermissionContractReport, String> {
    let policy = load_agent_permission_policy(policy_path)?;
    let report = build_agent_permission_contract_report(policy_path, &policy);
    write_report(out_json, &report)?;
    if strict && !report.ok {
        return Err("agent_permission_contract_guard_failed".to_string());
    }
    Ok(report)
}

fn allowed_patch_keys(manifest: &AgentPermissionManifest) -> Vec<String> {
    manifest
        .requested_patch_keys
        .iter()
        .filter(|key| patch_key_allowed(manifest, key).is_ok())
        .cloned()
        .collect()
}

fn blocked_patch_key_lineage(
    manifest: &AgentPermissionManifest,
) -> Vec<BlockedPermissionKeyLineage> {
    manifest
        .requested_patch_keys
        .iter()
        .filter_map(|key| {
            patch_key_allowed(manifest, key).err().map(|reason| BlockedPermissionKeyLineage {
                requested_key: key.clone(),
                agent_id: manifest.agent_id.clone(),
                parent_agent_id: manifest.parent_agent_id.clone(),
                parent_permission_root: manifest.parent_permission_root.clone(),
                nearest_allowed_prefix: nearest_allowed_prefix(manifest, key),
                reason,
            })
        })
        .collect()
}

fn patch_key_allowed(manifest: &AgentPermissionManifest, key: &str) -> Result<(), String> {
    if !under_prefix(key, manifest.parent_permission_root.as_str()) {
        return Err("outside_parent_permission_root".to_string());
    }
    if !manifest
        .allowed_patch_prefixes
        .iter()
        .any(|prefix| under_prefix(key, prefix))
    {
        return Err("missing_allowed_patch_prefix".to_string());
    }
    Ok(())
}

fn under_prefix(key: &str, prefix: &str) -> bool {
    key == prefix || key.strip_prefix(prefix).map(|tail| tail.starts_with('.')).unwrap_or(false)
}

fn nearest_allowed_prefix(manifest: &AgentPermissionManifest, key: &str) -> Option<String> {
    manifest
        .allowed_patch_prefixes
        .iter()
        .filter(|prefix| common_prefix_len(prefix, key) > 0)
        .max_by_key(|prefix| common_prefix_len(prefix, key))
        .cloned()
}

fn common_prefix_len(left: &str, right: &str) -> usize {
    left.chars()
        .zip(right.chars())
        .take_while(|(left, right)| left == right)
        .count()
}

fn stable_receipt(value: Value) -> String {
    let raw = serde_json::to_vec(&value).unwrap_or_default();
    let digest = Sha256::digest(raw);
    format!("sha256:{}", hex::encode(digest))
}

fn check_row(id: &str, ok: bool) -> Value {
    json!({ "id": id, "ok": ok })
}

fn write_report(out_json: &str, report: &AgentPermissionContractReport) -> Result<(), String> {
    let path = Path::new(out_json);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| format!("create_report_dir_failed:{err}"))?;
    }
    let raw = serde_json::to_string_pretty(report)
        .map_err(|err| format!("serialize_report_failed:{err}"))?;
    fs::write(path, format!("{raw}\n")).map_err(|err| format!("write_report_failed:{err}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn policy() -> AgentPermissionPolicy {
        serde_json::from_str(include_str!(
            "../config/agent_permission_contract_policy.json"
        ))
        .expect("policy")
    }

    #[test]
    fn parent_bounded_patch_clamp_blocks_widening_keys() {
        let policy = policy();
        let receipt = evaluate_manifest(&policy, &policy.manifests[0]);
        let reasons = receipt
            .blocked_key_lineage
            .iter()
            .map(|lineage| lineage.reason.as_str())
            .collect::<BTreeSet<_>>();
        assert!(reasons.contains("outside_parent_permission_root"));
        assert!(reasons.contains("missing_allowed_patch_prefix"));
        assert_eq!(receipt.allowed_count, 2);
        assert_eq!(receipt.blocked_count, 2);
    }

    #[test]
    fn manifest_receipts_include_enforcement_mode_and_lineage() {
        let policy = policy();
        let report = build_agent_permission_contract_report(
            DEFAULT_AGENT_PERMISSION_POLICY_PATH,
            &policy,
        );
        assert!(report.ok);
        let receipt = &report.manifest_receipts[0];
        assert_eq!(receipt.enforcement_mode, "fail_closed");
        assert_eq!(receipt.telemetry["permissions_enforcement_mode"], "fail_closed");
        assert!(receipt.manifest_receipt.starts_with("sha256:"));
        assert!(!receipt.blocked_key_lineage.is_empty());
    }

    #[test]
    fn required_telemetry_fields_are_fail_closed() {
        let policy = policy();
        let report = build_agent_permission_contract_report(
            DEFAULT_AGENT_PERMISSION_POLICY_PATH,
            &policy,
        );
        let telemetry = &report.manifest_receipts[0].telemetry;
        for field in &policy.manifests[0].required_telemetry_fields {
            assert!(telemetry.get(field).is_some(), "missing {field}");
        }
    }
}
