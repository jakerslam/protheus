// SPDX-License-Identifier: Apache-2.0
use crate::execution_lane_bridge::{
    band_for_score as initiative_band_for_score,
    front_jump_for_score as initiative_front_jump_for_score,
    initiative_for_score as initiative_contract_for_score, DEFAULT_FRONT_JUMP_THRESHOLD,
    INITIATIVE_POLICY_VERSION,
};
use serde_json::{json, Value};
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct ImportanceDecision {
    pub score: f64,
    pub band: String,
    pub priority: i64,
    pub reason_codes: Vec<String>,
    pub criticality: f64,
    pub urgency: f64,
    pub impact: f64,
    pub user_relevance: f64,
    pub confidence: f64,
    pub core_floor: f64,
    pub initiative_action: String,
    pub initiative_policy_version: String,
    pub initiative_repeat_after_sec: i64,
    pub initiative_max_messages: i64,
    pub queue_front: bool,
}

fn clamp01(v: f64) -> f64 {
    if !v.is_finite() {
        return 0.0;
    }
    v.clamp(0.0, 1.0)
}

fn parse_f64(v: Option<&Value>) -> Option<f64> {
    v.and_then(|value| {
        value
            .as_f64()
            .or_else(|| value.as_str().and_then(|raw| raw.trim().parse::<f64>().ok()))
    })
    .map(clamp01)
}

fn parse_bool(v: Option<&Value>) -> bool {
    v.and_then(|value| {
        value.as_bool().or_else(|| {
            value.as_str().map(|raw| {
                matches!(
                    raw.trim().to_ascii_lowercase().as_str(),
                    "1" | "true" | "yes" | "on"
                )
            })
        })
    })
    .unwrap_or(false)
}

fn canonical_severity(raw: &str) -> String {
    let token = raw.trim().to_ascii_lowercase();
    match token.as_str() {
        "error" | "fatal" | "sev0" | "sev1" => "critical".to_string(),
        "warning" | "degraded" | "sev2" => "warn".to_string(),
        "notice" | "debug" | "sev3" => "info".to_string(),
        _ => token,
    }
}

fn severity_criticality(severity: &str) -> f64 {
    match severity {
        "critical" => 1.0,
        "warn" => 0.65,
        "info" => 0.35,
        _ => 0.35,
    }
}

fn text_contains_any(text: &str, needles: &[&str]) -> bool {
    let hay = text.to_ascii_lowercase();
    needles.iter().any(|needle| hay.contains(needle))
}

pub fn band_rank(band: &str) -> i64 {
    match band.trim().to_ascii_lowercase().as_str() {
        "p0" => 5,
        "p1" => 4,
        "p2" => 3,
        "p3" => 2,
        "p4" => 1,
        _ => 1,
    }
}

pub fn infer_from_event(
    event: &Value,
    severity: &str,
    priority_map: &BTreeMap<String, i64>,
) -> ImportanceDecision {
    let severity = canonical_severity(severity);
    let source = event
        .get("source")
        .and_then(Value::as_str)
        .unwrap_or("unknown_source")
        .trim()
        .to_ascii_lowercase();
    let source_type = event
        .get("source_type")
        .and_then(Value::as_str)
        .unwrap_or("unknown_type")
        .trim()
        .to_ascii_lowercase();
    let summary = event
        .get("summary")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    let importance_obj = event.get("importance").and_then(Value::as_object);

    let explicit_priority = event
        .get("priority")
        .and_then(Value::as_i64)
        .or_else(|| {
            importance_obj
                .and_then(|v| v.get("priority"))
                .and_then(Value::as_i64)
        })
        .map(|n| n.clamp(1, 1000));
    let explicit_score = parse_f64(
        importance_obj
            .and_then(|v| v.get("score"))
            .or_else(|| event.get("importance_score")),
    );

    let criticality = parse_f64(
        importance_obj
            .and_then(|v| v.get("criticality"))
            .or_else(|| event.get("criticality")),
    )
    .unwrap_or_else(|| severity_criticality(&severity));
    let urgency = parse_f64(
        importance_obj
            .and_then(|v| v.get("urgency"))
            .or_else(|| event.get("urgency")),
    )
    .unwrap_or_else(|| {
        if severity == "critical" {
            0.85
        } else if severity == "warn" {
            0.55
        } else {
            0.25
        }
    });
    let mut impact = parse_f64(
        importance_obj
            .and_then(|v| v.get("impact"))
            .or_else(|| event.get("impact")),
    )
    .unwrap_or_else(|| {
        if severity == "critical" {
            0.80
        } else if severity == "warn" {
            0.55
        } else {
            0.35
        }
    });
    let mut user_relevance = parse_f64(
        importance_obj
            .and_then(|v| v.get("user_relevance"))
            .or_else(|| event.get("user_relevance")),
    )
    .unwrap_or_else(|| {
        if text_contains_any(&summary, &["t1", "goal", "revenue", "deadline"]) {
            0.75
        } else {
            0.50
        }
    });
    let confidence = parse_f64(
        importance_obj
            .and_then(|v| v.get("confidence"))
            .or_else(|| event.get("confidence")),
    )
    .unwrap_or(0.80);

    let mut reason_codes = Vec::new();

    // Core/system health dominates higher-layer cognition tasks.
    let mut core_floor: f64 = if severity == "critical" { 0.85 } else { 0.0 };
    if severity == "critical" {
        reason_codes.push("severity_critical_floor".to_string());
    }
    if text_contains_any(
        &source,
        &[
            "spine",
            "conduit",
            "security",
            "integrity",
            "memory_ambient",
        ],
    ) || text_contains_any(
        &source_type,
        &["infra_outage", "system_fault", "eye_run_failed"],
    ) {
        impact = impact.max(0.85);
        reason_codes.push("core_subsystem_signal".to_string());
    }
    if text_contains_any(
        &summary,
        &["conduit", "bridge", "runtime_gate", "timeout", "degraded"],
    ) {
        core_floor = core_floor.max(0.92);
        reason_codes.push("conduit_health_risk".to_string());
    }
    if text_contains_any(
        &summary,
        &[
            "security_global_gate_failed",
            "integrity",
            "tamper",
            "quarantine",
            "attestation",
        ],
    ) || text_contains_any(&source, &["security"])
    {
        core_floor = core_floor.max(0.97);
        reason_codes.push("security_integrity_risk".to_string());
    }
    if text_contains_any(
        &summary,
        &[
            "data loss",
            "corrupt",
            "missing receipts",
            "receipt mismatch",
        ],
    ) {
        core_floor = core_floor.max(0.95);
        reason_codes.push("data_integrity_risk".to_string());
    }
    if parse_bool(
        importance_obj
            .and_then(|v| v.get("dream_inclusion"))
            .or_else(|| event.get("dream_inclusion")),
    ) {
        user_relevance = (user_relevance + 0.10).clamp(0.0, 1.0);
        reason_codes.push("dream_inclusion_boost".to_string());
    }

    let weighted = clamp01(
        (criticality * 0.35)
            + (urgency * 0.25)
            + (impact * 0.20)
            + (user_relevance * 0.15)
            + (confidence * 0.05),
    );

    let mut score = weighted.max(core_floor);
    if let Some(explicit) = explicit_score {
        score = score.max(explicit);
        reason_codes.push("explicit_score_override".to_string());
    }
    if let Some(priority) = explicit_priority {
        score = score.max((priority as f64 / 1000.0).clamp(0.0, 1.0));
        reason_codes.push("explicit_priority_hint".to_string());
    }
    score = clamp01(score);

    let band = initiative_band_for_score(score).to_string();
    let severity_priority = *priority_map.get(&severity).unwrap_or(&20);
    let derived_priority = ((score * 1000.0).round() as i64).clamp(1, 1000);
    let priority = explicit_priority
        .unwrap_or(derived_priority)
        .max(severity_priority)
        .clamp(1, 1000);

    let (initiative_action, initiative_repeat_after_sec, initiative_max_messages) =
        initiative_contract_for_score(score);
    let queue_front = initiative_front_jump_for_score(score, DEFAULT_FRONT_JUMP_THRESHOLD);

    ImportanceDecision {
        score,
        band,
        priority,
        reason_codes,
        criticality,
        urgency,
        impact,
        user_relevance,
        confidence,
        core_floor,
        initiative_action: initiative_action.to_string(),
        initiative_policy_version: INITIATIVE_POLICY_VERSION.to_string(),
        initiative_repeat_after_sec,
        initiative_max_messages,
        queue_front,
    }
}

pub fn to_json(decision: &ImportanceDecision) -> Value {
    json!({
        "score": decision.score,
        "band": decision.band,
        "priority": decision.priority,
        "reason_codes": decision.reason_codes,
        "criticality": decision.criticality,
        "urgency": decision.urgency,
        "impact": decision.impact,
        "user_relevance": decision.user_relevance,
        "confidence": decision.confidence,
        "core_floor": decision.core_floor,
        "initiative": {
            "action": decision.initiative_action,
            "policy_version": decision.initiative_policy_version,
            "repeat_after_sec": decision.initiative_repeat_after_sec,
            "max_messages": decision.initiative_max_messages
        },
        "queue_front": decision.queue_front
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn critical_core_health_gets_floor_and_high_band() {
        let mut pm = BTreeMap::new();
        pm.insert("critical".to_string(), 100);
        let event = json!({
            "source": "spine",
            "source_type": "infra_outage_state",
            "severity": "critical",
            "summary": "conduit bridge timeout degraded"
        });
        let out = infer_from_event(&event, "critical", &pm);
        assert!(out.score >= 0.92);
        assert_eq!(out.band, "p1");
        assert!(out.priority >= 920);
        assert_eq!(out.initiative_action, "triple_escalation");
        assert_eq!(out.initiative_policy_version, INITIATIVE_POLICY_VERSION);
    }

    #[test]
    fn low_info_stays_quiet() {
        let pm = BTreeMap::new();
        let event = json!({
            "source": "external_eyes",
            "source_type": "external_item",
            "severity": "info",
            "summary": "new article"
        });
        let out = infer_from_event(&event, "info", &pm);
        assert_eq!(out.band, "p4");
        assert_eq!(out.initiative_action, "silent");
        assert_eq!(out.initiative_max_messages, 0);
        assert_eq!(out.initiative_policy_version, INITIATIVE_POLICY_VERSION);
    }

    #[test]
    fn layer0_and_layer2_initiative_contract_match_for_same_inputs() {
        let pm = BTreeMap::new();
        for score in [0.20, 0.40, 0.70, 0.85, 0.95, 1.0] {
            let event = json!({
                "source": "external_eyes",
                "source_type": "external_item",
                "severity": "info",
                "summary": "contract parity",
                "importance": {
                    "criticality": score,
                    "urgency": score,
                    "impact": score,
                    "user_relevance": score,
                    "confidence": score,
                    "score": score
                }
            });
            let layer0 = infer_from_event(&event, "info", &pm);
            let layer2 = crate::execution_lane_bridge::evaluate_importance(
                &crate::execution_lane_bridge::ImportanceInput {
                    criticality: Some(score),
                    urgency: Some(score),
                    impact: Some(score),
                    user_relevance: Some(score),
                    confidence: Some(score),
                    core_floor: Some(0.0),
                    inherited_score: Some(score),
                },
                DEFAULT_FRONT_JUMP_THRESHOLD,
            );
            assert!((layer0.score - layer2.score).abs() <= f64::EPSILON);
            assert_eq!(layer0.band, layer2.band);
            assert_eq!(layer0.initiative_action, layer2.initiative_action);
            assert_eq!(
                layer0.initiative_repeat_after_sec,
                layer2.initiative_repeat_after_sec
            );
            assert_eq!(
                layer0.initiative_max_messages,
                layer2.initiative_max_messages
            );
            assert_eq!(layer0.queue_front, layer2.front_jump);
            assert_eq!(layer0.initiative_policy_version, INITIATIVE_POLICY_VERSION);
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: 64,
            .. ProptestConfig::default()
        })]

        #[test]
        fn inferred_importance_stays_bounded_and_consistent(
            criticality in -10.0f64..10.0,
            urgency in -10.0f64..10.0,
            impact in -10.0f64..10.0,
            user_relevance in -10.0f64..10.0,
            confidence in -10.0f64..10.0,
            severity in prop_oneof![Just("info"), Just("warn"), Just("critical")]
        ) {
            let mut pm = BTreeMap::new();
            pm.insert("critical".to_string(), 100);
            pm.insert("warn".to_string(), 60);
            pm.insert("info".to_string(), 20);
            let event = json!({
                "source": "proptest",
                "source_type": "property",
                "severity": severity,
                "summary": "property test event",
                "importance": {
                    "criticality": criticality,
                    "urgency": urgency,
                    "impact": impact,
                    "user_relevance": user_relevance,
                    "confidence": confidence
                }
            });
            let out = infer_from_event(&event, severity, &pm);
            prop_assert!((0.0..=1.0).contains(&out.score));
            prop_assert!((1..=1000).contains(&out.priority));
            prop_assert!(matches!(out.band.as_str(), "p0" | "p1" | "p2" | "p3" | "p4"));
            let (expected_action, expected_repeat_after_sec, expected_max_messages) =
                initiative_contract_for_score(out.score);
            prop_assert_eq!(out.band, initiative_band_for_score(out.score));
            prop_assert_eq!(out.initiative_action, expected_action);
            prop_assert_eq!(out.initiative_repeat_after_sec, expected_repeat_after_sec);
            prop_assert_eq!(out.initiative_max_messages, expected_max_messages);
            prop_assert_eq!(
                out.queue_front,
                initiative_front_jump_for_score(out.score, DEFAULT_FRONT_JUMP_THRESHOLD)
            );
        }
    }
}
