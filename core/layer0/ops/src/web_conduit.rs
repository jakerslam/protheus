// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)
// WEB CONDUIT + SAFETY: fail-closed routed fetch with deterministic receipts.

use chrono::{DateTime, Utc};
use regex::Regex;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use crate::parse_args;

const POLICY_REL: &str = "client/runtime/config/web_conduit_policy.json";
const RECEIPTS_REL: &str = "client/runtime/local/state/web_conduit/receipts.jsonl";
const APPROVALS_REL: &str = "client/runtime/local/state/ui/infring_dashboard/approvals.json";
const ARTIFACTS_DIR_REL: &str = "client/runtime/local/state/web_conduit/artifacts";
const DEFAULT_ACCEPT_LANGUAGE: &str = "en-US,en;q=0.9";
const DEFAULT_REFERER: &str = "https://www.google.com/";
const DEFAULT_WEB_USER_AGENTS: &[&str] = &[
    "Infring-WebConduit/1.0",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 14_5) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Safari/537.36",
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/123.0 Safari/537.36",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:125.0) Gecko/20100101 Firefox/125.0",
];

fn usage() {
    println!("web-conduit commands:");
    println!("  protheus-ops web-conduit status");
    println!("  protheus-ops web-conduit receipts [--limit=<n>]");
    println!("  protheus-ops web-conduit fetch --url=<https://...> [--human-approved=1] [--approval-id=<id>] [--summary-only=1]");
    println!(
        "  protheus-ops web-conduit search --query=<terms> [--human-approved=1] [--summary-only=1]"
    );
    println!("  protheus-ops browse fetch --url=<https://...>");
}

fn clean_text(raw: &str, max_len: usize) -> String {
    raw.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(max_len.max(1))
        .collect::<String>()
}

fn read_json_or(path: &Path, fallback: Value) -> Value {
    match fs::read_to_string(path) {
        Ok(raw) => serde_json::from_str::<Value>(&raw).unwrap_or(fallback),
        Err(_) => fallback,
    }
}

fn write_json_atomic(path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("web_conduit_create_policy_dir_failed:{err}"))?;
    }
    let tmp = path.with_extension(format!(
        "tmp-{}-{}",
        std::process::id(),
        Utc::now().timestamp_millis()
    ));
    let encoded = serde_json::to_vec_pretty(value)
        .map_err(|err| format!("web_conduit_encode_policy_failed:{err}"))?;
    fs::write(&tmp, encoded).map_err(|err| format!("web_conduit_write_policy_tmp_failed:{err}"))?;
    fs::rename(&tmp, path).map_err(|err| format!("web_conduit_rename_policy_failed:{err}"))?;
    Ok(())
}

fn append_jsonl(path: &Path, row: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("web_conduit_create_state_dir_failed:{err}"))?;
    }
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|err| format!("web_conduit_open_receipts_failed:{err}"))?;
    let line = serde_json::to_string(row)
        .map_err(|err| format!("web_conduit_encode_receipt_failed:{err}"))?;
    writeln!(file, "{line}").map_err(|err| format!("web_conduit_append_receipt_failed:{err}"))?;
    Ok(())
}

fn parse_bool(value: Option<&String>) -> bool {
    value
        .map(|raw| {
            matches!(
                raw.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false)
}

fn parse_u64(value: Option<&String>, fallback: u64, min: u64, max: u64) -> u64 {
    value
        .and_then(|raw| raw.trim().parse::<u64>().ok())
        .unwrap_or(fallback)
        .clamp(min, max)
}

fn policy_path(root: &Path) -> PathBuf {
    if let Ok(raw) = std::env::var("INFRING_WEB_CONDUIT_POLICY_PATH") {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    root.join(POLICY_REL)
}

fn receipts_path(root: &Path) -> PathBuf {
    root.join(RECEIPTS_REL)
}

fn approvals_path(root: &Path) -> PathBuf {
    root.join(APPROVALS_REL)
}

fn artifacts_dir_path(root: &Path) -> PathBuf {
    root.join(ARTIFACTS_DIR_REL)
}

fn default_policy() -> Value {
    json!({
        "version": "v1",
        "mode": "production",
        "web_conduit": {
            "enabled": true,
            "max_response_bytes": 350000,
            "timeout_ms": 9000,
            "rate_limit_per_minute": 30,
            "allow_domains": [],
            "deny_domains": [
                "127.0.0.1",
                "localhost",
                "metadata.google.internal",
                "169.254.169.254"
            ],
            "sensitive_domains": [
                "accounts.google.com",
                "api.stripe.com",
                "paypal.com",
                "chase.com",
                "bankofamerica.com"
            ],
            "require_human_for_sensitive": true
        }
    })
}

fn load_policy(root: &Path) -> (Value, PathBuf) {
    let path = policy_path(root);
    if !path.exists() {
        let _ = write_json_atomic(&path, &default_policy());
    }
    (read_json_or(&path, default_policy()), path)
}

fn load_approvals(root: &Path) -> Vec<Value> {
    let path = approvals_path(root);
    let raw = read_json_or(&path, json!({"approvals": []}));
    raw.get("approvals")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn save_approvals(root: &Path, approvals: &[Value]) -> Result<(), String> {
    write_json_atomic(
        &approvals_path(root),
        &json!({
            "type": "infring_dashboard_approvals",
            "updated_at": crate::now_iso(),
            "approvals": approvals
        }),
    )
}

fn approval_state_for_request(
    root: &Path,
    approval_id: &str,
    requested_url: &str,
) -> Option<String> {
    let approval_key = clean_text(approval_id, 160);
    if approval_key.is_empty() {
        return None;
    }
    let url_key = clean_text(requested_url, 2200);
    for row in load_approvals(root) {
        let row_id = clean_text(row.get("id").and_then(Value::as_str).unwrap_or(""), 160);
        if row_id != approval_key {
            continue;
        }
        let row_url = clean_text(
            row.get("requested_url")
                .and_then(Value::as_str)
                .unwrap_or(""),
            2200,
        );
        if !row_url.is_empty() && !url_key.is_empty() && row_url != url_key {
            return Some("mismatched".to_string());
        }
        let state = clean_text(
            row.get("status")
                .and_then(Value::as_str)
                .unwrap_or("pending"),
            40,
        )
        .to_ascii_lowercase();
        return if state.is_empty() {
            Some("pending".to_string())
        } else {
            Some(state)
        };
    }
    None
}

fn ensure_sensitive_web_approval(
    root: &Path,
    requested_url: &str,
    policy_eval: &Value,
) -> Option<Value> {
    let requested = clean_text(requested_url, 2200);
    if requested.is_empty() {
        return None;
    }
    let domain = extract_domain(&requested);
    let approval_id = format!(
        "approval-web-{}",
        &sha256_hex(&format!("{}:{}", domain, requested))[..10]
    );
    let mut approvals = load_approvals(root);
    if let Some(existing) = approvals
        .iter()
        .find(|row| {
            clean_text(row.get("id").and_then(Value::as_str).unwrap_or(""), 160) == approval_id
                && clean_text(
                    row.get("requested_url")
                        .and_then(Value::as_str)
                        .unwrap_or(""),
                    2200,
                ) == requested
                && clean_text(
                    row.get("status")
                        .and_then(Value::as_str)
                        .unwrap_or("pending"),
                    40,
                )
                .to_ascii_lowercase()
                    == "pending"
        })
        .cloned()
    {
        return Some(existing);
    }
    let now = crate::now_iso();
    let row = json!({
        "id": approval_id,
        "action": "Web fetch approval",
        "description": format!("Approve governed web fetch for {}.", requested),
        "agent_name": "web_conduit",
        "status": "pending",
        "domain": domain,
        "requested_url": requested,
        "policy_reason": clean_text(policy_eval.get("reason").and_then(Value::as_str).unwrap_or("human_approval_required_for_sensitive_domain"), 180),
        "created_at": now,
        "updated_at": now
    });
    approvals.push(row.clone());
    let _ = save_approvals(root, &approvals);
    Some(row)
}

fn read_recent_receipts(root: &Path, limit: usize) -> Vec<Value> {
    let raw = fs::read_to_string(receipts_path(root)).unwrap_or_default();
    raw.lines()
        .rev()
        .take(limit.max(1))
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .collect::<Vec<_>>()
}

fn receipt_count(root: &Path) -> usize {
    fs::read_to_string(receipts_path(root))
        .ok()
        .map(|raw| raw.lines().count())
        .unwrap_or(0)
}

fn requests_last_minute(root: &Path) -> u64 {
    let now = Utc::now();
    let mut count = 0u64;
    for row in read_recent_receipts(root, 400) {
        let ts = row
            .get("timestamp")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let Ok(parsed) = DateTime::parse_from_rfc3339(ts) else {
            continue;
        };
        let age = now.signed_duration_since(parsed.with_timezone(&Utc));
        if age.num_seconds() <= 60 {
            count = count.saturating_add(1);
        }
    }
    count
}

fn extract_domain(raw_url: &str) -> String {
    let mut url = clean_text(raw_url, 2200).to_ascii_lowercase();
    if let Some(rest) = url.strip_prefix("http://") {
        url = rest.to_string();
    } else if let Some(rest) = url.strip_prefix("https://") {
        url = rest.to_string();
    }
    let host = url
        .split(['/', '?', '#'])
        .next()
        .unwrap_or_default()
        .split('@')
        .next_back()
        .unwrap_or_default()
        .split(':')
        .next()
        .unwrap_or_default()
        .trim_matches('.');
    clean_text(host, 220).to_ascii_lowercase()
}

fn sha256_hex(raw: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(raw.as_bytes());
    hex::encode(hasher.finalize())
}

fn clip_bytes(raw: &str, max_bytes: usize) -> String {
    if raw.len() <= max_bytes {
        return raw.to_string();
    }
    let mut out = String::new();
    let mut used = 0usize;
    for ch in raw.chars() {
        let width = ch.len_utf8();
        if used + width > max_bytes {
            break;
        }
        out.push(ch);
        used += width;
    }
    out
}

fn regex_script() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"(?is)<script[^>]*>.*?</script>").expect("regex"))
}

fn regex_style() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"(?is)<style[^>]*>.*?</style>").expect("regex"))
}

fn regex_tags() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"(?is)<[^>]+>").expect("regex"))
}

fn clean_html_content(raw: &str, max_chars: usize) -> String {
    let no_script = regex_script().replace_all(raw, " ");
    let no_style = regex_style().replace_all(&no_script, " ");
    let no_tags = regex_tags().replace_all(&no_style, " ");
    let decoded = no_tags
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'");
    clean_text(&decoded, max_chars)
}

fn summarize_text(text: &str, max_chars: usize) -> String {
    let cleaned = clean_text(text, max_chars.max(200));
    if cleaned.is_empty() {
        return String::new();
    }
    let mut sentences = Vec::<String>::new();
    let mut current = String::new();
    for ch in cleaned.chars() {
        current.push(ch);
        if matches!(ch, '.' | '!' | '?') {
            let sentence = clean_text(&current, 280);
            if !sentence.is_empty() {
                sentences.push(sentence);
            }
            current.clear();
            if sentences.len() >= 5 {
                break;
            }
        }
    }
    if sentences.is_empty() {
        return clean_text(&cleaned, 320);
    }
    clean_text(&sentences.join(" "), max_chars)
}

fn persist_artifact(
    root: &Path,
    requested_url: &str,
    response_hash: &str,
    content: &str,
) -> Option<Value> {
    if response_hash.trim().is_empty() || content.trim().is_empty() {
        return None;
    }
    let artifact_id = format!(
        "web-{}",
        response_hash
            .chars()
            .take(16)
            .collect::<String>()
            .to_ascii_lowercase()
    );
    let dir = artifacts_dir_path(root);
    if fs::create_dir_all(&dir).is_err() {
        return None;
    }
    let path = dir.join(format!("{artifact_id}.txt"));
    if !path.exists() {
        if fs::write(&path, content.as_bytes()).is_err() {
            return None;
        }
    }
    Some(json!({
        "artifact_id": artifact_id,
        "path": crate::rel_path(root, &path),
        "bytes": content.len(),
        "source_url": clean_text(requested_url, 2200)
    }))
}

fn encode_query_component(raw: &str) -> String {
    let mut out = String::new();
    for byte in raw.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            out.push(byte as char);
        } else if byte == b' ' {
            out.push('+');
        } else {
            out.push('%');
            out.push_str(&format!("{byte:02X}"));
        }
    }
    out
}

fn web_search_url(query: &str) -> String {
    format!(
        "https://duckduckgo.com/html/?q={}",
        encode_query_component(&clean_text(query, 600))
    )
}

fn web_search_lite_url(query: &str) -> String {
    format!(
        "https://lite.duckduckgo.com/lite/?q={}",
        encode_query_component(&clean_text(query, 600))
    )
}

fn normalize_allowed_domains(raw: &Value) -> Vec<String> {
    let rows = if let Some(array) = raw.as_array() {
        array
            .iter()
            .filter_map(|row| row.as_str().map(|v| v.to_string()))
            .collect::<Vec<_>>()
    } else if let Some(single) = raw.as_str() {
        single
            .split(|ch: char| ch == ',' || ch.is_ascii_whitespace())
            .map(str::trim)
            .filter(|row| !row.is_empty())
            .map(ToString::to_string)
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    rows.into_iter()
        .map(|v| clean_text(v.as_str(), 180).to_ascii_lowercase())
        .map(|row| {
            row.trim()
                .trim_start_matches("http://")
                .trim_start_matches("https://")
                .trim_start_matches("www.")
                .trim_start_matches("*.")
                .split('/')
                .next()
                .unwrap_or("")
                .trim()
                .to_string()
        })
        .filter(|row| {
            !row.is_empty()
                && row.contains('.')
                && row
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-'))
        })
        .fold(Vec::<String>::new(), |mut acc, row| {
            if !acc.iter().any(|existing| existing == &row) {
                acc.push(row);
            }
            acc
        })
}

fn scoped_search_query(query: &str, allowed_domains: &[String], exclude_subdomains: bool) -> String {
    let cleaned = clean_text(query, 600);
    if cleaned.is_empty() || allowed_domains.is_empty() {
        return cleaned;
    }
    let scope = allowed_domains
        .iter()
        .map(|domain| {
            if exclude_subdomains {
                format!("(site:{domain} -site:*.{domain})")
            } else {
                format!("site:{domain}")
            }
        })
        .collect::<Vec<_>>()
        .join(" OR ");
    clean_text(format!("({scope}) {cleaned}").as_str(), 900)
}

fn looks_like_search_challenge_payload(summary: &str, content: &str) -> bool {
    let combined = format!("{summary}\n{content}").to_ascii_lowercase();
    if combined.is_empty() {
        return false;
    }
    [
        "unfortunately, bots use duckduckgo too",
        "please complete the following challenge",
        "select all squares containing a duck",
        "anomaly-modal",
        "images not loading?",
        "error-lite@duckduckgo.com",
    ]
    .iter()
    .any(|marker| combined.contains(marker))
}

fn fetch_with_curl(
    url: &str,
    timeout_ms: u64,
    max_response_bytes: usize,
    user_agent: &str,
) -> Value {
    let timeout_sec = ((timeout_ms as f64) / 1000.0).ceil() as u64;
    let output = Command::new("curl")
        .arg("-sS")
        .arg("-L")
        .arg("--compressed")
        .arg("--proto")
        .arg("=http,https")
        .arg("--connect-timeout")
        .arg(timeout_sec.max(1).to_string())
        .arg("--max-time")
        .arg(timeout_sec.max(1).to_string())
        .arg("-A")
        .arg(clean_text(user_agent, 260))
        .arg("-H")
        .arg(format!("Accept-Language: {DEFAULT_ACCEPT_LANGUAGE}"))
        .arg("-e")
        .arg(DEFAULT_REFERER)
        .arg("-w")
        .arg("\n__STATUS__:%{http_code}\n__CTYPE__:%{content_type}")
        .arg(url)
        .output();

    match output {
        Ok(run) => {
            let stdout = String::from_utf8_lossy(&run.stdout).to_string();
            let stderr = clean_text(&String::from_utf8_lossy(&run.stderr), 320);
            let status_marker = "\n__STATUS__:";
            let ctype_marker = "\n__CTYPE__:";
            let (body_and_status, content_type) = match stdout.rsplit_once(ctype_marker) {
                Some((left, right)) => (left.to_string(), clean_text(right, 120)),
                None => (stdout, String::new()),
            };
            let (body_raw, status_raw) = match body_and_status.rsplit_once(status_marker) {
                Some((left, right)) => (left.to_string(), clean_text(right, 12)),
                None => (body_and_status, "0".to_string()),
            };
            let status_code = status_raw.parse::<i64>().unwrap_or(0);
            let body = clip_bytes(&body_raw, max_response_bytes.max(256));
            let status_ok = (200..400).contains(&status_code);
            json!({
                "ok": run.status.success() && status_ok,
                "status_code": status_code,
                "content_type": content_type,
                "body": body,
                "stderr": if stderr.is_empty() { Value::Null } else { Value::String(stderr) },
                "user_agent": clean_text(user_agent, 260)
            })
        }
        Err(err) => json!({
            "ok": false,
            "status_code": 0,
            "content_type": "",
            "body": "",
            "stderr": format!("curl_spawn_failed:{err}"),
            "user_agent": clean_text(user_agent, 260)
        }),
    }
}

fn is_retryable_fetch_result(row: &Value) -> bool {
    let status = row.get("status_code").and_then(Value::as_i64).unwrap_or(0);
    if matches!(status, 408 | 425 | 429 | 500 | 502 | 503 | 504) {
        return true;
    }
    let error = clean_text(row.get("stderr").and_then(Value::as_str).unwrap_or(""), 220).to_ascii_lowercase();
    error.contains("timed out")
        || error.contains("timeout")
        || error.contains("econnreset")
        || error.contains("temporarily unavailable")
        || error.contains("could not resolve host")
        || error.contains("empty reply")
}

fn content_type_is_textual(content_type: &str) -> bool {
    let lowered = clean_text(content_type, 120).to_ascii_lowercase();
    if lowered.is_empty() {
        return true;
    }
    lowered.starts_with("text/")
        || lowered.contains("json")
        || lowered.contains("xml")
        || lowered.contains("javascript")
        || lowered.contains("yaml")
        || lowered.contains("csv")
}

fn fetch_with_curl_retry(
    url: &str,
    timeout_ms: u64,
    max_response_bytes: usize,
    max_attempts: usize,
) -> Value {
    let mut attempts = 0usize;
    let mut best = json!({
        "ok": false,
        "status_code": 0,
        "content_type": "",
        "body": "",
        "stderr": "fetch_not_attempted"
    });
    let target_attempts = max_attempts.clamp(1, 4);
    for idx in 0..target_attempts {
        attempts += 1;
        let ua = DEFAULT_WEB_USER_AGENTS
            .get(idx % DEFAULT_WEB_USER_AGENTS.len())
            .copied()
            .unwrap_or(DEFAULT_WEB_USER_AGENTS[0]);
        let current = fetch_with_curl(url, timeout_ms, max_response_bytes, ua);
        let current_ok = current.get("ok").and_then(Value::as_bool).unwrap_or(false);
        best = current;
        if current_ok {
            break;
        }
        if !is_retryable_fetch_result(&best) || idx + 1 >= target_attempts {
            break;
        }
        let sleep_ms = match idx {
            0 => 180_u64,
            1 => 360_u64,
            _ => 720_u64,
        };
        std::thread::sleep(std::time::Duration::from_millis(sleep_ms));
    }
    if let Some(obj) = best.as_object_mut() {
        obj.insert("retry_attempts".to_string(), json!(attempts));
        obj.insert("retry_used".to_string(), json!(attempts > 1));
    }
    best
}

fn build_receipt(
    requested_url: &str,
    policy_decision: &str,
    response_hash: Option<&str>,
    status_code: i64,
    policy_reason: &str,
    error: Option<&str>,
) -> Value {
    let timestamp = crate::now_iso();
    let mut row = json!({
        "type": "web_conduit_receipt",
        "timestamp": timestamp,
        "requested_url": clean_text(requested_url, 2200),
        "domain": extract_domain(requested_url),
        "policy_decision": clean_text(policy_decision, 40),
        "policy_reason": clean_text(policy_reason, 160),
        "status_code": status_code,
        "response_hash": response_hash.unwrap_or(""),
        "error": clean_text(error.unwrap_or(""), 320)
    });
    row["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&row));
    row
}

pub fn api_status(root: &Path) -> Value {
    let (policy, policy_path_value) = load_policy(root);
    let recent = read_recent_receipts(root, 12);
    let denied = recent
        .iter()
        .filter(|row| row.get("policy_decision").and_then(Value::as_str) == Some("deny"))
        .count();
    let last = recent.first().cloned().unwrap_or(Value::Null);
    json!({
        "ok": true,
        "enabled": policy.pointer("/web_conduit/enabled").and_then(Value::as_bool).unwrap_or(true),
        "policy_path": policy_path_value.to_string_lossy().to_string(),
        "policy": policy,
        "receipts_total": receipt_count(root),
        "recent_denied": denied,
        "recent_receipts": recent,
        "last_receipt": last
    })
}

pub fn api_receipts(root: &Path, limit: usize) -> Value {
    json!({
        "ok": true,
        "receipts": read_recent_receipts(root, limit.clamp(1, 200))
    })
}

pub fn api_fetch(root: &Path, request: &Value) -> Value {
    let requested_url = clean_text(
        request
            .get("requested_url")
            .or_else(|| request.get("url"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        2200,
    );
    let summary_only = request
        .get("summary_only")
        .or_else(|| request.get("summary"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let human_approved = request
        .get("human_approved")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let approval_id = clean_text(
        request
            .get("approval_id")
            .and_then(Value::as_str)
            .unwrap_or(""),
        160,
    );
    let approval_state = approval_state_for_request(root, &approval_id, &requested_url);
    let token_approved = approval_state.as_deref() == Some("approved");
    let effective_human_approved = human_approved || token_approved;
    let (policy, _policy_path_value) = load_policy(root);
    let policy_eval = infring_layer1_security::evaluate_web_conduit_policy(
        root,
        &json!({
            "requested_url": requested_url,
            "domain": extract_domain(&requested_url),
            "human_approved": effective_human_approved,
            "requests_last_minute": requests_last_minute(root)
        }),
        &policy,
    );
    let allow = policy_eval
        .get("allow")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let decision = clean_text(
        policy_eval
            .get("decision")
            .and_then(Value::as_str)
            .unwrap_or("deny"),
        20,
    );
    let reason = clean_text(
        policy_eval
            .get("reason")
            .and_then(Value::as_str)
            .unwrap_or("policy_denied"),
        180,
    );
    if !allow {
        let approval = if reason == "human_approval_required_for_sensitive_domain" {
            ensure_sensitive_web_approval(root, &requested_url, &policy_eval)
        } else {
            None
        };
        let receipt = build_receipt(
            &requested_url,
            "deny",
            None,
            0,
            &reason,
            Some(if approval.is_some() {
                "approval_required"
            } else {
                "policy_denied"
            }),
        );
        let _ = append_jsonl(&receipts_path(root), &receipt);
        return json!({
            "ok": false,
            "error": "web_conduit_policy_denied",
            "requested_url": requested_url,
            "policy_decision": policy_eval,
            "receipt": receipt,
            "approval_required": approval.is_some(),
            "approval": approval,
            "approval_state": approval_state,
            "retry_with": if reason == "human_approval_required_for_sensitive_domain" {
                json!({
                    "url": requested_url,
                    "approval_id": approval
                        .as_ref()
                        .and_then(|row| row.get("id"))
                        .and_then(Value::as_str)
                        .unwrap_or(approval_id.as_str()),
                    "summary_only": summary_only
                })
            } else {
                Value::Null
            }
        });
    }

    let timeout_ms = policy_eval
        .pointer("/policy/timeout_ms")
        .and_then(Value::as_u64)
        .unwrap_or(9000);
    let max_response_bytes = policy_eval
        .pointer("/policy/max_response_bytes")
        .and_then(Value::as_u64)
        .unwrap_or(350_000) as usize;
    let retry_attempts = policy_eval
        .pointer("/policy/retry_attempts")
        .and_then(Value::as_u64)
        .unwrap_or(2)
        .clamp(1, 4) as usize;
    let fetched = fetch_with_curl_retry(&requested_url, timeout_ms, max_response_bytes, retry_attempts);
    let status_code = fetched
        .get("status_code")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let content_type = clean_text(
        fetched.get("content_type").and_then(Value::as_str).unwrap_or(""),
        180,
    );
    let fetched_body = fetched.get("body").and_then(Value::as_str).unwrap_or("");
    let content_is_textual = content_type_is_textual(&content_type);
    let content = if content_is_textual {
        clean_html_content(fetched_body, max_response_bytes.min(240_000))
    } else {
        String::new()
    };
    let summary = if content_is_textual {
        summarize_text(&content, 900)
    } else if requested_url.is_empty() {
        format!("Fetched non-text content ({}).", if content_type.is_empty() { "binary/unknown" } else { content_type.as_str() })
    } else {
        format!(
            "Fetched non-text content from {} ({}).",
            clean_text(&requested_url, 220),
            if content_type.is_empty() {
                "binary/unknown"
            } else {
                content_type.as_str()
            }
        )
    };
    let response_hash = if content.is_empty() {
        String::new()
    } else {
        sha256_hex(&content)
    };
    let materialize_artifact = request
        .get("materialize_artifact")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let artifact = if materialize_artifact {
        persist_artifact(root, &requested_url, &response_hash, &content)
    } else {
        None
    };
    let fetch_ok = fetched.get("ok").and_then(Value::as_bool).unwrap_or(false)
        && if content_is_textual {
            !content.is_empty()
        } else {
            status_code >= 200 && status_code < 400
        };
    let error_value = fetched
        .get("stderr")
        .and_then(Value::as_str)
        .map(|v| clean_text(v, 320))
        .unwrap_or_default();
    let receipt = build_receipt(
        &requested_url,
        &decision,
        if response_hash.is_empty() {
            None
        } else {
            Some(response_hash.as_str())
        },
        status_code,
        &reason,
        if error_value.is_empty() {
            None
        } else {
            Some(error_value.as_str())
        },
    );
    let _ = append_jsonl(&receipts_path(root), &receipt);

    json!({
        "ok": fetch_ok,
        "requested_url": requested_url,
        "status_code": status_code,
        "content_type": if content_type.is_empty() { Value::String(String::new()) } else { Value::String(content_type) },
        "summary": summary,
        "content": if summary_only { Value::String(String::new()) } else { Value::String(content.clone()) },
        "retry_attempts": fetched.get("retry_attempts").cloned().unwrap_or_else(|| json!(1)),
        "retry_used": fetched.get("retry_used").cloned().unwrap_or_else(|| json!(false)),
        "user_agent": fetched.get("user_agent").cloned().unwrap_or_else(|| json!(DEFAULT_WEB_USER_AGENTS[0])),
        "response_hash": response_hash,
        "artifact": artifact.clone().unwrap_or(Value::Null),
        "policy_decision": policy_eval,
        "receipt": receipt,
        "epistemic_object": {
            "kind": "web_document",
            "trusted": false,
            "provenance": {
                "source": "web_conduit",
                "requested_url": requested_url,
                "response_hash": response_hash,
                "artifact_id": artifact
                    .as_ref()
                    .and_then(|row| row.get("artifact_id"))
                    .cloned()
                    .unwrap_or(Value::Null),
                "artifact_path": artifact
                    .as_ref()
                    .and_then(|row| row.get("path"))
                    .cloned()
                    .unwrap_or(Value::Null),
                "receipt_hash": receipt.get("receipt_hash").cloned().unwrap_or(Value::Null)
            },
            "verity": {
                "validated": false,
                "checks": [
                    "policy_gate_passed",
                    "content_hash_recorded",
                    "source_marked_untrusted_until_verified"
                ]
            }
        },
        "error": if fetch_ok {
            Value::Null
        } else if error_value.is_empty() {
            json!("web_conduit_fetch_failed")
        } else {
            json!(error_value)
        }
    })
}

pub fn api_search(root: &Path, request: &Value) -> Value {
    let query = clean_text(
        request
            .get("query")
            .or_else(|| request.get("q"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        600,
    );
    if query.is_empty() {
        let receipt = build_receipt(
            "",
            "deny",
            None,
            0,
            "query_required",
            Some("query_required"),
        );
        let _ = append_jsonl(&receipts_path(root), &receipt);
        return json!({
            "ok": false,
            "error": "query_required",
            "query": "",
            "receipt": receipt
        });
    }
    let allowed_domains = normalize_allowed_domains(
        request.get("allowed_domains").unwrap_or(&Value::Null),
    );
    let exclude_subdomains = request
        .get("exclude_subdomains")
        .or_else(|| request.get("exact_domain_only"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let scoped_query = scoped_search_query(&query, &allowed_domains, exclude_subdomains);
    let primary_url = web_search_url(&scoped_query);
    let fallback_url = web_search_lite_url(&scoped_query);
    let summary_only = request
        .get("summary_only")
        .or_else(|| request.get("summary"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let mut out = api_fetch(
        root,
        &json!({
            "url": primary_url,
            "summary_only": summary_only,
            "human_approved": request
                .get("human_approved")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            "approval_id": request
                .get("approval_id")
                .and_then(Value::as_str)
                .unwrap_or("")
        }),
    );
    let mut used_lite_fallback = false;
    if out.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        let summary = clean_text(out.get("summary").and_then(Value::as_str).unwrap_or(""), 2400);
        let content = clean_text(out.get("content").and_then(Value::as_str).unwrap_or(""), 4000);
        if looks_like_search_challenge_payload(&summary, &content) {
            let lite_out = api_fetch(
                root,
                &json!({
                    "url": fallback_url,
                    "summary_only": summary_only,
                    "human_approved": request
                        .get("human_approved")
                        .and_then(Value::as_bool)
                        .unwrap_or(false),
                    "approval_id": request
                        .get("approval_id")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                }),
            );
            if lite_out.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                out = lite_out;
                used_lite_fallback = true;
            } else if let Some(obj) = out.as_object_mut() {
                obj.insert(
                    "search_lite_fallback_error".to_string(),
                    clean_text(
                        lite_out.get("error").and_then(Value::as_str).unwrap_or(""),
                        220,
                    )
                    .into(),
                );
            }
        }
    }
    if let Some(obj) = out.as_object_mut() {
        obj.insert(
            "type".to_string(),
            Value::String("web_conduit_search".to_string()),
        );
        obj.insert("query".to_string(), Value::String(query.clone()));
        obj.insert("effective_query".to_string(), Value::String(scoped_query));
        obj.insert("allowed_domains".to_string(), json!(allowed_domains));
        obj.insert("exclude_subdomains".to_string(), json!(exclude_subdomains));
        obj.insert(
            "provider".to_string(),
            Value::String(if used_lite_fallback {
                "duckduckgo_lite".to_string()
            } else {
                "duckduckgo".to_string()
            }),
        );
        obj.insert("search_lite_fallback".to_string(), json!(used_lite_fallback));
    }
    out
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let parsed = parse_args(argv);
    let command = parsed
        .positional
        .first()
        .map(|row| row.to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());
    let payload = match command.as_str() {
        "help" => {
            usage();
            json!({"ok": true, "type": "web_conduit_help"})
        }
        "status" => api_status(root),
        "receipts" => {
            let limit = parse_u64(parsed.flags.get("limit"), 20, 1, 200) as usize;
            api_receipts(root, limit)
        }
        "fetch" | "browse" => {
            let url = clean_text(
                parsed
                    .flags
                    .get("url")
                    .map(String::as_str)
                    .unwrap_or_else(|| parsed.positional.get(1).map(String::as_str).unwrap_or("")),
                2200,
            );
            api_fetch(
                root,
                &json!({
                    "url": url,
                    "human_approved": parse_bool(parsed.flags.get("human-approved")) || parse_bool(parsed.flags.get("human_approved")),
                    "approval_id": clean_text(
                        parsed
                            .flags
                            .get("approval-id")
                            .or_else(|| parsed.flags.get("approval_id"))
                            .map(String::as_str)
                            .unwrap_or(""),
                        160
                    ),
                    "summary_only": parse_bool(parsed.flags.get("summary-only")) || parse_bool(parsed.flags.get("summary_only"))
                }),
            )
        }
        "search" => {
            let query = clean_text(
                parsed
                    .flags
                    .get("query")
                    .or_else(|| parsed.flags.get("q"))
                    .map(String::as_str)
                    .unwrap_or_else(|| parsed.positional.get(1).map(String::as_str).unwrap_or("")),
                600,
            );
            let allowed_domains = parsed
                .flags
                .get("allowed-domains")
                .or_else(|| parsed.flags.get("allowed_domains"))
                .cloned()
                .unwrap_or_default();
            api_search(
                root,
                &json!({
                    "query": query,
                    "allowed_domains": normalize_allowed_domains(&json!(allowed_domains)),
                    "exclude_subdomains": parse_bool(parsed.flags.get("exclude-subdomains")) || parse_bool(parsed.flags.get("exclude_subdomains")) || parse_bool(parsed.flags.get("exact-domain-only")) || parse_bool(parsed.flags.get("exact_domain_only")),
                    "human_approved": parse_bool(parsed.flags.get("human-approved")) || parse_bool(parsed.flags.get("human_approved")),
                    "approval_id": clean_text(
                        parsed
                            .flags
                            .get("approval-id")
                            .or_else(|| parsed.flags.get("approval_id"))
                            .map(String::as_str)
                            .unwrap_or(""),
                        160
                    ),
                    "summary_only": parse_bool(parsed.flags.get("summary-only")) || parse_bool(parsed.flags.get("summary_only"))
                }),
            )
        }
        _ => json!({
            "ok": false,
            "error": "web_conduit_unknown_command",
            "command": command
        }),
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&payload)
            .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode_failed\"}".to_string())
    );
    if payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        0
    } else {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_bootstraps_default_policy_and_receipts_surface() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let out = api_status(tmp.path());
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert!(out.get("policy").is_some());
    }

    #[test]
    fn sensitive_domain_requires_explicit_human_approval() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let out = api_fetch(
            tmp.path(),
            &json!({"url": "https://accounts.google.com/login", "human_approved": false}),
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(
            out.pointer("/policy_decision/reason")
                .and_then(Value::as_str),
            Some("human_approval_required_for_sensitive_domain")
        );
        assert_eq!(
            out.get("approval_required").and_then(Value::as_bool),
            Some(true)
        );
        assert!(out.pointer("/approval/id").is_some());
    }

    #[test]
    fn approved_token_allows_sensitive_domain_policy_gate() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let first = api_fetch(
            tmp.path(),
            &json!({"url": "https://accounts.google.com/login", "human_approved": false}),
        );
        let approval_id = first
            .pointer("/approval/id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        assert!(!approval_id.is_empty());

        let mut approvals = load_approvals(tmp.path());
        if let Some(row) = approvals.iter_mut().find(|row| {
            clean_text(row.get("id").and_then(Value::as_str).unwrap_or(""), 160) == approval_id
        }) {
            row["status"] = json!("approved");
            row["updated_at"] = json!(crate::now_iso());
        }
        save_approvals(tmp.path(), &approvals).expect("save approvals");

        let second = api_fetch(
            tmp.path(),
            &json!({
                "url": "https://accounts.google.com/login",
                "approval_id": approval_id,
                "summary_only": true
            }),
        );
        assert_eq!(
            second
                .pointer("/policy_decision/allow")
                .and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn fetch_example_com_and_summarize_smoke() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let out = api_fetch(
            tmp.path(),
            &json!({"url": "https://example.com", "summary_only": true}),
        );
        assert!(out.get("receipt").is_some());
        if out.get("ok").and_then(Value::as_bool).unwrap_or(false) {
            assert!(out
                .get("summary")
                .and_then(Value::as_str)
                .map(|v| !v.trim().is_empty())
                .unwrap_or(false));
        } else {
            assert!(out.get("error").is_some());
        }
    }

    #[test]
    fn search_requires_query() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let out = api_search(tmp.path(), &json!({"query": ""}));
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(
            out.get("error").and_then(Value::as_str),
            Some("query_required")
        );
        assert!(out.get("receipt").is_some());
    }

    #[test]
    fn search_smoke_records_receipt() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let out = api_search(
            tmp.path(),
            &json!({"query": "example domain", "summary_only": true}),
        );
        assert!(out.get("receipt").is_some());
        assert_eq!(
            out.get("type").and_then(Value::as_str),
            Some("web_conduit_search")
        );
        assert_eq!(
            out.get("provider").and_then(Value::as_str),
            Some("duckduckgo")
        );
    }

    #[test]
    fn challenge_detector_flags_anomaly_copy() {
        assert!(looks_like_search_challenge_payload(
            "Unfortunately, bots use DuckDuckGo too.",
            "Please complete the following challenge and select all squares containing a duck."
        ));
    }

    #[test]
    fn challenge_detector_ignores_normal_results() {
        assert!(!looks_like_search_challenge_payload(
            "Tech News | Today's Latest Technology News | Reuters",
            "www.reuters.com/technology/ Find latest technology news from every corner of the globe."
        ));
    }

    #[test]
    fn scoped_search_query_applies_domain_filters() {
        let scoped = scoped_search_query(
            "agent reliability",
            &vec!["github.com".to_string(), "docs.rs".to_string()],
            false,
        );
        assert!(scoped.contains("site:github.com"));
        assert!(scoped.contains("site:docs.rs"));
        assert!(scoped.contains("agent reliability"));
    }

    #[test]
    fn scoped_search_query_leaves_plain_query_when_domains_empty() {
        let scoped = scoped_search_query("agent reliability", &[], false);
        assert_eq!(scoped, "agent reliability");
    }

    #[test]
    fn normalize_allowed_domains_sanitizes_urls_and_duplicates() {
        let domains = normalize_allowed_domains(&json!([
            "https://www.github.com/openai",
            "docs.rs",
            "github.com",
            "not a domain"
        ]));
        assert_eq!(domains, vec!["github.com".to_string(), "docs.rs".to_string()]);
    }

    #[test]
    fn scoped_search_query_supports_exact_domain_mode() {
        let scoped = scoped_search_query("agent reliability", &vec!["example.com".to_string()], true);
        assert!(scoped.contains("site:example.com"));
        assert!(scoped.contains("-site:*.example.com"));
    }

    #[test]
    fn normalize_allowed_domains_supports_comma_string() {
        let domains = normalize_allowed_domains(&json!("https://www.github.com, docs.rs *.example.com"));
        assert_eq!(
            domains,
            vec![
                "github.com".to_string(),
                "docs.rs".to_string(),
                "example.com".to_string()
            ]
        );
    }
}
