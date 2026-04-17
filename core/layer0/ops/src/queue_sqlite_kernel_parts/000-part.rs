// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use rusqlite::{params, Connection};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use crate::contract_lane_utils as lane_utils;
use crate::{deterministic_receipt_hash, now_iso};

#[derive(Clone, Debug)]
struct SqliteCfg {
    db_path: PathBuf,
    journal_mode: String,
    synchronous: String,
    busy_timeout_ms: u64,
}

fn usage() {
    println!("queue-sqlite-kernel commands:");
    println!(
        "  protheus-ops queue-sqlite-kernel <open|ensure-schema|migrate-history|upsert-item|append-event|insert-receipt|queue-stats> [--payload-base64=<base64_json>]"
    );
}

fn with_receipt_hash(mut value: Value) -> Value {
    value["receipt_hash"] = Value::String(deterministic_receipt_hash(&value));
    value
}

fn cli_receipt(kind: &str, payload: Value) -> Value {
    let ts = now_iso();
    let ok = payload.get("ok").and_then(Value::as_bool).unwrap_or(true);
    with_receipt_hash(json!({
        "ok": ok,
        "type": kind,
        "ts": ts,
        "date": ts[..10].to_string(),
        "payload": payload
    }))
}

fn cli_error(kind: &str, error: &str) -> Value {
    let ts = now_iso();
    with_receipt_hash(json!({
        "ok": false,
        "type": kind,
        "ts": ts,
        "date": ts[..10].to_string(),
        "error": error,
        "fail_closed": true
    }))
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
            .map_err(|err| format!("queue_sqlite_kernel_payload_decode_failed:{err}"));
    }
    if let Some(raw_b64) = lane_utils::parse_flag(argv, "payload-base64", false) {
        let bytes = BASE64_STANDARD
            .decode(raw_b64.as_bytes())
            .map_err(|err| format!("queue_sqlite_kernel_payload_base64_decode_failed:{err}"))?;
        let text = String::from_utf8(bytes)
            .map_err(|err| format!("queue_sqlite_kernel_payload_utf8_decode_failed:{err}"))?;
        return serde_json::from_str::<Value>(&text)
            .map_err(|err| format!("queue_sqlite_kernel_payload_decode_failed:{err}"));
    }
    Ok(json!({}))
}

fn payload_obj<'a>(value: &'a Value) -> &'a Map<String, Value> {
    value.as_object().unwrap_or_else(|| {
        static EMPTY: std::sync::OnceLock<Map<String, Value>> = std::sync::OnceLock::new();
        EMPTY.get_or_init(Map::new)
    })
}

fn as_string(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(v)) => v.trim().to_string(),
        Some(Value::Null) | None => String::new(),
        Some(v) => v.to_string().trim_matches('"').trim().to_string(),
    }
}

fn as_i64(value: Option<&Value>, fallback: i64) -> i64 {
    match value {
        Some(Value::Number(v)) => v.as_i64().unwrap_or(fallback),
        Some(Value::String(v)) => v.trim().parse::<i64>().unwrap_or(fallback),
        _ => fallback,
    }
}

fn clean_text(value: Option<&Value>, max_len: usize) -> String {
    let mut out = as_string(value)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if out.len() > max_len {
        out.truncate(max_len);
    }
    out
}

fn canonical_json(value: &Value) -> String {
    match value {
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {
            serde_json::to_string(value).unwrap_or_else(|_| "null".to_string())
        }
        Value::Array(items) => {
            let mut out = String::from("[");
            for (idx, item) in items.iter().enumerate() {
                if idx > 0 {
                    out.push(',');
                }
                out.push_str(&canonical_json(item));
            }
            out.push(']');
            out
        }
        Value::Object(map) => {
            let mut keys = map.keys().cloned().collect::<Vec<_>>();
            keys.sort();
            let mut out = String::from("{");
            for (idx, key) in keys.iter().enumerate() {
                if idx > 0 {
                    out.push(',');
                }
                out.push_str(&serde_json::to_string(key).unwrap_or_else(|_| "\"\"".to_string()));
                out.push(':');
                out.push_str(&canonical_json(map.get(key).unwrap_or(&Value::Null)));
            }
            out.push('}');
            out
        }
    }
}

fn sha256_hex(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    hex::encode(hasher.finalize())
}

fn normalize_queue_name(raw: &str) -> String {
    let lowered = raw.trim().to_ascii_lowercase();
    let mut out = String::new();
    let mut prev_underscore = false;
    for ch in lowered.chars() {
        let mapped = if ch.is_ascii_alphanumeric() || matches!(ch, '.' | ':' | '-') {
            prev_underscore = false;
            ch
        } else if prev_underscore {
            continue;
        } else {
            prev_underscore = true;
            '_'
        };
        out.push(mapped);
    }
    let trimmed = out.trim_matches('_').to_string();
    if trimmed.is_empty() {
        "default_queue".to_string()
    } else {
        trimmed
    }
}

fn clean_lane_id(raw: &str) -> String {
    raw.trim().to_ascii_uppercase()
}

fn sanitize_journal_mode(raw: &str) -> String {
    match raw.trim().to_ascii_uppercase().as_str() {
        "DELETE" => "DELETE".to_string(),
        "TRUNCATE" => "TRUNCATE".to_string(),
        "PERSIST" => "PERSIST".to_string(),
        "MEMORY" => "MEMORY".to_string(),
        "WAL" => "WAL".to_string(),
        "OFF" => "OFF".to_string(),
        _ => "WAL".to_string(),
    }
}

fn sanitize_synchronous(raw: &str) -> String {
    match raw.trim().to_ascii_uppercase().as_str() {
        "OFF" => "OFF".to_string(),
        "NORMAL" => "NORMAL".to_string(),
        "FULL" => "FULL".to_string(),
        "EXTRA" => "EXTRA".to_string(),
        _ => "NORMAL".to_string(),
    }
}

fn sqlite_cfg_from_payload(root: &Path, payload: &Map<String, Value>) -> Result<SqliteCfg, String> {
    let source = payload
        .get("sqlite_cfg")
        .and_then(Value::as_object)
        .or_else(|| {
            payload
                .get("db")
                .and_then(Value::as_object)
                .and_then(|db| db.get("sqlite_cfg"))
                .and_then(Value::as_object)
        })
        .unwrap_or(payload);
    let db_path_raw = clean_text(source.get("db_path"), 520);
    if db_path_raw.is_empty() {
        return Err("queue_sqlite_db_path_required".to_string());
    }
    let db_path = {
        let candidate = PathBuf::from(&db_path_raw);
        if candidate.is_absolute() {
            candidate
        } else {
            root.join(candidate)
        }
    };
    Ok(SqliteCfg {
        db_path,
        journal_mode: sanitize_journal_mode(&clean_text(source.get("journal_mode"), 24)),
        synchronous: sanitize_synchronous(&clean_text(source.get("synchronous"), 24)),
        busy_timeout_ms: as_i64(source.get("busy_timeout_ms"), 5000).clamp(100, 120_000) as u64,
    })
}

fn cfg_to_value(cfg: &SqliteCfg) -> Value {
    json!({
        "db_path": cfg.db_path.to_string_lossy(),
        "journal_mode": cfg.journal_mode,
        "synchronous": cfg.synchronous,
        "busy_timeout_ms": cfg.busy_timeout_ms
    })
}

fn ensure_parent(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("queue_sqlite_kernel_create_dir_failed:{err}"))?;
    }
    Ok(())
}

fn open_connection(cfg: &SqliteCfg) -> Result<Connection, String> {
    ensure_parent(&cfg.db_path)?;
    let conn = Connection::open(&cfg.db_path)
        .map_err(|err| format!("queue_sqlite_kernel_open_failed:{err}"))?;
    conn.execute_batch(&format!(
        "PRAGMA busy_timeout={};PRAGMA journal_mode={};PRAGMA synchronous={};PRAGMA foreign_keys=ON;",
        cfg.busy_timeout_ms, cfg.journal_mode, cfg.synchronous
    ))
    .map_err(|err| format!("queue_sqlite_kernel_pragma_failed:{err}"))?;
    Ok(conn)
}

fn execute_batch_with_retry(conn: &Connection, sql: &str) -> Result<(), String> {
    let mut attempt = 0u32;
    loop {
        match conn.execute_batch(sql) {
            Ok(()) => return Ok(()),
            Err(err) => {
                let msg = err.to_string().to_ascii_lowercase();
                if !msg.contains("database is locked") || attempt >= 6 {
                    return Err(format!("queue_sqlite_kernel_exec_failed:{err}"));
                }
                thread::sleep(Duration::from_millis(20 * 2u64.pow(attempt)));
                attempt += 1;
            }
        }
    }
}

fn ensure_schema(conn: &Connection) -> Result<(), String> {
    execute_batch_with_retry(
        conn,
        r#"
        CREATE TABLE IF NOT EXISTS queue_schema_migrations (
          migration_id TEXT PRIMARY KEY,
          applied_at TEXT NOT NULL,
          detail_json TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS backlog_queue_items (
          lane_id TEXT PRIMARY KEY,
          queue_name TEXT NOT NULL,
          class TEXT,
          wave TEXT,
          status TEXT NOT NULL,
          title TEXT,
          dependencies_json TEXT NOT NULL,
          payload_json TEXT NOT NULL,
          updated_at TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_backlog_queue_lookup
          ON backlog_queue_items(queue_name, status, updated_at DESC);
        CREATE TABLE IF NOT EXISTS backlog_queue_events (
          event_id TEXT PRIMARY KEY,
          queue_name TEXT NOT NULL,
          lane_id TEXT,
          event_type TEXT NOT NULL,
          payload_json TEXT NOT NULL,
          ts TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_backlog_queue_events_lookup
          ON backlog_queue_events(queue_name, ts DESC);
        CREATE TABLE IF NOT EXISTS backlog_queue_receipts (
          receipt_id TEXT PRIMARY KEY,
          lane_id TEXT NOT NULL,
          receipt_json TEXT NOT NULL,
          ts TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_backlog_queue_receipts_lane
          ON backlog_queue_receipts(lane_id, ts DESC);
        "#,
    )
}

fn migration_already_applied(conn: &Connection, migration_id: &str) -> Result<bool, String> {
    let mut stmt = conn
        .prepare("SELECT migration_id FROM queue_schema_migrations WHERE migration_id = ?1 LIMIT 1")
        .map_err(|err| format!("queue_sqlite_kernel_prepare_failed:{err}"))?;
    let mut rows = stmt
        .query(params![migration_id])
        .map_err(|err| format!("queue_sqlite_kernel_query_failed:{err}"))?;
    rows.next()
        .map_err(|err| format!("queue_sqlite_kernel_query_failed:{err}"))
        .map(|row| row.is_some())
}

fn mark_migration_applied(
    conn: &Connection,
    migration_id: &str,
    detail: &Value,
) -> Result<(), String> {
    conn.execute(
        "INSERT OR REPLACE INTO queue_schema_migrations (migration_id, applied_at, detail_json) VALUES (?1, ?2, ?3)",
        params![migration_id, now_iso(), canonical_json(detail)],
    )
    .map_err(|err| format!("queue_sqlite_kernel_insert_failed:{err}"))?;
    Ok(())
}

fn read_jsonl_rows(path: &Path) -> Vec<Value> {
    let raw = match fs::read_to_string(path) {
        Ok(value) => value,
        Err(_) => return Vec::new(),
    };
    raw.lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .collect()
}

fn migrate_history(
    conn: &mut Connection,
    history_path: &Path,
    queue_name: &str,
