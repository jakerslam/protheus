// SPDX-License-Identifier: Apache-2.0
// Plane ownership: Safety Plane provenance primitives.

use crate::receipt::{canonicalize_json_value, ProvenanceError, Receipt, ReceiptSink};
use blake3::Hasher;
use serde::{Deserialize, Serialize};
use serde_json::json;
use thiserror::Error;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LedgerEntry {
    pub receipt: Receipt,
    pub prev_hash: Option<String>,
    pub entry_hash: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReplayStart {
    Genesis,
    Checkpoint(String),
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum LedgerError {
    #[error("genesis append requires prev_hash = None")]
    GenesisPrevHashMustBeNone,
    #[error("prev_hash mismatch")]
    PrevHashMismatch {
        expected: Option<String>,
        found: Option<String>,
    },
    #[error("unknown checkpoint: {0}")]
    UnknownCheckpoint(String),
    #[error("entry hash mismatch at index {index}")]
    EntryHashMismatch { index: usize },
    #[error("prev_hash chain mismatch at index {index}")]
    PrevHashChainMismatch {
        index: usize,
        expected: Option<String>,
        found: Option<String>,
    },
    #[error("serialization error: {0}")]
    Serialization(String),
}

#[derive(Clone, Debug, Default)]
pub struct ProvenanceLedger {
    entries: Vec<LedgerEntry>,
}

impl ProvenanceLedger {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn entries(&self) -> &[LedgerEntry] {
        &self.entries
    }

    pub fn tail_hash(&self) -> Option<&str> {
        self.entries.last().map(|entry| entry.entry_hash.as_str())
    }

    pub fn append(&mut self, mut entry: LedgerEntry) -> Result<String, LedgerError> {
        let expected_prev = self.tail_hash().map(ToString::to_string);
        match (&expected_prev, &entry.prev_hash) {
            (None, None) => {}
            (None, Some(_)) => return Err(LedgerError::GenesisPrevHashMustBeNone),
            (Some(expected), Some(found)) if expected == found => {}
            (Some(expected), found) => {
                return Err(LedgerError::PrevHashMismatch {
                    expected: Some(expected.clone()),
                    found: found.clone(),
                })
            }
        }
        entry.entry_hash = compute_entry_hash(&entry.receipt, &entry.prev_hash)?;
        let final_hash = entry.entry_hash.clone();
        self.entries.push(entry);
        Ok(final_hash)
    }

    pub fn replay_from(&self, start: ReplayStart) -> Result<Vec<LedgerEntry>, LedgerError> {
        let start_index = self.find_replay_start_index(start)?;
        Ok(self.entries[start_index..].to_vec())
    }

    pub fn verify_from(&self, start: ReplayStart) -> Result<(), LedgerError> {
        let start_index = self.find_replay_start_index(start)?;
        if self.entries.is_empty() {
            return Ok(());
        }
        let mut expected_prev = if start_index == 0 {
            None
        } else {
            Some(self.entries[start_index - 1].entry_hash.clone())
        };
        for (offset, entry) in self.entries[start_index..].iter().enumerate() {
            let index = start_index + offset;
            if entry.prev_hash != expected_prev {
                return Err(LedgerError::PrevHashChainMismatch {
                    index,
                    expected: expected_prev,
                    found: entry.prev_hash.clone(),
                });
            }
            let computed = compute_entry_hash(&entry.receipt, &entry.prev_hash)?;
            if computed != entry.entry_hash {
                return Err(LedgerError::EntryHashMismatch { index });
            }
            expected_prev = Some(entry.entry_hash.clone());
        }
        Ok(())
    }

    fn find_replay_start_index(&self, start: ReplayStart) -> Result<usize, LedgerError> {
        match start {
            ReplayStart::Genesis => Ok(0),
            ReplayStart::Checkpoint(hash) => self
                .entries
                .iter()
                .position(|entry| entry.entry_hash == hash)
                .ok_or(LedgerError::UnknownCheckpoint(hash)),
        }
    }
}

impl ReceiptSink for ProvenanceLedger {
    fn append(&mut self, receipt: &Receipt) -> Result<(), ProvenanceError> {
        let entry = LedgerEntry {
            receipt: receipt.clone(),
            prev_hash: self.tail_hash().map(ToString::to_string),
            entry_hash: String::new(),
        };
        self.append(entry)
            .map(|_| ())
            .map_err(|err| ProvenanceError::SinkAppend(format!("ledger_append_failed:{err}")))
    }
}

fn compute_entry_hash(
    receipt: &Receipt,
    prev_hash: &Option<String>,
) -> Result<String, LedgerError> {
    let payload = json!({
        "receipt": receipt,
        "prev_hash": prev_hash,
    });
    let canonical = canonicalize_json_value(&payload);
    let bytes = serde_json::to_vec(&canonical)
        .map_err(|err| LedgerError::Serialization(err.to_string()))?;
    let mut hasher = Hasher::new();
    hasher.update(&bytes);
    Ok(hasher.finalize().to_hex().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::receipt::Receipt;
    use chrono::Utc;
    use uuid::Uuid;

    fn sample_receipt(op: &str) -> Receipt {
        Receipt {
            id: Uuid::new_v4(),
            parent_id: None,
            op_type: op.to_string(),
            subject: Some("subject".to_string()),
            payload_hash: "abcd".to_string(),
            actor: "tester".to_string(),
            timestamp: Utc::now(),
            confidence: Some(0.8),
        }
    }

    #[test]
    fn genesis_append_succeeds() {
        let mut ledger = ProvenanceLedger::new();
        let hash = ledger
            .append(LedgerEntry {
                receipt: sample_receipt("genesis"),
                prev_hash: None,
                entry_hash: "caller-will-be-overwritten".to_string(),
            })
            .expect("append genesis");
        assert_eq!(ledger.entries.len(), 1);
        assert_eq!(ledger.entries[0].entry_hash, hash);
    }

    #[test]
    fn append_with_correct_prev_hash_chains_correctly() {
        let mut ledger = ProvenanceLedger::new();
        let first_hash = ledger
            .append(LedgerEntry {
                receipt: sample_receipt("a"),
                prev_hash: None,
                entry_hash: String::new(),
            })
            .expect("first");
        let second_hash = ledger
            .append(LedgerEntry {
                receipt: sample_receipt("b"),
                prev_hash: Some(first_hash.clone()),
                entry_hash: String::new(),
            })
            .expect("second");
        assert_eq!(ledger.entries[1].prev_hash, Some(first_hash));
        assert_eq!(ledger.entries[1].entry_hash, second_hash);
    }

    #[test]
    fn append_with_incorrect_prev_hash_is_rejected() {
        let mut ledger = ProvenanceLedger::new();
        ledger
            .append(LedgerEntry {
                receipt: sample_receipt("a"),
                prev_hash: None,
                entry_hash: String::new(),
            })
            .expect("first");
        let err = ledger
            .append(LedgerEntry {
                receipt: sample_receipt("b"),
                prev_hash: Some("bad".to_string()),
                entry_hash: String::new(),
            })
            .expect_err("must reject bad prev_hash");
        assert!(matches!(err, LedgerError::PrevHashMismatch { .. }));
    }

    #[test]
    fn replay_from_genesis_returns_full_ordered_sequence() {
        let mut ledger = ProvenanceLedger::new();
        for op in ["a", "b", "c"] {
            let prev = ledger.tail_hash().map(ToString::to_string);
            ledger
                .append(LedgerEntry {
                    receipt: sample_receipt(op),
                    prev_hash: prev,
                    entry_hash: String::new(),
                })
                .expect("append");
        }
        let replay = ledger
            .replay_from(ReplayStart::Genesis)
            .expect("replay genesis");
        assert_eq!(replay.len(), 3);
        assert_eq!(replay[0].receipt.op_type, "a");
        assert_eq!(replay[2].receipt.op_type, "c");
    }

    #[test]
    fn replay_from_checkpoint_is_inclusive() {
        let mut ledger = ProvenanceLedger::new();
        let mut hashes = Vec::new();
        for op in ["a", "b", "c"] {
            let prev = ledger.tail_hash().map(ToString::to_string);
            let hash = ledger
                .append(LedgerEntry {
                    receipt: sample_receipt(op),
                    prev_hash: prev,
                    entry_hash: String::new(),
                })
                .expect("append");
            hashes.push(hash);
        }
        let replay = ledger
            .replay_from(ReplayStart::Checkpoint(hashes[1].clone()))
            .expect("replay checkpoint");
        assert_eq!(replay.len(), 2);
        assert_eq!(replay[0].entry_hash, hashes[1]);
        assert_eq!(replay[1].entry_hash, hashes[2]);
    }

    #[test]
    fn verify_from_detects_corruption() {
        let mut ledger = ProvenanceLedger::new();
        for op in ["a", "b"] {
            let prev = ledger.tail_hash().map(ToString::to_string);
            ledger
                .append(LedgerEntry {
                    receipt: sample_receipt(op),
                    prev_hash: prev,
                    entry_hash: String::new(),
                })
                .expect("append");
        }
        ledger.entries[1].prev_hash = Some("corrupted-prev".to_string());
        let err = ledger
            .verify_from(ReplayStart::Genesis)
            .expect_err("verification should fail");
        assert!(matches!(err, LedgerError::PrevHashChainMismatch { .. }));
    }

    #[test]
    fn unknown_checkpoint_returns_unknown_checkpoint() {
        let ledger = ProvenanceLedger::new();
        let err = ledger
            .replay_from(ReplayStart::Checkpoint("missing".to_string()))
            .expect_err("missing checkpoint");
        assert!(matches!(err, LedgerError::UnknownCheckpoint(_)));
    }
}
