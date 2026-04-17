// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, BTreeSet};

use crate::contract_lane_utils as lane_utils;
use crate::now_iso;

fn usage() {
    println!("strategy-campaign-scheduler-kernel commands:");
    println!("  protheus-ops strategy-campaign-scheduler-kernel normalize-campaigns --payload-base64=<json>");
    println!("  protheus-ops strategy-campaign-scheduler-kernel annotate-priority --payload-base64=<json>");
    println!("  protheus-ops strategy-campaign-scheduler-kernel build-decomposition-plans --payload-base64=<json>");
}

fn payload_json(argv: &[String]) -> Result<Value, String> {
    lane_utils::payload_json(argv, "strategy_campaign_scheduler_kernel")
}

fn payload_obj<'a>(value: &'a Value) -> &'a Map<String, Value> {
    value.as_object().unwrap_or_else(|| {
        static EMPTY: std::sync::OnceLock<Map<String, Value>> = std::sync::OnceLock::new();
        EMPTY.get_or_init(Map::new)
    })
}

fn as_object<'a>(value: Option<&'a Value>) -> Option<&'a Map<String, Value>> {
    value.and_then(Value::as_object)
}

fn as_array<'a>(value: Option<&'a Value>) -> &'a Vec<Value> {
    value.and_then(Value::as_array).unwrap_or_else(|| {
        static EMPTY: std::sync::OnceLock<Vec<Value>> = std::sync::OnceLock::new();
        EMPTY.get_or_init(Vec::new)
    })
}

fn as_str(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(v)) => v.trim().to_string(),
        Some(Value::Null) | None => String::new(),
        Some(v) => v.to_string().trim_matches('"').trim().to_string(),
    }
}

fn clean_text(value: Option<&Value>, max_len: usize) -> String {
    let mut out = as_str(value)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if out.len() > max_len {
        out.truncate(max_len);
    }
    out
}

fn as_lower(value: Option<&Value>, max_len: usize) -> String {
    clean_text(value, max_len).to_ascii_lowercase()
}

fn as_i64(value: Option<&Value>) -> Option<i64> {
    match value {
        Some(Value::Number(n)) => n.as_i64(),
        Some(Value::String(v)) => v.trim().parse::<i64>().ok(),
        _ => None,
    }
}

fn as_string_array_lower(value: Option<&Value>) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = BTreeSet::new();
    for row in as_array(value) {
        let token = as_lower(Some(row), 120);
        if token.is_empty() || !seen.insert(token.clone()) {
            continue;
        }
        out.push(token);
    }
    out
}

#[derive(Clone, Debug)]
struct Phase {
    raw: Value,
    id: String,
    name: String,
    objective_id: String,
    order: i64,
    priority: i64,
    proposal_types: Vec<String>,
    source_eyes: Vec<String>,
    tags: Vec<String>,
}

#[derive(Clone, Debug)]
struct Campaign {
    raw: Value,
    id: String,
    name: String,
    objective_id: String,
    priority: i64,
    proposal_types: Vec<String>,
    source_eyes: Vec<String>,
    tags: Vec<String>,
    phases: Vec<Phase>,
}

fn campaign_cmp(a: &Campaign, b: &Campaign) -> std::cmp::Ordering {
    a.priority.cmp(&b.priority).then_with(|| a.id.cmp(&b.id))
}

fn phase_cmp(a: &Phase, b: &Phase) -> std::cmp::Ordering {
    a.order
        .cmp(&b.order)
        .then_with(|| b.priority.cmp(&a.priority))
        .then_with(|| a.id.cmp(&b.id))
}

fn normalize_campaigns(strategy: &Value) -> Vec<Campaign> {
    let mut campaigns = Vec::new();
    let rows = strategy
        .get("campaigns")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for row in rows {
        let Some(obj) = row.as_object() else {
            continue;
        };
        if as_lower(obj.get("status"), 40) != "active" {
            continue;
        }
        let mut phases = Vec::new();
        for phase_row in as_array(obj.get("phases")).iter().cloned() {
            let Some(phase_obj) = phase_row.as_object() else {
                continue;
            };
            if as_lower(phase_obj.get("status"), 40) != "active" {
                continue;
            }
            let phase = Phase {
                raw: phase_row.clone(),
                id: as_lower(phase_obj.get("id"), 120),
                name: clean_text(phase_obj.get("name"), 260),
                objective_id: clean_text(phase_obj.get("objective_id"), 160),
                order: as_i64(phase_obj.get("order")).unwrap_or(99),
                priority: as_i64(phase_obj.get("priority")).unwrap_or(0),
                proposal_types: as_string_array_lower(phase_obj.get("proposal_types")),
                source_eyes: as_string_array_lower(phase_obj.get("source_eyes")),
                tags: as_string_array_lower(phase_obj.get("tags")),
            };
            if !phase.id.is_empty() {
                phases.push(phase);
            }
        }
        phases.sort_by(phase_cmp);
        let campaign = Campaign {
            raw: row.clone(),
            id: as_lower(obj.get("id"), 120),
            name: clean_text(obj.get("name"), 260),
            objective_id: clean_text(obj.get("objective_id"), 160),
            priority: as_i64(obj.get("priority")).unwrap_or(50),
            proposal_types: as_string_array_lower(obj.get("proposal_types")),
            source_eyes: as_string_array_lower(obj.get("source_eyes")),
            tags: as_string_array_lower(obj.get("tags")),
            phases,
        };
        if !campaign.id.is_empty() && !campaign.phases.is_empty() {
            campaigns.push(campaign);
        }
    }
    campaigns.sort_by(campaign_cmp);
    campaigns
}

fn campaigns_as_value(campaigns: &[Campaign]) -> Value {
    Value::Array(
        campaigns
            .iter()
            .map(|campaign| {
                let Some(obj) = campaign.raw.as_object() else {
                    return Value::Null;
                };
                let mut out = obj.clone();
                out.insert("id".to_string(), Value::String(campaign.id.clone()));
                out.insert("name".to_string(), Value::String(campaign.name.clone()));
                out.insert(
                    "objective_id".to_string(),
                    if campaign.objective_id.is_empty() {
                        Value::Null
                    } else {
                        Value::String(campaign.objective_id.clone())
                    },
                );
                out.insert("priority".to_string(), Value::from(campaign.priority));
                out.insert("proposal_types".to_string(), json!(campaign.proposal_types));
                out.insert("source_eyes".to_string(), json!(campaign.source_eyes));
                out.insert("tags".to_string(), json!(campaign.tags));
                out.insert(
                    "phases".to_string(),
                    Value::Array(
                        campaign
                            .phases
                            .iter()
                            .map(|phase| {
                                let Some(phase_obj) = phase.raw.as_object() else {
                                    return Value::Null;
                                };
                                let mut next = phase_obj.clone();
                                next.insert("id".to_string(), Value::String(phase.id.clone()));
                                next.insert("name".to_string(), Value::String(phase.name.clone()));
                                next.insert(
                                    "objective_id".to_string(),
                                    if phase.objective_id.is_empty() {
                                        Value::Null
                                    } else {
                                        Value::String(phase.objective_id.clone())
                                    },
                                );
                                next.insert("order".to_string(), Value::from(phase.order));
                                next.insert("priority".to_string(), Value::from(phase.priority));
                                next.insert(
                                    "proposal_types".to_string(),
                                    json!(phase.proposal_types),
                                );
                                next.insert("source_eyes".to_string(), json!(phase.source_eyes));
                                next.insert("tags".to_string(), json!(phase.tags));
                                Value::Object(next)
                            })
                            .collect(),
                    ),
                );
                Value::Object(out)
            })
            .collect(),
    )
}

fn candidate_objective_id(candidate: &Value) -> String {
    let parts = [
        candidate.pointer("/objective_binding/objective_id"),
        candidate.pointer("/directive_pulse/objective_id"),
        candidate.pointer("/proposal/meta/objective_id"),
        candidate.pointer("/proposal/meta/directive_objective_id"),
        candidate.pointer("/proposal/action_spec/objective_id"),
    ];
    for value in parts {
        let token = clean_text(value, 160);
        if !token.is_empty() {
            return token;
        }
    }
    String::new()
}

fn candidate_type(candidate: &Value) -> String {
    as_lower(candidate.pointer("/proposal/type"), 120)
}

fn candidate_source_eye(candidate: &Value) -> String {
    as_lower(candidate.pointer("/proposal/meta/source_eye"), 120)
}

fn candidate_tag_set(candidate: &Value) -> BTreeSet<String> {
    let mut tags = BTreeSet::new();
    for row in as_string_array_lower(candidate.pointer("/proposal/tags")) {
        tags.insert(row);
    }
    for row in as_string_array_lower(candidate.pointer("/proposal/meta/tags")) {
        tags.insert(row);
    }
    tags
}

fn has_any_overlap(required: &[String], values: &BTreeSet<String>) -> bool {
    if required.is_empty() {
        return true;
    }
    required.iter().any(|row| values.contains(row))
}

fn is_filter_match(required: &[String], value: &str) -> bool {
    required.is_empty() || required.iter().any(|row| row == value)
}

fn is_phase_preferred_filter_match(
    campaign_required: &[String],
    phase_required: &[String],
    value: &str,
) -> bool {
    if !phase_required.is_empty() {
        return is_filter_match(phase_required, value);
    }
    is_filter_match(campaign_required, value)
}

fn score_match(campaign: &Campaign, phase: &Phase, candidate: &Value) -> Option<Value> {
    let objective_id = candidate_objective_id(candidate);
    let proposal_type = candidate_type(candidate);
    let source_eye = candidate_source_eye(candidate);
    let tags = candidate_tag_set(candidate);

    if !campaign.objective_id.is_empty() && objective_id != campaign.objective_id {
        return None;
    }
    if !phase.objective_id.is_empty() && objective_id != phase.objective_id {
        return None;
    }
    if !is_phase_preferred_filter_match(
        &campaign.proposal_types,
        &phase.proposal_types,
        &proposal_type,
    ) {
        return None;
    }
    if !is_filter_match(&campaign.source_eyes, &source_eye) {
        return None;
    }
    if !is_filter_match(&phase.source_eyes, &source_eye) {
        return None;
    }
    if !has_any_overlap(&campaign.tags, &tags) {
        return None;
    }
    if !has_any_overlap(&phase.tags, &tags) {
        return None;
    }

    let tag_overlap = tags
        .iter()
        .filter(|tag| campaign.tags.contains(tag) || phase.tags.contains(tag))
        .count() as i64;

    let mut score = 0_i64;
    score += (120 - campaign.priority).max(0);
    score += (80 - (phase.order * 5)).max(0);
    score += phase.priority;
    if !campaign.objective_id.is_empty() && !objective_id.is_empty() {
        score += 35;
    }
    if !phase.objective_id.is_empty() && !objective_id.is_empty() {
        score += 20;
    }
    if !campaign.proposal_types.is_empty() {
        score += 18;
    }
    if !phase.proposal_types.is_empty() {
        score += 14;
    }
    if !campaign.source_eyes.is_empty() || !phase.source_eyes.is_empty() {
        score += 10;
    }
    score += (tag_overlap * 4).min(20);

    Some(json!({
        "matched": true,
        "score": score,
        "campaign_id": campaign.id,
        "campaign_name": if campaign.name.is_empty() { campaign.id.clone() } else { campaign.name.clone() },
        "campaign_priority": campaign.priority,
        "phase_id": phase.id,
        "phase_name": if phase.name.is_empty() { phase.id.clone() } else { phase.name.clone() },
        "phase_order": phase.order,
        "phase_priority": phase.priority,
        "objective_id": if objective_id.is_empty() {
            if !campaign.objective_id.is_empty() {
                campaign.objective_id.clone()
            } else {
                phase.objective_id.clone()
            }
        } else {
            objective_id
        }
    }))
}

fn best_campaign_match(candidate: &Value, campaigns: &[Campaign]) -> Option<Value> {
    let mut best: Option<Value> = None;
    for campaign in campaigns {
        for phase in &campaign.phases {
            let Some(next) = score_match(campaign, phase, candidate) else {
                continue;
            };
            let next_score = next.get("score").and_then(Value::as_i64).unwrap_or(0);
            let best_score = best
                .as_ref()
                .and_then(|row| row.get("score"))
                .and_then(Value::as_i64)
                .unwrap_or(i64::MIN);
            if best.is_none() || next_score > best_score {
                best = Some(next);
            }
        }
    }
    best
}

fn annotate_campaign_priority(candidates: &[Value], strategy: &Value) -> Value {
    let campaigns = normalize_campaigns(strategy);
    if campaigns.is_empty() {
        let annotated = candidates
            .iter()
            .map(|candidate| {
                let mut next = candidate.as_object().cloned().unwrap_or_default();
                next.insert("campaign_match".to_string(), Value::Null);
                next.insert("campaign_sort_bucket".to_string(), Value::from(0));
                next.insert("campaign_sort_score".to_string(), Value::from(0));
                Value::Object(next)
            })
            .collect::<Vec<_>>();
        return json!({
            "summary": {
                "enabled": false,
                "campaign_count": 0,
                "matched_count": 0
            },
            "candidates": annotated
        });
    }
