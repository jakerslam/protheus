use crate::schemas::{MemoryDerivation, MemorySalience, MemoryVersion, TrustState};
use crate::{deterministic_hash, now_ms};

fn build_rollback_version_id(
    object_id: &str,
    source_version_id: &str,
    current_head_version_id: Option<&str>,
    receipt_id: &str,
    ts: u64,
) -> String {
    format!(
        "version_{}",
        &deterministic_hash(&(
            object_id.to_string(),
            source_version_id.to_string(),
            current_head_version_id.map(str::to_string),
            receipt_id.to_string(),
            ts
        ))[..24]
    )
}

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
    let version_id = build_rollback_version_id(
        object_id,
        source_version.version_id.as_str(),
        current_head_version_id.as_deref(),
        receipt_id,
        ts,
    );
    let mut lineage_refs = source_version.lineage_refs.clone();
    lineage_refs.push(source_version.version_id.clone());
    MemoryVersion {
        version_id,
        object_id: object_id.to_string(),
        scope: source_version.scope.clone(),
        kind: source_version.kind.clone(),
        parent_version_id: current_head_version_id,
        lineage_refs,
        receipt_id: receipt_id.to_string(),
        trust_state: source_version.trust_state.clone(),
        salience: MemorySalience::for_kind(&source_version.kind, &source_version.trust_state),
        derivation: source_version.derivation.clone().or_else(|| {
            Some(MemoryDerivation {
                kind: crate::schemas::DerivationKind::RetrievalFeedback,
                source_version_ids: vec![source_version.version_id.clone()],
                notes: "rollback restored prior memory head".to_string(),
                confidence_bps: 7000,
            })
        }),
        payload: source_version.payload.clone(),
        payload_hash,
        timestamp_ms: ts,
        proposed_by: principal_id.to_string(),
    }
}
