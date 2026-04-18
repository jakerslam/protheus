// SPDX-License-Identifier: Apache-2.0
//! Layer 2 initiative primitive.
//!
//! This module is the canonical extension point for queue-lane initiative behavior:
//! - normalize incoming importance fields,
//! - compute deterministic score/priority/band decisions,
//! - convert score to bounded escalation action contracts,
//! - and prioritize attention events without bypassing Layer 0 safety authority.
//!
//! If you are adding a custom execution lane, start by composing around
//! `evaluate_importance`, `evaluate_initiative_json`, and `prioritize_attention_json`.
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

const WEIGHT_CRITICALITY: f64 = 0.35;
const WEIGHT_URGENCY: f64 = 0.25;
const WEIGHT_IMPACT: f64 = 0.20;
const WEIGHT_USER_RELEVANCE: f64 = 0.15;
const WEIGHT_CONFIDENCE: f64 = 0.05;
pub const DEFAULT_FRONT_JUMP_THRESHOLD: f64 = 0.70;
pub const INITIATIVE_POLICY_VERSION: &str = "initiative_policy_v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportanceInput {
    #[serde(default)]
    pub criticality: Option<f64>,
    #[serde(default)]
    pub urgency: Option<f64>,
    #[serde(default)]
    pub impact: Option<f64>,
    #[serde(default)]
    pub user_relevance: Option<f64>,
    #[serde(default)]
    pub confidence: Option<f64>,
    #[serde(default)]
    pub core_floor: Option<f64>,
    #[serde(default)]
    pub inherited_score: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportanceDecision {
    pub score: f64,
    pub priority: i64,
    pub band: String,
    pub front_jump: bool,
    pub initiative_action: String,
    pub initiative_repeat_after_sec: i64,
    pub initiative_max_messages: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AttentionPriorityInput {
    #[serde(default)]
    events: Vec<Value>,
    #[serde(default)]
    front_jump_threshold: Option<f64>,
}

fn clamp01(v: f64) -> f64 {
    if !v.is_finite() {
        return 0.0;
    }
    v.clamp(0.0, 1.0)
}

fn parse_numeric_token(raw: &str) -> Option<f64> {
    let token = raw.trim();
    if token.is_empty() {
        return None;
    }
    if let Some(stripped) = token.strip_suffix('%') {
        return stripped.trim().parse::<f64>().ok().map(|value| clamp01(value / 100.0));
    }
    token.parse::<f64>().ok().map(clamp01)
}

fn numeric_value(v: Option<&Value>) -> Option<f64> {
    match v {
        Some(Value::Number(number)) => number.as_f64().map(clamp01),
        Some(Value::String(text)) => parse_numeric_token(text),
        Some(Value::Bool(flag)) => Some(if *flag { 1.0 } else { 0.0 }),
        _ => None,
    }
}

fn metric_aliases(key: &str) -> &'static [&'static str] {
    match key {
        "criticality" => &["criticality"],
        "urgency" => &["urgency"],
        "impact" => &["impact"],
        "user_relevance" => &["user_relevance", "userRelevance"],
        "confidence" => &["confidence"],
        "core_floor" => &["core_floor", "coreFloor"],
        "score" => &["score", "importance_score", "importanceScore"],
        "importance_score" => &["importance_score", "importanceScore", "score"],
        "inherited_score" => &[
            "inherited_score",
            "inheritedScore",
            "importance_score",
            "importanceScore",
            "score",
        ],
        _ => &[],
    }
}

fn metric_from_event(event: &Value, key: &str) -> Option<f64> {
    for alias in metric_aliases(key) {
        if let Some(value) = numeric_value(event.get(*alias)) {
            return Some(value);
        }
    }
    let importance = event.get("importance").and_then(Value::as_object)?;
    for alias in metric_aliases(key) {
        if let Some(value) = numeric_value(importance.get(*alias)) {
            return Some(value);
        }
    }
    None
}

fn score_from_input(input: &ImportanceInput) -> f64 {
    let weighted = clamp01(
        WEIGHT_CRITICALITY * clamp01(input.criticality.unwrap_or(0.0))
            + WEIGHT_URGENCY * clamp01(input.urgency.unwrap_or(0.0))
            + WEIGHT_IMPACT * clamp01(input.impact.unwrap_or(0.0))
            + WEIGHT_USER_RELEVANCE * clamp01(input.user_relevance.unwrap_or(0.0))
            + WEIGHT_CONFIDENCE * clamp01(input.confidence.unwrap_or(0.0)),
    );

    let inherited = input.inherited_score.map(clamp01);
    let mut score = if let Some(inherited_score) = inherited {
        // Inherited scores stay authoritative with light deterministic adjustment.
        clamp01((inherited_score * 0.8) + (weighted * 0.2))
    } else {
        weighted
    };

    if let Some(floor) = input.core_floor {
        score = score.max(clamp01(floor));
    }

    score
}

pub fn band_for_score(score: f64) -> &'static str {
    if score >= 0.95 {
        "p0"
    } else if score >= 0.85 {
        "p1"
    } else if score >= 0.70 {
        "p2"
    } else if score >= 0.40 {
        "p3"
    } else {
        "p4"
    }
}

pub fn initiative_for_score(score: f64) -> (&'static str, i64, i64) {
    if score >= 0.95 {
        ("persistent_until_ack", 60, 999)
    } else if score >= 0.85 {
        ("triple_escalation", 120, 3)
    } else if score >= 0.70 {
        ("double_message", 300, 2)
    } else if score >= 0.40 {
        ("single_message", 900, 1)
    } else {
        ("silent", 0, 0)
    }
}

pub fn front_jump_for_score(score: f64, front_jump_threshold: f64) -> bool {
    clamp01(score) >= clamp01(front_jump_threshold)
}

fn initiative_thresholds_json() -> Value {
    json!({
        "silent_below": 0.4,
        "single_message_min": 0.4,
        "double_message_min": 0.7,
        "triple_escalation_min": 0.85,
        "persistent_min": 0.95
    })
}

/// Compute a deterministic importance decision from normalized input metrics.
///
/// Extension guidance:
/// - keep score bounded (`0..=1`) and deterministic,
/// - do not bypass `front_jump_threshold` gates,
/// - map new initiative behaviors through explicit score bands.
pub fn evaluate_importance(
    input: &ImportanceInput,
    front_jump_threshold: f64,
) -> ImportanceDecision {
    let score = score_from_input(input);
    let priority = ((score * 1000.0).round() as i64).clamp(1, 1000);
    let band = band_for_score(score).to_string();
    let front_jump = front_jump_for_score(score, front_jump_threshold);
    let (initiative_action, initiative_repeat_after_sec, initiative_max_messages) =
        initiative_for_score(score);

    ImportanceDecision {
        score,
        priority,
        band,
        front_jump,
        initiative_action: initiative_action.to_string(),
        initiative_repeat_after_sec,
        initiative_max_messages,
    }
}

fn importance_from_event(event: &Value, front_jump_threshold: f64) -> ImportanceDecision {
    let mut input = ImportanceInput {
        criticality: metric_from_event(event, "criticality"),
        urgency: metric_from_event(event, "urgency"),
        impact: metric_from_event(event, "impact"),
        user_relevance: metric_from_event(event, "user_relevance"),
        confidence: metric_from_event(event, "confidence"),
        core_floor: metric_from_event(event, "core_floor"),
        inherited_score: metric_from_event(event, "inherited_score"),
    };
    let direct_score = metric_from_event(event, "score");
    if direct_score.is_some()
        && input.criticality.is_none()
        && input.urgency.is_none()
        && input.impact.is_none()
        && input.user_relevance.is_none()
        && input.confidence.is_none()
    {
        let score = direct_score.unwrap_or(0.0);
        input.criticality = Some(score);
        input.urgency = Some(score);
        input.impact = Some(score);
        input.user_relevance = Some(score);
        input.confidence = Some(score);
        input.inherited_score = None;
    }
    evaluate_importance(&input, front_jump_threshold)
}

pub fn evaluate_importance_json(payload: &str) -> Result<String, String> {
    let parsed: Value =
        serde_json::from_str(payload).map_err(|err| format!("invalid_json:{err}"))?;
    let threshold =
        numeric_value(parsed.get("front_jump_threshold")).unwrap_or(DEFAULT_FRONT_JUMP_THRESHOLD);
    let input = if let Some(event) = parsed.get("event") {
        ImportanceInput {
            criticality: metric_from_event(event, "criticality"),
            urgency: metric_from_event(event, "urgency"),
            impact: metric_from_event(event, "impact"),
            user_relevance: metric_from_event(event, "user_relevance"),
            confidence: metric_from_event(event, "confidence"),
            core_floor: metric_from_event(event, "core_floor"),
            inherited_score: metric_from_event(event, "inherited_score"),
        }
    } else {
        serde_json::from_value::<ImportanceInput>(parsed.clone())
            .map_err(|err| format!("invalid_importance_input:{err}"))?
    };

    let decision = evaluate_importance(&input, threshold);
    let out = json!({
        "ok": true,
        "type": "layer2_importance_score",
        "initiative_policy_version": INITIATIVE_POLICY_VERSION,
        "score": decision.score,
        "priority": decision.priority,
        "band": decision.band,
        "front_jump": decision.front_jump,
        "initiative_action": decision.initiative_action,
        "initiative_repeat_after_sec": decision.initiative_repeat_after_sec,
        "initiative_max_messages": decision.initiative_max_messages,
        "initiative_thresholds": initiative_thresholds_json(),
        "weights": {
            "criticality": WEIGHT_CRITICALITY,
            "urgency": WEIGHT_URGENCY,
            "impact": WEIGHT_IMPACT,
            "user_relevance": WEIGHT_USER_RELEVANCE,
            "confidence": WEIGHT_CONFIDENCE
        }
    });

    serde_json::to_string(&out).map_err(|err| format!("encode_failed:{err}"))
}

pub fn evaluate_initiative_json(payload: &str) -> Result<String, String> {
    let parsed: Value =
        serde_json::from_str(payload).map_err(|err| format!("invalid_json:{err}"))?;
    let score = numeric_value(parsed.get("score")).unwrap_or(0.0);
    let (action, repeat_after_sec, max_messages) = initiative_for_score(score);
    let out = json!({
        "ok": true,
        "type": "layer2_initiative_decision",
        "initiative_policy_version": INITIATIVE_POLICY_VERSION,
        "score": score,
        "action": action,
        "repeat_after_sec": repeat_after_sec,
        "max_messages": max_messages,
        "front_jump": score >= DEFAULT_FRONT_JUMP_THRESHOLD,
        "thresholds": initiative_thresholds_json()
    });
    serde_json::to_string(&out).map_err(|err| format!("encode_failed:{err}"))
}

/// Sort attention events by front-jump, then score, then original order.
///
/// This function is intentionally stable for equal-ranked events to keep replay
/// and receipt chains deterministic.
pub fn prioritize_attention_json(payload: &str) -> Result<String, String> {
    let input: AttentionPriorityInput =
        serde_json::from_str(payload).map_err(|err| format!("invalid_json:{err}"))?;
    let threshold = clamp01(
        input
            .front_jump_threshold
            .unwrap_or(DEFAULT_FRONT_JUMP_THRESHOLD),
    );

    let mut decorated: Vec<(usize, bool, f64, Value)> = Vec::new();
    for (idx, mut event) in input.events.into_iter().enumerate() {
        let decision = importance_from_event(&event, threshold);
        if let Some(map) = event.as_object_mut() {
            map.insert("score".to_string(), json!(decision.score));
            map.insert("priority".to_string(), json!(decision.priority));
            map.insert("band".to_string(), json!(decision.band));
            map.insert(
                "initiative_action".to_string(),
                json!(decision.initiative_action),
            );
            map.insert("queue_front".to_string(), json!(decision.front_jump));
            map.insert(
                "importance".to_string(),
                json!({
                    "score": decision.score,
                    "priority": decision.priority,
                    "band": decision.band,
                    "initiative_action": decision.initiative_action,
                    "initiative_policy_version": INITIATIVE_POLICY_VERSION,
                    "initiative_repeat_after_sec": decision.initiative_repeat_after_sec,
                    "initiative_max_messages": decision.initiative_max_messages
                }),
            );
        }
        decorated.push((idx, decision.front_jump, decision.score, event));
    }

    decorated.sort_by(|a, b| {
        b.1.cmp(&a.1)
            .then_with(|| b.2.total_cmp(&a.2))
            .then_with(|| a.0.cmp(&b.0))
    });

    let ordered: Vec<Value> = decorated.into_iter().map(|(_, _, _, row)| row).collect();
    let out = json!({
        "ok": true,
        "type": "layer2_attention_priority",
        "initiative_policy_version": INITIATIVE_POLICY_VERSION,
        "front_jump_threshold": threshold,
        "queue_depth": ordered.len(),
        "events": ordered
    });

    serde_json::to_string(&out).map_err(|err| format!("encode_failed:{err}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn importance_scoring_applies_core_floor() {
        let input = ImportanceInput {
            criticality: Some(0.1),
            urgency: Some(0.1),
            impact: Some(0.1),
            user_relevance: Some(0.1),
            confidence: Some(0.1),
            core_floor: Some(0.85),
            inherited_score: None,
        };
        let out = evaluate_importance(&input, DEFAULT_FRONT_JUMP_THRESHOLD);
        assert!(out.score >= 0.85);
        assert_eq!(out.band, "p1");
    }

    #[test]
    fn initiative_thresholds_match_contract() {
        let low = evaluate_initiative_json(r#"{"score":0.2}"#).unwrap();
        assert!(low.contains("\"action\":\"silent\""));

        let mid = evaluate_initiative_json(r#"{"score":0.72}"#).unwrap();
        assert!(mid.contains("\"action\":\"double_message\""));

        let boundary = evaluate_initiative_json(r#"{"score":0.85}"#).unwrap();
        assert!(boundary.contains("\"action\":\"triple_escalation\""));

        let high = evaluate_initiative_json(r#"{"score":0.96}"#).unwrap();
        assert!(high.contains("\"action\":\"persistent_until_ack\""));
    }

    #[test]
    fn attention_priority_front_jumps_high_importance() {
        let payload = r#"{
          "events": [
            {"summary":"low","criticality":0.1,"urgency":0.1,"impact":0.1,"user_relevance":0.1,"confidence":0.1},
            {"summary":"high","criticality":1.0,"urgency":1.0,"impact":1.0,"user_relevance":1.0,"confidence":1.0}
          ]
        }"#;
        let out = prioritize_attention_json(payload).unwrap();
        let parsed: Value = serde_json::from_str(&out).unwrap();
        let events = parsed.get("events").and_then(Value::as_array).unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(
            events[0].get("summary").and_then(Value::as_str),
            Some("high")
        );
        assert_eq!(
            events[0].get("queue_front").and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn importance_json_accepts_percent_strings_and_aliases() {
        let payload = r#"{
          "front_jump_threshold": "70%",
          "event": {
            "criticality": "95%",
            "urgency": "0.8",
            "impact": 0.7,
            "userRelevance": "0.9",
            "confidence": "1",
            "coreFloor": "0.6"
          }
        }"#;
        let out = evaluate_importance_json(payload).unwrap();
        let parsed: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(parsed.get("front_jump").and_then(Value::as_bool), Some(true));
        assert!(parsed.get("score").and_then(Value::as_f64).unwrap_or(0.0) >= 0.6);
    }

    #[test]
    fn attention_priority_accepts_importance_score_alias() {
        let payload = r#"{
          "events": [
            {"summary":"low","importanceScore":"25%"},
            {"summary":"high","importanceScore":"97%"}
          ]
        }"#;
        let out = prioritize_attention_json(payload).unwrap();
        let parsed: Value = serde_json::from_str(&out).unwrap();
        let events = parsed.get("events").and_then(Value::as_array).unwrap();
        assert_eq!(
            events[0].get("summary").and_then(Value::as_str),
            Some("high")
        );
        assert_eq!(events[0].get("band").and_then(Value::as_str), Some("p0"));
    }
}
