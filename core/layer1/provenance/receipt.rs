// SPDX-License-Identifier: Apache-2.0
// Plane ownership: Safety Plane provenance primitives.

use blake3::Hasher;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use thiserror::Error;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Receipt {
    pub id: Uuid,
    pub parent_id: Option<Uuid>,
    pub op_type: String,
    pub subject: Option<String>,
    pub payload_hash: String,
    pub actor: String,
    pub timestamp: DateTime<Utc>,
    pub confidence: Option<f32>,
}

pub struct ReceiptDraft<'a, T: Serialize> {
    pub parent_id: Option<Uuid>,
    pub op_type: &'a str,
    pub subject: Option<String>,
    pub payload: &'a T,
    pub actor: &'a str,
    pub confidence: Option<f32>,
}

#[derive(Debug, Error, PartialEq)]
pub enum ProvenanceError {
    #[error("invalid confidence value: {0}")]
    InvalidConfidence(String),
    #[error("payload serialization failed: {0}")]
    PayloadSerialization(String),
    #[error("sink append failed: {0}")]
    SinkAppend(String),
}

pub trait ReceiptSink {
    fn append(&mut self, receipt: &Receipt) -> Result<(), ProvenanceError>;
}

pub struct ReceiptEmitter<S: ReceiptSink> {
    sink: S,
}

impl<S: ReceiptSink> ReceiptEmitter<S> {
    pub fn new(sink: S) -> Self {
        Self { sink }
    }

    pub fn sink(&self) -> &S {
        &self.sink
    }

    pub fn sink_mut(&mut self) -> &mut S {
        &mut self.sink
    }

    pub fn into_sink(self) -> S {
        self.sink
    }

    pub fn emit<T: Serialize>(
        &mut self,
        draft: ReceiptDraft<'_, T>,
    ) -> Result<Receipt, ProvenanceError> {
        validate_confidence(draft.confidence)?;
        let payload_hash = hash_canonical_payload(draft.payload)?;
        let receipt = Receipt {
            id: Uuid::new_v4(),
            parent_id: draft.parent_id,
            op_type: draft.op_type.to_string(),
            subject: draft.subject,
            payload_hash,
            actor: draft.actor.to_string(),
            timestamp: Utc::now(),
            confidence: draft.confidence,
        };
        self.sink.append(&receipt)?;
        Ok(receipt)
    }
}

#[derive(Clone, Debug, Default)]
pub struct InMemoryReceiptSink {
    pub receipts: Vec<Receipt>,
    pub fail_append: bool,
}

impl InMemoryReceiptSink {
    pub fn with_failure() -> Self {
        Self {
            receipts: Vec::new(),
            fail_append: true,
        }
    }
}

impl ReceiptSink for InMemoryReceiptSink {
    fn append(&mut self, receipt: &Receipt) -> Result<(), ProvenanceError> {
        if self.fail_append {
            return Err(ProvenanceError::SinkAppend(
                "in_memory_sink_forced_failure".to_string(),
            ));
        }
        self.receipts.push(receipt.clone());
        Ok(())
    }
}

pub fn hash_canonical_payload<T: Serialize>(payload: &T) -> Result<String, ProvenanceError> {
    let canonical_bytes = canonical_payload_bytes(payload)?;
    Ok(blake3_hex(&canonical_bytes))
}

pub fn canonicalize_json_value(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut sorted_keys = map.keys().cloned().collect::<Vec<_>>();
            sorted_keys.sort();
            let mut out = Map::new();
            for key in sorted_keys {
                if let Some(inner) = map.get(&key) {
                    out.insert(key, canonicalize_json_value(inner));
                }
            }
            Value::Object(out)
        }
        Value::Array(values) => Value::Array(values.iter().map(canonicalize_json_value).collect()),
        _ => value.clone(),
    }
}

fn canonical_payload_bytes<T: Serialize>(payload: &T) -> Result<Vec<u8>, ProvenanceError> {
    let value = serde_json::to_value(payload)
        .map_err(|err| ProvenanceError::PayloadSerialization(err.to_string()))?;
    let canonical = canonicalize_json_value(&value);
    serde_json::to_vec(&canonical)
        .map_err(|err| ProvenanceError::PayloadSerialization(err.to_string()))
}

fn blake3_hex(bytes: &[u8]) -> String {
    let mut hasher = Hasher::new();
    hasher.update(bytes);
    hasher.finalize().to_hex().to_string()
}

fn validate_confidence(confidence: Option<f32>) -> Result<(), ProvenanceError> {
    if let Some(value) = confidence {
        if !value.is_finite() || !(0.0..=1.0).contains(&value) {
            return Err(ProvenanceError::InvalidConfidence(value.to_string()));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn identical_payloads_hash_identically() {
        let payload = json!({"a":1,"b":{"c":2}});
        let hash_a = hash_canonical_payload(&payload).expect("hash_a");
        let hash_b = hash_canonical_payload(&payload).expect("hash_b");
        assert_eq!(hash_a, hash_b);
    }

    #[test]
    fn canonicalization_sorts_nested_object_keys_recursively() {
        let input = json!({
            "z": {"k": 1, "a": {"x": 1, "b": 2}},
            "a": [{"d": 2, "c": 1}, {"b": 0, "a": 3}]
        });
        let canonical = canonicalize_json_value(&input);
        let encoded = serde_json::to_string(&canonical).expect("encode canonical");
        assert_eq!(
            encoded,
            r#"{"a":[{"c":1,"d":2},{"a":3,"b":0}],"z":{"a":{"b":2,"x":1},"k":1}}"#
        );
    }

    #[test]
    fn invalid_confidence_is_rejected() {
        let mut emitter = ReceiptEmitter::new(InMemoryReceiptSink::default());
        let err = emitter
            .emit(ReceiptDraft {
                parent_id: None,
                op_type: "mutate",
                subject: Some("subject".to_string()),
                payload: &json!({"x": 1}),
                actor: "tester",
                confidence: Some(1.2),
            })
            .expect_err("confidence should fail");
        assert!(matches!(err, ProvenanceError::InvalidConfidence(_)));
        assert!(emitter.sink().receipts.is_empty());
    }

    #[test]
    fn parent_id_is_preserved() {
        let parent = Uuid::new_v4();
        let mut emitter = ReceiptEmitter::new(InMemoryReceiptSink::default());
        let receipt = emitter
            .emit(ReceiptDraft {
                parent_id: Some(parent),
                op_type: "queue_enqueue",
                subject: Some("q1".to_string()),
                payload: &json!({"priority": 3}),
                actor: "queue",
                confidence: Some(0.9),
            })
            .expect("emit");
        assert_eq!(receipt.parent_id, Some(parent));
    }

    #[test]
    fn sink_append_failure_propagates() {
        let mut emitter = ReceiptEmitter::new(InMemoryReceiptSink::with_failure());
        let err = emitter
            .emit(ReceiptDraft {
                parent_id: None,
                op_type: "queue_enqueue",
                subject: None,
                payload: &json!({"task":"x"}),
                actor: "queue",
                confidence: None,
            })
            .expect_err("sink append failure");
        assert!(matches!(err, ProvenanceError::SinkAppend(_)));
    }

    #[test]
    fn emit_returns_the_same_receipt_that_was_appended() {
        let mut emitter = ReceiptEmitter::new(InMemoryReceiptSink::default());
        let receipt = emitter
            .emit(ReceiptDraft {
                parent_id: None,
                op_type: "hash_index_insert",
                subject: Some("hash".to_string()),
                payload: &json!({"hash":"abc"}),
                actor: "hash_index",
                confidence: Some(0.7),
            })
            .expect("emit");
        assert_eq!(emitter.sink().receipts.len(), 1);
        assert_eq!(receipt, emitter.sink().receipts[0]);
    }
}
