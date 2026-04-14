use crate::schemas::{MemoryKind, TrustState};
use std::collections::BTreeMap;

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
}

impl<B: VectorStoreBackend + Default> Default for VectorIndex<B> {
    fn default() -> Self {
        Self {
            backend: B::default(),
            metadata: BTreeMap::new(),
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
        self.backend.upsert(key.clone(), embedding);
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
        let mut scored = self
            .backend
            .entries()
            .into_iter()
            .filter_map(|(key, row)| {
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
}
