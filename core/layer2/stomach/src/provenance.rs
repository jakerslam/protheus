// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer2/stomach (authoritative)

use crate::quarantine::SnapshotMetadata;
use serde::{Deserialize, Serialize};
use std::path::Path;
use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SpdxDecision {
    Allow,
    Review,
    Deny,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProvenanceRecord {
    pub snapshot_id: String,
    pub origin_url: String,
    pub commit_hash: String,
    pub tree_hash: String,
    pub fetched_refs: Vec<String>,
    pub spdx: Option<String>,
    pub spdx_decision: SpdxDecision,
    pub notice_locations: Vec<String>,
    pub policy_version: String,
    pub analyzer_version: String,
    pub receipt_link: String,
}

pub fn classify_spdx(spdx: Option<&str>) -> SpdxDecision {
    let normalized = spdx
        .unwrap_or("")
        .trim()
        .to_ascii_uppercase()
        .replace(' ', "");
    match normalized.as_str() {
        "MIT" | "APACHE-2.0" | "BSD-3-CLAUSE" | "BSD-2-CLAUSE" | "MPL-2.0" => SpdxDecision::Allow,
        "AGPL-3.0" | "GPL-3.0" | "GPL-2.0" | "BUSL-1.1" | "SSPL-1.0" => SpdxDecision::Deny,
        _ => SpdxDecision::Review,
    }
}

pub fn collect_notice_locations(snapshot_root: &Path) -> Vec<String> {
    let mut out = Vec::<String>::new();
    for entry in WalkDir::new(snapshot_root)
        .into_iter()
        .filter_map(Result::ok)
    {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|row| row.to_str()) else {
            continue;
        };
        let upper = name.to_ascii_uppercase();
        if upper.starts_with("LICENSE")
            || upper.starts_with("NOTICE")
            || upper.starts_with("COPYING")
        {
            out.push(path.display().to_string());
        }
    }
    out.sort();
    out
}

pub fn build_provenance(
    snapshot: &SnapshotMetadata,
    commit_hash: &str,
    fetched_refs: Vec<String>,
    spdx: Option<&str>,
    policy_version: &str,
    analyzer_version: &str,
    receipt_link: &str,
) -> ProvenanceRecord {
    ProvenanceRecord {
        snapshot_id: snapshot.snapshot_id.clone(),
        origin_url: snapshot.origin_url.clone(),
        commit_hash: commit_hash.trim().to_string(),
        tree_hash: snapshot.tree_hash.clone(),
        fetched_refs,
        spdx: spdx
            .map(|row| row.trim().to_string())
            .filter(|row| !row.is_empty()),
        spdx_decision: classify_spdx(spdx),
        notice_locations: collect_notice_locations(Path::new(&snapshot.quarantine_root)),
        policy_version: policy_version.trim().to_string(),
        analyzer_version: analyzer_version.trim().to_string(),
        receipt_link: receipt_link.trim().to_string(),
    }
}

pub fn gate_provenance(record: &ProvenanceRecord) -> Result<(), String> {
    if record.snapshot_id.trim().is_empty() {
        return Err("provenance_gate_snapshot_id_missing".to_string());
    }
    if record.origin_url.trim().is_empty() {
        return Err("provenance_gate_origin_missing".to_string());
    }
    if record.commit_hash.trim().is_empty() {
        return Err("provenance_gate_commit_missing".to_string());
    }
    if matches!(record.spdx_decision, SpdxDecision::Deny) {
        return Err("provenance_gate_license_denied".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spdx_gate_denies_gpl_like_licenses() {
        assert_eq!(classify_spdx(Some("GPL-3.0")), SpdxDecision::Deny);
    }

    #[test]
    fn spdx_gate_allows_mit() {
        assert_eq!(classify_spdx(Some("MIT")), SpdxDecision::Allow);
    }
}
