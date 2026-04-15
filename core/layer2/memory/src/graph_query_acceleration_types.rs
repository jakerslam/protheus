use crate::graph_subsystem::{KnowledgeGraphEdge, KnowledgeRelationKind};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GraphQueryTerm {
    Node(String),
    Var(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KnowledgeTriplePattern {
    pub subject: GraphQueryTerm,
    pub relation: Option<KnowledgeRelationKind>,
    pub object: GraphQueryTerm,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GraphQueryPlanStep {
    pub pattern: KnowledgeTriplePattern,
    pub estimated_cardinality: usize,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct GraphQueryPlan {
    pub steps: Vec<GraphQueryPlanStep>,
    pub strategy: String,
    pub used_cache_seed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct GraphQueryResult {
    pub bindings: Vec<BTreeMap<String, String>>,
    pub matched_edge_ids: Vec<String>,
    pub plan: GraphQueryPlan,
    pub cache_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GraphTraversalAlgorithm {
    Bfs,
    Dfs,
    Dijkstra,
    AStar,
    Bidirectional,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GraphPathQuery {
    pub start_entity_id: String,
    pub target_entity_id: String,
    pub algorithm: GraphTraversalAlgorithm,
    pub max_depth: usize,
    pub relation_filter: Vec<KnowledgeRelationKind>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GraphPathResult {
    pub path_entity_ids: Vec<String>,
    pub explored_nodes: usize,
    pub total_cost_milli: u64,
    pub algorithm: GraphTraversalAlgorithm,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GraphSamplingStrategy {
    RandomWalk,
    ForestFire,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GraphSamplingQuery {
    pub strategy: GraphSamplingStrategy,
    pub seed_entity_id: String,
    pub max_nodes: usize,
    pub spread_bps: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct GraphSample {
    pub sampled_entity_ids: Vec<String>,
    pub sampled_edges: Vec<KnowledgeGraphEdge>,
    pub approximate: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct NeighborhoodSummary {
    pub entity_id: String,
    pub total_neighbors: usize,
    pub relation_counts: BTreeMap<String, usize>,
    pub neighbor_kind_counts: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EntityEmbeddingHit {
    pub entity_id: String,
    pub score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GraphPartitionStrategy {
    Hash,
    Community,
    Predicate,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct GraphPartitionPlan {
    pub strategy: String,
    pub partitions: BTreeMap<usize, Vec<String>>,
    pub cut_edges: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FederatedServiceProfile {
    pub service_id: String,
    pub supported_relations: Vec<KnowledgeRelationKind>,
    pub selectivity_hint_bps: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FederatedDispatchStep {
    pub service_id: String,
    pub pattern_indexes: Vec<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct FederatedQueryPlan {
    pub ordered_services: Vec<String>,
    pub dispatch_steps: Vec<FederatedDispatchStep>,
}
