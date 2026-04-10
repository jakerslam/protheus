use crate::now_ms;
use crate::schemas::EvidenceCard;
use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::collections::{HashMap, HashSet};
use std::fs::{create_dir_all, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InvalidationRelationType {
    Invalidated,
    SupersededBy,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvidenceInvalidationRecord {
    pub trace_id: String,
    pub task_id: String,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EvidenceLedgerEvent {
    pub event_id: String,
    pub event_sequence: u64,
    pub timestamp: u64,
    pub record: EvidenceRecord,
}

pub struct EvidenceStore {
    records: Vec<EvidenceRecord>,
    evidence_by_id: HashMap<String, EvidenceCard>,
    dedupe_index: HashMap<String, String>,
    invalidated_ids: HashSet<String>,
    event_sequence: u64,
    ledger_events: Vec<EvidenceLedgerEvent>,
    ledger_path: PathBuf,
}

impl Default for EvidenceStore {
    fn default() -> Self {
        let mut out = Self::with_ledger_path(default_ledger_path());
        let _ = out.recover_from_ledger();
        out
    }
}

impl EvidenceStore {
    pub fn with_ledger_path(path: PathBuf) -> Self {
        Self {
            records: Vec::new(),
            evidence_by_id: HashMap::new(),
            dedupe_index: HashMap::new(),
            invalidated_ids: HashSet::new(),
            event_sequence: 0,
            ledger_events: Vec::new(),
            ledger_path: path,
        }
    }

    pub fn ledger_path(&self) -> &PathBuf {
        &self.ledger_path
    }

    pub fn ledger_events(&self) -> &[EvidenceLedgerEvent] {
        self.ledger_events.as_slice()
    }

    pub fn recover_from_ledger(&mut self) -> Result<usize, String> {
        if !self.ledger_path.exists() {
            return Ok(0);
        }
        self.records.clear();
        self.evidence_by_id.clear();
        self.dedupe_index.clear();
        self.invalidated_ids.clear();
        self.ledger_events.clear();
        self.event_sequence = 0;
        let file = File::open(&self.ledger_path)
            .map_err(|err| format!("evidence_store_recover_open_failed:{err}"))?;
        let mut recovered = 0usize;
        for line in BufReader::new(file).lines() {
            let row = line.map_err(|err| format!("evidence_store_recover_read_failed:{err}"))?;
            let trimmed = row.trim();
            if trimmed.is_empty() {
                continue;
            }
            let event = serde_json::from_str::<EvidenceLedgerEvent>(trimmed)
                .map_err(|err| format!("evidence_store_recover_decode_failed:{err}"))?;
            self.event_sequence = self.event_sequence.max(event.event_sequence);
            self.apply_record(event.record.clone());
            self.ledger_events.push(event);
            recovered = recovered.saturating_add(1);
        }
        Ok(recovered)
    }

    pub fn append_evidence(&mut self, cards: &[EvidenceCard]) -> Vec<String> {
        let mut out = Vec::<String>::new();
        for card in cards {
            if let Some(existing) = self.dedupe_index.get(&card.dedupe_hash).cloned() {
                out.push(existing);
                continue;
            }
            let record = EvidenceRecord::Evidence(card.clone());
            let _ = self.append_record(record);
            out.push(card.evidence_id.clone());
        }
        out
    }

    pub fn append_invalidation(
        &mut self,
        trace_id: &str,
        task_id: &str,
        target_evidence_id: &str,
        relation_type: InvalidationRelationType,
        replacement_evidence_id: Option<String>,
        lineage: Vec<String>,
    ) -> EvidenceInvalidationRecord {
        let record = EvidenceInvalidationRecord {
            trace_id: clean_text(trace_id, 160),
            task_id: clean_text(task_id, 160),
            target_evidence_id: target_evidence_id.to_string(),
            relation_type,
            replacement_evidence_id,
            lineage: sanitize_lineage(&lineage),
            timestamp: now_ms(),
        };
        let _ = self.append_record(EvidenceRecord::Invalidation(record.clone()));
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

    fn append_record(&mut self, record: EvidenceRecord) -> Result<(), String> {
        self.event_sequence = self.event_sequence.saturating_add(1);
        let event_sequence = self.event_sequence;
        let timestamp = now_ms();
        let event_id = deterministic_hash(&event_identity_seed(&record, event_sequence, timestamp));
        let event = EvidenceLedgerEvent {
            event_id,
            event_sequence,
            timestamp,
            record: record.clone(),
        };
        self.persist_event(&event)?;
        self.apply_record(record);
        self.ledger_events.push(event);
        Ok(())
    }

    fn apply_record(&mut self, record: EvidenceRecord) {
        match &record {
            EvidenceRecord::Evidence(card) => {
                self.evidence_by_id
                    .insert(card.evidence_id.clone(), card.clone());
                self.dedupe_index
                    .entry(card.dedupe_hash.clone())
                    .or_insert_with(|| card.evidence_id.clone());
            }
            EvidenceRecord::Invalidation(row) => {
                self.invalidated_ids
                    .insert(row.target_evidence_id.to_string());
            }
        }
        self.records.push(record);
    }

    fn persist_event(&self, event: &EvidenceLedgerEvent) -> Result<(), String> {
        if let Some(parent) = self.ledger_path.parent() {
            create_dir_all(parent)
                .map_err(|err| format!("evidence_store_ledger_create_dir_failed:{err}"))?;
        }
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.ledger_path)
            .map_err(|err| format!("evidence_store_ledger_open_failed:{err}"))?;
        let row = serde_json::to_string(event)
            .map_err(|err| format!("evidence_store_ledger_encode_failed:{err}"))?;
        file.write_all(format!("{row}\n").as_bytes())
            .map_err(|err| format!("evidence_store_ledger_append_failed:{err}"))?;
        Ok(())
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

fn clean_text(raw: &str, max_len: usize) -> String {
    raw.trim().chars().take(max_len).collect::<String>()
}

fn deterministic_hash<T: Serialize>(value: &T) -> String {
    let payload = serde_json::to_vec(value).unwrap_or_else(|_| b"{}".to_vec());
    let mut hasher = sha2::Sha256::new();
    hasher.update(&payload);
    format!("{:x}", hasher.finalize())
}

fn event_identity_seed(record: &EvidenceRecord, event_sequence: u64, timestamp: u64) -> serde_json::Value {
    let mut seed = serde_json::json!({
        "kind": "evidence_store_event",
        "event_sequence": event_sequence,
        "timestamp": timestamp,
    });
    match record {
        EvidenceRecord::Evidence(card) => {
            seed["record_type"] = serde_json::Value::String("evidence".to_string());
            seed["record_ref"] = serde_json::Value::String(card.evidence_id.clone());
        }
        EvidenceRecord::Invalidation(row) => {
            seed["record_type"] = serde_json::Value::String("invalidation".to_string());
            seed["record_ref"] = serde_json::Value::String(row.target_evidence_id.clone());
        }
    }
    seed
}

fn default_ledger_path() -> PathBuf {
    std::env::var("INFRING_EVIDENCE_STORE_LEDGER_PATH")
        .ok()
        .map(|v| PathBuf::from(clean_text(&v, 400)))
        .filter(|v| !v.as_os_str().is_empty())
        .unwrap_or_else(|| {
            std::env::temp_dir()
                .join("infring")
                .join("evidence_store_records.jsonl")
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schemas::{ConfidenceVector, EvidenceCard};
    use std::path::PathBuf;

    fn card(id: &str, dedupe_hash: &str) -> EvidenceCard {
        EvidenceCard {
            evidence_id: id.to_string(),
            evidence_content_id: format!("content-{id}"),
            evidence_event_id: format!("event-{id}"),
            trace_id: "t1".to_string(),
            task_id: "task-1".to_string(),
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

    fn temp_ledger_path(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "infring_evidence_store_test_{}_{}_{}.jsonl",
            tag,
            std::process::id(),
            now_ms()
        ))
    }

    #[test]
    fn append_only_invalidation_preserves_replay_history() {
        let mut store = EvidenceStore::with_ledger_path(temp_ledger_path("append_only"));
        let ids = store.append_evidence(&[card("e1", "d1")]);
        assert_eq!(ids, vec!["e1".to_string()]);
        let invalidation = store.append_invalidation(
            "trace-1",
            "task-1",
            "e1",
            InvalidationRelationType::Invalidated,
            None,
            vec!["v".to_string()],
        );
        assert_eq!(invalidation.target_evidence_id, "e1");
        assert_eq!(invalidation.trace_id, "trace-1");
        assert_eq!(invalidation.task_id, "task-1");
        assert_eq!(store.records().len(), 2);
        assert!(store.active_evidence().is_empty());
    }

    #[test]
    fn dedupe_lookup_reuses_existing_evidence_id() {
        let mut store = EvidenceStore::with_ledger_path(temp_ledger_path("dedupe"));
        let first = store.append_evidence(&[card("e1", "same")]);
        let second = store.append_evidence(&[card("e2", "same")]);
        assert_eq!(first, vec!["e1".to_string()]);
        assert_eq!(second, vec!["e1".to_string()]);
        assert_eq!(store.records().len(), 1);
    }

    #[test]
    fn recovers_append_only_records_from_ledger() {
        let ledger_path = temp_ledger_path("recover");
        let mut writer = EvidenceStore::with_ledger_path(ledger_path.clone());
        let first_ids = writer.append_evidence(&[card("e1", "d1")]);
        assert_eq!(first_ids, vec!["e1".to_string()]);
        writer.append_invalidation(
            "trace-1",
            "task-1",
            "e1",
            InvalidationRelationType::Invalidated,
            None,
            vec!["lineage".to_string()],
        );
        assert_eq!(writer.records().len(), 2);
        assert_eq!(writer.active_evidence().len(), 0);

        let mut recovered = EvidenceStore::with_ledger_path(ledger_path.clone());
        let count = recovered.recover_from_ledger().expect("recover");
        assert_eq!(count, 2);
        assert_eq!(recovered.records().len(), 2);
        assert_eq!(recovered.active_evidence().len(), 0);
        assert!(recovered.evidence_by_id("e1").is_some());
        assert_eq!(recovered.ledger_events().len(), 2);

        let _ = std::fs::remove_file(ledger_path);
    }
}
