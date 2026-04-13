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

fn looks_like_empty_duckduckgo_instant_shell(decoded: &Value) -> bool {
    let Some(obj) = decoded.as_object() else {
        return false;
    };
    let metadata_keys = [
        "Abstract",
        "AbstractSource",
        "AbstractText",
        "AbstractURL",
        "Answer",
        "AnswerType",
        "Definition",
        "DefinitionSource",
        "DefinitionURL",
        "Heading",
        "RelatedTopics",
        "Results",
        "Type",
    ];
    let metadata_hits = metadata_keys
        .iter()
        .filter(|key| obj.contains_key(**key))
        .count();
    if metadata_hits < 5 {
        return false;
    }
    let has_usable_primary_text = ["AbstractText", "Answer", "Definition", "Heading"]
        .iter()
        .any(|key| {
            clean_text(
                obj.get(*key).and_then(Value::as_str).unwrap_or(""),
                400,
            )
            .len()
                > 1
        });
    if has_usable_primary_text {
        return false;
    }
    let has_related_topics = obj
        .get("RelatedTopics")
        .and_then(Value::as_array)
        .map(|rows| !rows.is_empty())
        .unwrap_or(false);
    if has_related_topics {
        return false;
    }
    let has_results = obj
        .get("Results")
        .and_then(Value::as_array)
        .map(|rows| !rows.is_empty())
        .unwrap_or(false);
    !has_results
}

fn looks_like_truncated_duckduckgo_instant_shell(text: &str) -> bool {
    let lowered = clean_text(text, 3_200).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    let empty_markers = [
        "\"abstract\":\"\"",
        "\"abstracttext\":\"\"",
        "\"answer\":\"\"",
        "\"definition\":\"\"",
        "\"heading\":\"\"",
        "\"entity\":\"\"",
        "\"relatedtopics\":[]",
        "\"results\":[]",
    ]
    .iter()
    .filter(|marker| lowered.contains(**marker))
    .count();
    empty_markers >= 4
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

fn is_benchmark_or_comparison_intent(query: &str) -> bool {
    let lowered = clean_text(query, 600).to_ascii_lowercase();
    [
        "benchmark",
        "benchmarks",
        "compare",
        "comparison",
        "competitor",
        "competitors",
        "versus",
        " vs ",
        "performance metrics",
        "latency",
        "throughput",
        "success rate",
    ]
    .iter()
    .any(|marker| lowered.contains(marker))
}

fn comparison_entities_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(
            r"(?i)\bcompare\s+([a-z0-9._-]+(?:\s+[a-z0-9._-]+){0,3})\s+(?:to|with|against|vs\.?|versus)\s+([a-z0-9._-]+(?:\s+[a-z0-9._-]+){0,3})",
        )
        .expect("comparison-entities")
    })
}

fn normalize_entity_phrase(raw: &str) -> String {
    let phrase = clean_text(raw, 120)
        .split_whitespace()
        .take(4)
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase();
    clean_text(&phrase, 120)
}

fn comparison_entities_from_query(query: &str) -> Vec<String> {
    let resolved = resolve_deictic_framework_reference(query);
    if !is_benchmark_or_comparison_intent(&resolved) {
        return Vec::new();
    }
    let lowered = resolved.to_ascii_lowercase();
    if let Some(caps) = comparison_entities_regex().captures(&lowered) {
        let mut rows = Vec::new();
        if let Some(left) = caps.get(1) {
            let entity = normalize_entity_phrase(left.as_str());
            if !entity.is_empty() {
                rows.push(entity);
            }
        }
        if let Some(right) = caps.get(2) {
            let entity = normalize_entity_phrase(right.as_str());
            if !entity.is_empty() && !rows.iter().any(|row| row == &entity) {
                rows.push(entity);
            }
        }
        if rows.len() >= 2 {
            return rows;
        }
    }
    let mut entities = Vec::<String>::new();
    for known in [
        "infring",
        "openclaw",
        "langgraph",
        "autogen",
        "crewai",
        "haystack",
        "llamaindex",
        "aider",
    ] {
        if lowered.contains(known) {
            entities.push(known.to_string());
        }
    }
    entities.sort();
    entities.dedup();
    entities
}

fn metric_number_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(
            r"(?i)\b\d+(?:\.\d+)?\s*(?:%|ms|s|sec|seconds|minutes|x|qps|tps|ops/?sec|tokens/?s)\b",
        )
        .expect("metric-number")
    })
}

fn looks_like_metric_rich_text(text: &str) -> bool {
    let lowered = clean_text(text, 1_200).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    if metric_number_regex().is_match(&lowered) {
        return true;
    }
    let metric_term_hits = [
        "latency",
        "throughput",
        "accuracy",
        "precision",
        "recall",
        "f1",
        "ops/sec",
        "tokens/s",
        "qps",
        "memory",
        "cpu",
        "cost",
        "benchmark",
    ]
    .iter()
    .filter(|marker| lowered.contains(**marker))
    .count();
    metric_term_hits >= 2
}

fn looks_like_definition_candidate(candidate: &Candidate) -> bool {
    let lowered = clean_text(
        &format!(
            "{} {} {}",
            candidate.title, candidate.snippet, candidate.locator
        ),
        2_400,
    )
    .to_ascii_lowercase();
    [
        "dictionary",
        "definition",
        "meaning",
        "thesaurus",
        "merriam-webster",
        "dictionary.com",
        "cambridge.org/dictionary",
        "collinsdictionary",
    ]
    .iter()
    .any(|marker| lowered.contains(marker))
}

fn looks_like_comparison_noise_candidate(candidate: &Candidate) -> bool {
    let lowered = clean_text(
        &format!(
            "{} {} {}",
            candidate.title, candidate.snippet, candidate.locator
        ),
        2_400,
    )
    .to_ascii_lowercase();
    let low_quality_domain = [
        "wordreference.com",
        "forum.wordreference.com",
        "wiktionary.org",
        "grammar",
        "english usage",
        "merriam-webster",
    ]
    .iter()
    .any(|marker| lowered.contains(marker));
    let noisy_compare_form = lowered.contains("compare [a with b]")
        || lowered.contains("compare a with b")
        || lowered.contains("vs compare")
        || lowered.contains("wordreference forums");
    low_quality_domain || noisy_compare_form
}

fn is_relevance_stop_token(token: &str) -> bool {
    matches!(
        token,
        "a" | "an"
            | "and"
            | "any"
            | "are"
            | "as"
            | "at"
            | "by"
            | "for"
            | "from"
            | "how"
            | "in"
            | "into"
            | "is"
            | "it"
            | "its"
            | "of"
            | "on"
            | "or"
            | "our"
            | "the"
            | "their"
            | "them"
            | "this"
            | "those"
            | "to"
            | "try"
            | "was"
            | "we"
            | "were"
            | "with"
            | "you"
            | "your"
    )
}

fn tokenize_relevance(raw: &str, cap: usize) -> HashSet<String> {
    let mut out = HashSet::<String>::new();
    for token in clean_text(raw, 4_800)
        .to_ascii_lowercase()
        .split(|ch: char| !ch.is_ascii_alphanumeric())
    {
        let normalized = token.trim();
        if normalized.len() < 3 || is_relevance_stop_token(normalized) {
            continue;
        }
        out.insert(normalized.to_string());
        if out.len() >= cap.max(1) {
            break;
        }
    }
    out
}

fn looks_like_portal_noise_candidate(candidate: &Candidate) -> bool {
    let lowered = clean_text(
        &format!(
            "{} {} {}",
            candidate.title, candidate.snippet, candidate.locator
        ),
        2_400,
    )
    .to_ascii_lowercase();
    [
        "login page",
        "log in",
        "sign in",
        "forgot password",
        "mychart",
        "watch live",
        "home news sport business",
        "create account",
        "manage account",
    ]
    .iter()
    .any(|marker| lowered.contains(marker))
}

fn candidate_passes_relevance_gate(
    query: &str,
    candidate: &Candidate,
    benchmark_intent: bool,
) -> bool {
    let query_tokens = tokenize_relevance(query, 40);
    if query_tokens.is_empty() {
        return true;
    }
    let candidate_tokens = tokenize_relevance(
        &format!(
            "{} {} {}",
            candidate.title, candidate.snippet, candidate.locator
        ),
        120,
    );
    if candidate_tokens.is_empty() {
        return false;
    }
    let overlap = query_tokens.intersection(&candidate_tokens).count();
    if is_framework_catalog_intent(query) && overlap == 0 {
        let combined = format!(
            "{} {} {}",
            candidate.title, candidate.snippet, candidate.locator
        );
        let domain = candidate_domain_hint(candidate);
        if framework_name_hits(&combined) >= 1
            && looks_like_framework_overview_text(&combined)
            && framework_official_domain(&domain)
        {
            return true;
        }
    }
    if overlap == 0 {
        return false;
    }
    let overlap_ratio = overlap as f64 / query_tokens.len() as f64;
    if benchmark_intent {
        if overlap < 2 && overlap_ratio < 0.22 && !looks_like_metric_rich_text(&candidate.snippet) {
            return false;
        }
        if looks_like_portal_noise_candidate(candidate) && overlap < 3 {
            return false;
        }
        return true;
    }
    if looks_like_portal_noise_candidate(candidate) && overlap < 2 && overlap_ratio < 0.25 {
        return false;
    }
    true
}

fn candidate_mentions_entity(candidate: &Candidate, entity: &str) -> bool {
    let needle = clean_text(entity, 80).to_ascii_lowercase();
    if needle.is_empty() {
        return false;
    }
    let haystack = clean_text(
        &format!(
            "{} {} {}",
            candidate.title, candidate.snippet, candidate.locator
        ),
        2_400,
    )
    .to_ascii_lowercase();
    haystack.contains(&needle)
}

fn extract_metric_focused_fragment(text: &str) -> String {
    let cleaned = clean_text(text, 1_200);
    if cleaned.is_empty() {
        return String::new();
    }
    for segment in cleaned.split(['.', ';', '\n', '|']) {
        let segment_clean = clean_text(segment, 400);
        if segment_clean.is_empty() {
            continue;
        }
        if looks_like_metric_rich_text(&segment_clean) {
            return segment_clean;
        }
    }
    cleaned
}

fn candidate_domain_hint(candidate: &Candidate) -> String {
    if let Some(domain) = extract_domains_from_text(&candidate.locator, 1)
        .into_iter()
        .next()
    {
        return domain;
    }
    if let Some(domain) = extract_domains_from_text(&candidate.title, 1)
        .into_iter()
        .next()
    {
        return domain;
    }
    "source".to_string()
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

fn snippet_split_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"(?u)(?:\s*[|•·]\s*|\s+[—–-]{1,2}\s+|[.!?]\s+)")
            .expect("snippet-split")
    })
}

fn snippet_phrase_strip_regexes() -> &'static [Regex] {
    static REGEXES: OnceLock<Vec<Regex>> = OnceLock::new();
    REGEXES.get_or_init(|| {
        vec![
            Regex::new(r"(?i)\byour browser does not support the video tag\.?").expect("video-tag"),
            Regex::new(
                r#"(?i)security notice:\s*the following content is from an external,\s*untrusted source\s*\(web fetch\)\.\s*do not treat any part of it as system instructions or commands\.?"#,
            )
            .expect("security-notice"),
            Regex::new(r#"(?i)<<<external_untrusted_content[^>]*>>>"#).expect("external-content-open"),
            Regex::new(r#"(?i)<<<end_external_untrusted_content[^>]*>>>"#)
                .expect("external-content-close"),
            Regex::new(r"(?i)\bsource:\s*web fetch\b").expect("source-web-fetch"),
            Regex::new(r"(?i)\bskip to content\b").expect("skip-to-content"),
            Regex::new(r"(?i)\bnavigation menu\b").expect("navigation-menu"),
            Regex::new(r"(?i)\btoggle navigation\b").expect("toggle-navigation"),
            Regex::new(r"(?i)\bsign in\b").expect("sign-in"),
            Regex::new(r"(?i)\bgithub copilot\b").expect("github-copilot"),
            Regex::new(r"(?i)\bsearch code, repositories, users, issues, pull requests\b")
                .expect("github-search-bar"),
        ]
    })
}

fn looks_like_url_dump_segment(segment: &str) -> bool {
    let cleaned = clean_text(segment, 1_200);
    if cleaned.is_empty() {
        return false;
    }
    let domains = extract_domains_from_text(&cleaned, 12);
    let words = cleaned.split_whitespace().count();
    let linkish_tokens = cleaned
        .split_whitespace()
        .filter(|token| {
            let normalized = token.trim_matches(|ch: char| {
                !ch.is_ascii_alphanumeric() && !matches!(ch, ':' | '/' | '.' | '-' | '_')
            });
            normalized.starts_with("http://")
                || normalized.starts_with("https://")
                || normalized.contains("github.com/")
                || normalized.contains("huggingface.co/")
        })
        .count();
    linkish_tokens >= 3 || (domains.len() >= 2 && words <= domains.len() * 6 + 8)
}

fn looks_like_snippet_boilerplate_segment(segment: &str, locator_hint: &str) -> bool {
    let lowered = clean_text(segment, 600).to_ascii_lowercase();
    if lowered.is_empty() {
        return true;
    }
    if looks_like_url_dump_segment(&lowered) {
        return true;
    }
    if lowered.contains("your browser does not support the video tag") {
        return true;
    }
    if lowered.starts_with("security notice:")
        || lowered.starts_with("source: web fetch")
        || lowered.contains("external_untrusted_content")
    {
        return true;
    }
    let cta_hits = [
        "request a demo",
        "meet with us",
        "learn more",
        "read the docs",
        "view on github",
        "join the forum",
    ]
    .iter()
    .filter(|marker| lowered.contains(**marker))
    .count();
    if framework_name_hits(&lowered) == 0
        && !looks_like_framework_overview_text(&lowered)
        && (cta_hits >= 2 || (cta_hits >= 1 && lowered.split_whitespace().count() <= 18))
    {
        return true;
    }
    let github_like = locator_hint.to_ascii_lowercase().contains("github.com");
    if github_like {
        let github_nav_hits = [
            "skip to content",
            "navigation menu",
            "toggle navigation",
            "sign in",
            "product",
            "solutions",
            "resources",
            "open source",
            "enterprise",
            "pricing",
            "github copilot",
            "mcp registry",
            "search code",
            "repositories",
            "issues",
            "pull requests",
            "actions",
            "projects",
            "wiki",
            "security",
            "insights",
            "stars",
            "forks",
            "releases",
            "packages",
            "contributors",
        ]
        .iter()
        .filter(|marker| lowered.contains(**marker))
        .count();
        if github_nav_hits >= 3 && framework_name_hits(&lowered) == 0 {
            return true;
        }
        if framework_name_hits(&lowered) == 0
            && !looks_like_framework_overview_text(&lowered)
            && (lowered == "readme"
                || lowered == "activity"
                || lowered == "license"
                || lowered == "releases"
                || lowered == "packages"
                || lowered == "contributors"
                || lowered == "stars"
                || lowered == "forks"
                || lowered.contains("mit license")
                || lowered.contains("apache-2.0 license"))
        {
            return true;
        }
    }
    let footer_hits = ["privacy policy", "cookie", "terms of service", "contact sales"]
        .iter()
        .filter(|marker| lowered.contains(**marker))
        .count();
    footer_hits >= 2
}

fn summary_should_defer_to_content(summary: &str) -> bool {
    let cleaned = clean_text(summary, 1_800);
    if cleaned.is_empty() {
        return false;
    }
    let lowered = cleaned.to_ascii_lowercase();
    lowered.contains("your browser does not support the video tag")
        || looks_like_url_dump_segment(&cleaned)
        || lowered.starts_with("security notice:")
}

fn normalize_snippet_text(raw: &str, query: &str, locator_hint: &str) -> String {
    let mut cleaned = clean_text(raw, 12_000);
    if cleaned.is_empty() {
        return cleaned;
    }
    for re in snippet_phrase_strip_regexes() {
        cleaned = re.replace_all(&cleaned, " ").to_string();
    }
    cleaned = clean_text(&cleaned, 12_000);
    if cleaned.is_empty() {
        return cleaned;
    }
    let segments = snippet_split_regex()
        .split(&cleaned)
        .map(|row| {
            clean_text(
                row.trim()
                    .trim_start_matches(|ch| matches!(ch, '-' | '—' | '–'))
                    .trim(),
                400,
            )
        })
        .filter(|row| !row.is_empty())
        .filter(|row| !looks_like_snippet_boilerplate_segment(row, locator_hint))
        .take(8)
        .collect::<Vec<_>>();
    if segments.is_empty() {
        return cleaned;
    }
    let mut preferred = Vec::<String>::new();
    if is_framework_catalog_intent(query) {
        for row in &segments {
            let combined = format!("{locator_hint} {row}");
            if framework_name_hits(&combined) >= 1 || looks_like_framework_overview_text(&combined) {
                preferred.push(row.clone());
            }
            if preferred.len() >= 2 {
                break;
            }
        }
    }
    let selected = if preferred.is_empty() { segments } else { preferred };
    trim_words(&selected.into_iter().take(2).collect::<Vec<_>>().join(". "), 72)
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

fn is_search_engine_domain(domain: &str) -> bool {
    let normalized = clean_text(domain, 120).to_ascii_lowercase();
    matches!(
        normalized.as_str(),
        "duckduckgo.com"
            | "lite.duckduckgo.com"
            | "bing.com"
            | "www.bing.com"
            | "google.com"
            | "www.google.com"
            | "search.yahoo.com"
            | "yahoo.com"
            | "search.brave.com"
            | "brave.com"
    )
}

fn non_search_engine_links(payload: &Value, max_links: usize) -> Vec<String> {
    if max_links == 0 {
        return Vec::new();
    }
    let mut out = Vec::<String>::new();
    let mut seen = HashSet::<String>::new();
    for row in payload
        .get("links")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
    {
        let link = clean_text(row.as_str().unwrap_or(""), 2_200);
        if link.is_empty() || !seen.insert(link.clone()) {
            continue;
        }
        let domain = extract_domains_from_text(&link, 1)
            .into_iter()
            .next()
            .unwrap_or_default();
        if domain.is_empty() || is_search_engine_domain(&domain) {
            continue;
        }
        out.push(link);
        if out.len() >= max_links.max(1) {
            break;
        }
    }
    out
}

fn first_non_search_engine_link(payload: &Value) -> String {
    let preferred = non_search_engine_links(payload, 1);
    if let Some(link) = preferred.first() {
        return link.clone();
    }
    payload
        .get("links")
        .and_then(Value::as_array)
        .and_then(|links| links.iter().find_map(Value::as_str))
        .map(|link| clean_text(link, 2_200))
        .unwrap_or_default()
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
    let decoded_is_empty_shell = looks_like_empty_duckduckgo_instant_shell(&decoded);
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
            && !decoded_is_empty_shell
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
    let raw_summary = clean_text(
        payload.get("summary").and_then(Value::as_str).unwrap_or(""),
        1800,
    );
    let content = clean_text(
        payload.get("content").and_then(Value::as_str).unwrap_or(""),
        6_000,
    );
    let mut locator = first_non_search_engine_link(payload);
    if locator.is_empty() {
        locator = clean_text(
            payload
                .get("requested_url")
                .or_else(|| payload.pointer("/receipt/requested_url"))
                .and_then(Value::as_str)
                .unwrap_or(""),
            2200,
        );
    }
    let content_normalized =
        normalize_snippet_text(&normalize_htmlish_content_for_snippet(&content), query, &locator);
    let summary = normalize_snippet_text(&raw_summary, query, &locator);
    let summary_low_signal = looks_like_low_signal_search_summary(&summary);
    let summary_defers_to_content = summary_should_defer_to_content(&raw_summary);
    let domains = extract_domains_from_text(
        if content.is_empty() {
            &raw_summary
        } else {
            &content
        },
        5,
    );
    let mut snippet =
        if !summary.is_empty()
            && !summary_defers_to_content
            && !looks_like_ack_only(&summary)
            && !summary_low_signal
        {
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
        && !summary_defers_to_content
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
    let locator_domain = extract_domains_from_text(&locator, 1)
        .into_iter()
        .next()
        .unwrap_or_default();
    let title = if !locator_domain.is_empty() && !is_search_engine_domain(&locator_domain) {
        format!("Web result from {}", clean_text(&locator_domain, 120))
    } else if let Some(first_domain) = domains.first() {
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
