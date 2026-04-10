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
    pub explicit_purge_approval_receipt: Option<String>,
    pub referenced_by_receipts: Vec<String>,
    pub referenced_by_benchmarks: Vec<String>,
    pub referenced_by_proposals: Vec<String>,
}

impl RetentionRecord {
    pub fn new(artifact_id: &str) -> Self {
        Self {
            artifact_id: artifact_id.to_string(),
            state: RetentionState::Retained,
            hold_reason: None,
            retained_until: None,
            explicit_purge_approval_receipt: None,
            referenced_by_receipts: Vec::new(),
            referenced_by_benchmarks: Vec::new(),
            referenced_by_proposals: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RetentionEvent {
    PlaceHold { reason: String },
    ReleaseHold,
    SetRetainedUntil { epoch_secs: u64 },
    ApprovePurge { receipt_id: String },
    MarkEligibleForPurge,
    PurgeRequested,
}

fn now_epoch_secs() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn ensure_state(
    record: &RetentionRecord,
    expected: RetentionState,
    error_code: &str,
) -> Result<(), String> {
    if record.state != expected {
        Err(error_code.to_string())
    } else {
        Ok(())
    }
}

fn has_external_references(record: &RetentionRecord) -> bool {
    !record.referenced_by_receipts.is_empty()
        || !record.referenced_by_benchmarks.is_empty()
        || !record.referenced_by_proposals.is_empty()
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
            ensure_state(
                record,
                RetentionState::OnHold,
                "retention_release_hold_invalid_state",
            )?;
            record.state = RetentionState::Retained;
            record.hold_reason = None;
            Ok(())
        }
        RetentionEvent::SetRetainedUntil { epoch_secs } => {
            record.retained_until = Some(epoch_secs.to_string());
            Ok(())
        }
        RetentionEvent::ApprovePurge { receipt_id } => {
            if receipt_id.trim().is_empty() {
                return Err("retention_approve_missing_receipt".to_string());
            }
            record.explicit_purge_approval_receipt = Some(receipt_id);
            Ok(())
        }
        RetentionEvent::MarkEligibleForPurge => {
            if record.state == RetentionState::Purged {
                return Err("retention_already_purged".to_string());
            }
            if record.hold_reason.is_some() {
                return Err("retention_mark_eligible_blocked_by_hold".to_string());
            }
            if !retention_ttl_elapsed(record, now_epoch_secs()) {
                return Err("retention_mark_eligible_ttl_not_elapsed".to_string());
            }
            record.state = RetentionState::EligibleForPurge;
            Ok(())
        }
        RetentionEvent::PurgeRequested => {
            ensure_state(
                record,
                RetentionState::EligibleForPurge,
                "retention_purge_requires_eligible_state",
            )?;
            Ok(())
        }
    }
}

pub fn retention_ttl_elapsed(record: &RetentionRecord, now_epoch_secs: u64) -> bool {
    let Some(raw) = record.retained_until.as_deref() else {
        return false;
    };
    raw.trim()
        .parse::<u64>()
        .ok()
        .map(|deadline| now_epoch_secs >= deadline)
        .unwrap_or(false)
}

pub fn can_physically_purge(record: &RetentionRecord, now_epoch_secs: u64) -> bool {
    record.state == RetentionState::EligibleForPurge
        && record.hold_reason.is_none()
        && record.explicit_purge_approval_receipt.is_some()
        && retention_ttl_elapsed(record, now_epoch_secs)
        && !has_external_references(record)
}

pub fn purge_artifact_path(
    path: &Path,
    record: &mut RetentionRecord,
    now_epoch_secs: u64,
) -> Result<(), String> {
    if !can_physically_purge(record, now_epoch_secs) {
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
        assert!(purge_artifact_path(dir.path(), &mut record, now_epoch_secs()).is_err());
        transition_retention(
            &mut record,
            RetentionEvent::SetRetainedUntil {
                epoch_secs: now_epoch_secs().saturating_sub(1),
            },
        )
        .expect("set ttl");
        transition_retention(
            &mut record,
            RetentionEvent::ApprovePurge {
                receipt_id: "receipt:x:approve".to_string(),
            },
        )
        .expect("approve");
        transition_retention(&mut record, RetentionEvent::MarkEligibleForPurge).expect("eligible");
        purge_artifact_path(dir.path(), &mut record, now_epoch_secs()).expect("purged");
        assert_eq!(record.state, RetentionState::Purged);
    }
}
