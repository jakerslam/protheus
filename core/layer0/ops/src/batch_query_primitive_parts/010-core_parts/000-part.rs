use regex::Regex;
use serde::Serialize;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::thread;
use std::time::{Duration, Instant};

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
            "max_parallel_subqueries": 4,
            "query_timeout_ms": 5000
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

fn query_timeout(policy: &Value) -> Duration {
    let timeout_ms = policy
        .pointer("/batch_query/query_timeout_ms")
        .and_then(Value::as_u64)
        .unwrap_or(5000)
        .clamp(500, 20_000);
    Duration::from_millis(timeout_ms)
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

fn instruction_frame_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(
            r"(?i)\b(?:verify|check|test|research(?:ing)?|find(?:\s+out)?|report|return|provide|show|summarize|compare|assess|evaluate|investigate|answer)\b",
        )
        .expect("instruction-frame")
    })
}

fn instruction_tail_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(
            r"(?i)\b(?:verify|check|test|research(?:ing)?|find(?:\s+out)?|report|return|provide|show|summarize|compare|assess|evaluate|investigate|answer)\b.{0,120}?\b(?:by|about|on)\b\s+(.+)$",
        )
        .expect("instruction-tail")
    })
}

fn looks_like_instructional_query(query: &str) -> bool {
    let base = clean_text(query, 600);
    if base.is_empty() {
        return false;
    }
    let word_count = base.split_whitespace().count();
    if word_count < 9 {
        return false;
    }
    instruction_frame_regex().is_match(&base)
}

fn is_instruction_stop_token(token: &str) -> bool {
    matches!(
        token,
        "please"
            | "kindly"
            | "verify"
            | "check"
            | "test"
            | "research"
            | "researching"
            | "find"
            | "found"
            | "report"
            | "return"
            | "provide"
            | "show"
            | "summarize"
            | "answer"
            | "question"
            | "questions"
            | "results"
            | "result"
            | "using"
            | "with"
            | "into"
            | "actual"
            | "proper"
            | "web"
            | "search"
            | "fetch"
            | "tool"
            | "tools"
            | "functionality"
            | "capabilities"
    )
}

fn normalize_instructional_query(query: &str) -> Option<String> {
    let base = clean_text(query, 600);
    if base.is_empty() {
        return None;
    }
    let lowered = base.to_ascii_lowercase();
    let focus_seed = instruction_tail_regex()
        .captures(&lowered)
        .and_then(|caps| caps.get(1).map(|row| row.as_str().to_string()))
        .unwrap_or(lowered);
    let tokens = focus_seed
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .filter(|token| token.len() > 2 || token.chars().all(|ch| ch.is_ascii_digit()))
        .map(|token| token.to_ascii_lowercase())
        .filter(|token| !is_instruction_stop_token(token.as_str()))
        .collect::<Vec<_>>();
    if tokens.len() < 3 {
        return None;
    }
    let candidate = clean_text(&tokens.join(" "), 600);
    if candidate.is_empty() {
        None
    } else {
        Some(candidate)
    }
}

fn deictic_framework_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"(?i)\bthis\s+(?:framework|system|platform|stack|agent\s+framework)\b")
            .expect("deictic-framework")
    })
}

fn resolve_deictic_framework_reference(query: &str) -> String {
    let cleaned = clean_text(query, 600);
    if cleaned.is_empty() {
        return cleaned;
    }
    let replaced = deictic_framework_regex().replace_all(&cleaned, "infring");
    clean_text(replaced.as_ref(), 600)
}

fn build_query_plan(query: &str, budget: ApertureBudget) -> (Vec<String>, Vec<String>, bool) {
    let base = resolve_deictic_framework_reference(query);
    if base.is_empty() {
        return (Vec::new(), Vec::new(), false);
    }
    let exact = is_exact_match_pattern(&base);
    if exact || budget.max_query_rewrites == 0 {
        return (vec![base], Vec::new(), false);
    }
    let rewrite = preferred_query_rewrite(&base);
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
    let cleaned = clean_text(text, 3_200);
    if cleaned.is_empty() {
        return true;
    }
    if looks_like_empty_duckduckgo_instant_shell_text(&cleaned) {
        return true;
    }
    let lowered = cleaned.to_ascii_lowercase();
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

fn looks_like_empty_duckduckgo_instant_shell_text(text: &str) -> bool {
    let cleaned = clean_text(text, 3_200);
    let start = match cleaned.find('{') {
        Some(idx) => idx,
        None => return looks_like_truncated_duckduckgo_instant_shell(&cleaned),
    };
    let end = match cleaned.rfind('}') {
        Some(idx) if idx > start => idx,
        _ => return looks_like_truncated_duckduckgo_instant_shell(&cleaned[start..]),
    };
    let decoded = serde_json::from_str::<Value>(&cleaned[start..=end]).unwrap_or(Value::Null);
    looks_like_empty_duckduckgo_instant_shell(&decoded)
        || looks_like_truncated_duckduckgo_instant_shell(&cleaned[start..=end])
}
