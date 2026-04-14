// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer1/memory::cleanup (authoritative).

use crate::{
    EphemeralCleanupReceipt, EphemeralMemoryError, EphemeralMemoryHeap, LeaseMode, LineageEvent,
    TerminalOutcome,
};
use serde_json::{json, Value};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CleanupReport {
    pub cleanup_cycle_id: String,
    pub cleaned_count: usize,
    pub bytes_marked_for_reclaim: u64,
    pub conflict_count: usize,
}

impl EphemeralMemoryHeap {
    pub fn run_sleep_cycle_cleanup(
        &mut self,
        sleep_cycle_id: &str,
    ) -> Result<CleanupReport, EphemeralMemoryError> {
        let normalized_cycle = sleep_cycle_id.trim();
        let cleanup_reason = if normalized_cycle.is_empty() {
            "sleep_cycle_ephemeral_wipe".to_string()
        } else {
            format!("sleep_cycle_ephemeral_wipe:{normalized_cycle}")
        };
        self.run_dream_cleanup(cleanup_reason.as_str())
    }

    pub fn run_dream_cleanup(
        &mut self,
        cleanup_reason: &str,
    ) -> Result<CleanupReport, EphemeralMemoryError> {
        let cleanup_cycle_id = self.next_cleanup_cycle_id("dream");
        let mut cleaned_count = 0_usize;
        let mut bytes_marked_for_reclaim = 0_u64;
        let mut conflict_count = 0_usize;
        for object_id in self.active_non_promoted_ids() {
            let expected_revision = self
                .objects
                .get(object_id.as_str())
                .map(|row| row.revision_id)
                .unwrap_or(0);
            match self.cleanup_with_cas(
                object_id.as_str(),
                expected_revision,
                cleanup_cycle_id.as_str(),
                cleanup_reason,
                "verity_cleanup",
            ) {
                Ok(receipt) => {
                    cleaned_count = cleaned_count.saturating_add(1);
                    bytes_marked_for_reclaim =
                        bytes_marked_for_reclaim.saturating_add(receipt.bytes_reclaimed);
                }
                Err(EphemeralMemoryError::CasMismatch(_))
                | Err(EphemeralMemoryError::LeaseHeld(_))
                | Err(EphemeralMemoryError::LeaseExpired(_)) => {
                    conflict_count = conflict_count.saturating_add(1);
                }
                Err(err) => return Err(err),
            }
        }
        Ok(CleanupReport {
            cleanup_cycle_id,
            cleaned_count,
            bytes_marked_for_reclaim,
            conflict_count,
        })
    }

    pub fn cleanup_with_cas(
        &mut self,
        object_id: &str,
        expected_revision: u64,
        cleanup_cycle_id: &str,
        cleanup_reason: &str,
        contender: &str,
    ) -> Result<EphemeralCleanupReceipt, EphemeralMemoryError> {
        let now = Self::now_ms();
        let snapshot = self
            .objects
            .get(object_id)
            .cloned()
            .ok_or_else(|| EphemeralMemoryError::ObjectNotFound(object_id.to_string()))?;
        match snapshot.terminal_outcome {
            TerminalOutcome::Promoted => {
                return Err(EphemeralMemoryError::AlreadyTerminal(
                    TerminalOutcome::Promoted.label().to_string(),
                ));
            }
            TerminalOutcome::Cleaned => {
                return Err(EphemeralMemoryError::AlreadyTerminal(
                    TerminalOutcome::Cleaned.label().to_string(),
                ));
            }
            TerminalOutcome::Active => {}
        }
        self.validate_mutation_claim(
            object_id,
            &snapshot,
            contender,
            expected_revision,
            LeaseMode::Optional,
            now,
            "competing_mutation_won",
            "lease_holder_won",
        )?;
        let receipt_id = self.next_receipt_id(
            "ephemeral_cleanup",
            json!([object_id, cleanup_cycle_id, cleanup_reason, snapshot.bytes]),
        );
        let object = self
            .objects
            .get_mut(object_id)
            .ok_or_else(|| EphemeralMemoryError::ObjectNotFound(object_id.to_string()))?;

        object.terminal_outcome = TerminalOutcome::Cleaned;
        object.cleanup_cycle_id = Some(cleanup_cycle_id.to_string());
        object.cleanup_reason = Some(cleanup_reason.to_string());
        object.revision_id = object.revision_id.saturating_add(1);

        let receipt = EphemeralCleanupReceipt {
            receipt_id,
            object_id: object_id.to_string(),
            cleanup_cycle_id: cleanup_cycle_id.to_string(),
            cleanup_reason: cleanup_reason.to_string(),
            bytes_reclaimed: object.bytes,
            cleaned_at: now,
        };
        self.push_lineage(LineageEvent::EphemeralCleanup(receipt.clone()));
        Ok(receipt)
    }

    pub fn begin_restart(&mut self) {
        self.runtime_epoch = self.runtime_epoch.saturating_add(1);
        self.resume_blocked = true;
        self.agent_usage.clear();
    }

    pub fn sweep_stale_before_resume(
        &mut self,
    ) -> Result<Vec<EphemeralCleanupReceipt>, EphemeralMemoryError> {
        let cycle_id = self.next_cleanup_cycle_id("boot");
        let stale_ids = self
            .objects
            .values()
            .filter(|row| row.runtime_epoch < self.runtime_epoch)
            .filter(|row| row.terminal_outcome == TerminalOutcome::Active)
            .map(|row| row.object_id.clone())
            .collect::<Vec<_>>();
        let mut receipts = Vec::new();
        for object_id in stale_ids {
            let expected_revision = self
                .objects
                .get(object_id.as_str())
                .map(|row| row.revision_id)
                .unwrap_or(0);
            let receipt = self.cleanup_with_cas(
                object_id.as_str(),
                expected_revision,
                cycle_id.as_str(),
                "boot_stale_ephemeral_sweep",
                "verity_boot_sweep",
            )?;
            receipts.push(receipt);
        }
        self.resume_blocked = self.stale_non_promoted_count() > 0;
        Ok(receipts)
    }

    pub fn resume_agents(&mut self) -> Result<(), EphemeralMemoryError> {
        let stale = self.stale_non_promoted_count();
        if stale > 0 {
            self.resume_blocked = true;
            return Err(EphemeralMemoryError::ResumeBlockedByStalePayload(stale));
        }
        self.resume_blocked = false;
        Ok(())
    }

    pub fn reclaim_cleaned_payloads(&mut self) -> u64 {
        let mut reclaimed = 0_u64;
        for object in self.objects.values_mut() {
            if object.terminal_outcome != TerminalOutcome::Cleaned {
                continue;
            }
            if object.payload != Value::Null {
                reclaimed = reclaimed.saturating_add(object.bytes);
                object.payload = Value::Null;
            }
        }
        reclaimed
    }

    fn active_non_promoted_ids(&self) -> Vec<String> {
        let mut ids = self
            .objects
            .values()
            .filter(|row| row.terminal_outcome == TerminalOutcome::Active)
            .map(|row| row.object_id.clone())
            .collect::<Vec<_>>();
        ids.sort();
        ids
    }

    fn stale_non_promoted_count(&self) -> usize {
        self.objects
            .values()
            .filter(|row| row.runtime_epoch < self.runtime_epoch)
            .filter(|row| row.terminal_outcome == TerminalOutcome::Active)
            .count()
    }
}
