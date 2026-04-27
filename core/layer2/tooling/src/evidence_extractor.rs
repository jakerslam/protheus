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
        let mut array_payload_seen = collect_array_cards(self, result, raw_payload, &mut cards);
        if let Some(nested) = raw_payload
            .get("data")
            .or_else(|| raw_payload.get("payload"))
        {
            if collect_array_cards(self, result, nested, &mut cards) {
                array_payload_seen = true;
            }
        }
        if !array_payload_seen {
            if let Some(card) = self.build_card(result, raw_payload, None) {
                cards.push(card);
            }
        }
        if cards.is_empty() && raw_payload.get("message").is_some() {
            if let Some(card) = self.build_card(result, raw_payload, None) {
                cards.push(card);
            }
        }
        let mut seen = HashSet::<String>::new();
        cards
            .into_iter()
            .filter(|card| {
                !card.excerpt.is_empty()
                    && !card.summary.is_empty()
                    && seen.insert(card.dedupe_hash.clone())
            })
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
        if looks_like_interface_chrome(&excerpt) && looks_like_interface_chrome(&summary) {
            return None;
        }
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
        let extracted_at = now_ms();
        let evidence_content_id = deterministic_hash(&serde_json::json!({
            "kind": "evidence_card_content",
            "derived_from_result_id": result.result_id,
            "source_ref": source_ref,
            "source_location": source_location,
            "dedupe_hash": dedupe_hash,
        }));
        let evidence_event_id = deterministic_hash(&serde_json::json!({
            "kind": "evidence_card_event",
            "trace_id": result.trace_id,
            "task_id": result.task_id,
            "result_event_id": result.result_event_id,
            "evidence_content_id": evidence_content_id,
            "source_location": source_location
        }));
        let evidence_id = evidence_content_id.clone();
        Some(EvidenceCard {
            evidence_id,
            evidence_content_id,
            evidence_event_id,
            trace_id: result.trace_id.clone(),
            task_id: result.task_id.clone(),
            derived_from_result_id: result.result_id.clone(),
            source_ref,
            source_location,
            excerpt,
            summary,
            confidence_vector,
            dedupe_hash,
            lineage: result.lineage.clone(),
            timestamp: extracted_at,
        })
    }
}

fn collect_array_cards(
    extractor: &EvidenceExtractor,
    result: &NormalizedToolResult,
    payload: &Value,
    cards: &mut Vec<EvidenceCard>,
) -> bool {
    let mut seen = false;
    for key in [
        "results",
        "items",
        "matches",
        "files",
        "search_results",
        "hits",
        "documents",
    ] {
        if let Some(rows) = payload.get(key).and_then(Value::as_array) {
            seen = true;
            for (idx, row) in rows.iter().enumerate() {
                if let Some(card) = extractor.build_card(result, row, Some(idx)) {
                    cards.push(card);
                }
            }
        }
    }
    seen
}

fn clean_text(raw: &str, max_len: usize) -> String {
    strip_markup_noise(raw)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .chars()
        .take(max_len)
        .collect::<String>()
}

fn strip_markup_noise(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut in_tag = false;
    for ch in raw.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                out.push(' ');
            }
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    out
}

fn looks_like_interface_chrome(text: &str) -> bool {
    let normalized = text.trim().to_ascii_lowercase();
    let provider_metadata_dump = normalized.contains("provider:") && normalized.contains("images:");
    let metadata_card_keyset_dump = (normalized.contains("title:")
        || normalized.contains("excerpt:")
        || normalized.contains("originalurl:")
        || normalized.contains("original_url:"))
        && (normalized.contains("provider:") || normalized.contains("featuredcontent:"));
    let iframe_embed_dump = normalized.contains("iframe") && normalized.contains("allowfullscreen");
    let tool_trace_scaffold = normalized.contains("tool trace complete")
        || normalized.starts_with("<function=")
        || normalized.contains("<function=")
        || normalized.contains("</function>")
        || normalized.contains("input {")
        || normalized.contains("done · 1 blocked")
        || (normalized.contains("file list") && normalized.contains("blocked"));
    let ingress_policy_failure_dump =
        normalized.contains("file_list") && normalized.contains("ingress delivery policy");
    let policy_gate_outage_scaffold =
        normalized.contains("this is a policy gate") && normalized.contains("web-provider outage");
    let workflow_gate_loop = normalized.contains("workflow gate")
        && normalized.contains("final workflow state was unexpected");
    let retry_loop_scaffold = normalized.contains("please retry so i can rerun the chain cleanly");
    let render_failure_scaffold = normalized.contains("final reply did not render")
        || normalized.contains("ask me to continue and i will synthesize");
    let benchmark_instruction_scaffold = normalized.contains("ops:benchmark:refresh")
        || normalized.contains("ops:benchmark:sanity")
        || normalized.contains("ops:benchmark:public-audit")
        || normalized.contains("ops:benchmark:repro")
        || normalized.contains("npm run -s ops:benchmark:");
    let fallback_next_actions_scaffold = normalized.contains("next actions:")
        && normalized.contains("targeted tool call")
        && normalized.contains("concise answer from current context");
    let capability_surface_boilerplate = normalized.contains("i can access runtime telemetry")
        && normalized.contains("persistent memory")
        && normalized.contains("approved command surfaces in this session");
    let route_classification_diagnostic_scaffold = normalized.contains("the first gate")
        && (normalized.contains("workflow_route") || normalized.contains("task_or_info_route"))
        && normalized.contains("classifying this as")
        && normalized.contains("info")
        && normalized.contains("task")
        && (normalized.contains("task classification path")
            || normalized.contains("tool operation request"));
    let tool_routing_diagnostic_scaffold =
        normalized.contains("fundamental misclassification error")
            || normalized.contains("tool routing mechanism is clearly malfunctioning")
            || normalized.contains(
                "requires recalibration to properly distinguish between internal system operations and external data retrieval requests",
            );
    provider_metadata_dump
        || metadata_card_keyset_dump
        || iframe_embed_dump
        || ingress_policy_failure_dump
        || policy_gate_outage_scaffold
        || workflow_gate_loop
        || retry_loop_scaffold
        || render_failure_scaffold
        || benchmark_instruction_scaffold
        || fallback_next_actions_scaffold
        || capability_surface_boilerplate
        || route_classification_diagnostic_scaffold
        || tool_routing_diagnostic_scaffold
        || tool_trace_scaffold
}

fn first_string<'a>(source: &'a Value, keys: &[&str]) -> Option<&'a str> {
    for key in keys {
        if let Some(value) = source.get(*key).and_then(Value::as_str) {
            return Some(value);
        }
    }
    None
}

fn pick_source_ref(source: &Value, result: &NormalizedToolResult) -> String {
    clean_text(
        first_string(
            source,
            &[
                "source_ref",
                "original_url",
                "originalUrl",
                "repository_url",
                "repo_url",
                "url",
                "source",
                "repository",
                "file_path",
                "workspace_path",
                "repo_path",
                "repo",
                "file",
                "path",
            ],
        )
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
    if let Some(text) = first_string(
        source,
        &[
            "excerpt",
            "snippet",
            "content",
            "text",
            "description",
            "message",
            "body",
            "title",
        ],
    ) {
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
    if !summary.is_empty() {
        return summary;
    }
    let title = source
        .get("title")
        .and_then(Value::as_str)
        .map(|v| clean_text(v, 180))
        .unwrap_or_default();
    if !title.is_empty() && !excerpt.is_empty() {
        return clean_text(&format!("{title}: {excerpt}"), 300);
    }
    if title.is_empty() {
        let message = source
            .get("message")
            .and_then(Value::as_str)
            .map(|v| clean_text(v, 240))
            .unwrap_or_default();
        if !message.is_empty() && !excerpt.is_empty() {
            return clean_text(&format!("{message}: {excerpt}"), 300);
        }
        if !message.is_empty() {
            return message;
        }
    }
    let provider = source
        .get("provider")
        .and_then(|value| {
            value
                .get("name")
                .and_then(Value::as_str)
                .or_else(|| value.as_str())
        })
        .map(|v| clean_text(v, 120))
        .unwrap_or_default();
    let language = source
        .get("language")
        .and_then(Value::as_str)
        .map(|v| clean_text(v, 80))
        .unwrap_or_default();
    let platform = source
        .get("platform")
        .and_then(Value::as_str)
        .map(|v| clean_text(v, 80))
        .unwrap_or_default();
    if !provider.is_empty() || !language.is_empty() || !platform.is_empty() {
        let mut facets = Vec::<String>::new();
        if !provider.is_empty() {
            facets.push(format!("provider={provider}"));
        }
        if !language.is_empty() {
            facets.push(format!("language={language}"));
        }
        if !platform.is_empty() {
            facets.push(format!("platform={platform}"));
        }
        if !excerpt.is_empty() {
            return clean_text(&format!("{}: {excerpt}", facets.join(", ")), 300);
        }
        return clean_text(&facets.join(", "), 300);
    }
    clean_text(excerpt, 300)
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
            result_content_id: "r1".to_string(),
            result_event_id: "evt-r1".to_string(),
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
        assert_eq!(cards[0].trace_id, "t1");
        assert_eq!(cards[0].task_id, "task");
    }

    #[test]
    fn extractor_prefers_original_url_and_strips_markup_from_excerpt() {
        let extractor = EvidenceExtractor;
        let raw = serde_json::json!({
            "results": [{
                "originalUrl":"https://example.com/video",
                "excerpt":"<div>hello <b>world</b></div>",
                "summary":"<p>summary text</p>"
            }]
        });
        let cards = extractor.extract(&sample_result(), &raw);
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].source_ref, "https://example.com/video");
        assert_eq!(cards[0].excerpt, "hello world");
        assert_eq!(cards[0].summary, "summary text");
    }

    #[test]
    fn extractor_ignores_tool_trace_scaffold_noise() {
        let extractor = EvidenceExtractor;
        let raw = serde_json::json!({
            "message":"Tool trace complete1 done · 1 blocked"
        });
        let cards = extractor.extract(&sample_result(), &raw);
        assert!(cards.is_empty());
    }

    #[test]
    fn extractor_reads_workspace_items_array() {
        let extractor = EvidenceExtractor;
        let raw = serde_json::json!({
            "items": [
                {"workspace_path":"core/layer2/tooling/src/request_validation.rs", "excerpt":"query synthesis logic"}
            ]
        });
        let cards = extractor.extract(&sample_result(), &raw);
        assert_eq!(cards.len(), 1);
        assert_eq!(
            cards[0].source_ref,
            "core/layer2/tooling/src/request_validation.rs"
        );
    }

    #[test]
    fn extractor_reads_nested_data_search_results_array() {
        let extractor = EvidenceExtractor;
        let raw = serde_json::json!({
            "data": {
                "search_results": [
                    {"source_ref":"workspace://notes", "excerpt":"synthesis candidate"}
                ]
            }
        });
        let cards = extractor.extract(&sample_result(), &raw);
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].source_ref, "workspace://notes");
    }

    #[test]
    fn extractor_does_not_drop_normal_result_summary_text() {
        let extractor = EvidenceExtractor;
        let raw = serde_json::json!({
            "results": [{"summary":"result throughput improved by 20%","source_ref":"https://example.com"}]
        });
        let cards = extractor.extract(&sample_result(), &raw);
        assert_eq!(cards.len(), 1);
    }

    #[test]
    fn extractor_reads_hits_array_payloads() {
        let extractor = EvidenceExtractor;
        let raw = serde_json::json!({
            "hits": [
                {"repository":"https://example.com/repo.git", "message":"hit", "excerpt":"tool route evidence"}
            ]
        });
        let cards = extractor.extract(&sample_result(), &raw);
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].source_ref, "https://example.com/repo.git");
    }

    #[test]
    fn extractor_builds_summary_from_provider_language_and_platform_facets() {
        let extractor = EvidenceExtractor;
        let raw = serde_json::json!({
            "results":[
                {
                    "provider":{"name":"openai"},
                    "language":"rust",
                    "platform":"windows",
                    "excerpt":"route fallback was blocked"
                }
            ]
        });
        let cards = extractor.extract(&sample_result(), &raw);
        assert_eq!(cards.len(), 1);
        assert!(cards[0].summary.contains("provider=openai"));
        assert!(cards[0].summary.contains("language=rust"));
        assert!(cards[0].summary.contains("platform=windows"));
    }
}
