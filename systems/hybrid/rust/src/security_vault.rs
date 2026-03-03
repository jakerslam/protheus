use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultKey {
    pub version: u32,
    pub fingerprint: String,
}

pub fn rotate_key(seed: &str, current_version: u32) -> VaultKey {
    let version = current_version.saturating_add(1);
    let mut h = Sha256::new();
    h.update(seed.as_bytes());
    h.update(version.to_le_bytes());
    let digest = format!("{:x}", h.finalize());
    VaultKey {
        version,
        fingerprint: digest,
    }
}

pub fn fail_closed_attestation(tampered: bool) -> bool {
    !tampered
}

pub fn sample_report() -> serde_json::Value {
    let key = rotate_key("vault-seed", 41);
    let pass = fail_closed_attestation(false);
    json!({
        "ok": true,
        "lane": "V5-RUST-HYB-004",
        "rotation": {
            "new_version": key.version,
            "fingerprint": key.fingerprint,
        },
        "attestation": {
            "tamper_detected": false,
            "allowed": pass,
            "mode": "fail_closed"
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotation_increments_version() {
        let k = rotate_key("x", 7);
        assert_eq!(k.version, 8);
    }

    #[test]
    fn fail_closed_denies_tamper() {
        assert!(!fail_closed_attestation(true));
        assert!(fail_closed_attestation(false));
    }
}
