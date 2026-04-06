use regex::Regex;
use serde::Serialize;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::thread;
use std::time::Instant;

use crate::parse_args;

const POLICY_REL: &str = "client/runtime/config/batch_query_policy.json";
const RECEIPTS_REL: &str = "client/runtime/local/state/batch_query/receipts.jsonl";

#[derive(Clone, Copy, Debug)]
struct ApertureBudget {
    max_candidates: usize,
    max_evidence: usize,
    max_summary_tokens: usize,
    max_query_rewrites: usize,
}

#[derive(Clone, Debug, Serialize)]
struct EvidenceRef {
    source_kind: String,
    title: String,
    locator: String,
    excerpt_hash: String,
    score: f64,
    timestamp: Option<String>,
    permissions: Option<String>,
}

#[derive(Clone, Debug)]
struct Candidate {
    source_kind: String,
    title: String,
    locator: String,
    snippet: String,
    excerpt_hash: String,
    timestamp: Option<String>,
    permissions: Option<String>,
    status_code: i64,
}

fn clean_text(raw: &str, max_len: usize) -> String {
    crate::contract_lane_utils::clean_text(Some(raw), max_len.max(1))
}

fn trim_words(raw: &str, max_words: usize) -> String {
    if max_words == 0 {
        return String::new();
    }
    raw.split_whitespace()
        .take(max_words)
        .collect::<Vec<_>>()
        .join(" ")
}

fn read_json_or(path: &Path, fallback: Value) -> Value {
    crate::contract_lane_utils::read_json(path).unwrap_or(fallback)
}

fn write_json_atomic(path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("batch_query_create_parent_failed:{err}"))?;
    }
    let tmp = path.with_extension(format!(
        "tmp-{}-{}",
        std::process::id(),
        chrono::Utc::now().timestamp_millis()
    ));
    let encoded = serde_json::to_vec_pretty(value)
        .map_err(|err| format!("batch_query_encode_policy_failed:{err}"))?;
    fs::write(&tmp, encoded).map_err(|err| format!("batch_query_write_policy_tmp_failed:{err}"))?;
    fs::rename(&tmp, path).map_err(|err| format!("batch_query_rename_policy_failed:{err}"))?;
    Ok(())
}

fn append_jsonl(path: &Path, row: &Value) -> Result<(), String> {
    crate::contract_lane_utils::append_jsonl(path, row)
        .map_err(|err| format!("batch_query_append_receipt_failed:{err}"))
}

fn default_policy() -> Value {
    json!({
        "version": "v1",
        "batch_query": {
            "enabled_sources": ["web"],
            "allow_large": false,
            "max_parallel_subqueries": 4
        }
    })
}

fn policy_path(root: &Path) -> PathBuf {
    root.join(POLICY_REL)
}

fn receipts_path(root: &Path) -> PathBuf {
    root.join(RECEIPTS_REL)
}

fn load_policy(root: &Path) -> Value {
    let path = policy_path(root);
    if !path.exists() {
        let _ = write_json_atomic(&path, &default_policy());
    }
    read_json_or(&path, default_policy())
}

fn aperture_budget(aperture: &str) -> Option<ApertureBudget> {
    match aperture {
        "small" => Some(ApertureBudget {
            max_candidates: 8,
            max_evidence: 2,
            max_summary_tokens: 180,
            max_query_rewrites: 0,
        }),
        "medium" => Some(ApertureBudget {
            max_candidates: 20,
            max_evidence: 6,
            max_summary_tokens: 350,
            max_query_rewrites: 1,
        }),
        "large" => None,
        _ => None,
    }
}

fn normalize_source(raw: &str) -> String {
    let normalized = clean_text(raw, 40).to_ascii_lowercase();
    if normalized.is_empty() {
        "web".to_string()
    } else {
        normalized
    }
}

fn normalize_aperture(raw: &str) -> String {
    let normalized = clean_text(raw, 20).to_ascii_lowercase();
    if normalized.is_empty() {
        "medium".to_string()
    } else {
        normalized
    }
}

fn enabled_sources(policy: &Value) -> Vec<String> {
    policy
        .pointer("/batch_query/enabled_sources")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|row| row.to_ascii_lowercase())
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| vec!["web".to_string()])
}

fn allow_large(policy: &Value) -> bool {
    policy
        .pointer("/batch_query/allow_large")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn max_parallel_subqueries(policy: &Value) -> usize {
    policy
        .pointer("/batch_query/max_parallel_subqueries")
        .and_then(Value::as_u64)
        .unwrap_or(4)
        .clamp(1, 16) as usize
}

fn exact_match_regexes() -> &'static [Regex] {
    static REGEXES: OnceLock<Vec<Regex>> = OnceLock::new();
    REGEXES.get_or_init(|| {
        vec![
            Regex::new(r#""[^"]+""#).expect("quoted"),
            Regex::new(r"https?://\S+").expect("url"),
            Regex::new(r"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}").expect("email"),
            Regex::new(r"\b[a-fA-F0-9]{8,}\b").expect("hex-id"),
            Regex::new(r"[/~][A-Za-z0-9._/\-]+").expect("path"),
            Regex::new(r"[A-Za-z_][A-Za-z0-9_]*::[A-Za-z_][A-Za-z0-9_]*").expect("symbol"),
        ]
    })
}

fn is_exact_match_pattern(query: &str) -> bool {
    exact_match_regexes().iter().any(|re| re.is_match(query))
}

fn build_query_plan(query: &str, budget: ApertureBudget) -> (Vec<String>, Vec<String>, bool) {
    let base = clean_text(query, 600);
    if base.is_empty() {
        return (Vec::new(), Vec::new(), false);
    }
    let exact = is_exact_match_pattern(&base);
    if exact || budget.max_query_rewrites == 0 {
        return (vec![base], Vec::new(), false);
    }
    let rewrite = clean_text(&format!("{base} overview"), 600);
    if rewrite == base {
        return (vec![base], Vec::new(), false);
    }
    (vec![base.clone(), rewrite.clone()], vec![rewrite], true)
}

fn sha256_hex(raw: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(raw.as_bytes());
    let digest = hasher.finalize();
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

fn looks_like_ack_only(text: &str) -> bool {
    let lowered = clean_text(text, 800).to_ascii_lowercase();
    if lowered.is_empty() {
        return true;
    }
    lowered.contains("web search completed")
        || lowered.contains("tool completed")
        || lowered.contains("searched the internet")
        || lowered == "search completed."
}

fn looks_like_low_signal_search_summary(text: &str) -> bool {
    let lowered = clean_text(text, 3_200).to_ascii_lowercase();
    if lowered.is_empty() {
        return true;
    }
    if lowered.contains("unfortunately, bots use duckduckgo too")
        || lowered.contains("please complete the following challenge")
        || lowered.contains("anomaly-modal")
    {
        return true;
    }
    let marker_hits = [
        "all regions",
        "safe search",
        "any time",
        "at duckduckgo",
        "viewing ads is privacy protected by duckduckgo",
        "ad clicks are managed by microsoft",
    ]
    .iter()
    .filter(|marker| lowered.contains(**marker))
    .count();
    marker_hits >= 2
}

fn looks_like_source_only_snippet(text: &str) -> bool {
    let lowered = clean_text(text, 1_200).to_ascii_lowercase();
    if lowered.is_empty() {
        return true;
    }
    if lowered.starts_with("potential sources:")
        || lowered.starts_with("candidate sources:")
        || lowered.starts_with("found sources:")
    {
        let domain_hits = extract_domains_from_text(&lowered, 8).len();
        let word_count = lowered.split_whitespace().count();
        if domain_hits > 0 && word_count <= 28 {
            return true;
        }
    }
    false
}

fn skip_duckduckgo_fallback_for_error(primary_err: &str) -> bool {
    let lowered = clean_text(primary_err, 240).to_ascii_lowercase();
    lowered.contains("policy_blocked")
        || lowered.contains("source_blocked")
        || lowered.contains("aperture_blocked")
        || lowered.contains("domain_blocked")
}

fn looks_like_html_markup(text: &str) -> bool {
    static HTML_HINT_RE: OnceLock<Regex> = OnceLock::new();
    let re = HTML_HINT_RE.get_or_init(|| {
        Regex::new(r"(?is)<!doctype\s+html|<html|<head|<body|<div\b|<p\b|<a\s+href=|<script\b")
            .expect("html-hint")
    });
    re.is_match(text)
}

fn html_slimdown_regexes() -> &'static [Regex] {
    static REGEXES: OnceLock<Vec<Regex>> = OnceLock::new();
    REGEXES.get_or_init(|| {
        vec![
            Regex::new(r"(?is)<script[^>]*>.*?</script>").expect("html-script"),
            Regex::new(r"(?is)<style[^>]*>.*?</style>").expect("html-style"),
            Regex::new(r"(?is)<svg[^>]*>.*?</svg>").expect("html-svg"),
            Regex::new(r"(?is)<img[^>]*>").expect("html-img"),
            Regex::new(r#"(?is)<[^>]*(?:href|src)\s*=\s*["']data:[^"']*["'][^>]*>"#)
                .expect("html-data-uri"),
        ]
    })
}

fn html_anchor_href_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r#"(?is)<a[^>]*href\s*=\s*["']([^"']+)["'][^>]*>"#).expect("html-anchor-href")
    })
}

fn html_tag_attr_strip_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"(?is)<([a-z0-9]+)\s+[^>]*>").expect("html-tag-attr-strip"))
}

fn html_all_tags_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"(?is)<[^>]+>").expect("html-all-tags"))
}

fn normalize_htmlish_content_for_snippet(raw: &str) -> String {
    if !looks_like_html_markup(raw) {
        return clean_text(raw, 12_000);
    }
    let mut slim = raw.to_string();
    for re in html_slimdown_regexes() {
        slim = re.replace_all(&slim, " ").to_string();
    }
    slim = html_anchor_href_regex()
        .replace_all(&slim, r#"<a href="$1">"#)
        .to_string();
    slim = html_tag_attr_strip_regex()
        .replace_all(&slim, "<$1>")
        .to_string();
    slim = html_all_tags_regex().replace_all(&slim, " ").to_string();
    clean_text(&slim, 12_000)
}

fn search_domain_capture_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"(?i)\b(?:https?://)?(?:www\.)?([a-z0-9][a-z0-9.-]*\.[a-z]{2,})(?:/[^\s]*)?")
            .expect("search-domain-regex")
    })
}

fn extract_domains_from_text(text: &str, max_domains: usize) -> Vec<String> {
    if max_domains == 0 {
        return Vec::new();
    }
    let mut out = Vec::<String>::new();
    let mut seen = HashSet::<String>::new();
    for capture in search_domain_capture_regex().captures_iter(text) {
        let host = capture
            .get(1)
            .map(|row| row.as_str())
            .unwrap_or("")
            .trim()
            .trim_matches('.')
            .to_ascii_lowercase();
        if host.is_empty() || host == "duckduckgo.com" || host.ends_with(".duckduckgo.com") {
            continue;
        }
        if !seen.insert(host.clone()) {
            continue;
        }
        out.push(host);
        if out.len() >= max_domains {
            break;
        }
    }
    out
}

fn fixture_payload_for_query(query: &str) -> Option<Value> {
    let fixtures = fixture_payload_map()?;
    fixtures
        .get(query)
        .cloned()
        .or_else(|| fixtures.get("*").cloned())
        .or_else(|| fixtures.get("default").cloned())
}

fn fixture_payload_for_stage_query(stage: &str, query: &str) -> Option<Value> {
    let fixtures = fixture_payload_map()?;
    let key = format!("{stage}::{query}");
    fixtures.get(&key).cloned()
}

fn fixture_payload_map() -> Option<Map<String, Value>> {
    let raw = std::env::var("INFRING_BATCH_QUERY_TEST_FIXTURE_JSON").ok()?;
    let decoded = serde_json::from_str::<Value>(&raw).ok()?;
    decoded.as_object().cloned()
}

fn duckduckgo_instant_answer_url(query: &str) -> String {
    let cleaned = clean_text(query, 600);
    let encoded = urlencoding::encode(&cleaned);
    format!("https://api.duckduckgo.com/?q={encoded}&format=json&no_html=1&skip_disambig=1")
}

fn first_related_topic_summary(rows: &[Value]) -> Option<(String, String)> {
    for row in rows {
        let text = clean_text(row.get("Text").and_then(Value::as_str).unwrap_or(""), 1_600);
        let locator = clean_text(
            row.get("FirstURL").and_then(Value::as_str).unwrap_or(""),
            2_200,
        );
        if !text.is_empty() {
            return Some((text, locator));
        }
        if let Some(children) = row.get("Topics").and_then(Value::as_array) {
            if let Some(found) = first_related_topic_summary(children) {
                return Some(found);
            }
        }
    }
    None
}

fn candidate_from_duckduckgo_instant_payload(
    query: &str,
    fallback_url: &str,
    payload: &Value,
) -> Result<Candidate, String> {
    if !payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        return Err(clean_text(
            payload
                .get("error")
                .and_then(Value::as_str)
                .unwrap_or("duckduckgo_instant_fetch_failed"),
            220,
        ));
    }
    let content = clean_text(
        payload.get("content").and_then(Value::as_str).unwrap_or(""),
        64_000,
    );
    let decoded = serde_json::from_str::<Value>(&content).unwrap_or(Value::Null);
    let mut snippet = clean_text(
        decoded
            .get("AbstractText")
            .and_then(Value::as_str)
            .unwrap_or(""),
        1_800,
    );
    if snippet.is_empty() {
        snippet = clean_text(
            decoded.get("Answer").and_then(Value::as_str).unwrap_or(""),
            1_200,
        );
    }
    if snippet.is_empty() {
        snippet = clean_text(
            decoded
                .get("Definition")
                .and_then(Value::as_str)
                .unwrap_or(""),
            1_800,
        );
    }
    let mut locator = clean_text(
        decoded
            .get("AbstractURL")
            .and_then(Value::as_str)
            .unwrap_or(""),
        2_200,
    );
    if snippet.is_empty() {
        if let Some(related) = decoded.get("RelatedTopics").and_then(Value::as_array) {
            if let Some((related_text, related_locator)) = first_related_topic_summary(related) {
                snippet = related_text;
                if locator.is_empty() {
                    locator = related_locator;
                }
            }
        }
    }
    if snippet.is_empty() {
        let summary = clean_text(
            payload.get("summary").and_then(Value::as_str).unwrap_or(""),
            1_200,
        );
        if !summary.is_empty()
            && !looks_like_ack_only(&summary)
            && !looks_like_low_signal_search_summary(&summary)
        {
            snippet = summary;
        }
    }
    if snippet.is_empty() {
        return Err("duckduckgo_instant_no_usable_summary".to_string());
    }
    let mut title = clean_text(
        decoded.get("Heading").and_then(Value::as_str).unwrap_or(""),
        160,
    );
    if title.is_empty() {
        title = format!("Instant web result for {}", clean_text(query, 120));
    }
    if locator.is_empty() {
        locator = clean_text(fallback_url, 2_200);
    }
    Ok(Candidate {
        source_kind: "web".to_string(),
        title,
        locator,
        snippet: snippet.clone(),
        excerpt_hash: sha256_hex(&snippet),
        timestamp: Some(crate::now_iso()),
        permissions: Some("public_web".to_string()),
        status_code: payload
            .get("status_code")
            .and_then(Value::as_i64)
            .unwrap_or(0),
    })
}

fn candidate_from_search_payload(query: &str, payload: &Value) -> Result<Candidate, String> {
    if !payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        return Err(clean_text(
            payload
                .get("error")
                .and_then(Value::as_str)
                .unwrap_or("adapter_failed"),
            200,
        ));
    }
    let summary = clean_text(
        payload.get("summary").and_then(Value::as_str).unwrap_or(""),
        1800,
    );
    let content = clean_text(
        payload.get("content").and_then(Value::as_str).unwrap_or(""),
        6_000,
    );
    let content_normalized = normalize_htmlish_content_for_snippet(&content);
    let summary_low_signal = looks_like_low_signal_search_summary(&summary);
    let domains = extract_domains_from_text(
        if content.is_empty() {
            &summary
        } else {
            &content
        },
        5,
    );
    let mut snippet =
        if !summary.is_empty() && !looks_like_ack_only(&summary) && !summary_low_signal {
            summary.clone()
        } else {
            String::new()
        };
    if snippet.is_empty()
        && !content_normalized.is_empty()
        && !looks_like_ack_only(&content_normalized)
    {
        snippet = trim_words(&content_normalized, 56);
    }
    if snippet.is_empty()
        && !summary.is_empty()
        && !looks_like_ack_only(&summary)
        && !summary_low_signal
    {
        snippet = trim_words(&summary, 56);
    }
    if snippet.is_empty() {
        return Err("no_usable_summary".to_string());
    }
    if looks_like_source_only_snippet(&snippet) {
        return Err("no_usable_summary".to_string());
    }
    let locator = clean_text(
        payload
            .get("requested_url")
            .or_else(|| payload.pointer("/receipt/requested_url"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        2200,
    );
    let title = if let Some(first_domain) = domains.first() {
        format!("Web result from {}", clean_text(first_domain, 120))
    } else if locator.is_empty() {
        format!("Web result for {}", clean_text(query, 120))
    } else {
        format!("Web result from {}", clean_text(&locator, 120))
    };
    Ok(Candidate {
        source_kind: "web".to_string(),
        title,
        locator,
        snippet: snippet.clone(),
        excerpt_hash: sha256_hex(&snippet),
        timestamp: Some(crate::now_iso()),
        permissions: Some("public_web".to_string()),
        status_code: payload
            .get("status_code")
            .and_then(Value::as_i64)
            .unwrap_or(0),
    })
}
