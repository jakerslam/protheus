// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use hmac::{Hmac, Mac};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeSet, HashSet};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::contract_lane_utils as lane_utils;
use crate::{deterministic_receipt_hash, now_iso};

type HmacSha256 = Hmac<Sha256>;

fn usage() {
    println!("action-receipts-kernel commands:");
    println!("  infring-ops action-receipts-kernel now-iso");
    println!("  infring-ops action-receipts-kernel append-jsonl --payload-base64=<json>");
    println!("  infring-ops action-receipts-kernel with-receipt-contract --payload-base64=<json>");
    println!("  infring-ops action-receipts-kernel write-contract-receipt --payload-base64=<json>");
    println!(
        "  infring-ops action-receipts-kernel replay-task-lineage --task-id=<id> [--trace-id=<id>] [--limit=<n>] [--scan-root=<path>] [--sources=<csv_paths>]"
    );
    println!(
        "  infring-ops action-receipts-kernel query-task-lineage --task-id=<id> [--trace-id=<id>] [--limit=<n>] [--scan-root=<path>] [--sources=<csv_paths>]"
    );
}

fn with_receipt_hash(mut value: Value) -> Value {
    value["receipt_hash"] = Value::String(deterministic_receipt_hash(&value));
    value
}

fn cli_receipt(kind: &str, payload: Value) -> Value {
    crate::contract_lane_utils::cli_receipt(kind, payload)
}

fn cli_error(kind: &str, error: &str) -> Value {
    crate::contract_lane_utils::cli_error(kind, error)
}

fn print_json_line(value: &Value) {
    crate::contract_lane_utils::print_json_line(value);
}

fn payload_json(argv: &[String]) -> Result<Value, String> {
    if let Some(raw) = lane_utils::parse_flag(argv, "payload", false) {
        return serde_json::from_str::<Value>(&raw)
            .map_err(|err| format!("action_receipts_kernel_payload_decode_failed:{err}"));
    }
    if let Some(raw_b64) = lane_utils::parse_flag(argv, "payload-base64", false) {
        let bytes = BASE64_STANDARD
            .decode(raw_b64.as_bytes())
            .map_err(|err| format!("action_receipts_kernel_payload_base64_decode_failed:{err}"))?;
        let text = String::from_utf8(bytes)
            .map_err(|err| format!("action_receipts_kernel_payload_utf8_decode_failed:{err}"))?;
        return serde_json::from_str::<Value>(&text)
            .map_err(|err| format!("action_receipts_kernel_payload_decode_failed:{err}"));
    }
    Ok(json!({}))
}

fn payload_obj<'a>(value: &'a Value) -> &'a Map<String, Value> {
    value.as_object().unwrap_or_else(|| {
        static EMPTY: std::sync::OnceLock<Map<String, Value>> = std::sync::OnceLock::new();
        EMPTY.get_or_init(Map::new)
    })
}

fn as_object<'a>(value: Option<&'a Value>) -> Option<&'a Map<String, Value>> {
    value.and_then(Value::as_object)
}

fn as_str(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(v)) => v.trim().to_string(),
        Some(Value::Null) | None => String::new(),
        Some(v) => v.to_string().trim_matches('"').trim().to_string(),
    }
}

fn resolve_file_path(root: &Path, raw: &str) -> PathBuf {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return root.join("local").join("state").join("receipts.jsonl");
    }
    let candidate = PathBuf::from(trimmed);
    if candidate.is_absolute() {
        candidate
    } else {
        root.join(candidate)
    }
}

fn ensure_parent(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("action_receipts_kernel_create_dir_failed:{err}"))?;
    }
    Ok(())
}

fn append_jsonl(path: &Path, row: &Value) -> Result<(), String> {
    ensure_parent(path)?;
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|err| format!("action_receipts_kernel_append_open_failed:{err}"))?;
    file.write_all(
        format!(
            "{}\n",
            serde_json::to_string(row).unwrap_or_else(|_| "null".to_string())
        )
        .as_bytes(),
    )
    .map_err(|err| format!("action_receipts_kernel_append_failed:{err}"))
}

fn chain_state_path(file_path: &Path) -> PathBuf {
    PathBuf::from(format!("{}.chain.json", file_path.to_string_lossy()))
}

fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    let digest = hasher.finalize();
    let mut out = String::with_capacity(digest.len() * 2);
    for byte in digest {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

fn optional_hmac(hash: &str) -> Result<Option<String>, String> {
    let key = std::env::var("RECEIPT_CHAIN_HMAC_KEY").unwrap_or_default();
    let key = key.trim();
    if key.is_empty() {
        return Ok(None);
    }
    let mut mac = HmacSha256::new_from_slice(key.as_bytes())
        .map_err(|err| format!("action_receipts_kernel_hmac_init_failed:{err}"))?;
    mac.update(hash.as_bytes());
    Ok(Some(hex::encode(mac.finalize().into_bytes())))
}

fn read_chain_state(file_path: &Path) -> (u64, Option<String>) {
    let state_path = chain_state_path(file_path);
    let parsed = fs::read_to_string(state_path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .unwrap_or_else(|| json!({}));
    let seq = parsed.get("seq").and_then(Value::as_u64).unwrap_or(0);
    let hash = parsed
        .get("hash")
        .and_then(Value::as_str)
        .map(|row| row.to_string());
    (seq, hash)
}

fn write_chain_state(file_path: &Path, seq: u64, hash: Option<&str>) -> Result<(), String> {
    let state_path = chain_state_path(file_path);
    ensure_parent(&state_path)?;
    let tmp_path = PathBuf::from(format!(
        "{}.tmp-{}",
        state_path.to_string_lossy(),
        std::process::id()
    ));
    fs::write(
        &tmp_path,
        format!(
            "{}\n",
            serde_json::to_string_pretty(&json!({
                "seq": seq,
                "hash": hash,
                "ts": now_iso(),
            }))
            .map_err(|err| format!("action_receipts_kernel_encode_failed:{err}"))?
        ),
    )
    .map_err(|err| format!("action_receipts_kernel_write_failed:{err}"))?;
    fs::rename(&tmp_path, &state_path)
        .map_err(|err| format!("action_receipts_kernel_rename_failed:{err}"))
}

fn with_receipt_contract_value(record: &Value, attempted: bool, verified: bool) -> Value {
    let src = as_object(Some(record)).cloned().unwrap_or_default();
    let mut receipt_contract = as_object(src.get("receipt_contract"))
        .cloned()
        .unwrap_or_default();
    receipt_contract.insert("version".to_string(), Value::String("1.0".to_string()));
    receipt_contract.insert("attempted".to_string(), Value::Bool(attempted));
    receipt_contract.insert("verified".to_string(), Value::Bool(verified));
    receipt_contract.insert("recorded".to_string(), Value::Bool(true));
    let mut out = src;
    out.insert(
        "receipt_contract".to_string(),
        Value::Object(receipt_contract),
    );
    Value::Object(out)
}

fn with_receipt_integrity_value(file_path: &Path, record: &Value) -> Result<Value, String> {
    let src = as_object(Some(record)).cloned().unwrap_or_default();
    let (prev_seq, prev_hash) = read_chain_state(file_path);
    let seq = prev_seq.saturating_add(1);
    let payload_hash = sha256_hex(
        &serde_json::to_string(&Value::Object(src.clone())).unwrap_or_else(|_| "{}".to_string()),
    );
    let link_hash = sha256_hex(&format!(
        "{seq}:{}:{payload_hash}",
        prev_hash.clone().unwrap_or_default()
    ));
    let hmac = optional_hmac(&link_hash)?;

    let mut receipt_contract = as_object(src.get("receipt_contract"))
        .cloned()
        .unwrap_or_default();
    receipt_contract.insert(
        "integrity".to_string(),
        json!({
            "version": "1.0",
            "seq": seq,
            "prev_hash": prev_hash,
            "payload_hash": payload_hash,
            "hash": link_hash,
            "hmac": hmac,
            "ts": now_iso(),
        }),
    );
    let mut out = src;
    out.insert(
        "receipt_contract".to_string(),
        Value::Object(receipt_contract),
    );
    let out_value = Value::Object(out);
    let current_hash = out_value
        .get("receipt_contract")
        .and_then(Value::as_object)
        .and_then(|row| row.get("integrity"))
        .and_then(Value::as_object)
        .and_then(|row| row.get("hash"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    write_chain_state(file_path, seq, Some(&current_hash))?;
    Ok(out_value)
}

fn parse_attempted(payload: &Map<String, Value>) -> bool {
    payload
        .get("attempted")
        .and_then(Value::as_bool)
        .unwrap_or(true)
}

fn parse_verified(payload: &Map<String, Value>) -> bool {
    payload
        .get("verified")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn write_contract_receipt_value(
    root: &Path,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let file_path = resolve_file_path(root, &as_str(payload.get("file_path")));
    let record = payload.get("record").cloned().unwrap_or_else(|| json!({}));
    let with_contract =
        with_receipt_contract_value(&record, parse_attempted(payload), parse_verified(payload));
    let with_integrity = with_receipt_integrity_value(&file_path, &with_contract)?;
    append_jsonl(&file_path, &with_integrity)?;
    Ok(json!({
        "ok": true,
        "file_path": file_path.to_string_lossy(),
        "record": with_integrity,
    }))
}

fn parse_lineage_limit(payload: &Map<String, Value>) -> usize {
    payload
        .get("limit")
        .and_then(Value::as_u64)
        .map(|v| v as usize)
        .filter(|v| *v > 0)
        .unwrap_or(4000)
        .min(50_000)
}

fn parse_scan_root(root: &Path, payload: &Map<String, Value>) -> PathBuf {
    let raw = as_str(payload.get("scan_root"));
    if raw.is_empty() {
        return root.to_path_buf();
    }
    let candidate = PathBuf::from(raw);
    if candidate.is_absolute() {
        candidate
    } else {
        root.join(candidate)
    }
}

fn source_paths_from_payload(root: &Path, payload: &Map<String, Value>) -> Vec<PathBuf> {
    let explicit = as_str(payload.get("sources"));
    if explicit.trim().is_empty() {
        return Vec::new();
    }
    explicit
        .split(',')
        .map(|row| resolve_file_path(root, row))
        .filter(|path| path.exists())
        .collect::<Vec<_>>()
}

fn known_lineage_paths(scan_root: &Path) -> Vec<PathBuf> {
    let mut out = vec![
        scan_root
            .join("local")
            .join("state")
            .join("runtime")
            .join("task_runtime")
            .join("verity_receipts.jsonl"),
        scan_root
            .join("local")
            .join("state")
            .join("runtime")
            .join("task_runtime")
            .join("conduit_messages.jsonl"),
        scan_root
            .join("client")
            .join("runtime")
            .join("local")
            .join("state")
            .join("ui")
            .join("infring_dashboard")
            .join("actions")
            .join("history.jsonl"),
        scan_root
            .join("local")
            .join("state")
            .join("attention")
            .join("receipts.jsonl"),
        scan_root
            .join("local")
            .join("state")
            .join("stomach")
            .join("receipts.jsonl"),
        scan_root
            .join("local")
            .join("state")
            .join("ops")
            .join("verity")
            .join("receipts.jsonl"),
    ];
    out.retain(|path| path.exists());
    out
}

fn is_replay_candidate_name(name: &str) -> bool {
    matches!(
        name,
        "history.jsonl"
            | "receipts.jsonl"
            | "verity_receipts.jsonl"
            | "conduit_messages.jsonl"
            | "protocol_step_receipts.jsonl"
            | "protocol_history.jsonl"
    )
}

fn should_skip_replay_path(path: &Path) -> bool {
    let lowered = path.to_string_lossy().to_ascii_lowercase();
    lowered.contains("/assimilation/isolated/")
        || lowered.contains("/assimilation/burned/")
        || lowered.contains("/node_modules/")
        || lowered.contains("/.git/")
        || lowered.contains("/target/")
}

fn discover_lineage_paths(scan_root: &Path) -> Vec<PathBuf> {
    let roots = [
        scan_root.join("local").join("state"),
        scan_root.join("core").join("local").join("state"),
        scan_root
            .join("client")
            .join("runtime")
            .join("local")
            .join("state"),
    ];
    let mut out = BTreeSet::<PathBuf>::new();
    for root in roots {
        if !root.exists() {
            continue;
        }
        for entry in WalkDir::new(&root)
            .follow_links(false)
            .into_iter()
            .filter_map(Result::ok)
        {
            let path = entry.path();
            if !path.is_file() || should_skip_replay_path(path) {
                continue;
            }
            let Some(name) = path.file_name().and_then(|v| v.to_str()) else {
                continue;
            };
            if is_replay_candidate_name(name) {
                out.insert(path.to_path_buf());
            }
        }
    }
    out.into_iter().collect::<Vec<_>>()
}

fn read_jsonl_rows(path: &Path, limit: usize) -> Vec<(usize, Value)> {
    let raw = fs::read_to_string(path).unwrap_or_default();
    let rows = raw
        .lines()
        .enumerate()
        .filter_map(|(idx, line)| {
            serde_json::from_str::<Value>(line.trim())
                .ok()
                .map(|v| (idx, v))
        })
        .collect::<Vec<_>>();
    if rows.len() <= limit {
        return rows;
    }
    rows[rows.len().saturating_sub(limit)..].to_vec()
}

fn collect_field_strings(value: &Value, field: &str, out: &mut Vec<String>, cap: usize) {
    if out.len() >= cap {
        return;
    }
    match value {
        Value::Object(map) => {
            for (k, v) in map {
                if out.len() >= cap {
                    break;
                }
                if k == field {
                    let parsed = as_str(Some(v));
                    if !parsed.is_empty() {
                        out.push(parsed);
                    }
                }
                collect_field_strings(v, field, out, cap);
            }
        }
        Value::Array(rows) => {
            for row in rows {
                if out.len() >= cap {
                    break;
                }
                collect_field_strings(row, field, out, cap);
            }
        }
        _ => {}
    }
}

fn row_matches_task_or_trace(row: &Value, task_id: &str, trace_id: Option<&str>) -> bool {
    let mut task_ids = Vec::<String>::new();
    collect_field_strings(row, "task_id", &mut task_ids, 32);
    let task_match = task_ids.iter().any(|v| v == task_id);
    if task_match {
        return true;
    }
    let trace_id = trace_id.unwrap_or("");
    if trace_id.is_empty() {
        return false;
    }
    let mut trace_ids = Vec::<String>::new();
    collect_field_strings(row, "trace_id", &mut trace_ids, 32);
    trace_ids.iter().any(|v| v == trace_id)
}

fn collect_tool_pipeline_objects(value: &Value, out: &mut Vec<Value>, cap: usize) {
    if out.len() >= cap {
        return;
    }
    match value {
        Value::Object(map) => {
            if map.contains_key("normalized_result")
                && (map.contains_key("evidence_cards")
                    || map.contains_key("claim_bundle")
                    || map.contains_key("worker_output"))
            {
                out.push(value.clone());
            }
            for child in map.values() {
                if out.len() >= cap {
                    break;
                }
                collect_tool_pipeline_objects(child, out, cap);
            }
        }
        Value::Array(rows) => {
            for row in rows {
                if out.len() >= cap {
                    break;
                }
                collect_tool_pipeline_objects(row, out, cap);
            }
        }
        _ => {}
    }
}

fn lower_compact_type(row: &Value) -> String {
    let typ = as_str(row.get("type")).to_ascii_lowercase();
    let event = as_str(row.get("event_type")).to_ascii_lowercase();
    let payload_type = row
        .get("payload")
        .and_then(Value::as_object)
        .and_then(|payload| payload.get("type"))
        .map(|value| as_str(Some(value)))
        .unwrap_or_default()
        .to_ascii_lowercase();
    format!("{typ}|{event}|{payload_type}")
}

fn replay_task_lineage_value(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let task_id = as_str(payload.get("task_id"));
    if task_id.is_empty() {
        return Err("task_id_required".to_string());
    }
    let trace_id = {
        let value = as_str(payload.get("trace_id"));
        if value.is_empty() {
            None
        } else {
            Some(value)
        }
    };
    let limit = parse_lineage_limit(payload);
    let scan_root = parse_scan_root(root, payload);

    let mut source_paths = known_lineage_paths(&scan_root);
    source_paths.extend(discover_lineage_paths(&scan_root));
    source_paths.extend(source_paths_from_payload(&scan_root, payload));
    let mut dedupe = BTreeSet::<PathBuf>::new();
    source_paths.retain(|path| dedupe.insert(path.clone()));

    let mut task_events = Vec::<Value>::new();
    let mut tool_calls = Vec::<Value>::new();
    let mut evidence_cards = Vec::<Value>::new();
    let mut claims = Vec::<Value>::new();
    let mut memory_mutations = Vec::<Value>::new();
    let mut assimilation_steps = Vec::<Value>::new();
    let mut scanned_files = 0usize;
    let mut scanned_rows = 0usize;
    let mut seen_result_ids = HashSet::<String>::new();
    let mut seen_evidence_ids = HashSet::<String>::new();
    let mut seen_claim_ids = HashSet::<String>::new();
    let mut seen_memory_receipts = HashSet::<String>::new();
    let mut seen_assimilation_receipts = HashSet::<String>::new();

    for path in source_paths {
        let rows = read_jsonl_rows(&path, limit);
        if rows.is_empty() {
            continue;
        }
        scanned_files = scanned_files.saturating_add(1);
        scanned_rows = scanned_rows.saturating_add(rows.len());
        let is_protocol_steps = path
            .file_name()
            .and_then(|v| v.to_str())
            .map(|name| name.eq_ignore_ascii_case("protocol_step_receipts.jsonl"))
            .unwrap_or(false);
        for (idx, row) in rows {
            if !row_matches_task_or_trace(&row, &task_id, trace_id.as_deref()) {
                continue;
            }
            let type_compact = lower_compact_type(&row);
            if type_compact.contains("task_")
                || row
                    .pointer("/payload/task_id")
                    .and_then(Value::as_str)
                    .is_some()
            {
                task_events.push(json!({
                    "source_file": path.to_string_lossy(),
                    "line_index": idx,
                    "receipt_hash": row.get("receipt_hash").cloned().unwrap_or(Value::Null),
                    "type": row.get("type").cloned().unwrap_or(Value::Null),
                    "event_type": row.get("event_type").cloned().unwrap_or(Value::Null),
                    "payload": row.get("payload").cloned().unwrap_or(Value::Null)
                }));
            }

            let mut pipelines = Vec::<Value>::new();
            collect_tool_pipeline_objects(&row, &mut pipelines, 16);
            for pipeline in pipelines {
                let normalized = pipeline
                    .get("normalized_result")
                    .cloned()
                    .unwrap_or_else(|| json!({}));
                let result_id = as_str(normalized.get("result_id"));
                if !result_id.is_empty() && !seen_result_ids.insert(result_id.clone()) {
                    continue;
                }
                if !result_id.is_empty() || !normalized.is_null() {
                    tool_calls.push(normalized.clone());
                }
                let evidence = pipeline
                    .get("evidence_cards")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                for card in evidence {
                    let evidence_id = as_str(card.get("evidence_id"));
                    if evidence_id.is_empty() || seen_evidence_ids.insert(evidence_id) {
                        evidence_cards.push(card);
                    }
                }
                let claim_rows = pipeline
                    .get("claim_bundle")
                    .and_then(Value::as_object)
                    .and_then(|bundle| bundle.get("claims"))
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                for claim in claim_rows {
                    let claim_id = as_str(claim.get("claim_id"));
                    if claim_id.is_empty() || seen_claim_ids.insert(claim_id) {
                        claims.push(claim);
                    }
                }
            }

            if type_compact.contains("memory_") || type_compact.contains("|memory") {
                let receipt_hash = as_str(row.get("receipt_hash"));
                if receipt_hash.is_empty() || seen_memory_receipts.insert(receipt_hash) {
                    memory_mutations.push(json!({
                        "source_file": path.to_string_lossy(),
                        "line_index": idx,
                        "receipt_hash": row.get("receipt_hash").cloned().unwrap_or(Value::Null),
                        "type": row.get("type").cloned().unwrap_or(Value::Null),
                        "event_type": row.get("event_type").cloned().unwrap_or(Value::Null),
                        "payload": row.get("payload").cloned().unwrap_or(Value::Null),
                    }));
                }
            }

            if is_protocol_steps || type_compact.contains("assimilation") {
                let receipt_hash = as_str(row.get("receipt_hash"));
                if receipt_hash.is_empty() || seen_assimilation_receipts.insert(receipt_hash) {
                    assimilation_steps.push(json!({
                        "source_file": path.to_string_lossy(),
                        "line_index": idx,
                        "receipt_hash": row.get("receipt_hash").cloned().unwrap_or(Value::Null),
                        "step_id": row.get("step_id").cloned().unwrap_or(Value::Null),
                        "type": row.get("type").cloned().unwrap_or(Value::Null),
                        "event_type": row.get("event_type").cloned().unwrap_or(Value::Null),
                        "payload": row.get("payload").cloned().unwrap_or(Value::Null),
                    }));
                }
            }
        }
    }

    let evidence_ids = evidence_cards
        .iter()
        .map(|row| as_str(row.get("evidence_id")))
        .filter(|row| !row.is_empty())
        .collect::<HashSet<_>>();
    let mut claims_without_evidence = Vec::<Value>::new();
    for claim in &claims {
        let claim_id = as_str(claim.get("claim_id"));
        let evidence_refs = claim
            .get("evidence_ids")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .map(|v| as_str(Some(&v)))
            .filter(|v| !v.is_empty())
            .collect::<Vec<_>>();
        if evidence_refs.is_empty() || evidence_refs.iter().any(|id| !evidence_ids.contains(id)) {
            claims_without_evidence.push(json!({
                "claim_id": claim_id,
                "evidence_ids": evidence_refs
            }));
        }
    }

    Ok(json!({
        "ok": true,
        "task_id": task_id,
        "trace_id": trace_id,
        "lineage": {
            "task": task_events,
            "tool_call": tool_calls,
            "evidence": evidence_cards,
            "claim": claims,
            "memory_mutation": memory_mutations,
            "assimilation_step": assimilation_steps
        },
        "validation": {
            "claims_without_evidence": claims_without_evidence,
            "claim_evidence_integrity_ok": claims_without_evidence.is_empty()
        },
        "stats": {
            "scanned_files": scanned_files,
            "scanned_rows": scanned_rows
        }
    }))
}

pub fn query_task_lineage(
    root: &Path,
    task_id: &str,
    trace_id: Option<&str>,
    limit: usize,
    scan_root: Option<&Path>,
) -> Result<Value, String> {
    let scan_root_value = scan_root
        .map(|path| path.to_string_lossy().to_string())
        .unwrap_or_default();
    let payload = json!({
        "task_id": task_id,
        "trace_id": trace_id.unwrap_or_default(),
        "limit": limit,
        "scan_root": scan_root_value
    });
    let obj = payload
        .as_object()
        .cloned()
        .unwrap_or_else(Map::<String, Value>::new);
    replay_task_lineage_value(root, &obj)
}

fn run_command(root: &Path, command: &str, payload: &Map<String, Value>) -> Result<Value, String> {
    match command {
        "now-iso" => Ok(json!({ "ok": true, "ts": now_iso() })),
        "append-jsonl" => {
            let file_path = resolve_file_path(root, &as_str(payload.get("file_path")));
            let row = payload.get("row").cloned().unwrap_or(Value::Null);
            append_jsonl(&file_path, &row)?;
            Ok(json!({
                "ok": true,
                "file_path": file_path.to_string_lossy(),
                "appended": true,
            }))
        }
        "with-receipt-contract" => Ok(json!({
            "ok": true,
            "record": with_receipt_contract_value(
                &payload.get("record").cloned().unwrap_or_else(|| json!({})),
                parse_attempted(payload),
                parse_verified(payload),
            ),
        })),
        "write-contract-receipt" => write_contract_receipt_value(root, payload),
        "replay-task-lineage" | "query-task-lineage" => replay_task_lineage_value(root, payload),
        _ => Err("action_receipts_kernel_unknown_command".to_string()),
    }
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let Some(command) = argv.first().map(|row| row.as_str()) else {
        usage();
        return 1;
    };
    if matches!(command, "help" | "--help" | "-h") {
        usage();
        return 0;
    }
    let mut payload = match payload_json(argv) {
        Ok(value) => value,
        Err(err) => {
            print_json_line(&cli_error("action_receipts_kernel", &err));
            return 1;
        }
    };
    if matches!(command, "replay-task-lineage" | "query-task-lineage") {
        let mut merged = payload.as_object().cloned().unwrap_or_default();
        if let Some(task_id) = lane_utils::parse_flag(argv, "task-id", false) {
            if !task_id.trim().is_empty() {
                merged.insert(
                    "task_id".to_string(),
                    Value::String(task_id.trim().to_string()),
                );
            }
        }
        if let Some(trace_id) = lane_utils::parse_flag(argv, "trace-id", false) {
            if !trace_id.trim().is_empty() {
                merged.insert(
                    "trace_id".to_string(),
                    Value::String(trace_id.trim().to_string()),
                );
            }
        }
        if let Some(limit) =
            lane_utils::parse_flag(argv, "limit", false).and_then(|value| value.parse::<u64>().ok())
        {
            merged.insert("limit".to_string(), Value::from(limit));
        }
        if let Some(scan_root) = lane_utils::parse_flag(argv, "scan-root", false) {
            if !scan_root.trim().is_empty() {
                merged.insert(
                    "scan_root".to_string(),
                    Value::String(scan_root.trim().to_string()),
                );
            }
        }
        if let Some(sources) = lane_utils::parse_flag(argv, "sources", false) {
            if !sources.trim().is_empty() {
                merged.insert(
                    "sources".to_string(),
                    Value::String(sources.trim().to_string()),
                );
            }
        }
        payload = Value::Object(merged);
    }
    match run_command(root, command, payload_obj(&payload)) {
        Ok(out) => {
            print_json_line(&cli_receipt("action_receipts_kernel", out));
            0
        }
        Err(err) => {
            print_json_line(&cli_error("action_receipts_kernel", &err));
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn write_contract_receipt_increments_chain() {
        let tmp = tempdir().expect("tempdir");
        let file_path = tmp.path().join("receipts.jsonl");
        let payload = json!({
            "file_path": file_path,
            "record": { "type": "unit" },
            "attempted": true,
            "verified": false
        });
        let first = write_contract_receipt_value(tmp.path(), payload_obj(&payload)).expect("first");
        let first_seq = first
            .get("record")
            .and_then(Value::as_object)
            .and_then(|row| row.get("receipt_contract"))
            .and_then(Value::as_object)
            .and_then(|row| row.get("integrity"))
            .and_then(Value::as_object)
            .and_then(|row| row.get("seq"))
            .and_then(Value::as_u64);
        assert_eq!(first_seq, Some(1));

        let second =
            write_contract_receipt_value(tmp.path(), payload_obj(&payload)).expect("second");
        let second_seq = second
            .get("record")
            .and_then(Value::as_object)
            .and_then(|row| row.get("receipt_contract"))
            .and_then(Value::as_object)
            .and_then(|row| row.get("integrity"))
            .and_then(Value::as_object)
            .and_then(|row| row.get("seq"))
            .and_then(Value::as_u64);
        assert_eq!(second_seq, Some(2));
        assert!(chain_state_path(&file_path).exists());
    }

    #[test]
    fn replay_task_lineage_reconstructs_end_to_end_chain() {
        let tmp = tempdir().expect("tempdir");
        let task_id = "task-123";
        let trace_id = "trace-abc";

        let task_receipts = tmp
            .path()
            .join("local/state/runtime/task_runtime/verity_receipts.jsonl");
        append_jsonl(
            &task_receipts,
            &json!({
                "type": "task_verity_receipt",
                "event_type": "task_result",
                "receipt_hash": "r-task",
                "payload": {"task_id": task_id, "status":"done"}
            }),
        )
        .expect("append task receipt");

        let actions_history = tmp
            .path()
            .join("client/runtime/local/state/ui/infring_dashboard/actions/history.jsonl");
        append_jsonl(
            &actions_history,
            &json!({
                "type": "dashboard_tool_result",
                "receipt_hash": "r-tool",
                "payload": {
                    "tool_pipeline": {
                        "normalized_result": {
                            "result_id": "res-1",
                            "task_id": task_id,
                            "trace_id": trace_id,
                            "tool_name": "web_search"
                        },
                        "evidence_cards": [{
                            "evidence_id":"ev-1",
                            "task_id": task_id,
                            "trace_id": trace_id,
                            "summary":"snippet"
                        }],
                        "claim_bundle": {
                            "task_id": task_id,
                            "claims": [{
                                "claim_id":"claim-1",
                                "text":"found",
                                "evidence_ids":["ev-1"],
                                "status":"supported"
                            }]
                        }
                    }
                }
            }),
        )
        .expect("append action history");

        let memory_history = tmp.path().join("local/state/ops/memory/history.jsonl");
        append_jsonl(
            &memory_history,
            &json!({
                "type":"memory_write",
                "task_id": task_id,
                "receipt_hash":"r-mem",
                "payload":{"object_id":"o-1","version_id":"v-1"}
            }),
        )
        .expect("append memory history");

        let assimilation_steps = tmp
            .path()
            .join("local/state/ops/runtime_systems/assimilate/protocol_step_receipts.jsonl");
        append_jsonl(
            &assimilation_steps,
            &json!({
                "type":"assimilation_protocol_step",
                "task_id": task_id,
                "step_id":"step-1",
                "receipt_hash":"r-assim"
            }),
        )
        .expect("append assimilation steps");

        let out = run_command(
            tmp.path(),
            "replay-task-lineage",
            payload_obj(&json!({"task_id": task_id, "trace_id": trace_id})),
        )
        .expect("replay lineage");
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            out.pointer("/lineage/tool_call")
                .and_then(Value::as_array)
                .map(|rows| rows.len()),
            Some(1)
        );
        assert_eq!(
            out.pointer("/lineage/evidence")
                .and_then(Value::as_array)
                .map(|rows| rows.len()),
            Some(1)
        );
        assert_eq!(
            out.pointer("/lineage/claim")
                .and_then(Value::as_array)
                .map(|rows| rows.len()),
            Some(1)
        );
        assert_eq!(
            out.pointer("/lineage/memory_mutation")
                .and_then(Value::as_array)
                .map(|rows| rows.len()),
            Some(1)
        );
        assert_eq!(
            out.pointer("/lineage/assimilation_step")
                .and_then(Value::as_array)
                .map(|rows| rows.len()),
            Some(1)
        );
        assert_eq!(
            out.pointer("/validation/claim_evidence_integrity_ok")
                .and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn replay_task_lineage_requires_task_id() {
        let tmp = tempdir().expect("tempdir");
        let err = run_command(tmp.path(), "replay-task-lineage", payload_obj(&json!({})))
            .expect_err("expected missing task id error");
        assert_eq!(err, "task_id_required");
    }
}

