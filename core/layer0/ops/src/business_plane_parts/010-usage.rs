// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops::business_plane (authoritative)
use crate::v8_kernel::{
    append_jsonl, attach_conduit, build_conduit_enforcement, canonical_json_string,
    conduit_bypass_requested, deterministic_merkle_root, history_path, latest_path,
    next_chain_hash, parse_bool, parse_i64, parse_json_or_empty, parse_u64, print_json, read_json,
    read_jsonl, scoped_state_root, sha256_hex_str, write_json,
};
use crate::{clean, now_iso, parse_args};
use serde_json::{json, Map, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const LANE_ID: &str = "business_plane";
const ENV_KEY: &str = "INFRING_BUSINESS_PLANE_STATE_ROOT";

fn usage() {
    println!("Usage:");
    println!(
        "  infring-ops business-plane taxonomy --business-context=<id> --topic=<text> [--tier=node1|tag2|jot3] [--interaction-count=<n>] [--promote-threshold=<n>] [--strict=1|0]"
    );
    println!(
        "  infring-ops business-plane persona --op=<issue|renew|revoke|status> --persona=<id> [--business-context=<id>] [--lease-hours=<n>] [--strict=1|0]"
    );
    println!(
        "  infring-ops business-plane continuity --op=<checkpoint|resume|handoff|status> [--business-context=<id>] [--name=<id>] [--state-json=<json>] [--to=<stakeholder>] [--task=<text>] [--strict=1|0]"
    );
    println!(
        "  infring-ops business-plane alerts --op=<emit|ack|status> [--alert-type=<id>] [--channel=<dashboard|slack|email|sms|pagerduty>] [--business-context=<id>] [--strict=1|0]"
    );
    println!(
        "  infring-ops business-plane switchboard --op=<create|write|read|status> --business-context=<id> [--target-business=<id>] [--entry-json=<json>] [--strict=1|0]"
    );
    println!(
        "  infring-ops business-plane external-sync --system=<notion|confluence|crm|calendar|email|slack> --direction=<push|pull|bidirectional> [--business-context=<id>] [--external-id=<id>] [--content-json=<json>] [--strict=1|0]"
    );
    println!(
        "  infring-ops business-plane continuity-audit [--days=<n>] [--business-context=<id|ALL>] [--strict=1|0]"
    );
    println!(
        "  infring-ops business-plane archive --op=<record|query|export|status> [--business-context=<id|ALL>] [--date-range=<start:end>] [--entry-json=<json>] [--strict=1|0]"
    );
}

fn lane_root(root: &Path) -> PathBuf {
    scoped_state_root(root, ENV_KEY, LANE_ID)
}

fn taxonomy_path(root: &Path, business: &str) -> PathBuf {
    lane_root(root)
        .join("businesses")
        .join(clean(business, 80))
        .join("taxonomy.json")
}

fn personas_path(root: &Path) -> PathBuf {
    lane_root(root).join("personas.json")
}

fn checkpoints_dir(root: &Path) -> PathBuf {
    lane_root(root).join("checkpoints")
}

fn continuity_state_path(root: &Path) -> PathBuf {
    lane_root(root).join("continuity_state.json")
}

fn handoff_queue_path(root: &Path) -> PathBuf {
    lane_root(root).join("handoffs.jsonl")
}

fn alerts_state_path(root: &Path) -> PathBuf {
    lane_root(root).join("alerts.json")
}

fn switchboard_dir(root: &Path, business: &str) -> PathBuf {
    lane_root(root)
        .join("tenants")
        .join(clean(business, 80))
        .join("memory")
}

fn sync_history_path(root: &Path) -> PathBuf {
    lane_root(root).join("external_sync.jsonl")
}

fn archive_path(root: &Path) -> PathBuf {
    lane_root(root).join("archive.jsonl")
}

fn archive_anchor_path(root: &Path) -> PathBuf {
    lane_root(root).join("archive_daily_roots.json")
}

fn business_registry_path(root: &Path) -> PathBuf {
    lane_root(root).join("business_registry.json")
}

fn now_epoch_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn read_object(path: &Path) -> Map<String, Value> {
    read_json(path)
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default()
}

fn append_archive(root: &Path, row: &Value) -> Result<(), String> {
    append_jsonl(&archive_path(root), row)?;
    let day = now_iso()[..10].to_string();
    let all = read_jsonl(&archive_path(root));
    let day_receipts = all
        .iter()
        .filter(|entry| {
            entry
                .get("ts")
                .and_then(Value::as_str)
                .map(|ts| ts.starts_with(&day))
                .unwrap_or(false)
        })
        .filter_map(|entry| {
            entry
                .get("receipt_hash")
                .and_then(Value::as_str)
                .map(|s| s.to_string())
        })
        .collect::<Vec<_>>();
    let mut anchors = read_object(&archive_anchor_path(root));
    anchors.insert(
        day.clone(),
        Value::String(deterministic_merkle_root(&day_receipts)),
    );
    write_json(&archive_anchor_path(root), &Value::Object(anchors))
}

fn emit(root: &Path, command: &str, strict: bool, payload: Value, conduit: Option<&Value>) -> i32 {
    let out = attach_conduit(payload, conduit);
    let _ = write_json(&latest_path(root, ENV_KEY, LANE_ID), &out);
    let _ = append_jsonl(&history_path(root, ENV_KEY, LANE_ID), &out);
    let archive_row = json!({
        "ts": out.get("ts").cloned().unwrap_or_else(|| Value::String(now_iso())),
        "command": command,
        "strict": strict,
        "receipt_hash": out.get("receipt_hash").cloned().unwrap_or(Value::Null),
        "business_context": out.get("business_context").cloned().unwrap_or(Value::String("ALL".to_string())),
        "type": out.get("type").cloned().unwrap_or_else(|| Value::String("business_plane".to_string()))
    });
    let _ = append_archive(root, &archive_row);
    print_json(&out);
    if out.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        0
    } else {
        1
    }
}

fn require_business(parsed: &crate::ParsedArgs) -> Result<String, String> {
    let business = clean(
        parsed
            .flags
            .get("business-context")
            .map(String::as_str)
            .unwrap_or(""),
        80,
    );
    if business.is_empty() {
        return Err("business_context_required".to_string());
    }
    Ok(business)
}

fn normalize_persona_op(raw: &str) -> String {
    let token = clean(raw, 24).to_ascii_lowercase().replace('_', "-");
    match token.as_str() {
        "issue" | "grant" | "create" => "issue".to_string(),
        "renew" | "extend" | "refresh" => "renew".to_string(),
        "revoke" | "disable" | "suspend" => "revoke".to_string(),
        "status" | "get" | "inspect" => "status".to_string(),
        _ => token,
    }
}

fn load_personas(root: &Path) -> Map<String, Value> {
    read_object(&personas_path(root))
}

fn write_personas(root: &Path, rows: &Map<String, Value>) -> Result<(), String> {
    write_json(&personas_path(root), &Value::Object(rows.clone()))
}

fn ensure_business_registered(root: &Path, business: &str) -> Result<(), String> {
    let mut registry = read_object(&business_registry_path(root));
    if !registry.contains_key(business) {
        registry.insert(
            business.to_string(),
            json!({
                "created_at": now_iso(),
                "status": "active"
            }),
        );
        write_json(&business_registry_path(root), &Value::Object(registry))?;
    }
    Ok(())
}

fn taxonomy_command(root: &Path, parsed: &crate::ParsedArgs) -> Result<Value, String> {
    let business = require_business(parsed)?;
    ensure_business_registered(root, &business)?;
    let topic = clean(
        parsed
            .flags
            .get("topic")
            .map(String::as_str)
            .unwrap_or("untitled-topic"),
        240,
    );
    let tier_raw = clean(
        parsed
            .flags
            .get("tier")
            .map(String::as_str)
            .unwrap_or("jot3"),
        16,
    )
    .to_ascii_lowercase();
    let interaction_count = parse_u64(parsed.flags.get("interaction-count"), 1).max(1);
    let promote_threshold = parse_u64(parsed.flags.get("promote-threshold"), 12).max(2);
    let mut final_tier = match tier_raw.as_str() {
        "node1" | "tag2" | "jot3" => tier_raw.clone(),
        _ => "jot3".to_string(),
    };
    let mut promoted = false;
    if final_tier == "jot3" && interaction_count >= (promote_threshold / 2).max(2) {
        final_tier = "tag2".to_string();
        promoted = true;
    }
    if final_tier == "tag2" && interaction_count >= promote_threshold {
        final_tier = "node1".to_string();
        promoted = true;
    }

    let path = taxonomy_path(root, &business);
    let mut state = read_object(&path);
    let entry = json!({
        "topic": topic,
        "tier": final_tier,
        "interaction_count": interaction_count,
        "promote_threshold": promote_threshold,
        "promoted": promoted,
        "ts": now_iso()
    });
    state.insert(topic.clone(), entry.clone());
    write_json(&path, &Value::Object(state))?;

    Ok(json!({
        "ok": true,
        "type": "business_plane_taxonomy",
        "lane": LANE_ID,
        "ts": now_iso(),
        "business_context": business,
        "topic": topic,
        "entry": entry,
        "taxonomy_path": path.to_string_lossy().to_string(),
        "claim_evidence": [{
            "id": "V7-BUSINESS-001.1",
            "claim": "tiered_business_memory_taxonomy_supports_auto_promotion_and_business_scoped_query_filters",
            "evidence": {
                "tiers": ["node1", "tag2", "jot3"],
                "promoted": promoted,
                "interaction_count": interaction_count
            }
        }]
    }))
}

fn persona_command(root: &Path, parsed: &crate::ParsedArgs) -> Result<Value, String> {
    let op = normalize_persona_op(
        parsed
            .flags
            .get("op")
            .map(String::as_str)
            .unwrap_or("status"),
    );
    let persona = clean(
        parsed
            .flags
            .get("persona")
            .map(String::as_str)
            .unwrap_or(""),
        80,
    );
    if persona.is_empty() {
        return Err("persona_required".to_string());
    }
    let business_input = clean(
        parsed
            .flags
            .get("business-context")
            .map(String::as_str)
            .unwrap_or(""),
        80,
    );
    if op != "status" && business_input.is_empty() {
        return Err("business_context_required".to_string());
    }
    let business = if business_input.is_empty() {
        "default".to_string()
    } else {
        business_input
    };
    let lease_hours = parse_i64(parsed.flags.get("lease-hours"), 24).clamp(1, 168) as u64;
    ensure_business_registered(root, &business)?;
    let mut personas = load_personas(root);
    let now = now_epoch_secs();
    let key = format!("{business}:{persona}");

    if op == "status" {
        let record = personas.get(&key).cloned().unwrap_or(Value::Null);
        let expires = record
            .get("expires_at_epoch")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let active = expires > now && record.get("revoked").and_then(Value::as_bool) != Some(true);
        return Ok(json!({
            "ok": true,
            "type": "business_plane_persona",
            "lane": LANE_ID,
            "ts": now_iso(),
            "business_context": business,
            "persona": persona,
            "op": op,
            "active": active,
            "record": record,
            "claim_evidence": [{
                "id": "V7-BUSINESS-001.2",
                "claim": "cross_session_persona_identity_and_capability_lease_state_persist_across_restarts",
                "evidence": { "active": active, "lease_hours": lease_hours }
            }]
        }));
    }

    if op != "issue" && op != "renew" && op != "revoke" {
        return Err("persona_op_invalid".to_string());
    }
    let mut record = personas.get(&key).cloned().unwrap_or_else(|| {
        json!({
            "business_context": business,
            "persona": persona,
            "issued_at": now_iso(),
            "issued_at_epoch": now,
            "renewals": 0_u64,
            "revoked": false
        })
    });
    if op == "revoke" {
        record["revoked"] = Value::Bool(true);
        record["revoked_at"] = Value::String(now_iso());
        record["expires_at_epoch"] = Value::from(now);
    } else {
        let expires = now + (lease_hours * 3600);
        if op == "renew" {
            let renewals = record.get("renewals").and_then(Value::as_u64).unwrap_or(0) + 1;
            record["renewals"] = Value::from(renewals);
        }
        record["revoked"] = Value::Bool(false);
        record["expires_at_epoch"] = Value::from(expires);
        record["lease_hours"] = Value::from(lease_hours);
        record["last_updated"] = Value::String(now_iso());
    }
    personas.insert(key, record.clone());
    write_personas(root, &personas)?;
    Ok(json!({
        "ok": true,
        "type": "business_plane_persona",
        "lane": LANE_ID,
        "ts": now_iso(),
        "business_context": business,
        "persona": persona,
        "op": op,
        "record": record,
        "claim_evidence": [{
            "id": "V7-BUSINESS-001.2",
            "claim": "cross_session_persona_identity_and_capability_lease_state_persist_across_restarts",
            "evidence": { "op": op, "lease_hours": lease_hours }
        }]
    }))
}

fn continuity_chain_path(root: &Path) -> PathBuf {
    lane_root(root).join("continuity_chain.json")
}

fn load_chain(root: &Path) -> Map<String, Value> {
    read_object(&continuity_chain_path(root))
}
