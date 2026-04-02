// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use chrono::Utc;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::contract_lane_utils as lane_utils;

fn usage() {
    println!("local-state-digest-kernel commands:");
    println!("  protheus-ops local-state-digest-kernel preflight --payload-base64=<json>");
    println!("  protheus-ops local-state-digest-kernel collect --payload-base64=<json>");
}

fn clean_text(raw: Option<&str>, max_len: usize) -> String {
    lane_utils::clean_text(raw, max_len)
}

fn sha16(value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    hex::encode(digest)[..16].to_string()
}

fn now_iso() -> String {
    Utc::now().to_rfc3339()
}

fn today_str() -> String {
    Utc::now().format("%Y-%m-%d").to_string()
}

fn read_json_safe(path: &Path) -> Option<Value> {
    let raw = fs::read_to_string(path).ok()?;
    serde_json::from_str::<Value>(&raw).ok()
}

fn read_jsonl_safe(path: &Path) -> Vec<Value> {
    let raw = match fs::read_to_string(path) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    raw.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .collect::<Vec<_>>()
}

fn payload_obj<'a>(value: &'a Value) -> &'a Map<String, Value> {
    lane_utils::payload_obj(value)
}

fn nested_obj<'a>(payload: &'a Map<String, Value>, key: &str) -> Option<&'a Map<String, Value>> {
    payload.get(key).and_then(Value::as_object)
}

fn nested_u64(payload: &Map<String, Value>, key: &str) -> Option<u64> {
    nested_obj(payload, "budgets")
        .and_then(|b| b.get(key))
        .and_then(Value::as_u64)
}

fn resolve_state_dir(root: &Path, payload: &Map<String, Value>) -> PathBuf {
    if let Some(raw) = payload.get("state_dir").and_then(Value::as_str) {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            let candidate = PathBuf::from(trimmed);
            if candidate.is_absolute() {
                return candidate;
            }
            return root.join(candidate);
        }
    }
    root.join("local").join("state")
}

fn resolve_date(payload: &Map<String, Value>) -> String {
    let override_date = payload
        .get("date")
        .and_then(Value::as_str)
        .map(|raw| clean_text(Some(raw), 32))
        .unwrap_or_default();
    if override_date.is_empty() {
        today_str()
    } else {
        override_date
    }
}

fn base_topics(payload: &Map<String, Value>) -> Vec<Value> {
    let defaults = ["automation", "system", "growth"];
    let mut seen = HashSet::<String>::new();
    let mut out = Vec::<Value>::new();

    if let Some(eye) = nested_obj(payload, "eye_config") {
        if let Some(rows) = eye.get("topics").and_then(Value::as_array) {
            for topic in rows {
                if let Some(raw) = topic.as_str() {
                    let topic_clean = clean_text(Some(&raw.to_lowercase()), 80);
                    if topic_clean.is_empty() || !seen.insert(topic_clean.clone()) {
                        continue;
                    }
                    out.push(Value::String(topic_clean));
                    if out.len() >= 5 {
                        return out;
                    }
                }
            }
        }
    }

    for d in defaults {
        let topic_clean = d.to_string();
        if seen.insert(topic_clean.clone()) {
            out.push(Value::String(topic_clean));
        }
        if out.len() >= 5 {
            break;
        }
    }
    out
}

fn normalize_proposals_payload(raw: Option<Value>) -> Vec<Value> {
    match raw {
        Some(Value::Array(rows)) => rows,
        Some(Value::Object(obj)) => obj
            .get("proposals")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

fn proposal_stats(state_dir: &Path, date_str: &str) -> Value {
    let fp = state_dir
        .join("sensory")
        .join("proposals")
        .join(format!("{date_str}.json"));
    let rows = normalize_proposals_payload(read_json_safe(&fp));
    let mut open = 0usize;
    let mut resolved = 0usize;
    for row in rows.iter() {
        let status = row
            .as_object()
            .and_then(|o| o.get("status"))
            .and_then(Value::as_str)
            .map(|s| s.to_lowercase())
            .unwrap_or_else(|| "open".to_string());
        if status == "resolved" || status == "rejected" {
            resolved += 1;
        } else {
            open += 1;
        }
    }
    json!({
        "total": rows.len(),
        "open": open,
        "resolved": resolved,
        "path": fp.display().to_string()
    })
}

fn decision_stats(state_dir: &Path, date_str: &str) -> Value {
    let fp = state_dir
        .join("queue")
        .join("decisions")
        .join(format!("{date_str}.jsonl"));
    let rows = read_jsonl_safe(&fp);
    let mut accepted = 0usize;
    let mut shipped = 0usize;
    let mut no_change = 0usize;
    let mut reverted = 0usize;

    for row in rows {
        let obj = match row.as_object() {
            Some(v) => v,
            None => continue,
        };
        let row_type = obj.get("type").and_then(Value::as_str).unwrap_or("");
        let decision = obj.get("decision").and_then(Value::as_str).unwrap_or("");
        let outcome = obj.get("outcome").and_then(Value::as_str).unwrap_or("");
        if row_type == "decision" && decision == "accept" {
            accepted += 1;
        }
        if row_type == "outcome" && outcome == "shipped" {
            shipped += 1;
        }
        if row_type == "outcome" && outcome == "no_change" {
            no_change += 1;
        }
        if row_type == "outcome" && outcome == "reverted" {
            reverted += 1;
        }
    }

    json!({
        "accepted": accepted,
        "shipped": shipped,
        "no_change": no_change,
        "reverted": reverted,
        "path": fp.display().to_string()
    })
}

fn git_outcome_stats(state_dir: &Path, date_str: &str) -> Value {
    let fp = state_dir
        .join("git")
        .join("outcomes")
        .join(format!("{date_str}.jsonl"));
    let rows = read_jsonl_safe(&fp);
    let latest = rows
        .iter()
        .rev()
        .find(|row| {
            row.as_object()
                .and_then(|o| o.get("type"))
                .and_then(Value::as_str)
                == Some("git_outcomes_ok")
        })
        .cloned();
    let obj = latest.as_ref().and_then(Value::as_object);
    json!({
        "tags_found": obj.and_then(|o| o.get("tags_found")).and_then(Value::as_u64).unwrap_or(0),
        "outcomes_recorded": obj.and_then(|o| o.get("outcomes_recorded")).and_then(Value::as_u64).unwrap_or(0),
        "outcomes_skipped": obj.and_then(|o| o.get("outcomes_skipped")).and_then(Value::as_u64).unwrap_or(0),
        "path": fp.display().to_string()
    })
}

fn outage_stats(state_dir: &Path) -> Value {
    let fp = state_dir.join("sensory").join("eyes").join("registry.json");
    let reg = read_json_safe(&fp).unwrap_or_else(|| json!({}));
    let outage = reg
        .as_object()
        .and_then(|o| o.get("outage_mode"))
        .and_then(Value::as_object);
    json!({
        "active": outage.and_then(|o| o.get("active")).and_then(Value::as_bool).unwrap_or(false),
        "failed_transport_eyes": outage.and_then(|o| o.get("last_failed_transport_eyes")).and_then(Value::as_u64).unwrap_or(0),
        "window_hours": outage.and_then(|o| o.get("last_window_hours")).and_then(Value::as_u64).unwrap_or(0),
        "since": outage.and_then(|o| o.get("since")).cloned().unwrap_or(Value::Null),
        "path": fp.display().to_string()
    })
}

fn preflight(payload: &Map<String, Value>, state_dir: &Path) -> Value {
    let mut checks = Vec::<Value>::new();
    let mut failures = Vec::<Value>::new();

    let max_items = nested_u64(payload, "max_items");
    if max_items.unwrap_or(0) == 0 {
        failures.push(json!({
            "code": "invalid_budget",
            "message": "budgets.max_items must be > 0"
        }));
    } else {
        checks.push(json!({
            "name": "max_items_valid",
            "ok": true,
            "value": max_items.unwrap_or(0)
        }));
    }

    if !state_dir.exists() {
        failures.push(json!({
            "code": "state_missing",
            "message": format!("state directory missing: {}", state_dir.display())
        }));
    } else {
        checks.push(json!({
            "name": "state_dir_present",
            "ok": true
        }));
    }

    json!({
        "ok": failures.is_empty(),
        "parser_type": "local_state_digest",
        "checks": checks,
        "failures": failures
    })
}

fn item_for(
    kind: &str,
    date_str: &str,
    title: &str,
    preview: &str,
    topics: &[Value],
    source_path: &str,
) -> Value {
    let safe_kind = kind
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>();
    let url = format!("https://local.workspace/signals/{date_str}/{safe_kind}");
    let id = sha16(&format!("{date_str}|{safe_kind}|{url}"));
    let title_clean = clean_text(Some(title), 180);
    let preview_clean = clean_text(Some(preview), 240);
    let bytes = (title_clean.len() + preview_clean.len() + source_path.len() + 96).min(1024) as u64;
    json!({
        "collected_at": now_iso(),
        "id": id,
        "url": url,
        "title": title_clean,
        "content_preview": preview_clean,
        "topics": topics.iter().take(5).cloned().collect::<Vec<_>>(),
        "bytes": bytes
    })
}

fn collect(payload: &Map<String, Value>, state_dir: &Path) -> Value {
    let started = Utc::now().timestamp_millis();
    let pf = preflight(payload, state_dir);
    if pf.get("ok").and_then(Value::as_bool) != Some(true) {
        let first = pf
            .get("failures")
            .and_then(Value::as_array)
            .and_then(|rows| rows.first())
            .cloned()
            .unwrap_or_else(
                || json!({ "code": "local_state_preflight_failed", "message": "unknown" }),
            );
        return json!({
            "ok": false,
            "error": first,
            "preflight": pf
        });
    }

    let date_str = resolve_date(payload);
    let max_items = nested_u64(payload, "max_items").unwrap_or(4).clamp(1, 8) as usize;
    let topics = base_topics(payload);

    let backlog_threshold = payload
        .get("backlog_threshold")
        .and_then(Value::as_u64)
        .unwrap_or_else(|| {
            std::env::var("LOCAL_STATE_BACKLOG_ALERT_THRESHOLD")
                .ok()
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(6)
        })
        .max(1);
    let outcome_gap_min = payload
        .get("outcome_gap_accepted_min")
        .and_then(Value::as_u64)
        .unwrap_or_else(|| {
            std::env::var("LOCAL_STATE_OUTCOME_GAP_ACCEPTED_MIN")
                .ok()
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(2)
        })
        .max(1);
    let tagging_gap_min = payload
        .get("tagging_gap_accepted_min")
        .and_then(Value::as_u64)
        .unwrap_or_else(|| {
            std::env::var("LOCAL_STATE_TAGGING_GAP_ACCEPTED_MIN")
                .ok()
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(1)
        })
        .max(1);

    let p = proposal_stats(state_dir, &date_str);
    let d = decision_stats(state_dir, &date_str);
    let g = git_outcome_stats(state_dir, &date_str);
    let o = outage_stats(state_dir);

    let mut candidates = Vec::<Value>::new();

    if o.get("active").and_then(Value::as_bool) == Some(true) {
        candidates.push(item_for(
            "infra_outage",
            &date_str,
            &format!(
                "Stabilize automation infrastructure: outage mode active across {} sensors",
                o.get("failed_transport_eyes")
                    .and_then(Value::as_u64)
                    .unwrap_or(0)
            ),
            &format!(
                "Outage mode has been active since {}. Prioritize resilient transport recovery and deterministic fallback routing.",
                o.get("since").and_then(Value::as_str).unwrap_or("unknown")
            ),
            &topics,
            o.get("path").and_then(Value::as_str).unwrap_or(""),
        ));
    }

    if p.get("open").and_then(Value::as_u64).unwrap_or(0) >= backlog_threshold {
        candidates.push(item_for(
            "proposal_backlog",
            &date_str,
            &format!(
                "Remediate backlog saturation: open={} (threshold={})",
                p.get("open").and_then(Value::as_u64).unwrap_or(0),
                backlog_threshold
            ),
            &format!(
                "Queue backlog exceeded threshold. Snapshot total={}, open={}, resolved={}. Reduce queue pressure with deterministic admission and closeout discipline.",
                p.get("total").and_then(Value::as_u64).unwrap_or(0),
                p.get("open").and_then(Value::as_u64).unwrap_or(0),
                p.get("resolved").and_then(Value::as_u64).unwrap_or(0)
            ),
            &topics,
            p.get("path").and_then(Value::as_str).unwrap_or(""),
        ));
    }

    if d.get("accepted").and_then(Value::as_u64).unwrap_or(0) >= outcome_gap_min
        && d.get("shipped").and_then(Value::as_u64).unwrap_or(0) == 0
    {
        candidates.push(item_for(
            "outcome_gap",
            &date_str,
            &format!(
                "Remediate execution gap: accepted={}, shipped={}",
                d.get("accepted").and_then(Value::as_u64).unwrap_or(0),
                d.get("shipped").and_then(Value::as_u64).unwrap_or(0)
            ),
            &format!(
                "Accepted proposals are not converting to shipped outcomes. no_change={}, reverted={}, recorded={}. Prioritize one accepted proposal to completion with verifiable evidence.",
                d.get("no_change").and_then(Value::as_u64).unwrap_or(0),
                d.get("reverted").and_then(Value::as_u64).unwrap_or(0),
                g.get("outcomes_recorded").and_then(Value::as_u64).unwrap_or(0)
            ),
            &topics,
            d.get("path").and_then(Value::as_str).unwrap_or(""),
        ));
    }

    if d.get("accepted").and_then(Value::as_u64).unwrap_or(0) >= tagging_gap_min
        && g.get("tags_found").and_then(Value::as_u64).unwrap_or(0) == 0
    {
        candidates.push(item_for(
            "tagging_gap",
            &date_str,
            &format!(
                "Increase automation reliability: enforce proposal traceability (accepted={}, git_tags={})",
                d.get("accepted").and_then(Value::as_u64).unwrap_or(0),
                g.get("tags_found").and_then(Value::as_u64).unwrap_or(0)
            ),
            &format!(
                "No proposal:<ID> commit tags were detected for accepted={}. Enforce deterministic proposal tagging to improve shipped outcome attribution.",
                d.get("accepted").and_then(Value::as_u64).unwrap_or(0)
            ),
            &topics,
            g.get("path").and_then(Value::as_str).unwrap_or(""),
        ));
    }

    let mut dedup = Vec::<Value>::new();
    let mut seen_urls = HashSet::<String>::new();
    for item in candidates {
        let url = item
            .as_object()
            .and_then(|o| o.get("url"))
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        if url.is_empty() || !seen_urls.insert(url) {
            continue;
        }
        dedup.push(item);
    }

    let items = dedup.into_iter().take(max_items).collect::<Vec<_>>();
    let bytes = items
        .iter()
        .map(|row| {
            row.as_object()
                .and_then(|o| o.get("bytes"))
                .and_then(Value::as_u64)
                .unwrap_or(0)
        })
        .sum::<u64>();
    let duration_ms = (Utc::now().timestamp_millis() - started).max(0) as u64;

    json!({
        "success": true,
        "items": items,
        "duration_ms": duration_ms,
        "requests": 0,
        "bytes": bytes
    })
}

fn dispatch(root: &Path, command: &str, payload: &Map<String, Value>) -> Result<Value, String> {
    let state_dir = resolve_state_dir(root, payload);
    match command {
        "preflight" => Ok(preflight(payload, &state_dir)),
        "collect" => Ok(collect(payload, &state_dir)),
        _ => Err("local_state_digest_kernel_unknown_command".to_string()),
    }
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    if argv.is_empty() || matches!(argv[0].as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }

    let command = argv[0].trim().to_ascii_lowercase();
    let payload = match lane_utils::payload_json(&argv[1..], "local_state_digest_kernel") {
        Ok(v) => v,
        Err(err) => {
            lane_utils::print_json_line(&lane_utils::cli_error(
                "local_state_digest_kernel_error",
                &err,
            ));
            return 1;
        }
    };
    let payload_obj = payload_obj(&payload);
    match dispatch(root, &command, payload_obj) {
        Ok(out) => {
            lane_utils::print_json_line(&lane_utils::cli_receipt("local_state_digest_kernel", out));
            0
        }
        Err(err) => {
            lane_utils::print_json_line(&lane_utils::cli_error(
                "local_state_digest_kernel_error",
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

    #[test]
    fn preflight_invalid_budget_fails() {
        let tmp = tempdir().expect("tmpdir");
        let payload = json!({
            "state_dir": tmp.path().display().to_string(),
            "budgets": { "max_items": 0 }
        });
        let out = preflight(payload_obj(&payload), tmp.path());
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
    }

    #[test]
    fn collect_emits_backlog_signal() {
        let tmp = tempdir().expect("tmpdir");
        let state_dir = tmp.path().join("state");
        let date = "2026-03-27";
        let proposals_dir = state_dir.join("sensory").join("proposals");
        fs::create_dir_all(&proposals_dir).expect("mkdir");
        let proposals_path = proposals_dir.join(format!("{date}.json"));
        fs::write(
            &proposals_path,
            serde_json::to_string(&json!({
                "proposals": [
                    {"status":"open"},
                    {"status":"open"},
                    {"status":"open"},
                    {"status":"open"},
                    {"status":"open"},
                    {"status":"open"},
                    {"status":"open"}
                ]
            }))
            .expect("encode"),
        )
        .expect("write");

        let payload = json!({
            "state_dir": state_dir.display().to_string(),
            "date": date,
            "budgets": { "max_items": 4 },
            "backlog_threshold": 6,
            "outcome_gap_accepted_min": 99,
            "tagging_gap_accepted_min": 99
        });
        let out = collect(payload_obj(&payload), &state_dir);
        assert_eq!(out.get("success").and_then(Value::as_bool), Some(true));
        assert_eq!(
            out.get("items")
                .and_then(Value::as_array)
                .map(|rows| !rows.is_empty()),
            Some(true)
        );
    }
}
