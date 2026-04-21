// SPDX-License-Identifier: Apache-2.0
// Plane ownership: Layer 1 direct primitives.

use protheus_nexus_core_v1::{ProvenanceError, ReceiptDraft, ReceiptEmitter, ReceiptSink};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Blake3Hash(pub String);

impl Blake3Hash {
    pub fn validate(&self) -> Result<(), HashIndexError> {
        let is_valid = self.0.len() == 64
            && self
                .0
                .chars()
                .all(|ch| ch.is_ascii_digit() || ('a'..='f').contains(&ch));
        if is_valid {
            Ok(())
        } else {
            Err(HashIndexError::InvalidHashFormat(self.0.clone()))
        }
    }

    #[cfg(test)]
    fn from_payload(payload: &str) -> Self {
        use blake3::Hasher;
        let mut hasher = Hasher::new();
        hasher.update(payload.as_bytes());
        Self(hasher.finalize().to_hex().to_string())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Reference {
    Task(Uuid),
    Receipt(Uuid),
    Object(Uuid),
    FilePath(String),
}

#[derive(Debug, Error)]
pub enum HashIndexError {
    #[error("invalid blake3 hash format: {0}")]
    InvalidHashFormat(String),
    #[error(transparent)]
    Provenance(#[from] ProvenanceError),
}

#[derive(Clone, Debug, Default)]
pub struct HashIndex {
    map: BTreeMap<Blake3Hash, BTreeSet<Reference>>,
}

impl HashIndex {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert<S: ReceiptSink>(
        &mut self,
        key: Blake3Hash,
        reference: Reference,
        emitter: &mut ReceiptEmitter<S>,
    ) -> Result<(), HashIndexError> {
        key.validate()?;
        let refs = self.map.entry(key.clone()).or_default();
        if !refs.insert(reference.clone()) {
            return Ok(());
        }
        emitter.emit(ReceiptDraft {
            parent_id: None,
            op_type: "hash_index_insert",
            subject: Some(key.0.clone()),
            payload: &json!({"reference": reference}),
            actor: "hash_index",
            confidence: None,
        })?;
        Ok(())
    }

    pub fn get(&self, key: &Blake3Hash) -> Vec<Reference> {
        if key.validate().is_err() {
            return Vec::new();
        }
        self.map
            .get(key)
            .map(|refs| refs.iter().cloned().collect())
            .unwrap_or_default()
    }

    pub fn get_many(&self, keys: &[Blake3Hash]) -> BTreeMap<Blake3Hash, Vec<Reference>> {
        let mut out = BTreeMap::new();
        for key in keys {
            out.insert(key.clone(), self.get(key));
        }
        out
    }

    pub fn remove<S: ReceiptSink>(
        &mut self,
        key: &Blake3Hash,
        reference: &Reference,
        emitter: &mut ReceiptEmitter<S>,
    ) -> Result<(), HashIndexError> {
        key.validate()?;
        let Some(refs) = self.map.get_mut(key) else {
            return Ok(());
        };
        if !refs.remove(reference) {
            return Ok(());
        }
        if refs.is_empty() {
            self.map.remove(key);
        }
        emitter.emit(ReceiptDraft {
            parent_id: None,
            op_type: "hash_index_remove",
            subject: Some(key.0.clone()),
            payload: &json!({"reference": reference}),
            actor: "hash_index",
            confidence: None,
        })?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use protheus_nexus_core_v1::{InMemoryReceiptSink, ReceiptEmitter};

    #[test]
    fn valid_and_invalid_blake3_hash_validation() {
        let valid = Blake3Hash::from_payload("abc");
        assert!(valid.validate().is_ok());
        let invalid_upper = Blake3Hash(
            "ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789".to_string(),
        );
        assert!(invalid_upper.validate().is_err());
        let invalid_short = Blake3Hash("abc".to_string());
        assert!(invalid_short.validate().is_err());
    }

    #[test]
    fn insert_then_get_returns_stored_refs() {
        let mut index = HashIndex::new();
        let mut emitter = ReceiptEmitter::new(InMemoryReceiptSink::default());
        let key = Blake3Hash::from_payload("insert-get");
        let reference = Reference::Task(Uuid::new_v4());
        index
            .insert(key.clone(), reference.clone(), &mut emitter)
            .expect("insert");
        assert_eq!(index.get(&key), vec![reference]);
    }

    #[test]
    fn duplicate_insert_is_no_op() {
        let mut index = HashIndex::new();
        let mut emitter = ReceiptEmitter::new(InMemoryReceiptSink::default());
        let key = Blake3Hash::from_payload("dup");
        let reference = Reference::Receipt(Uuid::new_v4());
        index
            .insert(key.clone(), reference.clone(), &mut emitter)
            .expect("first insert");
        let before = emitter.sink().receipts.len();
        index
            .insert(key.clone(), reference, &mut emitter)
            .expect("duplicate insert");
        assert_eq!(before, emitter.sink().receipts.len());
    }

    #[test]
    fn multiple_refs_under_same_key_are_preserved() {
        let mut index = HashIndex::new();
        let mut emitter = ReceiptEmitter::new(InMemoryReceiptSink::default());
        let key = Blake3Hash::from_payload("collision");
        let task = Reference::Task(Uuid::new_v4());
        let object = Reference::Object(Uuid::new_v4());
        index
            .insert(key.clone(), task.clone(), &mut emitter)
            .expect("insert task");
        index
            .insert(key.clone(), object.clone(), &mut emitter)
            .expect("insert object");
        assert_eq!(index.get(&key), vec![task, object]);
    }

    #[test]
    fn get_many_returns_requested_keys_deterministically() {
        let mut index = HashIndex::new();
        let mut emitter = ReceiptEmitter::new(InMemoryReceiptSink::default());
        let key_a = Blake3Hash::from_payload("a");
        let key_b = Blake3Hash::from_payload("b");
        let key_c = Blake3Hash::from_payload("c");
        let ref_a = Reference::FilePath("/tmp/a".to_string());
        index
            .insert(key_a.clone(), ref_a.clone(), &mut emitter)
            .expect("insert");
        let map = index.get_many(&[key_b.clone(), key_a.clone(), key_c.clone()]);
        assert_eq!(map.get(&key_a).cloned().unwrap_or_default(), vec![ref_a]);
        assert!(map.get(&key_b).cloned().unwrap_or_default().is_empty());
        assert!(map.get(&key_c).cloned().unwrap_or_default().is_empty());
    }

    #[test]
    fn remove_deletes_existing_ref() {
        let mut index = HashIndex::new();
        let mut emitter = ReceiptEmitter::new(InMemoryReceiptSink::default());
        let key = Blake3Hash::from_payload("remove");
        let reference = Reference::Task(Uuid::new_v4());
        index
            .insert(key.clone(), reference.clone(), &mut emitter)
            .expect("insert");
        index
            .remove(&key, &reference, &mut emitter)
            .expect("remove");
        assert!(index.get(&key).is_empty());
    }

    #[test]
    fn remove_missing_ref_is_idempotent_no_op() {
        let mut index = HashIndex::new();
        let mut emitter = ReceiptEmitter::new(InMemoryReceiptSink::default());
        let key = Blake3Hash::from_payload("missing");
        let existing = Reference::Task(Uuid::new_v4());
        let missing = Reference::Receipt(Uuid::new_v4());
        index
            .insert(key.clone(), existing, &mut emitter)
            .expect("insert");
        let before = emitter.sink().receipts.len();
        index
            .remove(&key, &missing, &mut emitter)
            .expect("missing remove");
        assert_eq!(before, emitter.sink().receipts.len());
    }
}
