use crate::{deterministic_hash, now_ms};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub const TASK_FABRIC_NAMESPACE: &str = "task_fabric";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GraphNode {
    pub node_id: String,
    pub namespace: String,
    pub payload: Value,
    pub cas_version: u64,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GraphEdge {
    pub edge_id: String,
    pub source_node_id: String,
    pub target_node_id: String,
    pub edge_type: String,
    pub created_at_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskFabricLease {
    pub lease_id: String,
    pub node_id: String,
    pub holder_principal: String,
    pub issued_at_ms: u64,
    pub expires_at_ms: u64,
    pub cas_snapshot: u64,
}

#[derive(Debug, Clone, Default)]
pub struct GraphSubsystem {
    nodes: BTreeMap<String, GraphNode>,
    edges: Vec<GraphEdge>,
    leases: BTreeMap<String, TaskFabricLease>,
}

fn lease_id(node_id: &str, holder_principal: &str, now: u64, ttl_ms: u64) -> String {
    format!(
        "lease_{}",
        &deterministic_hash(&(
            node_id.to_string(),
            holder_principal.to_string(),
            now,
            ttl_ms
        ))[..24]
    )
}

fn edge_id(source_node_id: &str, target_node_id: &str, edge_type: &str, now: u64) -> String {
    format!(
        "edge_{}",
        &deterministic_hash(&(
            source_node_id.to_string(),
            target_node_id.to_string(),
            edge_type.to_string(),
            now
        ))[..24]
    )
}

impl GraphSubsystem {
    pub fn create_task_node(&mut self, node_id: impl Into<String>, payload: Value) -> GraphNode {
        let now = now_ms();
        let node = GraphNode {
            node_id: node_id.into(),
            namespace: TASK_FABRIC_NAMESPACE.to_string(),
            payload,
            cas_version: 0,
            created_at_ms: now,
            updated_at_ms: now,
        };
        self.nodes.insert(node.node_id.clone(), node.clone());
        node
    }

    pub fn get_node(&self, node_id: &str) -> Option<&GraphNode> {
        self.nodes.get(node_id)
    }

    pub fn issue_lease(
        &mut self,
        node_id: &str,
        holder_principal: &str,
        ttl_ms: u64,
    ) -> Result<TaskFabricLease, String> {
        let node = self
            .nodes
            .get(node_id)
            .ok_or_else(|| "task_node_not_found".to_string())?;
        let now = now_ms();
        let lease = TaskFabricLease {
            lease_id: lease_id(node_id, holder_principal, now, ttl_ms),
            node_id: node_id.to_string(),
            holder_principal: holder_principal.to_string(),
            issued_at_ms: now,
            expires_at_ms: now.saturating_add(ttl_ms.max(1)),
            cas_snapshot: node.cas_version,
        };
        self.leases.insert(lease.lease_id.clone(), lease.clone());
        Ok(lease)
    }

    pub fn mutate_task_node(
        &mut self,
        node_id: &str,
        lease_id: &str,
        holder_principal: &str,
        expected_cas: u64,
        payload: Value,
    ) -> Result<GraphNode, String> {
        self.validate_lease(node_id, lease_id, holder_principal)?;
        let now = now_ms();
        let node = self
            .nodes
            .get_mut(node_id)
            .ok_or_else(|| "task_node_not_found".to_string())?;
        if node.cas_version != expected_cas {
            return Err("task_cas_mismatch".to_string());
        }
        node.payload = payload;
        node.cas_version = node.cas_version.saturating_add(1);
        node.updated_at_ms = now;
        Ok(node.clone())
    }

    pub fn add_edge(
        &mut self,
        source_node_id: &str,
        target_node_id: &str,
        lease_id: &str,
        holder_principal: &str,
        expected_source_cas: u64,
        edge_type: &str,
    ) -> Result<GraphEdge, String> {
        self.validate_lease(source_node_id, lease_id, holder_principal)?;
        let source = self
            .nodes
            .get(source_node_id)
            .ok_or_else(|| "source_task_node_not_found".to_string())?;
        if source.cas_version != expected_source_cas {
            return Err("task_cas_mismatch".to_string());
        }
        if !self.nodes.contains_key(target_node_id) {
            return Err("target_task_node_not_found".to_string());
        }
        let now = now_ms();
        let edge = GraphEdge {
            edge_id: edge_id(source_node_id, target_node_id, edge_type, now),
            source_node_id: source_node_id.to_string(),
            target_node_id: target_node_id.to_string(),
            edge_type: edge_type.to_string(),
            created_at_ms: now,
        };
        self.edges.push(edge.clone());
        Ok(edge)
    }

    pub fn edges(&self) -> &[GraphEdge] {
        self.edges.as_slice()
    }

    fn validate_lease(
        &self,
        node_id: &str,
        lease_id: &str,
        holder_principal: &str,
    ) -> Result<(), String> {
        let lease = self
            .leases
            .get(lease_id)
            .ok_or_else(|| "task_lease_not_found".to_string())?;
        if lease.node_id != node_id {
            return Err("task_lease_node_mismatch".to_string());
        }
        if lease.holder_principal != holder_principal {
            return Err("task_lease_holder_mismatch".to_string());
        }
        if now_ms() > lease.expires_at_ms {
            return Err("task_lease_expired".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeEntityKind {
    Person,
    Project,
    System,
    Incident,
    Preference,
    Concept,
    Procedure,
    Session,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeRelationKind {
    MentionedWith,
    DependsOn,
    Owns,
    Prefers,
    AffectedBy,
    StepOf,
    RefersTo,
    Supports,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KnowledgeGraphNode {
    pub entity_id: String,
    pub kind: KnowledgeEntityKind,
    pub label: String,
    pub aliases: Vec<String>,
    pub evidence_version_ids: Vec<String>,
    pub salience_hint: u32,
    pub metadata: Value,
    pub updated_at_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KnowledgeGraphEdge {
    pub edge_id: String,
    pub source_entity_id: String,
    pub target_entity_id: String,
    pub relation: KnowledgeRelationKind,
    pub weight_bps: u16,
    pub evidence_version_ids: Vec<String>,
    pub updated_at_ms: u64,
}

#[derive(Debug, Clone, Default)]
pub struct KnowledgeGraph {
    nodes: BTreeMap<String, KnowledgeGraphNode>,
    edges: Vec<KnowledgeGraphEdge>,
}

fn knowledge_edge_id(source: &str, target: &str, relation: &KnowledgeRelationKind) -> String {
    format!(
        "kedge_{}",
        &deterministic_hash(&(source.to_string(), target.to_string(), relation))[..24]
    )
}

fn normalize_aliases(label: &str, aliases: &[String]) -> Vec<String> {
    let mut out = BTreeSet::new();
    out.insert(label.trim().to_ascii_lowercase());
    for alias in aliases {
        let cleaned = alias.trim().to_ascii_lowercase();
        if !cleaned.is_empty() {
            out.insert(cleaned);
        }
    }
    out.into_iter().collect::<Vec<String>>()
}

fn tokenize_query(text: &str) -> Vec<String> {
    text.to_ascii_lowercase()
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|row| !row.is_empty())
        .map(str::to_string)
        .collect::<Vec<String>>()
}

impl KnowledgeGraph {
    pub fn upsert_entity(
        &mut self,
        entity_id: impl Into<String>,
        kind: KnowledgeEntityKind,
        label: impl Into<String>,
        aliases: Vec<String>,
        evidence_version_ids: Vec<String>,
        salience_hint: u32,
        metadata: Value,
    ) -> KnowledgeGraphNode {
        let entity_id = entity_id.into();
        let label = label.into();
        let now = now_ms();
        let aliases = normalize_aliases(&label, aliases.as_slice());
        let node = self
            .nodes
            .entry(entity_id.clone())
            .or_insert_with(|| KnowledgeGraphNode {
                entity_id: entity_id.clone(),
                kind: kind.clone(),
                label: label.clone(),
                aliases: aliases.clone(),
                evidence_version_ids: Vec::new(),
                salience_hint,
                metadata: metadata.clone(),
                updated_at_ms: now,
            });
        node.kind = kind;
        node.label = label;
        node.aliases = aliases;
        node.salience_hint = node.salience_hint.max(salience_hint);
        node.metadata = metadata;
        node.updated_at_ms = now;
        for version_id in evidence_version_ids {
            if !node
                .evidence_version_ids
                .iter()
                .any(|row| row == &version_id)
            {
                node.evidence_version_ids.push(version_id);
            }
        }
        node.clone()
    }

    pub fn connect(
        &mut self,
        source_entity_id: &str,
        target_entity_id: &str,
        relation: KnowledgeRelationKind,
        evidence_version_ids: Vec<String>,
        weight_bps: u16,
    ) -> Result<KnowledgeGraphEdge, String> {
        if !self.nodes.contains_key(source_entity_id) {
            return Err("knowledge_source_missing".to_string());
        }
        if !self.nodes.contains_key(target_entity_id) {
            return Err("knowledge_target_missing".to_string());
        }
        let edge_id = knowledge_edge_id(source_entity_id, target_entity_id, &relation);
        let now = now_ms();
        if let Some(existing) = self.edges.iter_mut().find(|edge| edge.edge_id == edge_id) {
            existing.weight_bps = existing.weight_bps.max(weight_bps);
            existing.updated_at_ms = now;
            for version_id in evidence_version_ids {
                if !existing
                    .evidence_version_ids
                    .iter()
                    .any(|row| row == &version_id)
                {
                    existing.evidence_version_ids.push(version_id);
                }
            }
            return Ok(existing.clone());
        }
        let edge = KnowledgeGraphEdge {
            edge_id,
            source_entity_id: source_entity_id.to_string(),
            target_entity_id: target_entity_id.to_string(),
            relation,
            weight_bps: weight_bps.max(1),
            evidence_version_ids,
            updated_at_ms: now,
        };
        self.edges.push(edge.clone());
        Ok(edge)
    }

    pub fn get_entity(&self, entity_id: &str) -> Option<&KnowledgeGraphNode> {
        self.nodes.get(entity_id)
    }

    pub fn nodes(&self) -> Vec<KnowledgeGraphNode> {
        self.nodes.values().cloned().collect::<Vec<_>>()
    }

    pub fn edges(&self) -> &[KnowledgeGraphEdge] {
        self.edges.as_slice()
    }

    pub fn resolve_entities(&self, query: &str) -> Vec<KnowledgeGraphNode> {
        let query_tokens = tokenize_query(query);
        let mut scored = self
            .nodes
            .values()
            .filter_map(|node| {
                let score = node
                    .aliases
                    .iter()
                    .map(|alias| {
                        query_tokens
                            .iter()
                            .filter(|token| alias.contains(token.as_str()))
                            .count() as i64
                    })
                    .sum::<i64>();
                if score <= 0 {
                    return None;
                }
                Some((score, node.clone()))
            })
            .collect::<Vec<(i64, KnowledgeGraphNode)>>();
        scored.sort_by(|a, b| {
            b.0.cmp(&a.0)
                .then_with(|| a.1.entity_id.cmp(&b.1.entity_id))
        });
        scored
            .into_iter()
            .take(8)
            .map(|(_, node)| node)
            .collect::<Vec<_>>()
    }

    pub fn expand_related_entity_ids(
        &self,
        seed_entity_ids: &[String],
        max_depth: usize,
        max_entities: usize,
    ) -> Vec<String> {
        let mut seen = BTreeSet::new();
        let mut queue = VecDeque::new();
        for entity_id in seed_entity_ids {
            if seen.insert(entity_id.clone()) {
                queue.push_back((entity_id.clone(), 0usize));
            }
        }
        while let Some((entity_id, depth)) = queue.pop_front() {
            if depth >= max_depth || seen.len() >= max_entities.max(1) {
                continue;
            }
            for edge in self.edges.iter().filter(|edge| {
                edge.source_entity_id == entity_id || edge.target_entity_id == entity_id
            }) {
                let next = if edge.source_entity_id == entity_id {
                    edge.target_entity_id.clone()
                } else {
                    edge.source_entity_id.clone()
                };
                if seen.insert(next.clone()) {
                    queue.push_back((next, depth + 1));
                }
            }
        }
        seen.into_iter().collect::<Vec<String>>()
    }

    pub fn evidence_path_for(&self, entity_id: &str) -> Vec<String> {
        let mut out = Vec::new();
        if let Some(node) = self.nodes.get(entity_id) {
            out.extend(node.evidence_version_ids.clone());
        }
        for edge in self
            .edges
            .iter()
            .filter(|edge| edge.source_entity_id == entity_id || edge.target_entity_id == entity_id)
        {
            out.extend(edge.evidence_version_ids.clone());
        }
        let mut deduped = BTreeSet::new();
        for row in out {
            deduped.insert(row);
        }
        deduped.into_iter().collect::<Vec<String>>()
    }
}

#[cfg(test)]
#[path = "graph_subsystem_tests.rs"]
mod knowledge_tests;
