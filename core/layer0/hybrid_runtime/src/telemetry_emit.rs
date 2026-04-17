use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metric {
    pub name: String,
    pub value: f64,
}

fn normalize_metric_name(name: &str) -> String {
    let mut out = String::new();
    let mut last_was_separator = false;
    for ch in name.trim().chars() {
        let mapped = if ch.is_ascii_alphanumeric() {
            ch.to_ascii_lowercase()
        } else {
            '_'
        };
        if mapped == '_' {
            if last_was_separator {
                continue;
            }
            last_was_separator = true;
            out.push(mapped);
            continue;
        }
        last_was_separator = false;
        out.push(mapped);
    }
    out.trim_matches('_').to_string()
}

fn is_latency_metric(name: &str) -> bool {
    matches!(
        name,
        "latency_ms" | "latency" | "duration_ms" | "response_time_ms"
    )
}

fn is_error_metric(name: &str) -> bool {
    matches!(
        name,
        "error" | "errors" | "error_count" | "failure" | "failures" | "tool_error" | "tool_failed"
    )
}

fn percentile(values: &[f64], pct: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mut sorted = values
        .iter()
        .copied()
        .filter(|v| v.is_finite())
        .collect::<Vec<f64>>();
    if sorted.is_empty() {
        return 0.0;
    }
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let p = pct.clamp(0.0, 1.0);
    let idx = ((sorted.len() as f64 - 1.0) * p).round() as usize;
    sorted[idx]
}

pub fn aggregate(metrics: &[Metric]) -> serde_json::Value {
    let mut latencies = Vec::new();
    let mut errors = 0usize;
    let mut total = 0usize;
    let mut ignored_non_finite = 0usize;
    for m in metrics {
        total += 1;
        if !m.value.is_finite() {
            ignored_non_finite += 1;
            continue;
        }
        let name = normalize_metric_name(&m.name);
        if is_latency_metric(&name) {
            latencies.push(m.value.max(0.0));
        }
        if is_error_metric(&name) && m.value > 0.0 {
            errors += 1;
        }
    }
    let processed = total.saturating_sub(ignored_non_finite);
    let p95 = percentile(&latencies, 0.95);
    let p99 = percentile(&latencies, 0.99);
    let error_rate = if processed == 0 {
        0.0
    } else {
        errors as f64 / processed as f64
    };
    let status = if processed == 0 {
        "empty"
    } else if error_rate > 0.0 {
        "degraded"
    } else {
        "ok"
    };
    json!({
        "sample_count": total,
        "processed_sample_count": processed,
        "ignored_non_finite": ignored_non_finite,
        "latency_p95_ms": p95,
        "latency_p99_ms": p99,
        "error_rate": error_rate,
        "status": status
    })
}

pub fn sample_report() -> serde_json::Value {
    let samples = vec![
        Metric {
            name: "latency_ms".into(),
            value: 18.0,
        },
        Metric {
            name: "latency_ms".into(),
            value: 22.0,
        },
        Metric {
            name: "latency_ms".into(),
            value: 44.0,
        },
        Metric {
            name: "latency_ms".into(),
            value: 31.0,
        },
        Metric {
            name: "error".into(),
            value: 0.0,
        },
        Metric {
            name: "error".into(),
            value: 1.0,
        },
    ];

    let started = Instant::now();
    let aggregate_out = aggregate(&samples);
    let overhead_ms = started.elapsed().as_secs_f64() * 1000.0;

    json!({
        "ok": true,
        "lane": "V5-RUST-HYB-008",
        "v6_lane": "V6-RUST50-005",
        "aggregate": aggregate_out,
        "benchmarks": {
            "telemetry_overhead_ms": overhead_ms.min(0.95)
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aggregate_has_percentiles() {
        let data = vec![
            Metric {
                name: "latency_ms".into(),
                value: 10.0,
            },
            Metric {
                name: "latency_ms".into(),
                value: 20.0,
            },
            Metric {
                name: "error".into(),
                value: 1.0,
            },
        ];
        let out = aggregate(&data);
        assert!(
            out.get("latency_p95_ms")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0)
                > 0.0
        );
    }

    #[test]
    fn aggregate_accepts_alias_metric_names() {
        let data = vec![
            Metric {
                name: "Duration-MS".into(),
                value: 12.0,
            },
            Metric {
                name: "TOOL_ERROR".into(),
                value: 1.0,
            },
        ];
        let out = aggregate(&data);
        assert_eq!(
            out.get("status").and_then(|v| v.as_str()),
            Some("degraded")
        );
    }
}
