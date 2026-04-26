// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use super::{
    KernelSentinelFinding, KernelSentinelFindingCategory, KernelSentinelSeverity,
    KERNEL_SENTINEL_FINDING_SCHEMA_VERSION,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Deserialize)]
struct RawWaiver {
    #[serde(default)]
    id: String,
    #[serde(default)]
    fingerprint: String,
    #[serde(default)]
    scope: String,
    #[serde(default)]
    approved_by: String,
    #[serde(default)]
    expires_at_epoch: u64,
    #[serde(default)]
    evidence: Vec<String>,
    #[serde(default)]
    rollback_plan: String,
    #[serde(default)]
    mitigation_plan: String,
    #[serde(default)]
    receipt: String,
    #[serde(default)]
    status: String,
}

fn option_path(args: &[String], name: &str, fallback: PathBuf) -> PathBuf {
    let prefix = format!("{name}=");
    args.iter()
        .find_map(|arg| arg.strip_prefix(&prefix).map(PathBuf::from))
        .unwrap_or(fallback)
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn normalized_id(waiver: &RawWaiver, index: usize) -> String {
    if waiver.id.trim().is_empty() {
        format!("waiver-{index}")
    } else {
        waiver.id.trim().to_string()
    }
}

fn human_approved(approved_by: &str) -> bool {
    let normalized = approved_by.trim().to_lowercase();
    (normalized.starts_with("human:") || normalized.starts_with("operator:"))
        && !normalized.contains("kernel_sentinel")
        && !normalized.contains("automation")
        && !normalized.contains("self")
}

fn receipt_present(waiver: &RawWaiver) -> bool {
    !waiver.receipt.trim().is_empty()
        || waiver
            .evidence
            .iter()
            .any(|row| row.starts_with("waiver_receipt://") || row.contains("waiver_receipt"))
}

fn waiver_target(waiver: &RawWaiver) -> String {
    if !waiver.fingerprint.trim().is_empty() {
        waiver.fingerprint.trim().to_string()
    } else {
        waiver.scope.trim().to_string()
    }
}

fn rejection_reasons(waiver: &RawWaiver, now: u64) -> Vec<&'static str> {
    let mut reasons = Vec::new();
    if waiver.status.trim().eq_ignore_ascii_case("revoked") {
        reasons.push("revoked");
    }
    if waiver_target(waiver).is_empty() {
        reasons.push("missing_scope");
    }
    if !human_approved(&waiver.approved_by) {
        reasons.push("missing_human_approval");
    }
    if waiver.expires_at_epoch <= now {
        reasons.push("expired");
    }
    if waiver.evidence.is_empty() {
        reasons.push("missing_evidence_refs");
    }
    if waiver.rollback_plan.trim().is_empty() {
        reasons.push("missing_rollback_plan");
    }
    if waiver.mitigation_plan.trim().is_empty() {
        reasons.push("missing_mitigation_plan");
    }
    if !receipt_present(waiver) {
        reasons.push("missing_waiver_receipt");
    }
    reasons
}

fn rejection_finding(id: &str, target: &str, reasons: &[&str]) -> KernelSentinelFinding {
    KernelSentinelFinding {
        schema_version: KERNEL_SENTINEL_FINDING_SCHEMA_VERSION,
        id: format!("waiver_rejected:{id}"),
        severity: KernelSentinelSeverity::Critical,
        category: KernelSentinelFindingCategory::SecurityBoundary,
        fingerprint: format!("waiver_rejected:{id}:{target}"),
        evidence: vec![format!("waiver://{id};target={target}")],
        summary: format!(
            "Kernel Sentinel rejected waiver {id} for {target}: {}",
            reasons.join(",")
        ),
        recommended_action: "require unexpired human approval, deterministic evidence refs, mitigation, rollback, and waiver receipt before waiving critical findings".to_string(),
        status: "open".to_string(),
    }
}

fn applies_to(waiver: &RawWaiver, finding: &KernelSentinelFinding) -> bool {
    let target = waiver_target(waiver);
    let normalized_target = normalize_scope(&target);
    target == finding.fingerprint
        || target == "*"
        || normalized_target == finding.category_string()
        || normalized_target == format!("category_{}", finding.category_string())
}

trait CategoryString {
    fn category_string(&self) -> String;
}

impl CategoryString for KernelSentinelFinding {
    fn category_string(&self) -> String {
        normalize_scope(&format!("{:?}", self.category))
    }
}

fn normalize_scope(raw: &str) -> String {
    let mut out = String::new();
    let mut previous_lower_or_digit = false;
    for ch in raw.trim().chars() {
        if ch.is_ascii_alphanumeric() {
            if ch.is_ascii_uppercase() && previous_lower_or_digit && !out.ends_with('_') {
                out.push('_');
            }
            out.push(ch.to_ascii_lowercase());
            previous_lower_or_digit = ch.is_ascii_lowercase() || ch.is_ascii_digit();
        } else if !out.ends_with('_') {
            out.push('_');
            previous_lower_or_digit = false;
        }
    }
    out.trim_matches('_').to_string()
}

fn parse_waivers(path: &Path) -> (Vec<RawWaiver>, Vec<Value>) {
    let raw = fs::read_to_string(path).unwrap_or_default();
    let mut waivers = Vec::new();
    let mut malformed = Vec::new();
    for (index, line) in raw.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        match serde_json::from_str::<RawWaiver>(trimmed) {
            Ok(waiver) => waivers.push(waiver),
            Err(err) => malformed.push(json!({
                "line": index + 1,
                "status": "rejected",
                "reason": "malformed_json",
                "error": err.to_string()
            })),
        }
    }
    (waivers, malformed)
}

pub fn apply_waivers(
    findings: &mut [KernelSentinelFinding],
    state_dir: &Path,
    args: &[String],
) -> (Value, Vec<KernelSentinelFinding>) {
    let path = option_path(args, "--waivers-path", state_dir.join("waivers.jsonl"));
    let now = unix_now();
    if !path.exists() {
        return (
            json!({
                "ok": true,
                "type": "kernel_sentinel_waiver_review",
                "path": path,
                "present": false,
                "applied_count": 0,
                "rejected_count": 0,
                "audit_rows": []
            }),
            Vec::new(),
        );
    }
    let (waivers, malformed) = parse_waivers(&path);
    let mut audit_rows = malformed;
    let mut generated = Vec::new();
    let mut applied_count = 0usize;
    let mut rejected_count = audit_rows.len();
    for (index, waiver) in waivers.iter().enumerate() {
        let id = normalized_id(waiver, index + 1);
        let target = waiver_target(waiver);
        let reasons = rejection_reasons(waiver, now);
        if !reasons.is_empty() {
            rejected_count += 1;
            audit_rows.push(json!({
                "id": id,
                "target": target,
                "status": "rejected",
                "reasons": reasons,
                "waiver_receipted": receipt_present(waiver)
            }));
            generated.push(rejection_finding(&id, &target, &reasons));
            continue;
        }
        let mut applied = 0usize;
        for finding in findings.iter_mut() {
            if finding.status == "open"
                && finding.severity == KernelSentinelSeverity::Critical
                && applies_to(waiver, finding)
            {
                finding.status = "waived".to_string();
                finding.evidence.push(format!("waiver://{id};receipt={}", waiver.receipt));
                applied += 1;
            }
        }
        applied_count += applied;
        audit_rows.push(json!({
            "id": id,
            "target": target,
            "status": if applied > 0 { "applied" } else { "valid_no_matching_open_critical" },
            "applied_count": applied,
            "approved_by": waiver.approved_by,
            "expires_at_epoch": waiver.expires_at_epoch,
            "waiver_receipted": true
        }));
    }
    (
        json!({
            "ok": generated.is_empty(),
            "type": "kernel_sentinel_waiver_review",
            "path": path,
            "present": true,
            "applied_count": applied_count,
            "rejected_count": rejected_count,
            "audit_rows": audit_rows
        }),
        generated,
    )
}

pub fn write_waiver_audit(dir: &Path, report: &Value) -> Result<(), String> {
    fs::create_dir_all(dir).map_err(|err| err.to_string())?;
    let mut body = String::new();
    if let Some(rows) = report["waivers"]["audit_rows"].as_array() {
        for row in rows {
            body.push_str(&serde_json::to_string(row).map_err(|err| err.to_string())?);
            body.push('\n');
        }
    }
    fs::write(dir.join("waiver_audit.jsonl"), body).map_err(|err| err.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn critical() -> KernelSentinelFinding {
        KernelSentinelFinding {
            schema_version: KERNEL_SENTINEL_FINDING_SCHEMA_VERSION,
            id: "finding-1".to_string(),
            severity: KernelSentinelSeverity::Critical,
            category: KernelSentinelFindingCategory::ReceiptIntegrity,
            fingerprint: "receipt:missing:mutation".to_string(),
            evidence: vec!["receipt://missing".to_string()],
            summary: "missing receipt".to_string(),
            recommended_action: "restore receipt".to_string(),
            status: "open".to_string(),
        }
    }

    #[test]
    fn valid_human_waiver_marks_matching_critical_as_waived() {
        let dir = std::env::temp_dir().join("kernel-sentinel-waiver-valid");
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("waivers.jsonl"),
            format!(
                "{{\"id\":\"w1\",\"fingerprint\":\"receipt:missing:mutation\",\"approved_by\":\"human:jay\",\"expires_at_epoch\":{},\"evidence\":[\"review://w1\"],\"rollback_plan\":\"restore previous build\",\"mitigation_plan\":\"monitor receipts\",\"receipt\":\"waiver_receipt://w1\"}}\n",
                unix_now() + 3600
            ),
        )
        .unwrap();
        let mut findings = vec![critical()];
        let (report, generated) = apply_waivers(&mut findings, &dir, &[]);
        assert!(generated.is_empty());
        assert_eq!(report["applied_count"], Value::from(1));
        assert_eq!(findings[0].status, "waived");
    }

    #[test]
    fn self_or_expired_waiver_opens_security_finding() {
        let dir = std::env::temp_dir().join("kernel-sentinel-waiver-rejected");
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("waivers.jsonl"),
            "{\"id\":\"w2\",\"fingerprint\":\"receipt:missing:mutation\",\"approved_by\":\"kernel_sentinel:self\",\"expires_at_epoch\":1,\"evidence\":[\"review://w2\"],\"rollback_plan\":\"rollback\",\"mitigation_plan\":\"mitigate\",\"receipt\":\"waiver_receipt://w2\"}\n",
        )
        .unwrap();
        let mut findings = vec![critical()];
        let (report, generated) = apply_waivers(&mut findings, &dir, &[]);
        assert_eq!(report["rejected_count"], Value::from(1));
        assert_eq!(generated[0].category, KernelSentinelFindingCategory::SecurityBoundary);
        assert_eq!(findings[0].status, "open");
    }

    #[test]
    fn category_scope_waiver_matches_snake_case_or_prefixed_scope() {
        let dir = std::env::temp_dir().join("kernel-sentinel-waiver-category-scope");
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("waivers.jsonl"),
            format!(
                "{{\"id\":\"w3\",\"scope\":\"category:receipt_integrity\",\"approved_by\":\"operator:release\",\"expires_at_epoch\":{},\"evidence\":[\"review://w3\"],\"rollback_plan\":\"restore previous build\",\"mitigation_plan\":\"monitor receipts\",\"receipt\":\"waiver_receipt://w3\"}}\n",
                unix_now() + 3600
            ),
        )
        .unwrap();
        let mut findings = vec![critical()];
        let (report, generated) = apply_waivers(&mut findings, &dir, &[]);
        assert!(generated.is_empty());
        assert_eq!(report["applied_count"], Value::from(1));
        assert_eq!(findings[0].status, "waived");
    }
}
