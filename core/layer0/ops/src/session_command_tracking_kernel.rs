// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)
//
// Imported pattern contract (RTK intake):
// - source: local/workspace/vendor/rtk/src/core/tracking.rs
// - concept: persisted command telemetry (SQLite) with savings and adoption summaries.

use rusqlite::{params, Connection};
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::contract_lane_utils as lane_utils;
use crate::session_command_discovery_kernel::{
    classify_command_detail_for_kernel, split_command_chain_for_kernel,
};
use crate::now_iso;

fn usage() {
    println!("session-command-tracking-kernel commands:");
    println!(
        "  protheus-ops session-command-tracking-kernel <record|summary|status> [--payload=<json>|--payload-base64=<base64_json>]"
    );
}

fn clean_text(raw: &str, max_len: usize) -> String {
    raw.chars()
        .take(max_len)
        .collect::<String>()
        .trim()
        .to_string()
}

fn db_path(root: &Path, payload: &Map<String, Value>) -> PathBuf {
    if let Some(raw) = payload.get("db_path").and_then(Value::as_str) {
        let cleaned = clean_text(raw, 500);
        if !cleaned.is_empty() {
            let candidate = PathBuf::from(&cleaned);
            if candidate.is_absolute() {
                return candidate;
            }
            return root.join(candidate);
        }
    }
    root.join("local/state/ops/session_command_tracking/tracking.sqlite")
}

fn open_db(path: &Path) -> Result<Connection, String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("session_command_tracking_dir_create_failed:{err}"))?;
    }
    let conn = Connection::open(path)
        .map_err(|err| format!("session_command_tracking_open_failed:{err}"))?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS command_events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            ts TEXT NOT NULL,
            session_id TEXT NOT NULL,
            raw_command TEXT NOT NULL,
            segment TEXT NOT NULL,
            supported INTEGER NOT NULL,
            ignored INTEGER NOT NULL,
            prefixed INTEGER NOT NULL,
            category TEXT,
            canonical TEXT,
            status TEXT,
            savings_pct REAL NOT NULL,
            output_tokens INTEGER NOT NULL,
            estimated_savings_tokens INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_command_events_ts ON command_events(ts);
        CREATE INDEX IF NOT EXISTS idx_command_events_session_id ON command_events(session_id);
        CREATE INDEX IF NOT EXISTS idx_command_events_segment ON command_events(segment);",
    )
    .map_err(|err| format!("session_command_tracking_schema_failed:{err}"))?;
    Ok(conn)
}

#[derive(Clone)]
struct RecordInput {
    session_id: String,
    command: String,
    output_tokens: usize,
}

fn parse_record_inputs(payload: &Map<String, Value>) -> Vec<RecordInput> {
    let default_session = {
        let cleaned = clean_text(
            payload
                .get("session_id")
                .and_then(Value::as_str)
                .unwrap_or("session"),
            120,
        );
        if cleaned.is_empty() {
            "session".to_string()
        } else {
            cleaned
        }
    };
    let default_output = payload
        .get("output_tokens")
        .and_then(Value::as_u64)
        .unwrap_or(0) as usize;
    let mut out = Vec::<RecordInput>::new();
    if let Some(records) = payload.get("records").and_then(Value::as_array) {
        for row in records {
            let Some(obj) = row.as_object() else {
                continue;
            };
            let command = clean_text(
                obj.get("command").and_then(Value::as_str).unwrap_or(""),
                3000,
            );
            if command.is_empty() {
                continue;
            }
            let record_session = clean_text(
                obj.get("session_id")
                    .and_then(Value::as_str)
                    .unwrap_or(default_session.as_str()),
                120,
            );
            out.push(RecordInput {
                session_id: if record_session.is_empty() {
                    default_session.clone()
                } else {
                    record_session
                },
                command,
                output_tokens: obj
                    .get("output_tokens")
                    .and_then(Value::as_u64)
                    .unwrap_or(default_output as u64) as usize,
            });
        }
        return out;
    }
    if let Some(commands) = payload.get("commands").and_then(Value::as_array) {
        for row in commands {
            let command = clean_text(row.as_str().unwrap_or(""), 3000);
            if command.is_empty() {
                continue;
            }
            out.push(RecordInput {
                session_id: default_session.clone(),
                command,
                output_tokens: default_output,
            });
        }
    }
    out
}

fn record_batch(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let records = parse_record_inputs(payload);
    let db = db_path(root, payload);
    let mut conn = open_db(&db)?;
    let tx = conn
        .transaction()
        .map_err(|err| format!("session_command_tracking_tx_failed:{err}"))?;
    let mut inserted = 0usize;
    let mut supported = 0usize;
    let mut unsupported = 0usize;
    let mut ignored = 0usize;
    let mut estimated_savings_tokens = 0usize;
    let now = now_iso();
    for row in records {
        let segments = split_command_chain_for_kernel(row.command.as_str());
        for segment in segments {
            let normalized = clean_text(&segment, 1000);
            if normalized.is_empty() {
                continue;
            }
            let prefixed = normalized.starts_with("rtk ");
            let detail = classify_command_detail_for_kernel(normalized.as_str());
            let is_ignored = detail
                .get("ignored")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let is_supported = prefixed
                || detail
                    .get("supported")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
            let category = clean_text(
                detail.get("category").and_then(Value::as_str).unwrap_or(""),
                80,
            );
            let canonical = clean_text(
                detail
                    .get("canonical")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                120,
            );
            let status = clean_text(
                detail.get("status").and_then(Value::as_str).unwrap_or(""),
                40,
            );
            let savings_pct = detail
                .get("estimated_savings_pct")
                .and_then(Value::as_f64)
                .unwrap_or(0.0);
            let estimated = ((row.output_tokens as f64) * (savings_pct / 100.0)).round() as usize;
            tx.execute(
                "INSERT INTO command_events (ts, session_id, raw_command, segment, supported, ignored, prefixed, category, canonical, status, savings_pct, output_tokens, estimated_savings_tokens)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                params![
                    now,
                    row.session_id,
                    row.command,
                    normalized,
                    if is_supported { 1 } else { 0 },
                    if is_ignored { 1 } else { 0 },
                    if prefixed { 1 } else { 0 },
                    category,
                    canonical,
                    status,
                    savings_pct,
                    row.output_tokens as i64,
                    estimated as i64
                ],
            )
            .map_err(|err| format!("session_command_tracking_insert_failed:{err}"))?;
            inserted += 1;
            if is_ignored {
                ignored += 1;
            } else if is_supported {
                supported += 1;
            } else {
                unsupported += 1;
            }
            estimated_savings_tokens += estimated;
        }
    }
    tx.commit()
        .map_err(|err| format!("session_command_tracking_tx_commit_failed:{err}"))?;
    Ok(json!({
      "ok": true,
      "db_path": db.to_string_lossy().to_string(),
      "inserted": inserted,
      "supported": supported,
      "unsupported": unsupported,
      "ignored": ignored,
      "estimated_savings_tokens": estimated_savings_tokens
    }))
}

fn summary_query_filter(payload: &Map<String, Value>) -> (Option<i64>, Option<String>, Option<String>) {
    let since_days = payload.get("since_days").and_then(Value::as_i64);
    let session = payload
        .get("session_id")
        .and_then(Value::as_str)
        .map(|row| clean_text(row, 120))
        .filter(|row| !row.is_empty());
    let status = payload
        .get("status")
        .or_else(|| payload.get("support_status"))
        .and_then(Value::as_str)
        .map(|row| clean_text(row, 40).to_ascii_lowercase())
        .filter(|row| !row.is_empty());
    (since_days, session, status)
}

fn summary(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let db = db_path(root, payload);
    let conn = open_db(&db)?;
    let (since_days, session_filter, status_filter) = summary_query_filter(payload);
    let mut sql = "SELECT session_id, segment, status, supported, ignored, prefixed, estimated_savings_tokens, output_tokens FROM command_events".to_string();
    let mut where_parts = Vec::<String>::new();
    if let Some(days) = since_days {
        where_parts.push(format!("ts >= datetime('now', '-{} days')", days.max(0)));
    }
    if let Some(session_id) = session_filter.clone() {
        where_parts.push(format!("session_id = '{}'", session_id.replace('\'', "''")));
    }
    if let Some(status) = status_filter.clone() {
        where_parts.push(format!("status = '{}'", status.replace('\'', "''")));
    }
    if !where_parts.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&where_parts.join(" AND "));
    }
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|err| format!("session_command_tracking_prepare_failed:{err}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, i64>(6)?,
                row.get::<_, i64>(7)?,
            ))
        })
        .map_err(|err| format!("session_command_tracking_query_failed:{err}"))?;

    let mut total = 0usize;
    let mut supported = 0usize;
    let mut unsupported = 0usize;
    let mut ignored = 0usize;
    let mut prefixed = 0usize;
    let mut status_existing_rows = 0usize;
    let mut status_passthrough_rows = 0usize;
    let mut status_other_rows = 0usize;
    let mut total_estimated = 0usize;
    let mut total_output = 0usize;
    let mut by_session = HashMap::<String, usize>::new();
    let mut by_segment = HashMap::<String, usize>::new();
    for row in rows {
        let (session_id, segment, status, is_supported, is_ignored, is_prefixed, est, out_tokens) =
            row.map_err(|err| format!("session_command_tracking_query_row_failed:{err}"))?;
        total += 1;
        if is_ignored == 1 {
            ignored += 1;
        } else if is_supported == 1 {
            supported += 1;
        } else {
            unsupported += 1;
        }
        if is_prefixed == 1 {
            prefixed += 1;
        }
        match status.trim().to_ascii_lowercase().as_str() {
            "existing" => status_existing_rows += 1,
            "passthrough" => status_passthrough_rows += 1,
            _ => status_other_rows += 1,
        }
        total_estimated += est.max(0) as usize;
        total_output += out_tokens.max(0) as usize;
        *by_session.entry(session_id).or_insert(0) += 1;
        *by_segment.entry(segment).or_insert(0) += 1;
    }
    let adoption_pct = if total == 0 {
        0.0
    } else {
        (supported as f64 / total as f64) * 100.0
    };
    let supported_coverage_ratio = if total == 0 {
        0.0
    } else {
        supported as f64 / total as f64
    };
    let estimated_savings_pct_of_output = if total_output == 0 {
        0.0
    } else {
        (total_estimated as f64 / total_output as f64) * 100.0
    };
    let mut top_sessions = by_session.into_iter().collect::<Vec<_>>();
    top_sessions.sort_by(|a, b| b.1.cmp(&a.1));
    top_sessions.truncate(10);
    let mut top_segments = by_segment.into_iter().collect::<Vec<_>>();
    top_segments.sort_by(|a, b| b.1.cmp(&a.1));
    top_segments.truncate(10);

    Ok(json!({
      "ok": true,
      "db_path": db.to_string_lossy().to_string(),
      "tracked_rows": total,
      "supported_rows": supported,
      "unsupported_rows": unsupported,
      "ignored_rows": ignored,
      "prefixed_rows": prefixed,
      "status_existing_rows": status_existing_rows,
      "status_passthrough_rows": status_passthrough_rows,
      "status_other_rows": status_other_rows,
      "adoption_pct": adoption_pct,
      "supported_coverage_ratio": supported_coverage_ratio,
      "total_output_tokens": total_output,
      "estimated_savings_tokens": total_estimated,
      "estimated_savings_pct_of_output": estimated_savings_pct_of_output,
      "top_sessions": top_sessions.into_iter().map(|row| json!({"session_id": row.0, "count": row.1})).collect::<Vec<_>>(),
      "top_segments": top_segments.into_iter().map(|row| json!({"segment": row.0, "count": row.1})).collect::<Vec<_>>()
    }))
}

pub(crate) fn record_batch_for_kernel(root: &Path, payload: &Value) -> Result<Value, String> {
    record_batch(root, lane_utils::payload_obj(payload))
}

pub(crate) fn summary_for_kernel(root: &Path, payload: &Value) -> Result<Value, String> {
    summary(root, lane_utils::payload_obj(payload))
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let command = argv
        .first()
        .map(|value| value.to_ascii_lowercase())
        .unwrap_or_else(|| "help".to_string());
    if matches!(command.as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }
    let payload = match lane_utils::payload_json(&argv[1..], "session_command_tracking") {
        Ok(payload) => payload,
        Err(err) => {
            lane_utils::print_json_line(&lane_utils::cli_error("session_command_tracking_kernel_error", &err));
            return 1;
        }
    };
    let input = lane_utils::payload_obj(&payload);
    let result = match command.as_str() {
        "record" => record_batch(root, input),
        "summary" | "status" => summary(root, input),
        _ => Err("session_command_tracking_kernel_unknown_command".to_string()),
    };
    match result {
        Ok(payload) => {
            lane_utils::print_json_line(&lane_utils::cli_receipt(
                &format!(
                    "session_command_tracking_kernel_{}",
                    command.replace('-', "_")
                ),
                payload,
            ));
            0
        }
        Err(err) => {
            lane_utils::print_json_line(&lane_utils::cli_error("session_command_tracking_kernel_error", &err));
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn tracking_round_trip_records_and_summarizes() {
        let tmp = tempdir().expect("tmp");
        let payload = json!({
          "db_path":"tracking.sqlite",
          "session_id":"sess-1",
          "commands":["git status","echo hello","rtk cargo test"],
          "output_tokens":200
        });
        let recorded = record_batch(tmp.path(), lane_utils::payload_obj(&payload)).expect("record");
        assert_eq!(recorded.get("inserted").and_then(Value::as_u64), Some(3));

        let status = summary(
            tmp.path(),
            lane_utils::payload_obj(&json!({"db_path":"tracking.sqlite"})),
        )
        .expect("summary");
        assert_eq!(status.get("tracked_rows").and_then(Value::as_u64), Some(3));
        assert_eq!(
            status.get("supported_rows").and_then(Value::as_u64),
            Some(2)
        );
    }

    #[test]
    fn tracking_summary_reports_status_breakdown_and_status_filter() {
        let tmp = tempdir().expect("tmp");
        let payload = json!({
          "db_path":"tracking.sqlite",
          "session_id":"sess-2",
          "records":[
            {"command":"git status","output_tokens":120},
            {"command":"cargo fmt","output_tokens":120},
            {"command":"unknowncmd deploy","output_tokens":120}
          ]
        });
        let _ = record_batch(tmp.path(), lane_utils::payload_obj(&payload)).expect("record");
        let summary_all = summary(
            tmp.path(),
            lane_utils::payload_obj(&json!({"db_path":"tracking.sqlite"})),
        )
        .expect("summary all");
        assert_eq!(
            summary_all.get("status_existing_rows").and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(
            summary_all
                .get("status_passthrough_rows")
                .and_then(Value::as_u64),
            Some(1)
        );
        let summary_existing = summary(
            tmp.path(),
            lane_utils::payload_obj(&json!({"db_path":"tracking.sqlite","status":"existing"})),
        )
        .expect("summary existing");
        assert_eq!(
            summary_existing.get("tracked_rows").and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(
            summary_existing
                .get("status_existing_rows")
                .and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(
            summary_existing
                .get("status_passthrough_rows")
                .and_then(Value::as_u64),
            Some(0)
        );
    }
}
