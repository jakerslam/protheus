use crate::graph_subsystem::{KnowledgeEntityKind, KnowledgeRelationKind};
use crate::heap_interface::{NexusRouteContext, UnifiedMemoryHeap};
use crate::policy::{MemoryPolicyGate, PolicyAction};
use crate::schemas::{
    CapabilityAction, CapabilityToken, Classification, DerivationKind, MemoryDerivation,
    MemoryKind, MemoryObject, MemoryScope, MemoryVersion, TrustState,
};
use crate::{deterministic_hash, now_ms};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConsolidatedMemoryDraft {
    pub object: MemoryObject,
    pub trust_state: TrustState,
    pub derivation: MemoryDerivation,
    pub payload: Value,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConsolidationReport {
    pub scanned_versions: usize,
    pub derived_semantic: usize,
    pub derived_procedural: usize,
    pub entity_nodes_upserted: usize,
    pub relation_edges_upserted: usize,
    pub written_versions: usize,
}

#[derive(Debug, Clone, Default)]
pub struct ExtractedGraphFacts {
    pub entity_refs: Vec<String>,
    pub entities: Vec<(String, KnowledgeEntityKind, String)>,
    pub relations: Vec<(String, String, KnowledgeRelationKind)>,
}

fn normalized_string(raw: &str) -> String {
    raw.trim().to_ascii_lowercase().replace(' ', "_")
}

fn clean_text(value: &Value) -> String {
    match value {
        Value::String(row) => row.clone(),
        _ => serde_json::to_string(value).unwrap_or_default(),
    }
}

fn parse_kind_hint(key: &str) -> KnowledgeEntityKind {
    match key {
        "person" | "user" | "owner" => KnowledgeEntityKind::Person,
        "project" => KnowledgeEntityKind::Project,
        "system" | "service" => KnowledgeEntityKind::System,
        "incident" => KnowledgeEntityKind::Incident,
        "preference" | "preference_key" => KnowledgeEntityKind::Preference,
        "procedure" | "procedure_name" => KnowledgeEntityKind::Procedure,
        "concept" | "subject" => KnowledgeEntityKind::Concept,
        "session" => KnowledgeEntityKind::Session,
        _ => KnowledgeEntityKind::Unknown,
    }
}

pub fn extract_graph_facts(
    version: &MemoryVersion,
    payload: &Value,
    metadata: &Value,
) -> ExtractedGraphFacts {
    let mut out = ExtractedGraphFacts::default();
    let mut entity_rows = Vec::<(String, KnowledgeEntityKind, String)>::new();
    let object_rows = [payload, metadata];
    for row in object_rows {
        if let Some(map) = row.as_object() {
            for (key, value) in map {
                if let Some(text) = value.as_str() {
                    let cleaned = text.trim();
                    if cleaned.is_empty() {
                        continue;
                    }
                    let kind = parse_kind_hint(key.as_str());
                    if matches!(kind, KnowledgeEntityKind::Unknown) {
                        continue;
                    }
                    let entity_id = format!("{}:{}", key, normalized_string(cleaned));
                    entity_rows.push((entity_id, kind, cleaned.to_string()));
                } else if key == "entity_refs" {
                    if let Some(values) = value.as_array() {
                        for item in values.iter().filter_map(Value::as_str) {
                            let parts = item.splitn(2, ':').collect::<Vec<&str>>();
                            if parts.len() == 2 {
                                entity_rows.push((
                                    item.to_string(),
                                    parse_kind_hint(parts[0]),
                                    parts[1].replace('_', " "),
                                ));
                            }
                        }
                    }
                }
            }
        }
    }
    let mut deduped = BTreeMap::new();
    for (entity_id, kind, label) in entity_rows {
        deduped.entry(entity_id.clone()).or_insert((kind, label));
    }
    for (entity_id, (kind, label)) in deduped {
        out.entity_refs.push(entity_id.clone());
        out.entities.push((entity_id, kind, label));
    }
    out.entity_refs.sort();
    for window in out.entity_refs.windows(2) {
        if let [left, right] = window {
            out.relations.push((
                left.clone(),
                right.clone(),
                KnowledgeRelationKind::MentionedWith,
            ));
        }
    }
    if let Some(map) = payload.as_object() {
        if let (Some(project), Some(system)) = (
            map.get("project").and_then(Value::as_str),
            map.get("system").and_then(Value::as_str),
        ) {
            out.relations.push((
                format!("project:{}", normalized_string(project)),
                format!("system:{}", normalized_string(system)),
                KnowledgeRelationKind::DependsOn,
            ));
        }
        if let (Some(person), Some(project)) = (
            map.get("person").and_then(Value::as_str),
            map.get("project").and_then(Value::as_str),
        ) {
            out.relations.push((
                format!("person:{}", normalized_string(person)),
                format!("project:{}", normalized_string(project)),
                KnowledgeRelationKind::Owns,
            ));
        }
        if let (Some(system), Some(incident)) = (
            map.get("system").and_then(Value::as_str),
            map.get("incident").and_then(Value::as_str),
        ) {
            out.relations.push((
                format!("system:{}", normalized_string(system)),
                format!("incident:{}", normalized_string(incident)),
                KnowledgeRelationKind::AffectedBy,
            ));
        }
        if let (Some(subject), Some(preference_key), Some(preference_value)) = (
            map.get("person")
                .or_else(|| map.get("user"))
                .and_then(Value::as_str),
            map.get("preference_key").and_then(Value::as_str),
            map.get("preference_value").and_then(Value::as_str),
        ) {
            out.relations.push((
                format!("person:{}", normalized_string(subject)),
                format!(
                    "preference:{}={}",
                    normalized_string(preference_key),
                    normalized_string(preference_value)
                ),
                KnowledgeRelationKind::Prefers,
            ));
        }
    }
    let _ = version;
    out
}

fn build_semantic_draft(
    scope: &MemoryScope,
    key: &str,
    value: &str,
    source_versions: &[MemoryVersion],
) -> ConsolidatedMemoryDraft {
    let object = MemoryObject {
        object_id: format!(
            "object_{}",
            &deterministic_hash(&(scope.label(), "semantic", key, value))[..24]
        ),
        scope: scope.clone(),
        kind: MemoryKind::Semantic,
        classification: Classification::Internal,
        namespace: "memory.consolidated.semantic".to_string(),
        key: format!("{}={}", key, value),
        payload: json!({
            "subject": source_versions
                .first()
                .and_then(|row| row.payload.get("person").or_else(|| row.payload.get("user")))
                .cloned()
                .unwrap_or(Value::Null),
            "preference_key": key,
            "preference_value": value,
            "support_count": source_versions.len(),
        }),
        metadata: json!({
            "consolidated_from": "episodic",
            "support_version_ids": source_versions.iter().map(|row| row.version_id.clone()).collect::<Vec<String>>(),
        }),
        created_at_ms: now_ms(),
        updated_at_ms: now_ms(),
    };
    ConsolidatedMemoryDraft {
        object,
        trust_state: TrustState::Validated,
        derivation: MemoryDerivation {
            kind: DerivationKind::ConsolidatedSemantic,
            source_version_ids: source_versions
                .iter()
                .map(|row| row.version_id.clone())
                .collect::<Vec<String>>(),
            notes: "repeated episodic preference consolidated into semantic memory".to_string(),
            confidence_bps: (6000 + (source_versions.len() as u16 * 750)).min(9800),
        },
        payload: json!({}),
    }
}

fn build_procedural_draft(
    scope: &MemoryScope,
    procedure_name: &str,
    ordered_steps: &[(u64, String)],
    source_versions: &[MemoryVersion],
) -> ConsolidatedMemoryDraft {
    let object = MemoryObject {
        object_id: format!(
            "object_{}",
            &deterministic_hash(&(scope.label(), "procedural", procedure_name))[..24]
        ),
        scope: scope.clone(),
        kind: MemoryKind::Procedural,
        classification: Classification::Internal,
        namespace: "memory.consolidated.procedural".to_string(),
        key: normalized_string(procedure_name),
        payload: json!({
            "procedure_name": procedure_name,
            "steps": ordered_steps.iter().map(|(_, step)| step.clone()).collect::<Vec<String>>(),
            "support_count": source_versions.len(),
        }),
        metadata: json!({
            "consolidated_from": "episodic",
            "support_version_ids": source_versions.iter().map(|row| row.version_id.clone()).collect::<Vec<String>>(),
        }),
        created_at_ms: now_ms(),
        updated_at_ms: now_ms(),
    };
    ConsolidatedMemoryDraft {
        object,
        trust_state: TrustState::Validated,
        derivation: MemoryDerivation {
            kind: DerivationKind::ConsolidatedProcedural,
            source_version_ids: source_versions
                .iter()
                .map(|row| row.version_id.clone())
                .collect::<Vec<String>>(),
            notes: "ordered episodic procedure steps consolidated into procedural memory"
                .to_string(),
            confidence_bps: (6500 + (source_versions.len() as u16 * 600)).min(9800),
        },
        payload: json!({}),
    }
}

impl<P: MemoryPolicyGate + Clone> UnifiedMemoryHeap<P> {
    pub(crate) fn refresh_memory_indexes(
        &mut self,
        object: &MemoryObject,
        version: &MemoryVersion,
    ) {
        let facts = extract_graph_facts(version, &version.payload, &object.metadata);
        let summary_text = format!(
            "{} {} {} {}",
            object.namespace,
            object.key,
            clean_text(&version.payload),
            clean_text(&object.metadata)
        );
        self.vector_index.upsert(
            version.version_id.clone(),
            crate::vector_index::embed_text(&summary_text, 64),
            crate::vector_index::VectorMetadata {
                scope_label: version.scope.label(),
                kind: version.kind.clone(),
                trust_state: version.trust_state.clone(),
                namespace: object.namespace.clone(),
                entity_refs: facts.entity_refs.clone(),
                source_object_id: object.object_id.clone(),
                source_version_id: version.version_id.clone(),
            },
        );
        for (entity_id, kind, label) in facts.entities {
            let _ = self.knowledge_graph.upsert_entity(
                entity_id,
                kind,
                label,
                Vec::new(),
                vec![version.version_id.clone()],
                version.salience.score,
                json!({
                    "object_id": object.object_id,
                    "version_id": version.version_id,
                }),
            );
        }
        for (source, target, relation) in facts.relations {
            let _ = self.knowledge_graph.connect(
                source.as_str(),
                target.as_str(),
                relation,
                vec![version.version_id.clone()],
                7000,
            );
        }
    }

    pub fn run_consolidation_cycle(
        &mut self,
        route: &NexusRouteContext,
        principal_id: &str,
        capability: &CapabilityToken,
        requested_scopes: Vec<MemoryScope>,
        lineage_refs: Vec<String>,
    ) -> Result<ConsolidationReport, String> {
        self.ensure_routed(route)?;
        let mut report = ConsolidationReport::default();
        let active_versions = self
            .version_ledger
            .all_versions()
            .into_iter()
            .filter(|row| !self.version_ledger.is_inactive(row.version_id.as_str()))
            .filter(|row| !row.trust_state.is_poisoned())
            .filter(|row| {
                requested_scopes.is_empty()
                    || requested_scopes.iter().any(|scope| scope == &row.scope)
            })
            .collect::<Vec<MemoryVersion>>();
        report.scanned_versions = active_versions.len();

        let entity_count_before = self.knowledge_graph.nodes().len();
        let edge_count_before = self.knowledge_graph.edges().len();
        for version in &active_versions {
            if let Some(object) = self.record_store.get_object(&version.object_id).cloned() {
                self.refresh_memory_indexes(&object, version);
            }
        }

        let mut preference_groups = BTreeMap::<(String, String, String), Vec<MemoryVersion>>::new();
        let mut procedure_groups = BTreeMap::<(String, String), Vec<MemoryVersion>>::new();
        for version in active_versions
            .into_iter()
            .filter(|row| matches!(row.kind, MemoryKind::Episodic | MemoryKind::Working))
        {
            if let Some(map) = version.payload.as_object() {
                if let (Some(key), Some(value)) = (
                    map.get("preference_key").and_then(Value::as_str),
                    map.get("preference_value").and_then(Value::as_str),
                ) {
                    preference_groups
                        .entry((version.scope.label(), key.to_string(), value.to_string()))
                        .or_default()
                        .push(version.clone());
                }
                if let Some(name) = map.get("procedure_name").and_then(Value::as_str) {
                    procedure_groups
                        .entry((version.scope.label(), name.to_string()))
                        .or_default()
                        .push(version.clone());
                }
            }
        }

        let mut drafts = Vec::<ConsolidatedMemoryDraft>::new();
        for ((scope_label, key, value), versions) in preference_groups {
            if versions.len() < 2 {
                continue;
            }
            let scope = versions
                .first()
                .map(|row| row.scope.clone())
                .unwrap_or_else(|| MemoryScope::Core);
            let _ = scope_label;
            drafts.push(build_semantic_draft(
                &scope,
                &key,
                &value,
                versions.as_slice(),
            ));
        }
        for ((_scope_label, name), versions) in procedure_groups {
            let mut ordered = versions
                .iter()
                .filter_map(|row| {
                    let map = row.payload.as_object()?;
                    let step = map.get("procedure_step").and_then(Value::as_str)?;
                    let idx = map.get("step_index").and_then(Value::as_u64).unwrap_or(999);
                    Some((idx, step.to_string()))
                })
                .collect::<Vec<(u64, String)>>();
            ordered.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
            let mut seen_steps = BTreeSet::new();
            ordered.retain(|(_, step)| seen_steps.insert(step.clone()));
            if ordered.len() < 2 {
                continue;
            }
            let scope = versions
                .first()
                .map(|row| row.scope.clone())
                .unwrap_or_else(|| MemoryScope::Core);
            drafts.push(build_procedural_draft(
                &scope,
                &name,
                ordered.as_slice(),
                versions.as_slice(),
            ));
        }

        let mut written = 0usize;
        for draft in drafts {
            let mut object = draft.object.clone();
            object.payload = match draft.object.kind {
                MemoryKind::Semantic | MemoryKind::Procedural => draft.object.payload.clone(),
                _ => draft.payload.clone(),
            };
            self.write_memory_object(
                route,
                principal_id,
                capability,
                object,
                draft.trust_state,
                lineage_refs.clone(),
            )?;
            if let Some(head_id) = self.record_store.head_version_id(&draft.object.object_id) {
                if let Some(version) = self.version_ledger.get(head_id.as_str()).cloned() {
                    if let Some(version_mut) = self.version_ledger.get_mut(head_id.as_str()) {
                        version_mut.derivation = Some(draft.derivation.clone());
                    }
                    if let Some(object) = self.record_store.get_object(&version.object_id).cloned()
                    {
                        self.refresh_memory_indexes(&object, &version);
                    }
                }
            }
            written = written.saturating_add(1);
            match draft.object.kind {
                MemoryKind::Semantic => {
                    report.derived_semantic = report.derived_semantic.saturating_add(1)
                }
                MemoryKind::Procedural => {
                    report.derived_procedural = report.derived_procedural.saturating_add(1)
                }
                _ => {}
            }
        }
        report.written_versions = written;
        report.entity_nodes_upserted = self
            .knowledge_graph
            .nodes()
            .len()
            .saturating_sub(entity_count_before);
        report.relation_edges_upserted = self
            .knowledge_graph
            .edges()
            .len()
            .saturating_sub(edge_count_before);
        let _ = self.policy.evaluate(&crate::policy::MemoryPolicyRequest {
            principal_id: principal_id.to_string(),
            action: PolicyAction::Write,
            source_scope: MemoryScope::Core,
            target_scope: None,
            trust_state: Some(TrustState::Validated),
            capability: Some(capability.clone()),
            owner_settings: self.owner_settings().clone(),
        });
        let _ = CapabilityAction::Write;
        Ok(report)
    }
}

#[cfg(test)]
#[path = "consolidation_tests.rs"]
mod tests;
