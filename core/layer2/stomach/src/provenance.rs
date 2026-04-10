// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer2/stomach (authoritative)

use crate::quarantine::{FetchMetadata, SnapshotMetadata};
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
    pub fetch_host: String,
    pub fetched_at: String,
    pub commit_hash: String,
    pub tree_hash: String,
    pub blob_hashes: Vec<String>,
    pub fetched_refs: Vec<String>,
    pub spdx: Option<String>,
    pub spdx_decision: SpdxDecision,
    pub notice_locations: Vec<String>,
    pub policy_version: String,
    pub analyzer_version: String,
    pub fetch_receipt_link: String,
    pub receipt_link: String,
}

fn normalized_spdx(spdx: Option<&str>) -> String {
    spdx.unwrap_or("")
        .trim()
        .to_ascii_uppercase()
        .replace(' ', "")
}

pub fn classify_spdx(spdx: Option<&str>) -> SpdxDecision {
    let normalized = normalized_spdx(spdx);
    match normalized.as_str() {
        "MIT" | "APACHE-2.0" | "BSD-3-CLAUSE" | "BSD-2-CLAUSE" | "MPL-2.0" => SpdxDecision::Allow,
        "AGPL-3.0" | "GPL-3.0" | "GPL-2.0" | "BUSL-1.1" | "SSPL-1.0" => SpdxDecision::Deny,
        _ => SpdxDecision::Review,
    }
}

fn require_non_empty(value: &str, error_code: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        Err(error_code.to_string())
    } else {
        Ok(())
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
    fetch: &FetchMetadata,
    spdx: Option<&str>,
    analyzer_version: &str,
    receipt_link: &str,
) -> ProvenanceRecord {
    let blob_hashes = fetch
        .blob_hashes
        .iter()
        .map(|row| format!("{}:{}", row.rel_path, row.sha256))
        .collect::<Vec<_>>();
    ProvenanceRecord {
        snapshot_id: snapshot.snapshot_id.clone(),
        origin_url: snapshot.origin_url.clone(),
        fetch_host: fetch.host.clone(),
        fetched_at: fetch.fetched_at.clone(),
        commit_hash: fetch.commit_hash.trim().to_string(),
        tree_hash: snapshot.tree_hash.clone(),
        blob_hashes,
        fetched_refs: fetch.fetched_refs.clone(),
        spdx: spdx
            .map(|row| row.trim().to_string())
            .filter(|row| !row.is_empty()),
        spdx_decision: classify_spdx(spdx),
        notice_locations: collect_notice_locations(Path::new(&snapshot.quarantine_root)),
        policy_version: fetch.policy_version.trim().to_string(),
        analyzer_version: analyzer_version.trim().to_string(),
        fetch_receipt_link: fetch.fetch_receipt_link.clone(),
        receipt_link: receipt_link.trim().to_string(),
    }
}

pub fn gate_provenance(record: &ProvenanceRecord) -> Result<(), String> {
    require_non_empty(&record.snapshot_id, "provenance_gate_snapshot_id_missing")?;
    require_non_empty(&record.origin_url, "provenance_gate_origin_missing")?;
    require_non_empty(&record.commit_hash, "provenance_gate_commit_missing")?;
    if record.blob_hashes.is_empty() {
        return Err("provenance_gate_blob_hashes_missing".to_string());
    }
    require_non_empty(
        &record.fetch_receipt_link,
        "provenance_gate_fetch_receipt_missing",
    )?;
    if matches!(record.spdx_decision, SpdxDecision::Deny) {
        return Err("provenance_gate_license_denied".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quarantine::FetchMetadata;

    fn sample_snapshot() -> SnapshotMetadata {
        SnapshotMetadata {
            snapshot_id: "snap".to_string(),
            origin_url: "https://github.com/acme/repo".to_string(),
            quarantine_root: ".".to_string(),
            tree_hash: "tree".to_string(),
            file_count: 1,
            symlink_count: 0,
            captured_at: "0".to_string(),
        }
    }

    fn sample_fetch() -> FetchMetadata {
        FetchMetadata {
            snapshot_id: "snap".to_string(),
            origin_url: "https://github.com/acme/repo".to_string(),
            host: "github.com".to_string(),
            commit_hash: "abc123".to_string(),
            fetched_refs: vec!["refs/heads/main".to_string()],
            policy_version: "policy-v1".to_string(),
            fetch_network_allowed: true,
            fetched_at: "1".to_string(),
            fetch_receipt_link: "receipt:snap:fetch".to_string(),
            blob_hashes: vec![crate::quarantine::BlobHash {
                rel_path: "a.rs".to_string(),
                sha256: "hash".to_string(),
            }],
        }
    }

    #[test]
    fn spdx_gate_denies_gpl_like_licenses() {
        assert_eq!(classify_spdx(Some("GPL-3.0")), SpdxDecision::Deny);
    }

    #[test]
    fn spdx_gate_allows_mit() {
        assert_eq!(classify_spdx(Some("MIT")), SpdxDecision::Allow);
    }

    #[test]
    fn provenance_gate_requires_blob_and_fetch_receipt() {
        let snapshot = sample_snapshot();
        let mut fetch = sample_fetch();
        fetch.blob_hashes.clear();
        let blocked = build_provenance(
            &snapshot,
            &fetch,
            Some("MIT"),
            "analyzer-v1",
            "receipt:snap:prov",
        );
        assert!(gate_provenance(&blocked).is_err());

        let ok_record = build_provenance(
            &snapshot,
            &sample_fetch(),
            Some("MIT"),
            "analyzer-v1",
            "receipt:snap:prov",
        );
        assert!(gate_provenance(&ok_record).is_ok());
    }
}
