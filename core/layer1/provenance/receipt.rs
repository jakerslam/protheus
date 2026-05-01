// SPDX-License-Identifier: Apache-2.0
// Plane ownership: Safety Plane provenance primitives.

pub use infring_types::{
    canonicalize_json_value, hash_canonical_payload, InMemoryReceiptSink, ProvenanceError, Receipt,
    ReceiptDraft, ReceiptEmitter, ReceiptSink,
};

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use uuid::Uuid;

    #[test]
    fn provenance_reexports_foundation_receipt_contract() {
        let parent = Uuid::new_v4();
        let mut emitter = ReceiptEmitter::new(InMemoryReceiptSink::default());
        let receipt = emitter
            .emit(ReceiptDraft {
                parent_id: Some(parent),
                op_type: "provenance_contract_reexport",
                subject: Some("receipt_contract".to_string()),
                payload: &json!({"through":"infring_types"}),
                actor: "provenance",
                confidence: Some(1.0),
            })
            .expect("emit");
        assert_eq!(receipt.parent_id, Some(parent));
        assert_eq!(emitter.sink().receipts, vec![receipt]);
    }
}
