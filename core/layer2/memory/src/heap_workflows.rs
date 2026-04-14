use super::{NexusRouteContext, UnifiedMemoryHeap};
use crate::context_budget::ContextBudgetRequest;
use crate::context_materializer::{
    materialize_context, materialize_topology_context, ContextMaterialization,
    ContextTopologyMaterialization,
};
use crate::context_topology::{
    ContextAppendInput, ContextAppendOutcome, ContextTopologyRebuildReport,
};
use crate::policy::{MemoryPolicyDecision, MemoryPolicyGate, MemoryPolicyRequest, PolicyAction};
use crate::schemas::{
    CapabilityToken, MemoryPurgeRecord, MemoryRetentionPolicy, MemoryScope, MemoryVersion,
    OwnerExportRedactionPolicy, PurgeRelationType, RetentionPurgeReport,
};
use crate::{deterministic_hash, now_ms};
use std::collections::BTreeMap;

impl<P: MemoryPolicyGate + Clone> UnifiedMemoryHeap<P> {
    fn visible_versions_for_scopes(
        &self,
        principal_id: &str,
        capability: &CapabilityToken,
        requested_scopes: &[MemoryScope],
        as_of_ms: Option<u64>,
    ) -> Vec<MemoryVersion> {
        let replay = self.replay_mutation_rows();
        let mut latest_by_object = BTreeMap::<String, MemoryVersion>::new();
        for row in replay {
            if let Some(as_of) = as_of_ms {
                if row.timestamp_ms > as_of {
                    continue;
                }
            }
            if self.version_ledger.is_inactive(row.version_id.as_str()) {
                continue;
            }
            if row.trust_state.is_poisoned() {
                continue;
            }
            if !requested_scopes.is_empty()
                && !requested_scopes.iter().any(|scope| scope == &row.scope)
            {
                continue;
            }
            let decision = self.policy.evaluate(&self.scoped_policy_request(
                principal_id,
                PolicyAction::Read,
                row.scope.clone(),
                row.trust_state.clone(),
                capability,
            ));
            if !decision.allow {
                continue;
            }
            if let Some(version) = self.version_ledger.get(row.version_id.as_str()).cloned() {
                latest_by_object.insert(row.object_id, version);
            }
        }
        latest_by_object.into_values().collect::<Vec<_>>()
    }

    fn scoped_policy_request(
        &self,
        principal_id: &str,
        action: PolicyAction,
        scope: MemoryScope,
        trust_state: crate::schemas::TrustState,
        capability: &CapabilityToken,
    ) -> MemoryPolicyRequest {
        MemoryPolicyRequest {
            principal_id: principal_id.to_string(),
            action,
            source_scope: scope,
            target_scope: None,
            trust_state: Some(trust_state),
            capability: Some(capability.clone()),
            owner_settings: self.config.owner_settings.clone(),
        }
    }

    pub fn apply_retention_policy_and_purge(
        &mut self,
        route: &NexusRouteContext,
        principal_id: &str,
        capability: &CapabilityToken,
        retention: MemoryRetentionPolicy,
        relation_type: PurgeRelationType,
        reason: &str,
        lineage_refs: Vec<String>,
    ) -> Result<RetentionPurgeReport, String> {
        self.ensure_routed(route)?;
        let now = now_ms();
        let mut report = RetentionPurgeReport {
            scanned_objects: 0,
            scanned_versions: 0,
            purged_versions: 0,
            skipped_due_to_policy: 0,
        };

        for object in self.record_store.all_objects() {
            report.scanned_objects = report.scanned_objects.saturating_add(1);
            let mut versions = self
                .version_ledger
                .versions_for_object(object.object_id.as_str());
            versions.sort_by(|a, b| {
                b.timestamp_ms
                    .cmp(&a.timestamp_ms)
                    .then_with(|| b.version_id.cmp(&a.version_id))
            });
            report.scanned_versions = report.scanned_versions.saturating_add(versions.len());
            if versions.is_empty() {
                continue;
            }

            let mut keep = BTreeMap::<String, bool>::new();
            if let Some(head_id) = self.record_store.head_version_id(object.object_id.as_str()) {
                keep.insert(head_id, true);
            }
            for row in versions
                .iter()
                .take(retention.max_versions_per_object.max(1))
            {
                keep.insert(row.version_id.clone(), true);
            }
            for row in versions.iter() {
                if retention
                    .protect_trust_states
                    .iter()
                    .any(|ts| ts == &row.trust_state)
                {
                    keep.insert(row.version_id.clone(), true);
                }
                if let Some(window_ms) = retention.retain_window_ms {
                    let age_ms = now.saturating_sub(row.timestamp_ms);
                    if age_ms <= window_ms {
                        keep.insert(row.version_id.clone(), true);
                    }
                }
            }

            for row in versions.iter() {
                if keep.contains_key(row.version_id.as_str()) {
                    continue;
                }
                if self.version_ledger.is_inactive(row.version_id.as_str()) {
                    continue;
                }
                let decision = self.evaluate_policy(self.scoped_policy_request(
                    principal_id,
                    PolicyAction::Write,
                    object.scope.clone(),
                    row.trust_state.clone(),
                    capability,
                ));
                if !decision.allow {
                    report.skipped_due_to_policy = report.skipped_due_to_policy.saturating_add(1);
                    continue;
                }

                let receipt = self.push_receipt(
                    route,
                    "memory_purge",
                    decision,
                    lineage_refs.clone(),
                    serde_json::json!({
                        "object_id": row.object_id,
                        "version_id": row.version_id,
                        "relation_type": format!("{:?}", relation_type),
                        "reason": reason,
                    }),
                );
                let purge_record = MemoryPurgeRecord {
                    purge_id: format!(
                        "purge_{}",
                        &deterministic_hash(&(
                            row.version_id.clone(),
                            receipt.receipt_id.clone(),
                            now_ms()
                        ))[..24]
                    ),
                    target_version_id: row.version_id.clone(),
                    target_object_id: row.object_id.clone(),
                    relation_type: relation_type.clone(),
                    reason: reason.to_string(),
                    issued_by: principal_id.to_string(),
                    receipt_id: receipt.receipt_id,
                    timestamp_ms: now_ms(),
                    lineage_refs: lineage_refs.clone(),
                };
                self.version_ledger.append_purge_record(purge_record)?;
                report.purged_versions = report.purged_versions.saturating_add(1);
            }

            if let Some(head_id) = self.record_store.head_version_id(object.object_id.as_str()) {
                if self.version_ledger.is_inactive(head_id.as_str()) {
                    let fallback = self
                        .version_ledger
                        .active_versions_for_object(object.object_id.as_str())
                        .into_iter()
                        .max_by(|a, b| {
                            a.timestamp_ms
                                .cmp(&b.timestamp_ms)
                                .then_with(|| a.version_id.cmp(&b.version_id))
                        });
                    if let Some(version) = fallback {
                        self.record_store.set_head_version(
                            object.object_id.as_str(),
                            version.version_id.as_str(),
                        );
                    }
                }
            }
        }
        Ok(report)
    }

    pub fn reconstruct_context_view(
        &mut self,
        route: &NexusRouteContext,
        principal_id: &str,
        capability: &CapabilityToken,
        requested_scopes: Vec<MemoryScope>,
        redaction_policy: Option<OwnerExportRedactionPolicy>,
        as_of_ms: Option<u64>,
        lineage_refs: Vec<String>,
    ) -> Result<ContextMaterialization, String> {
        self.ensure_routed(route)?;
        let versions = self.visible_versions_for_scopes(
            principal_id,
            capability,
            requested_scopes.as_slice(),
            as_of_ms,
        );
        let materialized = materialize_context(
            principal_id,
            requested_scopes.as_slice(),
            redaction_policy
                .unwrap_or_else(|| self.config.owner_settings.export_redaction_policy.clone()),
            versions.as_slice(),
        );
        let decision = MemoryPolicyDecision {
            allow: true,
            decision_id: format!(
                "policy_{}",
                &deterministic_hash(&(principal_id.to_string(), "context_reconstruct"))[..24]
            ),
            reason: "policy_allow".to_string(),
        };
        self.push_receipt(
            route,
            "context_reconstruction",
            decision,
            lineage_refs,
            serde_json::json!({
                "principal_id": principal_id,
                "entry_count": materialized.entries.len(),
                "as_of_ms": as_of_ms,
            }),
        );
        Ok(materialized)
    }

    pub fn append_context_atom(
        &mut self,
        route: &NexusRouteContext,
        principal_id: &str,
        capability: &CapabilityToken,
        input: ContextAppendInput,
        lineage_refs: Vec<String>,
    ) -> Result<ContextAppendOutcome, String> {
        self.ensure_routed(route)?;
        let decision = self.evaluate_policy(MemoryPolicyRequest {
            principal_id: principal_id.to_string(),
            action: PolicyAction::Write,
            source_scope: MemoryScope::Core,
            target_scope: None,
            trust_state: Some(crate::schemas::TrustState::Proposed),
            capability: Some(capability.clone()),
            owner_settings: self.config.owner_settings.clone(),
        });
        if !decision.allow {
            return Err(format!("context_atom_append_denied:{}", decision.reason));
        }
        let mut outcome = self.context_topology.append_atom(input)?;
        let atom_receipt = self.push_receipt(
            route,
            "context_atom_append",
            self.policy_allow_decision(serde_json::json!([
                principal_id,
                outcome.atom.atom_id.clone()
            ])),
            lineage_refs.clone(),
            serde_json::json!({
                "session_id": outcome.atom.session_id,
                "atom_id": outcome.atom.atom_id,
                "sequence_no": outcome.atom.sequence_no,
            }),
        );
        outcome.atom.lineage_refs.push(atom_receipt.receipt_id);

        for span in &mut outcome.sealed_spans {
            let receipt = self.push_receipt(
                route,
                "context_span_seal",
                self.policy_allow_decision(serde_json::json!([principal_id, span.span_id.clone()])),
                lineage_refs.clone(),
                serde_json::json!({
                    "session_id": span.session_id,
                    "span_id": span.span_id,
                    "level": span.level,
                    "coverage": { "start_seq": span.start_seq, "end_seq": span.end_seq },
                }),
            );
            span.receipt_id = receipt.receipt_id.clone();
            self.context_topology.set_span_receipt(
                span.session_id.as_str(),
                span.span_id.as_str(),
                span.receipt_id.as_str(),
            );
        }
        for span in &mut outcome.rolled_up_spans {
            let receipt = self.push_receipt(
                route,
                "context_span_rollup",
                self.policy_allow_decision(serde_json::json!([principal_id, span.span_id.clone()])),
                lineage_refs.clone(),
                serde_json::json!({
                    "session_id": span.session_id,
                    "span_id": span.span_id,
                    "level": span.level,
                    "fidelity_score": span.fidelity_score,
                }),
            );
            span.receipt_id = receipt.receipt_id.clone();
            self.context_topology.set_span_receipt(
                span.session_id.as_str(),
                span.span_id.as_str(),
                span.receipt_id.as_str(),
            );
        }
        Ok(outcome)
    }

    pub fn rebuild_context_topology(
        &mut self,
        route: &NexusRouteContext,
        principal_id: &str,
        capability: &CapabilityToken,
        session_id: &str,
        lineage_refs: Vec<String>,
    ) -> Result<ContextTopologyRebuildReport, String> {
        self.ensure_routed(route)?;
        let decision = self.evaluate_policy(MemoryPolicyRequest {
            principal_id: principal_id.to_string(),
            action: PolicyAction::Read,
            source_scope: MemoryScope::Core,
            target_scope: None,
            trust_state: Some(crate::schemas::TrustState::Validated),
            capability: Some(capability.clone()),
            owner_settings: self.config.owner_settings.clone(),
        });
        if !decision.allow {
            return Err(format!(
                "context_topology_rebuild_denied:{}",
                decision.reason
            ));
        }
        let report = self.context_topology.rebuild_session_topology(session_id)?;
        self.push_receipt(
            route,
            "context_topology_rebuild",
            self.policy_allow_decision(serde_json::json!([principal_id, session_id])),
            lineage_refs,
            serde_json::json!({
                "session_id": report.session_id,
                "atom_count": report.atom_count,
                "rebuilt_span_count": report.rebuilt_span_count,
            }),
        );
        Ok(report)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn materialize_context_topology(
        &mut self,
        route: &NexusRouteContext,
        principal_id: &str,
        capability: &CapabilityToken,
        session_id: &str,
        requested_scopes: Vec<MemoryScope>,
        budget_tokens: u32,
        pinned_anchor_refs: Vec<String>,
        lineage_refs: Vec<String>,
    ) -> Result<ContextTopologyMaterialization, String> {
        reconstruct_context_topology_view(
            self,
            route,
            principal_id,
            capability,
            session_id,
            requested_scopes,
            budget_tokens,
            pinned_anchor_refs,
            lineage_refs,
        )
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn reconstruct_context_topology_view<P: MemoryPolicyGate + Clone>(
    heap: &mut UnifiedMemoryHeap<P>,
    route: &NexusRouteContext,
    principal_id: &str,
    capability: &CapabilityToken,
    session_id: &str,
    requested_scopes: Vec<MemoryScope>,
    budget_tokens: u32,
    pinned_anchor_refs: Vec<String>,
    lineage_refs: Vec<String>,
) -> Result<ContextTopologyMaterialization, String> {
    heap.ensure_routed(route)?;
    let versions = heap.visible_versions_for_scopes(
        principal_id,
        capability,
        requested_scopes.as_slice(),
        None,
    );
    for span in heap.context_topology.compact_sealed_session(session_id)? {
        let receipt = heap.push_receipt(
            route,
            "context_span_rollup",
            heap.policy_allow_decision(serde_json::json!([principal_id, span.span_id.clone()])),
            lineage_refs.clone(),
            serde_json::json!({
                "session_id": span.session_id,
                "span_id": span.span_id,
                "level": span.level,
                "fidelity_score": span.fidelity_score,
            }),
        );
        heap.context_topology.set_span_receipt(
            session_id,
            span.span_id.as_str(),
            receipt.receipt_id.as_str(),
        );
    }
    let (frontier, budget_report) =
        heap.context_topology
            .materialize_frontier(ContextBudgetRequest {
                session_id: session_id.to_string(),
                budget_tokens,
                pinned_anchor_refs,
            });
    let atoms = heap.context_topology.session_atoms(session_id);
    let spans = heap.context_topology.session_spans(session_id);
    let materialized = materialize_topology_context(
        principal_id,
        requested_scopes.as_slice(),
        heap.config.owner_settings.export_redaction_policy.clone(),
        versions.as_slice(),
        atoms.as_slice(),
        spans.as_slice(),
        frontier.clone(),
        budget_report.clone(),
    );
    heap.push_receipt(
        route,
        "context_frontier_update",
        heap.policy_allow_decision(serde_json::json!([principal_id, session_id, "frontier"])),
        lineage_refs,
        serde_json::json!({
            "session_id": session_id,
            "budget_tokens": budget_report.budget_tokens,
            "used_tokens": budget_report.used_tokens,
            "hot_tokens": budget_report.hot_tokens,
            "warm_tokens": budget_report.warm_tokens,
            "cool_tokens": budget_report.cool_tokens,
            "cold_tokens": budget_report.cold_tokens,
            "pinned_tokens": budget_report.pinned_tokens,
            "pressure_state": frontier.pressure_state,
            "fidelity_score": frontier.fidelity_score,
            "fragment_count": materialized.fragments.len(),
        }),
    );
    Ok(materialized)
}
