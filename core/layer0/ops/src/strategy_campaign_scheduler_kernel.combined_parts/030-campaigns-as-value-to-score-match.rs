
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
