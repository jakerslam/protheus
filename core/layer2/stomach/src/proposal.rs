// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer2/stomach (authoritative)

use crate::quarantine::SnapshotMetadata;
use crate::stable_hash;
use serde::{Deserialize, Serialize};
use std::path::Path;
use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TransformKind {
    NamespaceFix,
    HeaderInjection,
    PathRemap,
    AdapterScaffold,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TransformRequest {
    pub kind: TransformKind,
    pub target_paths: Vec<String>,
    pub namespace_from: Option<String>,
    pub namespace_to: Option<String>,
    pub header_text: Option<String>,
    pub path_prefix_from: Option<String>,
    pub path_prefix_to: Option<String>,
    pub adapter_name: Option<String>,
}

impl TransformRequest {
    pub fn header_injection(target_paths: Vec<String>, header_text: String) -> Self {
        Self {
            kind: TransformKind::HeaderInjection,
            target_paths,
            namespace_from: None,
            namespace_to: None,
            header_text: Some(header_text),
            path_prefix_from: None,
            path_prefix_to: None,
            adapter_name: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProposalBundle {
    pub proposal_id: String,
    pub snapshot_id: String,
    pub base_snapshot_hash: String,
    pub target_path_set: Vec<String>,
    pub diff_hash: String,
    pub rationale: String,
    pub contributing_features: Vec<String>,
    pub notices_license_implications: Vec<String>,
    pub test_benchmark_plan: Vec<String>,
    pub revert_recipe: String,
    pub transformer_version: String,
    pub parent_receipt_ids: Vec<String>,
    pub patch_preview: Vec<String>,
    pub transform: TransformRequest,
}

fn default_target_paths(snapshot_root: &Path) -> Vec<String> {
    let mut rows = Vec::<String>::new();
    for entry in WalkDir::new(snapshot_root)
        .into_iter()
        .filter_map(Result::ok)
    {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if let Some(raw) = path.file_name().and_then(|row| row.to_str()) {
            rows.push(raw.to_string());
        }
        if rows.len() >= 8 {
            break;
        }
    }
    rows.sort();
    rows
}

fn build_preview_lines(request: &TransformRequest) -> Vec<String> {
    match request.kind {
        TransformKind::NamespaceFix => vec![
            format!(
                "replace namespace {:?} -> {:?}",
                request.namespace_from, request.namespace_to
            ),
            "apply to explicit target path set".to_string(),
        ],
        TransformKind::HeaderInjection => vec![
            format!("inject header {:?}", request.header_text),
            "prepend while preserving shebang boundaries".to_string(),
        ],
        TransformKind::PathRemap => vec![
            format!(
                "remap prefix {:?} -> {:?}",
                request.path_prefix_from, request.path_prefix_to
            ),
            "rewrite import/include targets only".to_string(),
        ],
        TransformKind::AdapterScaffold => vec![
            format!("create adapter scaffold {:?}", request.adapter_name),
            "add only thin adapter bridge stubs".to_string(),
        ],
    }
}

fn rationale_for(kind: &TransformKind) -> &'static str {
    match kind {
        TransformKind::NamespaceFix => "normalize namespace drift to local authority conventions",
        TransformKind::HeaderInjection => {
            "inject mandatory ownership/license headers for imported assets"
        }
        TransformKind::PathRemap => "remap imported paths into allowed workspace boundaries",
        TransformKind::AdapterScaffold => "scaffold thin adapters for external capability mapping",
    }
}

pub fn generate_proposal(
    snapshot: &SnapshotMetadata,
    snapshot_root: &Path,
    request: &TransformRequest,
    craving_signals: &[String],
    parent_receipt_ids: &[String],
    transformer_version: &str,
) -> Result<ProposalBundle, String> {
    let mut targets = request.target_paths.clone();
    if targets.is_empty() {
        targets = default_target_paths(snapshot_root);
    }
    targets.retain(|row| !row.trim().is_empty());
    targets.sort();
    targets.dedup();
    if targets.is_empty() {
        return Err("proposal_target_path_set_empty".to_string());
    }

    let preview = build_preview_lines(request);
    let pre_hash = stable_hash(&(snapshot.snapshot_id.clone(), &targets, &preview, request));
    let proposal_id = format!("proposal-{}", &pre_hash[..16]);
    let diff_hash = stable_hash(&(&proposal_id, request, &targets, &preview));
    let rationale = rationale_for(&request.kind).to_string();

    let mut contributing = vec![
        "deterministic_analysis".to_string(),
        "advisory_cravings".to_string(),
        "policy_gated_transforms".to_string(),
    ];
    for signal in craving_signals {
        let trimmed = signal.trim();
        if trimmed.is_empty() {
            continue;
        }
        contributing.push(format!("craving:{trimmed}"));
    }
    contributing.sort();
    contributing.dedup();

    Ok(ProposalBundle {
        proposal_id,
        snapshot_id: snapshot.snapshot_id.clone(),
        base_snapshot_hash: snapshot.tree_hash.clone(),
        target_path_set: targets,
        diff_hash,
        rationale,
        contributing_features: contributing,
        notices_license_implications: vec![
            "verify retained third-party notices in final merge".to_string(),
            "license gate required before merge approval".to_string(),
        ],
        test_benchmark_plan: vec![
            "cargo test --workspace".to_string(),
            "verify.sh".to_string(),
            "benchmark matrix spot-check on touched lanes".to_string(),
        ],
        revert_recipe: "git apply -R <proposal_patch> && re-run verification gates".to_string(),
        transformer_version: transformer_version.trim().to_string(),
        parent_receipt_ids: parent_receipt_ids.to_vec(),
        patch_preview: preview,
        transform: request.clone(),
    })
}

pub fn validate_proposal_bundle(bundle: &ProposalBundle) -> Result<(), String> {
    if bundle.proposal_id.trim().is_empty() {
        return Err("proposal_bundle_missing_id".to_string());
    }
    if bundle.base_snapshot_hash.trim().is_empty() {
        return Err("proposal_bundle_missing_snapshot_hash".to_string());
    }
    if bundle.target_path_set.is_empty() {
        return Err("proposal_bundle_missing_targets".to_string());
    }
    if bundle.diff_hash.trim().is_empty() {
        return Err("proposal_bundle_missing_diff_hash".to_string());
    }
    if bundle.transformer_version.trim().is_empty() {
        return Err("proposal_bundle_missing_transformer_version".to_string());
    }
    if bundle.parent_receipt_ids.is_empty() {
        return Err("proposal_bundle_missing_parent_receipts".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quarantine::SnapshotMetadata;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn proposal_bundle_has_required_fields() {
        let root = tempdir().expect("tmp");
        fs::write(root.path().join("a.rs"), "fn main(){}").expect("write");
        let snapshot = SnapshotMetadata {
            snapshot_id: "snap-1".to_string(),
            origin_url: "https://github.com/acme/repo".to_string(),
            quarantine_root: root.path().display().to_string(),
            tree_hash: "tree".to_string(),
            file_count: 1,
            symlink_count: 0,
            captured_at: "0".to_string(),
        };
        let req = TransformRequest::header_injection(vec!["a.rs".to_string()], "// x".to_string());
        let bundle = generate_proposal(
            &snapshot,
            root.path(),
            &req,
            &["ast_similarity".to_string()],
            &["r1".to_string()],
            "transform-v1",
        )
        .expect("proposal");
        validate_proposal_bundle(&bundle).expect("valid");
        assert!(!bundle.diff_hash.is_empty());
        assert!(bundle
            .contributing_features
            .iter()
            .any(|row| row == "craving:ast_similarity"));
    }
}
