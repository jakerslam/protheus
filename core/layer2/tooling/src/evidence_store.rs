use crate::schemas::EvidenceCard;
use crate::now_ms;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InvalidationRelationType {
    Invalidated,
    SupersededBy,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvidenceInvalidationRecord {
    pub target_evidence_id: String,
    pub relation_type: InvalidationRelationType,
    pub replacement_evidence_id: Option<String>,
    pub lineage: Vec<String>,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "record_type", rename_all = "snake_case")]
pub enum EvidenceRecord {
    Evidence(EvidenceCard),
    Invalidation(EvidenceInvalidationRecord),
}

#[derive(Default)]
pub struct EvidenceStore {
    records: Vec<EvidenceRecord>,
    evidence_by_id: HashMap<String, EvidenceCard>,
    dedupe_index: HashMap<String, String>,
    invalidated_ids: HashSet<String>,
}

impl EvidenceStore {
    pub fn append_evidence(&mut self, cards: &[EvidenceCard]) -> Vec<String> {
        let mut out = Vec::<String>::new();
        for card in cards {
            if let Some(existing) = self.dedupe_index.get(&card.dedupe_hash).cloned() {
                out.push(existing);
                continue;
            }
            self.records.push(EvidenceRecord::Evidence(card.clone()));
            self.evidence_by_id
                .insert(card.evidence_id.clone(), card.clone());
            self.dedupe_index
                .insert(card.dedupe_hash.clone(), card.evidence_id.clone());
            out.push(card.evidence_id.clone());
        }
        out
    }

    pub fn append_invalidation(
        &mut self,
        target_evidence_id: &str,
        relation_type: InvalidationRelationType,
        replacement_evidence_id: Option<String>,
        lineage: Vec<String>,
    ) -> EvidenceInvalidationRecord {
        let record = EvidenceInvalidationRecord {
            target_evidence_id: target_evidence_id.to_string(),
            relation_type,
            replacement_evidence_id,
            lineage: sanitize_lineage(&lineage),
            timestamp: now_ms(),
        };
        self.invalidated_ids
            .insert(record.target_evidence_id.to_string());
        self.records.push(EvidenceRecord::Invalidation(record.clone()));
        record
    }

    pub fn records(&self) -> &[EvidenceRecord] {
        self.records.as_slice()
    }

    pub fn evidence_by_id(&self, evidence_id: &str) -> Option<&EvidenceCard> {
        self.evidence_by_id.get(evidence_id)
    }

    pub fn active_evidence(&self) -> Vec<EvidenceCard> {
        self.evidence_by_id
            .values()
            .filter(|card| !self.invalidated_ids.contains(card.evidence_id.as_str()))
            .cloned()
            .collect::<Vec<_>>()
    }
}

fn sanitize_lineage(lineage: &[String]) -> Vec<String> {
    let mut out = lineage
        .iter()
        .map(|v| v.trim().chars().take(200).collect::<String>())
        .filter(|v| !v.is_empty())
        .collect::<Vec<_>>();
    if out.is_empty() {
        out.push("evidence_store_v1".to_string());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schemas::{ConfidenceVector, EvidenceCard};

    fn card(id: &str, dedupe_hash: &str) -> EvidenceCard {
        EvidenceCard {
            evidence_id: id.to_string(),
            derived_from_result_id: "r1".to_string(),
            source_ref: "https://example.com".to_string(),
            source_location: "results[0]".to_string(),
            excerpt: "excerpt".to_string(),
            summary: "summary".to_string(),
            confidence_vector: ConfidenceVector {
                relevance: 0.8,
                reliability: 0.8,
                freshness: 0.8,
            },
            dedupe_hash: dedupe_hash.to_string(),
            lineage: vec!["l1".to_string()],
            timestamp: 1,
        }
    }

    #[test]
    fn append_only_invalidation_preserves_replay_history() {
        let mut store = EvidenceStore::default();
        let ids = store.append_evidence(&[card("e1", "d1")]);
        assert_eq!(ids, vec!["e1".to_string()]);
        let invalidation = store.append_invalidation(
            "e1",
            InvalidationRelationType::Invalidated,
            None,
            vec!["v".to_string()],
        );
        assert_eq!(invalidation.target_evidence_id, "e1");
        assert_eq!(store.records().len(), 2);
        assert!(store.active_evidence().is_empty());
    }

    #[test]
    fn dedupe_lookup_reuses_existing_evidence_id() {
        let mut store = EvidenceStore::default();
        let first = store.append_evidence(&[card("e1", "same")]);
        let second = store.append_evidence(&[card("e2", "same")]);
        assert_eq!(first, vec!["e1".to_string()]);
        assert_eq!(second, vec!["e1".to_string()]);
        assert_eq!(store.records().len(), 1);
    }
}
