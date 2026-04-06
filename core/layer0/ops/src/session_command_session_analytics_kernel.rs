// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)
//
// Imported pattern contract (RTK intake):
// - source: local/workspace/vendor/rtk/src/discover/provider.rs
// - source: local/workspace/vendor/rtk/src/discover/mod.rs
// - source: local/workspace/vendor/rtk/src/analytics/session_cmd.rs
// - concept: provider transcript extraction + session-level command adoption analytics.

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::path::Path;

use crate::contract_lane_utils as lane_utils;
use crate::session_command_discovery_kernel::{
    classify_command_detail_for_kernel, classify_command_list_for_kernel,
    split_command_chain_for_kernel,
};
use crate::{deterministic_receipt_hash, now_iso};

#[derive(Debug, Clone)]
struct ExtractedCommand {
    command: String,
    output_len: Option<usize>,
    output_preview: Option<String>,
    is_error: bool,
    sequence_index: usize,
}

fn usage() {
    println!("session-command-session-analytics-kernel commands:");
    println!(
        "  protheus-ops session-command-session-analytics-kernel <extract-jsonl|classify-jsonl|adoption-report> [--payload=<json>|--payload-base64=<base64_json>]"
    );
}

fn clean_text(raw: &str, max_len: usize) -> String {
    raw.chars()
        .take(max_len)
        .collect::<String>()
        .trim()
        .to_string()
}

fn print_json_line(value: &Value) {
    println!(
        "{}",
        serde_json::to_string(value)
            .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode_failed\"}".to_string())
    );
}

fn cli_receipt(kind: &str, payload: Value) -> Value {
    let ts = now_iso();
    let ok = payload.get("ok").and_then(Value::as_bool).unwrap_or(true);
    let mut out = json!({
        "ok": ok,
        "type": kind,
        "ts": ts,
        "date": ts[..10].to_string(),
        "payload": payload
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
        "fail_closed": true
    });
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    out
}

fn payload_json(argv: &[String]) -> Result<Value, String> {
    if let Some(raw) = lane_utils::parse_flag(argv, "payload", false) {
        return serde_json::from_str::<Value>(&raw)
            .map_err(|err| format!("session_analytics_payload_decode_failed:{err}"));
    }
    if let Some(raw_b64) = lane_utils::parse_flag(argv, "payload-base64", false) {
        let bytes = BASE64_STANDARD
            .decode(raw_b64.as_bytes())
            .map_err(|err| format!("session_analytics_payload_base64_decode_failed:{err}"))?;
        let text = String::from_utf8(bytes)
            .map_err(|err| format!("session_analytics_payload_utf8_decode_failed:{err}"))?;
        return serde_json::from_str::<Value>(&text)
            .map_err(|err| format!("session_analytics_payload_decode_failed:{err}"));
    }
    Ok(json!({}))
}

fn payload_obj<'a>(value: &'a Value) -> &'a Map<String, Value> {
    value.as_object().unwrap_or_else(|| {
        static EMPTY: std::sync::OnceLock<Map<String, Value>> = std::sync::OnceLock::new();
        EMPTY.get_or_init(Map::new)
    })
}

fn extract_commands_from_jsonl(session_id: &str, jsonl: &str) -> Vec<ExtractedCommand> {
    let mut pending_tool_uses = Vec::<(String, String, usize)>::new();
    let mut tool_results = HashMap::<String, (usize, String, bool)>::new();
    let mut sequence_counter = 0usize;

    for line in jsonl.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if !trimmed.contains("\"Bash\"") && !trimmed.contains("\"tool_result\"") {
            continue;
        }
        let parsed = match serde_json::from_str::<Value>(trimmed) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let entry_type = parsed
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_ascii_lowercase();
        match entry_type.as_str() {
            "assistant" => {
                let Some(content) = parsed.pointer("/message/content").and_then(Value::as_array)
                else {
                    continue;
                };
                for block in content {
                    if block.get("type").and_then(Value::as_str) != Some("tool_use") {
                        continue;
                    }
                    if block.get("name").and_then(Value::as_str) != Some("Bash") {
                        continue;
                    }
                    let Some(tool_id) = block.get("id").and_then(Value::as_str) else {
                        continue;
                    };
                    let Some(command) = block.pointer("/input/command").and_then(Value::as_str)
                    else {
                        continue;
                    };
                    let normalized = clean_text(command, 2000);
                    if normalized.is_empty() {
                        continue;
                    }
                    pending_tool_uses.push((tool_id.to_string(), normalized, sequence_counter));
                    sequence_counter += 1;
                }
            }
            "user" => {
                let Some(content) = parsed.pointer("/message/content").and_then(Value::as_array)
                else {
                    continue;
                };
                for block in content {
                    if block.get("type").and_then(Value::as_str) != Some("tool_result") {
                        continue;
                    }
                    let Some(tool_id) = block.get("tool_use_id").and_then(Value::as_str) else {
                        continue;
                    };
                    let text = clean_text(
                        block.get("content").and_then(Value::as_str).unwrap_or(""),
                        1000,
                    );
                    let is_error = block
                        .get("is_error")
                        .and_then(Value::as_bool)
                        .unwrap_or(false);
                    tool_results.insert(tool_id.to_string(), (text.len(), text, is_error));
                }
            }
            _ => {}
        }
    }

    let mut out = Vec::<ExtractedCommand>::new();
    for (tool_id, command, sequence_index) in pending_tool_uses {
        let (output_len, output_preview, is_error) = tool_results
            .get(&tool_id)
            .map(|row| (Some(row.0), Some(row.1.clone()), row.2))
            .unwrap_or((None, None, false));
        let _ = session_id;
        out.push(ExtractedCommand {
            command,
            output_len,
            output_preview,
            is_error,
            sequence_index,
        });
    }
    out
}

fn command_list_from_payload(payload: &Map<String, Value>) -> Vec<String> {
    let mut out = Vec::<String>::new();
    if let Some(commands) = payload.get("commands").and_then(Value::as_array) {
        for row in commands {
            let command = clean_text(row.as_str().unwrap_or(""), 2000);
            if !command.is_empty() {
                out.push(command);
            }
        }
    }
    out
}

fn build_adoption_for_commands(
    session_id: &str,
    commands: &[String],
    output_tokens: usize,
) -> Value {
    let mut total = 0usize;
    let mut prefixed = 0usize;
    let mut supported = 0usize;
    let mut unsupported = 0usize;
    let mut ignored = 0usize;

    for raw in commands {
        for segment in split_command_chain_for_kernel(raw) {
            let trimmed = clean_text(&segment, 600);
            if trimmed.is_empty() {
                continue;
            }
            total += 1;
            if trimmed.starts_with("rtk ") {
                prefixed += 1;
                supported += 1;
                continue;
            }
            let detail = classify_command_detail_for_kernel(&trimmed);
            if detail
                .get("ignored")
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                ignored += 1;
                continue;
            }
            if detail
                .get("supported")
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                supported += 1;
            } else {
                unsupported += 1;
            }
        }
    }

    let adoption_pct = if total == 0 {
        0.0
    } else {
        (supported as f64 / total as f64) * 100.0
    };

    json!({
      "session_id": clean_text(session_id, 120),
      "total_commands": total,
      "supported_commands": supported,
      "prefixed_commands": prefixed,
      "unsupported_commands": unsupported,
      "ignored_commands": ignored,
      "adoption_pct": adoption_pct,
      "output_tokens": output_tokens
    })
}

fn build_adoption_report(payload: &Map<String, Value>, limit: usize) -> Value {
    let mut rows = Vec::<Value>::new();
    if let Some(sessions) = payload.get("sessions").and_then(Value::as_array) {
        for row in sessions {
            let Some(obj) = row.as_object() else {
                continue;
            };
            let session_id = clean_text(
                obj.get("session_id")
                    .and_then(Value::as_str)
                    .or_else(|| obj.get("id").and_then(Value::as_str))
                    .unwrap_or("session"),
                120,
            );
            if let Some(jsonl) = obj.get("jsonl").and_then(Value::as_str) {
                let extracted = extract_commands_from_jsonl(&session_id, jsonl);
                let commands = extracted
                    .iter()
                    .map(|entry| entry.command.clone())
                    .collect::<Vec<_>>();
                let output_tokens = extracted
                    .iter()
                    .filter_map(|entry| entry.output_len)
                    .sum::<usize>()
                    / 4;
                rows.push(build_adoption_for_commands(
                    &session_id,
                    &commands,
                    output_tokens,
                ));
            } else {
                let commands = command_list_from_payload(obj);
                let output_tokens = obj
                    .get("output_tokens")
                    .and_then(Value::as_u64)
                    .unwrap_or(0) as usize;
                rows.push(build_adoption_for_commands(
                    &session_id,
                    &commands,
                    output_tokens,
                ));
            }
        }
    } else {
        let session_id = clean_text(
            payload
                .get("session_id")
                .and_then(Value::as_str)
                .unwrap_or("session"),
            120,
        );
        if let Some(jsonl) = payload.get("jsonl").and_then(Value::as_str) {
            let extracted = extract_commands_from_jsonl(&session_id, jsonl);
            let commands = extracted
                .iter()
                .map(|entry| entry.command.clone())
                .collect::<Vec<_>>();
            let output_tokens = extracted
                .iter()
                .filter_map(|entry| entry.output_len)
                .sum::<usize>()
                / 4;
            rows.push(build_adoption_for_commands(
                &session_id,
                &commands,
                output_tokens,
            ));
        } else {
            let commands = command_list_from_payload(payload);
            let output_tokens = payload
                .get("output_tokens")
                .and_then(Value::as_u64)
                .unwrap_or(0) as usize;
            rows.push(build_adoption_for_commands(
                &session_id,
                &commands,
                output_tokens,
            ));
        }
    }

    rows.sort_by(|a, b| {
        let ac = a.get("total_commands").and_then(Value::as_u64).unwrap_or(0);
        let bc = b.get("total_commands").and_then(Value::as_u64).unwrap_or(0);
        bc.cmp(&ac)
    });
    rows.truncate(limit.max(1));

    let totals = rows
        .iter()
        .fold((0usize, 0usize, 0usize, 0usize), |acc, row| {
            (
                acc.0
                    + row
                        .get("total_commands")
                        .and_then(Value::as_u64)
                        .unwrap_or(0) as usize,
                acc.1
                    + row
                        .get("supported_commands")
                        .and_then(Value::as_u64)
                        .unwrap_or(0) as usize,
                acc.2
                    + row
                        .get("unsupported_commands")
                        .and_then(Value::as_u64)
                        .unwrap_or(0) as usize,
                acc.3
                    + row
                        .get("output_tokens")
                        .and_then(Value::as_u64)
                        .unwrap_or(0) as usize,
            )
        });

    let adoption_pct = if totals.0 == 0 {
        0.0
    } else {
        (totals.1 as f64 / totals.0 as f64) * 100.0
    };

    json!({
      "ok": true,
      "type": "session_command_adoption_report",
      "sessions_scanned": rows.len(),
      "total_commands": totals.0,
      "supported_commands": totals.1,
      "unsupported_commands": totals.2,
      "total_output_tokens": totals.3,
      "adoption_pct": adoption_pct,
      "sessions": rows
    })
}

fn recommendation_suggestions_from_report(
    payload: &Map<String, Value>,
    limit: usize,
) -> Vec<String> {
    let report = build_adoption_report(payload, limit.max(1));
    let total = report
        .get("total_commands")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    if total == 0 {
        return Vec::new();
    }
    let unsupported = report
        .get("unsupported_commands")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let adoption_pct = report
        .get("adoption_pct")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let output_tokens = report
        .get("total_output_tokens")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let commands = command_list_from_payload(payload);
    let classify = classify_command_list_for_kernel(&commands, 8);
    let unsupported_base = classify
        .get("unsupported")
        .and_then(Value::as_array)
        .and_then(|rows| rows.first())
        .and_then(|row| row.get("base_command"))
        .and_then(Value::as_str)
        .map(|row| clean_text(row, 80))
        .unwrap_or_default();
    let supported_canonical = classify
        .get("supported")
        .and_then(Value::as_array)
        .and_then(|rows| rows.first())
        .and_then(|row| row.get("canonical"))
        .and_then(Value::as_str)
        .map(|row| clean_text(row, 80))
        .unwrap_or_default();

    let mut out = Vec::<String>::new();
    if unsupported > 0 {
        if !unsupported_base.is_empty() {
            out.push(format!(
                "Map `{}` into a supported route.",
                unsupported_base
            ));
        } else {
            out.push("Convert unsupported commands into supported routes.".to_string());
        }
    }
    if adoption_pct < 80.0 {
        out.push("Optimize command flow for higher tool hit rate.".to_string());
    }
    if output_tokens > 1200 {
        out.push("Generate a concise digest of terminal output and next actions.".to_string());
    }
    if !supported_canonical.is_empty() {
        out.push(format!(
            "Run `{}` as the next safe step.",
            supported_canonical
        ));
    }
    if out.is_empty() {
        out.push("Run one focused command and summarize results.".to_string());
    }

    let mut dedup = Vec::<String>::new();
    for row in out {
        let cleaned = clean_text(&row, 180);
        if cleaned.is_empty() {
            continue;
        }
        if dedup
            .iter()
            .any(|existing| existing.eq_ignore_ascii_case(&cleaned))
        {
            continue;
        }
        dedup.push(cleaned);
        if dedup.len() >= limit.max(1) {
            break;
        }
    }
    dedup
}

pub(crate) fn adoption_report_for_kernel(payload: &Value, limit: usize) -> Value {
    build_adoption_report(payload_obj(payload), limit)
}

pub(crate) fn follow_up_suggestions_for_kernel(payload: &Value, limit: usize) -> Vec<String> {
    recommendation_suggestions_from_report(payload_obj(payload), limit)
}

include!("session_command_session_analytics_kernel_parts/020-run-and-tests.rs");
