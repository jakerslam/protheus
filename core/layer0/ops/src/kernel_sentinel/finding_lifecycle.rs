// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use super::{validate_finding, KernelSentinelFinding};
use std::collections::BTreeMap;

const MAX_EVIDENCE_REFS_PER_FINDING: usize = 12;

pub(super) fn normalize_finding_status(status: &str) -> String {
    match status.trim().to_ascii_lowercase().replace('-', "_").as_str() {
        "" | "active" | "new" | "unresolved" | "blocking" => "open".to_string(),
        "waive" | "waived_by_human" | "human_waived" => "waived".to_string(),
        "fixed" | "closed" | "complete" | "completed" => "resolved".to_string(),
        other => other.to_string(),
    }
}

pub(super) fn sanitize_finding(mut finding: KernelSentinelFinding) -> KernelSentinelFinding {
    finding.status = normalize_finding_status(&finding.status);
    finding.evidence = finding
        .evidence
        .into_iter()
        .map(|row| redact_evidence_ref(&row))
        .take(MAX_EVIDENCE_REFS_PER_FINDING)
        .collect();
    finding
}

pub(super) fn dedupe_findings(findings: Vec<KernelSentinelFinding>) -> Vec<KernelSentinelFinding> {
    let mut by_fingerprint: BTreeMap<String, KernelSentinelFinding> = BTreeMap::new();
    for mut finding in findings {
        finding.status = normalize_finding_status(&finding.status);
        if validate_finding(&finding).is_err() {
            continue;
        }
        by_fingerprint
            .entry(finding.fingerprint.clone())
            .and_modify(|existing| merge_finding(existing, finding.clone()))
            .or_insert(finding);
    }
    by_fingerprint.into_values().collect()
}

fn redact_evidence_ref(raw: &str) -> String {
    let lowered = raw.to_ascii_lowercase();
    if lowered.contains("token=")
        || lowered.contains("api_key=")
        || lowered.contains("authorization")
        || lowered.contains("github_pat_")
        || lowered.contains("ghp_")
        || lowered.contains("secret")
    {
        "redacted://sensitive-evidence-ref".to_string()
    } else {
        raw.to_string()
    }
}

fn status_rank(status: &str) -> u8 {
    match normalize_finding_status(status).as_str() {
        "open" => 0,
        "waived" => 1,
        "resolved" => 2,
        _ => 3,
    }
}

fn merge_finding(existing: &mut KernelSentinelFinding, incoming: KernelSentinelFinding) {
    let incoming_status = normalize_finding_status(&incoming.status);
    let existing_status = normalize_finding_status(&existing.status);
    let replace = incoming.severity < existing.severity
        || (incoming.severity == existing.severity
            && status_rank(&incoming_status) < status_rank(&existing_status));
    let mut kept = if replace {
        incoming
    } else {
        let mut kept = existing.clone();
        for evidence in incoming.evidence {
            if !kept.evidence.contains(&evidence) {
                kept.evidence.push(evidence);
            }
        }
        kept
    };
    kept.status = normalize_finding_status(&kept.status);
    *existing = kept;
}
