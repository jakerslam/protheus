// SPDX-License-Identifier: Apache-2.0
use crate::contract_lane_utils as lane_utils;
use crate::now_iso;
use base64::Engine;
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone)]
struct PersonaAmbientPolicy {
    enabled: bool,
    ambient_stance: bool,
    auto_apply: bool,
    full_reload: bool,
    push_attention_queue: bool,
    cache_path: PathBuf,
    latest_path: PathBuf,
    receipts_path: PathBuf,
    max_personas: usize,
    max_patch_bytes: usize,
}

fn usage() {
    eprintln!("Usage:");
    eprintln!(
        "  protheus-ops persona-ambient apply --persona=<id> --stance-json-base64=<base64-json> [--source=<value>] [--reason=<value>] [--run-context=<value>] [--full-reload=1|0]"
    );
    eprintln!(
        "  protheus-ops persona-ambient apply --persona=<id> --stance-json=<json-object> [flags]"
    );
    eprintln!("  protheus-ops persona-ambient status [--persona=<id>]");
}

fn read_json(path: &Path) -> Option<Value> {
    lane_utils::read_json(path)
}

fn write_json(path: &Path, value: &Value) {
    let _ = lane_utils::write_json(path, value);
}

fn append_jsonl(path: &Path, row: &Value) {
    let _ = lane_utils::append_jsonl(path, row);
}

fn parse_cli_flags(argv: &[String]) -> BTreeMap<String, String> {
    crate::contract_lane_utils::parse_cli_flags(argv)
}

fn bool_from_env(name: &str) -> Option<bool> {
    let raw = std::env::var(name).ok();
    lane_utils::parse_opt_bool(raw.as_deref())
}

fn parse_bool(raw: Option<&str>, fallback: bool) -> bool {
    lane_utils::parse_bool(raw, fallback)
}

fn clean_text(value: Option<&str>, max_len: usize) -> String {
    lane_utils::clean_text(value, max_len)
}

fn sanitize_persona_id(raw: Option<&str>) -> String {
    let mut out = String::new();
    for ch in clean_text(raw, 120).chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.') {
            out.push(ch.to_ascii_lowercase());
        }
    }
    out.trim_matches('_').trim_matches('-').to_string()
}

fn normalize_path(root: &Path, value: Option<&Value>, fallback: &str) -> PathBuf {
    let raw = value
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(fallback);
    let candidate = PathBuf::from(raw);
    if candidate.is_absolute() {
        candidate
    } else {
        root.join(candidate)
    }
}

fn load_policy(root: &Path) -> PersonaAmbientPolicy {
    let default_policy = root.join("config").join("mech_suit_mode_policy.json");
    let policy_path = std::env::var("MECH_SUIT_MODE_POLICY_PATH")
        .ok()
        .map(PathBuf::from)
        .map(|p| if p.is_absolute() { p } else { root.join(p) })
        .unwrap_or(default_policy);
    let policy = read_json(&policy_path).unwrap_or_else(|| json!({}));
    let enabled = bool_from_env("MECH_SUIT_MODE_FORCE").unwrap_or_else(|| {
        policy
            .get("enabled")
            .and_then(Value::as_bool)
            .unwrap_or(true)
    });
    let personas = policy.get("personas");
    let eyes = policy.get("eyes");

    PersonaAmbientPolicy {
        enabled,
        ambient_stance: personas
            .and_then(|v| v.get("ambient_stance"))
            .and_then(Value::as_bool)
            .unwrap_or(true),
        auto_apply: personas
            .and_then(|v| v.get("auto_apply"))
            .and_then(Value::as_bool)
            .unwrap_or(true),
        full_reload: personas
            .and_then(|v| v.get("full_reload"))
            .and_then(Value::as_bool)
            .unwrap_or(false),
        push_attention_queue: eyes
            .and_then(|v| v.get("push_attention_queue"))
            .and_then(Value::as_bool)
            .unwrap_or(true),
        cache_path: normalize_path(
            root,
            personas.and_then(|v| v.get("cache_path")),
            "local/state/personas/ambient_stance/cache.json",
        ),
        latest_path: normalize_path(
            root,
            personas.and_then(|v| v.get("latest_path")),
            "local/state/personas/ambient_stance/latest.json",
        ),
        receipts_path: normalize_path(
            root,
            personas.and_then(|v| v.get("receipts_path")),
            "local/state/personas/ambient_stance/receipts.jsonl",
        ),
        max_personas: personas
            .and_then(|v| v.get("max_personas"))
            .and_then(Value::as_u64)
            .map(|n| n as usize)
            .unwrap_or(256)
            .clamp(1, 10_000),
        max_patch_bytes: personas
            .and_then(|v| v.get("max_patch_bytes"))
            .and_then(Value::as_u64)
            .map(|n| n as usize)
            .unwrap_or(64 * 1024)
            .clamp(256, 8 * 1024 * 1024),
    }
}

fn parse_stance(flags: &BTreeMap<String, String>) -> Result<Value, String> {
    if let Some(raw) = flags.get("stance-json-base64") {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(raw.as_bytes())
            .map_err(|err| format!("stance_json_base64_invalid:{err}"))?;
        let text =
            String::from_utf8(bytes).map_err(|err| format!("stance_json_utf8_invalid:{err}"))?;
        let value = serde_json::from_str::<Value>(&text)
            .map_err(|err| format!("stance_json_invalid:{err}"))?;
        return Ok(value);
    }
    if let Some(raw) = flags.get("stance-json") {
        let value = serde_json::from_str::<Value>(raw)
            .map_err(|err| format!("stance_json_invalid:{err}"))?;
        return Ok(value);
    }
    if let Some(raw) = flags.get("stance-file") {
        let path = PathBuf::from(raw);
        let content =
            fs::read_to_string(path).map_err(|err| format!("stance_file_read_failed:{err}"))?;
        let value = serde_json::from_str::<Value>(&content)
            .map_err(|err| format!("stance_file_json_invalid:{err}"))?;
        return Ok(value);
    }
    Err("missing_stance_json".to_string())
}

fn default_cache() -> Value {
    json!({
        "schema_id": "persona_ambient_stance_cache",
        "schema_version": "1.0",
        "ts": now_iso(),
        "ambient_mode_active": true,
        "personas": {}
    })
}

fn as_object_mut(value: &mut Value) -> &mut Map<String, Value> {
    if !value.is_object() {
        *value = Value::Object(Map::new());
    }
    value.as_object_mut().expect("object")
}

fn load_cache(path: &Path) -> Value {
    let mut cache = read_json(path).unwrap_or_else(default_cache);
    if !cache.is_object() {
        cache = default_cache();
    }
    if !cache.get("personas").map(Value::is_object).unwrap_or(false) {
        cache["personas"] = Value::Object(Map::new());
    }
    cache
}

fn parse_json_payload(stdout: &str) -> Option<Value> {
    let raw = stdout.trim();
    if raw.is_empty() {
        return None;
    }
    if let Ok(payload) = serde_json::from_str::<Value>(raw) {
        return Some(payload);
    }
    for line in raw.lines().rev() {
        let trimmed = line.trim();
        if !trimmed.starts_with('{') {
            continue;
        }
        if let Ok(payload) = serde_json::from_str::<Value>(trimmed) {
            return Some(payload);
        }
    }
    None
}

fn repo_root_from_current_dir() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

fn resolve_protheus_ops_command(root: &PathBuf, domain: &str) -> (String, Vec<String>) {
    crate::contract_lane_utils::resolve_protheus_ops_command(root.as_path(), domain)
}

fn enqueue_attention(persona: &str, patch_hash: &str, run_context: &str) -> Result<Value, String> {
    let root = repo_root_from_current_dir();
    let event = json!({
        "ts": now_iso(),
        "source": "persona_ambient",
        "source_type": "persona_stance_apply",
        "severity": "info",
        "summary": format!("persona ambient stance apply ({persona})"),
        "attention_key": format!("persona_stance:{persona}:{patch_hash}"),
        "persona": persona,
        "patch_hash": patch_hash
    });

    let payload = serde_json::to_string(&event)
        .map_err(|err| format!("attention_event_encode_failed:{err}"))?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(payload.as_bytes());

    let (command, mut args) = resolve_protheus_ops_command(&root, "attention-queue");
    args.push("enqueue".to_string());
    args.push(format!("--event-json-base64={encoded}"));
    args.push(format!("--run-context={run_context}"));

    let output = Command::new(&command)
        .args(&args)
        .current_dir(&root)
        .env(
            "PROTHEUS_NODE_BINARY",
            std::env::var("PROTHEUS_NODE_BINARY").unwrap_or_else(|_| "node".to_string()),
        )
        .output()
        .map_err(|err| format!("attention_queue_spawn_failed:{err}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let exit_code = output.status.code().unwrap_or(1);
    let mut receipt = parse_json_payload(&stdout).unwrap_or_else(|| {
        json!({
            "ok": false,
            "type": "attention_queue_enqueue_error",
            "reason": "attention_queue_empty_payload",
            "exit_code": exit_code,
            "stderr": clean_text(Some(&stderr), 280)
        })
    });

    if !receipt.is_object() {
        receipt = json!({
            "ok": false,
            "type": "attention_queue_enqueue_error",
            "reason": "attention_queue_invalid_payload",
            "exit_code": exit_code,
            "stderr": clean_text(Some(&stderr), 280)
        });
    }
    receipt["bridge_exit_code"] = Value::Number((exit_code as i64).into());
    if !stderr.trim().is_empty() {
        receipt["bridge_stderr"] = Value::String(clean_text(Some(&stderr), 280));
    }

    let decision = receipt
        .get("decision")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let accepted = matches!(decision, "admitted" | "deduped" | "disabled");

    if exit_code != 0 && !accepted {
        return Err(format!("attention_queue_enqueue_failed:{decision}"));
    }
    Ok(receipt)
}

fn policy_snapshot(policy: &PersonaAmbientPolicy) -> Value {
    json!({
        "enabled": policy.enabled,
        "ambient_stance": policy.ambient_stance,
        "auto_apply": policy.auto_apply,
        "full_reload": policy.full_reload,
        "push_attention_queue": policy.push_attention_queue,
        "cache_path": policy.cache_path.to_string_lossy().to_string(),
        "latest_path": policy.latest_path.to_string_lossy().to_string(),
        "receipts_path": policy.receipts_path.to_string_lossy().to_string(),
        "max_personas": policy.max_personas,
        "max_patch_bytes": policy.max_patch_bytes
    })
}

fn emit(value: &Value) {
    println!(
        "{}",
        serde_json::to_string(value).unwrap_or_else(|_| {
            "{\"ok\":false,\"type\":\"persona_ambient_encode_failed\"}".to_string()
        })
    );
}

fn stamp_receipt(value: &mut Value) {
    value["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(value));
}

fn persist_and_emit(latest_path: &Path, receipts_path: &Path, value: &mut Value) {
    stamp_receipt(value);
    write_json(latest_path, value);
    append_jsonl(receipts_path, value);
    emit(value);
}

fn fail_receipt(
    policy: &PersonaAmbientPolicy,
    command: &str,
    reason: &str,
    detail: Option<Value>,
