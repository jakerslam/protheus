use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use rand::RngCore;
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, Default)]
pub struct DbIndexEntry {
    pub node_id: String,
    pub uid: String,
    pub file_rel: String,
    pub summary: String,
    pub tags: Vec<String>,
    pub kind: String,
}

pub struct MemoryDb {
    conn: Connection,
    db_path: PathBuf,
    cipher_key: [u8; 32],
}

#[derive(Clone, Debug, Default)]
pub struct HotStateEnvelopeStats {
    pub total_rows: usize,
    pub enveloped_rows: usize,
    pub legacy_cipher_rows: usize,
    pub plain_rows: usize,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct DbFragmentationStats {
    pub page_count: u64,
    pub free_pages: u64,
    pub fragmentation_ratio: f64,
    pub physical_fragmentation_ratio: f64,
    pub tier_fragmentation_ratio: f64,
    pub db_file_bytes: u64,
    pub working_rows: u64,
    pub episodic_rows: u64,
    pub semantic_rows: u64,
    pub procedural_rows: u64,
}

const HOT_STATE_ENVELOPE_SCHEMA_ID: &str = "organ_state_envelope";
const HOT_STATE_ENVELOPE_SCHEMA_VERSION: &str = "1.0";

fn now_iso() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|dur| dur.as_secs())
        .unwrap_or(0);
    format!("{secs}")
}

fn default_db_path(root: &Path) -> PathBuf {
    root.join("local")
        .join("state")
        .join("memory")
        .join("runtime_memory.sqlite")
}

fn resolve_db_path(root: &Path, raw: &str) -> PathBuf {
    let v = raw.trim();
    if v.is_empty() {
        return default_db_path(root);
    }
    let p = PathBuf::from(v);
    if p.is_absolute() {
        p
    } else {
        root.join(p)
    }
}

fn derive_key_material(root: &Path) -> String {
    if let Ok(v) = env::var("PROTHEUS_MEMORY_DB_KEY") {
        let trimmed = v.trim().to_string();
        if !trimmed.is_empty() {
            return trimmed;
        }
    }
    let keyring_path = root
        .join("local")
        .join("state")
        .join("security")
        .join("organ_state_encryption")
        .join("keyring.json");
    if let Ok(raw) = fs::read_to_string(&keyring_path) {
        if !raw.trim().is_empty() {
            return raw;
        }
    }
    format!("fallback:{}:memory-runtime-db", root.to_string_lossy())
}

fn derive_cipher_key(root: &Path) -> [u8; 32] {
    let material = derive_key_material(root);
    let mut hasher = Sha256::new();
    hasher.update(material.as_bytes());
    let digest = hasher.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&digest[..32]);
    out
}

fn legacy_keystream_block(cipher_key: &[u8; 32], nonce: u64, block_index: u64) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(cipher_key);
    hasher.update(nonce.to_le_bytes());
    hasher.update(block_index.to_le_bytes());
    let digest = hasher.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&digest[..32]);
    out
}

fn legacy_xor_stream(cipher_key: &[u8; 32], nonce: u64, input: &[u8]) -> Vec<u8> {
    let mut out = vec![0u8; input.len()];
    let mut offset = 0usize;
    let mut block_index = 0u64;
    while offset < input.len() {
        let block = legacy_keystream_block(cipher_key, nonce, block_index);
        let mut local = 0usize;
        while local < block.len() && (offset + local) < input.len() {
            out[offset + local] = input[offset + local] ^ block[local];
            local += 1;
        }
        offset += block.len();
        block_index += 1;
    }
    out
}

fn encrypt_value(cipher_key: &[u8; 32], plaintext: &str) -> Result<String, String> {
    let cipher =
        Aes256Gcm::new_from_slice(cipher_key).map_err(|err| format!("aead_init_failed:{err}"))?;
    let mut nonce_bytes = [0u8; 12];
    rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|err| format!("aead_encrypt_failed:{err}"))?;
    Ok(format!(
        "aead-v1:{}:{}",
        hex::encode(nonce_bytes),
        hex::encode(ciphertext)
    ))
}

fn decrypt_legacy_value(cipher_key: &[u8; 32], payload: &str) -> Result<String, String> {
    let body = payload.trim();
    if !body.starts_with("enc-v1:") {
        return Ok(body.to_string());
    }
    let parts = body.splitn(3, ':').collect::<Vec<&str>>();
    if parts.len() != 3 {
        return Err("legacy_cipher_invalid_parts".to_string());
    }
    let nonce = u64::from_str_radix(parts[1], 16).unwrap_or(0);
    let Ok(cipher_bytes) = hex::decode(parts[2]) else {
        return Err("legacy_cipher_invalid_hex".to_string());
    };
    let plain = legacy_xor_stream(cipher_key, nonce, &cipher_bytes);
    String::from_utf8(plain).map_err(|_| "legacy_cipher_invalid_utf8".to_string())
}

fn decrypt_value(cipher_key: &[u8; 32], payload: &str) -> Result<String, String> {
    let body = payload.trim();
    if body.starts_with("enc-v1:") {
        return Err("legacy_cipher_retired".to_string());
    }
    if !body.starts_with("aead-v1:") {
        return Err("legacy_plaintext_retired".to_string());
    }
    let parts = body.splitn(3, ':').collect::<Vec<&str>>();
    if parts.len() != 3 {
        return Err("aead_invalid_parts".to_string());
    }
    let nonce_bytes = hex::decode(parts[1]).map_err(|_| "aead_invalid_nonce_hex".to_string())?;
    if nonce_bytes.len() != 12 {
        return Err("aead_invalid_nonce_len".to_string());
    }
    let cipher_bytes = hex::decode(parts[2]).map_err(|_| "aead_invalid_cipher_hex".to_string())?;
    let cipher =
        Aes256Gcm::new_from_slice(cipher_key).map_err(|err| format!("aead_init_failed:{err}"))?;
    let nonce = Nonce::from_slice(&nonce_bytes);
    let plaintext = cipher
        .decrypt(nonce, cipher_bytes.as_ref())
        .map_err(|_| "aead_decrypt_failed".to_string())?;
    String::from_utf8(plaintext).map_err(|_| "aead_invalid_utf8".to_string())
}

fn hot_state_key_ref(cipher_key: &[u8; 32]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(cipher_key);
    let digest = hasher.finalize();
    format!("organ_state_encryption:key_{}", hex::encode(&digest[..8]))
}

fn parse_hot_state_envelope_ciphertext(payload: &str) -> Option<String> {
    let parsed = serde_json::from_str::<Value>(payload).ok()?;
    let schema_id = parsed
        .get("schema_id")
        .and_then(Value::as_str)
        .unwrap_or("");
    if schema_id != HOT_STATE_ENVELOPE_SCHEMA_ID {
        return None;
    }
    let lane = parsed.get("lane").and_then(Value::as_str).unwrap_or("");
    if lane != "hot_state" {
        return None;
    }
    parsed
        .get("ciphertext")
        .and_then(Value::as_str)
        .map(|s| s.to_string())
}

fn wrap_hot_state_envelope(cipher_key: &[u8; 32], plaintext: &str) -> Result<String, String> {
    let ciphertext = encrypt_value(cipher_key, plaintext)?;
    let envelope = json!({
        "schema_id": HOT_STATE_ENVELOPE_SCHEMA_ID,
        "schema_version": HOT_STATE_ENVELOPE_SCHEMA_VERSION,
        "organ": "memory",
        "lane": "hot_state",
        "algorithm": "aes256_gcm",
        "key_ref": hot_state_key_ref(cipher_key),
        "wrapped_at": now_iso(),
        "ciphertext": ciphertext
    });
    serde_json::to_string(&envelope)
        .map_err(|err| format!("hot_state_envelope_encode_failed:{err}"))
}

fn decode_hot_state_payload_for_migration(
    cipher_key: &[u8; 32],
    payload: &str,
) -> Result<Option<String>, String> {
    let body = payload.trim();
    if body.is_empty() {
        return Ok(Some(String::new()));
    }
    if parse_hot_state_envelope_ciphertext(body).is_some() {
        return Ok(None);
    }
    if body.starts_with("aead-v1:") {
        let plain = decrypt_value(cipher_key, body)?;
        return Ok(Some(plain));
    }
    if body.starts_with("enc-v1:") {
        let plain = decrypt_legacy_value(cipher_key, body)?;
        return Ok(Some(plain));
    }
    Ok(Some(body.to_string()))
}

fn decrypt_hot_state_envelope(cipher_key: &[u8; 32], payload: &str) -> Result<String, String> {
    let body = payload.trim();
    let Some(ciphertext) = parse_hot_state_envelope_ciphertext(body) else {
        return Err("hot_state_envelope_required".to_string());
    };
    decrypt_value(cipher_key, &ciphertext)
}

fn parse_tags_json(raw: &str) -> Vec<String> {
    let parsed = serde_json::from_str::<Vec<String>>(raw).unwrap_or_default();
    let mut out = parsed
        .into_iter()
        .filter(|tag| !tag.trim().is_empty())
        .collect::<Vec<String>>();
    out.sort();
    out.dedup();
    out
}

fn normalize_vector(values: &[f32]) -> Vec<f32> {
    if values.is_empty() {
        return vec![];
    }
    let mut out = values
        .iter()
        .map(|value| if value.is_finite() { *value } else { 0.0f32 })
        .collect::<Vec<f32>>();
    let norm = out
        .iter()
        .fold(0.0f32, |acc, value| acc + (*value * *value))
        .sqrt();
    if norm > 0.0 {
        for value in out.iter_mut() {
            *value /= norm;
        }
    }
    out
}

fn encode_vector_blob(values: &[f32]) -> Result<Vec<u8>, String> {
    let normalized = normalize_vector(values);
    serde_json::to_vec(&normalized).map_err(|err| format!("db_vector_encode_failed:{err}"))
}

fn decode_vector_blob(blob: &[u8]) -> Result<Vec<f32>, String> {
    let parsed = serde_json::from_slice::<Vec<f32>>(blob)
        .map_err(|err| format!("db_vector_decode_failed:{err}"))?;
    Ok(normalize_vector(&parsed))
}
