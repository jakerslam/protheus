use crate::schemas::{MemoryScope, MemoryVersion, TrustState};
use crate::{deterministic_hash, now_ms};

pub fn is_valid_trust_transition(from: TrustState, to: TrustState) -> bool {
    matches!(
        (from, to),
        (TrustState::Proposed, TrustState::Corroborated)
            | (TrustState::Corroborated, TrustState::Validated)
            | (TrustState::Validated, TrustState::Canonical)
            | (TrustState::Proposed, TrustState::Quarantined)
            | (TrustState::Corroborated, TrustState::Contested)
            | (TrustState::Canonical, TrustState::Revoked)
            | (TrustState::Validated, TrustState::Revoked)
            | (TrustState::Corroborated, TrustState::Revoked)
    )
}

pub fn rollback_head_from_version(
    object_id: &str,
    source_version: &MemoryVersion,
    current_head_version_id: Option<String>,
    receipt_id: &str,
    principal_id: &str,
) -> MemoryVersion {
    let ts = now_ms();
    let payload_hash =
        deterministic_hash(&(source_version.payload.clone(), source_version.scope.label()));
    MemoryVersion {
        version_id: format!(
            "version_{}",
            &deterministic_hash(&(
                object_id.to_string(),
                source_version.version_id.clone(),
                current_head_version_id.clone(),
                receipt_id.to_string(),
                ts
            ))[..24]
        ),
        object_id: object_id.to_string(),
        scope: match &source_version.scope {
            MemoryScope::Ephemeral => MemoryScope::Ephemeral,
            MemoryScope::Public => MemoryScope::Public,
            MemoryScope::Agent(id) => MemoryScope::Agent(id.clone()),
            MemoryScope::Swarm(id) => MemoryScope::Swarm(id.clone()),
            MemoryScope::Core => MemoryScope::Core,
            MemoryScope::Owner => MemoryScope::Owner,
        },
        parent_version_id: current_head_version_id,
        lineage_refs: {
            let mut refs = source_version.lineage_refs.clone();
            refs.push(source_version.version_id.clone());
            refs
        },
        receipt_id: receipt_id.to_string(),
        trust_state: source_version.trust_state.clone(),
        payload: source_version.payload.clone(),
        payload_hash,
        timestamp_ms: ts,
        proposed_by: principal_id.to_string(),
    }
}
