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
) -> Result<Value, String> {
    ensure_schema(conn)?;
    let migration_id = format!(
        "jsonl_history_to_sqlite:{}",
        history_path
            .canonicalize()
            .unwrap_or_else(|_| history_path.to_path_buf())
            .display()
    );
    if !history_path.exists() {
        return Ok(json!({
            "ok": true,
            "applied": false,
            "skipped": true,
            "reason": "history_path_missing",
            "rows_migrated": 0,
            "migration_id": migration_id
        }));
    }
    if migration_already_applied(conn, &migration_id)? {
        return Ok(json!({
            "ok": true,
            "applied": false,
            "skipped": true,
            "reason": "already_applied",
            "rows_migrated": 0,
            "migration_id": migration_id
        }));
    }

    let rows = read_jsonl_rows(history_path);
    if rows.is_empty() {
        mark_migration_applied(
            conn,
            &migration_id,
            &json!({ "source_path": history_path.to_string_lossy(), "rows_migrated": 0 }),
        )?;
        return Ok(json!({
            "ok": true,
            "applied": true,
            "skipped": false,
            "reason": "empty_source",
            "rows_migrated": 0,
            "migration_id": migration_id
        }));
    }

    let tx = conn
        .transaction()
        .map_err(|err| format!("queue_sqlite_kernel_tx_failed:{err}"))?;
    let mut migrated = 0u64;
    {
        let mut insert = tx
            .prepare(
                "INSERT OR IGNORE INTO backlog_queue_events (event_id, queue_name, lane_id, event_type, payload_json, ts) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            )
            .map_err(|err| format!("queue_sqlite_kernel_prepare_failed:{err}"))?;
        for row in &rows {
            let payload_json = canonical_json(row);
            let event_id = sha256_hex(&payload_json);
            let lane_id = clean_lane_id(&clean_text(
                row.get("lane_id").or_else(|| row.get("id")),
                120,
            ));
            let event_type = clean_text(row.get("action"), 80);
            let ts = clean_text(row.get("ts").or_else(|| row.get("timestamp")), 120);
            let changes = insert
                .execute(params![
                    event_id,
                    normalize_queue_name(queue_name),
                    if lane_id.is_empty() {
                        None::<String>
                    } else {
                        Some(lane_id)
                    },
                    if event_type.is_empty() {
                        "history_import".to_string()
                    } else {
                        event_type
                    },
                    payload_json,
                    if ts.is_empty() { now_iso() } else { ts }
                ])
                .map_err(|err| format!("queue_sqlite_kernel_insert_failed:{err}"))?;
            if changes > 0 {
                migrated += changes as u64;
            }
        }
    }
    tx.commit()
        .map_err(|err| format!("queue_sqlite_kernel_tx_commit_failed:{err}"))?;
    mark_migration_applied(
        conn,
        &migration_id,
        &json!({
            "source_path": history_path.to_string_lossy(),
            "rows_seen": rows.len(),
            "rows_migrated": migrated
        }),
    )?;
    Ok(json!({
        "ok": true,
        "applied": true,
        "skipped": false,
        "reason": "ok",
        "rows_seen": rows.len(),
        "rows_migrated": migrated,
        "migration_id": migration_id
    }))
}

fn upsert_item(
    conn: &Connection,
    queue_name: &str,
    row: &Value,
    status: Option<&str>,
) -> Result<Value, String> {
    ensure_schema(conn)?;
    let lane_id = clean_lane_id(&clean_text(row.get("id"), 120));
    if lane_id.is_empty() {
        return Err("queue_sqlite_lane_id_missing".to_string());
    }
    let payload_json = canonical_json(row);
    let updated_at = now_iso();
    let dependencies = row
        .get("dependencies")
        .cloned()
        .filter(|value| value.is_array())
        .unwrap_or_else(|| json!([]));
    conn.execute(
        r#"
        INSERT INTO backlog_queue_items (
          lane_id, queue_name, class, wave, status, title, dependencies_json, payload_json, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
        ON CONFLICT(lane_id) DO UPDATE SET
          queue_name=excluded.queue_name,
          class=excluded.class,
          wave=excluded.wave,
          status=excluded.status,
          title=excluded.title,
          dependencies_json=excluded.dependencies_json,
          payload_json=excluded.payload_json,
          updated_at=excluded.updated_at
        "#,
        params![
            lane_id,
            normalize_queue_name(queue_name),
            {
                let value = clean_text(row.get("class"), 120);
                if value.is_empty() { None::<String> } else { Some(value) }
            },
            {
                let value = clean_text(row.get("wave"), 80);
                if value.is_empty() { None::<String> } else { Some(value) }
            },
            {
                let raw = status.unwrap_or_else(|| row.get("status").and_then(Value::as_str).unwrap_or("queued"));
                clean_text(Some(&Value::String(raw.to_string())), 40).to_ascii_lowercase()
            },
            {
                let value = clean_text(row.get("title"), 400);
                if value.is_empty() { None::<String> } else { Some(value) }
            },
            canonical_json(&dependencies),
            payload_json,
            updated_at
        ],
    )
    .map_err(|err| format!("queue_sqlite_kernel_insert_failed:{err}"))?;
    Ok(json!({
        "ok": true,
        "lane_id": clean_lane_id(&clean_text(row.get("id"), 120)),
        "updated_at": updated_at
    }))
}

fn append_event(
    conn: &Connection,
    queue_name: &str,
    lane_id: &str,
    event_type: &str,
    payload: &Value,
    ts: Option<&str>,
) -> Result<Value, String> {
    ensure_schema(conn)?;
    let normalized_queue = normalize_queue_name(queue_name);
    let normalized_lane = clean_lane_id(lane_id);
    let normalized_type = if event_type.trim().is_empty() {
        "event".to_string()
    } else {
        clean_text(Some(&Value::String(event_type.to_string())), 80)
    };
    let normalized_ts = if ts.unwrap_or("").trim().is_empty() {
        now_iso()
    } else {
        ts.unwrap().trim().to_string()
    };
    let payload_json = canonical_json(payload);
    let event_id = sha256_hex(&format!(
        "{}|{}|{}|{}|{}",
        normalized_queue, normalized_lane, normalized_type, payload_json, normalized_ts
    ));
    conn.execute(
        "INSERT OR IGNORE INTO backlog_queue_events (event_id, queue_name, lane_id, event_type, payload_json, ts) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            event_id,
            normalized_queue,
            if normalized_lane.is_empty() { None::<String> } else { Some(normalized_lane) },
            normalized_type,
            payload_json,
            normalized_ts
        ],
    )
    .map_err(|err| format!("queue_sqlite_kernel_insert_failed:{err}"))?;
    Ok(json!({ "ok": true, "event_id": event_id }))
}

fn insert_receipt(conn: &Connection, lane_id: &str, receipt: &Value) -> Result<Value, String> {
    ensure_schema(conn)?;
    let payload_json = canonical_json(receipt);
    let receipt_id = sha256_hex(&payload_json);
    let ts = clean_text(receipt.get("ts"), 120);
    let final_ts = if ts.is_empty() { now_iso() } else { ts };
    conn.execute(
        "INSERT OR REPLACE INTO backlog_queue_receipts (receipt_id, lane_id, receipt_json, ts) VALUES (?1, ?2, ?3, ?4)",
        params![receipt_id, clean_lane_id(lane_id), payload_json, final_ts],
    )
    .map_err(|err| format!("queue_sqlite_kernel_insert_failed:{err}"))?;
    Ok(json!({
        "ok": true,
        "receipt_id": receipt_id,
        "ts": final_ts
    }))
}

fn queue_stats(conn: &Connection, queue_name: &str) -> Result<Value, String> {
    ensure_schema(conn)?;
    let normalized_queue = normalize_queue_name(queue_name);
    let items: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM backlog_queue_items WHERE queue_name = ?1",
            params![normalized_queue.clone()],
            |row| row.get(0),
        )
        .map_err(|err| format!("queue_sqlite_kernel_query_failed:{err}"))?;
    let events: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM backlog_queue_events WHERE queue_name = ?1",
            params![normalized_queue.clone()],
            |row| row.get(0),
        )
        .map_err(|err| format!("queue_sqlite_kernel_query_failed:{err}"))?;
    let receipts: i64 = conn
        .query_row("SELECT COUNT(*) FROM backlog_queue_receipts", [], |row| {
            row.get(0)
        })
        .map_err(|err| format!("queue_sqlite_kernel_query_failed:{err}"))?;
    Ok(json!({
        "ok": true,
        "queue_name": normalized_queue,
        "items": items,
        "events": events,
        "receipts": receipts
    }))
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let command = argv
        .first()
        .map(|value| value.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "open".to_string());
    if matches!(command.as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }
    let payload = match payload_json(&argv[1..]) {
        Ok(value) => value,
        Err(err) => {
            print_json_line(&cli_error("queue_sqlite_kernel_error", &err));
            return 1;
        }
    };
    let payload = payload_obj(&payload);
    let cfg = match sqlite_cfg_from_payload(root, payload) {
        Ok(cfg) => cfg,
        Err(err) => {
            print_json_line(&cli_error("queue_sqlite_kernel_error", &err));
            return 1;
        }
    };
    let mut conn = match open_connection(&cfg) {
        Ok(conn) => conn,
        Err(err) => {
            print_json_line(&cli_error("queue_sqlite_kernel_error", &err));
            return 1;
        }
    };

    let result = match command.as_str() {
        "open" => Ok(json!({
            "ok": true,
            "sqlite_cfg": cfg_to_value(&cfg),
            "db_path": cfg.db_path.to_string_lossy(),
            "db_exists": cfg.db_path.exists()
        })),
        "ensure-schema" => ensure_schema(&conn).map(|_| {
            json!({
                "ok": true,
                "sqlite_cfg": cfg_to_value(&cfg),
                "schema_ready": true
            })
        }),
        "migrate-history" => {
            let history_path_raw = clean_text(payload.get("history_path"), 520);
            if history_path_raw.is_empty() {
                Err("queue_sqlite_history_path_required".to_string())
            } else {
                let history_path = {
                    let candidate = PathBuf::from(&history_path_raw);
                    if candidate.is_absolute() {
                        candidate
                    } else {
                        root.join(candidate)
                    }
                };
                let queue_name = clean_text(payload.get("queue_name"), 160);
                migrate_history(
                    &mut conn,
                    &history_path,
                    if queue_name.is_empty() {
                        "backlog_queue_executor"
                    } else {
                        &queue_name
                    },
                )
            }
        }
        "upsert-item" => {
            let row = payload
                .get("row")
                .cloned()
                .unwrap_or_else(|| Value::Object(Map::new()));
            let queue_name = clean_text(payload.get("queue_name"), 160);
            let status = clean_text(payload.get("status"), 40);
            upsert_item(
                &conn,
                if queue_name.is_empty() {
                    "default_queue"
                } else {
                    &queue_name
                },
                &row,
                if status.is_empty() {
                    None
                } else {
                    Some(status.as_str())
                },
            )
        }
        "append-event" => {
            let queue_name = clean_text(payload.get("queue_name"), 160);
            let lane_id = clean_text(payload.get("lane_id"), 120);
            let event_type = clean_text(payload.get("event_type"), 80);
            let event_payload = payload
                .get("payload")
                .cloned()
                .unwrap_or_else(|| Value::Object(Map::new()));
            let ts = clean_text(payload.get("ts"), 120);
            append_event(
                &conn,
                if queue_name.is_empty() {
                    "default_queue"
                } else {
                    &queue_name
                },
                &lane_id,
                &event_type,
                &event_payload,
                if ts.is_empty() {
                    None
                } else {
                    Some(ts.as_str())
                },
            )
        }
        "insert-receipt" => {
            let lane_id = clean_text(payload.get("lane_id"), 120);
            if lane_id.is_empty() {
                Err("queue_sqlite_lane_id_missing".to_string())
            } else {
                let receipt = payload
                    .get("receipt")
                    .cloned()
                    .unwrap_or_else(|| Value::Object(Map::new()));
                insert_receipt(&conn, &lane_id, &receipt)
            }
        }
        "queue-stats" => {
            let queue_name = clean_text(payload.get("queue_name"), 160);
            queue_stats(
                &conn,
                if queue_name.is_empty() {
                    "default_queue"
                } else {
                    &queue_name
                },
            )
        }
        _ => Err(format!("queue_sqlite_kernel_unknown_command:{command}")),
    };

    match result {
        Ok(payload) => {
            print_json_line(&cli_receipt(
                &format!("queue_sqlite_kernel_{}", command.replace('-', "_")),
                payload,
            ));
            0
        }
        Err(err) => {
            print_json_line(&cli_error(
                &format!("queue_sqlite_kernel_{}", command.replace('-', "_")),
                &err,
            ));
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn cfg(dir: &Path) -> SqliteCfg {
        SqliteCfg {
            db_path: dir.join("queue.sqlite"),
            journal_mode: "WAL".to_string(),
            synchronous: "NORMAL".to_string(),
            busy_timeout_ms: 5_000,
        }
    }

    #[test]
    fn queue_sqlite_kernel_round_trip() {
        let temp = tempdir().expect("tempdir");
        let cfg = cfg(temp.path());
        let conn = open_connection(&cfg).expect("open");
        ensure_schema(&conn).expect("schema");

        let row = json!({
            "id": "bl-1",
            "class": "memory",
            "wave": "w1",
            "status": "queued",
            "title": "Ship durable queue",
            "dependencies": ["BL-0"]
        });
        let upserted =
            upsert_item(&conn, "backlog_queue_executor", &row, Some("queued")).expect("upsert");
        assert_eq!(
            upserted.get("lane_id").and_then(Value::as_str),
            Some("BL-1")
        );

        let event = append_event(
            &conn,
            "backlog_queue_executor",
            "BL-1",
            "queued",
            &json!({ "detail": "scheduled" }),
            None,
        )
        .expect("append event");
        assert!(event.get("event_id").and_then(Value::as_str).is_some());

        let receipt = insert_receipt(&conn, "BL-1", &json!({ "ok": true })).expect("receipt");
        assert!(receipt.get("receipt_id").and_then(Value::as_str).is_some());

        let stats = queue_stats(&conn, "backlog_queue_executor").expect("stats");
        assert_eq!(stats.get("items").and_then(Value::as_i64), Some(1));
        assert_eq!(stats.get("events").and_then(Value::as_i64), Some(1));
        assert_eq!(stats.get("receipts").and_then(Value::as_i64), Some(1));
    }

    #[test]
    fn queue_sqlite_kernel_migrates_history_once() {
        let temp = tempdir().expect("tempdir");
        let history_path = temp.path().join("history.jsonl");
        fs::write(
            &history_path,
            format!(
                "{}\n{}\n",
                json!({ "lane_id": "bl-1", "action": "queued", "ts": "2026-03-17T00:00:00Z" }),
                json!({ "lane_id": "bl-1", "action": "started", "ts": "2026-03-17T00:01:00Z" })
            ),
        )
        .expect("history");

        let cfg = cfg(temp.path());
        let mut conn = open_connection(&cfg).expect("open");
        let first =
            migrate_history(&mut conn, &history_path, "backlog_queue_executor").expect("first");
        assert_eq!(first.get("rows_migrated").and_then(Value::as_u64), Some(2));

        let second =
            migrate_history(&mut conn, &history_path, "backlog_queue_executor").expect("second");
        assert_eq!(second.get("skipped").and_then(Value::as_bool), Some(true));
        assert_eq!(
            second.get("reason").and_then(Value::as_str),
            Some("already_applied")
        );
    }
}
