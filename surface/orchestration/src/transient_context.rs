// Layer ownership: surface/orchestration (non-canonical orchestration coordination only).
use crate::contracts::CoreExecutionObservation;
use infring_layer1_memory::{
    Classification, EphemeralMemoryHeap, TerminalOutcome, TrustState, VerityEphemeralPolicy,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransientContextEntry {
    pub object_id: String,
    pub session_id: String,
    pub value: String,
    pub created_at_ms: u64,
    pub expires_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransientSleepCleanupReport {
    pub cleanup_cycle_id: String,
    pub cleaned_count: usize,
    pub bytes_marked_for_reclaim: u64,
    pub reclaimed_payload_bytes: u64,
    pub conflict_count: usize,
    pub removed_session_count: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TransientExecutionObservationEntry {
    pub object_id: String,
    pub session_id: String,
    pub observation: CoreExecutionObservation,
    pub updated_at_ms: u64,
}

#[derive(Debug)]
pub struct TransientContextStore {
    entries: BTreeMap<String, TransientContextEntry>,
    execution_observations: BTreeMap<String, TransientExecutionObservationEntry>,
    heap: EphemeralMemoryHeap,
}

impl Default for TransientContextStore {
    fn default() -> Self {
        let mut heap = EphemeralMemoryHeap::new(VerityEphemeralPolicy::default());
        heap.grant_debug_principal("orchestration_surface");
        Self {
            entries: BTreeMap::new(),
            execution_observations: BTreeMap::new(),
            heap,
        }
    }
}

impl TransientContextStore {
    #[cfg(test)]
    pub(crate) fn with_heap(heap: EphemeralMemoryHeap) -> Self {
        Self {
            entries: BTreeMap::new(),
            execution_observations: BTreeMap::new(),
            heap,
        }
    }

    pub fn upsert(
        &mut self,
        session_id: &str,
        value: impl Into<String>,
        now_ms: u64,
        ttl_ms: u64,
    ) -> Result<TransientContextEntry, String> {
        let expires_at_ms = now_ms.saturating_add(ttl_ms.max(1));
        let payload = serde_json::json!({
            "session_id": session_id,
            "value": value.into(),
            "created_at_ms": now_ms,
            "expires_at_ms": expires_at_ms,
            "surface": "orchestration"
        });
        let object_id = self
            .heap
            .write_ephemeral(
                "orchestration_surface",
                format!("transient:{session_id}:{now_ms}").as_str(),
                payload.clone(),
                Classification::Internal,
                TrustState::Proposed,
                "cap:orchestration_transient_context",
            )
            .map(|(object, _)| object.object_id)
            .map_err(|err| format!("transient_context_write_failed:{err}"))?;
        let value = payload
            .get("value")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_string();
        let entry = TransientContextEntry {
            object_id,
            session_id: session_id.to_string(),
            value,
            created_at_ms: now_ms,
            expires_at_ms,
        };
        self.entries.insert(session_id.to_string(), entry.clone());
        Ok(entry)
    }

    pub fn upsert_execution_observation(
        &mut self,
        session_id: &str,
        observation: CoreExecutionObservation,
        now_ms: u64,
    ) -> Result<(), String> {
        let session_id = session_id.trim();
        if session_id.is_empty() {
            return Ok(());
        }
        let payload = serde_json::json!({
            "session_id": session_id,
            "observation": observation,
            "updated_at_ms": now_ms,
            "surface": "orchestration"
        });
        let object_id = self
            .heap
            .write_ephemeral(
                "orchestration_surface",
                format!("execution_observation:{session_id}:{now_ms}").as_str(),
                payload.clone(),
                Classification::Internal,
                TrustState::Proposed,
                "cap:orchestration_execution_observation",
            )
            .map(|(object, _)| object.object_id)
            .map_err(|err| format!("transient_execution_observation_write_failed:{err}"))?;
        let observation = payload
            .get("observation")
            .cloned()
            .and_then(|value| serde_json::from_value::<CoreExecutionObservation>(value).ok())
            .unwrap_or(CoreExecutionObservation {
                plan_status: None,
                receipt_ids: Vec::new(),
                outcome_refs: Vec::new(),
                step_statuses: Vec::new(),
            });
        let entry = TransientExecutionObservationEntry {
            object_id,
            session_id: session_id.to_string(),
            observation,
            updated_at_ms: now_ms,
        };
        if let Some(previous) = self
            .execution_observations
            .insert(session_id.to_string(), entry)
        {
            self.cleanup_ephemeral_object(
                previous.object_id.as_str(),
                "orchestration_observation_replace",
                "observation_replaced",
            );
        }
        Ok(())
    }

    pub fn execution_observation(&self, session_id: &str) -> Option<&CoreExecutionObservation> {
        self.execution_observations
            .get(session_id)
            .map(|entry| &entry.observation)
    }

    pub fn clear_execution_observation(&mut self, session_id: &str) -> bool {
        let Some(entry) = self.execution_observations.remove(session_id) else {
            return false;
        };
        self.cleanup_ephemeral_object(
            entry.object_id.as_str(),
            "orchestration_observation_clear",
            "observation_cleared",
        );
        true
    }

    pub fn get(&self, session_id: &str) -> Option<&TransientContextEntry> {
        self.entries.get(session_id)
    }

    pub fn sweep_expired(&mut self, now_ms: u64) -> usize {
        let expired = self
            .entries
            .values()
            .filter(|entry| entry.expires_at_ms <= now_ms)
            .cloned()
            .collect::<Vec<_>>();
        for entry in &expired {
            self.cleanup_ephemeral_object(
                entry.object_id.as_str(),
                "orchestration_transient_sweep",
                "session_expired",
            );
            self.entries.remove(entry.session_id.as_str());
        }
        self.prune_inactive_execution_observations();
        let _ = self.heap.reclaim_cleaned_payloads();
        expired.len()
    }

    pub fn run_sleep_cycle_cleanup(
        &mut self,
        sleep_cycle_id: &str,
    ) -> Result<TransientSleepCleanupReport, String> {
        let report = self
            .heap
            .run_sleep_cycle_cleanup(sleep_cycle_id)
            .map_err(|err| format!("transient_context_sleep_cleanup_failed:{err}"))?;
        let entry_count_before = self.entries.len();
        self.entries.retain(|_, entry| {
            self.heap
                .ephemeral_object(entry.object_id.as_str())
                .map(|object| object.terminal_outcome == TerminalOutcome::Active)
                .unwrap_or(false)
        });
        self.execution_observations.retain(|_, entry| {
            self.heap
                .ephemeral_object(entry.object_id.as_str())
                .map(|object| object.terminal_outcome == TerminalOutcome::Active)
                .unwrap_or(false)
        });
        let removed_session_count = entry_count_before.saturating_sub(self.entries.len());
        let reclaimed_payload_bytes = self.heap.reclaim_cleaned_payloads();
        Ok(TransientSleepCleanupReport {
            cleanup_cycle_id: report.cleanup_cycle_id,
            cleaned_count: report.cleaned_count,
            bytes_marked_for_reclaim: report.bytes_marked_for_reclaim,
            reclaimed_payload_bytes,
            conflict_count: report.conflict_count,
            removed_session_count,
        })
    }

    pub fn active_ephemeral_count(&self) -> usize {
        self.heap
            .materialize_context_stack("orchestration_surface", true)
            .into_iter()
            .filter(|row| row.scope == "ephemeral")
            .count()
    }

    pub fn begin_restart(&mut self) {
        self.heap.begin_restart();
    }

    pub fn sweep_stale_before_resume(&mut self) -> Result<usize, String> {
        let receipts = self
            .heap
            .sweep_stale_before_resume()
            .map_err(|err| format!("transient_context_boot_sweep_failed:{err}"))?;
        let cleaned_ids = receipts
            .iter()
            .map(|row| row.object_id.clone())
            .collect::<BTreeSet<_>>();
        self.entries
            .retain(|_, entry| !cleaned_ids.contains(entry.object_id.as_str()));
        self.execution_observations
            .retain(|_, entry| !cleaned_ids.contains(entry.object_id.as_str()));
        let _ = self.heap.reclaim_cleaned_payloads();
        Ok(receipts.len())
    }

    pub fn resume_after_restart(&mut self) -> Result<(), String> {
        self.heap
            .resume_agents()
            .map_err(|err| format!("transient_context_resume_blocked:{err}"))
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    fn cleanup_ephemeral_object(&mut self, object_id: &str, lane: &str, reason: &str) {
        if let Some(object) = self.heap.ephemeral_object(object_id) {
            let expected_revision = object.revision_id;
            let _ = self
                .heap
                .cleanup_with_cas(
                    object_id,
                    expected_revision,
                    lane,
                    reason,
                    "orchestration_surface",
                )
                .ok();
        }
    }

    fn prune_inactive_execution_observations(&mut self) {
        self.execution_observations.retain(|_, entry| {
            self.heap
                .ephemeral_object(entry.object_id.as_str())
                .map(|object| object.terminal_outcome == TerminalOutcome::Active)
                .unwrap_or(false)
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upsert_fails_closed_when_ephemeral_write_is_denied() {
        let mut heap = EphemeralMemoryHeap::new(VerityEphemeralPolicy::default());
        heap.set_agent_revoked("orchestration_surface", true);
        let mut store = TransientContextStore::with_heap(heap);
        let err = store
            .upsert("session-1", "value", 10, 1_000)
            .expect_err("write should fail for revoked actor");
        assert!(err.starts_with("transient_context_write_failed:"));
        assert!(store.is_empty());
    }

    #[test]
    fn restart_requires_stale_sweep_before_resume() {
        let mut store = TransientContextStore::default();
        let _ = store
            .upsert("session-1", "value", 10, 1_000)
            .expect("upsert");
        assert_eq!(store.active_ephemeral_count(), 1);

        store.begin_restart();
        let blocked = store
            .resume_after_restart()
            .expect_err("resume should block");
        assert!(blocked.starts_with("transient_context_resume_blocked:"));

        let swept = store
            .sweep_stale_before_resume()
            .expect("stale sweep should succeed");
        assert_eq!(swept, 1);
        assert_eq!(store.active_ephemeral_count(), 0);
        assert!(store.get("session-1").is_none());

        store
            .resume_after_restart()
            .expect("resume should succeed after sweep");
    }

    #[test]
    fn sleep_cycle_cleanup_wipes_active_transient_context() {
        let mut store = TransientContextStore::default();
        let _ = store
            .upsert("session-1", "value a", 100, 60_000)
            .expect("upsert");
        let _ = store
            .upsert("session-2", "value b", 200, 60_000)
            .expect("upsert");
        assert_eq!(store.len(), 2);
        assert_eq!(store.active_ephemeral_count(), 2);

        let report = store
            .run_sleep_cycle_cleanup("night_cycle")
            .expect("sleep cycle cleanup");
        assert_eq!(report.cleaned_count, 2);
        assert_eq!(report.removed_session_count, 2);
        assert!(report.cleanup_cycle_id.starts_with("cycle_"));
        assert_eq!(store.len(), 0);
        assert_eq!(store.active_ephemeral_count(), 0);
    }
}
