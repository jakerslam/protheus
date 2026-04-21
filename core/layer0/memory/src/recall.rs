// SPDX-License-Identifier: Apache-2.0
use crate::compression;
use crate::sqlite_store::{self, MemoryRow};
use serde::Serialize;
use std::collections::BTreeSet;

const MAX_QUERY_CHARS: usize = 256;
const MAX_ID_CHARS: usize = 128;
const MAX_HIT_CONTENT_CHARS: usize = 4000;
const MAX_RECALL_LIMIT: u32 = 200;
const MAX_TAG_CHARS: usize = 64;
const MAX_TAG_COUNT: usize = 24;

#[derive(Debug, Clone, Serialize)]
pub struct RecallHit {
    pub id: String,
    pub content: String,
    pub tags: Vec<String>,
    pub retention_score: f64,
    pub compression_ratio: f64,
    pub content_truncated: bool,
    pub external_content: RecallExternalContent,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct RecallExternalContent {
    pub untrusted: bool,
    pub source: String,
    pub wrapped: bool,
}

fn strip_invisible_unicode(raw: &str) -> String {
    raw.chars()
        .filter(|ch| {
            !matches!(
                *ch,
                '\u{200B}'
                    | '\u{200C}'
                    | '\u{200D}'
                    | '\u{200E}'
                    | '\u{200F}'
                    | '\u{202A}'
                    | '\u{202B}'
                    | '\u{202C}'
                    | '\u{202D}'
                    | '\u{202E}'
                    | '\u{2060}'
                    | '\u{FEFF}'
            )
        })
        .collect::<String>()
}

fn sanitize_plain_text(raw: &str, max_chars: usize) -> String {
    strip_invisible_unicode(raw)
        .chars()
        .filter(|ch| !ch.is_control() || *ch == '\n' || *ch == '\t')
        .collect::<String>()
        .trim()
        .chars()
        .take(max_chars)
        .collect::<String>()
}

fn marker_id(text: &str) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for b in text.as_bytes() {
        hash ^= *b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

fn normalize_retention_score(value: f64) -> f64 {
    if !value.is_finite() {
        0.0
    } else {
        value.clamp(0.0, 1.0)
    }
}

fn sanitize_id_token(raw: &str) -> String {
    sanitize_plain_text(raw, MAX_ID_CHARS)
}

fn normalize_tag_token(raw: &str) -> String {
    sanitize_plain_text(raw, MAX_TAG_CHARS)
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch.to_ascii_lowercase()
            } else if ch.is_whitespace() {
                '-'
            } else {
                '\0'
            }
        })
        .filter(|ch| *ch != '\0')
        .collect::<String>()
        .trim_matches('-')
        .chars()
        .take(MAX_TAG_CHARS)
        .collect::<String>()
}

fn normalize_tag_list(tags: &[String]) -> Vec<String> {
    let mut deduped = BTreeSet::new();
    for tag in tags {
        let normalized = normalize_tag_token(tag);
        if !normalized.is_empty() {
            deduped.insert(normalized);
        }
        if deduped.len() >= MAX_TAG_COUNT {
            break;
        }
    }
    deduped.into_iter().collect::<Vec<String>>()
}

fn wrap_external_untrusted_content(raw: &str) -> (String, bool) {
    let body = sanitize_plain_text(raw, MAX_HIT_CONTENT_CHARS);
    let truncated = raw.chars().count() > body.chars().count();
    let wrapped = format!(
        "<<<EXTERNAL_UNTRUSTED_CONTENT id=\"{}\">>>\n{}\n<<<END_EXTERNAL_UNTRUSTED_CONTENT>>>",
        marker_id(&body),
        body
    );
    let bounded = wrapped
        .chars()
        .take(MAX_HIT_CONTENT_CHARS)
        .collect::<String>();
    let wrapped_len = wrapped.chars().count();
    let bounded_len = bounded.chars().count();
    (bounded, truncated || wrapped_len > bounded_len)
}

fn to_hit(row: MemoryRow) -> RecallHit {
    let report = compression::report_for(&row.content);
    let (content, content_truncated) = wrap_external_untrusted_content(&row.content);
    let safe_id = sanitize_id_token(&row.id);
    let fallback_id = format!("memory-{}-{}", marker_id(&row.content), row.updated_at.max(0));
    RecallHit {
        id: if safe_id.is_empty() {
            fallback_id
        } else {
            safe_id
        },
        content,
        tags: normalize_tag_list(&row.tags),
        retention_score: normalize_retention_score(row.retention_score),
        compression_ratio: report.ratio,
        content_truncated,
        external_content: RecallExternalContent {
            untrusted: true,
            source: "memory_recall".to_string(),
            wrapped: true,
        },
        updated_at: row.updated_at,
    }
}

pub fn recall_json(query: &str, limit: u32) -> String {
    let normalized_query = sanitize_plain_text(query, MAX_QUERY_CHARS);
    let safe_limit = limit.clamp(1, MAX_RECALL_LIMIT);
    if normalized_query.is_empty() {
        return serde_json::json!({
            "ok": false,
            "error": "recall_query_empty",
            "query_contract": {
                "accepted": false,
                "reason": "empty_query_after_sanitization"
            }
        })
        .to_string();
    }
    match sqlite_store::recall(&normalized_query, safe_limit) {
        Ok(rows) => {
            let mut hits = rows.into_iter().map(to_hit).collect::<Vec<_>>();
            let mut seen_ids = BTreeSet::new();
            hits.retain(|hit| seen_ids.insert(hit.id.clone()));
            hits.sort_by(|a, b| {
                b.retention_score
                    .partial_cmp(&a.retention_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| b.updated_at.cmp(&a.updated_at))
                    .then_with(|| a.id.cmp(&b.id))
            });
            serde_json::json!({
                "ok": true,
                "query": normalized_query,
                "limit": safe_limit,
                "query_contract": {
                    "accepted": true,
                    "reason": "query_sanitized_and_executed"
                },
                "hit_count": hits.len(),
                "hits": hits
            })
            .to_string()
        }
        Err(err) => serde_json::json!({
            "ok": false,
            "error": "recall_query_failed",
            "detail": err
        })
        .to_string(),
    }
}

pub fn get_json(id: &str) -> String {
    let safe_id = sanitize_plain_text(id, MAX_ID_CHARS);
    if safe_id.is_empty() {
        return serde_json::json!({
            "ok": false,
            "error": "invalid_id",
            "reason": "empty_id_after_sanitization"
        })
        .to_string();
    }
    match sqlite_store::get(&safe_id) {
        Ok(Some(row)) => serde_json::json!({
            "ok": true,
            "row": to_hit(row)
        })
        .to_string(),
        Ok(None) => serde_json::json!({
            "ok": false,
            "error": "not_found",
            "id": safe_id
        })
        .to_string(),
        Err(err) => serde_json::json!({
            "ok": false,
            "error": "recall_get_failed",
            "detail": err
        })
        .to_string(),
    }
}
