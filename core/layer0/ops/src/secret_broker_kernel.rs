// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use base64::engine::general_purpose::{STANDARD as BASE64_STANDARD, URL_SAFE_NO_PAD};
use base64::Engine;
use chrono::{DateTime, SecondsFormat, TimeZone, Utc};
use hmac::{Hmac, Mac};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::contract_lane_utils as lane_utils;
use crate::{deterministic_receipt_hash, now_iso};

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct SecretBrokerState {
    version: String,
    #[serde(default)]
    issued: BTreeMap<String, SecretHandleStateRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct SecretHandleStateRow {
    handle_id: String,
    secret_id: String,
    scope: String,
    caller: String,
    reason: Option<String>,
    issued_at: String,
    expires_at: String,
    value_hash: String,
    backend_provider_type: Option<String>,
    backend_provider_ref: Option<String>,
    rotation_status: Option<String>,
    resolve_count: u64,
    last_resolved_at: Option<String>,
    last_backend_provider_type: Option<String>,
    last_rotation_status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct RotationConfig {
    warn_after_days: f64,
    max_after_days: f64,
    require_rotated_at: bool,
    enforce_on_issue: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ProviderConfig {
    Env {
        enabled: bool,
        env: String,
        rotated_at_env: String,
    },
    JsonFile {
        enabled: bool,
        paths: Vec<String>,
        field: String,
        rotated_at_field: String,
    },
    Command {
        enabled: bool,
        command: CommandSpec,
        parse_json: bool,
        value_path: String,
        rotated_at_path: String,
        timeout_ms: i64,
        env: BTreeMap<String, String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
enum CommandSpec {
    Shell(String),
    Argv(Vec<String>),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct SecretSpec {
    secret_id: String,
    providers: Vec<ProviderConfig>,
    rotation: RotationConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct SecretBrokerPolicy {
    version: String,
    path: String,
    include_backend_details: bool,
    command_timeout_ms: i64,
    secrets: BTreeMap<String, SecretSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct ResolvedBackend {
    provider_type: String,
    provider_ref: Option<String>,
    external: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct RotationHealth {
    status: String,
    reason: String,
    rotated_at: Option<String>,
    age_days: Option<f64>,
    warn_after_days: f64,
    max_after_days: f64,
    require_rotated_at: bool,
    enforce_on_issue: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct LoadedSecret {
    ok: bool,
    secret_id: String,
    value: String,
    value_hash: String,
    backend: Option<ResolvedBackend>,
    rotation: Option<RotationHealth>,
    error: Option<String>,
    provider_errors: Vec<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct RotationCheckRow {
    secret_id: String,
    status: String,
    reason: Option<String>,
    available: bool,
    provider_type: Option<String>,
    provider_ref: Option<String>,
    external_backend: Option<bool>,
    rotated_at: Option<String>,
    age_days: Option<f64>,
    warn_after_days: Option<f64>,
    max_after_days: Option<f64>,
    enforce_on_issue: Option<bool>,
    provider_errors: Vec<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct RotationHealthReport {
    ok: bool,
    #[serde(rename = "type")]
    report_type: String,
    ts: String,
    policy_path: String,
    policy_version: String,
    total: usize,
    level: String,
    counts: Value,
    checks: Vec<RotationCheckRow>,
}

fn usage() {
    println!("secret-broker-kernel commands:");
    println!("  protheus-ops secret-broker-kernel load-policy [--payload-base64=<json>]");
    println!("  protheus-ops secret-broker-kernel load-secret --payload-base64=<json>");
    println!("  protheus-ops secret-broker-kernel rotation-health [--payload-base64=<json>]");
    println!("  protheus-ops secret-broker-kernel status [--payload-base64=<json>]");
    println!("  protheus-ops secret-broker-kernel issue-handle --payload-base64=<json>");
    println!("  protheus-ops secret-broker-kernel resolve-handle --payload-base64=<json>");
}

fn cli_receipt(kind: &str, payload: Value) -> Value {
    let ts = now_iso();
    let ok = payload.get("ok").and_then(Value::as_bool).unwrap_or(true);
    let mut out = json!({
        "ok": ok,
        "type": kind,
        "ts": ts,
        "date": ts[..10].to_string(),
        "payload": payload,
    });
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    out
}

fn cli_error(kind: &str, error: &str) -> Value {
    let ts = now_iso();
    let mut out = json!({
        "ok": false,
        "type": kind,
        "ts": ts,
        "date": ts[..10].to_string(),
        "error": error,
        "fail_closed": true,
    });
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    out
}

fn print_json_line(value: &Value) {
    println!(
        "{}",
        serde_json::to_string(value)
            .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode_failed\"}".to_string())
    );
}

fn payload_json(argv: &[String]) -> Result<Value, String> {
    if let Some(raw) = lane_utils::parse_flag(argv, "payload", false) {
        return serde_json::from_str::<Value>(&raw)
            .map_err(|err| format!("secret_broker_kernel_payload_decode_failed:{err}"));
    }
    if let Some(raw_b64) = lane_utils::parse_flag(argv, "payload-base64", false) {
        let bytes = BASE64_STANDARD
            .decode(raw_b64.as_bytes())
            .map_err(|err| format!("secret_broker_kernel_payload_base64_decode_failed:{err}"))?;
        let text = String::from_utf8(bytes)
            .map_err(|err| format!("secret_broker_kernel_payload_utf8_decode_failed:{err}"))?;
        return serde_json::from_str::<Value>(&text)
            .map_err(|err| format!("secret_broker_kernel_payload_decode_failed:{err}"));
    }
    Ok(json!({}))
}

fn payload_obj<'a>(value: &'a Value) -> &'a Map<String, Value> {
    value.as_object().unwrap_or_else(|| {
        static EMPTY: std::sync::OnceLock<Map<String, Value>> = std::sync::OnceLock::new();
        EMPTY.get_or_init(Map::new)
    })
}

fn text(value: Option<&Value>, max_len: usize) -> String {
    value.and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .chars()
        .take(max_len)
        .collect::<String>()
}

fn int_value(value: Option<&Value>) -> Option<i64> {
    value.and_then(|v| {
        if let Some(n) = v.as_i64() {
            return Some(n);
        }
        if let Some(n) = v.as_u64() {
            return i64::try_from(n).ok();
        }
        v.as_str()?.trim().parse::<i64>().ok()
    })
}

fn bool_value(value: Option<&Value>, fallback: bool) -> bool {
    match value {
        Some(Value::Bool(v)) => *v,
        Some(Value::String(v)) => lane_utils::parse_bool(Some(v.as_str()), fallback),
        Some(Value::Number(v)) => v.as_i64().map(|n| n != 0).unwrap_or(fallback),
        _ => fallback,
    }
}

fn number_clamped(value: Option<&Value>, lo: f64, hi: f64, fallback: f64) -> f64 {
    let raw = value.and_then(|v| {
        if let Some(n) = v.as_f64() {
            return Some(n);
        }
        v.as_str()?.trim().parse::<f64>().ok()
    });
    raw.unwrap_or(fallback).clamp(lo, hi)
}

fn runtime_root(root: &Path) -> PathBuf {
    root.join("client").join("runtime")
}

fn default_policy_path(root: &Path) -> PathBuf {
    runtime_root(root).join("config").join("secret_broker_policy.json")
}

fn default_state_path(root: &Path) -> PathBuf {
    runtime_root(root)
        .join("local")
        .join("state")
        .join("security")
        .join("secret_broker_state.json")
}

fn default_audit_path(root: &Path) -> PathBuf {
    runtime_root(root)
        .join("local")
        .join("state")
        .join("security")
        .join("secret_broker_audit.jsonl")
}

fn default_secrets_dir() -> PathBuf {
    if let Ok(raw) = std::env::var("SECRET_BROKER_SECRETS_DIR") {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home)
        .join(".config")
        .join("protheus")
        .join("secrets")
}

fn legacy_local_key_path(root: &Path) -> PathBuf {
    runtime_root(root)
        .join("state")
        .join("security")
        .join("secret_broker_key.txt")
}

fn local_key_path(_root: &Path) -> PathBuf {
    if let Ok(raw) = std::env::var("SECRET_BROKER_LOCAL_KEY_PATH") {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    default_secrets_dir().join("secret_broker_key.txt")
}

fn resolve_path(root: &Path, payload: &Map<String, Value>, payload_key: &str, env_key: &str, default_path: PathBuf) -> PathBuf {
    if let Some(raw) = payload.get(payload_key).and_then(Value::as_str) {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            let candidate = PathBuf::from(trimmed);
            return if candidate.is_absolute() { candidate } else { root.join(candidate) };
        }
    }
    if let Ok(raw) = std::env::var(env_key) {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            let candidate = PathBuf::from(trimmed);
            return if candidate.is_absolute() { candidate } else { root.join(candidate) };
        }
    }
    default_path
}

fn read_text(path: &Path) -> String {
    fs::read_to_string(path)
        .map(|v| v.trim().to_string())
        .unwrap_or_default()
}

fn write_secret(path: &Path, value: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("secret_broker_kernel_create_secret_dir_failed:{err}"))?;
    }
    fs::write(path, format!("{value}\n"))
        .map_err(|err| format!("secret_broker_kernel_write_secret_failed:{err}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

fn generated_key() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
}

fn secret_broker_key(root: &Path) -> Result<String, String> {
    for env_name in ["SECRET_BROKER_KEY", "REQUEST_GATE_SECRET", "CAPABILITY_LEASE_KEY"] {
        if let Ok(raw) = std::env::var(env_name) {
            let trimmed = raw.trim();
            if !trimmed.is_empty() {
                return Ok(trimmed.to_string());
            }
        }
    }
    let local = local_key_path(root);
    let existing = read_text(&local);
    if !existing.is_empty() {
        return Ok(existing);
    }
    let legacy = legacy_local_key_path(root);
    let legacy_existing = read_text(&legacy);
    if !legacy_existing.is_empty() {
        return Ok(legacy_existing);
    }
    let generated = generated_key();
    write_secret(&local, &generated)?;
    Ok(generated)
}

fn sha16(value: &str) -> String {
    hex::encode(Sha256::digest(value.as_bytes()))[..16].to_string()
}

fn sign_handle(body: &str, key: &str) -> Result<String, String> {
    let mut mac =
        HmacSha256::new_from_slice(key.as_bytes()).map_err(|err| format!("hmac_init_failed:{err}"))?;
    mac.update(body.as_bytes());
    Ok(hex::encode(mac.finalize().into_bytes()))
}

fn verify_handle_sig(body: &str, sig_hex: &str, key: &str) -> bool {
    let Ok(sig) = hex::decode(sig_hex) else {
        return false;
    };
    let Ok(mut mac) = HmacSha256::new_from_slice(key.as_bytes()) else {
        return false;
    };
    mac.update(body.as_bytes());
    mac.verify_slice(&sig).is_ok()
}

fn parse_ts_ms(value: &Value) -> Option<i64> {
    if let Some(n) = value.as_i64() {
        if n > 100_000_000_000 {
            return Some(n);
        }
        if n > 1_000_000_000 {
            return Some(n * 1000);
        }
    }
    if let Some(n) = value.as_u64() {
        if n > 100_000_000_000 {
            return i64::try_from(n).ok();
        }
        if n > 1_000_000_000 {
            return i64::try_from(n).ok().map(|v| v * 1000);
        }
    }
    let raw = value.as_str()?.trim();
    if raw.is_empty() {
        return None;
    }
    if let Ok(num) = raw.parse::<i64>() {
        if num > 100_000_000_000 {
            return Some(num);
        }
        if num > 1_000_000_000 {
            return Some(num * 1000);
        }
    }
    DateTime::parse_from_rfc3339(raw)
        .ok()
        .map(|ts| ts.timestamp_millis())
}

fn iso_from_ms(ms: i64) -> String {
    Utc.timestamp_millis_opt(ms)
        .single()
        .unwrap_or_else(Utc::now)
        .to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn now_ms(payload: &Map<String, Value>) -> i64 {
    int_value(payload.get("now_ms")).unwrap_or_else(|| Utc::now().timestamp_millis())
}

fn append_audit(audit_path: &Path, row: Value) -> Result<(), String> {
    let mut full = json!({ "ts": now_iso() });
    if let Some(map) = row.as_object() {
        let target = full.as_object_mut().expect("object");
        for (key, value) in map {
            target.insert(key.clone(), value.clone());
        }
    }
    lane_utils::append_jsonl(audit_path, &full)
}

fn get_path_value<'a>(value: &'a Value, dotted: &str) -> Option<&'a Value> {
    let mut current = value;
    for part in dotted.split('.').filter(|part| !part.trim().is_empty()) {
        let obj = current.as_object()?;
        current = obj.get(part.trim())?;
    }
    Some(current)
}

fn resolve_template(root: &Path, raw: &str, secret_id: &str) -> String {
    let home = std::env::var("HOME").unwrap_or_default();
    let runtime = runtime_root(root).to_string_lossy().replace('\\', "/");
    let default_dir = default_secrets_dir().to_string_lossy().replace('\\', "/");
    let mut out = raw.trim().to_string();
    out = out.replace("${HOME}", &home);
    out = out.replace("${REPO_ROOT}", &runtime);
    out = out.replace("${DEFAULT_SECRETS_DIR}", &default_dir);
    out = out.replace("${SECRET_ID}", secret_id);
    if Path::new(&out).is_absolute() {
        out
    } else {
        root.join(out).to_string_lossy().replace('\\', "/")
    }
}

fn parse_command_spec(value: &Value) -> Option<CommandSpec> {
    if let Some(raw) = value.as_str() {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            return Some(CommandSpec::Shell(trimmed.to_string()));
        }
    }
    let items = value
        .as_array()?
        .iter()
        .filter_map(Value::as_str)
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .collect::<Vec<_>>();
    if items.is_empty() {
        None
    } else {
        Some(CommandSpec::Argv(items))
    }
}

fn default_policy(root: &Path) -> SecretBrokerPolicy {
    let default_dir = default_secrets_dir();
    let home = std::env::var("HOME").unwrap_or_default();
    let mut secrets = BTreeMap::new();
    secrets.insert(
        "moltbook_api_key".to_string(),
        SecretSpec {
            secret_id: "moltbook_api_key".to_string(),
            providers: vec![
                ProviderConfig::Env {
                    enabled: true,
                    env: "MOLTBOOK_TOKEN".to_string(),
                    rotated_at_env: "MOLTBOOK_TOKEN_ROTATED_AT".to_string(),
                },
                ProviderConfig::JsonFile {
                    enabled: true,
                    paths: vec![
                        default_dir
                            .join("moltbook.credentials.json")
                            .to_string_lossy()
                            .into_owned(),
                        PathBuf::from(home.clone())
                            .join(".config")
                            .join("moltbook")
                            .join("credentials.json")
                            .to_string_lossy()
                            .into_owned(),
                        root.join("config")
                            .join("moltbook")
                            .join("credentials.json")
                            .to_string_lossy()
                            .into_owned(),
                    ],
                    field: "api_key".to_string(),
                    rotated_at_field: "rotated_at".to_string(),
                },
            ],
            rotation: RotationConfig {
                warn_after_days: 30.0,
                max_after_days: 60.0,
                require_rotated_at: false,
                enforce_on_issue: false,
            },
        },
    );
    secrets.insert(
        "moltstack_api_key".to_string(),
        SecretSpec {
            secret_id: "moltstack_api_key".to_string(),
            providers: vec![
                ProviderConfig::Env {
                    enabled: true,
                    env: "MOLTSTACK_TOKEN".to_string(),
                    rotated_at_env: "MOLTSTACK_TOKEN_ROTATED_AT".to_string(),
                },
                ProviderConfig::JsonFile {
                    enabled: true,
                    paths: vec![
                        default_dir
                            .join("moltstack.credentials.json")
                            .to_string_lossy()
                            .into_owned(),
                        PathBuf::from(std::env::var("HOME").unwrap_or_default())
                            .join(".config")
                            .join("moltstack")
                            .join("credentials.json")
                            .to_string_lossy()
                            .into_owned(),
                    ],
                    field: "api_key".to_string(),
                    rotated_at_field: "rotated_at".to_string(),
                },
            ],
            rotation: RotationConfig {
                warn_after_days: 30.0,
                max_after_days: 60.0,
                require_rotated_at: false,
                enforce_on_issue: false,
            },
        },
    );
    SecretBrokerPolicy {
        version: "1.0".to_string(),
        path: default_policy_path(root).to_string_lossy().into_owned(),
        include_backend_details: true,
        command_timeout_ms: 5000,
        secrets,
    }
}

fn normalize_provider(root: &Path, secret_id: &str, raw: &Value, command_timeout_ms: i64) -> Option<ProviderConfig> {
    let provider_type = text(raw.get("type"), 32).to_ascii_lowercase();
    match provider_type.as_str() {
        "env" => Some(ProviderConfig::Env {
            enabled: !matches!(raw.get("enabled"), Some(Value::Bool(false))),
            env: text(raw.get("env"), 120),
            rotated_at_env: text(raw.get("rotated_at_env"), 120),
        }),
        "json_file" => {
            let mut paths = raw
                .get("paths")
                .and_then(Value::as_array)
                .map(|rows| {
                    rows.iter()
                        .filter_map(Value::as_str)
                        .map(|row| resolve_template(root, row, secret_id))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            if paths.is_empty() {
                let path_text = text(raw.get("path"), 520);
                if !path_text.is_empty() {
                    paths.push(resolve_template(root, &path_text, secret_id));
                }
            }
            Some(ProviderConfig::JsonFile {
                enabled: !matches!(raw.get("enabled"), Some(Value::Bool(false))),
                paths,
                field: {
                    let v = text(raw.get("field"), 120);
                    if v.is_empty() { "api_key".to_string() } else { v }
                },
                rotated_at_field: {
                    let v = text(raw.get("rotated_at_field"), 120);
                    if v.is_empty() { "rotated_at".to_string() } else { v }
                },
            })
        }
        "command" => {
            let command = parse_command_spec(raw.get("command").unwrap_or(&Value::Null))?;
            let value_path = {
                let v = text(raw.get("value_path").or_else(|| raw.get("value_field")), 160);
                if v.is_empty() { "value".to_string() } else { v }
            };
            let rotated_at_path = {
                let v = text(raw.get("rotated_at_path").or_else(|| raw.get("rotated_at_field")), 160);
                if v.is_empty() { "rotated_at".to_string() } else { v }
            };
            let env = raw
                .get("env")
                .and_then(Value::as_object)
                .map(|map| {
                    map.iter()
                        .map(|(k, v)| (k.clone(), v.as_str().unwrap_or_default().to_string()))
                        .collect::<BTreeMap<_, _>>()
                })
                .unwrap_or_default();
            Some(ProviderConfig::Command {
                enabled: matches!(raw.get("enabled"), Some(Value::Bool(true))),
                command,
                parse_json: !matches!(raw.get("parse_json"), Some(Value::Bool(false))),
                value_path,
                rotated_at_path,
                timeout_ms: int_value(raw.get("timeout_ms"))
                    .unwrap_or(command_timeout_ms)
                    .clamp(500, 60_000),
                env,
            })
        }
        _ => None,
    }
}

fn normalize_secret_spec(
    root: &Path,
    secret_id: &str,
    raw: Option<&Value>,
    base: Option<&SecretSpec>,
    policy_rotation: &RotationConfig,
    command_timeout_ms: i64,
) -> SecretSpec {
    let raw_obj = raw.and_then(Value::as_object);
    let base_providers = base.map(|row| row.providers.clone()).unwrap_or_default();
    let providers = if let Some(raw_providers) = raw_obj.and_then(|obj| obj.get("providers")).and_then(Value::as_array) {
        raw_providers
            .iter()
            .filter_map(|provider| normalize_provider(root, secret_id, provider, command_timeout_ms))
            .collect::<Vec<_>>()
    } else {
        base_providers
    };
    let base_rotation = base
        .map(|row| row.rotation.clone())
        .unwrap_or_else(|| policy_rotation.clone());
    let raw_rotation = raw_obj.and_then(|obj| obj.get("rotation")).and_then(Value::as_object);
    let rotation = RotationConfig {
        warn_after_days: number_clamped(
            raw_rotation.and_then(|row| row.get("warn_after_days")),
            1.0,
            3650.0,
            base_rotation.warn_after_days,
        ),
        max_after_days: number_clamped(
            raw_rotation.and_then(|row| row.get("max_after_days")),
            1.0,
            3650.0,
            base_rotation.max_after_days.max(base_rotation.warn_after_days),
        )
        .max(number_clamped(
            raw_rotation.and_then(|row| row.get("warn_after_days")),
            1.0,
            3650.0,
            base_rotation.warn_after_days,
        )),
        require_rotated_at: bool_value(
            raw_rotation.and_then(|row| row.get("require_rotated_at")),
            base_rotation.require_rotated_at,
        ),
        enforce_on_issue: bool_value(
            raw_rotation.and_then(|row| row.get("enforce_on_issue")),
            base_rotation.enforce_on_issue,
        ),
    };
    SecretSpec {
        secret_id: secret_id.to_string(),
        providers,
        rotation,
    }
}

fn load_policy(root: &Path, payload: &Map<String, Value>) -> SecretBrokerPolicy {
    let policy_path = resolve_path(
        root,
        payload,
        "policy_path",
        "SECRET_BROKER_POLICY_PATH",
        default_policy_path(root),
    );
    let base = default_policy(root);
    let raw = lane_utils::read_json(&policy_path).unwrap_or_else(|| json!({}));
    let raw_obj = raw.as_object();
    let include_backend_details = raw_obj
        .and_then(|obj| obj.get("audit"))
        .and_then(Value::as_object)
        .map(|audit| bool_value(audit.get("include_backend_details"), base.include_backend_details))
        .unwrap_or(base.include_backend_details);
    let command_timeout_ms = raw_obj
        .and_then(|obj| obj.get("command_backend"))
        .and_then(Value::as_object)
        .and_then(|command| int_value(command.get("timeout_ms")))
        .unwrap_or(base.command_timeout_ms)
        .clamp(500, 60_000);
    let base_rotation = RotationConfig {
        warn_after_days: raw_obj
            .and_then(|obj| obj.get("rotation_policy"))
            .and_then(Value::as_object)
            .map(|rotation| {
                number_clamped(
                    rotation.get("warn_after_days"),
                    1.0,
                    3650.0,
                    45.0,
                )
            })
            .unwrap_or(45.0),
        max_after_days: raw_obj
            .and_then(|obj| obj.get("rotation_policy"))
            .and_then(Value::as_object)
            .map(|rotation| {
                number_clamped(
                    rotation.get("max_after_days"),
                    1.0,
                    3650.0,
                    90.0,
                )
            })
            .unwrap_or(90.0),
        require_rotated_at: raw_obj
            .and_then(|obj| obj.get("rotation_policy"))
            .and_then(Value::as_object)
            .map(|rotation| bool_value(rotation.get("require_rotated_at"), false))
            .unwrap_or(false),
        enforce_on_issue: raw_obj
            .and_then(|obj| obj.get("rotation_policy"))
            .and_then(Value::as_object)
            .map(|rotation| bool_value(rotation.get("enforce_on_issue"), false))
            .unwrap_or(false),
    };
    let mut secrets = BTreeMap::new();
    let raw_secrets = raw_obj
        .and_then(|obj| obj.get("secrets"))
        .and_then(Value::as_object);
    let secret_ids = base
        .secrets
        .keys()
        .cloned()
        .chain(
            raw_secrets
                .map(|row| row.keys().cloned().collect::<Vec<_>>())
                .unwrap_or_default(),
        )
        .collect::<std::collections::BTreeSet<_>>();
    for secret_id in secret_ids {
        let spec = normalize_secret_spec(
            root,
            &secret_id,
            raw_secrets.and_then(|row| row.get(&secret_id)),
            base.secrets.get(&secret_id),
            &base_rotation,
            command_timeout_ms,
        );
        secrets.insert(secret_id.clone(), spec);
    }
    SecretBrokerPolicy {
        version: text(raw_obj.and_then(|obj| obj.get("version")), 32).or_else_if_empty("1.0"),
        path: policy_path.to_string_lossy().into_owned(),
        include_backend_details,
        command_timeout_ms,
        secrets,
    }
}

trait OrElseIfEmpty {
    fn or_else_if_empty(self, fallback: &str) -> String;
}

impl OrElseIfEmpty for String {
    fn or_else_if_empty(self, fallback: &str) -> String {
        if self.trim().is_empty() {
            fallback.to_string()
        } else {
            self
        }
    }
}

fn read_state(path: &Path) -> SecretBrokerState {
    lane_utils::read_json(path)
        .and_then(|value| serde_json::from_value::<SecretBrokerState>(value).ok())
        .unwrap_or_else(|| SecretBrokerState {
            version: "1.1".to_string(),
            issued: BTreeMap::new(),
        })
}

fn write_state(path: &Path, state: &SecretBrokerState) -> Result<(), String> {
    let payload =
        serde_json::to_value(state).map_err(|err| format!("secret_broker_kernel_state_encode_failed:{err}"))?;
    lane_utils::write_json(path, &payload)
}

fn provider_env(provider: &ProviderConfig) -> Option<Value> {
    let ProviderConfig::Env {
        env,
        rotated_at_env,
        ..
    } = provider
    else {
        return None;
    };
    let value = std::env::var(env).ok()?.trim().to_string();
    if value.is_empty() {
        return None;
    }
    let rotated_at = if rotated_at_env.trim().is_empty() {
        Value::Null
    } else {
        std::env::var(rotated_at_env)
            .ok()
            .filter(|row| !row.trim().is_empty())
            .map(Value::String)
            .unwrap_or(Value::Null)
    };
    Some(json!({
        "ok": true,
        "value": value,
        "rotated_at": rotated_at,
        "provider_type": "env",
        "provider_ref": env,
        "external": true
    }))
}

fn provider_json_file(root: &Path, secret_id: &str, provider: &ProviderConfig) -> Option<Value> {
    let ProviderConfig::JsonFile {
        paths,
        field,
        rotated_at_field,
        ..
    } = provider
    else {
        return None;
    };
    for raw_path in paths {
        let resolved = resolve_template(root, raw_path, secret_id);
        let resolved_path = PathBuf::from(&resolved);
        if !resolved_path.exists() {
            continue;
        }
        let Ok(text) = fs::read_to_string(&resolved_path) else {
            continue;
        };
        let Ok(payload) = serde_json::from_str::<Value>(&text) else {
            continue;
        };
        let Some(value) = get_path_value(&payload, field).and_then(Value::as_str) else {
            continue;
        };
        let trimmed = value.trim();
        if trimmed.is_empty() {
            continue;
        }
        let rotated_at = get_path_value(&payload, rotated_at_field)
            .cloned()
            .unwrap_or(Value::Null);
        return Some(json!({
            "ok": true,
            "value": trimmed,
            "rotated_at": rotated_at,
            "provider_type": "json_file",
            "provider_ref": resolved,
            "external": false
        }));
    }
    None
}

fn provider_command(secret_id: &str, provider: &ProviderConfig) -> Option<Value> {
    let ProviderConfig::Command {
        command,
        parse_json,
        value_path,
        rotated_at_path,
        env,
        ..
    } = provider
    else {
        return None;
    };
    let mut command_builder = match command {
        CommandSpec::Argv(argv) if !argv.is_empty() => {
            let mut builder = Command::new(&argv[0]);
            builder.args(&argv[1..]);
            builder
        }
        CommandSpec::Shell(shell) => {
            let mut builder = Command::new("/bin/sh");
            builder.args(["-lc", shell]);
            builder
        }
        _ => return None,
    };
    command_builder.env("SECRET_ID", secret_id);
    command_builder.env("SECRET_BROKER_SECRET_ID", secret_id);
    for (key, value) in env {
        command_builder.env(key, value);
    }
    let output = command_builder.output().ok()?;
    if !output.status.success() {
        return Some(json!({
            "ok": false,
            "reason": "command_exit_nonzero",
            "code": output.status.code().unwrap_or(1),
            "stderr": String::from_utf8_lossy(&output.stderr).trim().chars().take(200).collect::<String>(),
        }));
    }
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if stdout.is_empty() {
        return Some(json!({
            "ok": false,
            "reason": "command_empty_stdout",
        }));
    }
    if !parse_json {
        return Some(json!({
            "ok": true,
            "value": stdout,
            "rotated_at": Value::Null,
            "provider_type": "command",
            "provider_ref": match command {
                CommandSpec::Argv(argv) => argv.first().cloned().unwrap_or_default(),
                CommandSpec::Shell(shell) => shell.clone(),
            },
            "external": true
        }));
    }
    let Ok(payload) = serde_json::from_str::<Value>(&stdout) else {
        return Some(json!({
            "ok": false,
            "reason": "command_json_invalid"
        }));
    };
    let value = get_path_value(&payload, value_path)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_string();
    if value.is_empty() {
        return Some(json!({
            "ok": false,
            "reason": "command_value_missing"
        }));
    }
    let rotated_at = get_path_value(&payload, rotated_at_path)
        .cloned()
        .unwrap_or(Value::Null);
    Some(json!({
        "ok": true,
        "value": value,
        "rotated_at": rotated_at,
        "provider_type": "command",
        "provider_ref": match command {
            CommandSpec::Argv(argv) => argv.first().cloned().unwrap_or_default(),
            CommandSpec::Shell(shell) => shell.clone(),
        },
        "external": true
    }))
}

fn evaluate_rotation(rotation_cfg: &RotationConfig, rotated_at: Option<&Value>, now_ms: i64) -> RotationHealth {
    let rotated_at_ms = rotated_at.and_then(parse_ts_ms);
    if rotated_at_ms.is_none() {
        return RotationHealth {
            status: if rotation_cfg.require_rotated_at {
                "critical".to_string()
            } else {
                "unknown".to_string()
            },
            reason: "rotated_at_missing".to_string(),
            rotated_at: None,
            age_days: None,
            warn_after_days: rotation_cfg.warn_after_days,
            max_after_days: rotation_cfg.max_after_days,
            require_rotated_at: rotation_cfg.require_rotated_at,
            enforce_on_issue: rotation_cfg.enforce_on_issue,
        };
    }
    let rotated_at_ms = rotated_at_ms.unwrap_or(now_ms);
    let age_days = ((now_ms - rotated_at_ms).max(0) as f64) / 86_400_000f64;
    let (status, reason) = if age_days > rotation_cfg.max_after_days {
        ("critical", "rotation_age_exceeded")
    } else if age_days > rotation_cfg.warn_after_days {
        ("warn", "rotation_age_warning")
    } else {
        ("ok", "rotation_fresh")
    };
    RotationHealth {
        status: status.to_string(),
        reason: reason.to_string(),
        rotated_at: Some(iso_from_ms(rotated_at_ms)),
        age_days: Some((age_days * 1000.0).round() / 1000.0),
        warn_after_days: rotation_cfg.warn_after_days,
        max_after_days: rotation_cfg.max_after_days,
        require_rotated_at: rotation_cfg.require_rotated_at,
        enforce_on_issue: rotation_cfg.enforce_on_issue,
    }
}

fn load_secret_by_id(
    root: &Path,
    payload: &Map<String, Value>,
    policy: &SecretBrokerPolicy,
    audit_path: &Path,
    with_audit: bool,
) -> LoadedSecret {
    let secret_id = text(payload.get("secret_id"), 160);
    let now = now_ms(payload);
    let Some(spec) = policy.secrets.get(&secret_id) else {
        return LoadedSecret {
            ok: false,
            secret_id,
            error: Some("secret_id_unsupported".to_string()),
            ..LoadedSecret::default()
        };
    };
    let mut provider_errors = Vec::new();
    for provider in &spec.providers {
        let enabled = match provider {
            ProviderConfig::Env { enabled, .. }
            | ProviderConfig::JsonFile { enabled, .. }
            | ProviderConfig::Command { enabled, .. } => *enabled,
        };
        if !enabled {
            continue;
        }
        let result = match provider {
            ProviderConfig::Env { .. } => provider_env(provider),
            ProviderConfig::JsonFile { .. } => provider_json_file(root, &secret_id, provider),
            ProviderConfig::Command { .. } => provider_command(&secret_id, provider),
        };
        let Some(result) = result else {
            provider_errors.push(json!({
                "provider_type": match provider {
                    ProviderConfig::Env { .. } => "env",
                    ProviderConfig::JsonFile { .. } => "json_file",
                    ProviderConfig::Command { .. } => "command",
                },
                "reason": "provider_failed"
            }));
            continue;
        };
        if result.get("ok").and_then(Value::as_bool) != Some(true) {
            provider_errors.push(json!({
                "provider_type": result.get("provider_type").and_then(Value::as_str).unwrap_or(match provider {
                    ProviderConfig::Env { .. } => "env",
                    ProviderConfig::JsonFile { .. } => "json_file",
                    ProviderConfig::Command { .. } => "command",
                }),
                "reason": result.get("reason").and_then(Value::as_str).unwrap_or("provider_failed"),
                "code": result.get("code").cloned().unwrap_or(Value::Null),
                "ref": result.get("provider_ref").cloned().unwrap_or(Value::Null)
            }));
            continue;
        }
        let value = text(result.get("value"), 8192);
        if value.is_empty() {
            provider_errors.push(json!({
                "provider_type": result.get("provider_type").and_then(Value::as_str).unwrap_or("unknown"),
                "reason": "value_empty"
            }));
            continue;
        }
        let rotation = evaluate_rotation(&spec.rotation, result.get("rotated_at"), now);
        let backend = ResolvedBackend {
            provider_type: text(result.get("provider_type"), 64),
            provider_ref: {
                let v = text(result.get("provider_ref"), 240);
                if v.is_empty() { None } else { Some(v) }
            },
            external: bool_value(result.get("external"), false),
        };
        if with_audit {
            let _ = append_audit(
                audit_path,
                json!({
                    "type": "secret_value_loaded",
                    "secret_id": secret_id,
                    "provider_type": backend.provider_type,
                    "provider_ref": if policy.include_backend_details { backend.provider_ref.clone() } else { None },
                    "external_backend": backend.external,
                    "value_hash": sha16(&value),
                    "rotation_status": rotation.status,
                    "rotation_age_days": rotation.age_days,
                }),
            );
        }
        return LoadedSecret {
            ok: true,
            secret_id: secret_id.clone(),
            value: value.clone(),
            value_hash: sha16(&value),
            backend: Some(backend),
            rotation: Some(rotation),
            error: None,
            provider_errors: Vec::new(),
        };
    }
    if with_audit {
        let _ = append_audit(
            audit_path,
            json!({
                "type": "secret_value_load_failed",
                "secret_id": secret_id,
                "reason": "all_providers_failed",
                "provider_errors": provider_errors,
            }),
        );
    }
    LoadedSecret {
        ok: false,
        secret_id,
        error: Some("secret_value_missing".to_string()),
        provider_errors,
        ..LoadedSecret::default()
    }
}

fn rotation_health_report(
    root: &Path,
    payload: &Map<String, Value>,
    policy: &SecretBrokerPolicy,
    audit_path: &Path,
    with_audit: bool,
) -> RotationHealthReport {
    let ids = payload
        .get("secret_ids")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|row| row.trim().to_string())
                .filter(|row| !row.is_empty())
                .collect::<Vec<_>>()
        })
        .filter(|rows| !rows.is_empty())
        .unwrap_or_else(|| policy.secrets.keys().cloned().collect());
    let mut ok_count = 0usize;
    let mut warn_count = 0usize;
    let mut critical_count = 0usize;
    let mut unknown_count = 0usize;
    let mut unavailable_count = 0usize;
    let mut checks = Vec::new();
    let now = now_ms(payload);
    for secret_id in ids {
        let loaded = load_secret_by_id(
            root,
            &json!({
                "secret_id": secret_id,
                "now_ms": now,
            })
            .as_object()
            .cloned()
            .unwrap_or_default(),
            policy,
            audit_path,
            false,
        );
        if !loaded.ok {
            unavailable_count += 1;
            checks.push(RotationCheckRow {
                secret_id,
                status: "critical".to_string(),
                reason: loaded.error.clone(),
                available: false,
                provider_errors: loaded.provider_errors.clone(),
                ..RotationCheckRow::default()
            });
            continue;
        }
        let rotation = loaded.rotation.clone().unwrap_or_default();
        match rotation.status.as_str() {
            "ok" => ok_count += 1,
            "warn" => warn_count += 1,
            "critical" => critical_count += 1,
            _ => unknown_count += 1,
        }
        checks.push(RotationCheckRow {
            secret_id: loaded.secret_id,
            status: rotation.status.clone(),
            reason: Some(rotation.reason.clone()),
            available: true,
            provider_type: loaded.backend.as_ref().map(|row| row.provider_type.clone()),
            provider_ref: if policy.include_backend_details {
                loaded.backend.as_ref().and_then(|row| row.provider_ref.clone())
            } else {
                None
            },
            external_backend: loaded.backend.as_ref().map(|row| row.external),
            rotated_at: rotation.rotated_at.clone(),
            age_days: rotation.age_days,
            warn_after_days: Some(rotation.warn_after_days),
            max_after_days: Some(rotation.max_after_days),
            enforce_on_issue: Some(rotation.enforce_on_issue),
            provider_errors: Vec::new(),
        });
    }
    let level = if critical_count > 0 || unavailable_count > 0 {
        "critical"
    } else if warn_count > 0 {
        "warn"
    } else {
        "ok"
    };
    let report = RotationHealthReport {
        ok: level != "critical",
        report_type: "secret_rotation_health".to_string(),
        ts: now_iso(),
        policy_path: policy.path.clone(),
        policy_version: policy.version.clone(),
        total: checks.len(),
        level: level.to_string(),
        counts: json!({
            "ok": ok_count,
            "warn": warn_count,
            "critical": critical_count,
            "unknown": unknown_count,
            "unavailable": unavailable_count
        }),
        checks,
    };
    if with_audit {
        let _ = append_audit(
            audit_path,
            json!({
                "type": "secret_rotation_check",
                "level": report.level,
                "total": report.total,
                "counts": report.counts,
            }),
        );
    }
    report
}

fn secret_broker_status(
    root: &Path,
    payload: &Map<String, Value>,
    policy: &SecretBrokerPolicy,
    state_path: &Path,
    audit_path: &Path,
) -> Value {
    let state = read_state(state_path);
    let now = now_ms(payload);
    let issued_total = state.issued.len();
    let issued_active = state
        .issued
        .values()
        .filter(|row| parse_ts_ms(&Value::String(row.expires_at.clone())).unwrap_or(0) > now)
        .count();
    let rotation = serde_json::to_value(rotation_health_report(root, payload, policy, audit_path, false))
        .unwrap_or_else(|_| json!({"ok": false, "type": "secret_rotation_health"}));
    json!({
        "ok": true,
        "type": "secret_broker_status",
        "ts": now_iso(),
        "policy_path": policy.path,
        "policy_version": policy.version,
        "state_path": state_path.to_string_lossy(),
        "audit_path": audit_path.to_string_lossy(),
        "supported_secret_ids": policy.secrets.keys().cloned().collect::<Vec<_>>(),
        "issued_total": issued_total,
        "issued_active": issued_active,
        "rotation": rotation,
    })
}

fn issue_handle(
    root: &Path,
    payload: &Map<String, Value>,
    policy: &SecretBrokerPolicy,
    state_path: &Path,
    audit_path: &Path,
) -> Value {
    let key = match secret_broker_key(root) {
        Ok(value) => value,
        Err(err) => {
            return json!({
                "ok": false,
                "error": err
            })
        }
    };
    let secret_id = text(payload.get("secret_id"), 160);
    let scope = text(payload.get("scope"), 180);
    let caller = {
        let clean = text(payload.get("caller"), 180);
        if clean.is_empty() { "unknown".to_string() } else { clean }
    };
    let reason = {
        let clean = text(payload.get("reason"), 240);
        if clean.is_empty() { None } else { Some(clean) }
    };
    if secret_id.is_empty() {
        return json!({ "ok": false, "error": "secret_id_required" });
    }
    if scope.is_empty() {
        return json!({ "ok": false, "error": "scope_required" });
    }
    let ttl_sec = int_value(payload.get("ttl_sec"))
        .unwrap_or(300)
        .clamp(30, 3600);
    let loaded = load_secret_by_id(root, payload, policy, audit_path, true);
    if !loaded.ok {
        let _ = append_audit(
            audit_path,
            json!({
                "type": "secret_handle_issue_denied",
                "secret_id": secret_id,
                "scope": scope,
                "caller": caller,
                "reason": loaded.error,
            }),
        );
        return serde_json::to_value(loaded).unwrap_or_else(|_| json!({ "ok": false, "error": "secret_value_missing" }));
    }
    let rotation = loaded.rotation.clone().unwrap_or_default();
    if rotation.enforce_on_issue && rotation.status == "critical" {
        let _ = append_audit(
            audit_path,
            json!({
                "type": "secret_handle_issue_denied",
                "secret_id": secret_id,
                "scope": scope,
                "caller": caller,
                "reason": "rotation_policy_enforced",
                "rotation_status": rotation.status,
            }),
        );
        return json!({
            "ok": false,
            "error": "rotation_policy_enforced",
            "secret_id": secret_id,
            "rotation": rotation,
        });
    }
    let issued_ms = now_ms(payload);
    let expires_ms = issued_ms + ttl_sec * 1000;
    let handle_id = format!("sh_{}", &deterministic_receipt_hash(&json!({
        "secret_id": secret_id,
        "scope": scope,
        "caller": caller,
        "issued_ms": issued_ms,
    }))[..16]);
    let body_payload = json!({
        "v": "1.1",
        "handle_id": handle_id,
        "secret_id": secret_id,
        "scope": scope,
        "caller": caller,
        "reason": reason,
        "issued_at_ms": issued_ms,
        "issued_at": iso_from_ms(issued_ms),
        "expires_at_ms": expires_ms,
        "expires_at": iso_from_ms(expires_ms),
        "nonce": &deterministic_receipt_hash(&json!({"handle_id": handle_id, "issued_ms": issued_ms}))[..16]
    });
    let body_text = serde_json::to_string(&body_payload).unwrap_or_else(|_| "{}".to_string());
    let body = URL_SAFE_NO_PAD.encode(body_text.as_bytes());
    let sig = match sign_handle(&body, &key) {
        Ok(value) => value,
        Err(err) => return json!({ "ok": false, "error": err }),
    };
    let handle = format!("{body}.{sig}");
    let mut state = read_state(state_path);
    state.issued.insert(
        handle_id.clone(),
        SecretHandleStateRow {
            handle_id: handle_id.clone(),
            secret_id: secret_id.clone(),
            scope: scope.clone(),
            caller: caller.clone(),
            reason: reason.clone(),
            issued_at: iso_from_ms(issued_ms),
            expires_at: iso_from_ms(expires_ms),
            value_hash: loaded.value_hash.clone(),
            backend_provider_type: loaded.backend.as_ref().map(|row| row.provider_type.clone()),
            backend_provider_ref: loaded.backend.as_ref().and_then(|row| row.provider_ref.clone()),
            rotation_status: loaded.rotation.as_ref().map(|row| row.status.clone()),
            ..SecretHandleStateRow::default()
        },
    );
    let _ = write_state(state_path, &state);
    let _ = append_audit(
        audit_path,
        json!({
            "type": "secret_handle_issued",
            "handle_id": handle_id,
            "secret_id": secret_id,
            "scope": scope,
            "caller": caller,
            "ttl_sec": ttl_sec,
            "reason": reason,
            "backend_provider_type": loaded.backend.as_ref().map(|row| row.provider_type.clone()),
            "backend_provider_ref": if policy.include_backend_details {
                loaded.backend.as_ref().and_then(|row| row.provider_ref.clone())
            } else { None },
            "rotation_status": loaded.rotation.as_ref().map(|row| row.status.clone()),
            "rotation_age_days": loaded.rotation.as_ref().and_then(|row| row.age_days),
        }),
    );
    json!({
        "ok": true,
        "handle": handle,
        "handle_id": handle_id,
        "secret_id": secret_id,
        "scope": scope,
        "caller": caller,
        "issued_at": iso_from_ms(issued_ms),
        "expires_at": iso_from_ms(expires_ms),
        "ttl_sec": ttl_sec,
        "backend": loaded.backend,
        "rotation": loaded.rotation,
    })
}

fn resolve_handle(
    root: &Path,
    payload: &Map<String, Value>,
    policy: &SecretBrokerPolicy,
    state_path: &Path,
    audit_path: &Path,
) -> Value {
    let key = match secret_broker_key(root) {
        Ok(value) => value,
        Err(err) => return json!({ "ok": false, "error": err }),
    };
    let handle = text(payload.get("handle"), 8192);
    let parts = handle.split('.').collect::<Vec<_>>();
    if parts.len() != 2 {
        return json!({ "ok": false, "error": "handle_malformed" });
    }
    let body = parts[0];
    let sig = parts[1];
    if !verify_handle_sig(body, sig, &key) {
        return json!({ "ok": false, "error": "handle_signature_invalid" });
    }
    let decoded = match URL_SAFE_NO_PAD.decode(body.as_bytes()) {
        Ok(value) => value,
        Err(_) => return json!({ "ok": false, "error": "handle_payload_invalid" }),
    };
    let payload_value = match serde_json::from_slice::<Value>(&decoded) {
        Ok(value) => value,
        Err(_) => return json!({ "ok": false, "error": "handle_payload_invalid" }),
    };
    let handle_payload = match payload_value.as_object() {
        Some(value) => value,
        None => return json!({ "ok": false, "error": "handle_payload_invalid" }),
    };
    let handle_id = text(handle_payload.get("handle_id"), 160);
    let secret_id = text(handle_payload.get("secret_id"), 160);
    let scope = text(handle_payload.get("scope"), 180);
    let caller = text(handle_payload.get("caller"), 180);
    let expires_at_ms = int_value(handle_payload.get("expires_at_ms")).unwrap_or(0);
    let now = now_ms(payload);
    if handle_id.is_empty() || secret_id.is_empty() || scope.is_empty() || caller.is_empty() {
        return json!({ "ok": false, "error": "handle_payload_missing_fields" });
    }
    if expires_at_ms <= now {
        let _ = append_audit(
            audit_path,
            json!({
                "type": "secret_handle_resolve_denied",
                "reason": "handle_expired",
                "handle_id": handle_id,
                "secret_id": secret_id,
            }),
        );
        return json!({ "ok": false, "error": "handle_expired", "handle_id": handle_id, "secret_id": secret_id });
    }
    let required_scope = text(payload.get("scope"), 180);
    if !required_scope.is_empty() && required_scope != scope {
        return json!({
            "ok": false,
            "error": "scope_mismatch",
            "handle_id": handle_id,
            "secret_id": secret_id,
            "required_scope": required_scope,
            "handle_scope": scope,
        });
    }
    let required_caller = text(payload.get("caller"), 180);
    if !required_caller.is_empty() && required_caller != caller {
        return json!({
            "ok": false,
            "error": "caller_mismatch",
            "handle_id": handle_id,
            "secret_id": secret_id,
            "required_caller": required_caller,
            "handle_caller": caller,
        });
    }
    let mut state = read_state(state_path);
    if !state.issued.contains_key(&handle_id) {
        return json!({ "ok": false, "error": "handle_unknown", "handle_id": handle_id, "secret_id": secret_id });
    }
    let loaded = load_secret_by_id(
        root,
        &json!({
            "secret_id": secret_id,
            "policy_path": payload.get("policy_path").cloned().unwrap_or(Value::Null),
            "now_ms": now,
        })
        .as_object()
        .cloned()
        .unwrap_or_default(),
        policy,
        audit_path,
        true,
    );
    if !loaded.ok {
        return serde_json::to_value(loaded).unwrap_or_else(|_| json!({ "ok": false, "error": "secret_value_missing" }));
    }
    if let Some(row) = state.issued.get_mut(&handle_id) {
        row.resolve_count += 1;
        row.last_resolved_at = Some(iso_from_ms(now));
        row.last_backend_provider_type = loaded.backend.as_ref().map(|item| item.provider_type.clone());
        row.last_rotation_status = loaded.rotation.as_ref().map(|item| item.status.clone());
    }
    let _ = write_state(state_path, &state);
    let _ = append_audit(
        audit_path,
        json!({
            "type": "secret_handle_resolved",
            "handle_id": handle_id,
            "secret_id": secret_id,
            "scope": scope,
            "caller": caller,
            "resolve_count": state.issued.get(&handle_id).map(|row| row.resolve_count).unwrap_or(0),
            "backend_provider_type": loaded.backend.as_ref().map(|row| row.provider_type.clone()),
            "backend_provider_ref": if policy.include_backend_details {
                loaded.backend.as_ref().and_then(|row| row.provider_ref.clone())
            } else { None },
            "rotation_status": loaded.rotation.as_ref().map(|row| row.status.clone()),
            "rotation_age_days": loaded.rotation.as_ref().and_then(|row| row.age_days),
        }),
    );
    json!({
        "ok": true,
        "handle_id": handle_id,
        "secret_id": secret_id,
        "scope": scope,
        "caller": caller,
        "expires_at": handle_payload.get("expires_at").cloned().unwrap_or(Value::Null),
        "value": loaded.value,
        "value_hash": loaded.value_hash,
        "backend": loaded.backend,
        "rotation": loaded.rotation,
    })
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let command = argv
        .first()
        .map(|row| row.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());
    if matches!(command.as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }
    let payload_value = match payload_json(argv) {
        Ok(value) => value,
        Err(err) => {
            print_json_line(&cli_error("secret_broker_kernel", &err));
            return 2;
        }
    };
    let payload = payload_obj(&payload_value);
    let policy = load_policy(root, payload);
    let state_path = resolve_path(
        root,
        payload,
        "state_path",
        "SECRET_BROKER_STATE_PATH",
        default_state_path(root),
    );
    let audit_path = resolve_path(
        root,
        payload,
        "audit_path",
        "SECRET_BROKER_AUDIT_PATH",
        default_audit_path(root),
    );
    let result = match command.as_str() {
        "load-policy" => json!({
            "ok": true,
            "policy": policy,
        }),
        "load-secret" => serde_json::to_value(load_secret_by_id(root, payload, &policy, &audit_path, bool_value(payload.get("with_audit"), true)))
            .unwrap_or_else(|_| json!({ "ok": false, "error": "secret_value_missing" })),
        "rotation-health" => serde_json::to_value(rotation_health_report(root, payload, &policy, &audit_path, bool_value(payload.get("with_audit"), true)))
            .unwrap_or_else(|_| json!({ "ok": false, "error": "rotation_health_failed" })),
        "status" => secret_broker_status(root, payload, &policy, &state_path, &audit_path),
        "issue-handle" => issue_handle(root, payload, &policy, &state_path, &audit_path),
        "resolve-handle" => resolve_handle(root, payload, &policy, &state_path, &audit_path),
        _ => {
            print_json_line(&cli_error("secret_broker_kernel", "unknown_command"));
            return 2;
        }
    };
    let ok = result.get("ok").and_then(Value::as_bool).unwrap_or(false);
    print_json_line(&cli_receipt("secret_broker_kernel", result));
    if ok { 0 } else { 2 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn temp_root() -> tempfile::TempDir {
        let dir = tempdir().expect("tempdir");
        fs::create_dir_all(dir.path().join("client/runtime/config")).expect("config");
        dir
    }

    #[test]
    fn issue_and_resolve_handle_round_trip() {
        let root = temp_root();
        let secret_dir = root.path().join(".secrets-roundtrip");
        fs::create_dir_all(&secret_dir).expect("secret_dir");
        let policy_path = root.path().join("client/runtime/config/secret_broker_policy.json");
        fs::write(
            &policy_path,
            format!(
                "{{\"version\":\"1.0\",\"audit\":{{\"include_backend_details\":true}},\"command_backend\":{{\"timeout_ms\":5000}},\"secrets\":{{\"moltbook_api_key\":{{\"providers\":[{{\"type\":\"json_file\",\"paths\":[\"{}\"],\"field\":\"api_key\",\"rotated_at_field\":\"rotated_at\"}}]}}}}}}",
                secret_dir
                    .join("moltbook.credentials.json")
                    .to_string_lossy()
                    .replace('\\', "\\\\")
            ),
        )
        .expect("policy");
        fs::write(
            secret_dir.join("moltbook.credentials.json"),
            "{\"api_key\":\"mb-test\",\"rotated_at\":\"2026-03-01T00:00:00Z\"}",
        )
        .expect("credentials");
        std::env::set_var("SECRET_BROKER_KEY", "test-secret-key");
        let payload = json!({
            "secret_id": "moltbook_api_key",
            "scope": "scope.test",
            "caller": "caller.test",
            "ttl_sec": 60,
            "policy_path": policy_path.to_string_lossy().to_string()
        });
        let policy = load_policy(root.path(), payload_obj(&payload));
        let state_path = default_state_path(root.path());
        let audit_path = default_audit_path(root.path());
        let issued = issue_handle(
            root.path(),
            payload_obj(&payload),
            &policy,
            &state_path,
            &audit_path,
        );
        assert_eq!(issued.get("ok").and_then(Value::as_bool), Some(true));
        let resolved = resolve_handle(
            root.path(),
            payload_obj(&json!({
                "handle": issued.get("handle").and_then(Value::as_str).unwrap_or_default(),
                "scope": "scope.test",
                "caller": "caller.test"
            })),
            &policy,
            &state_path,
            &audit_path,
        );
        assert_eq!(resolved.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            resolved.get("value").and_then(Value::as_str),
            Some("mb-test")
        );
    }

    #[test]
    fn load_secret_supports_json_file_provider() {
        let root = temp_root();
        let secret_dir = root.path().join(".secrets");
        fs::create_dir_all(&secret_dir).expect("secret_dir");
        let policy_path = root.path().join("client/runtime/config/secret_broker_policy.json");
        fs::write(
            &policy_path,
            format!(
                "{{\"version\":\"1.0\",\"audit\":{{\"include_backend_details\":true}},\"command_backend\":{{\"timeout_ms\":5000}},\"secrets\":{{\"moltbook_api_key\":{{\"providers\":[{{\"type\":\"json_file\",\"paths\":[\"{}\"],\"field\":\"api_key\",\"rotated_at_field\":\"rotated_at\"}}]}}}}}}",
                secret_dir
                    .join("moltbook.credentials.json")
                    .to_string_lossy()
                    .replace('\\', "\\\\")
            ),
        )
        .expect("policy");
        fs::write(
            secret_dir.join("moltbook.credentials.json"),
            "{\"api_key\":\"json-secret\",\"rotated_at\":\"2026-03-01T00:00:00Z\"}",
        )
        .expect("json");
        let policy = load_policy(
            root.path(),
            payload_obj(&json!({
                "policy_path": policy_path.to_string_lossy().to_string()
            })),
        );
        let loaded = load_secret_by_id(
            root.path(),
            payload_obj(&json!({ "secret_id": "moltbook_api_key" })),
            &policy,
            &default_audit_path(root.path()),
            false,
        );
        assert!(loaded.ok);
        assert_eq!(loaded.value, "json-secret");
    }
}
