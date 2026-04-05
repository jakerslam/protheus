// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer2/stomach (authoritative)

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RetentionState {
    Retained,
    OnHold,
    EligibleForPurge,
    Purged,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RetentionRecord {
    pub artifact_id: String,
    pub state: RetentionState,
    pub hold_reason: Option<String>,
    pub retained_until: Option<String>,
    pub referenced_by_receipts: Vec<String>,
}

impl RetentionRecord {
    pub fn new(artifact_id: &str) -> Self {
        Self {
            artifact_id: artifact_id.to_string(),
            state: RetentionState::Retained,
            hold_reason: None,
            retained_until: None,
            referenced_by_receipts: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RetentionEvent {
    PlaceHold { reason: String },
    ReleaseHold,
    MarkEligibleForPurge,
    PurgeRequested,
}

pub fn transition_retention(
    record: &mut RetentionRecord,
    event: RetentionEvent,
) -> Result<(), String> {
    match event {
        RetentionEvent::PlaceHold { reason } => {
            record.state = RetentionState::OnHold;
            record.hold_reason = Some(reason);
            Ok(())
        }
        RetentionEvent::ReleaseHold => {
            if record.state != RetentionState::OnHold {
                return Err("retention_release_hold_invalid_state".to_string());
            }
            record.state = RetentionState::Retained;
            record.hold_reason = None;
            Ok(())
        }
        RetentionEvent::MarkEligibleForPurge => {
            if record.state == RetentionState::Purged {
                return Err("retention_already_purged".to_string());
            }
            record.state = RetentionState::EligibleForPurge;
            Ok(())
        }
        RetentionEvent::PurgeRequested => {
            if record.state != RetentionState::EligibleForPurge {
                return Err("retention_purge_requires_eligible_state".to_string());
            }
            Ok(())
        }
    }
}

pub fn can_physically_purge(record: &RetentionRecord) -> bool {
    record.state == RetentionState::EligibleForPurge && record.referenced_by_receipts.is_empty()
}

pub fn purge_artifact_path(path: &Path, record: &mut RetentionRecord) -> Result<(), String> {
    if !can_physically_purge(record) {
        return Err("retention_purge_denied".to_string());
    }
    if path.exists() {
        fs::remove_dir_all(path).map_err(|e| format!("retention_purge_remove_failed:{e}"))?;
    }
    record.state = RetentionState::Purged;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn purge_requires_eligible_state_and_no_receipt_links() {
        let dir = tempdir().expect("tmp");
        let mut record = RetentionRecord::new("x");
        assert!(purge_artifact_path(dir.path(), &mut record).is_err());
        transition_retention(&mut record, RetentionEvent::MarkEligibleForPurge).expect("eligible");
        purge_artifact_path(dir.path(), &mut record).expect("purged");
        assert_eq!(record.state, RetentionState::Purged);
    }
}
