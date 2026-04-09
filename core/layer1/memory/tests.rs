// SPDX-License-Identifier: Apache-2.0

use super::*;
use serde_json::json;

fn policy() -> VerityEphemeralPolicy {
    let mut policy = VerityEphemeralPolicy {
        max_bytes_per_agent_per_epoch: 512 * 1024,
        max_writes_per_agent_per_epoch: 32,
        ..VerityEphemeralPolicy::default()
    };
    policy
        .promotion_approvers
        .insert("security_gate".to_string());
    policy
}

#[test]
fn writing_to_ephemeral_scope_works() {
    let mut heap = EphemeralMemoryHeap::new(policy());
    let (object, receipt) = heap
        .write_ephemeral(
            "agent:alpha",
            "trace_a",
            json!({"scratch":"value"}),
            Classification::Internal,
            TrustState::Proposed,
            "cap:ephemeral_write",
        )
        .expect("write");
    assert_eq!(object.scope, UnifiedScope::Ephemeral);
    assert_eq!(receipt.scope, "ephemeral");
    assert_eq!(receipt.object_id, object.object_id);
}

#[test]
fn dream_cleanup_removes_non_promoted_ephemeral_objects() {
    let mut heap = EphemeralMemoryHeap::new(policy());
    let (object, _) = heap
        .write_ephemeral(
            "agent:alpha",
            "trace_b",
            json!({"tmp":1}),
            Classification::Internal,
            TrustState::Proposed,
            "cap:ephemeral_write",
        )
        .expect("write");
    let report = heap.run_dream_cleanup("dream_cycle").expect("cleanup");
    assert_eq!(report.cleaned_count, 1);
    let stored = heap
        .ephemeral_object(object.object_id.as_str())
        .expect("stored");
    assert_eq!(stored.terminal_outcome, TerminalOutcome::Cleaned);
}

#[test]
fn promotion_requires_verity_approval_and_creates_copy_forward_target() {
    let mut heap = EphemeralMemoryHeap::new(policy());
    let (object, _) = heap
        .write_ephemeral(
            "agent:alpha",
            "trace_c",
            json!({"important":"keep"}),
            Classification::Sensitive,
            TrustState::Validated,
            "cap:ephemeral_write",
        )
        .expect("write");
    let rev = heap
        .claim_lease(object.object_id.as_str(), "agent:alpha", 10_000)
        .expect("lease");
    let (target, receipt) = heap
        .promote_ephemeral(
            object.object_id.as_str(),
            PermanentScope::Owner,
            "security_gate",
            "agent:alpha",
            rev,
        )
        .expect("promote");
    assert_eq!(receipt.source_object_id, object.object_id);
    assert_eq!(receipt.target_scope, "owner");
    assert_ne!(target.target_object_id, object.object_id);
    let source = heap
        .ephemeral_object(object.object_id.as_str())
        .expect("source");
    assert_eq!(source.terminal_outcome, TerminalOutcome::Promoted);
}

#[test]
fn receipts_and_lineage_are_generated_for_write_promotion_and_cleanup() {
    let mut heap = EphemeralMemoryHeap::new(policy());
    let (promote_obj, _) = heap
        .write_ephemeral(
            "agent:alpha",
            "trace_d1",
            json!({"promote":true}),
            Classification::Internal,
            TrustState::Validated,
            "cap:ephemeral_write",
        )
        .expect("write promote");
    let promote_rev = heap
        .claim_lease(promote_obj.object_id.as_str(), "agent:alpha", 10_000)
        .expect("lease promote");
    let _ = heap
        .promote_ephemeral(
            promote_obj.object_id.as_str(),
            PermanentScope::Core,
            "security_gate",
            "agent:alpha",
            promote_rev,
        )
        .expect("promote");

    let (cleanup_obj, _) = heap
        .write_ephemeral(
            "agent:alpha",
            "trace_d2",
            json!({"cleanup":true}),
            Classification::Internal,
            TrustState::Proposed,
            "cap:ephemeral_write",
        )
        .expect("write cleanup");
    let _ = heap
        .run_dream_cleanup("dream_cycle")
        .expect("cleanup cycle");
    let events = heap.lineage_events();
    assert!(events
        .iter()
        .any(|row| matches!(row, LineageEvent::EphemeralWrite(_))));
    assert!(events
        .iter()
        .any(|row| matches!(row, LineageEvent::EphemeralPromotion(_))));
    assert!(events
        .iter()
        .any(|row| matches!(row, LineageEvent::EphemeralCleanup(_))));
    let cleanup_state = heap
        .ephemeral_object(cleanup_obj.object_id.as_str())
        .expect("cleanup object");
    assert_eq!(cleanup_state.terminal_outcome, TerminalOutcome::Cleaned);
}

#[test]
fn rogue_agents_can_be_revoked_or_throttled() {
    let mut heap = EphemeralMemoryHeap::new(policy());
    heap.set_agent_revoked("agent:rogue", true);
    let revoked = heap.write_ephemeral(
        "agent:rogue",
        "trace_e1",
        json!({"x":1}),
        Classification::Internal,
        TrustState::Proposed,
        "cap:ephemeral_write",
    );
    assert!(matches!(
        revoked,
        Err(EphemeralMemoryError::AccessRevoked(_))
    ));

    heap.set_agent_revoked("agent:rogue", false);
    heap.set_agent_throttled("agent:rogue", true);
    let throttled = heap.write_ephemeral(
        "agent:rogue",
        "trace_e2",
        json!({"x":2}),
        Classification::Internal,
        TrustState::Proposed,
        "cap:ephemeral_write",
    );
    assert!(matches!(
        throttled,
        Err(EphemeralMemoryError::AccessThrottled(_))
    ));
}

#[test]
fn boot_restart_sweeps_stale_non_promoted_payload_before_resume() {
    let mut heap = EphemeralMemoryHeap::new(policy());
    let _ = heap
        .write_ephemeral(
            "agent:alpha",
            "trace_f",
            json!({"stale":"payload"}),
            Classification::Internal,
            TrustState::Proposed,
            "cap:ephemeral_write",
        )
        .expect("write");
    heap.begin_restart();
    let blocked = heap.resume_agents();
    assert!(matches!(
        blocked,
        Err(EphemeralMemoryError::ResumeBlockedByStalePayload(_))
    ));
    let sweep = heap.sweep_stale_before_resume().expect("sweep");
    assert_eq!(sweep.len(), 1);
    heap.resume_agents().expect("resume");
}

#[test]
fn promotion_cleanup_race_resolves_with_one_terminal_outcome() {
    let mut heap = EphemeralMemoryHeap::new(policy());
    let (object, _) = heap
        .write_ephemeral(
            "agent:alpha",
            "trace_g",
            json!({"race":true}),
            Classification::Internal,
            TrustState::Validated,
            "cap:ephemeral_write",
        )
        .expect("write");
    let rev = heap
        .claim_lease(object.object_id.as_str(), "agent:alpha", 10_000)
        .expect("lease");
    let cleanup_cycle = "cycle_race";
    let _cleanup = heap
        .cleanup_with_cas(
            object.object_id.as_str(),
            rev,
            cleanup_cycle,
            "race_cleanup",
            "agent:alpha",
        )
        .expect("cleanup wins");
    let promote = heap.promote_ephemeral(
        object.object_id.as_str(),
        PermanentScope::Core,
        "security_gate",
        "agent:alpha",
        rev,
    );
    assert!(matches!(promote, Err(EphemeralMemoryError::CasMismatch(_))));
    let source = heap
        .ephemeral_object(object.object_id.as_str())
        .expect("source");
    assert_eq!(source.terminal_outcome, TerminalOutcome::Cleaned);
    assert!(heap
        .lineage_events()
        .iter()
        .any(|row| matches!(row, LineageEvent::EphemeralConflict(_))));
}

#[test]
fn default_context_and_owner_export_exclude_ephemeral_by_default() {
    let mut heap = EphemeralMemoryHeap::new(policy());
    let (object, _) = heap
        .write_ephemeral(
            "agent:alpha",
            "trace_h",
            json!({"debug":"only"}),
            Classification::Internal,
            TrustState::Proposed,
            "cap:ephemeral_write",
        )
        .expect("write");
    let default_context = heap.materialize_context_stack_default("operator");
    assert!(default_context.is_empty());
    let default_export = heap.owner_export_default("operator");
    assert!(default_export.is_empty());

    heap.grant_debug_principal("operator");
    let debug_context = heap.materialize_context_stack("operator", true);
    assert!(debug_context
        .iter()
        .any(|row| row.object_id == object.object_id && row.scope == "ephemeral"));
    let debug_export = heap.owner_export("operator", true);
    assert_eq!(debug_export.len(), 1);
}
