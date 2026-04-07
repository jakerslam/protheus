use std::collections::BTreeMap;

#[derive(Debug, Default, Clone)]
pub struct VectorIndex {
    embeddings: BTreeMap<String, Vec<f32>>,
}

impl VectorIndex {
    pub fn upsert(&mut self, key: impl Into<String>, embedding: Vec<f32>) {
        self.embeddings.insert(key.into(), embedding);
    }

    pub fn get(&self, key: &str) -> Option<&[f32]> {
        self.embeddings.get(key).map(|row| row.as_slice())
    }

    pub fn query_cosine(&self, query: &[f32], top_k: usize) -> Vec<(String, f32)> {
        let mut scored = self
            .embeddings
            .iter()
            .filter_map(|(key, row)| {
                cosine_similarity(query, row).map(|score| (key.clone(), score))
            })
            .collect::<Vec<_>>();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(top_k.max(1));
        scored
    }
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
