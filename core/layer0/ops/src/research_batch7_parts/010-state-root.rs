// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops::research_batch7 (authoritative)

use crate::v8_kernel::{
    append_jsonl, parse_u64, read_json, scoped_state_root, sha256_hex_str, write_json,
};
use crate::{clean, deterministic_receipt_hash, now_iso, ParsedArgs};
use base64::engine::general_purpose::{STANDARD, URL_SAFE, URL_SAFE_NO_PAD};
use base64::Engine;
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};

const STATE_ENV: &str = "RESEARCH_PLANE_STATE_ROOT";
const STATE_SCOPE: &str = "research_plane";

pub const GOAL_CRAWL_CONTRACT_PATH: &str =
    "planes/contracts/research/goal_seedless_crawl_contract_v1.json";
pub const SITE_MAP_CONTRACT_PATH: &str =
    "planes/contracts/research/site_map_graph_contract_v1.json";
pub const STRUCTURED_EXTRACT_CONTRACT_PATH: &str =
    "planes/contracts/research/structured_extraction_contract_v1.json";
pub const MONITOR_DELTA_CONTRACT_PATH: &str =
    "planes/contracts/research/monitor_delta_contract_v1.json";
pub const FIRECRAWL_TEMPLATE_CONTRACT_PATH: &str =
    "planes/contracts/research/firecrawl_template_governance_contract_v1.json";
pub const FIRECRAWL_TEMPLATE_MANIFEST_PATH: &str =
    "planes/contracts/research/firecrawl_template_pack_manifest_v1.json";
pub const JS_SCRAPE_CONTRACT_PATH: &str =
    "planes/contracts/research/js_render_scrape_profile_contract_v1.json";
pub const AUTH_SESSION_CONTRACT_PATH: &str =
    "planes/contracts/research/auth_session_lifecycle_contract_v1.json";
pub const PROXY_ROTATION_CONTRACT_PATH: &str =
    "planes/contracts/research/proxy_rotation_trap_matrix_contract_v1.json";
pub const NEWS_DECODE_CONTRACT_PATH: &str =
    "planes/contracts/research/google_news_decode_contract_v1.json";

fn state_root(root: &Path) -> PathBuf {
    scoped_state_root(root, STATE_ENV, STATE_SCOPE)
}

fn read_json_or(root: &Path, rel_or_abs: &str, fallback: Value) -> Value {
    let path = if Path::new(rel_or_abs).is_absolute() {
        PathBuf::from(rel_or_abs)
    } else {
        root.join(rel_or_abs)
    };
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
        let path = if Path::new(rel).is_absolute() {
            PathBuf::from(rel)
        } else {
            root.join(rel)
        };
        return read_json(&path).ok_or_else(|| format!("json_path_not_found:{}", path.display()));
    }
    Ok(fallback)
}

fn load_payload(root: &Path, parsed: &ParsedArgs) -> Option<String> {
    parsed
        .flags
        .get("payload")
        .cloned()
        .or_else(|| parsed.flags.get("html").cloned())
        .or_else(|| {
            parsed.flags.get("payload-path").and_then(|p| {
                let path = if Path::new(p).is_absolute() {
                    PathBuf::from(p)
                } else {
                    root.join(p)
                };
                fs::read_to_string(path).ok()
            })
        })
        .or_else(|| {
            parsed.flags.get("html-path").and_then(|p| {
                let path = if Path::new(p).is_absolute() {
                    PathBuf::from(p)
                } else {
                    root.join(p)
                };
                fs::read_to_string(path).ok()
            })
        })
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

fn extract_links(html: &str) -> Vec<String> {
    let mut out = Vec::<String>::new();
    for token in ["href=\"", "href='"] {
        let mut start = 0usize;
        while let Some(found) = html[start..].find(token) {
            let begin = start + found + token.len();
            let rest = &html[begin..];
            let end = rest.find(|c| c == '"' || c == '\'').unwrap_or(rest.len());
            let value = clean(&rest[..end], 1500);
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

fn read_url_content(root: &Path, url: &str) -> String {
    if let Some(path) = url.strip_prefix("file://") {
        let abs = if Path::new(path).is_absolute() {
            PathBuf::from(path)
        } else {
            root.join(path)
        };
        return fs::read_to_string(abs).unwrap_or_default();
    }
    format!("synthetic_page_content_for:{}", clean(url, 1800))
}

fn domain_of(url: &str) -> String {
    if url.starts_with("file://") {
        return "file".to_string();
    }
    clean(
        url.split("://")
            .nth(1)
            .unwrap_or(url)
            .split('/')
            .next()
            .unwrap_or("unknown"),
        180,
    )
    .to_ascii_lowercase()
}

fn guess_bool(raw: Option<&str>, fallback: bool) -> bool {
    raw.map(|v| {
        matches!(
            v.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )
    })
    .unwrap_or(fallback)
}

fn conduit_enforcement(root: &Path, parsed: &ParsedArgs, strict: bool, action: &str) -> Value {
    let bypass_requested = guess_bool(parsed.flags.get("bypass").map(String::as_str), false)
        || guess_bool(parsed.flags.get("direct").map(String::as_str), false)
        || guess_bool(
            parsed.flags.get("unsafe-client-route").map(String::as_str),
            false,
        )
        || guess_bool(parsed.flags.get("client-bypass").map(String::as_str), false);
    let ok = !bypass_requested;
    let mut out = json!({
        "ok": if strict { ok } else { true },
        "type": "research_conduit_enforcement",
        "ts": now_iso(),
        "action": clean(action, 160),
        "required_path": "core/layer0/ops/research_plane",
        "bypass_requested": bypass_requested,
        "errors": if ok { Value::Array(Vec::new()) } else { json!(["conduit_bypass_rejected"]) },
        "claim_evidence": [
            {
                "id": "V6-RESEARCH-004.6",
                "claim": "all_research_planning_crawling_and_extraction_mutations_are_conduit_routed",
                "evidence": {
                    "required_path": "core/layer0/ops/research_plane",
                    "bypass_requested": bypass_requested
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    let history_path = state_root(root).join("conduit").join("history.jsonl");
    let _ = append_jsonl(&history_path, &out);
    out
}

fn fail_payload(kind: &str, strict: bool, errors: Vec<String>, conduit: Option<Value>) -> Value {
    json!({
        "ok": false,
        "strict": strict,
        "type": kind,
        "errors": errors,
        "conduit_enforcement": conduit
    })
}

fn parse_graph(value: Value) -> BTreeMap<String, Vec<String>> {
    let mut out = BTreeMap::<String, Vec<String>>::new();
    let Some(obj) = value.as_object() else {
        return out;
    };
    for (node, row) in obj {
        let links = if let Some(arr) = row.as_array() {
            arr.iter()
                .filter_map(Value::as_str)
                .map(|v| clean(v, 1800))
                .filter(|v| !v.is_empty())
                .collect::<Vec<_>>()
        } else {
            row.get("links")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
                .iter()
                .filter_map(Value::as_str)
                .map(|v| clean(v, 1800))
                .filter(|v| !v.is_empty())
                .collect::<Vec<_>>()
        };
        out.insert(clean(node, 1800), links);
    }
    out
}

fn canonicalize_json(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut keys = map.keys().cloned().collect::<Vec<_>>();
            keys.sort();
            let mut out = Map::new();
            for key in keys {
                if let Some(v) = map.get(&key) {
                    out.insert(key, canonicalize_json(v));
                }
            }
            Value::Object(out)
        }
        Value::Array(rows) => Value::Array(rows.iter().map(canonicalize_json).collect()),
        _ => value.clone(),
    }
}

fn canonical_json_string(value: &Value) -> String {
    serde_json::to_string(&canonicalize_json(value)).unwrap_or_else(|_| "null".to_string())
}

