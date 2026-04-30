// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use serde_json::{json, Value};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ArtifactEvidenceState {
    Observed,
    Malformed,
    Failed,
    Missing,
}

impl ArtifactEvidenceState {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Observed => "artifact_observed",
            Self::Malformed => "artifact_malformed",
            Self::Failed => "artifact_failed",
            Self::Missing => "artifact_missing",
        }
    }
}

pub(super) fn build_source_report(
    source: &str,
    path: &std::path::Path,
    file_name: &str,
    required: bool,
    required_for_observation: bool,
    collector_family: &str,
    authority_class: Value,
    record_count: usize,
    malformed_record_count: usize,
    failed_record_count: usize,
    present: bool,
) -> Value {
    let artifact_state = if !present {
        ArtifactEvidenceState::Missing
    } else if malformed_record_count > 0 {
        ArtifactEvidenceState::Malformed
    } else if failed_record_count > 0 {
        ArtifactEvidenceState::Failed
    } else {
        ArtifactEvidenceState::Observed
    };
    json!({
        "source": source,
        "path": path,
        "file_name": file_name,
        "present": present,
        "required": required,
        "required_for_observation": required_for_observation,
        "collector_family": collector_family,
        "authority_class": authority_class,
        "record_count": record_count,
        "malformed_record_count": malformed_record_count,
        "failed_record_count": failed_record_count,
        "artifact_state": artifact_state.as_str()
    })
}

pub(super) fn artifact_state_counts(rows: &[Value]) -> BTreeMap<String, usize> {
    let mut out = BTreeMap::<String, usize>::new();
    for row in rows {
        let key = row
            .get("artifact_state")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string();
        *out.entry(key).or_insert(0) += 1;
    }
    out
}
