// SPDX-License-Identifier: Apache-2.0
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use infring_vault_core_v1::{
    evaluate_vault_policy_json, load_embedded_vault_policy_json, VaultOperationRequest,
};
use std::env;
use std::fs;
use std::path::{Component, Path};

const MAX_ARG_KEY_LEN: usize = 48;
const MAX_REQUEST_BYTES: usize = 32 * 1024;
const MAX_TOKEN_LEN: usize = 128;
const MAX_PROOF_LEN: usize = 512;

fn strip_invisible_unicode(raw: &str) -> String {
    raw.chars()
        .filter(|ch| {
            !matches!(
                ch,
                '\u{200B}' | '\u{200C}' | '\u{200D}' | '\u{2060}' | '\u{FEFF}'
            )
        })
        .collect()
}

fn sanitize_text_token(raw: &str, max_len: usize) -> String {
    let mut token: String = strip_invisible_unicode(raw)
        .chars()
        .filter(|ch| !ch.is_control())
        .collect();
    token = token.trim().to_string();
    if token.chars().count() > max_len {
        token = token.chars().take(max_len).collect();
    }
    token
}

fn parse_arg(args: &[String], key: &str) -> Option<String> {
    let key = sanitize_text_token(key, MAX_ARG_KEY_LEN);
    for arg in args {
        if let Some((k, v)) = arg.split_once('=') {
            if sanitize_text_token(k, MAX_ARG_KEY_LEN) == key {
                let value = sanitize_text_token(v, MAX_REQUEST_BYTES);
                if !value.is_empty() {
                    return Some(value);
                }
            }
        }
    }
    None
}

fn is_safe_request_file_path(raw: &str) -> bool {
    let path = Path::new(raw);
    if raw.is_empty() || path.is_dir() {
        return false;
    }
    if path
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return false;
    }
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("json"))
        .unwrap_or(false)
}

fn valid_sha256_digest(raw: &str) -> bool {
    let token = raw.strip_prefix("sha256:").unwrap_or(raw);
    token.len() == 64 && token.chars().all(|ch| ch.is_ascii_hexdigit())
}

fn load_request_json(args: &[String]) -> Result<String, String> {
    if let Some(v) = parse_arg(args, "--request-json") {
        if v.len() > MAX_REQUEST_BYTES {
            return Err("request_json_too_large".to_string());
        }
        return Ok(v);
    }
    if let Some(v) = parse_arg(args, "--request-base64") {
        let bytes = BASE64_STANDARD
            .decode(v.as_bytes())
            .map_err(|err| format!("base64_decode_failed:{err}"))?;
        if bytes.len() > MAX_REQUEST_BYTES {
            return Err("request_base64_too_large".to_string());
        }
        let text = String::from_utf8(bytes).map_err(|err| format!("utf8_decode_failed:{err}"))?;
        return Ok(text);
    }
    if let Some(v) = parse_arg(args, "--request-file") {
        if !is_safe_request_file_path(&v) {
            return Err("request_file_path_invalid".to_string());
        }
        let metadata =
            fs::metadata(v.as_str()).map_err(|err| format!("request_file_stat_failed:{err}"))?;
        if !metadata.is_file() {
            return Err("request_file_not_a_file".to_string());
        }
        if metadata.len() > MAX_REQUEST_BYTES as u64 {
            return Err("request_file_too_large".to_string());
        }
        let text = fs::read_to_string(v.as_str())
            .map_err(|err| format!("request_file_read_failed:{err}"))?;
        if text.len() > MAX_REQUEST_BYTES {
            return Err("request_file_too_large".to_string());
        }
        return Ok(text);
    }
    Err("missing_request_payload".to_string())
}

fn normalize_request_json(raw: &str) -> Result<String, String> {
    let mut request: VaultOperationRequest =
        serde_json::from_str(raw).map_err(|err| format!("request_parse_failed:{err}"))?;
    request.operation_id = sanitize_text_token(&request.operation_id, MAX_TOKEN_LEN);
    request.key_id = sanitize_text_token(&request.key_id, MAX_TOKEN_LEN);
    request.action = sanitize_text_token(&request.action, 32).to_ascii_lowercase();
    request.zk_proof = request
        .zk_proof
        .as_ref()
        .map(|value| sanitize_text_token(value, MAX_PROOF_LEN))
        .filter(|value| !value.is_empty());
    request.ciphertext_digest = request
        .ciphertext_digest
        .as_ref()
        .map(|value| sanitize_text_token(value, MAX_PROOF_LEN))
        .filter(|value| !value.is_empty());
    request.audit_receipt_nonce = request
        .audit_receipt_nonce
        .as_ref()
        .map(|value| sanitize_text_token(value, MAX_TOKEN_LEN))
        .filter(|value| !value.is_empty());

    if request.operation_id.is_empty() {
        return Err("request_operation_id_invalid".to_string());
    }
    if request.key_id.is_empty() {
        return Err("request_key_id_invalid".to_string());
    }
    if request.action.is_empty() {
        return Err("request_action_invalid".to_string());
    }
    if !matches!(request.action.as_str(), "seal" | "unseal" | "rotate") {
        return Err("request_action_unsupported".to_string());
    }
    if request.operator_quorum == 0 || request.operator_quorum > 32 {
        return Err("request_operator_quorum_invalid".to_string());
    }
    if request.fhe_noise_budget > 4096 {
        return Err("request_fhe_noise_budget_out_of_bounds".to_string());
    }
    if request.key_age_hours > 24 * 365 * 20 {
        return Err("request_key_age_out_of_bounds".to_string());
    }
    if let Some(digest) = request.ciphertext_digest.as_ref() {
        if !valid_sha256_digest(digest) {
            return Err("request_ciphertext_digest_invalid".to_string());
        }
    }

    serde_json::to_string(&request).map_err(|err| format!("request_encode_failed:{err}"))
}

fn demo_request() -> VaultOperationRequest {
    VaultOperationRequest {
        operation_id: "vault_demo_001".to_string(),
        key_id: "vault_key_primary".to_string(),
        action: "seal".to_string(),
        zk_proof: Some("zkp:demo-proof".to_string()),
        ciphertext_digest: Some("sha256:demo-cipher".to_string()),
        fhe_noise_budget: 24,
        key_age_hours: 8,
        tamper_signal: false,
        operator_quorum: 2,
        audit_receipt_nonce: Some("nonce-demo".to_string()),
    }
}

fn usage() {
    eprintln!("Usage:");
    eprintln!("  vault_core load-policy");
    eprintln!("  vault_core evaluate --request-json=<payload>");
    eprintln!("  vault_core evaluate --request-base64=<base64_payload>");
    eprintln!("  vault_core evaluate --request-file=<path>");
    eprintln!("  vault_core demo");
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let command = args
        .first()
        .map(|value| sanitize_text_token(value, 24).to_ascii_lowercase())
        .unwrap_or_else(|| "demo".to_string());

    match command.as_str() {
        "load-policy" => match load_embedded_vault_policy_json() {
            Ok(payload) => println!("{}", payload),
            Err(err) => {
                eprintln!(
                    "{}",
                    serde_json::json!({
                        "ok": false,
                        "error": err.to_string()
                    })
                );
                std::process::exit(1);
            }
        },
        "evaluate" => match load_request_json(&args[1..]) {
            Ok(request_json) => match normalize_request_json(&request_json) {
                Ok(normalized_request_json) => {
                    match evaluate_vault_policy_json(&normalized_request_json) {
                        Ok(payload) => println!("{}", payload),
                        Err(err) => {
                            eprintln!(
                                "{}",
                                serde_json::json!({
                                    "ok": false,
                                    "error": err.to_string()
                                })
                            );
                            std::process::exit(1);
                        }
                    }
                }
                Err(err) => {
                    eprintln!(
                        "{}",
                        serde_json::json!({
                            "ok": false,
                            "error": err
                        })
                    );
                    std::process::exit(1);
                }
            },
            Err(err) => {
                eprintln!(
                    "{}",
                    serde_json::json!({
                        "ok": false,
                        "error": err
                    })
                );
                std::process::exit(1);
            }
        },
        "demo" => {
            let request = demo_request();
            let request_json = serde_json::to_string(&request).unwrap_or_else(|_| "{}".to_string());
            match evaluate_vault_policy_json(&request_json) {
                Ok(payload) => println!("{}", payload),
                Err(err) => {
                    eprintln!(
                        "{}",
                        serde_json::json!({
                            "ok": false,
                            "error": err.to_string()
                        })
                    );
                    std::process::exit(1);
                }
            }
        }
        _ => {
            usage();
            std::process::exit(1);
        }
    }
}
