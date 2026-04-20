
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


    let mut matched_count = 0_i64;
    let mut matched_by_campaign = BTreeMap::<String, i64>::new();
    let annotated = candidates
        .iter()
        .map(|candidate| {
            let mut next = candidate.as_object().cloned().unwrap_or_default();
            if let Some(found) = best_campaign_match(candidate, &campaigns) {
                matched_count += 1;
                if let Some(campaign_id) = found.get("campaign_id").and_then(Value::as_str) {
                    *matched_by_campaign
                        .entry(campaign_id.to_string())
                        .or_insert(0) += 1;
                }
                next.insert("campaign_match".to_string(), found.clone());
                next.insert("campaign_sort_bucket".to_string(), Value::from(1));
                next.insert(
                    "campaign_sort_score".to_string(),
                    Value::from(found.get("score").and_then(Value::as_i64).unwrap_or(0)),
                );
            } else {
                next.insert("campaign_match".to_string(), Value::Null);
                next.insert("campaign_sort_bucket".to_string(), Value::from(0));
                next.insert("campaign_sort_score".to_string(), Value::from(0));
            }
            Value::Object(next)
        })
        .collect::<Vec<_>>();
    json!({
        "summary": {
            "enabled": true,
            "campaign_count": campaigns.len(),
            "matched_count": matched_count,
            "unmatched_count": (candidates.len() as i64 - matched_count).max(0),
            "matched_by_campaign": matched_by_campaign
        },
        "candidates": annotated
    })
}

fn proposal_status_lower(proposal: &Value) -> String {
    as_lower(proposal.get("status").or_else(|| proposal.get("state")), 80)
}

fn is_terminal_proposal_status(status: &str) -> bool {
    matches!(
        status,
        "resolved"
            | "done"
            | "closed"
            | "shipped"
            | "no_change"
            | "reverted"
            | "rejected"
            | "filtered"
            | "superseded"
            | "archived"
            | "dropped"
    )
}

fn sanitize_token(raw: &str, fallback: &str) -> String {
    let mut out = String::new();
    let mut prev_dash = false;
    for ch in raw.chars() {
        let lower = ch.to_ascii_lowercase();
        if lower.is_ascii_alphanumeric() {
            out.push(lower);
            prev_dash = false;
        } else if !prev_dash {
            out.push('-');
            prev_dash = true;
        }
        if out.len() >= 28 {
            break;
        }
    }
    let trimmed = out.trim_matches('-').to_string();
    if trimmed.is_empty() {
        fallback.to_string()
    } else {
        trimmed
    }
}

fn campaign_seed_key(
    campaign: &Campaign,
    phase: &Phase,
    proposal_type: &str,
    objective_id: &str,
) -> String {
    format!(
        "{}|{}|{}|{}",
        sanitize_token(&campaign.id, "campaign"),
        sanitize_token(&phase.id, "phase"),
        sanitize_token(proposal_type, "proposal"),
        sanitize_token(objective_id, "objective")
    )
}

fn campaign_seed_id(seed_key: &str) -> String {
    let compact = sanitize_token(&seed_key.replace('|', "-"), "seed");
    format!(
        "CAMP-{}",
        compact[..compact.len().min(52)].to_ascii_uppercase()
    )
}

fn existing_campaign_seed_keys(proposals: &[Value]) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for proposal in proposals {
        let key = as_lower(proposal.pointer("/meta/campaign_seed_key"), 240);
        if !key.is_empty() {
            out.insert(key);
        }
    }
    out
}

fn open_proposal_type_counts(proposals: &[Value]) -> BTreeMap<String, i64> {
    let mut counts = BTreeMap::new();
    for proposal in proposals {
        if is_terminal_proposal_status(&proposal_status_lower(proposal)) {
            continue;
        }
        let proposal_type = as_lower(proposal.get("type"), 120);
        if proposal_type.is_empty() {
            continue;
        }
        *counts.entry(proposal_type).or_insert(0) += 1;
    }
    counts
}
