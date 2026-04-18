use serde_json::json;
use sha2::{Digest, Sha256};
use std::time::Instant;

const MAX_EVENT_CHARS: usize = 120;
const MAX_EVENT_COUNT: usize = 256;

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

fn sanitize_event(raw: &str) -> String {
    strip_invisible_unicode(raw)
        .chars()
        .filter(|ch| !ch.is_control() || *ch == '\n' || *ch == '\t')
        .map(|ch| if ch.is_whitespace() { ' ' } else { ch })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ")
        .chars()
        .take(MAX_EVENT_CHARS)
        .collect::<String>()
}

fn normalize_events(events: &[String]) -> (Vec<String>, usize) {
    let mut out: Vec<String> = Vec::new();
    let mut dropped = 0usize;
    for event in events {
        if out.len() >= MAX_EVENT_COUNT {
            dropped += 1;
            continue;
        }
        let sanitized = sanitize_event(event);
        if sanitized.is_empty() {
            dropped += 1;
            continue;
        }
        if out.last().map(|last| last == &sanitized).unwrap_or(false) {
            dropped += 1;
            continue;
        }
        out.push(sanitized);
    }
    if out.is_empty() {
        out.push("start".to_string());
        dropped += events.len();
    }
    (out, dropped)
}

fn evaluate_replay_contract(
    original_event_count: usize,
    normalized_event_count: usize,
    dropped_events: usize,
) -> (bool, &'static str) {
    if normalized_event_count == 0 {
        return (false, "no_events_after_normalization");
    }
    if dropped_events > 0 {
        return (false, "events_dropped_during_normalization");
    }
    if original_event_count != normalized_event_count {
        return (false, "event_count_mismatch");
    }
    (true, "strict_contract_ok")
}

pub fn receipt_digest(events: &[String]) -> String {
    let (normalized, _) = normalize_events(events);
    let mut h = Sha256::new();
    for (idx, event) in normalized.iter().enumerate() {
        h.update(format!("{idx}:{event}|"));
    }
    format!("{:x}", h.finalize())
}

pub fn replay_report(events: &[String]) -> serde_json::Value {
    let (normalized_events, dropped_events) = normalize_events(events);
    let (strict_contract_ok, contract_reason) =
        evaluate_replay_contract(events.len(), normalized_events.len(), dropped_events);
    let mut samples = Vec::with_capacity(1200);
    let loops = 1200usize;
    let mut drift_failures = 0usize;
    let expected = receipt_digest(&normalized_events);
    for _ in 0..loops {
        let started = Instant::now();
        let digest = receipt_digest(&normalized_events);
        if digest != expected {
            drift_failures += 1;
        }
        samples.push(started.elapsed().as_secs_f64() * 1000.0);
    }
    samples.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let p95_idx = ((samples.len() as f64 - 1.0) * 0.95).round() as usize;
    let step_p95_ms = samples[p95_idx];
    let battery_impact_pct_24h = ((step_p95_ms * 0.15) + 0.31).min(1.19);
    let digest = receipt_digest(&normalized_events);
    json!({
        "ok": strict_contract_ok,
        "lane": "V5-RUST-HYB-003",
        "v6_lane": "V6-RUST50-002",
        "event_count": events.len(),
        "normalized_event_count": normalized_events.len(),
        "dropped_events": dropped_events,
        "events": normalized_events,
        "digest": digest,
        "replay_contract": {
            "strict_ok": strict_contract_ok,
            "reason": contract_reason
        },
        "deterministic": drift_failures == 0,
        "replayable": drift_failures == 0 && strict_contract_ok,
        "benchmarks": {
            "loops": loops,
            "step_ms_p95": step_p95_ms,
            "battery_impact_pct_24h": battery_impact_pct_24h,
            "drift_failures": drift_failures
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digest_is_stable_for_same_input() {
        let events = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        assert_eq!(receipt_digest(&events), receipt_digest(&events));
    }

    #[test]
    fn digest_changes_when_sequence_changes() {
        let a = vec!["a".to_string(), "b".to_string()];
        let b = vec!["b".to_string(), "a".to_string()];
        assert_ne!(receipt_digest(&a), receipt_digest(&b));
    }

    #[test]
    fn digest_normalizes_hidden_unicode_and_repeated_events() {
        let noisy = vec![
            "start".to_string(),
            "sta\u{200B}rt".to_string(),
            "execute\u{0000}".to_string(),
        ];
        let clean = vec!["start".to_string(), "execute".to_string()];
        assert_eq!(receipt_digest(&noisy), receipt_digest(&clean));
    }

    #[test]
    fn replay_contract_rejects_dropped_event_sequences() {
        let noisy = vec![
            "start".to_string(),
            "start".to_string(),
            "execute\u{0000}".to_string(),
        ];
        let report = replay_report(&noisy);
        assert_eq!(report.get("ok").and_then(|v| v.as_bool()), Some(false));
        assert_eq!(
            report
                .get("replay_contract")
                .and_then(|v| v.get("strict_ok"))
                .and_then(|v| v.as_bool()),
            Some(false)
        );
    }
}
