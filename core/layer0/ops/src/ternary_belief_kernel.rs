// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use serde_json::{json, Map, Value};
use std::path::Path;

use crate::contract_lane_utils as lane_utils;
use crate::{deterministic_receipt_hash, now_iso};

const TRIT_PAIN: i64 = -1;
const TRIT_UNKNOWN: i64 = 0;
const TRIT_OK: i64 = 1;

fn usage() {
    println!("ternary-belief-kernel commands:");
    println!("  protheus-ops ternary-belief-kernel evaluate --payload-base64=<json>");
    println!("  protheus-ops ternary-belief-kernel merge --payload-base64=<json>");
    println!("  protheus-ops ternary-belief-kernel serialize --payload-base64=<json>");
}

fn cli_receipt(kind: &str, payload: Value) -> Value {
    let ts = now_iso();
    let ok = payload.get("ok").and_then(Value::as_bool).unwrap_or(true);
    let mut out = json!({
        "ok": ok,
        "type": kind,
        "ts": ts,
        "date": ts[..10].to_string(),
        "payload": payload,
    });
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    out
}

fn cli_error(kind: &str, error: &str) -> Value {
    let ts = now_iso();
    let mut out = json!({
        "ok": false,
        "type": kind,
        "ts": ts,
        "date": ts[..10].to_string(),
        "error": error,
        "fail_closed": true,
    });
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    out
}

fn print_json_line(value: &Value) {
    println!(
        "{}",
        serde_json::to_string(value)
            .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode_failed\"}".to_string())
    );
}

fn payload_json(argv: &[String]) -> Result<Value, String> {
    if let Some(raw) = lane_utils::parse_flag(argv, "payload", false) {
        return serde_json::from_str::<Value>(&raw)
            .map_err(|err| format!("ternary_belief_payload_decode_failed:{err}"));
    }
    if let Some(raw_b64) = lane_utils::parse_flag(argv, "payload-base64", false) {
        let bytes = BASE64_STANDARD
            .decode(raw_b64.as_bytes())
            .map_err(|err| format!("ternary_belief_payload_base64_decode_failed:{err}"))?;
        let text = String::from_utf8(bytes)
            .map_err(|err| format!("ternary_belief_payload_utf8_decode_failed:{err}"))?;
        return serde_json::from_str::<Value>(&text)
            .map_err(|err| format!("ternary_belief_payload_decode_failed:{err}"));
    }
    Ok(json!({}))
}

fn payload_obj<'a>(value: &'a Value) -> &'a Map<String, Value> {
    value.as_object().unwrap_or_else(|| {
        static EMPTY: std::sync::OnceLock<Map<String, Value>> = std::sync::OnceLock::new();
        EMPTY.get_or_init(Map::new)
    })
}

fn clean_text(raw: Option<&Value>, max_len: usize) -> String {
    let input = match raw {
        Some(Value::String(v)) => v.clone(),
        Some(v) => v.to_string(),
        None => String::new(),
    };
    input
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(max_len)
        .collect::<String>()
}

fn normalize_token(raw: Option<&Value>, max_len: usize) -> String {
    let lowered = clean_text(raw, max_len).to_ascii_lowercase();
    let mut out = String::new();
    let mut prev_us = false;
    for ch in lowered.chars() {
        let mapped = if ch.is_ascii_alphanumeric() { ch } else { '_' };
        if mapped == '_' {
            if prev_us || out.is_empty() {
                continue;
            }
            prev_us = true;
            out.push('_');
        } else {
            prev_us = false;
            out.push(mapped);
        }
        if out.len() >= max_len {
            break;
        }
    }
    out.trim_matches('_').to_string()
}

fn as_f64(value: Option<&Value>) -> Option<f64> {
    value.and_then(|v| match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse::<f64>().ok(),
        _ => None,
    })
}

fn clamp_number(value: Option<&Value>, lo: f64, hi: f64, fallback: f64) -> f64 {
    as_f64(value).unwrap_or(fallback).clamp(lo, hi)
}

fn round_to(value: f64, digits: u32) -> f64 {
    let factor = 10_f64.powi(i32::try_from(digits).unwrap_or(4));
    (value * factor).round() / factor
}

fn parse_ts_ms(value: Option<&Value>) -> Option<i64> {
    let text = clean_text(value, 80);
    if text.is_empty() {
        return None;
    }
    chrono::DateTime::parse_from_rfc3339(&text)
        .ok()
        .map(|dt| dt.timestamp_millis())
}

fn normalize_trit(value: Option<&Value>) -> i64 {
    match value {
        Some(Value::Number(n)) => {
            let num = n.as_f64().unwrap_or(0.0);
            if num > 0.0 {
                TRIT_OK
            } else if num < 0.0 {
                TRIT_PAIN
            } else {
                TRIT_UNKNOWN
            }
        }
        Some(Value::Bool(v)) => {
            if *v {
                TRIT_OK
            } else {
                TRIT_PAIN
            }
        }
        _ => {
            let token = normalize_token(value, 64);
            match token.as_str() {
                "ok" | "pass" | "allow" | "approved" | "healthy" | "up" | "true" | "success"
                | "green" | "ready" => TRIT_OK,
                "pain" | "fail" | "failed" | "error" | "blocked" | "deny" | "denied"
                | "critical" | "false" | "down" | "red" => TRIT_PAIN,
                _ => TRIT_UNKNOWN,
            }
        }
    }
}

fn trit_label(trit: i64) -> &'static str {
    match trit {
        TRIT_PAIN => "pain",
        TRIT_OK => "ok",
        _ => "unknown",
    }
}

fn normalize_weight(value: Option<&Value>, fallback: f64) -> f64 {
    let n = as_f64(value).unwrap_or(fallback);
    if !n.is_finite() || n <= 0.0 {
        fallback
    } else {
        n
    }
}

fn normalize_source(value: Option<&Value>, idx: usize) -> String {
    let text = clean_text(value, 120);
    if text.is_empty() {
        format!("signal_{}", idx + 1)
    } else {
        text
    }
}

fn source_trust_value(source_trust: Option<&Value>, source: &str, fallback: f64) -> f64 {
    let Some(map) = source_trust.and_then(Value::as_object) else {
        return fallback;
    };
    let direct = map
        .get(source)
        .or_else(|| map.get(&source.to_ascii_lowercase()));
    if let Some(value) = direct {
        if let Some(n) = as_f64(Some(value)) {
            return n;
        }
        if let Some(obj) = value.as_object() {
            if let Some(n) = as_f64(obj.get("trust")) {
                return n;
            }
            if let Some(n) = as_f64(obj.get("weight")) {
                return n;
            }
        }
    }
    fallback
}

fn signal_freshness_factor(signal_ts_ms: Option<i64>, now_ms: i64, half_life_hours: f64) -> f64 {
    let Some(signal_ms) = signal_ts_ms else {
        return 1.0;
    };
    let age_ms = (now_ms - signal_ms).max(0) as f64;
    let half_life_ms = half_life_hours.max(1.0) * 60.0 * 60.0 * 1000.0;
    let decay_power = age_ms / half_life_ms;
    (0.5_f64.powf(decay_power)).clamp(0.05, 1.0)
}

fn majority_trit(values: &[i64], weights: &[f64], tie_breaker: &str) -> i64 {
    let mut pain = 0.0;
    let mut unknown = 0.0;
    let mut ok = 0.0;
    for (idx, trit) in values.iter().enumerate() {
        let weight = *weights.get(idx).unwrap_or(&1.0);
        match *trit {
            TRIT_PAIN => pain += weight,
            TRIT_OK => ok += weight,
            _ => unknown += weight,
        }
    }
    if pain > ok && pain > unknown {
        return TRIT_PAIN;
    }
    if ok > pain && ok > unknown {
        return TRIT_OK;
    }
    if unknown > pain && unknown > ok {
        return TRIT_UNKNOWN;
    }
    match tie_breaker {
        "pain" => TRIT_PAIN,
        "ok" => TRIT_OK,
        "first_non_zero" => values
            .iter()
            .copied()
            .find(|v| *v != TRIT_UNKNOWN)
            .unwrap_or(TRIT_UNKNOWN),
        _ => TRIT_UNKNOWN,
    }
}

fn consensus_trit(values: &[i64]) -> i64 {
    let non_zero = values
        .iter()
        .copied()
        .filter(|v| *v != TRIT_UNKNOWN)
        .collect::<Vec<_>>();
    if non_zero.is_empty() {
        return TRIT_UNKNOWN;
    }
    let has_pain = non_zero.iter().any(|v| *v == TRIT_PAIN);
    let has_ok = non_zero.iter().any(|v| *v == TRIT_OK);
    if has_pain && has_ok {
        TRIT_UNKNOWN
    } else if has_pain {
        TRIT_PAIN
    } else {
        TRIT_OK
    }
}

fn propagate_trit(parent: i64, child: i64, mode: &str) -> i64 {
    match mode {
        "strict" => {
            if parent == TRIT_PAIN || child == TRIT_PAIN {
                TRIT_PAIN
            } else if parent == TRIT_OK && child == TRIT_OK {
                TRIT_OK
            } else {
                TRIT_UNKNOWN
            }
        }
        "permissive" => {
            if parent == TRIT_OK || child == TRIT_OK {
                TRIT_OK
            } else if parent == TRIT_PAIN && child == TRIT_PAIN {
                TRIT_PAIN
            } else {
                TRIT_UNKNOWN
            }
        }
        _ => {
            if child == TRIT_PAIN {
                TRIT_PAIN
            } else if parent == TRIT_PAIN && child == TRIT_UNKNOWN {
                TRIT_PAIN
            } else if parent == TRIT_OK && child == TRIT_OK {
                TRIT_OK
            } else if parent == TRIT_UNKNOWN && child == TRIT_OK {
                TRIT_OK
            } else {
                TRIT_UNKNOWN
            }
        }
    }
}

fn serialize_trit_vector(values: &[i64]) -> Value {
    let digits = values
        .iter()
        .map(|trit| match *trit {
            TRIT_PAIN => '-',
            TRIT_OK => '+',
            _ => '0',
        })
        .collect::<String>();
    let rows = values
        .iter()
        .map(|trit| match *trit {
            TRIT_PAIN => Value::String("-1".to_string()),
            TRIT_OK => Value::String("1".to_string()),
            _ => Value::String("0".to_string()),
        })
        .collect::<Vec<_>>();
    json!({
        "schema_id": "balanced_trit_vector",
        "schema_version": "1.0.0",
        "encoding": "balanced_ternary_sign",
        "digits": digits,
        "values": rows
    })
}

fn classify_belief_trit(score: f64, positive_threshold: f64, negative_threshold: f64) -> i64 {
    if score >= positive_threshold {
        TRIT_OK
    } else if score <= negative_threshold {
        TRIT_PAIN
    } else {
        TRIT_UNKNOWN
    }
}

fn evaluate(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let signals = payload
        .get("signals")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let opts = payload
        .get("opts")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let label = {
        let v = clean_text(opts.get("label"), 120);
        if v.is_empty() {
            "belief".to_string()
        } else {
            v
        }
    };
    let default_weight = clamp_number(opts.get("default_weight"), 0.0001, 1000.0, 1.0);
    let positive_threshold = clamp_number(opts.get("positive_threshold"), 0.01, 0.99, 0.2);
    let negative_threshold = clamp_number(opts.get("negative_threshold"), -0.99, -0.01, -0.2);
    let evidence_saturation_count =
        clamp_number(opts.get("evidence_saturation_count"), 1.0, 1000.0, 8.0);
    let source_trust_floor = clamp_number(opts.get("source_trust_floor"), 0.01, 10.0, 0.6);
    let source_trust_ceiling = clamp_number(
        opts.get("source_trust_ceiling"),
        source_trust_floor,
        10.0,
        1.5,
    );
    let freshness_half_life_hours = clamp_number(
        opts.get("freshness_half_life_hours"),
        1.0,
        (24 * 365) as f64,
        72.0,
    );
    let min_non_neutral_signals =
        clamp_number(opts.get("min_non_neutral_signals"), 0.0, 1000.0, 1.0);
    let min_non_neutral_weight = clamp_number(opts.get("min_non_neutral_weight"), 0.0, 1000.0, 0.9);
    let min_confidence_for_non_neutral =
        clamp_number(opts.get("min_confidence_for_non_neutral"), 0.0, 1.0, 0.3);
    let force_neutral_on_insufficient_evidence = opts
        .get("force_neutral_on_insufficient_evidence")
        .map(Value::as_bool)
        .flatten()
        .unwrap_or(true);
    let now_ms =
        parse_ts_ms(opts.get("now_iso")).unwrap_or_else(|| chrono::Utc::now().timestamp_millis());

    let duality_signal = crate::duality_seed::invoke(
        root,
        "duality_evaluate",
        Some(&json!({
            "context": {
                "lane": "belief_formation",
                "label": label,
                "source": "ternary_belief_engine",
                "signals": signals.iter().map(|row| json!({
                    "source": row.get("source").cloned().unwrap_or(Value::Null),
                    "trit": row.get("trit").cloned().unwrap_or(Value::Null),
                    "tags": row.get("tags").cloned().unwrap_or(Value::Null)
                })).collect::<Vec<_>>()
            },
            "opts": {
                "lane": "belief_formation",
                "source": "ternary_belief_engine",
                "persist": true
            }
        })),
    )
    .ok();

    let mut normalized = Vec::new();
    let mut pain_weight = 0.0;
    let mut unknown_weight = 0.0;
    let mut ok_weight = 0.0;
    let mut total_weight = 0.0;
    let mut weighted_sum = 0.0;
    let mut non_neutral_count = 0.0;
    let mut non_neutral_weight = 0.0;

    for (idx, row) in signals.iter().enumerate() {
        let source = normalize_source(row.get("source"), idx);
        let mut tags = row
            .get("tags")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|tag| {
                let text = clean_text(Some(&tag), 80);
                if text.is_empty() {
                    None
                } else {
                    Some(Value::String(text))
                }
            })
            .collect::<Vec<_>>();
        let has_trit = row.get("trit").is_some();
        let mut trit = if has_trit {
            normalize_trit(row.get("trit"))
        } else {
            TRIT_UNKNOWN
        };
        if !has_trit && force_neutral_on_insufficient_evidence {
            tags.push(Value::String("missing_trit_neutralized".to_string()));
        }
        let base_weight = normalize_weight(row.get("weight"), default_weight);
        let confidence = clamp_number(row.get("confidence"), 0.0, 1.0, 1.0);
        if trit != TRIT_UNKNOWN && confidence < min_confidence_for_non_neutral {
            trit = TRIT_UNKNOWN;
            tags.push(Value::String("low_confidence_neutralized".to_string()));
        }
        let source_trust = source_trust_value(opts.get("source_trust"), &source, 1.0)
            .clamp(source_trust_floor, source_trust_ceiling);
        let signal_ts_ms = parse_ts_ms(row.get("ts"))
            .or_else(|| {
                row.get("meta")
                    .and_then(|v| v.get("ts"))
                    .and_then(|v| parse_ts_ms(Some(v)))
            })
            .or_else(|| {
                row.get("meta")
                    .and_then(|v| v.get("updated_at"))
                    .and_then(|v| parse_ts_ms(Some(v)))
            });
        let freshness = signal_freshness_factor(signal_ts_ms, now_ms, freshness_half_life_hours);
        let weighted = base_weight * confidence * source_trust * freshness;
        total_weight += weighted;
        weighted_sum += trit as f64 * weighted;
        match trit {
            TRIT_PAIN => pain_weight += weighted,
            TRIT_OK => ok_weight += weighted,
            _ => unknown_weight += weighted,
        }
        if trit != TRIT_UNKNOWN {
            non_neutral_count += 1.0;
            non_neutral_weight += weighted;
        }
        normalized.push(json!({
            "source": source,
            "trit": trit,
            "label": trit_label(trit),
            "weight": round_to(base_weight, 4),
            "confidence": round_to(confidence, 4),
            "source_trust": round_to(source_trust, 4),
            "freshness": round_to(freshness, 4),
            "weighted": round_to(weighted, 4),
            "tags": tags,
            "meta": row.get("meta").cloned().unwrap_or_else(|| json!({}))
        }));
    }

    let score = if total_weight > 0.0 {
        weighted_sum / total_weight
    } else {
        0.0
    };
    let duality_influence = if let Some(signal) = duality_signal.as_ref() {
        if signal.get("enabled").and_then(Value::as_bool) == Some(true) {
            let score_trit = as_f64(signal.get("score_trit")).unwrap_or(0.0);
            let effective_weight = as_f64(signal.get("effective_weight")).unwrap_or(0.0);
            (score_trit * effective_weight * 0.08).clamp(-0.08, 0.08)
        } else {
            0.0
        }
    } else {
        0.0
    };
    let adjusted_score = (score + duality_influence).clamp(-1.0, 1.0);
    let insufficient_evidence =
        non_neutral_count < min_non_neutral_signals || non_neutral_weight < min_non_neutral_weight;
    let trit = if force_neutral_on_insufficient_evidence && insufficient_evidence {
        TRIT_UNKNOWN
    } else {
        classify_belief_trit(adjusted_score, positive_threshold, negative_threshold)
    };
    let trits = normalized
        .iter()
        .map(|row| {
            row.get("trit")
                .and_then(Value::as_i64)
                .unwrap_or(TRIT_UNKNOWN)
        })
        .collect::<Vec<_>>();
    let weights = normalized
        .iter()
        .map(|row| row.get("weighted").and_then(Value::as_f64).unwrap_or(0.0))
        .collect::<Vec<_>>();
    let majority = majority_trit(&trits, &weights, "unknown");
    let consensus = consensus_trit(&trits) == trit && trit != TRIT_UNKNOWN;
    let evidence_coverage = (normalized.len() as f64 / evidence_saturation_count).min(1.0);
    let concentration = if total_weight > 0.0 {
        pain_weight.max(unknown_weight).max(ok_weight) / total_weight
    } else {
        0.0
    };
    let confidence =
        ((adjusted_score.abs() * 0.45) + (concentration * 0.35) + (evidence_coverage * 0.2))
            .min(1.0);

    let mut top_sources = normalized.clone();
    top_sources.sort_by(|a, b| {
        b.get("weighted")
            .and_then(Value::as_f64)
            .unwrap_or(0.0)
            .partial_cmp(&a.get("weighted").and_then(Value::as_f64).unwrap_or(0.0))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    top_sources.truncate(8);
    let top_sources = top_sources
        .into_iter()
        .map(|row| {
            json!({
                "source": row.get("source").cloned().unwrap_or(Value::Null),
                "label": row.get("label").cloned().unwrap_or(Value::Null),
                "trit": row.get("trit").cloned().unwrap_or(Value::Null),
                "weighted": row.get("weighted").cloned().unwrap_or(Value::Null)
            })
        })
        .collect::<Vec<_>>();

    let result = json!({
        "schema_id": "ternary_belief",
        "schema_version": "1.0.0",
        "label": label,
        "trit": trit,
        "trit_label": trit_label(trit),
        "score": round_to(adjusted_score, 4),
        "raw_score": round_to(score, 4),
        "confidence": round_to(confidence, 4),
        "consensus": consensus,
        "majority_trit": majority,
        "majority_label": trit_label(majority),
        "evidence_count": normalized.len(),
        "total_weight": round_to(total_weight, 4),
        "support": {
            "pain_weight": round_to(pain_weight, 4),
            "unknown_weight": round_to(unknown_weight, 4),
            "ok_weight": round_to(ok_weight, 4)
        },
        "thresholds": {
            "positive": round_to(positive_threshold, 4),
            "negative": round_to(negative_threshold, 4)
        },
        "evidence_guard": {
            "force_neutral_on_insufficient_evidence": force_neutral_on_insufficient_evidence,
            "min_non_neutral_signals": min_non_neutral_signals,
            "min_non_neutral_weight": round_to(min_non_neutral_weight, 4),
            "non_neutral_signals": non_neutral_count,
            "non_neutral_weight": round_to(non_neutral_weight, 4),
            "insufficient": insufficient_evidence
        },
        "weighting_model": {
            "source_trust_floor": round_to(source_trust_floor, 4),
            "source_trust_ceiling": round_to(source_trust_ceiling, 4),
            "freshness_half_life_hours": round_to(freshness_half_life_hours, 4),
            "min_confidence_for_non_neutral": round_to(min_confidence_for_non_neutral, 4)
        },
        "top_sources": top_sources,
        "duality": duality_signal.clone().map(|signal| {
            json!({
                "enabled": signal.get("enabled").and_then(Value::as_bool).unwrap_or(false),
                "lane": signal.get("lane").cloned().unwrap_or_else(|| Value::String("belief_formation".to_string())),
                "score_trit": signal.get("score_trit").cloned().unwrap_or(Value::from(0)),
                "score_label": signal.get("score_label").cloned().unwrap_or_else(|| Value::String("unknown".to_string())),
                "zero_point_harmony_potential": round_to(as_f64(signal.get("zero_point_harmony_potential")).unwrap_or(0.0), 4),
                "recommended_adjustment": signal.get("recommended_adjustment").cloned().unwrap_or_else(|| Value::String("hold_balance_near_zero_point".to_string())),
                "effective_weight": round_to(as_f64(signal.get("effective_weight")).unwrap_or(0.0), 4),
                "advisory_delta": round_to(duality_influence, 4),
                "indicator": signal.get("indicator").cloned().unwrap_or(Value::Null)
            })
        }).unwrap_or_else(|| json!({"enabled": false, "advisory_delta": 0})),
        "signals": normalized
    });

    if duality_signal
        .as_ref()
        .and_then(|v| v.get("enabled"))
        .and_then(Value::as_bool)
        == Some(true)
    {
        let _ = crate::duality_seed::invoke(
            root,
            "registerDualityObservation",
            Some(&json!({
                "input": {
                    "lane": "belief_formation",
                    "source": "ternary_belief_engine",
                    "predicted_trit": duality_signal.as_ref().and_then(|v| v.get("score_trit")).cloned().unwrap_or(Value::from(0)),
                    "observed_trit": trit
                }
            })),
        );
    }

    Ok(result)
}

fn merge(payload: &Map<String, Value>) -> Value {
    let parent = payload
        .get("parent_belief")
        .or_else(|| payload.get("parent"))
        .unwrap_or(&Value::Null);
    let child = payload
        .get("child_belief")
        .or_else(|| payload.get("child"))
        .unwrap_or(&Value::Null);
    let opts = payload
        .get("opts")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let mode = {
        let v = clean_text(opts.get("mode"), 24).to_ascii_lowercase();
        if v.is_empty() {
            "cautious".to_string()
        } else {
            v
        }
    };
    let parent_weight = clamp_number(opts.get("parent_weight"), 0.0001, 1000.0, 1.0);
    let child_weight = clamp_number(opts.get("child_weight"), 0.0001, 1000.0, 1.0);
    let parent_trit = normalize_trit(parent.get("trit"));
    let child_trit = normalize_trit(child.get("trit"));
    let merged_trit = propagate_trit(parent_trit, child_trit, &mode);
    let parent_score = clamp_number(parent.get("score"), -1.0, 1.0, parent_trit as f64);
    let child_score = clamp_number(child.get("score"), -1.0, 1.0, child_trit as f64);
    let total_weight = parent_weight + child_weight;
    let merged_score = if total_weight > 0.0 {
        ((parent_score * parent_weight) + (child_score * child_weight)) / total_weight
    } else {
        0.0
    };
    let parent_confidence = clamp_number(parent.get("confidence"), 0.0, 1.0, 0.5);
    let child_confidence = clamp_number(child.get("confidence"), 0.0, 1.0, 0.5);
    let merged_confidence = if total_weight > 0.0 {
        ((parent_confidence * parent_weight) + (child_confidence * child_weight)) / total_weight
    } else {
        0.0
    };
    json!({
        "schema_id": "ternary_belief_merge",
        "schema_version": "1.0.0",
        "mode": mode,
        "trit": merged_trit,
        "trit_label": trit_label(merged_trit),
        "score": round_to(merged_score, 4),
        "confidence": round_to(merged_confidence, 4),
        "parent": {
            "trit": parent_trit,
            "trit_label": trit_label(parent_trit),
            "score": round_to(parent_score, 4),
            "confidence": round_to(parent_confidence, 4),
            "weight": round_to(parent_weight, 4)
        },
        "child": {
            "trit": child_trit,
            "trit_label": trit_label(child_trit),
            "score": round_to(child_score, 4),
            "confidence": round_to(child_confidence, 4),
            "weight": round_to(child_weight, 4)
        }
    })
}

fn serialize(payload: &Map<String, Value>) -> Value {
    let belief = payload
        .get("result")
        .or_else(|| payload.get("belief"))
        .unwrap_or(&Value::Null);
    let trit = normalize_trit(belief.get("trit"));
    let majority = normalize_trit(
        belief
            .get("majority_trit")
            .or_else(|| belief.get("majority")),
    );
    let consensus_signal = if belief.get("consensus").and_then(Value::as_bool) == Some(true) {
        trit
    } else {
        TRIT_UNKNOWN
    };
    json!({
        "schema_id": "ternary_belief_serialized",
        "schema_version": "1.0.0",
        "trit": trit,
        "trit_label": trit_label(trit),
        "score": round_to(clamp_number(belief.get("score"), -1.0, 1.0, trit as f64), 4),
        "confidence": round_to(clamp_number(belief.get("confidence"), 0.0, 1.0, 0.0), 4),
        "vector": serialize_trit_vector(&[trit, majority, consensus_signal]),
        "portability": {
            "target_hardware": "balanced_ternary_ready",
            "carrier_order": ["belief", "majority", "consensus"],
            "carriers": 3
        }
    })
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let Some(command) = argv.first().map(|v| v.as_str()) else {
        usage();
        return 1;
    };
    let payload = match payload_json(argv) {
        Ok(value) => value,
        Err(err) => {
            print_json_line(&cli_error("ternary_belief_kernel", &err));
            return 1;
        }
    };
    let payload = payload_obj(&payload).clone();
    let result = match command {
        "evaluate" => evaluate(root, &payload),
        "merge" => Ok(merge(&payload)),
        "serialize" => Ok(serialize(&payload)),
        "help" | "--help" | "-h" => {
            usage();
            return 0;
        }
        _ => Err("ternary_belief_kernel_unknown_command".to_string()),
    };
    match result {
        Ok(payload) => {
            print_json_line(&cli_receipt("ternary_belief_kernel", payload));
            0
        }
        Err(err) => {
            print_json_line(&cli_error("ternary_belief_kernel", &err));
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evaluate_returns_positive_belief() {
        let root = Path::new(".");
        let payload = json!({
            "signals": [
                {"source":"policy","trit":"ok","weight":1.0,"confidence":0.9},
                {"source":"health","trit":1,"weight":1.2,"confidence":0.95},
                {"source":"risk","trit":"unknown","weight":0.7,"confidence":0.8}
            ],
            "opts": {
                "label": "system_health",
                "source_trust": {"policy": 1.2, "health": 1.1},
                "force_neutral_on_insufficient_evidence": true
            }
        });
        let out = evaluate(root, payload.as_object().unwrap()).unwrap();
        assert_eq!(
            out["schema_id"],
            Value::String("ternary_belief".to_string())
        );
        assert_eq!(out["trit_label"], Value::String("ok".to_string()));
        assert_eq!(out["evidence_count"], Value::from(3));
    }

    #[test]
    fn merge_and_serialize_preserve_trit_vector() {
        let merged = merge(payload_obj(&json!({
            "parent": {"trit": 1, "score": 0.8, "confidence": 0.9},
            "child": {"trit": 1, "score": 0.6, "confidence": 0.7},
            "opts": {"mode": "cautious", "parent_weight": 1.0, "child_weight": 1.0}
        })));
        assert_eq!(merged["trit_label"], Value::String("ok".to_string()));
        let serialized = serialize(payload_obj(&json!({"belief": merged})));
        assert_eq!(
            serialized["vector"]["digits"],
            Value::String("+00".to_string())
        );
    }
}
