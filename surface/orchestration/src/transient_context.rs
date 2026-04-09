use infring_layer1_memory::{
    Classification, EphemeralMemoryHeap, TrustState, VerityEphemeralPolicy,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransientContextEntry {
    pub object_id: String,
    pub session_id: String,
    pub value: String,
    pub created_at_ms: u64,
    pub expires_at_ms: u64,
}

#[derive(Debug)]
pub struct TransientContextStore {
    entries: BTreeMap<String, TransientContextEntry>,
    heap: EphemeralMemoryHeap,
}

impl Default for TransientContextStore {
    fn default() -> Self {
        let mut heap = EphemeralMemoryHeap::new(VerityEphemeralPolicy::default());
        heap.grant_debug_principal("orchestration_surface");
        Self {
            entries: BTreeMap::new(),
            heap,
        }
    }
}

impl TransientContextStore {
    pub fn upsert(
        &mut self,
        session_id: &str,
        value: impl Into<String>,
        now_ms: u64,
        ttl_ms: u64,
    ) -> TransientContextEntry {
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
            .unwrap_or_else(|_| format!("transient-fallback-{session_id}-{now_ms}"));
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
        entry
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
            if let Some(object) = self.heap.ephemeral_object(entry.object_id.as_str()) {
                let expected_revision = object.revision_id;
                let _ = self
                    .heap
                    .cleanup_with_cas(
                        entry.object_id.as_str(),
                        expected_revision,
                        "orchestration_transient_sweep",
                        "session_expired",
                        "orchestration_surface",
                    )
                    .ok();
            }
            self.entries.remove(entry.session_id.as_str());
        }
        expired.len()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
