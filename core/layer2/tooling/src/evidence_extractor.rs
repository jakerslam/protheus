use crate::schemas::{ConfidenceVector, EvidenceCard, NormalizedToolResult, NormalizedToolStatus};
use crate::{deterministic_hash, now_ms};
use serde_json::Value;
use std::collections::HashSet;

#[derive(Default)]
pub struct EvidenceExtractor;

impl EvidenceExtractor {
    pub fn extract(&self, result: &NormalizedToolResult, raw_payload: &Value) -> Vec<EvidenceCard> {
        if result.status != NormalizedToolStatus::Ok {
            return Vec::new();
        }
        let mut cards = Vec::<EvidenceCard>::new();
        if let Some(rows) = raw_payload.get("results").and_then(Value::as_array) {
            for (idx, row) in rows.iter().enumerate() {
                if let Some(card) = self.build_card(result, row, Some(idx)) {
                    cards.push(card);
                }
            }
        } else if let Some(card) = self.build_card(result, raw_payload, None) {
            cards.push(card);
        }
        let mut seen = HashSet::<String>::new();
        cards
            .into_iter()
            .filter(|card| seen.insert(card.dedupe_hash.clone()))
            .collect::<Vec<_>>()
    }

    fn build_card(
        &self,
        result: &NormalizedToolResult,
        source: &Value,
        idx: Option<usize>,
    ) -> Option<EvidenceCard> {
        let source_ref = pick_source_ref(source, result);
        let source_location = pick_source_location(source, idx);
        let excerpt = pick_excerpt(source);
        let summary = pick_summary(source, &excerpt);
        if excerpt.is_empty() && summary.is_empty() {
            return None;
        }
        let confidence_vector = pick_confidence(source);
        let dedupe_hash = deterministic_hash(&serde_json::json!({
            "result_id": result.result_id,
            "source_ref": source_ref,
            "excerpt": excerpt,
            "summary": summary
        }));
        let evidence_id = deterministic_hash(&serde_json::json!({
            "kind": "evidence_card",
            "dedupe_hash": dedupe_hash,
            "timestamp": now_ms()
        }));
        Some(EvidenceCard {
            evidence_id,
            derived_from_result_id: result.result_id.clone(),
            source_ref,
            source_location,
            excerpt,
            summary,
            confidence_vector,
            dedupe_hash,
            lineage: result.lineage.clone(),
            timestamp: now_ms(),
        })
    }
}

fn clean_text(raw: &str, max_len: usize) -> String {
    raw.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .chars()
        .take(max_len)
        .collect::<String>()
}

fn pick_source_ref(source: &Value, result: &NormalizedToolResult) -> String {
    clean_text(
        source
            .get("source_ref")
            .or_else(|| source.get("url"))
            .or_else(|| source.get("path"))
            .and_then(Value::as_str)
            .unwrap_or(result.raw_ref.as_str()),
        2000,
    )
}

fn pick_source_location(source: &Value, idx: Option<usize>) -> String {
    let default_location = idx
        .map(|v| format!("results[{v}]"))
        .unwrap_or_else(|| "payload".to_string());
    clean_text(
        source
            .get("source_location")
            .and_then(Value::as_str)
            .unwrap_or(default_location.as_str()),
        400,
    )
}

fn pick_excerpt(source: &Value) -> String {
    if let Some(text) = source
        .get("excerpt")
        .or_else(|| source.get("snippet"))
        .or_else(|| source.get("content"))
        .or_else(|| source.get("text"))
        .and_then(Value::as_str)
    {
        return clean_text(text, 1200);
    }
    if let Some(text) = source.as_str() {
        return clean_text(text, 1200);
    }
    clean_text(&source.to_string(), 1200)
}

fn pick_summary(source: &Value, excerpt: &str) -> String {
    let summary = source
        .get("summary")
        .and_then(Value::as_str)
        .map(|v| clean_text(v, 600))
        .unwrap_or_default();
    if summary.is_empty() {
        clean_text(excerpt, 300)
    } else {
        summary
    }
}

fn pick_confidence(source: &Value) -> ConfidenceVector {
    let fallback = ConfidenceVector {
        relevance: 0.7,
        reliability: 0.65,
        freshness: 0.6,
    };
    let Some(obj) = source.get("confidence_vector").and_then(Value::as_object) else {
        return fallback;
    };
    ConfidenceVector {
        relevance: obj
            .get("relevance")
            .and_then(Value::as_f64)
            .unwrap_or(fallback.relevance),
        reliability: obj
            .get("reliability")
            .and_then(Value::as_f64)
            .unwrap_or(fallback.reliability),
        freshness: obj
            .get("freshness")
            .and_then(Value::as_f64)
            .unwrap_or(fallback.freshness),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schemas::{NormalizedToolMetrics, NormalizedToolStatus};

    fn sample_result() -> NormalizedToolResult {
        NormalizedToolResult {
            result_id: "r1".to_string(),
            trace_id: "t1".to_string(),
            task_id: "task".to_string(),
            tool_name: "web_search".to_string(),
            status: NormalizedToolStatus::Ok,
            normalized_args: serde_json::json!({"query":"test"}),
            dedupe_hash: "d1".to_string(),
            lineage: vec!["l1".to_string()],
            timestamp: 1,
            metrics: NormalizedToolMetrics {
                duration_ms: 1,
                output_bytes: 1,
            },
            raw_ref: "raw://r1".to_string(),
            errors: vec![],
        }
    }

    #[test]
    fn extracts_and_dedupes_evidence_cards() {
        let extractor = EvidenceExtractor;
        let raw = serde_json::json!({
            "results": [
                {"url":"https://a","summary":"A","excerpt":"alpha"},
                {"url":"https://a","summary":"A","excerpt":"alpha"}
            ]
        });
        let cards = extractor.extract(&sample_result(), &raw);
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].derived_from_result_id, "r1");
    }
}
