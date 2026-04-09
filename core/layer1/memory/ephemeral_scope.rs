// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer1/memory::ephemeral_scope (authoritative).

use crate::{
    AgentUsage, Classification, EphemeralMemoryError, EphemeralMemoryHeap, EphemeralObject,
    EphemeralWriteReceipt, LineageEvent, MaterializedEntry, PermanentScope, TerminalOutcome,
    TrustState, UnifiedScope,
};
use serde_json::{json, Value};

impl EphemeralMemoryHeap {
    pub fn set_agent_revoked(&mut self, agent_id: &str, revoked: bool) {
        if revoked {
            self.policy.revoked_agents.insert(agent_id.to_string());
        } else {
            self.policy.revoked_agents.remove(agent_id);
        }
    }

    pub fn set_agent_throttled(&mut self, agent_id: &str, throttled: bool) {
        if throttled {
            self.policy.throttled_agents.insert(agent_id.to_string());
        } else {
            self.policy.throttled_agents.remove(agent_id);
        }
    }

    pub fn grant_debug_principal(&mut self, principal_id: &str) {
        self.policy
            .debug_principals
            .insert(principal_id.to_string());
    }

    pub fn write_ephemeral(
        &mut self,
        writer_agent_id: &str,
        trace_id: &str,
        payload: Value,
        classification: Classification,
        trust_state: TrustState,
        capability: &str,
    ) -> Result<(EphemeralObject, EphemeralWriteReceipt), EphemeralMemoryError> {
        self.policy.can_write(writer_agent_id)?;
        if capability.trim().is_empty() {
            return Err(EphemeralMemoryError::CapabilityRequired);
        }
        let (content_hash, bytes) = Self::payload_hash_and_size(&payload)?;
        let usage = self
            .agent_usage
            .entry(writer_agent_id.to_string())
            .or_insert_with(AgentUsage::default);
        if usage.writes.saturating_add(1) > self.policy.max_writes_per_agent_per_epoch {
            return Err(EphemeralMemoryError::WriteRateLimitExceeded(
                writer_agent_id.to_string(),
            ));
        }
        if usage.bytes_written.saturating_add(bytes) > self.policy.max_bytes_per_agent_per_epoch {
            return Err(EphemeralMemoryError::ByteBudgetExceeded(
                writer_agent_id.to_string(),
            ));
        }
        usage.writes = usage.writes.saturating_add(1);
        usage.bytes_written = usage.bytes_written.saturating_add(bytes);

        let written_at = Self::now_ms();
        let object_id = self.next_entity_id(
            "ephemeral_",
            "ephemeral_object_id",
            json!([writer_agent_id, trace_id, content_hash, written_at]),
        );
        let object = EphemeralObject {
            object_id: object_id.clone(),
            writer_agent_id: writer_agent_id.to_string(),
            trace_id: trace_id.to_string(),
            scope: UnifiedScope::Ephemeral,
            classification,
            trust_state,
            capability: capability.to_string(),
            payload,
            content_hash: content_hash.clone(),
            bytes,
            written_at,
            runtime_epoch: self.runtime_epoch,
            revision_id: 1,
            lease_holder: None,
            lease_expires_at: None,
            terminal_outcome: TerminalOutcome::Active,
            promoted_target_object_id: None,
            cleanup_cycle_id: None,
            cleanup_reason: None,
            canonical: false,
        };
        self.objects.insert(object_id.clone(), object.clone());
        let receipt = EphemeralWriteReceipt {
            receipt_id: self.next_receipt_id(
                "ephemeral_write",
                json!([object_id, writer_agent_id, trace_id, content_hash]),
            ),
            object_id,
            writer_agent_id: writer_agent_id.to_string(),
            trace_id: trace_id.to_string(),
            scope: "ephemeral".to_string(),
            content_hash,
            bytes,
            written_at,
        };
        self.push_lineage(LineageEvent::EphemeralWrite(receipt.clone()));
        Ok((object, receipt))
    }

    pub fn claim_lease(
        &mut self,
        object_id: &str,
        lease_holder: &str,
        ttl_ms: u64,
    ) -> Result<u64, EphemeralMemoryError> {
        let now = Self::now_ms();
        let object = self
            .objects
            .get_mut(object_id)
            .ok_or_else(|| EphemeralMemoryError::ObjectNotFound(object_id.to_string()))?;
        if object.terminal_outcome != TerminalOutcome::Active {
            return Err(EphemeralMemoryError::AlreadyTerminal(
                object.terminal_outcome.label().to_string(),
            ));
        }
        if let (Some(holder), Some(expires_at)) =
            (object.lease_holder.as_ref(), object.lease_expires_at)
        {
            if holder != lease_holder && expires_at > now {
                return Err(EphemeralMemoryError::LeaseHeld(object_id.to_string()));
            }
        }
        object.lease_holder = Some(lease_holder.to_string());
        object.lease_expires_at = Some(now.saturating_add(ttl_ms.max(1)));
        object.revision_id = object.revision_id.saturating_add(1);
        Ok(object.revision_id)
    }

    pub fn heartbeat_lease(
        &mut self,
        object_id: &str,
        lease_holder: &str,
        ttl_ms: u64,
    ) -> Result<u64, EphemeralMemoryError> {
        let now = Self::now_ms();
        let object = self
            .objects
            .get_mut(object_id)
            .ok_or_else(|| EphemeralMemoryError::ObjectNotFound(object_id.to_string()))?;
        let Some(current_holder) = object.lease_holder.as_ref() else {
            return Err(EphemeralMemoryError::LeaseRequired(object_id.to_string()));
        };
        if current_holder != lease_holder {
            return Err(EphemeralMemoryError::LeaseHeld(object_id.to_string()));
        }
        if object.lease_expires_at.unwrap_or(0) <= now {
            return Err(EphemeralMemoryError::LeaseExpired(object_id.to_string()));
        }
        object.lease_expires_at = Some(now.saturating_add(ttl_ms.max(1)));
        object.revision_id = object.revision_id.saturating_add(1);
        Ok(object.revision_id)
    }

    pub fn materialize_context_stack_default(&self, principal_id: &str) -> Vec<MaterializedEntry> {
        self.materialize_context_stack(principal_id, false)
    }

    pub fn materialize_context_stack(
        &self,
        principal_id: &str,
        include_ephemeral: bool,
    ) -> Vec<MaterializedEntry> {
        let allow_ephemeral = include_ephemeral && self.policy.can_debug_ephemeral(principal_id);
        let mut out = self
            .promoted
            .values()
            .map(|row| MaterializedEntry {
                object_id: row.target_object_id.clone(),
                scope: row.target_scope.label(),
                payload: row.payload.clone(),
                redacted: false,
                canonical: true,
            })
            .collect::<Vec<_>>();
        if allow_ephemeral {
            out.extend(
                self.objects
                    .values()
                    .filter(|row| row.terminal_outcome == TerminalOutcome::Active)
                    .map(|row| MaterializedEntry {
                        object_id: row.object_id.clone(),
                        scope: row.scope.label(),
                        payload: row.payload.clone(),
                        redacted: false,
                        canonical: false,
                    }),
            );
        }
        out.sort_by(|a, b| a.object_id.cmp(&b.object_id));
        out
    }

    pub fn owner_export_default(&self, principal_id: &str) -> Vec<Value> {
        self.owner_export(principal_id, false)
    }

    pub fn owner_export(&self, principal_id: &str, include_ephemeral: bool) -> Vec<Value> {
        let mut rows = self
            .promoted
            .values()
            .filter(|row| row.target_scope == PermanentScope::Owner)
            .map(|row| row.payload.clone())
            .collect::<Vec<_>>();
        if include_ephemeral && self.policy.can_debug_ephemeral(principal_id) {
            rows.extend(
                self.objects
                    .values()
                    .filter(|row| row.terminal_outcome == TerminalOutcome::Active)
                    .map(|row| row.payload.clone()),
            );
        }
        rows
    }
}
