// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops::research_batch6 (authoritative)

use crate::v8_kernel::{
    append_jsonl, build_conduit_enforcement, conduit_bypass_requested, parse_u64, read_json,
    scoped_state_root, sha256_hex_str, write_json,
};
use crate::{clean, deterministic_receipt_hash, now_iso, ParsedArgs};
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};

const STATE_ENV: &str = "RESEARCH_PLANE_STATE_ROOT";
const STATE_SCOPE: &str = "research_plane";

pub const MCP_CONTRACT_PATH: &str = "planes/contracts/research/mcp_extraction_contract_v1.json";
pub const SPIDER_CONTRACT_PATH: &str = "planes/contracts/research/rule_spider_contract_v1.json";
pub const MIDDLEWARE_CONTRACT_PATH: &str =
    "planes/contracts/research/middleware_stack_contract_v1.json";
pub const PIPELINE_CONTRACT_PATH: &str = "planes/contracts/research/item_pipeline_contract_v1.json";
pub const SIGNAL_BUS_CONTRACT_PATH: &str = "planes/contracts/research/signal_bus_contract_v1.json";
pub const CONSOLE_CONTRACT_PATH: &str = "planes/contracts/research/crawl_console_contract_v1.json";
pub const TEMPLATE_GOVERNANCE_CONTRACT_PATH: &str =
    "planes/contracts/research/template_governance_contract_v1.json";
pub const TEMPLATE_MANIFEST_PATH: &str = "planes/contracts/research/template_pack_manifest_v1.json";
const WIT_WORLD_REGISTRY_PATH: &str = "planes/contracts/wit/world_registry_v1.json";
const WIT_WORLD_BINDING: &str = "infring.metakernel.v1";

fn state_root(root: &Path) -> PathBuf {
    scoped_state_root(root, STATE_ENV, STATE_SCOPE)
}

fn resolve_rooted_path(root: &Path, rel_or_abs: &str) -> PathBuf {
    if Path::new(rel_or_abs).is_absolute() {
        PathBuf::from(rel_or_abs)
    } else {
        root.join(rel_or_abs)
    }
}

fn normalize_claim_evidence(rows: Vec<Value>) -> Vec<Value> {
    rows.into_iter()
        .map(|row| {
            let mut obj = row.as_object().cloned().unwrap_or_default();
            if obj
                .get("id")
                .and_then(Value::as_str)
                .map(|v| v.trim().is_empty())
                .unwrap_or(true)
            {
                obj.insert(
                    "id".to_string(),
                    Value::String("V6-RESEARCH-PLANE".to_string()),
                );
            }
            if obj
                .get("claim")
                .and_then(Value::as_str)
                .map(|v| v.trim().is_empty())
                .unwrap_or(true)
            {
                obj.insert(
                    "claim".to_string(),
                    Value::String("research_plane_deterministic_receipt_emission".to_string()),
                );
            }
            if !obj.get("evidence").map(Value::is_object).unwrap_or(false) {
                obj.insert("evidence".to_string(), json!({}));
            }
            Value::Object(obj)
        })
        .collect::<Vec<_>>()
}

fn finalize_receipt(mut out: Value) -> Value {
    if !out.is_object() {
        out = json!({
            "ok": false,
            "type": "research_plane_error",
            "errors": ["invalid_receipt_shape"]
        });
    }
    if out.get("lane").is_none() {
        out["lane"] = Value::String("core/layer0/ops".to_string());
    }
    if out.get("strict").is_none() {
        out["strict"] = Value::Bool(true);
    }
    out["world_binding"] = json!({
        "registry_path": WIT_WORLD_REGISTRY_PATH,
        "world": WIT_WORLD_BINDING
    });
    let claims = out
        .get("claim_evidence")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    out["claim_evidence"] = Value::Array(normalize_claim_evidence(claims));
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    out
}

fn read_json_or(root: &Path, rel_or_abs: &str, fallback: Value) -> Value {
    let path = resolve_rooted_path(root, rel_or_abs);
    read_json(&path).unwrap_or(fallback)
}

fn parse_list_flag(parsed: &ParsedArgs, key: &str, max_item_len: usize) -> Vec<String> {
    parsed
        .flags
        .get(key)
        .map(|v| {
            v.split(',')
                .map(|part| clean(part, max_item_len))
                .filter(|part| !part.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn parse_json_flag_or_path(
    root: &Path,
    parsed: &ParsedArgs,
    json_key: &str,
    path_key: &str,
    fallback: Value,
) -> Result<Value, String> {
    if let Some(raw) = parsed.flags.get(json_key) {
        return serde_json::from_str::<Value>(raw)
            .map_err(|err| format!("invalid_json_flag:{json_key}:{err}"));
    }
    if let Some(rel) = parsed.flags.get(path_key) {
        let path = resolve_rooted_path(root, rel);
        return read_json(&path).ok_or_else(|| format!("json_path_not_found:{}", path.display()));
    }
    Ok(fallback)
}

fn contract_name_set(contract: &Value, key: &str) -> BTreeSet<String> {
    contract
        .get(key)
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|value| value.as_str().map(|v| clean(v, 80).to_ascii_lowercase()))
        .filter(|value| !value.is_empty())
        .collect::<BTreeSet<_>>()
}

fn display_value_text(value: &Value, max_len: usize) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(text) => clean(text, max_len),
        _ => clean(value.to_string(), max_len),
    }
}

fn csv_escape(value: &Value) -> String {
    let rendered = display_value_text(value, 600);
    if rendered.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", rendered.replace('"', "\"\""))
    } else {
        rendered
    }
}

fn mode_requires_safety(mode: &str, policy: &Value) -> bool {
    let mode_norm = clean(mode, 64).to_ascii_lowercase();
    let defaults = vec!["stealth".to_string(), "browser".to_string()];
    let required = policy
        .get("safety_plane")
        .and_then(|v| v.get("required_modes"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_else(|| defaults.iter().map(|v| Value::String(v.clone())).collect());
    required
        .iter()
        .filter_map(Value::as_str)
        .map(|v| v.to_ascii_lowercase())
        .any(|v| v == mode_norm)
}

fn pattern_match(action: &str, pattern: &str) -> bool {
    let a = action.to_ascii_lowercase();
    let p = pattern.to_ascii_lowercase();
    if p.is_empty() || p == "*" || p == "all" {
        return true;
    }
    if p.contains('*') {
        let parts = p
            .split('*')
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .collect::<Vec<_>>();
        if parts.is_empty() {
            return true;
        }
        return parts.iter().all(|part| a.contains(part));
    }
    a == p || a.contains(&p)
}

fn conduit_enforcement(root: &Path, parsed: &ParsedArgs, strict: bool, action: &str) -> Value {
    let bypass_requested = conduit_bypass_requested(&parsed.flags);
    let out = build_conduit_enforcement(
        root,
        STATE_ENV,
        STATE_SCOPE,
        strict,
        action,
        "research_conduit_enforcement",
        "core/layer0/ops/research_plane",
        bypass_requested,
        vec![json!({
            "id": "V6-RESEARCH-002.6",
            "claim": "research_template_and_crawler_controls_are_conduit_routed_with_fail_closed_bypass_rejection",
            "evidence": {
                "required_path": "core/layer0/ops/research_plane",
                "bypass_requested": bypass_requested
            }
        })],
    );
    finalize_receipt(out)
}

pub fn safety_gate_receipt(
    root: &Path,
    policy: &Value,
    mode: &str,
    action: &str,
    target: &str,
    strict: bool,
) -> Value {
    let mode_norm = clean(mode, 64).to_ascii_lowercase();
    let action_norm = clean(action, 160).to_ascii_lowercase();
    let target_norm = clean(target, 400);
    let enabled = policy
        .get("safety_plane")
        .and_then(|v| v.get("enabled"))
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let safety_required = mode_requires_safety(&mode_norm, policy);
    let allowed_patterns = policy
        .get("safety_plane")
        .and_then(|v| v.get("allow_actions"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_else(|| {
            vec![
                Value::String("research:*".to_string()),
                Value::String("research_fetch:*".to_string()),
                Value::String("research_crawl:*".to_string()),
            ]
        });
    let action_allowed = allowed_patterns
        .iter()
        .filter_map(Value::as_str)
        .any(|pattern| pattern_match(&action_norm, pattern));

    let counters_path = state_root(root).join("safety").join("gate_counters.json");
    let mut counters = read_json(&counters_path).unwrap_or_else(|| {
        json!({
            "total": 0_u64,
            "modes": {}
        })
    });
    let mode_limit = policy
        .get("safety_plane")
        .and_then(|v| v.get("max_requests_per_mode"))
        .and_then(|v| v.get(&mode_norm))
        .and_then(Value::as_u64)
        .unwrap_or(20_000);
    let mode_used = counters
        .get("modes")
        .and_then(|v| v.get(&mode_norm))
        .and_then(Value::as_u64)
        .unwrap_or(0);

    let mut errors = Vec::<String>::new();
    if safety_required && !enabled {
        errors.push("safety_plane_disabled_for_required_mode".to_string());
    }
    if safety_required && !action_allowed {
        errors.push("safety_plane_action_not_allowed".to_string());
    }
    if safety_required && mode_used >= mode_limit {
        errors.push("safety_plane_budget_exhausted".to_string());
    }

    let ok = errors.is_empty() && enabled;
    if ok {
        let next_total = counters
            .get("total")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            .saturating_add(1);
        counters["total"] = Value::Number(next_total.into());
        if !counters.get("modes").map(Value::is_object).unwrap_or(false) {
            counters["modes"] = Value::Object(Map::new());
        }
        let next_mode = mode_used.saturating_add(1);
        counters["modes"][mode_norm.clone()] = Value::Number(next_mode.into());
        let _ = write_json(&counters_path, &counters);
    }

    let out = finalize_receipt(json!({
        "ok": if strict { ok } else { true },
        "type": "research_safety_gate",
        "ts": now_iso(),
        "mode": mode_norm,
        "action": action_norm,
        "target": target_norm,
        "enabled": enabled,
        "required": safety_required,
        "action_allowed": action_allowed,
        "mode_budget": {"used": mode_used, "limit": mode_limit},
        "errors": errors,
        "claim_evidence": [
            {
                "id": "V6-RESEARCH-001.5",
                "claim": "stealth_and_browser_paths_are_fail_closed_through_safety_plane_gate",
                "evidence": {
                    "mode": mode,
                    "required": safety_required
                }
            }
        ]
    }));
    let history_path = state_root(root).join("safety").join("history.jsonl");
    let _ = append_jsonl(&history_path, &out);
    out
}

fn strip_tags(html: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for ch in html.chars() {
        if ch == '<' {
            in_tag = true;
            continue;
        }
        if ch == '>' {
            in_tag = false;
            out.push(' ');
            continue;
        }
        if !in_tag {
            out.push(ch);
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn extract_links(html: &str) -> Vec<String> {
    let mut out = Vec::<String>::new();
    for token in ["href=\"", "href='"] {
        let mut start = 0usize;
        while let Some(found) = html[start..].find(token) {
            let begin = start + found + token.len();
            let rest = &html[begin..];
            let end = rest.find(|c| c == '"' || c == '\'').unwrap_or(rest.len());
            let value = clean(&rest[..end], 1024);
            if !value.is_empty() {
                out.push(value);
            }
            start = begin.saturating_add(end);
            if start >= html.len() {
                break;
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

fn parse_title(html: &str) -> String {
    let low = html.to_ascii_lowercase();
    if let Some(start) = low.find("<title>") {
        let body = &html[start + 7..];
        if let Some(end) = body.to_ascii_lowercase().find("</title>") {
            return clean(&body[..end], 200);
        }
    }
    "untitled".to_string()
}
