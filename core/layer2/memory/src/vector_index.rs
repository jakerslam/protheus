use crate::schemas::{MemoryKind, TrustState};
use std::collections::{BTreeMap, BTreeSet};

pub trait VectorStoreBackend: Clone {
    fn upsert(&mut self, key: String, embedding: Vec<f32>);
    fn get_owned(&self, key: &str) -> Option<Vec<f32>>;
    fn entries(&self) -> Vec<(String, Vec<f32>)>;
}

#[derive(Debug, Default, Clone)]
pub struct InMemoryVectorStore {
    embeddings: BTreeMap<String, Vec<f32>>,
}

impl VectorStoreBackend for InMemoryVectorStore {
    fn upsert(&mut self, key: String, embedding: Vec<f32>) {
        self.embeddings.insert(key, embedding);
    }

    fn get_owned(&self, key: &str) -> Option<Vec<f32>> {
        self.embeddings.get(key).cloned()
    }

    fn entries(&self) -> Vec<(String, Vec<f32>)> {
        self.embeddings
            .iter()
            .map(|(key, row)| (key.clone(), row.clone()))
            .collect::<Vec<_>>()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VectorMetadata {
    pub scope_label: String,
    pub kind: MemoryKind,
    pub trust_state: TrustState,
    pub namespace: String,
    pub entity_refs: Vec<String>,
    pub source_object_id: String,
    pub source_version_id: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VectorQueryFilter {
    pub scope_labels: Vec<String>,
    pub kinds: Vec<MemoryKind>,
    pub trust_states: Vec<TrustState>,
    pub namespaces: Vec<String>,
    pub entity_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VectorQueryRow {
    pub key: String,
    pub score: f32,
    pub metadata: VectorMetadata,
}

#[derive(Debug, Clone)]
pub struct VectorIndex<B: VectorStoreBackend = InMemoryVectorStore> {
    backend: B,
    metadata: BTreeMap<String, VectorMetadata>,
    scope_index: BTreeMap<String, BTreeSet<String>>,
    kind_index: BTreeMap<String, BTreeSet<String>>,
    trust_state_index: BTreeMap<String, BTreeSet<String>>,
    namespace_index: BTreeMap<String, BTreeSet<String>>,
    entity_ref_index: BTreeMap<String, BTreeSet<String>>,
}

impl<B: VectorStoreBackend + Default> Default for VectorIndex<B> {
    fn default() -> Self {
        Self {
            backend: B::default(),
            metadata: BTreeMap::new(),
            scope_index: BTreeMap::new(),
            kind_index: BTreeMap::new(),
            trust_state_index: BTreeMap::new(),
            namespace_index: BTreeMap::new(),
            entity_ref_index: BTreeMap::new(),
        }
    }
}

impl<B: VectorStoreBackend> VectorIndex<B> {
    pub fn upsert(
        &mut self,
        key: impl Into<String>,
        embedding: Vec<f32>,
        metadata: VectorMetadata,
    ) {
        let key = key.into();
        if let Some(previous) = self.metadata.get(&key).cloned() {
            self.remove_filter_indexes(key.as_str(), &previous);
        }
        self.backend.upsert(key.clone(), embedding);
        self.insert_filter_indexes(key.as_str(), &metadata);
        self.metadata.insert(key, metadata);
    }

    pub fn get(&self, key: &str) -> Option<Vec<f32>> {
        self.backend.get_owned(key)
    }

    pub fn get_metadata(&self, key: &str) -> Option<&VectorMetadata> {
        self.metadata.get(key)
    }

    pub fn query_cosine_filtered(
        &self,
        query: &[f32],
        top_k: usize,
        filter: &VectorQueryFilter,
    ) -> Vec<VectorQueryRow> {
        let candidates = self.candidate_keys(filter);
        if candidates.is_empty() {
            return Vec::new();
        }
        let mut scored = candidates
            .into_iter()
            .filter_map(|key| {
                let row = self.backend.get_owned(&key)?;
                let metadata = self.metadata.get(&key)?.clone();
                if !matches_filter(&metadata, filter) {
                    return None;
                }
                let score = cosine_similarity(query, row.as_slice())?;
                Some(VectorQueryRow {
                    key,
                    score,
                    metadata,
                })
            })
            .collect::<Vec<VectorQueryRow>>();
        scored.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        scored.truncate(top_k.max(1));
        scored
    }

    fn candidate_keys(&self, filter: &VectorQueryFilter) -> BTreeSet<String> {
        let mut dimensions = Vec::<BTreeSet<String>>::new();
        if !filter.scope_labels.is_empty() {
            dimensions.push(collect_union(
                &self.scope_index,
                filter.scope_labels.as_slice(),
            ));
        }
        if !filter.kinds.is_empty() {
            let keys = filter
                .kinds
                .iter()
                .map(kind_index_key)
                .collect::<Vec<&'static str>>();
            dimensions.push(collect_union_labels(&self.kind_index, keys.as_slice()));
        }
        if !filter.trust_states.is_empty() {
            let keys = filter
                .trust_states
                .iter()
                .map(trust_state_index_key)
                .collect::<Vec<&'static str>>();
            dimensions.push(collect_union_labels(
                &self.trust_state_index,
                keys.as_slice(),
            ));
        }
        if !filter.namespaces.is_empty() {
            dimensions.push(collect_union(
                &self.namespace_index,
                filter.namespaces.as_slice(),
            ));
        }
        if !filter.entity_refs.is_empty() {
            dimensions.push(collect_union(
                &self.entity_ref_index,
                filter.entity_refs.as_slice(),
            ));
        }
        if dimensions.is_empty() {
            return self.metadata.keys().cloned().collect::<BTreeSet<String>>();
        }
        dimensions.sort_by_key(BTreeSet::len);
        let mut iter = dimensions.into_iter();
        let mut out = iter.next().unwrap_or_default();
        for dimension in iter {
            out = out
                .intersection(&dimension)
                .cloned()
                .collect::<BTreeSet<String>>();
            if out.is_empty() {
                break;
            }
        }
        out
    }

    fn insert_filter_indexes(&mut self, key: &str, metadata: &VectorMetadata) {
        index_insert(&mut self.scope_index, metadata.scope_label.as_str(), key);
        index_insert(&mut self.kind_index, kind_index_key(&metadata.kind), key);
        index_insert(
            &mut self.trust_state_index,
            trust_state_index_key(&metadata.trust_state),
            key,
        );
        index_insert(&mut self.namespace_index, metadata.namespace.as_str(), key);
        for entity_ref in &metadata.entity_refs {
            index_insert(&mut self.entity_ref_index, entity_ref.as_str(), key);
        }
    }

    fn remove_filter_indexes(&mut self, key: &str, metadata: &VectorMetadata) {
        index_remove(&mut self.scope_index, metadata.scope_label.as_str(), key);
        index_remove(&mut self.kind_index, kind_index_key(&metadata.kind), key);
        index_remove(
            &mut self.trust_state_index,
            trust_state_index_key(&metadata.trust_state),
            key,
        );
        index_remove(&mut self.namespace_index, metadata.namespace.as_str(), key);
        for entity_ref in &metadata.entity_refs {
            index_remove(&mut self.entity_ref_index, entity_ref.as_str(), key);
        }
    }
}

fn kind_index_key(kind: &MemoryKind) -> &'static str {
    match kind {
        MemoryKind::Working => "working",
        MemoryKind::Episodic => "episodic",
        MemoryKind::Semantic => "semantic",
        MemoryKind::Procedural => "procedural",
    }
}

fn trust_state_index_key(state: &TrustState) -> &'static str {
    match state {
        TrustState::Proposed => "proposed",
        TrustState::Corroborated => "corroborated",
        TrustState::Validated => "validated",
        TrustState::Canonical => "canonical",
        TrustState::Contested => "contested",
        TrustState::Quarantined => "quarantined",
        TrustState::Revoked => "revoked",
    }
}

fn collect_union(index: &BTreeMap<String, BTreeSet<String>>, keys: &[String]) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for key in keys {
        if let Some(rows) = index.get(key) {
            out.extend(rows.iter().cloned());
        }
    }
    out
}

fn collect_union_labels(
    index: &BTreeMap<String, BTreeSet<String>>,
    keys: &[&str],
) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for key in keys {
        if let Some(rows) = index.get(*key) {
            out.extend(rows.iter().cloned());
        }
    }
    out
}

fn index_insert(index: &mut BTreeMap<String, BTreeSet<String>>, bucket: &str, key: &str) {
    index
        .entry(bucket.to_string())
        .or_default()
        .insert(key.to_string());
}

fn index_remove(index: &mut BTreeMap<String, BTreeSet<String>>, bucket: &str, key: &str) {
    if let Some(values) = index.get_mut(bucket) {
        values.remove(key);
        if values.is_empty() {
            index.remove(bucket);
        }
    }
}

fn matches_filter(metadata: &VectorMetadata, filter: &VectorQueryFilter) -> bool {
    if !filter.scope_labels.is_empty()
        && !filter
            .scope_labels
            .iter()
            .any(|row| row == &metadata.scope_label)
    {
        return false;
    }
    if !filter.kinds.is_empty() && !filter.kinds.iter().any(|row| row == &metadata.kind) {
        return false;
    }
    if !filter.trust_states.is_empty()
        && !filter
            .trust_states
            .iter()
            .any(|row| row == &metadata.trust_state)
    {
        return false;
    }
    if !filter.namespaces.is_empty()
        && !filter
            .namespaces
            .iter()
            .any(|row| row == &metadata.namespace)
    {
        return false;
    }
    if !filter.entity_refs.is_empty()
        && !metadata
            .entity_refs
            .iter()
            .any(|row| filter.entity_refs.iter().any(|candidate| candidate == row))
    {
        return false;
    }
    true
}

pub fn embed_text(text: &str, dims: usize) -> Vec<f32> {
    let dims = dims.max(8);
    let mut out = vec![0.0f32; dims];
    let mut total = 0.0f32;
    for token in text
        .to_ascii_lowercase()
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|row| !row.is_empty())
    {
        let mut hash = 0u64;
        for byte in token.as_bytes() {
            hash = hash.wrapping_mul(16777619).wrapping_add(u64::from(*byte));
        }
        let idx = (hash as usize) % dims;
        out[idx] += 1.0;
        total += 1.0;
    }
    if total > 0.0 {
        for value in &mut out {
            *value /= total;
        }
    }
    out
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> Option<f32> {
    if a.is_empty() || b.is_empty() || a.len() != b.len() {
        return None;
    }
    let mut dot = 0.0f32;
    let mut norm_a = 0.0f32;
    let mut norm_b = 0.0f32;
    for (av, bv) in a.iter().zip(b.iter()) {
        dot += av * bv;
        norm_a += av * av;
        norm_b += bv * bv;
    }
    if norm_a <= f32::EPSILON || norm_b <= f32::EPSILON {
        return None;
    }
    Some(dot / (norm_a.sqrt() * norm_b.sqrt()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn metadata(entity_ref: &str, kind: MemoryKind) -> VectorMetadata {
        VectorMetadata {
            scope_label: "core".to_string(),
            kind,
            trust_state: TrustState::Validated,
            namespace: "memory.tests".to_string(),
            entity_refs: vec![entity_ref.to_string()],
            source_object_id: "obj".to_string(),
            source_version_id: "v1".to_string(),
        }
    }

    #[test]
    fn filtered_query_honors_entity_and_kind() {
        let mut index: VectorIndex<InMemoryVectorStore> = VectorIndex::default();
        index.upsert(
            "alpha",
            embed_text("alice atlas", 32),
            metadata("person:alice", MemoryKind::Episodic),
        );
        index.upsert(
            "beta",
            embed_text("postgres outage", 32),
            metadata("system:postgres", MemoryKind::Semantic),
        );
        let rows = index.query_cosine_filtered(
            &embed_text("alice", 32),
            4,
            &VectorQueryFilter {
                kinds: vec![MemoryKind::Episodic],
                entity_refs: vec!["person:alice".to_string()],
                ..VectorQueryFilter::default()
            },
        );
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].key, "alpha");
    }

    #[test]
    fn upsert_reindexes_filter_buckets_for_existing_key() {
        let mut index: VectorIndex<InMemoryVectorStore> = VectorIndex::default();
        index.upsert(
            "alpha",
            embed_text("alice atlas", 32),
            metadata("person:alice", MemoryKind::Episodic),
        );
        index.upsert(
            "alpha",
            embed_text("postgres cluster", 32),
            metadata("system:postgres", MemoryKind::Semantic),
        );
        let stale = index.query_cosine_filtered(
            &embed_text("alice", 32),
            4,
            &VectorQueryFilter {
                kinds: vec![MemoryKind::Episodic],
                entity_refs: vec!["person:alice".to_string()],
                ..VectorQueryFilter::default()
            },
        );
        assert!(stale.is_empty());
        let fresh = index.query_cosine_filtered(
            &embed_text("postgres", 32),
            4,
            &VectorQueryFilter {
                kinds: vec![MemoryKind::Semantic],
                entity_refs: vec!["system:postgres".to_string()],
                ..VectorQueryFilter::default()
            },
        );
        assert_eq!(fresh.len(), 1);
        assert_eq!(fresh[0].key, "alpha");
    }
}
