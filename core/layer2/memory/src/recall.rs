use crate::heap_interface::{NexusRouteContext, UnifiedMemoryHeap};
use crate::policy::{MemoryPolicyGate, MemoryPolicyRequest, PolicyAction};
use crate::schemas::{
    CapabilityToken, DerivationKind, MemoryDerivation, MemoryInvalidationReason,
    MemoryInvalidationRecord, MemoryKind, MemorySalience, MemoryScope, MemoryVersion,
    TrustState,
};
use crate::vector_index::{embed_text, VectorQueryFilter};
use crate::{deterministic_hash, now_ms};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct MemoryRecallQuery {
    pub query: String,
    pub requested_scopes: Vec<MemoryScope>,
    pub top_k: usize,
    pub allowed_kinds: Vec<MemoryKind>,
    pub session_entity_hints: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct MemoryRecallExplanation {
    pub lexical_score: i64,
    pub vector_score: i64,
    pub graph_score: i64,
    pub session_score: i64,
    pub salience_score: i64,
    pub matched_terms: Vec<String>,
    pub matched_entity_ids: Vec<String>,
    pub expanded_entity_ids: Vec<String>,
    pub derivation_refs: Vec<String>,
    pub rationale: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MemoryRecallHit {
    pub object_id: String,
    pub version_id: String,
    pub scope: MemoryScope,
    pub kind: MemoryKind,
    pub trust_state: TrustState,
    pub summary: String,
    pub payload: Value,
    pub score: i64,
    pub explanation: MemoryRecallExplanation,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct MemoryRecallFeedbackSignal {
    pub useful: bool,
    pub cited_in_response: bool,
    pub corrected_user: bool,
    pub explicit_pin: bool,
}

fn tokenize_query(text: &str) -> Vec<String> {
    text.to_ascii_lowercase()
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|row| !row.is_empty())
        .map(str::to_string)
        .collect::<Vec<String>>()
}

fn summary_for(payload: &Value) -> String {
    let raw = match payload {
        Value::String(row) => row.clone(),
        Value::Object(map) => map
            .get("summary")
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| serde_json::to_string(payload).unwrap_or_default()),
        _ => serde_json::to_string(payload).unwrap_or_default(),
    };
    raw.split_whitespace()
        .take(24)
        .collect::<Vec<&str>>()
        .join(" ")
}

fn update_salience(base: &MemorySalience, signal: &MemoryRecallFeedbackSignal) -> MemorySalience {
    let mut next = base.clone();
    next.retrieval_hits = next.retrieval_hits.saturating_add(1);
    if signal.useful || signal.cited_in_response {
        next.reinforcement_count = next.reinforcement_count.saturating_add(1);
        next.score = next.score.saturating_add(45);
    }
    if signal.corrected_user {
        next.correction_count = next.correction_count.saturating_add(1);
        next.score = next.score.saturating_add(25);
    }
    if signal.explicit_pin {
        next.pinned = true;
        next.score = next.score.saturating_add(120);
    }
    next.last_accessed_ms = Some(now_ms());
    next
}

impl<P: MemoryPolicyGate + Clone> UnifiedMemoryHeap<P> {
    pub fn hybrid_recall(
        &self,
        principal_id: &str,
        capability: &CapabilityToken,
        query: MemoryRecallQuery,
    ) -> Result<Vec<MemoryRecallHit>, String> {
        let query_tokens = tokenize_query(&query.query);
        let vector_query = embed_text(&query.query, 64);
        let mut entity_matches = self
            .knowledge_graph
            .resolve_entities(&query.query)
            .into_iter()
            .map(|row| row.entity_id)
            .collect::<Vec<String>>();
        for hint in &query.session_entity_hints {
            entity_matches.push(hint.clone());
        }
        entity_matches.sort();
        entity_matches.dedup();
        let expanded = self
            .knowledge_graph
            .expand_related_entity_ids(entity_matches.as_slice(), 2, 16);
        let vector_hits = self.vector_index.query_cosine_filtered(
            vector_query.as_slice(),
            query.top_k.saturating_mul(4).max(8),
            &VectorQueryFilter {
                scope_labels: query.requested_scopes.iter().map(MemoryScope::label).collect(),
                kinds: query.allowed_kinds.clone(),
                ..VectorQueryFilter::default()
            },
        );
        let vector_map = vector_hits
            .iter()
            .map(|row| (row.key.clone(), (row.score * 1000.0).round() as i64))
            .collect::<BTreeMap<String, i64>>();
        let mut hits = Vec::<MemoryRecallHit>::new();
        for object in self.record_store.all_objects() {
            if !query.requested_scopes.is_empty()
                && !query.requested_scopes.iter().any(|scope| scope == &object.scope)
            {
                continue;
            }
            let decision = self.policy.evaluate(&MemoryPolicyRequest {
                principal_id: principal_id.to_string(),
                action: PolicyAction::Read,
                source_scope: object.scope.clone(),
                target_scope: None,
                trust_state: None,
                capability: Some(capability.clone()),
                owner_settings: self.owner_settings().clone(),
            });
            if !decision.allow {
                continue;
            }
            let Some(version_id) = self.record_store.head_version_id(&object.object_id) else {
                continue;
            };
            if self.version_ledger.is_inactive(&version_id) {
                continue;
            }
            let Some(version) = self.version_ledger.get(&version_id).cloned() else {
                continue;
            };
            if version.trust_state.is_poisoned() {
                continue;
            }
            if !query.allowed_kinds.is_empty() && !query.allowed_kinds.iter().any(|row| row == &version.kind) {
                continue;
            }
            let lexical = query_tokens
                .iter()
                .filter(|token| {
                    object.namespace.to_ascii_lowercase().contains(token.as_str())
                        || object.key.to_ascii_lowercase().contains(token.as_str())
                        || summary_for(&version.payload)
                            .to_ascii_lowercase()
                            .contains(token.as_str())
                })
                .count() as i64
                * 1000;
            let vector_score = *vector_map.get(&version.version_id).unwrap_or(&0);
            let metadata = self.vector_index.get_metadata(&version.version_id);
            let mut matched_entity_ids = Vec::<String>::new();
            let mut graph_score = 0i64;
            let mut session_score = 0i64;
            if let Some(meta) = metadata {
                for entity_ref in &meta.entity_refs {
                    if expanded.iter().any(|row| row == entity_ref) {
                        graph_score += 850;
                        matched_entity_ids.push(entity_ref.clone());
                    }
                    if query.session_entity_hints.iter().any(|row| row == entity_ref) {
                        session_score += 700;
                    }
                }
            }
            let salience_score = i64::from(version.salience.score);
            let total = lexical + vector_score + graph_score + session_score + salience_score;
            if total <= 0 {
                continue;
            }
            let mut rationale = Vec::<String>::new();
            if lexical > 0 {
                rationale.push("lexical_match".to_string());
            }
            if vector_score > 0 {
                rationale.push("vector_similarity".to_string());
            }
            if graph_score > 0 {
                rationale.push("graph_expansion".to_string());
            }
            if session_score > 0 {
                rationale.push("session_anchor".to_string());
            }
            if salience_score > 0 {
                rationale.push("salience_priority".to_string());
            }
            hits.push(MemoryRecallHit {
                object_id: object.object_id.clone(),
                version_id: version.version_id.clone(),
                scope: version.scope.clone(),
                kind: version.kind.clone(),
                trust_state: version.trust_state.clone(),
                summary: summary_for(&version.payload),
                payload: version.payload.clone(),
                score: total,
                explanation: MemoryRecallExplanation {
                    lexical_score: lexical,
                    vector_score,
                    graph_score,
                    session_score,
                    salience_score,
                    matched_terms: query_tokens
                        .iter()
                        .filter(|token| summary_for(&version.payload).to_ascii_lowercase().contains(token.as_str()))
                        .cloned()
                        .collect::<Vec<String>>(),
                    matched_entity_ids,
                    expanded_entity_ids: expanded.clone(),
                    derivation_refs: version
                        .derivation
                        .as_ref()
                        .map(|row| row.source_version_ids.clone())
                        .unwrap_or_default(),
                    rationale,
                },
            });
        }
        hits.sort_by(|a, b| {
            b.score
                .cmp(&a.score)
                .then_with(|| a.object_id.cmp(&b.object_id))
                .then_with(|| a.version_id.cmp(&b.version_id))
        });
        hits.truncate(query.top_k.max(1));
        Ok(hits)
    }

    pub fn record_retrieval_feedback(
        &mut self,
        route: &NexusRouteContext,
        principal_id: &str,
        capability: &CapabilityToken,
        version_id: &str,
        signal: MemoryRecallFeedbackSignal,
        lineage_refs: Vec<String>,
    ) -> Result<MemoryVersion, String> {
        self.ensure_routed(route)?;
        let source = self
            .version_ledger
            .get(version_id)
            .cloned()
            .ok_or_else(|| "feedback_source_missing".to_string())?;
        let object = self
            .record_store
            .get_object(&source.object_id)
            .cloned()
            .ok_or_else(|| "feedback_object_missing".to_string())?;
        let write_decision = self.policy.evaluate(&MemoryPolicyRequest {
            principal_id: principal_id.to_string(),
            action: PolicyAction::Write,
            source_scope: source.scope.clone(),
            target_scope: None,
            trust_state: Some(source.trust_state.clone()),
            capability: Some(capability.clone()),
            owner_settings: self.owner_settings().clone(),
        });
        if !write_decision.allow {
            return Err(format!("retrieval_feedback_denied:{}", write_decision.reason));
        }
        let receipt = self.push_receipt(
            route,
            "memory_retrieval_feedback",
            write_decision,
            lineage_refs.clone(),
            json!({
                "object_id": source.object_id,
                "version_id": source.version_id,
                "signal": signal,
            }),
        );
        let next = MemoryVersion {
            version_id: format!(
                "version_{}",
                &deterministic_hash(&(source.object_id.clone(), source.version_id.clone(), receipt.receipt_id.clone(), "feedback"))[..24]
            ),
            object_id: source.object_id.clone(),
            scope: source.scope.clone(),
            kind: source.kind.clone(),
            parent_version_id: Some(source.version_id.clone()),
            lineage_refs: {
                let mut out = lineage_refs;
                out.push(source.version_id.clone());
                out
            },
            receipt_id: receipt.receipt_id,
            trust_state: source.trust_state.clone(),
            salience: update_salience(&source.salience, &signal),
            derivation: Some(MemoryDerivation {
                kind: DerivationKind::RetrievalFeedback,
                source_version_ids: vec![source.version_id.clone()],
                notes: "retrieval feedback reinforced salience".to_string(),
                confidence_bps: 7000,
            }),
            payload: source.payload.clone(),
            payload_hash: source.payload_hash.clone(),
            timestamp_ms: now_ms(),
            proposed_by: principal_id.to_string(),
        };
        self.version_ledger.append(next.clone())?;
        self.record_store.register_version(&next.object_id, &next.version_id);
        self.record_store.set_head_version(&next.object_id, &next.version_id);
        self.refresh_memory_indexes(&object, &next);
        Ok(next)
    }

    pub fn invalidate_version(
        &mut self,
        route: &NexusRouteContext,
        principal_id: &str,
        capability: &CapabilityToken,
        version_id: &str,
        replacement_version_id: Option<String>,
        reason: MemoryInvalidationReason,
        details: Value,
        lineage_refs: Vec<String>,
    ) -> Result<MemoryInvalidationRecord, String> {
        self.ensure_routed(route)?;
        let target = self
            .version_ledger
            .get(version_id)
            .cloned()
            .ok_or_else(|| "invalidate_target_missing".to_string())?;
        let write_decision = self.policy.evaluate(&MemoryPolicyRequest {
            principal_id: principal_id.to_string(),
            action: PolicyAction::Write,
            source_scope: target.scope.clone(),
            target_scope: None,
            trust_state: Some(target.trust_state.clone()),
            capability: Some(capability.clone()),
            owner_settings: self.owner_settings().clone(),
        });
        if !write_decision.allow {
            return Err(format!("memory_invalidation_denied:{}", write_decision.reason));
        }
        let receipt = self.push_receipt(
            route,
            "memory_invalidation",
            write_decision,
            lineage_refs.clone(),
            json!({
                "target_version_id": target.version_id,
                "replacement_version_id": replacement_version_id,
                "reason": reason,
            }),
        );
        let record = MemoryInvalidationRecord {
            invalidation_id: format!(
                "invalid_{}",
                &deterministic_hash(&(target.version_id.clone(), receipt.receipt_id.clone(), now_ms()))[..24]
            ),
            target_version_id: target.version_id.clone(),
            target_object_id: target.object_id.clone(),
            replacement_version_id,
            reason,
            details,
            receipt_id: receipt.receipt_id,
            timestamp_ms: now_ms(),
            lineage_refs,
        };
        self.version_ledger.append_invalidation_record(record.clone())?;
        if self
            .record_store
            .head_version_id(&target.object_id)
            .as_deref()
            == Some(target.version_id.as_str())
        {
            if let Some(fallback) = self.version_ledger.latest_active_for_object(&target.object_id) {
                self.record_store
                    .set_head_version(&target.object_id, &fallback.version_id);
            }
        }
        Ok(record)
    }
}

#[cfg(test)]
#[path = "recall_tests.rs"]
mod tests;
