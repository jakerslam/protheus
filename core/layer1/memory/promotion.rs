// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer1/memory::promotion (authoritative).

use crate::{
    EphemeralMemoryError, EphemeralMemoryHeap, EphemeralPromotionReceipt, LeaseMode, LineageEvent,
    PermanentScope, PromotedObject, TerminalOutcome,
};
use serde_json::json;

impl EphemeralMemoryHeap {
    pub fn promote_ephemeral(
        &mut self,
        source_object_id: &str,
        target_scope: PermanentScope,
        approved_by: &str,
        lease_holder: &str,
        expected_revision: u64,
    ) -> Result<(PromotedObject, EphemeralPromotionReceipt), EphemeralMemoryError> {
        if !self.policy.can_approve_promotion(approved_by) {
            return Err(EphemeralMemoryError::PromotionApprovalDenied(
                approved_by.to_string(),
            ));
        }
        let now = Self::now_ms();
        let source_snapshot =
            self.objects.get(source_object_id).cloned().ok_or_else(|| {
                EphemeralMemoryError::ObjectNotFound(source_object_id.to_string())
            })?;
        if source_snapshot.terminal_outcome != TerminalOutcome::Active {
            self.push_conflict_receipt(
                source_object_id,
                "promotion",
                expected_revision,
                source_snapshot.revision_id,
                source_snapshot.terminal_outcome.label(),
            );
            return Err(EphemeralMemoryError::CasMismatch(
                source_object_id.to_string(),
            ));
        }
        self.validate_mutation_claim(
            source_object_id,
            &source_snapshot,
            lease_holder,
            expected_revision,
            LeaseMode::Required,
            now,
            "cleanup_or_other_mutation_won",
            "lease_holder_won",
        )?;

        let target_object_id = self.next_entity_id(
            "object_",
            "promotion_object_id",
            json!([
                source_object_id,
                target_scope.label(),
                source_snapshot.content_hash
            ]),
        );
        let promoted = PromotedObject {
            target_object_id: target_object_id.clone(),
            source_object_id: source_object_id.to_string(),
            target_scope: target_scope.clone(),
            classification: source_snapshot.classification.clone(),
            trust_state: source_snapshot.trust_state.clone(),
            capability: source_snapshot.capability.clone(),
            payload: source_snapshot.payload.clone(),
            promoted_at: now,
            lineage_refs: vec![
                source_snapshot.object_id.clone(),
                source_snapshot.trace_id.clone(),
            ],
        };
        self.promoted
            .insert(promoted.target_object_id.clone(), promoted.clone());
        let source = self
            .objects
            .get_mut(source_object_id)
            .ok_or_else(|| EphemeralMemoryError::ObjectNotFound(source_object_id.to_string()))?;

        source.terminal_outcome = TerminalOutcome::Promoted;
        source.promoted_target_object_id = Some(target_object_id.clone());
        source.revision_id = source.revision_id.saturating_add(1);

        let receipt = EphemeralPromotionReceipt {
            receipt_id: self.next_receipt_id(
                "ephemeral_promotion",
                json!([
                    source_object_id,
                    target_object_id,
                    target_scope.label(),
                    approved_by
                ]),
            ),
            source_object_id: source_object_id.to_string(),
            target_object_id,
            target_scope: target_scope.label(),
            approved_by: approved_by.to_string(),
            promoted_at: now,
        };
        self.push_lineage(LineageEvent::EphemeralPromotion(receipt.clone()));
        Ok((promoted, receipt))
    }
}
