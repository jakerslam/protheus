use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MerkleReceiptOptions {
    pub enabled: bool,
    pub algorithm: String,
    pub seed: Option<String>,
}

impl Default for MerkleReceiptOptions {
    fn default() -> Self {
        Self {
            enabled: false,
            algorithm: "sha256".to_string(),
            seed: None,
        }
    }
}

pub fn merkle_receipt_options_from_value(raw: Option<&Value>) -> MerkleReceiptOptions {
    let mut out = MerkleReceiptOptions::default();
    let Some(value) = raw else {
        return out;
    };
    if let Some(flag) = value.get("enabled").and_then(Value::as_bool) {
        out.enabled = flag;
    }
    if let Some(text) = value.get("algorithm").and_then(Value::as_str) {
        let normalized = text.trim().to_ascii_lowercase();
        if !normalized.is_empty() {
            out.algorithm = normalized;
        }
    }
    if let Some(text) = value.get("seed").and_then(Value::as_str) {
        let normalized = text.trim().to_string();
        if !normalized.is_empty() {
            out.seed = Some(normalized);
        }
    }
    out
}

pub fn merkle_receipt_payload(
    receipt: &Value,
    previous_root: Option<&str>,
    options: &MerkleReceiptOptions,
) -> Value {
    if !options.enabled {
        return json!({
            "enabled": false,
            "algorithm": options.algorithm,
            "previous_root": previous_root,
            "root": Value::Null,
        });
    }
    let previous = previous_root.unwrap_or("");
    let canonical_receipt = serde_json::to_vec(receipt).unwrap_or_default();
    let seed = options.seed.clone().unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(options.algorithm.as_bytes());
    hasher.update(b":");
    hasher.update(seed.as_bytes());
    hasher.update(b":");
    hasher.update(previous.as_bytes());
    hasher.update(b":");
    hasher.update(&canonical_receipt);
    let digest = hex::encode(hasher.finalize());
    json!({
        "enabled": true,
        "algorithm": options.algorithm,
        "previous_root": if previous.is_empty() { Value::Null } else { Value::String(previous.to_string()) },
        "root": digest,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merkle_receipt_is_deterministic_for_same_inputs() {
        let receipt = json!({"type":"agent_run_receipt","ok":true});
        let options = MerkleReceiptOptions {
            enabled: true,
            algorithm: "sha256".to_string(),
            seed: Some("seed-a".to_string()),
        };
        let a = merkle_receipt_payload(&receipt, Some("prev"), &options);
        let b = merkle_receipt_payload(&receipt, Some("prev"), &options);
        assert_eq!(a.get("root"), b.get("root"));
    }

    #[test]
    fn merkle_receipt_changes_when_previous_root_changes() {
        let receipt = json!({"type":"agent_run_receipt","ok":true});
        let options = MerkleReceiptOptions {
            enabled: true,
            algorithm: "sha256".to_string(),
            seed: None,
        };
        let a = merkle_receipt_payload(&receipt, Some("a"), &options);
        let b = merkle_receipt_payload(&receipt, Some("b"), &options);
        assert_ne!(a.get("root"), b.get("root"));
    }
}
