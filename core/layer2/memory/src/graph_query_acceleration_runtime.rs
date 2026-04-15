use crate::deterministic_hash;
use crate::vector_index::embed_text;
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

use super::graph_query_acceleration_query::pattern_fingerprint;
use super::graph_query_acceleration_state::relation_label;
use super::graph_query_acceleration_types::{
    EntityEmbeddingHit, FederatedDispatchStep, FederatedQueryPlan, FederatedServiceProfile,
    GraphPartitionPlan, GraphPartitionStrategy, GraphQueryPlan, GraphQueryPlanStep,
    GraphQueryResult, GraphSample, GraphSamplingQuery, GraphSamplingStrategy,
    KnowledgeTriplePattern, NeighborhoodSummary,
};
use super::{KnowledgeGraph, KnowledgeGraphEdge, KnowledgeRelationKind};

const GRAPH_QUERY_CACHE_TTL_MS: u64 = 5 * 60 * 1000;

impl KnowledgeGraph {
    pub fn plan_triple_query(&self, patterns: &[KnowledgeTriplePattern]) -> GraphQueryPlan {
        let mut steps = patterns
            .iter()
            .cloned()
            .map(|pattern| GraphQueryPlanStep {
                estimated_cardinality: self.estimate_pattern_cardinality(&pattern),
                reasons: vec!["selectivity_first".to_string()],
                pattern,
            })
            .collect::<Vec<GraphQueryPlanStep>>();
        steps.sort_by(|a, b| {
            a.estimated_cardinality
                .cmp(&b.estimated_cardinality)
                .then_with(|| pattern_fingerprint(&a.pattern).cmp(&pattern_fingerprint(&b.pattern)))
        });
        GraphQueryPlan {
            steps,
            strategy: "selectivity_first_with_leapfrog_domains".to_string(),
            used_cache_seed: false,
        }
    }

    pub fn execute_triple_query(
        &mut self,
        patterns: Vec<KnowledgeTriplePattern>,
        max_results: usize,
    ) -> GraphQueryResult {
        if patterns.is_empty() {
            return GraphQueryResult::default();
        }
        let fingerprints = patterns
            .iter()
            .map(pattern_fingerprint)
            .collect::<BTreeSet<String>>();
        let cache_key = format!("graph_query_{}", deterministic_hash(&fingerprints));
        if let Some(cached) = self.acceleration.cache_get_fresh(cache_key.as_str()) {
            return GraphQueryResult {
                bindings: cached.bindings.clone(),
                matched_edge_ids: cached.matched_edge_ids.clone(),
                plan: self.plan_triple_query(&patterns),
                cache_status: "exact_hit".to_string(),
            };
        }
        let mut plan = self.plan_triple_query(&patterns);
        let mut bindings = vec![BTreeMap::<String, String>::new()];
        let mut edge_ids = BTreeSet::<String>::new();
        let mut active_steps = plan.steps.clone();
        if let Some(seed) = self.acceleration.cache_get_seed(&fingerprints) {
            plan.used_cache_seed = true;
            bindings = seed.bindings.clone();
            let seeded_patterns = seed.pattern_fingerprints.clone();
            active_steps
                .retain(|step| !seeded_patterns.contains(&pattern_fingerprint(&step.pattern)));
        }
        let variable_domains = self.build_variable_domains(&patterns);
        for step in active_steps {
            let mut next = Vec::<BTreeMap<String, String>>::new();
            for binding in &bindings {
                for (candidate, edge_id) in
                    self.match_pattern(&step.pattern, binding, &variable_domains)
                {
                    edge_ids.insert(edge_id);
                    next.push(candidate);
                    if next.len() >= max_results.max(1) {
                        break;
                    }
                }
                if next.len() >= max_results.max(1) {
                    break;
                }
            }
            bindings = next;
            if bindings.is_empty() {
                break;
            }
        }
        self.acceleration.cache_put(
            cache_key,
            fingerprints,
            bindings.clone(),
            edge_ids.iter().cloned().collect::<Vec<String>>(),
            GRAPH_QUERY_CACHE_TTL_MS,
        );
        let cache_status = if plan.used_cache_seed {
            "seeded_partial".to_string()
        } else {
            "miss".to_string()
        };
        GraphQueryResult {
            bindings,
            matched_edge_ids: edge_ids.into_iter().collect::<Vec<String>>(),
            plan,
            cache_status,
        }
    }

    pub fn relation_exists_probabilistic(
        &self,
        source_entity_id: &str,
        relation: KnowledgeRelationKind,
    ) -> bool {
        self.acceleration
            .relation_source_might_exist(relation_label(&relation), source_entity_id)
    }

    pub fn filter_entities_with_all_relations(
        &self,
        relations: &[KnowledgeRelationKind],
    ) -> Vec<String> {
        let relation_labels = relations
            .iter()
            .map(relation_label)
            .map(str::to_string)
            .collect::<Vec<String>>();
        self.acceleration
            .relation_bitmap_and(&relation_labels)
            .into_iter()
            .collect::<Vec<String>>()
    }

    pub fn materialize_transitive_closure(
        &mut self,
        relation: KnowledgeRelationKind,
    ) -> BTreeMap<String, Vec<String>> {
        let relation_key = relation_label(&relation).to_string();
        if let Some(cached) = self.acceleration.materialized_transitive.get(&relation_key) {
            return cached
                .iter()
                .map(|(source, rows)| {
                    (
                        source.clone(),
                        rows.iter().cloned().collect::<Vec<String>>(),
                    )
                })
                .collect::<BTreeMap<String, Vec<String>>>();
        }
        let mut out = BTreeMap::<String, BTreeSet<String>>::new();
        for source in self.nodes.keys() {
            let mut seen = BTreeSet::new();
            let mut queue = VecDeque::from([source.clone()]);
            while let Some(current) = queue.pop_front() {
                for edge in self
                    .edges
                    .iter()
                    .filter(|edge| edge.source_entity_id == current && edge.relation == relation)
                {
                    if seen.insert(edge.target_entity_id.clone()) {
                        queue.push_back(edge.target_entity_id.clone());
                    }
                }
            }
            if !seen.is_empty() {
                out.insert(source.clone(), seen);
            }
        }
        self.acceleration
            .materialized_transitive
            .insert(relation_key.clone(), out.clone());
        out.into_iter()
            .map(|(source, rows)| (source, rows.into_iter().collect::<Vec<String>>()))
            .collect::<BTreeMap<String, Vec<String>>>()
    }

    pub fn neighborhood_summary(&mut self, entity_id: &str) -> Option<NeighborhoodSummary> {
        if let Some(summary) = self
            .acceleration
            .neighborhood_summaries
            .get(entity_id)
            .cloned()
        {
            return Some(summary);
        }
        let node = self.nodes.get(entity_id)?;
        let neighbors = self.adjacency.get(entity_id).cloned().unwrap_or_default();
        let mut relation_counts = BTreeMap::<String, usize>::new();
        let mut neighbor_kind_counts = BTreeMap::<String, usize>::new();
        for neighbor_id in &neighbors {
            if let Some(neighbor) = self.nodes.get(neighbor_id) {
                *neighbor_kind_counts
                    .entry(format!("{:?}", neighbor.kind).to_ascii_lowercase())
                    .or_insert(0) += 1;
            }
            for edge in self.edges.iter().filter(|edge| {
                edge.source_entity_id == *entity_id && edge.target_entity_id == *neighbor_id
            }) {
                *relation_counts
                    .entry(relation_label(&edge.relation).to_string())
                    .or_insert(0) += 1;
            }
        }
        let summary = NeighborhoodSummary {
            entity_id: node.entity_id.clone(),
            total_neighbors: neighbors.len(),
            relation_counts,
            neighbor_kind_counts,
        };
        self.acceleration
            .neighborhood_summaries
            .insert(entity_id.to_string(), summary.clone());
        Some(summary)
    }

    pub fn materialize_inference_edges(&mut self) -> Vec<KnowledgeGraphEdge> {
        self.acceleration.inferred_edges.clear();
        for left in &self.edges {
            for right in &self.edges {
                if left.target_entity_id != right.source_entity_id {
                    continue;
                }
                let inferred_relation = match (left.relation.clone(), right.relation.clone()) {
                    (KnowledgeRelationKind::DependsOn, KnowledgeRelationKind::DependsOn) => {
                        Some(KnowledgeRelationKind::DependsOn)
                    }
                    (KnowledgeRelationKind::StepOf, KnowledgeRelationKind::StepOf) => {
                        Some(KnowledgeRelationKind::StepOf)
                    }
                    _ => None,
                };
                if let Some(relation) = inferred_relation {
                    self.acceleration.inferred_edges.insert((
                        left.source_entity_id.clone(),
                        relation_label(&relation).to_string(),
                        right.target_entity_id.clone(),
                    ));
                }
            }
        }
        self.acceleration
            .inferred_edges
            .iter()
            .map(|(source, relation, target)| KnowledgeGraphEdge {
                edge_id: format!(
                    "inferred_{}",
                    &deterministic_hash(&(source.clone(), relation.clone(), target.clone()))[..24]
                ),
                source_entity_id: source.clone(),
                target_entity_id: target.clone(),
                relation: parse_relation_label(relation).unwrap_or(KnowledgeRelationKind::RefersTo),
                weight_bps: 5000,
                evidence_version_ids: Vec::new(),
                updated_at_ms: 0,
            })
            .collect::<Vec<KnowledgeGraphEdge>>()
    }

    pub fn sample_subgraph(&self, query: GraphSamplingQuery) -> GraphSample {
        let mut sampled = BTreeSet::new();
        let mut queue = VecDeque::new();
        let max_nodes = query.max_nodes.max(1);
        sampled.insert(query.seed_entity_id.clone());
        queue.push_back(query.seed_entity_id.clone());
        let mut counter = 0usize;
        while let Some(current) = queue.pop_front() {
            if sampled.len() >= max_nodes {
                break;
            }
            let neighbors = self.adjacency.get(&current).cloned().unwrap_or_default();
            for neighbor in neighbors {
                if sampled.len() >= max_nodes {
                    break;
                }
                counter = counter.saturating_add(1);
                let accept = match query.strategy {
                    GraphSamplingStrategy::RandomWalk => counter % 2 == 0,
                    GraphSamplingStrategy::ForestFire => {
                        (counter * 7919 + sampled.len()) % 10000 < usize::from(query.spread_bps)
                    }
                };
                if accept && sampled.insert(neighbor.clone()) {
                    queue.push_back(neighbor);
                }
            }
        }
        let sampled_edges = self
            .edges
            .iter()
            .filter(|edge| {
                sampled.contains(&edge.source_entity_id) && sampled.contains(&edge.target_entity_id)
            })
            .cloned()
            .collect::<Vec<KnowledgeGraphEdge>>();
        GraphSample {
            sampled_entity_ids: sampled.into_iter().collect::<Vec<String>>(),
            sampled_edges,
            approximate: true,
        }
    }

    pub fn approximate_tail_candidates(
        &mut self,
        head_entity_id: &str,
        relation: KnowledgeRelationKind,
        top_k: usize,
    ) -> Vec<EntityEmbeddingHit> {
        self.rebuild_entity_embeddings_if_needed(64);
        let Some(head_vec) = self
            .acceleration
            .entity_embeddings
            .get(head_entity_id)
            .cloned()
        else {
            return Vec::new();
        };
        let relation_vec = embed_text(relation_label(&relation), head_vec.len());
        let target = head_vec
            .iter()
            .zip(relation_vec.iter())
            .map(|(h, r)| h + r)
            .collect::<Vec<f32>>();
        let bucket_key = ann_bucket_key(&target);
        let candidates = self
            .acceleration
            .ann_buckets
            .get(&bucket_key)
            .cloned()
            .unwrap_or_else(|| self.nodes.keys().cloned().collect::<Vec<String>>());
        let mut scored = candidates
            .into_iter()
            .filter(|candidate| candidate != head_entity_id)
            .filter_map(|entity_id| {
                let embedding = self.acceleration.entity_embeddings.get(&entity_id)?;
                Some(EntityEmbeddingHit {
                    entity_id,
                    score: cosine_similarity(target.as_slice(), embedding.as_slice()),
                })
            })
            .collect::<Vec<EntityEmbeddingHit>>();
        scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal));
        scored.truncate(top_k.max(1));
        scored
    }

    pub fn build_partition_plan(
        &self,
        strategy: GraphPartitionStrategy,
        partition_count: usize,
    ) -> GraphPartitionPlan {
        let partition_count = partition_count.max(1);
        let mut partitions = BTreeMap::<usize, Vec<String>>::new();
        for entity_id in self.nodes.keys() {
            let idx = match strategy {
                GraphPartitionStrategy::Hash => {
                    deterministic_hash(entity_id)
                        .bytes()
                        .take(2)
                        .fold(0usize, |acc, row| {
                            acc.wrapping_mul(131).wrapping_add(usize::from(row))
                        })
                        % partition_count
                }
                GraphPartitionStrategy::Community => {
                    self.adjacency.get(entity_id).map(Vec::len).unwrap_or(0) % partition_count
                }
                GraphPartitionStrategy::Predicate => self
                    .edges
                    .iter()
                    .find(|edge| edge.source_entity_id == *entity_id)
                    .map(|edge| {
                        deterministic_hash(&relation_label(&edge.relation).to_string())
                            .bytes()
                            .next()
                            .map(usize::from)
                            .unwrap_or(0)
                            % partition_count
                    })
                    .unwrap_or(0),
            };
            partitions.entry(idx).or_default().push(entity_id.clone());
        }
        let owner = partitions
            .iter()
            .flat_map(|(idx, rows)| rows.iter().map(|row| (row.clone(), *idx)))
            .collect::<BTreeMap<String, usize>>();
        let cut_edges = self
            .edges
            .iter()
            .filter(|edge| owner.get(&edge.source_entity_id) != owner.get(&edge.target_entity_id))
            .count();
        GraphPartitionPlan {
            strategy: format!("{strategy:?}").to_ascii_lowercase(),
            partitions,
            cut_edges,
        }
    }

    pub fn build_federated_query_plan(
        &self,
        patterns: &[KnowledgeTriplePattern],
        services: Vec<FederatedServiceProfile>,
    ) -> FederatedQueryPlan {
        let mut steps = BTreeMap::<String, Vec<usize>>::new();
        for (idx, pattern) in patterns.iter().enumerate() {
            let best = services
                .iter()
                .filter(|service| {
                    pattern.relation.is_none()
                        || pattern
                            .relation
                            .as_ref()
                            .map(|relation| {
                                service
                                    .supported_relations
                                    .iter()
                                    .any(|row| row == relation)
                            })
                            .unwrap_or(true)
                })
                .max_by_key(|service| service.selectivity_hint_bps)
                .map(|service| service.service_id.clone());
            if let Some(service_id) = best {
                steps.entry(service_id).or_default().push(idx);
            }
        }
        let mut ordered = steps.keys().cloned().collect::<Vec<String>>();
        ordered.sort();
        FederatedQueryPlan {
            ordered_services: ordered.clone(),
            dispatch_steps: ordered
                .into_iter()
                .map(|service_id| FederatedDispatchStep {
                    pattern_indexes: steps.get(&service_id).cloned().unwrap_or_default(),
                    service_id,
                })
                .collect::<Vec<FederatedDispatchStep>>(),
        }
    }
}

fn parse_relation_label(label: &str) -> Option<KnowledgeRelationKind> {
    match label {
        "mentioned_with" => Some(KnowledgeRelationKind::MentionedWith),
        "depends_on" => Some(KnowledgeRelationKind::DependsOn),
        "owns" => Some(KnowledgeRelationKind::Owns),
        "prefers" => Some(KnowledgeRelationKind::Prefers),
        "affected_by" => Some(KnowledgeRelationKind::AffectedBy),
        "step_of" => Some(KnowledgeRelationKind::StepOf),
        "refers_to" => Some(KnowledgeRelationKind::RefersTo),
        "supports" => Some(KnowledgeRelationKind::Supports),
        _ => None,
    }
}

fn ann_bucket_key(embedding: &[f32]) -> String {
    embedding
        .iter()
        .take(4)
        .map(|row| if *row >= 0.0 { "1" } else { "0" })
        .collect::<Vec<&str>>()
        .join("")
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.is_empty() || b.is_empty() || a.len() != b.len() {
        return 0.0;
    }
    let mut dot = 0.0f32;
    let mut norm_a = 0.0f32;
    let mut norm_b = 0.0f32;
    for (left, right) in a.iter().zip(b.iter()) {
        dot += left * right;
        norm_a += left * left;
        norm_b += right * right;
    }
    if norm_a <= f32::EPSILON || norm_b <= f32::EPSILON {
        return 0.0;
    }
    dot / (norm_a.sqrt() * norm_b.sqrt())
}
