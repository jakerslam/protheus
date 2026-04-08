// Layer ownership: core/layer2/ops (authoritative)
// SPDX-License-Identifier: Apache-2.0
use crate::{deterministic_receipt_hash, now_epoch_ms};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

const DEFAULT_POLICY_REL: &str = "client/runtime/config/public_api_catalog_policy.json";
const DEFAULT_STATE_REL: &str = "local/state/ops/public_api_catalog/state.json";
const DEFAULT_HISTORY_REL: &str = "local/state/ops/public_api_catalog/history.jsonl";
const DEFAULT_FRESHNESS_DAYS: f64 = 14.0;
const DEFAULT_MIN_SYNC_ACTIONS: usize = 1;

const USAGE: &[&str] = &[
    "Usage:",
    "  protheus-ops public-api-catalog status [--state-path=<path>] [--policy=<path>] [--strict=1|0]",
    "  protheus-ops public-api-catalog sync|run [--catalog-path=<path>|--catalog-json=<json>] [--source=<label>] [--state-path=<path>] [--strict=1|0]",
    "  protheus-ops public-api-catalog search --query=<text> [--limit=<n>] [--state-path=<path>]",
    "  protheus-ops public-api-catalog integrate --action-id=<id> [--state-path=<path>] [--strict=1|0]",
    "  protheus-ops public-api-catalog connect --platform=<name> [--connection-key=<key>] [--access-token=<token>] [--refresh-token=<token>] [--expires-epoch-ms=<u64>] [--oauth-passthrough=1|0] [--state-path=<path>]",
    "  protheus-ops public-api-catalog import-flow [--flow-path=<path>|--flow-json=<json>] [--workflow-id=<id>] [--state-path=<path>] [--strict=1|0]",
    "  protheus-ops public-api-catalog run-flow [--workflow-id=<id>|--flow-path=<path>] [--input-json=<json>] [--state-path=<path>] [--strict=1|0]",
    "  protheus-ops public-api-catalog verify [--state-path=<path>] [--max-age-days=<f64>] [--strict=1|0]",
];

#[derive(Debug, Clone)]
struct Policy {
    strict: bool,
    max_age_days: f64,
    min_sync_actions: usize,
    state_path: PathBuf,
    history_path: PathBuf,
    source_catalog_path: Option<PathBuf>,
}

#[derive(Debug, Clone)]
struct CommandResult {
    exit_code: i32,
    payload: Value,
}

fn usage() {
    for row in USAGE {
        println!("{row}");
    }
}

fn print_json_line(value: &Value) {
    println!(
        "{}",
        serde_json::to_string(value)
            .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode_failed\"}".to_string())
    );
}

fn parse_flag(argv: &[String], key: &str) -> Option<String> {
    let pref = format!("--{key}=");
    let long = format!("--{key}");
    let mut idx = 0usize;
    while idx < argv.len() {
        let token = argv[idx].trim();
        if let Some(v) = token.strip_prefix(&pref) {
            return Some(v.to_string());
        }
        if token == long && idx + 1 < argv.len() {
            return Some(argv[idx + 1].clone());
        }
        idx += 1;
    }
    None
}

fn first_positional(argv: &[String], skip: usize) -> Option<String> {
    argv.iter()
        .skip(skip)
        .find(|token| !token.trim_start().starts_with('-'))
        .cloned()
}

fn parse_bool(raw: Option<String>, fallback: bool) -> bool {
    match raw.map(|v| v.trim().to_ascii_lowercase()) {
        Some(v) if ["1", "true", "yes", "on"].contains(&v.as_str()) => true,
        Some(v) if ["0", "false", "no", "off"].contains(&v.as_str()) => false,
        _ => fallback,
    }
}

fn parse_u64(raw: Option<String>) -> Option<u64> {
    raw.and_then(|v| v.trim().parse::<u64>().ok())
}

fn parse_f64(raw: Option<String>) -> Option<f64> {
    raw.and_then(|v| v.trim().parse::<f64>().ok())
}

fn parse_usize(raw: Option<String>, fallback: usize, min: usize, max: usize) -> usize {
    raw.and_then(|v| v.trim().parse::<usize>().ok())
        .map(|v| v.clamp(min, max))
        .unwrap_or(fallback)
}

fn parse_json_flag(argv: &[String], key: &str) -> Option<Value> {
    parse_flag(argv, key).and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
}

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

fn clean_text(value: &str, max_len: usize) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(max_len)
        .collect()
}

fn clean_id(value: &str) -> String {
    let mut out = String::new();
    for ch in value.trim().to_ascii_lowercase().chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-' | ':') {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        "unknown".to_string()
    } else {
        out
    }
}

fn normalize_method(value: &str) -> String {
    match value.trim().to_ascii_uppercase().as_str() {
        "GET" => "GET".to_string(),
        "PUT" => "PUT".to_string(),
        "PATCH" => "PATCH".to_string(),
        "DELETE" => "DELETE".to_string(),
        _ => "POST".to_string(),
    }
}

fn hash_fingerprint(secret: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(secret.as_bytes());
    let digest = hex::encode(hasher.finalize());
    format!("sha256:{}", &digest[..16.min(digest.len())])
}

fn resolve_root(cli_root: &Path) -> PathBuf {
    std::env::var("PUBLIC_API_CATALOG_ROOT")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| cli_root.to_path_buf())
}

fn resolve_path(root: &Path, raw: Option<String>, fallback_rel: &str) -> PathBuf {
    match raw {
        Some(v) if !v.trim().is_empty() => {
            let p = PathBuf::from(v);
            if p.is_absolute() {
                p
            } else {
                root.join(p)
            }
        }
        _ => root.join(fallback_rel),
    }
}

fn rel(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn read_json(path: &Path) -> Option<Value> {
    fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
}

fn write_json_atomic(path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let tmp = path.with_extension(format!("tmp-{}", std::process::id()));
    fs::write(
        &tmp,
        format!(
            "{}\n",
            serde_json::to_string_pretty(value).map_err(|e| e.to_string())?
        ),
    )
    .map_err(|e| e.to_string())?;
    fs::rename(&tmp, path).map_err(|e| e.to_string())
}

fn append_jsonl(path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    let mut line = serde_json::to_string(value).map_err(|e| e.to_string())?;
    line.push('\n');
    file.write_all(line.as_bytes()).map_err(|e| e.to_string())
}

fn with_hash(mut value: Value) -> Value {
    value["receipt_hash"] = Value::String(deterministic_receipt_hash(&value));
    value
}

fn load_policy(root: &Path, argv: &[String]) -> Policy {
    let policy_path = resolve_path(root, parse_flag(argv, "policy"), DEFAULT_POLICY_REL);
    let raw = read_json(&policy_path).unwrap_or_else(|| json!({}));
    let strict = parse_bool(
        parse_flag(argv, "strict"),
        raw.get("strict_fail_closed")
            .and_then(Value::as_bool)
            .unwrap_or(true),
    );
    let max_age_days = parse_f64(parse_flag(argv, "max-age-days"))
        .or_else(|| {
            raw.pointer("/freshness/max_age_days")
                .and_then(Value::as_f64)
        })
        .unwrap_or(DEFAULT_FRESHNESS_DAYS)
        .clamp(1.0, 3650.0);
    let min_sync_actions = parse_usize(
        parse_flag(argv, "min-sync-actions").or_else(|| {
            raw.pointer("/sync/min_actions")
                .and_then(Value::as_u64)
                .map(|v| v.to_string())
        }),
        DEFAULT_MIN_SYNC_ACTIONS,
        1,
        100_000,
    );
    let state_path = resolve_path(
        root,
        parse_flag(argv, "state-path").or_else(|| {
            raw.pointer("/outputs/state_path")
                .and_then(Value::as_str)
                .map(ToString::to_string)
        }),
        DEFAULT_STATE_REL,
    );
    let history_path = resolve_path(
        root,
        parse_flag(argv, "history-path").or_else(|| {
            raw.pointer("/outputs/history_path")
                .and_then(Value::as_str)
                .map(ToString::to_string)
        }),
        DEFAULT_HISTORY_REL,
    );
    let source_catalog_path = parse_flag(argv, "catalog-path")
        .or_else(|| {
            raw.pointer("/source/catalog_path")
                .and_then(Value::as_str)
                .map(ToString::to_string)
        })
        .map(PathBuf::from);
    Policy {
        strict,
        max_age_days,
        min_sync_actions,
        state_path,
        history_path,
        source_catalog_path,
    }
}

fn default_state() -> Value {
    json!({
        "version": "1.0",
        "synced_epoch_ms": 0,
        "last_verified_epoch_ms": 0,
        "source_ref": "",
        "actions": [],
        "connections": [],
        "workflows": [],
        "recent_events": []
    })
}

fn load_state(path: &Path) -> Result<Value, String> {
    if !path.exists() {
        return Ok(default_state());
    }
    let raw = fs::read_to_string(path).map_err(|e| format!("state_read_failed:{e}"))?;
    let mut parsed =
        serde_json::from_str::<Value>(&raw).map_err(|e| format!("state_parse_failed:{e}"))?;
    if !parsed.is_object() {
        parsed = default_state();
    }
    for key in ["actions", "connections", "workflows", "recent_events"] {
        if parsed.get(key).and_then(Value::as_array).is_none() {
            parsed[key] = Value::Array(Vec::new());
        }
    }
    Ok(parsed)
}

fn save_state(path: &Path, state: &Value) -> Result<(), String> {
    write_json_atomic(path, state)
}

fn event(kind: &str, detail: Value) -> Value {
    json!({
        "ts_epoch_ms": now_epoch_ms(),
        "ts": now_iso(),
        "kind": kind,
        "detail": detail
    })
}

fn push_event(state: &mut Value, kind: &str, detail: Value) {
    let rows = state
        .get_mut("recent_events")
        .and_then(Value::as_array_mut)
        .expect("recent_events array ensured");
    rows.push(event(kind, detail));
    if rows.len() > 100 {
        let excess = rows.len() - 100;
        rows.drain(0..excess);
    }
}

fn builtin_actions(now_ms: u64) -> Vec<Value> {
    vec![
        json!({
            "id": "github.issues.create",
            "platform": "github",
            "title": "Create GitHub Issue",
            "description": "Create an issue in a repository.",
            "method": "POST",
            "url": "https://api.github.com/repos/{owner}/{repo}/issues",
            "parameters": {"required":["owner","repo","title"],"optional":["body","labels","assignees"]},
            "auth": {"type":"oauth","scope":["repo"]},
            "enforcement_rules": {"rate_limit_per_minute":60},
            "response_schema": {"type":"object","required":["id","html_url","number"]},
            "examples": [{"owner":"protheuslabs","repo":"InfRing","title":"Bug report"}],
            "tags": ["github","issues","tracker"],
            "updated_epoch_ms": now_ms,
            "source": "builtin_seed",
            "verified": true
        }),
        json!({
            "id": "slack.chat.post_message",
            "platform": "slack",
            "title": "Post Slack Message",
            "description": "Send a message to a Slack channel.",
            "method": "POST",
            "url": "https://slack.com/api/chat.postMessage",
            "parameters": {"required":["channel","text"],"optional":["thread_ts","blocks"]},
            "auth": {"type":"oauth","scope":["chat:write"]},
            "enforcement_rules": {"rate_limit_per_minute":50},
            "response_schema": {"type":"object","required":["ok","ts"]},
            "examples": [{"channel":"#alerts","text":"deploy complete"}],
            "tags": ["slack","chat"],
            "updated_epoch_ms": now_ms,
            "source": "builtin_seed",
            "verified": true
        }),
        json!({
            "id": "gmail.messages.send",
            "platform": "gmail",
            "title": "Send Gmail Message",
            "description": "Send an email using Gmail API.",
            "method": "POST",
            "url": "https://gmail.googleapis.com/gmail/v1/users/me/messages/send",
            "parameters": {"required":["to","subject","body"],"optional":["cc","bcc","attachments"]},
            "auth": {"type":"oauth","scope":["gmail.send"]},
            "enforcement_rules": {"rate_limit_per_minute":30},
            "response_schema": {"type":"object","required":["id","threadId"]},
            "examples": [{"to":"ops@example.com","subject":"Status","body":"All green"}],
            "tags": ["gmail","email"],
            "updated_epoch_ms": now_ms,
            "source": "builtin_seed",
            "verified": true
        }),
    ]
}

fn normalize_action(raw: &Value, source: &str, now_ms: u64) -> Option<Value> {
    let id = raw
        .get("id")
        .and_then(Value::as_str)
        .map(clean_id)
        .unwrap_or_default();
    let platform = raw
        .get("platform")
        .and_then(Value::as_str)
        .map(clean_id)
        .unwrap_or_default();
    let url = raw
        .get("url")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    if id.is_empty() || platform.is_empty() || url.is_empty() {
        return None;
    }
    let tags = raw
        .get("tags")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(clean_id)
                .filter(|v| !v.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    Some(json!({
        "id": id,
        "platform": platform,
        "title": raw.get("title").and_then(Value::as_str).unwrap_or("").trim(),
        "description": raw.get("description").and_then(Value::as_str).unwrap_or("").trim(),
        "method": normalize_method(raw.get("method").and_then(Value::as_str).unwrap_or("POST")),
        "url": url,
        "parameters": raw.get("parameters").cloned().unwrap_or_else(|| json!({})),
        "auth": raw.get("auth").cloned().unwrap_or_else(|| json!({})),
        "enforcement_rules": raw.get("enforcement_rules").cloned().or_else(|| raw.get("enforcement").cloned()).unwrap_or_else(|| json!({})),
        "response_schema": raw.get("response_schema").cloned().or_else(|| raw.get("response").cloned()).unwrap_or_else(|| json!({})),
        "examples": raw.get("examples").and_then(Value::as_array).cloned().unwrap_or_default(),
        "tags": tags,
        "updated_epoch_ms": raw.get("updated_epoch_ms").and_then(Value::as_u64).unwrap_or(now_ms),
        "source": raw.get("source").and_then(Value::as_str).unwrap_or(source),
        "verified": raw.get("verified").and_then(Value::as_bool).unwrap_or(true)
    }))
}

fn parse_actions(
    root: &Path,
    argv: &[String],
    policy: &Policy,
) -> Result<(Vec<Value>, String), String> {
    let source_label =
        parse_flag(argv, "source").unwrap_or_else(|| "one_knowledge_sync".to_string());
    let now_ms = now_epoch_ms();
    if let Some(raw_json) = parse_flag(argv, "catalog-json") {
        let parsed = serde_json::from_str::<Value>(&raw_json)
            .map_err(|e| format!("catalog_json_parse_failed:{e}"))?;
        let rows = parsed
            .get("actions")
            .and_then(Value::as_array)
            .cloned()
            .or_else(|| parsed.as_array().cloned())
            .unwrap_or_default();
        let actions = rows
            .iter()
            .filter_map(|row| normalize_action(row, &source_label, now_ms))
            .collect::<Vec<_>>();
        return Ok((actions, source_label));
    }

    let catalog_path = parse_flag(argv, "catalog-path")
        .map(PathBuf::from)
        .or_else(|| policy.source_catalog_path.clone());
    if let Some(path) = catalog_path {
        let resolved = if path.is_absolute() {
            path
        } else {
            root.join(path)
        };
        if let Some(parsed) = read_json(&resolved) {
            let rows = parsed
                .get("actions")
                .and_then(Value::as_array)
                .cloned()
                .or_else(|| parsed.as_array().cloned())
                .unwrap_or_default();
            let actions = rows
                .iter()
                .filter_map(|row| normalize_action(row, &source_label, now_ms))
                .collect::<Vec<_>>();
            return Ok((actions, rel(root, &resolved)));
        }
    }

    Ok((builtin_actions(now_ms), "builtin_seed".to_string()))
}

fn lane_receipt(
    lane_type: &str,
    command: &str,
    argv: &[String],
    payload: Value,
    root: &Path,
    policy: &Policy,
) -> Value {
    with_hash(json!({
        "ok": payload.get("ok").and_then(Value::as_bool).unwrap_or(true),
        "type": lane_type,
        "lane": "public_api_catalog",
        "ts_epoch_ms": now_epoch_ms(),
        "ts": now_iso(),
        "command": command,
        "argv": argv,
        "root": root.to_string_lossy(),
        "state_path": rel(root, &policy.state_path),
        "history_path": rel(root, &policy.history_path),
        "strict_fail_closed": policy.strict,
        "payload": payload,
        "claim_evidence": [{
            "id": "v6_tooling_047",
            "claim": "human_verified_action_schemas_are_routed_through_core_authority_with_receipts",
            "evidence": {"layer":"core/layer2/ops","route":"conduit","catalog":"public_api_catalog"}
        }]
    }))
}

fn err(
    root: &Path,
    policy: &Policy,
    command: &str,
    argv: &[String],
    code: &str,
    message: &str,
    exit_code: i32,
) -> CommandResult {
    CommandResult {
        exit_code,
        payload: lane_receipt(
            "public_api_catalog_error",
            command,
            argv,
            json!({"ok":false,"code":code,"error":clean_text(message,320),"routed_via":"conduit"}),
            root,
            policy,
        ),
    }
}

fn action_is_stale(action: &Value, now_ms: u64, max_age_days: f64) -> bool {
    let updated = action
        .get("updated_epoch_ms")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    if updated == 0 {
        return true;
    }
    let max_age_ms = (max_age_days * 24.0 * 60.0 * 60.0 * 1000.0).round() as u64;
    now_ms.saturating_sub(updated) > max_age_ms
}

fn action_template(action: &Value) -> Value {
    let platform = action
        .get("platform")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let mut headers = Map::new();
    headers.insert(
        "Content-Type".to_string(),
        Value::String("application/json".to_string()),
    );
    headers.insert(
        "Authorization".to_string(),
        Value::String(format!(
            "Bearer {{{{connection.{}.access_token}}}}",
            platform
        )),
    );
    json!({
        "action_id": action.get("id").cloned().unwrap_or(Value::Null),
        "platform": platform,
        "method": action.get("method").cloned().unwrap_or_else(|| Value::String("POST".to_string())),
        "url": action.get("url").cloned().unwrap_or(Value::Null),
        "headers": Value::Object(headers),
        "parameters": action.get("parameters").cloned().unwrap_or_else(|| json!({})),
        "response_schema": action.get("response_schema").cloned().unwrap_or_else(|| json!({})),
        "enforcement_rules": action.get("enforcement_rules").cloned().unwrap_or_else(|| json!({})),
        "examples": action.get("examples").cloned().unwrap_or_else(|| json!([]))
    })
}

fn lookup_json_path<'a>(root: &'a Value, expr: &str) -> Option<&'a Value> {
    let trimmed = expr.trim();
    if trimmed.is_empty() || trimmed == "$" {
        return Some(root);
    }
    let path = trimmed
        .strip_prefix("$.")
        .or_else(|| trimmed.strip_prefix('$'))?;
    let mut cur = root;
    for segment in path.split('.') {
        if segment.is_empty() {
            continue;
        }
        if let Some(idx_start) = segment.find('[') {
            let key = &segment[..idx_start];
            if !key.is_empty() {
                cur = cur.get(key)?;
            }
            let idx_end = segment[idx_start + 1..].find(']')?;
            let idx = segment[idx_start + 1..idx_start + 1 + idx_end]
                .parse::<usize>()
                .ok()?;
            cur = cur.get(idx)?;
        } else {
            cur = cur.get(segment)?;
        }
    }
    Some(cur)
}

fn truthy(value: &Value) -> bool {
    match value {
        Value::Bool(v) => *v,
        Value::Number(v) => v.as_f64().map(|n| n != 0.0).unwrap_or(false),
        Value::String(v) => !v.trim().is_empty(),
        Value::Array(v) => !v.is_empty(),
        Value::Object(v) => !v.is_empty(),
        Value::Null => false,
    }
}
