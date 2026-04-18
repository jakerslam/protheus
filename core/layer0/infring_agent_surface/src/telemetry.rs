use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReceiptEvent {
    pub event_id: String,
    pub status: String,
    pub duration_ms: u64,
    pub error_code: Option<String>,
    pub timestamp_ms: i64,
    pub attributes: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReceiptSpan {
    pub trace_id: String,
    pub agent_name: String,
    pub started_at_ms: i64,
    pub events: Vec<ReceiptEvent>,
    pub attributes: BTreeMap<String, String>,
}

pub struct ReceiptTraceSink;

impl ReceiptTraceSink {
    pub fn to_jsonl(spans: &[ReceiptSpan]) -> String {
        let mut lines = Vec::<String>::new();
        for span in spans {
            if let Ok(encoded) = serde_json::to_string(span) {
                lines.push(encoded);
            }
        }
        lines.join("\n")
    }

    pub fn to_otel_like(spans: &[ReceiptSpan]) -> Value {
        let mut scope_spans = Vec::<Value>::new();
        for span in spans {
            let events = span
                .events
                .iter()
                .map(|event| {
                    json!({
                        "name": event.event_id,
                        "timeUnixNano": (event.timestamp_ms.max(0) as u64) * 1_000_000,
                        "attributes": event.attributes.iter().map(|(key, value)| json!({"key": key, "value": {"stringValue": value}})).collect::<Vec<_>>(),
                    })
                })
                .collect::<Vec<_>>();
            scope_spans.push(json!({
                "traceId": span.trace_id,
                "name": span.agent_name,
                "startTimeUnixNano": (span.started_at_ms.max(0) as u64) * 1_000_000,
                "events": events,
            }));
        }
        json!({
            "resourceSpans": [{
                "resource": {
                    "attributes": [
                        {"key": "service.name", "value": {"stringValue": "infring-agent-surface"}},
                        {"key": "generated_at_ms", "value": {"intValue": Utc::now().timestamp_millis()}}
                    ]
                },
                "scopeSpans": [{
                    "scope": {"name": "receipt-trace"},
                    "spans": scope_spans
                }]
            }]
        })
    }
}

pub struct ReceiptVisualizer;

impl ReceiptVisualizer {
    pub fn render_compact(span: &ReceiptSpan) -> String {
        let mut out = Vec::<String>::new();
        out.push(format!("trace={} agent={}", span.trace_id, span.agent_name));
        for event in &span.events {
            out.push(format!(
                "- {} status={} duration_ms={} error={}",
                event.event_id,
                event.status,
                event.duration_ms,
                event.error_code.clone().unwrap_or_default()
            ));
        }
        out.join("\n")
    }

    pub fn render_markdown_table(span: &ReceiptSpan) -> String {
        let mut out = String::new();
        out.push_str("| event | status | duration_ms | error |\n");
        out.push_str("| --- | --- | --- | --- |\n");
        for event in &span.events {
            out.push_str(&format!(
                "| {} | {} | {} | {} |\n",
                event.event_id,
                event.status,
                event.duration_ms,
                event.error_code.clone().unwrap_or_default()
            ));
        }
        out
    }

    pub fn summarize(spans: &[ReceiptSpan]) -> Value {
        let mut event_count = 0usize;
        let mut errors = 0usize;
        for span in spans {
            event_count += span.events.len();
            errors += span
                .events
                .iter()
                .filter(|event| event.status.eq_ignore_ascii_case("error"))
                .count();
        }
        json!({
            "trace_count": spans.len(),
            "event_count": event_count,
            "error_count": errors,
        })
    }
}

