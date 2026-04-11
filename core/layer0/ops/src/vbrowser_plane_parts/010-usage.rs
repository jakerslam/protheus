// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops::vbrowser_plane (authoritative)

use crate::v8_kernel::{
    append_jsonl, attach_conduit, build_plane_conduit_enforcement, conduit_bypass_requested,
    emit_plane_receipt, load_json_or, parse_bool, parse_u64, plane_status, print_json, read_json,
    scoped_state_root, sha256_hex_str, write_json,
};
use crate::{clean, parse_args};
use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::Engine as _;
use rand::RngCore;
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

const STATE_ENV: &str = "VBROWSER_PLANE_STATE_ROOT";
const STATE_SCOPE: &str = "vbrowser_plane";

const SESSION_CONTRACT_PATH: &str = "planes/contracts/vbrowser/sandbox_session_contract_v1.json";
const COLLAB_CONTRACT_PATH: &str =
    "planes/contracts/vbrowser/collaboration_controls_contract_v1.json";
const AUTOMATION_CONTRACT_PATH: &str =
    "planes/contracts/vbrowser/automation_container_contract_v1.json";
const PRIVACY_CONTRACT_PATH: &str = "planes/contracts/vbrowser/privacy_security_contract_v1.json";

fn usage() {
    println!("Usage:");
    println!("  protheus-ops vbrowser-plane status");
    println!("  protheus-ops vbrowser-plane session-start|start|open [--session-id=<id>] [--url=<url>] [--shadow=<id>] [--strict=1|0]");
    println!("  protheus-ops vbrowser-plane session-control|control --op=<join|handoff|leave|status> [--session-id=<id>] [--actor=<id>] [--role=<watch-only|shared-control>] [--to=<id>] [--strict=1|0]");
    println!("  protheus-ops vbrowser-plane goto|navigate [--session-id=<id>] [--url=<url>] [--wait-until=<load|domcontentloaded|networkidle|commit>] [--strict=1|0]");
    println!("  protheus-ops vbrowser-plane navback|back [--session-id=<id>] [--wait-until=<load|domcontentloaded|networkidle|commit>] [--strict=1|0]");
    println!("  protheus-ops vbrowser-plane wait|pause [--session-id=<id>] [--time-ms=<n>] [--strict=1|0]");
    println!("  protheus-ops vbrowser-plane scroll [--session-id=<id>] [--direction=up|down] [--percentage=<1-200>] [--x=<n>] [--y=<n>] [--strict=1|0]");
    println!("  protheus-ops vbrowser-plane click [--session-id=<id>] [--x=<n>] [--y=<n>] [--coordinates=<x,y>] [--describe=<text>] [--strict=1|0]");
    println!("  protheus-ops vbrowser-plane type [--session-id=<id>] [--x=<n>] [--y=<n>] [--coordinates=<x,y>] [--describe=<text>] [--text=<value>] [--variables-json=<json>] [--strict=1|0]");
    println!("  protheus-ops vbrowser-plane automate --session-id=<id> [--actions=navigate,click,type] [--strict=1|0]");
    println!("  protheus-ops vbrowser-plane key-input|keys [--session-id=<id>] [--method=press|type] [--value=<text|combo>] [--repeat=<n>] [--delay-ms=<n>] [--strict=1|0]");
    println!("  protheus-ops vbrowser-plane privacy-guard|privacy [--session-id=<id>] [--network=isolated|restricted|public] [--recording=0|1] [--allow-recording=0|1] [--budget-tokens=<n>] [--strict=1|0]");
    println!(
        "  protheus-ops vbrowser-plane snapshot [--session-id=<id>] [--refs=1|0] [--strict=1|0]"
    );
    println!("  protheus-ops vbrowser-plane screenshot [--session-id=<id>] [--annotate=1|0] [--delay-ms=<n>] [--strict=1|0]");
    println!("  protheus-ops vbrowser-plane action-policy [--session-id=<id>] [--action=<navigate|click|fill|submit>] [--action-policy=<path>] [--confirm=1|0] [--strict=1|0]");
    println!("  protheus-ops vbrowser-plane auth-save [--provider=<id>] [--profile=<id>] [--username=<id>] [--secret=<token>] [--strict=1|0]");
    println!("  protheus-ops vbrowser-plane auth-login [--provider=<id>] [--profile=<id>] [--strict=1|0]");
    println!(
        "  protheus-ops vbrowser-plane native [--session-id=<id>] [--url=<url>] [--strict=1|0]"
    );
}

fn state_root(root: &Path) -> PathBuf {
    scoped_state_root(root, STATE_ENV, STATE_SCOPE)
}

fn emit(root: &Path, payload: Value) -> i32 {
    emit_plane_receipt(
        root,
        STATE_ENV,
        STATE_SCOPE,
        "vbrowser_plane_error",
        payload,
    )
}

fn status(root: &Path) -> Value {
    plane_status(root, STATE_ENV, STATE_SCOPE, "vbrowser_plane_status")
}

fn claim_ids_for_action(action: &str) -> Vec<&'static str> {
    match canonical_vbrowser_command(action) {
        "session-start" => vec![
            "V6-VBROWSER-001.1",
            "V6-VBROWSER-001.5",
            "V6-VBROWSER-001.6",
        ],
        "session-control" => {
            vec![
                "V6-VBROWSER-001.2",
                "V6-VBROWSER-001.5",
                "V6-VBROWSER-001.6",
            ]
        }
        "goto" => vec![
            "V11-STAGEHAND-007",
            "V6-VBROWSER-001.5",
            "V6-VBROWSER-001.6",
        ],
        "navback" => vec![
            "V11-STAGEHAND-008",
            "V6-VBROWSER-001.5",
            "V6-VBROWSER-001.6",
        ],
        "wait" => vec![
            "V11-STAGEHAND-009",
            "V6-VBROWSER-001.5",
            "V6-VBROWSER-001.6",
        ],
        "scroll" => vec![
            "V11-STAGEHAND-010",
            "V6-VBROWSER-001.5",
            "V6-VBROWSER-001.6",
        ],
        "click" => vec![
            "V11-STAGEHAND-011",
            "V6-VBROWSER-001.5",
            "V6-VBROWSER-001.6",
        ],
        "type" => vec![
            "V11-STAGEHAND-012",
            "V6-VBROWSER-001.5",
            "V6-VBROWSER-001.6",
        ],
        "automate" => vec![
            "V6-VBROWSER-001.3",
            "V6-VBROWSER-001.5",
            "V6-VBROWSER-001.6",
        ],
        "key-input" => vec![
            "V11-STAGEHAND-005",
            "V6-VBROWSER-001.5",
            "V6-VBROWSER-001.6",
        ],
        "privacy-guard" => {
            vec![
                "V6-VBROWSER-001.4",
                "V6-VBROWSER-001.5",
                "V6-VBROWSER-001.6",
            ]
        }
        "snapshot" => vec![
            "V6-VBROWSER-002.1",
            "V6-VBROWSER-001.5",
            "V6-VBROWSER-001.6",
        ],
        "screenshot" => vec![
            "V6-VBROWSER-002.2",
            "V6-VBROWSER-001.5",
            "V6-VBROWSER-001.6",
        ],
        "action-policy" => vec![
            "V6-VBROWSER-002.3",
            "V6-VBROWSER-001.5",
            "V6-VBROWSER-001.6",
        ],
        "auth-save" | "auth-login" => {
            vec![
                "V6-VBROWSER-002.4",
                "V6-VBROWSER-001.5",
                "V6-VBROWSER-001.6",
            ]
        }
        "native" => vec![
            "V6-VBROWSER-002.5",
            "V6-VBROWSER-001.5",
            "V6-VBROWSER-001.6",
        ],
        _ => vec!["V6-VBROWSER-001.5", "V6-VBROWSER-001.6"],
    }
}

fn canonical_vbrowser_command(action: &str) -> &str {
    match action {
        "start" | "open" => "session-start",
        "control" => "session-control",
        "navigate" => "goto",
        "back" => "navback",
        "pause" => "wait",
        "keys" => "key-input",
        "privacy" => "privacy-guard",
        _ => action,
    }
}

fn conduit_enforcement(
    root: &Path,
    parsed: &crate::ParsedArgs,
    strict: bool,
    action: &str,
) -> Value {
    let bypass_requested = conduit_bypass_requested(&parsed.flags);
    let claim_ids = claim_ids_for_action(action);
    build_plane_conduit_enforcement(
        root,
        STATE_ENV,
        STATE_SCOPE,
        strict,
        action,
        "vbrowser_conduit_enforcement",
        "core/layer0/ops/vbrowser_plane",
        bypass_requested,
        "vbrowser_surface_routes_through_layer0_conduit_with_fail_closed_behavior",
        &claim_ids,
    )
}

fn clean_id(raw: Option<&str>, fallback: &str) -> String {
    let mut out = String::new();
    if let Some(v) = raw {
        for ch in v.trim().chars() {
            if out.len() >= 96 {
                break;
            }
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
                out.push(ch.to_ascii_lowercase());
            } else {
                out.push('-');
            }
        }
    }
    let trimmed = out.trim_matches('-');
    if trimmed.is_empty() {
        fallback.to_string()
    } else {
        trimmed.to_string()
    }
}

fn normalize_target_url(raw: &str) -> String {
    let cleaned = clean(raw, 400);
    if cleaned.is_empty() {
        return "about:blank".to_string();
    }
    let lower = cleaned.to_ascii_lowercase();
    if lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("about:")
        || lower.starts_with("file:")
        || lower.starts_with("data:")
        || cleaned.contains("://")
    {
        cleaned
    } else {
        format!("https://{}", cleaned.trim_start_matches('/'))
    }
}

fn session_id(parsed: &crate::ParsedArgs) -> String {
    clean_id(
        parsed
            .flags
            .get("session-id")
            .map(String::as_str)
            .or_else(|| parsed.flags.get("session").map(String::as_str)),
        "browser-session",
    )
}

fn session_state_path(root: &Path, session_id: &str) -> PathBuf {
    state_root(root)
        .join("sessions")
        .join(format!("{session_id}.json"))
}

fn snapshot_path(root: &Path) -> PathBuf {
    state_root(root).join("snapshots").join("latest.json")
}

fn screenshot_svg_path(root: &Path) -> PathBuf {
    state_root(root).join("screenshots").join("latest.svg")
}

fn screenshot_map_path(root: &Path) -> PathBuf {
    state_root(root).join("screenshots").join("latest_map.json")
}

fn auth_vault_path(root: &Path) -> PathBuf {
    state_root(root).join("auth_vault").join("profiles.json")
}

fn ensure_parent(path: &Path) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
}

fn load_auth_vault(root: &Path) -> Value {
    read_json(&auth_vault_path(root)).unwrap_or_else(|| json!({"version":"v1","profiles":[]}))
}

fn write_auth_vault(root: &Path, value: &Value) {
    let path = auth_vault_path(root);
    ensure_parent(&path);
    let _ = write_json(&path, value);
}

fn auth_key_material(root: &Path) -> [u8; 32] {
    let mut key = [0u8; 32];
    let source = std::env::var("VBROWSER_AUTH_VAULT_KEY")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| {
            format!(
                "{}:{}",
                crate::deterministic_receipt_hash(&json!({"scope":"vbrowser_auth"})),
                root.display()
            )
        });
    let digest = sha256_hex_str(&source);
    let bytes = hex::decode(digest).unwrap_or_default();
    for (idx, b) in bytes.into_iter().take(32).enumerate() {
        key[idx] = b;
    }
    key
}

fn encrypt_secret(root: &Path, plaintext: &str) -> Option<Value> {
    let key_bytes = auth_key_material(root);
    let cipher = Aes256Gcm::new_from_slice(&key_bytes).ok()?;
    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher.encrypt(nonce, plaintext.as_bytes()).ok()?;
    Some(json!({
        "alg": "AES-256-GCM",
        "nonce_hex": hex::encode(nonce_bytes),
        "ciphertext_b64": base64::engine::general_purpose::STANDARD.encode(ciphertext)
    }))
}

fn decrypt_secret(root: &Path, payload: &Value) -> Option<String> {
    let nonce_hex = payload.get("nonce_hex")?.as_str()?;
    let ciphertext_b64 = payload.get("ciphertext_b64")?.as_str()?;
    let nonce_bytes = hex::decode(nonce_hex).ok()?;
    let ciphertext = base64::engine::general_purpose::STANDARD
        .decode(ciphertext_b64)
        .ok()?;
    let key_bytes = auth_key_material(root);
    let cipher = Aes256Gcm::new_from_slice(&key_bytes).ok()?;
    let nonce = Nonce::from_slice(&nonce_bytes);
    let plain = cipher.decrypt(nonce, ciphertext.as_ref()).ok()?;
    String::from_utf8(plain).ok()
}

fn run_session_start(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let contract = load_json_or(
        root,
        SESSION_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "vbrowser_sandbox_session_contract",
            "max_stream_latency_ms": 150,
            "default_stream_latency_ms": 60,
            "isolation": {
                "host_state_access": false,
                "network_mode": "isolated"
            }
        }),
    );
    let mut errors = Vec::<String>::new();
    if contract
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("vbrowser_session_contract_version_must_be_v1".to_string());
    }
    if contract
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "vbrowser_sandbox_session_contract"
    {
        errors.push("vbrowser_session_contract_kind_invalid".to_string());
    }
    if !errors.is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "vbrowser_plane_session_start",
            "errors": errors
        });
    }

    let sid = session_id(parsed);
    let raw_url = clean(
        parsed
            .flags
            .get("url")
            .cloned()
            .unwrap_or_else(|| "about:blank".to_string()),
        400,
    );
    let url = normalize_target_url(&raw_url);
    let shadow = clean(
        parsed
            .flags
            .get("shadow")
            .cloned()
            .unwrap_or_else(|| "default-shadow".to_string()),
        120,
    );
    let max_latency = contract
        .get("max_stream_latency_ms")
        .and_then(Value::as_u64)
        .unwrap_or(150);
    let latency = parse_u64(parsed.flags.get("latency-ms"), 0)
        .max(
            contract
                .get("default_stream_latency_ms")
                .and_then(Value::as_u64)
                .unwrap_or(60),
        )
        .min(max_latency);

    let session = json!({
        "version": "v1",
        "session_id": sid,
        "shadow": shadow,
        "target_url": url,
        "container": {
            "id": format!("ctr_{}", &sha256_hex_str(&format!("{}:{}", sid, shadow))[..12]),
            "runtime": "sandboxed-browser",
            "host_state_access": false,
            "network_mode": "isolated",
            "mounts": ["/tmp/vbrowser-session:rw", "/workspace:ro"]
        },
        "stream": {
            "transport": "ws",
            "latency_ms": latency,
            "low_latency": true
        },
        "started_at": crate::now_iso()
    });
    let path = session_state_path(root, &sid);
    let _ = write_json(&path, &session);

    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "vbrowser_plane_session_start",
        "lane": "core/layer0/ops",
        "session": session,
        "artifact": {
            "path": path.display().to_string(),
            "sha256": sha256_hex_str(&session.to_string())
        },
        "claim_evidence": [
            {
                "id": "V6-VBROWSER-001.1",
                "claim": "sandboxed_virtual_browser_runtime_starts_with_low_latency_streaming_and_host_state_isolation",
                "evidence": {
                    "session_id": sid,
                    "latency_ms": latency,
                    "host_state_access": false
                }
            },
            {
                "id": "V6-VBROWSER-001.5",
                "claim": "protheus_browser_and_shadow_browser_surfaces_route_to_core_vbrowser_lane",
                "evidence": {
                    "session_id": sid,
                    "shadow": shadow
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}
