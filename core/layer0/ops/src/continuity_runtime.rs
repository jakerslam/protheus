// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops::continuity_runtime (authoritative)
use crate::{client_state_root, core_state_root, deterministic_receipt_hash, now_iso};
use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use base64::engine::general_purpose::STANDARD as BASE64_STD;
use base64::Engine;
use rand::RngCore;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

const LANE_ID: &str = "continuity_runtime";

#[derive(Debug, Clone)]
struct ContinuityPolicy {
    max_state_bytes: usize,
    allow_degraded_restore: bool,
    allow_sessionless_resurrection: bool,
    require_vault_encryption: bool,
    vault_key_env: String,
}

fn usage() {
    println!("Usage:");
    println!(
        "  protheus-ops continuity-runtime resurrection-protocol <checkpoint|restore|status> [flags]"
    );
    println!("  protheus-ops continuity-runtime session-continuity-vault <put|get|status> [flags]");
}

fn print_json_line(value: &Value) {
    println!(
        "{}",
        serde_json::to_string(value)
            .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode_failed\"}".to_string())
    );
}

fn receipt_hash(v: &Value) -> String {
    deterministic_receipt_hash(v)
}

fn parse_flag(argv: &[String], key: &str) -> Option<String> {
    let with_eq = format!("--{key}=");
    let plain = format!("--{key}");
    let mut i = 0usize;
    while i < argv.len() {
        let token = argv[i].trim();
        if let Some(v) = token.strip_prefix(&with_eq) {
            return Some(v.trim().to_string());
        }
        if token == plain {
            if let Some(next) = argv.get(i + 1) {
                if !next.trim_start().starts_with("--") {
                    return Some(next.trim().to_string());
                }
            }
            return Some("true".to_string());
        }
        i += 1;
    }
    None
}

fn parse_bool(raw: Option<&str>, default: bool) -> bool {
    let Some(v) = raw else {
        return default;
    };
    match v.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => true,
        "0" | "false" | "no" | "off" => false,
        _ => default,
    }
}

fn clean_id(raw: Option<&str>, fallback: &str) -> String {
    let mut out = String::new();
    if let Some(v) = raw {
        for ch in v.trim().chars() {
            if out.len() >= 96 {
                break;
            }
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
                out.push(ch);
            } else {
                out.push('-');
            }
        }
    }
    let cleaned = out.trim_matches('-');
    if cleaned.is_empty() {
        fallback.to_string()
    } else {
        cleaned.to_string()
    }
}

fn parse_json(raw: Option<&str>) -> Result<Value, String> {
    let text = raw.ok_or_else(|| "missing_json_payload".to_string())?;
    serde_json::from_str::<Value>(text).map_err(|err| format!("invalid_json_payload:{err}"))
}

fn ensure_parent(path: &Path) -> Result<(), String> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    fs::create_dir_all(parent).map_err(|err| format!("mkdir_failed:{}:{err}", parent.display()))
}

fn write_json(path: &Path, payload: &Value) -> Result<(), String> {
    ensure_parent(path)?;
    let mut encoded =
        serde_json::to_string_pretty(payload).map_err(|err| format!("encode_failed:{err}"))?;
    encoded.push('\n');
    fs::write(path, encoded).map_err(|err| format!("write_failed:{}:{err}", path.display()))
}

fn append_jsonl(path: &Path, row: &Value) -> Result<(), String> {
    ensure_parent(path)?;
    let line = serde_json::to_string(row).map_err(|err| format!("encode_failed:{err}"))? + "\n";
    let mut opts = fs::OpenOptions::new();
    opts.create(true).append(true);
    use std::io::Write;
    let mut file = opts
        .open(path)
        .map_err(|err| format!("open_failed:{}:{err}", path.display()))?;
    file.write_all(line.as_bytes())
        .map_err(|err| format!("append_failed:{}:{err}", path.display()))
}

fn read_json(path: &Path) -> Option<Value> {
    let text = fs::read_to_string(path).ok()?;
    serde_json::from_str::<Value>(&text).ok()
}

fn continuity_dir(root: &Path) -> PathBuf {
    client_state_root(root).join("continuity")
}

fn checkpoints_dir(root: &Path) -> PathBuf {
    continuity_dir(root).join("checkpoints")
}

fn checkpoint_index_path(root: &Path) -> PathBuf {
    continuity_dir(root).join("checkpoint_index.json")
}

fn continuity_history_path(root: &Path) -> PathBuf {
    continuity_dir(root).join("history.jsonl")
}

fn continuity_restore_path(root: &Path) -> PathBuf {
    continuity_dir(root).join("restored").join("latest.json")
}

fn vault_dir(root: &Path) -> PathBuf {
    core_state_root(root).join("continuity").join("vault")
}

fn vault_history_path(root: &Path) -> PathBuf {
    core_state_root(root)
        .join("continuity")
        .join("vault_history.jsonl")
}

fn rel_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .ok()
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|| path.to_string_lossy().replace('\\', "/"))
}

fn default_policy() -> ContinuityPolicy {
    ContinuityPolicy {
        max_state_bytes: 512 * 1024,
        allow_degraded_restore: false,
        allow_sessionless_resurrection: true,
        require_vault_encryption: true,
        vault_key_env: "PROTHEUS_CONTINUITY_VAULT_KEY".to_string(),
    }
}

fn policy_path(root: &Path) -> PathBuf {
    root.join("client")
        .join("runtime")
        .join("config")
        .join("continuity_policy.json")
}

fn load_policy(root: &Path) -> ContinuityPolicy {
    let mut policy = default_policy();
    let path = policy_path(root);
    if let Some(v) = read_json(&path) {
        policy.max_state_bytes = v
            .get("max_state_bytes")
            .and_then(Value::as_u64)
            .map(|n| n as usize)
            .filter(|n| *n >= 256)
            .unwrap_or(policy.max_state_bytes);
        policy.allow_degraded_restore = v
            .get("allow_degraded_restore")
            .and_then(Value::as_bool)
            .unwrap_or(policy.allow_degraded_restore);
        policy.allow_sessionless_resurrection = v
            .get("allow_sessionless_resurrection")
            .and_then(Value::as_bool)
            .unwrap_or(policy.allow_sessionless_resurrection);
        policy.require_vault_encryption = v
            .get("require_vault_encryption")
            .and_then(Value::as_bool)
            .unwrap_or(policy.require_vault_encryption);
        policy.vault_key_env = v
            .get("vault_key_env")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or(policy.vault_key_env.as_str())
            .to_string();
    }
    policy
}

fn normalized_state(state: Value) -> Value {
    let mut map = state.as_object().cloned().unwrap_or_default();
    map.entry("attention_queue".to_string())
        .or_insert_with(|| Value::Array(Vec::new()));
    map.entry("memory_graph".to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    map.entry("active_personas".to_string())
        .or_insert_with(|| Value::Array(Vec::new()));
    Value::Object(map)
}

fn is_degraded_state(state: &Value) -> bool {
    let obj = state.as_object();
    let has_attention = obj
        .and_then(|m| m.get("attention_queue"))
        .and_then(Value::as_array);
    let has_graph = obj
        .and_then(|m| m.get("memory_graph"))
        .and_then(Value::as_object);
    let has_personas = obj
        .and_then(|m| m.get("active_personas"))
        .and_then(Value::as_array);
    has_attention.is_none() || has_graph.is_none() || has_personas.is_none()
}

fn checkpoint_index(root: &Path) -> BTreeMap<String, String> {
    let path = checkpoint_index_path(root);
    let mut out = BTreeMap::new();
    if let Some(v) = read_json(&path).and_then(|row| row.as_object().cloned()) {
        for (k, v) in v {
            if let Some(s) = v.as_str() {
                out.insert(k, s.to_string());
            }
        }
    }
    out
}

fn write_checkpoint_index(root: &Path, index: &BTreeMap<String, String>) -> Result<(), String> {
    let mut map = Map::new();
    for (k, v) in index {
        map.insert(k.clone(), Value::String(v.clone()));
    }
    write_json(&checkpoint_index_path(root), &Value::Object(map))
}

fn checkpoint_payload(
    root: &Path,
    policy: &ContinuityPolicy,
    argv: &[String],
) -> Result<Value, String> {
    let session_id = clean_id(parse_flag(argv, "session-id").as_deref(), "session-default");
    let state_raw = parse_flag(argv, "state-json")
        .map(|raw| parse_json(Some(raw.as_str())))
        .transpose()?
        .unwrap_or_else(|| {
            json!({
                "attention_queue": [],
                "memory_graph": {},
                "active_personas": []
            })
        });
    let state = normalized_state(state_raw);
    let state_encoded =
        serde_json::to_vec(&state).map_err(|err| format!("state_encode_failed:{err}"))?;
    if state_encoded.len() > policy.max_state_bytes {
        return Err(format!(
            "state_too_large:{}>{}",
            state_encoded.len(),
            policy.max_state_bytes
        ));
    }
    let degraded = is_degraded_state(&state);
    let apply = parse_bool(parse_flag(argv, "apply").as_deref(), true);
    let ts = now_iso();
    let checkpoint_name = format!(
        "{}_{}.json",
        session_id,
        ts.replace([':', '.'], "-")
            .replace('T', "_")
            .replace('Z', "")
    );
    let checkpoint_path = checkpoints_dir(root).join(checkpoint_name);
    let state_sha = hex::encode(Sha256::digest(&state_encoded));

    if apply {
        let row = json!({
            "session_id": session_id,
            "ts": ts,
            "state": state,
            "state_sha256": state_sha,
            "degraded": degraded,
            "lane": LANE_ID,
            "type": "continuity_checkpoint"
        });
        write_json(&checkpoint_path, &row)?;

        let mut index = checkpoint_index(root);
        index.insert(
            clean_id(Some(session_id.as_str()), "session-default"),
            rel_path(root, &checkpoint_path),
        );
        write_checkpoint_index(root, &index)?;
        append_jsonl(
            &continuity_history_path(root),
            &json!({
                "type": "continuity_checkpoint",
                "session_id": session_id,
                "path": rel_path(root, &checkpoint_path),
                "ts": ts,
                "state_sha256": state_sha,
                "degraded": degraded
            }),
        )?;
    }

    let mut out = json!({
        "ok": true,
        "type": "resurrection_protocol_checkpoint",
        "lane": LANE_ID,
        "session_id": session_id,
        "apply": apply,
        "degraded": degraded,
        "state_bytes": state_encoded.len(),
        "state_sha256": state_sha,
        "checkpoint_path": rel_path(root, &checkpoint_path),
        "policy": {
            "max_state_bytes": policy.max_state_bytes,
            "allow_degraded_restore": policy.allow_degraded_restore,
            "allow_sessionless_resurrection": policy.allow_sessionless_resurrection
        },
        "claim_evidence": [
            {
                "id": "checkpoint_with_deterministic_receipt",
                "claim": "session_state_is_checkpointed_with_integrity_hash",
                "evidence": {
                    "session_id": session_id,
                    "state_sha256": state_sha,
                    "checkpoint_path": rel_path(root, &checkpoint_path)
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(receipt_hash(&out));
    Ok(out)
}

fn restore_payload(
    root: &Path,
    policy: &ContinuityPolicy,
    argv: &[String],
) -> Result<Value, String> {
    let session_id = clean_id(parse_flag(argv, "session-id").as_deref(), "session-default");
    let allow_degraded = parse_bool(
        parse_flag(argv, "allow-degraded").as_deref(),
        policy.allow_degraded_restore,
    );
    let apply = parse_bool(parse_flag(argv, "apply").as_deref(), true);

    let checkpoint_path = if let Some(raw) = parse_flag(argv, "checkpoint-path") {
        let p = PathBuf::from(raw.trim());
        if p.is_absolute() {
            p
        } else {
            root.join(p)
        }
    } else {
        let index = checkpoint_index(root);
        match index.get(&session_id) {
            Some(rel) => root.join(rel),
            None => {
                if policy.allow_sessionless_resurrection {
                    let mut out = json!({
                        "ok": true,
                        "type": "resurrection_protocol_restore",
                        "lane": LANE_ID,
                        "session_id": session_id,
                        "sessionless": true,
                        "degraded": true,
                        "policy_gate": "allow_sessionless_resurrection",
                        "restored_state": {
                            "attention_queue": [],
                            "memory_graph": {},
                            "active_personas": []
                        }
                    });
                    out["receipt_hash"] = Value::String(receipt_hash(&out));
                    return Ok(out);
                }
                return Err("checkpoint_not_found".to_string());
            }
        }
    };

    let checkpoint = read_json(&checkpoint_path).ok_or_else(|| {
        format!(
            "checkpoint_missing:{}",
            rel_path(root, checkpoint_path.as_path())
        )
    })?;
    let state = checkpoint
        .get("state")
        .cloned()
        .ok_or_else(|| "checkpoint_state_missing".to_string())?;
    let degraded = is_degraded_state(&state);
    if degraded && !allow_degraded {
        return Err("degraded_restore_blocked_by_policy".to_string());
    }

    if apply {
        write_json(
            &continuity_restore_path(root),
            &json!({
                "session_id": session_id,
                "restored_at": now_iso(),
                "checkpoint_path": rel_path(root, &checkpoint_path),
                "degraded": degraded,
                "state": state
            }),
        )?;
    }

    let mut out = json!({
        "ok": true,
        "type": "resurrection_protocol_restore",
        "lane": LANE_ID,
        "session_id": session_id,
        "apply": apply,
        "checkpoint_path": rel_path(root, &checkpoint_path),
        "degraded": degraded,
        "allow_degraded": allow_degraded,
        "restored_state": state,
        "claim_evidence": [
            {
                "id": "restore_with_layer0_gate",
                "claim": "restore_fails_closed_on_degraded_state_unless_policy_allows",
                "evidence": {
                    "allow_degraded": allow_degraded,
                    "degraded": degraded,
                    "checkpoint_path": rel_path(root, &checkpoint_path)
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(receipt_hash(&out));
    Ok(out)
}

fn continuity_status_payload(root: &Path, policy: &ContinuityPolicy) -> Value {
    let index = checkpoint_index(root);
    let latest_restore = read_json(&continuity_restore_path(root));
    let mut out = json!({
        "ok": true,
        "type": "continuity_runtime_status",
        "lane": LANE_ID,
        "checkpoint_sessions": index.len(),
        "checkpoint_index": index,
        "latest_restore": latest_restore,
        "policy": {
            "max_state_bytes": policy.max_state_bytes,
            "allow_degraded_restore": policy.allow_degraded_restore,
            "allow_sessionless_resurrection": policy.allow_sessionless_resurrection,
            "require_vault_encryption": policy.require_vault_encryption,
            "vault_key_env": policy.vault_key_env
        }
    });
    out["receipt_hash"] = Value::String(receipt_hash(&out));
    out
}

fn derive_vault_key(secret: &str) -> [u8; 32] {
    let digest = Sha256::digest(secret.as_bytes());
    let mut key = [0u8; 32];
    key.copy_from_slice(&digest[..32]);
    key
}

fn encrypt_state(secret: &str, state: &[u8], aad: &[u8]) -> Result<(String, String), String> {
    let key_bytes = derive_vault_key(secret);
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);

    let mut nonce = [0u8; 12];
    rand::rngs::OsRng.fill_bytes(&mut nonce);
    let nonce_ref = Nonce::from_slice(&nonce);

    let encrypted = cipher
        .encrypt(nonce_ref, aes_gcm::aead::Payload { msg: state, aad })
        .map_err(|err| format!("encrypt_failed:{err}"))?;

    Ok((BASE64_STD.encode(nonce), BASE64_STD.encode(encrypted)))
}

fn decrypt_state(
    secret: &str,
    nonce_b64: &str,
    cipher_b64: &str,
    aad: &[u8],
) -> Result<Vec<u8>, String> {
    let key_bytes = derive_vault_key(secret);
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);

    let nonce_bytes = BASE64_STD
        .decode(nonce_b64.as_bytes())
        .map_err(|err| format!("nonce_decode_failed:{err}"))?;
    if nonce_bytes.len() != 12 {
        return Err("nonce_len_invalid".to_string());
    }
    let cipher_bytes = BASE64_STD
        .decode(cipher_b64.as_bytes())
        .map_err(|err| format!("cipher_decode_failed:{err}"))?;
    let nonce_ref = Nonce::from_slice(&nonce_bytes);
    cipher
        .decrypt(
            nonce_ref,
            aes_gcm::aead::Payload {
                msg: &cipher_bytes,
                aad,
            },
        )
        .map_err(|err| format!("decrypt_failed:{err}"))
}

fn vault_put_payload(
    root: &Path,
    policy: &ContinuityPolicy,
    argv: &[String],
) -> Result<Value, String> {
    let session_id = clean_id(parse_flag(argv, "session-id").as_deref(), "session-default");
    let state = parse_json(parse_flag(argv, "state-json").as_deref())?;
    let encoded = serde_json::to_vec(&state).map_err(|err| format!("state_encode_failed:{err}"))?;
    if encoded.len() > policy.max_state_bytes {
        return Err(format!(
            "state_too_large:{}>{}",
            encoded.len(),
            policy.max_state_bytes
        ));
    }

    let key_env = parse_flag(argv, "vault-key-env").unwrap_or_else(|| policy.vault_key_env.clone());
    let vault_key = std::env::var(&key_env).unwrap_or_default();
    if policy.require_vault_encryption && vault_key.trim().is_empty() {
        return Err(format!("vault_key_missing_env:{key_env}"));
    }

    let aad = format!("{LANE_ID}:{session_id}");
    let (nonce_b64, cipher_b64) = if vault_key.trim().is_empty() {
        (String::new(), BASE64_STD.encode(encoded.as_slice()))
    } else {
        encrypt_state(vault_key.trim(), &encoded, aad.as_bytes())?
    };

    let apply = parse_bool(parse_flag(argv, "apply").as_deref(), true);
    let vault_path = vault_dir(root).join(format!("{}.json", session_id));
    let ciphertext_sha = hex::encode(Sha256::digest(cipher_b64.as_bytes()));

    if apply {
        write_json(
            &vault_path,
            &json!({
                "session_id": session_id,
                "updated_at": now_iso(),
                "lane": LANE_ID,
                "encryption": {
                    "algo": if vault_key.trim().is_empty() { "base64-plain" } else { "aes-256-gcm" },
                    "key_env": key_env,
                    "aad": aad,
                    "nonce_b64": nonce_b64
                },
                "ciphertext_b64": cipher_b64,
                "ciphertext_sha256": ciphertext_sha
            }),
        )?;
        append_jsonl(
            &vault_history_path(root),
            &json!({
                "type": "session_continuity_vault_put",
                "session_id": session_id,
                "ts": now_iso(),
                "vault_path": rel_path(root, &vault_path),
                "ciphertext_sha256": ciphertext_sha
            }),
        )?;
    }

    let mut out = json!({
        "ok": true,
        "type": "session_continuity_vault_put",
        "lane": LANE_ID,
        "session_id": session_id,
        "apply": apply,
        "vault_path": rel_path(root, &vault_path),
        "ciphertext_sha256": ciphertext_sha,
        "encrypted": !vault_key.trim().is_empty(),
        "claim_evidence": [
            {
                "id": "vault_encrypted_at_rest",
                "claim": "session_state_is_stored_with_cryptographic_envelope",
                "evidence": {
                    "encrypted": !vault_key.trim().is_empty(),
                    "vault_path": rel_path(root, &vault_path),
                    "key_env": key_env
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(receipt_hash(&out));
    Ok(out)
}

fn vault_get_payload(
    root: &Path,
    policy: &ContinuityPolicy,
    argv: &[String],
) -> Result<Value, String> {
    let session_id = clean_id(parse_flag(argv, "session-id").as_deref(), "session-default");
    let emit_state = parse_bool(parse_flag(argv, "emit-state").as_deref(), false);
    let vault_path = vault_dir(root).join(format!("{}.json", session_id));
    let record = read_json(&vault_path).ok_or_else(|| "vault_record_missing".to_string())?;

    let encryption = record
        .get("encryption")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let algo = encryption
        .get("algo")
        .and_then(Value::as_str)
        .unwrap_or("aes-256-gcm")
        .to_string();
    let key_env = encryption
        .get("key_env")
        .and_then(Value::as_str)
        .unwrap_or(policy.vault_key_env.as_str())
        .to_string();
    let aad = encryption
        .get("aad")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let nonce_b64 = encryption
        .get("nonce_b64")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let cipher_b64 = record
        .get("ciphertext_b64")
        .and_then(Value::as_str)
        .ok_or_else(|| "vault_ciphertext_missing".to_string())?
        .to_string();

    let decoded = if algo == "base64-plain" {
        BASE64_STD
            .decode(cipher_b64.as_bytes())
            .map_err(|err| format!("cipher_decode_failed:{err}"))?
    } else {
        let vault_key = std::env::var(&key_env).unwrap_or_default();
        if vault_key.trim().is_empty() {
            return Err(format!("vault_key_missing_env:{key_env}"));
        }
        decrypt_state(vault_key.trim(), &nonce_b64, &cipher_b64, aad.as_bytes())?
    };

    let state: Value =
        serde_json::from_slice(&decoded).map_err(|err| format!("state_decode_failed:{err}"))?;

    let mut out = json!({
        "ok": true,
        "type": "session_continuity_vault_get",
        "lane": LANE_ID,
        "session_id": session_id,
        "vault_path": rel_path(root, &vault_path),
        "encrypted": algo != "base64-plain",
        "state_sha256": hex::encode(Sha256::digest(&decoded)),
        "state_summary": {
            "attention_items": state.get("attention_queue").and_then(Value::as_array).map(|r| r.len()).unwrap_or(0),
            "memory_nodes": state.get("memory_graph").and_then(Value::as_object).map(|r| r.len()).unwrap_or(0),
            "active_personas": state.get("active_personas").and_then(Value::as_array).map(|r| r.len()).unwrap_or(0),
        }
    });
    if emit_state {
        out["state"] = state;
    }
    out["receipt_hash"] = Value::String(receipt_hash(&out));
    Ok(out)
}

fn vault_status_payload(root: &Path, policy: &ContinuityPolicy) -> Value {
    let dir = vault_dir(root);
    let mut records = 0usize;
    if let Ok(read) = fs::read_dir(&dir) {
        for entry in read.flatten() {
            if entry.path().extension().and_then(|s| s.to_str()) == Some("json") {
                records += 1;
            }
        }
    }
    let mut out = json!({
        "ok": true,
        "type": "session_continuity_vault_status",
        "lane": LANE_ID,
        "vault_dir": rel_path(root, &dir),
        "record_count": records,
        "history_path": rel_path(root, &vault_history_path(root)),
        "policy": {
            "require_vault_encryption": policy.require_vault_encryption,
            "vault_key_env": policy.vault_key_env
        }
    });
    out["receipt_hash"] = Value::String(receipt_hash(&out));
    out
}

fn cli_error(argv: &[String], err: &str, exit_code: i32) -> Value {
    let mut out = json!({
        "ok": false,
        "type": "continuity_runtime_cli_error",
        "lane": LANE_ID,
        "ts": now_iso(),
        "argv": argv,
        "error": err,
        "exit_code": exit_code,
    });
    out["receipt_hash"] = Value::String(receipt_hash(&out));
    out
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    if argv.is_empty() {
        usage();
        print_json_line(&cli_error(argv, "missing_surface", 2));
        return 2;
    }

    let policy = load_policy(root);
    let surface = argv[0].trim().to_ascii_lowercase();
    let command = argv
        .get(1)
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());

    let result = match (surface.as_str(), command.as_str()) {
        ("resurrection-protocol", "checkpoint")
        | ("resurrection-protocol", "bundle")
        | ("resurrection-protocol", "run")
        | ("resurrection-protocol", "build") => checkpoint_payload(root, &policy, &argv[2..]),
        ("resurrection-protocol", "restore") => restore_payload(root, &policy, &argv[2..]),
        ("resurrection-protocol", "status") | ("resurrection-protocol", "verify") => {
            Ok(continuity_status_payload(root, &policy))
        }
        ("session-continuity-vault", "put") | ("session-continuity-vault", "archive") => {
            vault_put_payload(root, &policy, &argv[2..])
        }
        ("session-continuity-vault", "get") | ("session-continuity-vault", "restore") => {
            vault_get_payload(root, &policy, &argv[2..])
        }
        ("session-continuity-vault", "status") | ("session-continuity-vault", "verify") => {
            Ok(vault_status_payload(root, &policy))
        }
        _ => Err("unknown_command".to_string()),
    };

    match result {
        Ok(payload) => {
            let ok = payload.get("ok").and_then(Value::as_bool).unwrap_or(false);
            print_json_line(&payload);
            if ok {
                0
            } else {
                1
            }
        }
        Err(err) => {
            if err == "unknown_command" {
                usage();
            }
            print_json_line(&cli_error(argv, &err, 2));
            2
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn root() -> tempfile::TempDir {
        tempfile::tempdir().expect("tempdir")
    }

    #[test]
    fn checkpoint_and_restore_roundtrip() {
        let dir = root();
        let checkpoint = checkpoint_payload(
            dir.path(),
            &default_policy(),
            &[
                "--session-id=session-a".to_string(),
                "--state-json={\"attention_queue\":[\"a\"],\"memory_graph\":{\"n1\":{}},\"active_personas\":[\"planner\"]}".to_string(),
                "--apply=1".to_string(),
            ],
        )
        .expect("checkpoint");
        assert!(checkpoint
            .get("ok")
            .and_then(Value::as_bool)
            .unwrap_or(false));

        let restored = restore_payload(
            dir.path(),
            &default_policy(),
            &[
                "--session-id=session-a".to_string(),
                "--apply=1".to_string(),
            ],
        )
        .expect("restore");
        assert!(restored.get("ok").and_then(Value::as_bool).unwrap_or(false));
        assert_eq!(
            restored
                .get("restored_state")
                .and_then(|v| v.get("active_personas"))
                .and_then(Value::as_array)
                .map(|rows| rows.len()),
            Some(1)
        );
    }

    #[test]
    fn degraded_restore_is_blocked_without_override() {
        let dir = root();
        let policy = default_policy();
        let ckpt_path = checkpoints_dir(dir.path()).join("s1_manual_degraded.json");
        write_json(
            &ckpt_path,
            &json!({
                "session_id": "s1",
                "ts": now_iso(),
                "state": { "attention_queue": ["a"] },
                "degraded": true
            }),
        )
        .expect("write degraded checkpoint");
        let mut index = BTreeMap::new();
        index.insert("s1".to_string(), rel_path(dir.path(), &ckpt_path));
        write_checkpoint_index(dir.path(), &index).expect("write index");

        let err = restore_payload(
            dir.path(),
            &policy,
            &["--session-id=s1".to_string(), "--apply=0".to_string()],
        )
        .expect_err("blocked");
        assert!(err.contains("degraded_restore_blocked_by_policy"));
    }

    #[test]
    fn vault_encrypts_and_decrypts_state() {
        let dir = root();
        let policy = default_policy();
        std::env::set_var("TEST_CONTINUITY_KEY", "s3cr3t");

        let put = vault_put_payload(
            dir.path(),
            &ContinuityPolicy {
                vault_key_env: "TEST_CONTINUITY_KEY".to_string(),
                ..policy.clone()
            },
            &[
                "--session-id=s2".to_string(),
                "--state-json={\"attention_queue\":[\"a\"],\"memory_graph\":{},\"active_personas\":[]}".to_string(),
                "--apply=1".to_string(),
            ],
        )
        .expect("vault put");
        assert!(put
            .get("encrypted")
            .and_then(Value::as_bool)
            .unwrap_or(false));

        let get = vault_get_payload(
            dir.path(),
            &ContinuityPolicy {
                vault_key_env: "TEST_CONTINUITY_KEY".to_string(),
                ..policy
            },
            &["--session-id=s2".to_string(), "--emit-state=1".to_string()],
        )
        .expect("vault get");

        assert_eq!(
            get.get("state")
                .and_then(|v| v.get("attention_queue"))
                .and_then(Value::as_array)
                .map(|rows| rows.len()),
            Some(1)
        );

        std::env::remove_var("TEST_CONTINUITY_KEY");
    }
}
