// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use super::{kernel_sentinel_root_frame_for_finding, KernelSentinelFinding, KernelSentinelSeverity};

#[derive(Debug, Clone)]
pub(super) struct FindingCluster {
    pub cluster_key: String,
    pub issue_family_fingerprint: String,
    pub issue_family_kind: String,
    pub scenario_id: String,
    pub violated_invariants: Vec<String>,
    pub exemplar: KernelSentinelFinding,
    pub occurrence_count: usize,
    pub first_seen_index: usize,
    pub last_seen_index: usize,
    pub session: String,
    pub surface: String,
    pub receipt_type: String,
    pub recovery_reason: String,
    pub evidence: std::collections::BTreeSet<String>,
}

pub(super) fn severity_rank(severity: KernelSentinelSeverity) -> u8 {
    match severity {
        KernelSentinelSeverity::Critical => 0,
        KernelSentinelSeverity::High => 1,
        KernelSentinelSeverity::Medium => 2,
        KernelSentinelSeverity::Low => 3,
    }
}

pub(super) fn evidence_token(rows: &[String], key: &str) -> Option<String> {
    let needle = format!("{key}=");
    rows.iter().find_map(|row| {
        let start = row.find(&needle)? + needle.len();
        let value = row[start..]
            .split(|ch: char| matches!(ch, ';' | ',' | '|' | '&' | ' ' | '#'))
            .next()
            .unwrap_or("")
            .trim();
        (!value.is_empty()).then(|| value.to_string())
    })
}

pub(super) fn evidence_scheme(rows: &[String], scheme: &str) -> Option<String> {
    let prefix = format!("{scheme}://");
    rows.iter().find_map(|row| {
        let value = row.strip_prefix(&prefix)?;
        let value = value
            .split(|ch: char| matches!(ch, '/' | ';' | ',' | '|' | '&' | ' ' | '#'))
            .next()
            .unwrap_or("")
            .trim();
        (!value.is_empty()).then(|| value.to_string())
    })
}

fn recovery_reason(finding: &KernelSentinelFinding) -> String {
    let text = format!("{} {}", finding.summary, finding.recommended_action).to_lowercase();
    if text.contains("quarantine") {
        "quarantine".to_string()
    } else if text.contains("rollback") {
        "rollback".to_string()
    } else if text.contains("shed") || text.contains("backpressure") {
        "shed_or_defer".to_string()
    } else if text.contains("receipt") {
        "restore_receipt".to_string()
    } else if text.contains("grant") || text.contains("capability") {
        "restore_capability_grant".to_string()
    } else {
        "inspect_kernel_evidence".to_string()
    }
}

pub(super) fn cluster_fields(finding: &KernelSentinelFinding) -> (String, String, String, String) {
    let session = evidence_token(&finding.evidence, "session")
        .or_else(|| evidence_scheme(&finding.evidence, "session"))
        .unwrap_or_else(|| "unknown_session".to_string());
    let surface = evidence_token(&finding.evidence, "surface")
        .or_else(|| evidence_scheme(&finding.evidence, "surface"))
        .unwrap_or_else(|| format!("{:?}", finding.category).to_lowercase());
    let receipt_type = evidence_token(&finding.evidence, "receipt_type")
        .or_else(|| evidence_scheme(&finding.evidence, "receipt"))
        .unwrap_or_else(|| "unspecified_receipt".to_string());
    let recovery_reason = recovery_reason(finding);
    (session, surface, receipt_type, recovery_reason)
}

pub(super) fn issue_family_fingerprint(fingerprint: &str) -> String {
    const MISTY_ROUND_PREFIX: &str = "misty_simulated_round";
    let normalized = fingerprint.to_ascii_lowercase();
    let Some(prefix_index) = normalized.find(MISTY_ROUND_PREFIX) else {
        return fingerprint.to_string();
    };
    let suffix_index = prefix_index + MISTY_ROUND_PREFIX.len();
    let rest = &normalized[suffix_index..];
    let digit_count = rest.chars().take_while(|ch| ch.is_ascii_digit()).count();
    if digit_count == 0 {
        return fingerprint.to_string();
    }
    let after_digits = &rest[digit_count..];
    if matches!(after_digits, "_failure" | "_failures" | "-failure" | "-failures" | ":failure" | ":failures") {
        "synthetic_user_chat_harness:misty_simulated_failures".to_string()
    } else {
        fingerprint.to_string()
    }
}

pub(super) fn synthetic_issue_scenario_id(issue_family_fingerprint: &str) -> String {
    if issue_family_fingerprint == "synthetic_user_chat_harness:misty_simulated_failures" {
        "misty_simulated_failures".to_string()
    } else {
        "none".to_string()
    }
}

pub(super) fn issue_family_kind(scenario_id: &str) -> String {
    if scenario_id == "none" {
        "fingerprint_cluster".to_string()
    } else {
        "synthetic_scenario".to_string()
    }
}

pub(super) fn issue_cluster_key(
    issue_family_fingerprint: &str,
    scenario_id: &str,
    finding: &KernelSentinelFinding,
    violated_invariants: &[String],
) -> String {
    if scenario_id != "none" {
        return format!("scenario={scenario_id}|fingerprint={issue_family_fingerprint}");
    }
    format!(
        "root_frame={}|violated_invariants={}",
        kernel_sentinel_root_frame_for_finding(finding),
        violated_invariants.join(",")
    )
}

pub(super) fn issue_title(finding: &KernelSentinelFinding) -> String {
    format!(
        "[{:?}] Kernel Sentinel {:?}: {}",
        finding.severity, finding.category, finding.summary
    )
}
