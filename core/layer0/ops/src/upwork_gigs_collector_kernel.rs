// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use chrono::{DateTime, Utc};
use regex::Regex;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::contract_lane_utils as lane_utils;

fn usage() {
    println!("upwork-gigs-collector-kernel commands:");
    println!("  protheus-ops upwork-gigs-collector-kernel run --payload-base64=<json>");
    println!("  protheus-ops upwork-gigs-collector-kernel prepare-run --payload-base64=<json>");
    println!("  protheus-ops upwork-gigs-collector-kernel build-fetch-plan --payload-base64=<json>");
    println!("  protheus-ops upwork-gigs-collector-kernel finalize-run --payload-base64=<json>");
    println!("  protheus-ops upwork-gigs-collector-kernel parse-rss --payload-base64=<json>");
    println!("  protheus-ops upwork-gigs-collector-kernel map-gigs --payload-base64=<json>");
    println!("  protheus-ops upwork-gigs-collector-kernel fallback-gigs --payload-base64=<json>");
    println!("  protheus-ops upwork-gigs-collector-kernel collect --payload-base64=<json>");
}

const COLLECTOR_ID: &str = "upwork_gigs";
const EYES_STATE_DEFAULT_REL: &str = "local/state/sensory/eyes";

fn clean_text(raw: Option<&str>, max_len: usize) -> String {
    lane_utils::clean_text(raw, max_len)
}

fn clamp_u64(payload: &Map<String, Value>, key: &str, fallback: u64, lo: u64, hi: u64) -> u64 {
    payload
        .get(key)
        .and_then(Value::as_u64)
        .unwrap_or(fallback)
        .clamp(lo, hi)
}

fn now_iso() -> String {
    Utc::now().to_rfc3339()
}

fn as_bool(value: Option<&Value>, fallback: bool) -> bool {
    value.and_then(Value::as_bool).unwrap_or(fallback)
}

fn as_f64(value: Option<&Value>, fallback: f64) -> f64 {
    value.and_then(Value::as_f64).unwrap_or(fallback)
}

fn resolve_eyes_state_dir(root: &Path, payload: &Map<String, Value>) -> PathBuf {
    if let Some(raw) = payload.get("eyes_state_dir").and_then(Value::as_str) {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            let candidate = PathBuf::from(trimmed);
            if candidate.is_absolute() {
                return candidate;
            }
            return root.join(candidate);
        }
    }
    if let Ok(raw) = std::env::var("EYES_STATE_DIR") {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    root.join(EYES_STATE_DEFAULT_REL)
}

fn meta_path_for(root: &Path, payload: &Map<String, Value>) -> PathBuf {
    resolve_eyes_state_dir(root, payload)
        .join("collector_meta")
        .join(format!("{COLLECTOR_ID}.json"))
}

fn cache_path_for(root: &Path, payload: &Map<String, Value>) -> PathBuf {
    resolve_eyes_state_dir(root, payload)
        .join("collector_meta")
        .join(format!("{COLLECTOR_ID}.cache.json"))
}

fn read_json(path: &Path, fallback: Value) -> Value {
    match fs::read_to_string(path) {
        Ok(raw) => serde_json::from_str::<Value>(&raw).unwrap_or(fallback),
        Err(_) => fallback,
    }
}

fn write_json_atomic(path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("upwork_gigs_collector_kernel_create_dir_failed:{err}"))?;
    }
    let tmp = path.with_extension(format!("tmp-{}-{}", std::process::id(), Utc::now().timestamp_millis()));
    let body = format!(
        "{}\n",
        serde_json::to_string_pretty(value)
            .map_err(|err| format!("upwork_gigs_collector_kernel_encode_failed:{err}"))?
    );
    fs::write(&tmp, body).map_err(|err| format!("upwork_gigs_collector_kernel_write_failed:{err}"))?;
    fs::rename(&tmp, path).map_err(|err| format!("upwork_gigs_collector_kernel_rename_failed:{err}"))
}

fn clean_seen_id(raw: &str) -> String {
    let mut out = String::new();
    for ch in raw.chars() {
        if out.len() >= 120 {
            break;
        }
        let keep = ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | ':' | '.');
        if keep {
            out.push(ch);
        }
    }
    out
}

fn normalize_meta_value(raw: Option<&Value>) -> Value {
    let obj = raw.and_then(Value::as_object);
    let last_run = clean_text(obj.and_then(|o| o.get("last_run")).and_then(Value::as_str), 80);
    let last_success = clean_text(obj.and_then(|o| o.get("last_success")).and_then(Value::as_str), 80);
    let mut seen_ids = Vec::new();
    if let Some(items) = obj.and_then(|o| o.get("seen_ids")).and_then(Value::as_array) {
        for entry in items {
            if let Some(raw_id) = entry.as_str() {
                let cleaned = clean_seen_id(raw_id);
                if !cleaned.is_empty() {
                    seen_ids.push(Value::String(cleaned));
                }
            }
        }
    }
    if seen_ids.len() > 2000 {
        let split = seen_ids.len() - 2000;
        seen_ids = seen_ids.into_iter().skip(split).collect::<Vec<_>>();
    }
    json!({
        "collector_id": COLLECTOR_ID,
        "last_run": if last_run.is_empty() { Value::Null } else { Value::String(last_run) },
        "last_success": if last_success.is_empty() { Value::Null } else { Value::String(last_success) },
        "seen_ids": seen_ids
    })
}

fn parse_iso_ms(raw: &str) -> Option<i64> {
    DateTime::parse_from_rfc3339(raw)
        .ok()
        .map(|dt| dt.timestamp_millis())
}

fn encode_query_component(raw: &str) -> String {
    let mut out = String::new();
    for b in raw.bytes() {
        let keep = b.is_ascii_uppercase()
            || b.is_ascii_lowercase()
            || b.is_ascii_digit()
            || matches!(b, b'-' | b'_' | b'.' | b'~');
        if keep {
            out.push(char::from(b));
        } else if b == b' ' {
            out.push('+');
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

fn classify_curl_transport_error(stderr: &str) -> String {
    let lower = stderr.to_ascii_lowercase();
    if lower.contains("could not resolve host")
        || lower.contains("name or service not known")
        || lower.contains("temporary failure in name resolution")
    {
        return "dns_unreachable".to_string();
    }
    if lower.contains("connection refused") {
        return "connection_refused".to_string();
    }
    if lower.contains("operation timed out")
        || lower.contains("connection timed out")
        || lower.contains("timed out")
    {
        return "timeout".to_string();
    }
    if lower.contains("ssl") || lower.contains("tls") || lower.contains("certificate") {
        return "tls_error".to_string();
    }
    "collector_error".to_string()
}

fn http_status_to_code(status: u64) -> &'static str {
    match status {
        401 => "auth_unauthorized",
        403 => "auth_forbidden",
        404 => "http_404",
        408 => "timeout",
        429 => "rate_limited",
        500..=u64::MAX => "http_5xx",
        400..=499 => "http_4xx",
        _ => "http_error",
    }
}

fn curl_fetch_with_status(url: &str, timeout_ms: u64, accept: &str) -> Result<(u64, String, u64), String> {
    let timeout_secs = ((timeout_ms.max(1_000) as f64) / 1_000.0).ceil() as u64;
    let output = Command::new("curl")
        .arg("--silent")
        .arg("--show-error")
        .arg("--location")
        .arg("--max-time")
        .arg(timeout_secs.to_string())
        .arg("-H")
        .arg("User-Agent: Infring-Eyes/1.0")
        .arg("-H")
        .arg(format!("Accept: {accept}"))
        .arg("-H")
        .arg("Accept-Language: en-US,en;q=0.9")
        .arg("-w")
        .arg("\n__PROTHEUS_STATUS__:%{http_code}\n")
        .arg(url)
        .output()
        .map_err(|err| format!("collector_fetch_spawn_failed:{err}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let code = classify_curl_transport_error(&stderr);
        return Err(format!("{code}:{}", clean_text(Some(&stderr), 220)));
    }

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let marker = "\n__PROTHEUS_STATUS__:";
    let marker_pos = stdout
        .rfind(marker)
        .ok_or_else(|| "collector_fetch_missing_status_marker".to_string())?;
    let body = stdout[..marker_pos].to_string();
    let status_raw = stdout[(marker_pos + marker.len())..]
        .lines()
        .next()
        .unwrap_or("0")
        .trim()
        .to_string();
    let status = status_raw.parse::<u64>().unwrap_or(0);
    let bytes = body.as_bytes().len() as u64;
    Ok((status, body, bytes))
}

fn load_cache_items(root: &Path, payload: &Map<String, Value>) -> Vec<Value> {
    read_json(&cache_path_for(root, payload), json!({ "items": [] }))
        .get("items")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn today_utc() -> String {
    Utc::now().format("%Y-%m-%d").to_string()
}

fn sha16(input: &str) -> String {
    let digest = Sha256::digest(input.as_bytes());
    hex::encode(digest)[..16].to_string()
}

fn strip_tags(raw: &str) -> String {
    let no_tags = match Regex::new(r"(?is)<[^>]+>") {
        Ok(re) => re.replace_all(raw, " ").to_string(),
        Err(_) => raw.to_string(),
    };
    clean_text(Some(&no_tags), 1_600)
}

fn decode_html(raw: &str) -> String {
    raw.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
}

fn extract_tag(block: &str, tag: &str, max_len: usize) -> String {
    let pat = format!(r"(?is)<{}\b[^>]*>(.*?)</{}\s*>", regex::escape(tag), regex::escape(tag));
    let re = match Regex::new(&pat) {
        Ok(v) => v,
        Err(_) => return String::new(),
    };
    re.captures(block)
        .and_then(|caps| caps.get(1).map(|m| m.as_str()))
        .map(|raw| clean_text(Some(&decode_html(&strip_tags(raw))), max_len))
        .unwrap_or_default()
}

fn parse_rss(xml: &str) -> Vec<Value> {
    let item_re = match Regex::new(r"(?is)<item\b.*?</item>") {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    let mut out = Vec::<Value>::new();
    for mat in item_re.find_iter(xml) {
        let block = mat.as_str();
        let title = extract_tag(block, "title", 220);
        let url = extract_tag(block, "link", 600);
        if title.is_empty() || url.is_empty() {
            continue;
        }
        out.push(json!({
            "title": title,
            "url": url,
            "description": extract_tag(block, "description", 420),
            "pubDate": extract_tag(block, "pubDate", 120),
            "budget": extract_tag(block, "budget", 120),
        }));
    }
    out
}

fn score_keywords() -> &'static [&'static str] {
    &[
        "ai",
        "artificial intelligence",
        "automation",
        "chatbot",
        "gpt",
        "llm",
        "openai",
        "claude",
        "anthropic",
        "agent",
        "workflow",
        "n8n",
        "make",
        "nocode",
        "no-code",
        "lowcode",
        "bubble",
        "webflow",
        "zapier",
        "script",
        "bot",
        "scraper",
        "api integration",
        "webhook",
        "chrome extension",
        "browser extension",
        "plugin",
        "data pipeline",
        "etl",
        "database",
        "supabase",
        "firebase",
        "nextjs",
        "next.js",
        "react",
        "typescript",
        "javascript",
    ]
}

fn score_gig_value(title: &str, description: &str) -> i64 {
    let hay = format!("{title} {description}").to_lowercase();
    let mut score = 0_i64;
    for kw in score_keywords() {
        if hay.contains(kw) {
            score += 2;
        }
    }
    if hay.contains("$$$$") || hay.contains("fixed price") {
        score += 3;
    }
    if hay.contains("$$$") {
        score += 2;
    }
    score
}

fn normalize_seen_ids(payload: &Map<String, Value>) -> HashSet<String> {
    payload
        .get("seen_ids")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|row| clean_text(Some(row), 120))
                .filter(|row| !row.is_empty())
                .collect::<HashSet<_>>()
        })
        .unwrap_or_default()
}

fn date_seed(payload: &Map<String, Value>) -> String {
    let raw = clean_text(payload.get("date").and_then(Value::as_str), 32);
    if raw.is_empty() { today_utc() } else { raw }
}

fn map_gigs(payload: &Map<String, Value>) -> Value {
    let max_items = clamp_u64(payload, "max_items", 10, 1, 200) as usize;
    let date = date_seed(payload);
    let collected_at = now_iso();
    let mut seen = normalize_seen_ids(payload);
    let rows = payload
        .get("gigs")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let mut items = Vec::<Value>::new();
    for row in rows {
        if items.len() >= max_items {
            break;
        }
        let obj = match row.as_object() {
            Some(v) => v,
            None => continue,
        };
        let title = clean_text(obj.get("title").and_then(Value::as_str), 220);
        let url = clean_text(obj.get("url").and_then(Value::as_str), 600);
        if title.is_empty() || url.is_empty() {
            continue;
        }
        let description = clean_text(obj.get("description").and_then(Value::as_str), 420);
        let budget = clean_text(obj.get("budget").and_then(Value::as_str), 120);
        let pub_date = clean_text(obj.get("pubDate").and_then(Value::as_str), 120);
        let id = sha16(&format!("gig-{title}-{url}-{date}"));
        if seen.contains(&id) {
            continue;
        }
        seen.insert(id.clone());

        let value_score = score_gig_value(&title, &description);
        let is_high_value = value_score >= 4;
        items.push(json!({
            "id": id,
            "collected_at": collected_at,
            "url": url,
            "title": title,
            "description": if description.is_empty() {
                Value::String(format!("Upwork gig value score: {value_score}"))
            } else {
                Value::String(description)
            },
            "budget": if budget.is_empty() { Value::Null } else { Value::String(budget) },
            "pubDate": if pub_date.is_empty() { Value::Null } else { Value::String(pub_date) },
            "value_score": value_score,
            "signal_type": if is_high_value { "high_value_gig" } else { "freelance_opportunity" },
            "signal": is_high_value,
            "source": "upwork_gigs",
            "tags": ["freelance", if is_high_value { "high-value" } else { "standard" }, "gig"],
            "topics": ["revenue", "freelance", "gigs", "opportunities"],
            "bytes": 0
        }));
    }

    let mut seen_ids = seen.into_iter().collect::<Vec<_>>();
    seen_ids.sort();
    if seen_ids.len() > 2000 {
        let drop = seen_ids.len() - 2000;
        seen_ids.drain(0..drop);
    }
    json!({
        "ok": true,
        "items": items,
        "seen_ids": seen_ids
    })
}

fn fallback_gigs(payload: &Map<String, Value>) -> Value {
    let max_items = clamp_u64(payload, "max_items", 10, 1, 200) as usize;
    let date = date_seed(payload);
    let collected_at = now_iso();
    let mut seen = normalize_seen_ids(payload);
    let seed = [
        (
            "AI Automation Specialist - Workflow Optimization",
            "https://www.upwork.com/jobs/ai-automation-workflow",
            "Looking for expert to build AI agent workflows using n8n and OpenAI API. Budget: $5,000+",
            "$5,000+",
        ),
        (
            "Chrome Extension Developer - AI Assistant",
            "https://www.upwork.com/jobs/chrome-extension-ai",
            "Build browser extension that integrates with Claude API for content summarization. Budget: $2,000-$5,000",
            "$2,000-$5,000",
        ),
        (
            "No-Code SaaS MVP Builder",
            "https://www.upwork.com/jobs/nocode-saas-mvp",
            "Create functional MVP using Bubble or Webflow with database integration. Budget: $3,000-$8,000",
            "$3,000-$8,000",
        ),
    ];

    let mut items = Vec::<Value>::new();
    for (title, url, description, budget) in seed {
        if items.len() >= max_items {
            break;
        }
        let id = sha16(&format!("gig-{title}-{date}"));
        if seen.contains(&id) {
            continue;
        }
        seen.insert(id.clone());
        let value_score = score_gig_value(title, description);
        items.push(json!({
            "id": id,
            "collected_at": collected_at,
            "url": url,
            "title": format!("{title} — Freelance Opportunity"),
            "description": format!("{description} Value score: {value_score}. Fallback data."),
            "budget": budget,
            "value_score": value_score,
            "signal_type": "high_value_gig",
            "signal": true,
            "source": "upwork_gigs",
            "tags": ["freelance", "high-value", "gig", "fallback"],
            "topics": ["revenue", "freelance", "gigs"],
            "bytes": 0
        }));
    }

    let mut seen_ids = seen.into_iter().collect::<Vec<_>>();
    seen_ids.sort();
    if seen_ids.len() > 2000 {
        let drop = seen_ids.len() - 2000;
        seen_ids.drain(0..drop);
    }
    json!({
        "ok": true,
        "items": items,
        "seen_ids": seen_ids
    })
}

fn command_prepare_run(root: &Path, payload: &Map<String, Value>) -> Value {
    let force = as_bool(payload.get("force"), false);
    let min_hours = as_f64(payload.get("min_hours"), 4.0).clamp(0.0, 24.0 * 365.0);
    let meta_path = meta_path_for(root, payload);
    let meta = normalize_meta_value(Some(&read_json(&meta_path, normalize_meta_value(None))));
    let last_run_ms = meta
        .get("last_run")
        .and_then(Value::as_str)
        .and_then(parse_iso_ms);
    let hours_since_last =
        last_run_ms.map(|ms| ((Utc::now().timestamp_millis() - ms) as f64 / 3_600_000.0).max(0.0));
    let skipped = !force && hours_since_last.map(|h| h < min_hours).unwrap_or(false);
    json!({
        "ok": true,
        "collector_id": COLLECTOR_ID,
        "force": force,
        "min_hours": min_hours,
        "hours_since_last": hours_since_last,
        "skipped": skipped,
        "reason": if skipped { Value::String("cadence".to_string()) } else { Value::Null },
        "meta": meta,
        "meta_path": meta_path.display().to_string()
    })
}

fn command_build_fetch_plan(payload: &Map<String, Value>) -> Value {
    let query = clean_text(
        payload
            .get("search_query")
            .and_then(Value::as_str)
            .or_else(|| payload.get("q").and_then(Value::as_str)),
        240,
    );
    let query = if query.is_empty() {
        "automation OR ai OR nocode OR chatbot OR agent".to_string()
    } else {
        query
    };
    let encoded = encode_query_component(&query);
    json!({
        "ok": true,
        "collector_id": COLLECTOR_ID,
        "search_query": query,
        "requests": [
            {
                "key": "rss",
                "url": format!("https://www.upwork.com/ab/feed/jobs/rss?q={encoded}&sort=recency&paging=0-10"),
                "required": true,
                "accept": "application/rss+xml,application/xml,text/xml,*/*"
            }
        ]
    })
}

fn finalize_success(
    root: &Path,
    payload: &Map<String, Value>,
    min_hours: f64,
    max_items: usize,
    bytes: u64,
    requests: u64,
    duration_ms: u64,
) -> Result<Value, String> {
    let mut meta = normalize_meta_value(payload.get("meta"));
    let today = date_seed(payload);
    let initial_seen = meta
        .get("seen_ids")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let gigs = payload
        .get("gigs")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let mut degraded = false;
    let mapped = if !gigs.is_empty() {
        map_gigs(&json!({
            "date": today,
            "max_items": max_items,
            "seen_ids": initial_seen,
            "gigs": gigs
        }).as_object().cloned().unwrap_or_default())
    } else {
        degraded = true;
        fallback_gigs(&json!({
            "date": today,
            "max_items": max_items,
            "seen_ids": initial_seen
        }).as_object().cloned().unwrap_or_default())
    };

    let items = mapped
        .get("items")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let seen_ids = mapped
        .get("seen_ids")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    meta["seen_ids"] = Value::Array(seen_ids);
    meta["last_run"] = Value::String(now_iso());
    if !items.is_empty() {
        meta["last_success"] = Value::String(now_iso());
        write_json_atomic(&cache_path_for(root, payload), &json!({ "items": items }))?;
    }
    write_json_atomic(&meta_path_for(root, payload), &meta)?;

    let sample = items
        .first()
        .and_then(Value::as_object)
        .and_then(|o| o.get("title"))
        .and_then(Value::as_str)
        .map(|s| clean_text(Some(s), 80))
        .filter(|s| !s.is_empty())
        .map(Value::String)
        .unwrap_or(Value::Null);

    Ok(json!({
        "ok": true,
        "success": true,
        "eye": COLLECTOR_ID,
        "items": items,
        "bytes": bytes,
        "duration_ms": duration_ms,
        "requests": requests.max(1),
        "cadence_hours": min_hours,
        "degraded": degraded,
        "sample": sample
    }))
}

fn finalize_error(
    root: &Path,
    payload: &Map<String, Value>,
    min_hours: f64,
    max_items: usize,
    bytes: u64,
    requests: u64,
    duration_ms: u64,
    error: &str,
) -> Result<Value, String> {
    let mut meta = normalize_meta_value(payload.get("meta"));
    let cached = load_cache_items(root, payload);
    if !cached.is_empty() {
        return Ok(json!({
            "ok": true,
            "success": true,
            "eye": COLLECTOR_ID,
            "cache_hit": true,
            "degraded": true,
            "error": clean_text(Some(error), 120),
            "items": cached.into_iter().take(max_items).collect::<Vec<_>>(),
            "bytes": bytes,
            "requests": requests,
            "duration_ms": duration_ms,
            "cadence_hours": min_hours
        }));
    }

    let initial_seen = meta
        .get("seen_ids")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let fallback = fallback_gigs(&json!({
        "date": date_seed(payload),
        "max_items": max_items,
        "seen_ids": initial_seen
    }).as_object().cloned().unwrap_or_default());
    let fallback_items = fallback
        .get("items")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    meta["last_run"] = Value::String(now_iso());
    if let Some(seen_ids) = fallback.get("seen_ids").and_then(Value::as_array) {
        meta["seen_ids"] = Value::Array(seen_ids.clone());
    }
    write_json_atomic(&meta_path_for(root, payload), &meta)?;

    let sample = fallback_items
        .first()
        .and_then(Value::as_object)
        .and_then(|o| o.get("title"))
        .and_then(Value::as_str)
        .map(|s| clean_text(Some(s), 80))
        .filter(|s| !s.is_empty())
        .map(Value::String)
        .unwrap_or(Value::Null);

    Ok(json!({
        "ok": true,
        "success": true,
        "eye": COLLECTOR_ID,
        "items": fallback_items,
        "bytes": bytes,
        "duration_ms": duration_ms,
        "requests": requests.max(1),
        "cadence_hours": min_hours,
        "degraded": true,
        "error": clean_text(Some(error), 120),
        "sample": sample
    }))
}

fn command_finalize_run(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let min_hours = as_f64(payload.get("min_hours"), 4.0).clamp(0.0, 24.0 * 365.0);
    let max_items = clamp_u64(payload, "max_items", 10, 1, 200) as usize;
    let bytes = clamp_u64(payload, "bytes", 0, 0, u64::MAX);
    let requests = clamp_u64(payload, "requests", 0, 0, u64::MAX);
    let duration_ms = clamp_u64(payload, "duration_ms", 0, 0, u64::MAX);
    let fetch_error = clean_text(payload.get("fetch_error").and_then(Value::as_str), 220);
    if !fetch_error.is_empty() {
        return finalize_error(
            root,
            payload,
            min_hours,
            max_items,
            bytes,
            requests,
            duration_ms,
            &fetch_error,
        );
    }
    finalize_success(root, payload, min_hours, max_items, bytes, requests, duration_ms)
}

fn command_collect(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let min_hours = as_f64(payload.get("min_hours"), 4.0).clamp(0.0, 24.0 * 365.0);
    let max_items = clamp_u64(payload, "max_items", 10, 1, 200) as usize;
    let force = as_bool(payload.get("force"), false);
    let prepared = command_prepare_run(
        root,
        &json!({
            "eyes_state_dir": payload.get("eyes_state_dir").cloned().unwrap_or(Value::Null),
            "force": force,
            "min_hours": min_hours
        })
        .as_object()
        .cloned()
        .unwrap_or_default(),
    );

    if prepared.get("skipped").and_then(Value::as_bool) == Some(true) {
        return Ok(json!({
            "ok": true,
            "success": true,
            "eye": COLLECTOR_ID,
            "skipped": true,
            "reason": "cadence",
            "hours_since_last": prepared.get("hours_since_last").cloned().unwrap_or(Value::Null),
            "min_hours": min_hours,
            "items": []
        }));
    }

    let rss_xml = payload
        .get("rss_xml")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let gigs = if rss_xml.trim().is_empty() {
        Vec::<Value>::new()
    } else {
        parse_rss(rss_xml.as_str())
    };
    let bytes = clamp_u64(payload, "bytes", 0, 0, u64::MAX);
    let requests = clamp_u64(payload, "requests", 0, 0, u64::MAX);
    let duration_ms = clamp_u64(payload, "duration_ms", 0, 0, u64::MAX);
    let fetch_error = clean_text(payload.get("fetch_error").and_then(Value::as_str), 220);

    command_finalize_run(
        root,
        &json!({
            "eyes_state_dir": payload.get("eyes_state_dir").cloned().unwrap_or(Value::Null),
            "meta": prepared.get("meta").cloned().unwrap_or_else(|| normalize_meta_value(None)),
            "min_hours": min_hours,
            "max_items": max_items,
            "bytes": bytes,
            "requests": requests,
            "duration_ms": duration_ms,
            "gigs": gigs,
            "fetch_error": if fetch_error.is_empty() { Value::Null } else { Value::String(fetch_error) }
        })
        .as_object()
        .cloned()
        .unwrap_or_default(),
    )
}

fn command_run(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let max_items = clamp_u64(payload, "max_items", 10, 1, 200) as usize;
    let min_hours = as_f64(payload.get("min_hours"), 4.0).clamp(0.0, 24.0 * 365.0);
    let force = as_bool(payload.get("force"), false);
    let timeout_ms = clamp_u64(payload, "timeout_ms", 15_000, 1_000, 120_000);
    let search_query = clean_text(payload.get("search_query").and_then(Value::as_str), 240);
    let started_at_ms = Utc::now().timestamp_millis().max(0) as u64;

    let plan = command_build_fetch_plan(
        &json!({ "search_query": search_query })
            .as_object()
            .cloned()
            .unwrap_or_default(),
    );
    let request = plan
        .get("requests")
        .and_then(Value::as_array)
        .and_then(|rows| rows.first())
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let fetch_url = clean_text(request.get("url").and_then(Value::as_str), 800);
    let accept = clean_text(request.get("accept").and_then(Value::as_str), 120);

    let (rss_xml, bytes, requests, fetch_error) =
        match curl_fetch_with_status(&fetch_url, timeout_ms, &accept) {
            Ok((status, body, body_bytes)) => {
                if status >= 400 {
                    (
                        String::new(),
                        0_u64,
                        0_u64,
                        Some(http_status_to_code(status).to_string()),
                    )
                } else {
                    (body, body_bytes, 1_u64, None)
                }
            }
            Err(err) => {
                let code = clean_text(Some(&err), 120)
                    .split(':')
                    .next()
                    .unwrap_or("collector_error")
                    .to_string();
                (String::new(), 0_u64, 0_u64, Some(code))
            }
        };

    let duration_ms = Utc::now()
        .timestamp_millis()
        .max(0)
        .saturating_sub(started_at_ms as i64) as u64;

    command_collect(
        root,
        &json!({
            "eyes_state_dir": payload.get("eyes_state_dir").cloned().unwrap_or(Value::Null),
            "force": force,
            "min_hours": min_hours,
            "max_items": max_items,
            "bytes": bytes,
            "requests": requests,
            "duration_ms": duration_ms,
            "rss_xml": rss_xml,
            "fetch_error": fetch_error
        })
        .as_object()
        .cloned()
        .unwrap_or_default(),
    )
}

fn dispatch(root: &Path, command: &str, payload: &Map<String, Value>) -> Result<Value, String> {
    match command {
        "run" => command_run(root, payload),
        "prepare-run" => Ok(command_prepare_run(root, payload)),
        "build-fetch-plan" => Ok(command_build_fetch_plan(payload)),
        "finalize-run" => command_finalize_run(root, payload),
        "collect" => command_collect(root, payload),
        "parse-rss" => {
            let xml = payload.get("xml").and_then(Value::as_str).unwrap_or("");
            Ok(json!({ "ok": true, "gigs": parse_rss(xml) }))
        }
        "map-gigs" => Ok(map_gigs(payload)),
        "fallback-gigs" => Ok(fallback_gigs(payload)),
        _ => Err("upwork_gigs_collector_kernel_unknown_command".to_string()),
    }
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    if argv.is_empty() || matches!(argv[0].as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }

    let command = argv[0].trim().to_ascii_lowercase();
    let payload = match lane_utils::payload_json(&argv[1..], "upwork_gigs_collector_kernel") {
        Ok(v) => v,
        Err(err) => {
            lane_utils::print_json_line(&lane_utils::cli_error("upwork_gigs_collector_kernel_error", &err));
            return 1;
        }
    };
    let payload_obj = lane_utils::payload_obj(&payload);

    match dispatch(root, &command, payload_obj) {
        Ok(out) => {
            lane_utils::print_json_line(&lane_utils::cli_receipt("upwork_gigs_collector_kernel", out));
            0
        }
        Err(err) => {
            lane_utils::print_json_line(&lane_utils::cli_error("upwork_gigs_collector_kernel_error", &err));
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn temp_root() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "upwork_gigs_collector_kernel_test_{}",
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        let _ = fs::create_dir_all(&dir);
        dir
    }

    #[test]
    fn parse_rss_extracts_items() {
        let xml = r#"<rss><channel><item><title>AI gig</title><link>https://x/jobs/1</link><description>Automation</description></item></channel></rss>"#;
        let rows = parse_rss(xml);
        assert_eq!(rows.len(), 1);
    }

    #[test]
    fn map_gigs_sets_signal_for_high_value() {
        let payload = json!({
            "date": "2026-03-27",
            "max_items": 10,
            "seen_ids": [],
            "gigs": [
                {
                    "title": "OpenAI automation workflow",
                    "url": "https://x/jobs/1",
                    "description": "Build GPT automation with API integration"
                }
            ]
        });
        let out = map_gigs(lane_utils::payload_obj(&payload));
        assert_eq!(
            out.get("items")
                .and_then(Value::as_array)
                .and_then(|rows| rows.first())
                .and_then(|row| row.get("signal"))
                .and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn build_fetch_plan_returns_rss_request() {
        let out = command_build_fetch_plan(&Map::new());
        let reqs = out.get("requests").and_then(Value::as_array).cloned().unwrap_or_default();
        assert_eq!(reqs.len(), 1);
        assert_eq!(
            reqs.first()
                .and_then(Value::as_object)
                .and_then(|o| o.get("key"))
                .and_then(Value::as_str),
            Some("rss")
        );
    }

    #[test]
    fn prepare_run_skips_when_recent() {
        let root = temp_root();
        let payload = json!({ "min_hours": 48.0, "force": false });
        let meta_path = meta_path_for(&root, lane_utils::payload_obj(&payload));
        let _ = write_json_atomic(
            &meta_path,
            &json!({
                "collector_id": COLLECTOR_ID,
                "last_run": now_iso(),
                "last_success": now_iso(),
                "seen_ids": []
            }),
        );
        let out = command_prepare_run(&root, lane_utils::payload_obj(&payload));
        assert_eq!(out.get("skipped").and_then(Value::as_bool), Some(true));
    }

    #[test]
    fn collect_returns_skip_payload_when_cadence_not_met() {
        let root = temp_root();
        let payload = json!({ "min_hours": 48.0, "force": false });
        let meta_path = meta_path_for(&root, lane_utils::payload_obj(&payload));
        let _ = write_json_atomic(
            &meta_path,
            &json!({
                "collector_id": COLLECTOR_ID,
                "last_run": now_iso(),
                "last_success": now_iso(),
                "seen_ids": []
            }),
        );

        let out = command_collect(&root, lane_utils::payload_obj(&payload)).expect("collect");
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(out.get("skipped").and_then(Value::as_bool), Some(true));
        assert_eq!(out.get("reason").and_then(Value::as_str), Some("cadence"));
    }
}
