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
use crate::web_conduit_provider_runtime::{
    load_search_cache, provider_chain_from_request, provider_circuit_open_until,
    provider_health_snapshot, record_provider_attempt, search_cache_key, store_search_cache,
};

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
const SERPER_SEARCH_URL: &str = "https://google.serper.dev/search";

fn usage() {
    println!("web-conduit commands:");
    println!("  protheus-ops web-conduit status");
    println!("  protheus-ops web-conduit receipts [--limit=<n>]");
    println!("  protheus-ops web-conduit fetch --url=<https://...> [--human-approved=1] [--approval-id=<id>] [--summary-only=1]");
    println!(
        "  protheus-ops web-conduit search --query=<terms> [--provider=auto|serper|duckduckgo|bing] [--top-k=8] [--allowed-domains=docs.rs,github.com] [--exact-domain-only=1] [--human-approved=1] [--summary-only=1]"
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

fn serper_api_key() -> Option<String> {
    for key in [
        "INFRING_SERPERDEV_API_KEY",
        "SERPERDEV_API_KEY",
        "INFRING_SERPER_API_KEY",
        "SERPER_API_KEY",
    ] {
        if let Ok(raw) = std::env::var(key) {
            let cleaned = clean_text(&raw, 600);
            if !cleaned.is_empty() {
                return Some(cleaned);
            }
        }
    }
    None
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
            "require_human_for_sensitive": true,
            "search_provider_order": ["serperdev", "duckduckgo", "duckduckgo_lite", "bing_rss"],
            "provider_circuit_breaker": {
                "enabled": true,
                "failure_threshold": 3,
                "open_for_secs": 300
            }
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

fn web_search_bing_rss_url(query: &str) -> String {
    format!(
        "https://www.bing.com/search?q={}&format=rss&setlang=en-US",
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

fn scoped_search_query(
    query: &str,
    allowed_domains: &[String],
    exclude_subdomains: bool,
) -> String {
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

fn domain_matches_filter(domain: &str, filter: &str, exclude_subdomains: bool) -> bool {
    if domain == filter {
        return true;
    }
    if exclude_subdomains {
        return false;
    }
    domain
        .strip_suffix(filter)
        .map(|prefix| prefix.ends_with('.'))
        .unwrap_or(false)
}

fn domain_allowed_for_scope(
    raw_url: &str,
    allowed_domains: &[String],
    exclude_subdomains: bool,
) -> bool {
    if allowed_domains.is_empty() {
        return true;
    }
    let domain = extract_domain(raw_url);
    if domain.is_empty() {
        return false;
    }
    allowed_domains
        .iter()
        .any(|filter| domain_matches_filter(&domain, filter, exclude_subdomains))
}

fn render_serper_payload(
    body: &str,
    allowed_domains: &[String],
    exclude_subdomains: bool,
    top_k: usize,
    max_response_bytes: usize,
) -> Value {
    let parsed = match serde_json::from_str::<Value>(body) {
        Ok(value) => value,
        Err(_) => {
            return json!({
                "ok": false,
                "error": "serper_decode_failed",
                "summary": "",
                "content": "",
                "links": [],
                "content_domains": [],
                "provider_raw_count": 0,
                "provider_filtered_count": 0
            });
        }
    };
    let organic = parsed
        .get("organic")
        .or_else(|| parsed.get("results"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut lines = Vec::<String>::new();
    let mut links = Vec::<String>::new();
    let mut domains = Vec::<String>::new();
    for row in &organic {
        let link = clean_text(row.get("link").and_then(Value::as_str).unwrap_or(""), 2200);
        if link.is_empty() || !domain_allowed_for_scope(&link, allowed_domains, exclude_subdomains)
        {
            continue;
        }
        let title = clean_text(row.get("title").and_then(Value::as_str).unwrap_or(""), 220);
        let snippet = clean_text(
            row.get("snippet").and_then(Value::as_str).unwrap_or(""),
            420,
        );
        let rendered = if title.is_empty() && snippet.is_empty() {
            clean_text(&link, 1200)
        } else if snippet.is_empty() {
            clean_text(format!("{title} — {link}").as_str(), 1200)
        } else if title.is_empty() {
            clean_text(format!("{link} — {snippet}").as_str(), 1200)
        } else {
            clean_text(format!("{title} — {link} — {snippet}").as_str(), 1200)
        };
        if rendered.is_empty() {
            continue;
        }
        lines.push(rendered);
        links.push(link.clone());
        let domain = extract_domain(&link);
        if !domain.is_empty() && !domains.iter().any(|existing| existing == &domain) {
            domains.push(domain);
        }
        if lines.len() >= top_k.max(1) {
            break;
        }
    }
    let content = clean_text(&lines.join("\n"), max_response_bytes.min(120_000));
    let ok = !content.is_empty();
    json!({
        "ok": ok,
        "summary": if ok {
            summarize_text(&content, 900)
        } else {
            crate::tool_output_match_filter::no_findings_user_copy().to_string()
        },
        "content": content,
        "links": links,
        "content_domains": domains,
        "provider_raw_count": organic.len(),
        "provider_filtered_count": lines.len(),
        "error": if ok {
            Value::Null
        } else {
            Value::String("no_relevant_results".to_string())
        }
    })
}

fn decode_xml_entities(raw: &str) -> String {
    raw.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
}

fn extract_xml_tag_value(block: &str, tag: &str) -> String {
    let pattern = format!(r"(?is)<{tag}[^>]*>(.*?)</{tag}>");
    let Ok(re) = Regex::new(&pattern) else {
        return String::new();
    };
    let Some(captures) = re.captures(block) else {
        return String::new();
    };
    let raw = captures.get(1).map(|m| m.as_str()).unwrap_or("");
    let trimmed = raw
        .trim()
        .trim_start_matches("<![CDATA[")
        .trim_end_matches("]]>");
    clean_html_content(&decode_xml_entities(trimmed), 2_400)
}

fn render_bing_rss_payload(
    body: &str,
    allowed_domains: &[String],
    exclude_subdomains: bool,
    top_k: usize,
    max_response_bytes: usize,
) -> Value {
    static ITEM_RE: OnceLock<Regex> = OnceLock::new();
    let item_re =
        ITEM_RE.get_or_init(|| Regex::new(r"(?is)<item\b[^>]*>(.*?)</item>").expect("item regex"));
    let mut lines = Vec::<String>::new();
    let mut links = Vec::<String>::new();
    let mut domains = Vec::<String>::new();
    let mut raw_count = 0usize;
    for captures in item_re.captures_iter(body) {
        raw_count += 1;
        let item = captures.get(1).map(|m| m.as_str()).unwrap_or("");
        let link = clean_text(&extract_xml_tag_value(item, "link"), 2_200);
        if link.is_empty() || !domain_allowed_for_scope(&link, allowed_domains, exclude_subdomains)
        {
            continue;
        }
        let title = clean_text(&extract_xml_tag_value(item, "title"), 220);
        let snippet = clean_text(&extract_xml_tag_value(item, "description"), 420);
        let rendered = if title.is_empty() && snippet.is_empty() {
            clean_text(&link, 1_200)
        } else if snippet.is_empty() {
            clean_text(format!("{title} — {link}").as_str(), 1_200)
        } else if title.is_empty() {
            clean_text(format!("{link} — {snippet}").as_str(), 1_200)
        } else {
            clean_text(format!("{title} — {link} — {snippet}").as_str(), 1_200)
        };
        if rendered.is_empty() {
            continue;
        }
        lines.push(rendered);
        links.push(link.clone());
        let domain = extract_domain(&link);
        if !domain.is_empty() && !domains.iter().any(|existing| existing == &domain) {
            domains.push(domain);
        }
        if lines.len() >= top_k.max(1) {
            break;
        }
    }
    let content = clean_text(&lines.join("\n"), max_response_bytes.min(120_000));
    let ok = !content.is_empty();
    json!({
        "ok": ok,
        "summary": if ok {
            summarize_text(&content, 900)
        } else {
            crate::tool_output_match_filter::no_findings_user_copy().to_string()
        },
        "content": content,
        "links": links,
        "content_domains": domains,
        "provider_raw_count": raw_count,
        "provider_filtered_count": lines.len(),
        "error": if ok {
            Value::Null
        } else {
            Value::String("no_relevant_results".to_string())
        }
    })
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

fn payload_looks_like_search_challenge(payload: &Value) -> bool {
    let summary = clean_text(
        payload.get("summary").and_then(Value::as_str).unwrap_or(""),
        2_400,
    );
    let content = clean_text(
        payload.get("content").and_then(Value::as_str).unwrap_or(""),
        4_000,
    );
    looks_like_search_challenge_payload(&summary, &content)
}

fn looks_like_low_signal_search_payload(summary: &str, content: &str) -> bool {
    let lowered = clean_text(&format!("{summary}\n{content}"), 6_000).to_ascii_lowercase();
    if lowered.is_empty() {
        return true;
    }
    if looks_like_search_challenge_payload(summary, content) {
        return true;
    }
    if lowered.contains("key findings for") && lowered.contains("potential sources:") {
        return true;
    }
    let marker_hits = [
        "duckduckgo all regions",
        "all regions argentina",
        "all regions australia",
        "all regions canada",
        "safe search",
        "any time",
        "at duckduckgo",
    ]
    .iter()
    .filter(|marker| lowered.contains(**marker))
    .count();
    marker_hits >= 2
}

fn payload_looks_low_signal_search(payload: &Value) -> bool {
    let summary = clean_text(
        payload.get("summary").and_then(Value::as_str).unwrap_or(""),
        2_400,
    );
    let content = clean_text(
        payload.get("content").and_then(Value::as_str).unwrap_or(""),
        4_000,
    );
    looks_like_low_signal_search_payload(&summary, &content)
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

fn fetch_serper_with_curl(
    api_key: &str,
    query: &str,
    timeout_ms: u64,
    max_response_bytes: usize,
    user_agent: &str,
    top_k: usize,
) -> Value {
    let timeout_sec = ((timeout_ms as f64) / 1000.0).ceil() as u64;
    let payload = json!({
        "q": clean_text(query, 900),
        "num": top_k.clamp(1, 12)
    });
    let payload_raw = serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string());
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
        .arg("Accept: application/json")
        .arg("-H")
        .arg("Content-Type: application/json")
        .arg("-H")
        .arg(format!("X-API-KEY: {}", clean_text(api_key, 600)))
        .arg("-d")
        .arg(payload_raw)
        .arg("-w")
        .arg("\n__STATUS__:%{http_code}\n__CTYPE__:%{content_type}")
        .arg(SERPER_SEARCH_URL)
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
            let status_ok = (200..300).contains(&status_code);
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
            "stderr": format!("serper_curl_spawn_failed:{err}"),
            "user_agent": clean_text(user_agent, 260)
        }),
    }
}

fn is_retryable_fetch_result(row: &Value) -> bool {
    let status = row.get("status_code").and_then(Value::as_i64).unwrap_or(0);
    if matches!(status, 408 | 425 | 429 | 500 | 502 | 503 | 504) {
        return true;
    }
    let error = clean_text(row.get("stderr").and_then(Value::as_str).unwrap_or(""), 220)
        .to_ascii_lowercase();
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

fn fetch_serper_with_retry(
    api_key: &str,
    query: &str,
    timeout_ms: u64,
    max_response_bytes: usize,
    max_attempts: usize,
    top_k: usize,
) -> Value {
    let mut attempts = 0usize;
    let mut best = json!({
        "ok": false,
        "status_code": 0,
        "content_type": "",
        "body": "",
        "stderr": "serper_not_attempted"
    });
    let target_attempts = max_attempts.clamp(1, 4);
    for idx in 0..target_attempts {
        attempts += 1;
        let ua = DEFAULT_WEB_USER_AGENTS
            .get(idx % DEFAULT_WEB_USER_AGENTS.len())
            .copied()
            .unwrap_or(DEFAULT_WEB_USER_AGENTS[0]);
        let current =
            fetch_serper_with_curl(api_key, query, timeout_ms, max_response_bytes, ua, top_k);
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
    let fetched = fetch_with_curl_retry(
        &requested_url,
        timeout_ms,
        max_response_bytes,
        retry_attempts,
    );
    let status_code = fetched
        .get("status_code")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let content_type = clean_text(
        fetched
            .get("content_type")
            .and_then(Value::as_str)
            .unwrap_or(""),
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
        format!(
            "Fetched non-text content ({}).",
            if content_type.is_empty() {
                "binary/unknown"
            } else {
                content_type.as_str()
            }
        )
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

fn api_search_serper(
    root: &Path,
    query: &str,
    summary_only: bool,
    human_approved: bool,
    allowed_domains: &[String],
    exclude_subdomains: bool,
    top_k: usize,
) -> Value {
    let requested_url = SERPER_SEARCH_URL.to_string();
    let Some(api_key) = serper_api_key() else {
        return json!({
            "ok": false,
            "error": "serper_api_key_missing",
            "requested_url": requested_url,
            "provider": "serperdev"
        });
    };
    let (policy, _policy_path_value) = load_policy(root);
    let policy_eval = infring_layer1_security::evaluate_web_conduit_policy(
        root,
        &json!({
            "requested_url": requested_url,
            "domain": extract_domain(&requested_url),
            "human_approved": human_approved,
            "requests_last_minute": requests_last_minute(root)
        }),
        &policy,
    );
    let allow = policy_eval
        .get("allow")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let reason = clean_text(
        policy_eval
            .get("reason")
            .and_then(Value::as_str)
            .unwrap_or("policy_denied"),
        180,
    );
    if !allow {
        let receipt = build_receipt(
            &requested_url,
            "deny",
            None,
            0,
            &reason,
            Some("policy_denied"),
        );
        let _ = append_jsonl(&receipts_path(root), &receipt);
        return json!({
            "ok": false,
            "error": "web_conduit_policy_denied",
            "requested_url": requested_url,
            "policy_decision": policy_eval,
            "provider": "serperdev",
            "receipt": receipt
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
    let fetched = fetch_serper_with_retry(
        &api_key,
        query,
        timeout_ms,
        max_response_bytes,
        retry_attempts,
        top_k,
    );
    let status_code = fetched
        .get("status_code")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let content_type = clean_text(
        fetched
            .get("content_type")
            .and_then(Value::as_str)
            .unwrap_or(""),
        180,
    );
    let parsed = render_serper_payload(
        fetched.get("body").and_then(Value::as_str).unwrap_or(""),
        allowed_domains,
        exclude_subdomains,
        top_k,
        max_response_bytes,
    );
    let content = clean_text(
        parsed.get("content").and_then(Value::as_str).unwrap_or(""),
        max_response_bytes,
    );
    let summary = clean_text(
        parsed.get("summary").and_then(Value::as_str).unwrap_or(""),
        900,
    );
    let response_hash = if content.is_empty() {
        String::new()
    } else {
        sha256_hex(&content)
    };
    let materialize_artifact = true;
    let artifact = if materialize_artifact {
        persist_artifact(root, &requested_url, &response_hash, &content)
    } else {
        None
    };
    let fetch_ok = fetched.get("ok").and_then(Value::as_bool).unwrap_or(false)
        && parsed.get("ok").and_then(Value::as_bool).unwrap_or(false)
        && !summary.is_empty();
    let mut error_value = clean_text(
        fetched.get("stderr").and_then(Value::as_str).unwrap_or(""),
        320,
    );
    if error_value.is_empty() {
        error_value = clean_text(
            parsed.get("error").and_then(Value::as_str).unwrap_or(""),
            220,
        );
    }
    let receipt = build_receipt(
        &requested_url,
        "allow",
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
        "content_type": if content_type.is_empty() { Value::String("application/json".to_string()) } else { Value::String(content_type) },
        "summary": summary,
        "content": if summary_only { Value::String(String::new()) } else { Value::String(content) },
        "links": parsed.get("links").cloned().unwrap_or_else(|| json!([])),
        "content_domains": parsed.get("content_domains").cloned().unwrap_or_else(|| json!([])),
        "provider_raw_count": parsed.get("provider_raw_count").cloned().unwrap_or_else(|| json!(0)),
        "provider_filtered_count": parsed.get("provider_filtered_count").cloned().unwrap_or_else(|| json!(0)),
        "retry_attempts": fetched.get("retry_attempts").cloned().unwrap_or_else(|| json!(1)),
        "retry_used": fetched.get("retry_used").cloned().unwrap_or_else(|| json!(false)),
        "user_agent": fetched.get("user_agent").cloned().unwrap_or_else(|| json!(DEFAULT_WEB_USER_AGENTS[0])),
        "response_hash": response_hash,
        "artifact": artifact.clone().unwrap_or(Value::Null),
        "policy_decision": policy_eval,
        "receipt": receipt,
        "provider": "serperdev",
        "error": if fetch_ok {
            Value::Null
        } else if error_value.is_empty() {
            Value::String("serper_search_failed".to_string())
        } else {
            Value::String(error_value)
        }
    })
}

fn api_search_bing_rss(
    query: &str,
    summary_only: bool,
    allowed_domains: &[String],
    exclude_subdomains: bool,
    top_k: usize,
) -> Value {
    let requested_url = web_search_bing_rss_url(query);
    let timeout_ms = 9_000u64;
    let max_response_bytes = 280_000usize;
    let retry_attempts = 2usize;
    let fetched = fetch_with_curl_retry(
        &requested_url,
        timeout_ms,
        max_response_bytes,
        retry_attempts,
    );
    let status_code = fetched
        .get("status_code")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let content_type = clean_text(
        fetched
            .get("content_type")
            .and_then(Value::as_str)
            .unwrap_or(""),
        180,
    );
    let parsed = render_bing_rss_payload(
        fetched.get("body").and_then(Value::as_str).unwrap_or(""),
        allowed_domains,
        exclude_subdomains,
        top_k,
        max_response_bytes,
    );
    let content = clean_text(
        parsed.get("content").and_then(Value::as_str).unwrap_or(""),
        max_response_bytes,
    );
    let summary = clean_text(
        parsed.get("summary").and_then(Value::as_str).unwrap_or(""),
        900,
    );
    let fetch_ok = fetched.get("ok").and_then(Value::as_bool).unwrap_or(false)
        && parsed.get("ok").and_then(Value::as_bool).unwrap_or(false)
        && !summary.is_empty();
    let mut error_value = clean_text(
        fetched.get("stderr").and_then(Value::as_str).unwrap_or(""),
        320,
    );
    if error_value.is_empty() {
        error_value = clean_text(
            parsed.get("error").and_then(Value::as_str).unwrap_or(""),
            220,
        );
    }
    json!({
        "ok": fetch_ok,
        "requested_url": requested_url,
        "status_code": status_code,
        "content_type": if content_type.is_empty() { Value::String("application/rss+xml".to_string()) } else { Value::String(content_type) },
        "summary": summary,
        "content": if summary_only { Value::String(String::new()) } else { Value::String(content) },
        "links": parsed.get("links").cloned().unwrap_or_else(|| json!([])),
        "content_domains": parsed.get("content_domains").cloned().unwrap_or_else(|| json!([])),
        "provider_raw_count": parsed.get("provider_raw_count").cloned().unwrap_or_else(|| json!(0)),
        "provider_filtered_count": parsed.get("provider_filtered_count").cloned().unwrap_or_else(|| json!(0)),
        "retry_attempts": fetched.get("retry_attempts").cloned().unwrap_or_else(|| json!(1)),
        "retry_used": fetched.get("retry_used").cloned().unwrap_or_else(|| json!(false)),
        "user_agent": fetched.get("user_agent").cloned().unwrap_or_else(|| json!(DEFAULT_WEB_USER_AGENTS[0])),
        "provider": "bing_rss",
        "error": if fetch_ok {
            Value::Null
        } else if error_value.is_empty() {
            Value::String("bing_rss_search_failed".to_string())
        } else {
            Value::String(error_value)
        }
    })
}

fn search_payload_usable(payload: &Value) -> bool {
    if !payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        return false;
    }
    if payload_looks_like_search_challenge(payload) || payload_looks_low_signal_search(payload) {
        return false;
    }
    let summary = clean_text(
        payload.get("summary").and_then(Value::as_str).unwrap_or(""),
        1_200,
    );
    if summary.is_empty() {
        return false;
    }
    let lowered = summary.to_ascii_lowercase();
    !lowered.contains("no relevant results found for that request yet")
        && !lowered.contains("couldn't produce source-backed findings in this turn")
}

fn search_payload_error(payload: &Value) -> String {
    let explicit = clean_text(
        payload.get("error").and_then(Value::as_str).unwrap_or(""),
        220,
    );
    if !explicit.is_empty() {
        return explicit;
    }
    if payload_looks_like_search_challenge(payload) {
        return "anti_bot_challenge".to_string();
    }
    if payload_looks_low_signal_search(payload) {
        return "low_signal_search_payload".to_string();
    }
    if !payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        return "search_provider_failed".to_string();
    }
    "no_usable_summary".to_string()
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
    let (policy, _policy_path_value) = load_policy(root);
    let allowed_domains =
        normalize_allowed_domains(request.get("allowed_domains").unwrap_or(&Value::Null));
    let exclude_subdomains = request
        .get("exclude_subdomains")
        .or_else(|| request.get("exact_domain_only"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let provider_hint = clean_text(
        request
            .get("provider")
            .or_else(|| request.get("source"))
            .or_else(|| request.get("search_provider"))
            .and_then(Value::as_str)
            .unwrap_or("auto"),
        40,
    )
    .to_ascii_lowercase();
    let top_k = request
        .get("top_k")
        .or_else(|| request.get("max_results"))
        .or_else(|| request.get("num"))
        .and_then(Value::as_u64)
        .unwrap_or(8)
        .clamp(1, 12) as usize;
    let scoped_query = scoped_search_query(&query, &allowed_domains, exclude_subdomains);
    let summary_only = request
        .get("summary_only")
        .or_else(|| request.get("summary"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let human_approved = request
        .get("human_approved")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let approval_id = request
        .get("approval_id")
        .and_then(Value::as_str)
        .unwrap_or("");
    let provider_chain = provider_chain_from_request(&provider_hint, request, &policy);
    let cache_key = search_cache_key(
        &query,
        &scoped_query,
        &allowed_domains,
        exclude_subdomains,
        top_k,
        summary_only,
        &provider_chain,
    );
    if let Some(mut cached) = load_search_cache(root, &cache_key) {
        if let Some(obj) = cached.as_object_mut() {
            obj.insert(
                "type".to_string(),
                Value::String("web_conduit_search".to_string()),
            );
            obj.insert("query".to_string(), Value::String(query.clone()));
            obj.insert(
                "effective_query".to_string(),
                Value::String(scoped_query.clone()),
            );
            obj.insert("allowed_domains".to_string(), json!(allowed_domains));
            obj.insert("exclude_subdomains".to_string(), json!(exclude_subdomains));
            obj.insert("top_k".to_string(), json!(top_k));
            obj.insert(
                "provider_hint".to_string(),
                Value::String(provider_hint.clone()),
            );
            obj.insert("provider_chain".to_string(), json!(provider_chain));
            obj.insert("cache_status".to_string(), json!("hit"));
        }
        return cached;
    }
    let primary_url = web_search_url(&scoped_query);
    let lite_url = web_search_lite_url(&scoped_query);
    let mut selected_provider = String::new();
    let mut selected = Value::Null;
    let mut attempted = Vec::<String>::new();
    let mut skipped = Vec::<Value>::new();
    let mut provider_errors = Vec::<Value>::new();
    let mut last_payload = None::<Value>;

    for provider in &provider_chain {
        if let Some(open_until) = provider_circuit_open_until(root, provider, &policy) {
            skipped.push(json!({
                "provider": provider,
                "reason": "circuit_open",
                "open_until": open_until
            }));
            continue;
        }
        attempted.push(provider.clone());
        let candidate = match provider.as_str() {
            "serperdev" => api_search_serper(
                root,
                &scoped_query,
                summary_only,
                human_approved,
                &allowed_domains,
                exclude_subdomains,
                top_k,
            ),
            "duckduckgo_lite" => api_fetch(
                root,
                &json!({
                    "url": lite_url,
                    "summary_only": summary_only,
                    "human_approved": human_approved,
                    "approval_id": approval_id
                }),
            ),
            "bing_rss" => api_search_bing_rss(
                &scoped_query,
                summary_only,
                &allowed_domains,
                exclude_subdomains,
                top_k,
            ),
            _ => api_fetch(
                root,
                &json!({
                    "url": primary_url,
                    "summary_only": summary_only,
                    "human_approved": human_approved,
                    "approval_id": approval_id
                }),
            ),
        };
        if search_payload_usable(&candidate) {
            record_provider_attempt(root, provider, true, "", &policy);
            selected_provider = provider.clone();
            selected = candidate;
            break;
        }
        let reason = search_payload_error(&candidate);
        record_provider_attempt(root, provider, false, &reason, &policy);
        provider_errors.push(json!({
            "provider": provider,
            "error": reason,
            "challenge": payload_looks_like_search_challenge(&candidate),
            "low_signal": payload_looks_low_signal_search(&candidate),
            "status_code": candidate.get("status_code").and_then(Value::as_i64).unwrap_or(0)
        }));
        last_payload = Some(candidate);
    }

    let mut out = if !selected.is_null() {
        selected
    } else {
        last_payload.unwrap_or_else(|| {
            json!({
                "ok": false,
                "error": "search_providers_exhausted",
                "summary": "Search providers returned no usable findings. Retry with narrower query or explicit source URLs.",
                "content": ""
            })
        })
    };
    if out
        .get("provider")
        .and_then(Value::as_str)
        .unwrap_or("")
        .is_empty()
    {
        if let Some(obj) = out.as_object_mut() {
            obj.insert(
                "provider".to_string(),
                if selected_provider.is_empty() {
                    Value::String("none".to_string())
                } else {
                    Value::String(selected_provider.clone())
                },
            );
        }
    }
    if !out.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        if let Some(obj) = out.as_object_mut() {
            if obj
                .get("summary")
                .and_then(Value::as_str)
                .unwrap_or("")
                .is_empty()
            {
                obj.insert(
                    "summary".to_string(),
                    Value::String(
                        "Search providers returned no usable findings. Retry with narrower query or explicit source URLs.".to_string(),
                    ),
                );
            }
            if obj.get("error").is_none() {
                obj.insert(
                    "error".to_string(),
                    Value::String("search_providers_exhausted".to_string()),
                );
            }
        }
    }
    let used_lite_fallback = selected_provider == "duckduckgo_lite";
    let used_bing_fallback = selected_provider == "bing_rss";
    if let Some(obj) = out.as_object_mut() {
        obj.insert(
            "type".to_string(),
            Value::String("web_conduit_search".to_string()),
        );
        obj.insert("query".to_string(), Value::String(query.clone()));
        obj.insert(
            "effective_query".to_string(),
            Value::String(scoped_query.clone()),
        );
        obj.insert(
            "allowed_domains".to_string(),
            json!(allowed_domains.clone()),
        );
        obj.insert("exclude_subdomains".to_string(), json!(exclude_subdomains));
        obj.insert("top_k".to_string(), json!(top_k));
        obj.insert("provider_chain".to_string(), json!(provider_chain.clone()));
        obj.insert("providers_attempted".to_string(), json!(attempted));
        obj.insert("providers_skipped".to_string(), json!(skipped));
        obj.insert("provider_errors".to_string(), json!(provider_errors));
        obj.insert(
            "provider_health".to_string(),
            provider_health_snapshot(root, &provider_chain),
        );
        obj.insert(
            "search_lite_fallback".to_string(),
            json!(used_lite_fallback),
        );
        obj.insert(
            "search_bing_fallback".to_string(),
            json!(used_bing_fallback),
        );
        obj.insert("provider_hint".to_string(), Value::String(provider_hint));
        obj.insert("cache_status".to_string(), json!("miss"));
    }
    let cache_status = if out.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        "ok"
    } else {
        "no_results"
    };
    store_search_cache(root, &cache_key, &out, cache_status);
    out
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let parsed = parse_args(argv);
    let command = parsed
        .positional
        .first()
        .map(|row| row.to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());
    let nexus_connection = match command.as_str() {
        "fetch" | "browse" => {
            match crate::dashboard_tool_turn_loop::authorize_ingress_tool_call_with_nexus(
                "web_conduit_fetch",
            ) {
                Ok(meta) => meta,
                Err(err) => {
                    println!(
                        "{}",
                        json!({
                            "ok": false,
                            "type": "web_conduit_nexus_gate",
                            "error": "nexus_route_denied",
                            "command": clean_text(command.as_str(), 40),
                            "reason": clean_text(&err, 240),
                            "fail_closed": true
                        })
                    );
                    return 1;
                }
            }
        }
        "search" => {
            match crate::dashboard_tool_turn_loop::authorize_ingress_tool_call_with_nexus(
                "web_search",
            ) {
                Ok(meta) => meta,
                Err(err) => {
                    println!(
                        "{}",
                        json!({
                            "ok": false,
                            "type": "web_conduit_nexus_gate",
                            "error": "nexus_route_denied",
                            "command": clean_text(command.as_str(), 40),
                            "reason": clean_text(&err, 240),
                            "fail_closed": true
                        })
                    );
                    return 1;
                }
            }
        }
        _ => None,
    };
    let mut payload = match command.as_str() {
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
            let provider = clean_text(
                parsed
                    .flags
                    .get("provider")
                    .or_else(|| parsed.flags.get("source"))
                    .or_else(|| parsed.flags.get("search-provider"))
                    .or_else(|| parsed.flags.get("search_provider"))
                    .map(String::as_str)
                    .unwrap_or("auto"),
                40,
            );
            let top_k = parse_u64(
                parsed
                    .flags
                    .get("top-k")
                    .or_else(|| parsed.flags.get("top_k"))
                    .or_else(|| parsed.flags.get("max-results"))
                    .or_else(|| parsed.flags.get("max_results")),
                8,
                1,
                12,
            );
            api_search(
                root,
                &json!({
                    "query": query,
                    "allowed_domains": normalize_allowed_domains(&json!(allowed_domains)),
                    "provider": provider,
                    "top_k": top_k,
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
    if let Some(meta) = nexus_connection {
        if let Some(obj) = payload.as_object_mut() {
            obj.insert("nexus_connection".to_string(), meta);
        }
    }
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
        assert!(
            matches!(
                out.get("provider").and_then(Value::as_str),
                Some("duckduckgo")
                    | Some("duckduckgo_lite")
                    | Some("bing_rss")
                    | Some("serperdev")
                    | Some("none")
            ),
            "unexpected provider: {:?}",
            out.get("provider")
        );
        assert!(out.get("provider_chain").is_some());
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
        assert_eq!(
            domains,
            vec!["github.com".to_string(), "docs.rs".to_string()]
        );
    }

    #[test]
    fn scoped_search_query_supports_exact_domain_mode() {
        let scoped =
            scoped_search_query("agent reliability", &vec!["example.com".to_string()], true);
        assert!(scoped.contains("site:example.com"));
        assert!(scoped.contains("-site:*.example.com"));
    }

    #[test]
    fn normalize_allowed_domains_supports_comma_string() {
        let domains =
            normalize_allowed_domains(&json!("https://www.github.com, docs.rs *.example.com"));
        assert_eq!(
            domains,
            vec![
                "github.com".to_string(),
                "docs.rs".to_string(),
                "example.com".to_string()
            ]
        );
    }

    #[test]
    fn domain_allowed_scope_respects_exact_domain_mode() {
        let filters = vec!["example.com".to_string()];
        assert!(domain_allowed_for_scope(
            "https://example.com/docs",
            &filters,
            true
        ));
        assert!(!domain_allowed_for_scope(
            "https://blog.example.com/post",
            &filters,
            true
        ));
        assert!(domain_allowed_for_scope(
            "https://blog.example.com/post",
            &filters,
            false
        ));
    }

    #[test]
    fn render_serper_payload_filters_domains_and_builds_content() {
        let body = serde_json::to_string(&json!({
            "organic": [
                {
                    "title": "Main",
                    "link": "https://example.com/main",
                    "snippet": "Main domain snippet"
                },
                {
                    "title": "Subdomain",
                    "link": "https://blog.example.com/post",
                    "snippet": "Subdomain snippet"
                },
                {
                    "title": "Other",
                    "link": "https://other.com/page",
                    "snippet": "Other domain snippet"
                }
            ]
        }))
        .expect("encode");
        let rendered =
            render_serper_payload(&body, &vec!["example.com".to_string()], true, 8, 12_000);
        assert_eq!(rendered.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            rendered.get("provider_raw_count").and_then(Value::as_u64),
            Some(3)
        );
        assert_eq!(
            rendered
                .get("provider_filtered_count")
                .and_then(Value::as_u64),
            Some(1)
        );
        let links = rendered
            .get("links")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].as_str(), Some("https://example.com/main"));
    }

    #[test]
    fn render_serper_payload_handles_invalid_json() {
        let rendered = render_serper_payload("not-json", &[], false, 8, 12_000);
        assert_eq!(rendered.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(
            rendered.get("error").and_then(Value::as_str),
            Some("serper_decode_failed")
        );
    }

    #[test]
    fn render_bing_rss_payload_filters_domains_and_builds_content() {
        let body = r#"
        <rss><channel>
          <item>
            <title>Main Result</title>
            <link>https://example.com/main</link>
            <description>Main description text</description>
          </item>
          <item>
            <title>Other Result</title>
            <link>https://other.com/page</link>
            <description>Other description text</description>
          </item>
        </channel></rss>
        "#;
        let rendered =
            render_bing_rss_payload(body, &vec!["example.com".to_string()], true, 8, 12_000);
        assert_eq!(rendered.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            rendered.get("provider_raw_count").and_then(Value::as_u64),
            Some(2)
        );
        assert_eq!(
            rendered
                .get("provider_filtered_count")
                .and_then(Value::as_u64),
            Some(1)
        );
        let links = rendered
            .get("links")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].as_str(), Some("https://example.com/main"));
    }

    #[test]
    fn payload_challenge_detector_flags_duckduckgo_challenge_dump() {
        let payload = json!({
            "summary": "DuckDuckGo challenge",
            "content": "Unfortunately, bots use DuckDuckGo too. Please complete the following challenge. Select all squares containing a duck."
        });
        assert!(payload_looks_like_search_challenge(&payload));
    }

    #[test]
    fn payload_low_signal_detector_flags_duckduckgo_chrome_summary() {
        let payload = json!({
            "summary": "latest technology news today at DuckDuckGo All Regions Argentina Australia Safe Search Any Time",
            "content": ""
        });
        assert!(payload_looks_low_signal_search(&payload));
    }

    #[test]
    fn payload_low_signal_detector_flags_source_scaffold_summary() {
        let payload = json!({
            "summary": "Key findings for \"Infring AI vs competitors\": - Potential sources: hai.stanford.edu, artificialanalysis.ai.",
            "content": ""
        });
        assert!(payload_looks_low_signal_search(&payload));
    }

    #[test]
    fn search_uses_cached_response_when_available() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let request = json!({
            "query": "agent reliability benchmark",
            "summary_only": true
        });
        let query = clean_text(
            request
                .get("query")
                .and_then(Value::as_str)
                .unwrap_or_default(),
            600,
        );
        let allowed_domains =
            normalize_allowed_domains(request.get("allowed_domains").unwrap_or(&Value::Null));
        let exclude_subdomains = request
            .get("exclude_subdomains")
            .or_else(|| request.get("exact_domain_only"))
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let top_k = 8usize;
        let summary_only = true;
        let scoped_query = scoped_search_query(&query, &allowed_domains, exclude_subdomains);
        let (policy, _) = load_policy(tmp.path());
        let provider_chain = crate::web_conduit_provider_runtime::provider_chain_from_request(
            "auto", &request, &policy,
        );
        let key = crate::web_conduit_provider_runtime::search_cache_key(
            &query,
            &scoped_query,
            &allowed_domains,
            exclude_subdomains,
            top_k,
            summary_only,
            &provider_chain,
        );
        crate::web_conduit_provider_runtime::store_search_cache(
            tmp.path(),
            &key,
            &json!({
                "ok": true,
                "summary": "cached search summary",
                "content": "",
                "provider": "duckduckgo"
            }),
            "ok",
        );

        let out = api_search(tmp.path(), &request);
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            out.get("summary").and_then(Value::as_str),
            Some("cached search summary")
        );
        assert_eq!(out.get("cache_status").and_then(Value::as_str), Some("hit"));
    }
}
