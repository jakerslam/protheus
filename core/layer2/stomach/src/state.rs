// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer2/stomach (authoritative)

use crate::burn::{RetentionRecord, RetentionState};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DigestStatus {
    Ingested,
    Analyzed,
    Proposed,
    Verified,
    Assimilated,
    Rejected,
    RolledBack,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StateTransitionReceipt {
    pub item_id: String,
    pub from: DigestStatus,
    pub to: DigestStatus,
    pub receipt_id: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DigestState {
    pub item_id: String,
    pub status: DigestStatus,
    pub retention: RetentionRecord,
    pub last_receipt_id: String,
    pub receipt_ids: Vec<String>,
    pub history: Vec<StateTransitionReceipt>,
}

impl DigestState {
    pub fn new(item_id: &str) -> Self {
        Self {
            item_id: item_id.to_string(),
            status: DigestStatus::Ingested,
            retention: RetentionRecord::new(item_id),
            last_receipt_id: format!("receipt:{item_id}:ingested"),
            receipt_ids: vec![format!("receipt:{item_id}:ingested")],
            history: Vec::new(),
        }
    }

    pub fn retention_state(&self) -> RetentionState {
        self.retention.state.clone()
    }
}

fn transition_allowed(from: &DigestStatus, to: &DigestStatus) -> bool {
    matches!(
        (from, to),
        (DigestStatus::Ingested, DigestStatus::Analyzed)
            | (DigestStatus::Analyzed, DigestStatus::Proposed)
            | (DigestStatus::Proposed, DigestStatus::Verified)
            | (DigestStatus::Verified, DigestStatus::Assimilated)
            | (DigestStatus::Proposed, DigestStatus::Rejected)
            | (DigestStatus::Verified, DigestStatus::Rejected)
            | (_, DigestStatus::RolledBack)
    )
}

pub fn transition(
    state: &mut DigestState,
    to: DigestStatus,
    receipt_id: String,
    reason: &str,
) -> Result<(), String> {
    if !transition_allowed(&state.status, &to) {
        return Err(format!(
            "state_transition_denied:{:?}->{:?}",
            state.status, to
        ));
    }
    let row = StateTransitionReceipt {
        item_id: state.item_id.clone(),
        from: state.status.clone(),
        to: to.clone(),
        receipt_id: receipt_id.clone(),
        reason: reason.trim().to_string(),
    };
    state.status = to;
    state.last_receipt_id = receipt_id.clone();
    state.receipt_ids.push(receipt_id);
    state.history.push(row);
    Ok(())
}

pub fn rollback_by_receipt(
    state: &mut DigestState,
    receipt_id: &str,
    reason: &str,
) -> Result<StateTransitionReceipt, String> {
    if !state.receipt_ids.iter().any(|row| row == receipt_id) {
        return Err("state_rollback_receipt_not_found".to_string());
    }
    let row = StateTransitionReceipt {
        item_id: state.item_id.clone(),
        from: state.status.clone(),
        to: DigestStatus::RolledBack,
        receipt_id: format!("receipt:{}:rollback", state.item_id),
        reason: reason.trim().to_string(),
    };
    state.status = DigestStatus::RolledBack;
    state.last_receipt_id = row.receipt_id.clone();
    state.receipt_ids.push(row.receipt_id.clone());
    state.history.push(row.clone());
    Ok(row)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lifecycle_transitions_are_enforced() {
        let mut state = DigestState::new("x");
        transition(
            &mut state,
            DigestStatus::Analyzed,
            "receipt:x:analyzed".to_string(),
            "ok",
        )
        .expect("analyzed");
        transition(
            &mut state,
            DigestStatus::Proposed,
            "receipt:x:proposed".to_string(),
            "ok",
        )
        .expect("proposed");
        assert_eq!(state.status, DigestStatus::Proposed);
    }

    #[test]
    fn rollback_requires_known_receipt() {
        let mut state = DigestState::new("x");
        assert!(rollback_by_receipt(&mut state, "missing", "reason").is_err());
        rollback_by_receipt(&mut state, "receipt:x:ingested", "manual").expect("rollback");
        assert_eq!(state.status, DigestStatus::RolledBack);
    }
}
