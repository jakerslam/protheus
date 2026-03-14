// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops::top1_assurance (authoritative)

#![cfg(kani)]

use crate::{clean, deterministic_receipt_hash};
use serde_json::json;

#[kani::proof]
fn prove_receipt_hash_is_deterministic_for_same_payload() {
    let payload = json!({
        "ok": true,
        "lane": "core/layer0/ops",
        "strict": true,
        "type": "proof_fixture"
    });
    let left = deterministic_receipt_hash(&payload);
    let right = deterministic_receipt_hash(&payload);
    assert_eq!(left, right);
}

#[kani::proof]
fn prove_receipt_hash_is_invariant_to_object_key_order() {
    let canonical = json!({
        "lane": "core/layer0/ops",
        "ok": true,
        "strict": true,
        "type": "proof_fixture"
    });
    let permuted = json!({
        "type": "proof_fixture",
        "strict": true,
        "ok": true,
        "lane": "core/layer0/ops"
    });
    assert_eq!(
        deterministic_receipt_hash(&canonical),
        deterministic_receipt_hash(&permuted)
    );
}

#[kani::proof]
fn prove_clean_respects_max_len_bound() {
    let bytes: [u8; 8] = kani::any();
    let raw = String::from_utf8_lossy(&bytes).to_string();
    let max_len = 4usize;
    let cleaned = clean(raw, max_len);
    assert!(cleaned.chars().count() <= max_len);
}
