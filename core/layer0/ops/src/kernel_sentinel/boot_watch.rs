// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use super::{
    KernelSentinelFinding, KernelSentinelFindingCategory, KernelSentinelSeverity,
    KERNEL_SENTINEL_FINDING_SCHEMA_VERSION,
};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn bool_flag(args: &[String], name: &str) -> bool {
    args.iter()
        .any(|arg| arg == name || arg == &format!("{name}=1") || arg == &format!("{name}=true"))
}

fn option_path(args: &[String], name: &str, fallback: PathBuf) -> PathBuf {
    let prefix = format!("{name}=");
    args.iter()
        .find_map(|arg| arg.strip_prefix(&prefix).map(PathBuf::from))
        .unwrap_or(fallback)
}

fn repeated_paths(args: &[String], name: &str) -> Vec<PathBuf> {
    let prefix = format!("{name}=");
    args.iter()
        .filter_map(|arg| arg.strip_prefix(&prefix).map(PathBuf::from))
        .collect()
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn finding(id: String, evidence: Vec<String>, summary: String, action: &str) -> KernelSentinelFinding {
    KernelSentinelFinding {
        schema_version: KERNEL_SENTINEL_FINDING_SCHEMA_VERSION,
        id: id.clone(),
        severity: KernelSentinelSeverity::Critical,
        category: KernelSentinelFindingCategory::RuntimeCorrectness,
        fingerprint: id,
        evidence,
        summary,
        recommended_action: action.to_string(),
        status: "open".to_string(),
    }
}

fn dir_check(label: &str, path: &Path) -> (Value, Option<KernelSentinelFinding>) {
    match fs::create_dir_all(path) {
        Ok(()) => (json!({"check": label, "path": path, "ok": true}), None),
        Err(err) => (
            json!({"check": label, "path": path, "ok": false, "error": err.to_string()}),
            Some(finding(
                format!("boot_self_check:{label}:unavailable"),
                vec![format!("boot://{label}/{}", path.display())],
                format!("Kernel Sentinel boot self-check could not prepare {label}: {err}"),
                "restore writable Kernel Sentinel state before trusting runtime safety reports",
            )),
        ),
    }
}

fn required_path_check(path: &Path) -> (Value, Option<KernelSentinelFinding>) {
    if path.exists() {
        return (json!({"check": "required_path", "path": path, "ok": true}), None);
    }
    (
        json!({"check": "required_path", "path": path, "ok": false, "error": "missing_required_path"}),
        Some(finding(
            format!("boot_self_check:missing_required_path:{}", crate::deterministic_receipt_hash(&json!({"path": path}))),
            vec![format!("boot://required_path/{}", path.display())],
            format!("Kernel Sentinel boot self-check required path is missing: {}", path.display()),
            "restore the required boot policy/proof path or remove it from the boot self-check contract",
        )),
    )
}

pub fn build_boot_watch_report(
    state_dir: &Path,
    args: &[String],
) -> (Value, Vec<KernelSentinelFinding>) {
    let boot_self_check = bool_flag(args, "--boot-self-check");
    let watch_refresh = bool_flag(args, "--watch-refresh");
    let evidence_dir = option_path(args, "--evidence-dir", state_dir.join("evidence"));
    let mut checks = Vec::new();
    let mut findings = Vec::new();
    if boot_self_check || watch_refresh {
        for (row, finding) in [dir_check("state_dir", state_dir), dir_check("evidence_dir", &evidence_dir)] {
            checks.push(row);
            if let Some(finding) = finding {
                findings.push(finding);
            }
        }
    }
    if boot_self_check {
        for path in repeated_paths(args, "--boot-required-path") {
            let (row, finding) = required_path_check(&path);
            checks.push(row);
            if let Some(finding) = finding {
                findings.push(finding);
            }
        }
    }
    (
        json!({
            "ok": findings.is_empty(),
            "type": "kernel_sentinel_boot_watch",
            "boot_self_check_enabled": boot_self_check,
            "background_watch_refresh_requested": watch_refresh,
            "state_dir": state_dir,
            "evidence_dir": evidence_dir,
            "check_count": checks.len(),
            "failure_count": findings.len(),
            "checks": checks,
            "freshness_metadata_contract": {
                "artifact": "watch_freshness.json",
                "writer": "kernel_sentinel",
                "shell_required": false
            }
        }),
        findings,
    )
}

pub fn write_watch_metadata(dir: &Path, report: &Value, args: &[String]) -> Result<(), String> {
    if !bool_flag(args, "--watch-refresh") {
        return Ok(());
    }
    fs::create_dir_all(dir).map_err(|err| err.to_string())?;
    let payload = json!({
        "ok": true,
        "type": "kernel_sentinel_watch_freshness",
        "generated_at_epoch_seconds": unix_now(),
        "freshness_age_seconds": 0,
        "shell_required": false,
        "boot_watch_ok": report["boot_watch"]["ok"],
        "boot_watch_failure_count": report["boot_watch"]["failure_count"],
        "report_verdict": report["verdict"]["verdict"],
        "report_receipt_hash": report["verdict"]["receipt_hash"]
    });
    let body = serde_json::to_string_pretty(&payload).map_err(|err| err.to_string())?;
    fs::write(dir.join("watch_freshness.json"), format!("{body}\n")).map_err(|err| err.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_required_boot_path_opens_finding() {
        let dir = std::env::temp_dir().join("kernel-sentinel-boot-watch-test");
        let missing = dir.join("missing-policy.json");
        let args = vec![
            "--boot-self-check=1".to_string(),
            format!("--boot-required-path={}", missing.display()),
        ];
        let (report, findings) = build_boot_watch_report(&dir, &args);
        assert_eq!(report["failure_count"], Value::from(1));
        assert!(findings[0].fingerprint.starts_with("boot_self_check:missing_required_path:"));
    }

    #[test]
    fn watch_refresh_writes_freshness_without_shell() {
        let dir = std::env::temp_dir().join("kernel-sentinel-watch-refresh-test");
        let args = vec!["--watch-refresh=1".to_string()];
        let report = json!({"verdict": {"verdict": "allow", "receipt_hash": "abc"}});
        write_watch_metadata(&dir, &report, &args).unwrap();
        let raw = fs::read_to_string(dir.join("watch_freshness.json")).unwrap();
        assert!(raw.contains("\"shell_required\": false"));
    }
}
