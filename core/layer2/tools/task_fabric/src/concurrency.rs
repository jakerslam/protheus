use crate::policy::{MutationKind, PolicyDecision};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MutationEnvelope {
    pub actor: String,
    pub trace_id: String,
    pub idempotency_key: String,
    pub expected_revision: Option<u64>,
    pub now_ms: u64,
    pub mutation_kind: MutationKind,
    pub payload: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaskEvent {
    pub event_id: String,
    pub event_sequence: u64,
    pub task_id: Option<String>,
    pub scope_id: String,
    pub mutation_kind: MutationKind,
    pub actor: String,
    pub trace_id: String,
    pub idempotency_key: String,
    pub previous_revision: Option<u64>,
    pub next_revision: Option<u64>,
    pub timestamp_ms: u64,
    pub policy: PolicyDecision,
    pub dna_lineage: Vec<String>,
    pub receipt_id: String,
    pub payload: Value,
}

#[derive(Debug, Clone, Default)]
pub struct ConcurrencyState {
    pub next_event_sequence: u64,
    pub idempotency_map: BTreeMap<String, TaskEvent>,
}

impl ConcurrencyState {
    pub fn idempotent_event(&self, key: &str) -> Option<TaskEvent> {
        self.idempotency_map.get(key).cloned()
    }

    pub fn allocate_event_sequence(&mut self) -> u64 {
        self.next_event_sequence = self.next_event_sequence.saturating_add(1);
        self.next_event_sequence
    }

    pub fn record_event(&mut self, event: TaskEvent) {
        self.idempotency_map
            .insert(event.idempotency_key.clone(), event);
    }
}

pub fn validate_expected_revision(expected: Option<u64>, actual: u64) -> Result<(), String> {
    if let Some(v) = expected {
        if v != actual {
            return Err("compare_and_swap_revision_mismatch".to_string());
        }
    }
    Ok(())
}
