use crate::{deterministic_hash, now_ms};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

use super::graph_query_acceleration_types::NeighborhoodSummary;
use super::KnowledgeRelationKind;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct CachedGraphQueryEntry {
    pub pattern_fingerprints: BTreeSet<String>,
    pub bindings: Vec<BTreeMap<String, String>>,
    pub matched_edge_ids: Vec<String>,
    pub created_at_ms: u64,
    pub ttl_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct TripleIndexes {
    pub spo: BTreeMap<(String, String), BTreeSet<String>>,
    pub sop: BTreeMap<(String, String), BTreeSet<String>>,
    pub pso: BTreeMap<(String, String), BTreeSet<String>>,
    pub pos: BTreeMap<(String, String), BTreeSet<String>>,
    pub osp: BTreeMap<(String, String), BTreeSet<String>>,
    pub ops: BTreeMap<(String, String), BTreeSet<String>>,
}

impl Default for TripleIndexes {
    fn default() -> Self {
        Self {
            spo: BTreeMap::new(),
            sop: BTreeMap::new(),
            pso: BTreeMap::new(),
            pos: BTreeMap::new(),
            osp: BTreeMap::new(),
            ops: BTreeMap::new(),
        }
    }
}

impl TripleIndexes {
    pub fn insert(&mut self, subject: &str, relation: &str, object: &str) {
        self.spo
            .entry((subject.to_string(), relation.to_string()))
            .or_default()
            .insert(object.to_string());
        self.sop
            .entry((subject.to_string(), object.to_string()))
            .or_default()
            .insert(relation.to_string());
        self.pso
            .entry((relation.to_string(), subject.to_string()))
            .or_default()
            .insert(object.to_string());
        self.pos
            .entry((relation.to_string(), object.to_string()))
            .or_default()
            .insert(subject.to_string());
        self.osp
            .entry((object.to_string(), subject.to_string()))
            .or_default()
            .insert(relation.to_string());
        self.ops
            .entry((object.to_string(), relation.to_string()))
            .or_default()
            .insert(subject.to_string());
    }

    pub fn has_edge(&self, subject: &str, relation: &str, object: &str) -> bool {
        self.spo
            .get(&(subject.to_string(), relation.to_string()))
            .map(|set| set.iter().any(|row| row == object))
            .unwrap_or(false)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct SimpleBloom {
    bits: Vec<u64>,
    hash_rounds: u8,
}

impl Default for SimpleBloom {
    fn default() -> Self {
        Self {
            bits: vec![0u64; 256],
            hash_rounds: 3,
        }
    }
}

impl SimpleBloom {
    fn hash_at(&self, value: &str, idx: u8) -> usize {
        let digest = deterministic_hash(&(value.to_string(), idx));
        let mut acc = 0usize;
        for byte in digest.as_bytes().iter().take(8) {
            acc = acc.wrapping_mul(16777619).wrapping_add(usize::from(*byte));
        }
        acc % (self.bits.len() * 64)
    }

    pub fn insert(&mut self, value: &str) {
        for idx in 0..self.hash_rounds {
            let bit = self.hash_at(value, idx);
            self.bits[bit / 64] |= 1u64 << (bit % 64);
        }
    }

    pub fn might_contain(&self, value: &str) -> bool {
        (0..self.hash_rounds).all(|idx| {
            let bit = self.hash_at(value, idx);
            (self.bits[bit / 64] & (1u64 << (bit % 64))) != 0
        })
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct KnowledgeGraphAccelerationState {
    pub triple_indexes: TripleIndexes,
    pub predicate_counts: BTreeMap<String, usize>,
    pub predicate_object_counts: BTreeMap<(String, String), usize>,
    pub node_ordinals: BTreeMap<String, usize>,
    pub predicate_bitmaps: BTreeMap<String, Vec<u64>>,
    pub relation_source_bloom: BTreeMap<String, SimpleBloom>,
    pub characteristic_sets: BTreeMap<String, BTreeSet<String>>,
    pub characteristic_counts: BTreeMap<String, usize>,
    pub subgraph_cache: BTreeMap<String, CachedGraphQueryEntry>,
    pub materialized_transitive: BTreeMap<String, BTreeMap<String, BTreeSet<String>>>,
    pub neighborhood_summaries: BTreeMap<String, NeighborhoodSummary>,
    pub inferred_edges: BTreeSet<(String, String, String)>,
    pub entity_embeddings: BTreeMap<String, Vec<f32>>,
    pub ann_buckets: BTreeMap<String, Vec<String>>,
}

impl KnowledgeGraphAccelerationState {
    pub fn register_node(&mut self, entity_id: &str) {
        if self.node_ordinals.contains_key(entity_id) {
            return;
        }
        let ordinal = self.node_ordinals.len();
        self.node_ordinals.insert(entity_id.to_string(), ordinal);
        let required_words = ((ordinal + 1) + 63) / 64;
        for bitmap in self.predicate_bitmaps.values_mut() {
            while bitmap.len() < required_words {
                bitmap.push(0);
            }
        }
    }

    fn set_bitmap_bit(&mut self, relation: &str, entity_id: &str) {
        let Some(ordinal) = self.node_ordinals.get(entity_id).copied() else {
            return;
        };
        let required_words = ((ordinal + 1) + 63) / 64;
        let bits = self
            .predicate_bitmaps
            .entry(relation.to_string())
            .or_default();
        while bits.len() < required_words {
            bits.push(0);
        }
        bits[ordinal / 64] |= 1u64 << (ordinal % 64);
    }

    pub fn register_edge(&mut self, source: &str, target: &str, relation: &KnowledgeRelationKind) {
        self.register_node(source);
        self.register_node(target);
        let relation_key = relation_label(relation);
        self.triple_indexes.insert(source, relation_key, target);
        *self
            .predicate_counts
            .entry(relation_key.to_string())
            .or_insert(0) += 1;
        *self
            .predicate_object_counts
            .entry((relation_key.to_string(), target.to_string()))
            .or_insert(0) += 1;
        self.set_bitmap_bit(relation_key, source);
        self.set_bitmap_bit(relation_key, target);
        self.relation_source_bloom
            .entry(relation_key.to_string())
            .or_default()
            .insert(source);
        self.relation_source_bloom
            .entry(relation_key.to_string())
            .or_default()
            .insert(target);
        self.characteristic_sets
            .entry(source.to_string())
            .or_default()
            .insert(relation_key.to_string());
        self.characteristic_sets
            .entry(target.to_string())
            .or_default()
            .insert(relation_key.to_string());
        self.recompute_characteristic_counts();
        self.materialized_transitive.clear();
        self.neighborhood_summaries.remove(source);
        self.neighborhood_summaries.remove(target);
        self.inferred_edges.clear();
        self.entity_embeddings.remove(source);
        self.entity_embeddings.remove(target);
        self.ann_buckets.clear();
    }

    pub fn relation_source_might_exist(&self, relation: &str, source: &str) -> bool {
        self.relation_source_bloom
            .get(relation)
            .map(|bloom| bloom.might_contain(source))
            .unwrap_or(false)
    }

    pub fn relation_bitmap_and(&self, relations: &[String]) -> BTreeSet<String> {
        if relations.is_empty() {
            return BTreeSet::new();
        }
        let mut relation_bitmaps = relations
            .iter()
            .filter_map(|row| self.predicate_bitmaps.get(row).cloned())
            .collect::<Vec<Vec<u64>>>();
        if relation_bitmaps.is_empty() {
            return BTreeSet::new();
        }
        relation_bitmaps.sort_by_key(Vec::len);
        let mut acc = relation_bitmaps.remove(0);
        for bitmap in relation_bitmaps {
            let len = acc.len().min(bitmap.len());
            for idx in 0..len {
                acc[idx] &= bitmap[idx];
            }
            for row in acc.iter_mut().skip(len) {
                *row = 0;
            }
        }
        let inverse_ordinals = self
            .node_ordinals
            .iter()
            .map(|(id, ordinal)| (*ordinal, id.clone()))
            .collect::<BTreeMap<usize, String>>();
        let mut out = BTreeSet::new();
        for (word_idx, word) in acc.iter().copied().enumerate() {
            if word == 0 {
                continue;
            }
            for bit in 0..64 {
                if (word & (1u64 << bit)) == 0 {
                    continue;
                }
                let ordinal = word_idx * 64 + bit;
                if let Some(entity_id) = inverse_ordinals.get(&ordinal) {
                    out.insert(entity_id.clone());
                }
            }
        }
        out
    }

    pub fn cache_get_fresh(&self, key: &str) -> Option<&CachedGraphQueryEntry> {
        self.subgraph_cache.get(key).and_then(|entry| {
            if now_ms() <= entry.created_at_ms.saturating_add(entry.ttl_ms) {
                Some(entry)
            } else {
                None
            }
        })
    }

    pub fn cache_get_seed(
        &self,
        pattern_fingerprints: &BTreeSet<String>,
    ) -> Option<&CachedGraphQueryEntry> {
        self.subgraph_cache.values().find(|entry| {
            now_ms() <= entry.created_at_ms.saturating_add(entry.ttl_ms)
                && entry.pattern_fingerprints.is_subset(pattern_fingerprints)
                && entry.pattern_fingerprints != *pattern_fingerprints
        })
    }

    pub fn cache_put(
        &mut self,
        key: String,
        pattern_fingerprints: BTreeSet<String>,
        bindings: Vec<BTreeMap<String, String>>,
        matched_edge_ids: Vec<String>,
        ttl_ms: u64,
    ) {
        self.subgraph_cache.insert(
            key,
            CachedGraphQueryEntry {
                pattern_fingerprints,
                bindings,
                matched_edge_ids,
                created_at_ms: now_ms(),
                ttl_ms: ttl_ms.max(1),
            },
        );
        if self.subgraph_cache.len() > 256 {
            let mut ordered = self
                .subgraph_cache
                .iter()
                .map(|(key, row)| (row.created_at_ms, key.clone()))
                .collect::<Vec<(u64, String)>>();
            ordered.sort_by_key(|(created, _)| *created);
            for (_, key) in ordered.into_iter().take(self.subgraph_cache.len() - 256) {
                self.subgraph_cache.remove(key.as_str());
            }
        }
    }

    fn recompute_characteristic_counts(&mut self) {
        self.characteristic_counts.clear();
        for relations in self.characteristic_sets.values() {
            let key = relations.iter().cloned().collect::<Vec<String>>().join("|");
            *self.characteristic_counts.entry(key).or_insert(0) += 1;
        }
    }
}

pub(crate) fn relation_label(relation: &KnowledgeRelationKind) -> &'static str {
    match relation {
        KnowledgeRelationKind::MentionedWith => "mentioned_with",
        KnowledgeRelationKind::DependsOn => "depends_on",
        KnowledgeRelationKind::Owns => "owns",
        KnowledgeRelationKind::Prefers => "prefers",
        KnowledgeRelationKind::AffectedBy => "affected_by",
        KnowledgeRelationKind::StepOf => "step_of",
        KnowledgeRelationKind::RefersTo => "refers_to",
        KnowledgeRelationKind::Supports => "supports",
    }
}
