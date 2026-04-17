
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

fn build_campaign_decomposition_plans(
    proposals: &[Value],
    strategy: &Value,
    opts: &Map<String, Value>,
) -> Value {
    let campaigns = normalize_campaigns(strategy);
    let min_open_per_type = as_i64(opts.get("min_open_per_type")).unwrap_or(1).max(1);
    let max_additions = as_i64(opts.get("max_additions")).unwrap_or(0).max(0);
    let default_objective_id = clean_text(opts.get("default_objective_id"), 160);
    let default_risk = {
        let token = as_lower(opts.get("default_risk"), 40);
        if token.is_empty() {
            "low".to_string()
        } else {
            token
        }
    };
    let default_impact = {
        let token = as_lower(opts.get("default_impact"), 40);
        if token.is_empty() {
            "medium".to_string()
        } else {
            token
        }
    };
    if campaigns.is_empty() || max_additions == 0 {
        return json!({
            "enabled": !campaigns.is_empty(),
            "additions": [],
            "campaign_count": campaigns.len(),
            "min_open_per_type": min_open_per_type,
            "max_additions": max_additions
        });
    }

    let mut existing_ids = proposals
        .iter()
        .filter_map(|proposal| proposal.get("id").and_then(Value::as_str))
        .map(|row| row.to_string())
        .collect::<BTreeSet<_>>();
    let mut existing_keys = existing_campaign_seed_keys(proposals);
    let mut open_counts = open_proposal_type_counts(proposals);
    let mut additions = Vec::new();

    'campaigns: for campaign in campaigns {
        for phase in &campaign.phases {
            let objective_id = if !phase.objective_id.is_empty() {
                phase.objective_id.clone()
            } else if !campaign.objective_id.is_empty() {
                campaign.objective_id.clone()
            } else {
                default_objective_id.clone()
            };
            for proposal_type in &phase.proposal_types {
                if additions.len() as i64 >= max_additions {
                    break 'campaigns;
                }
                let open = *open_counts.get(proposal_type).unwrap_or(&0);
                if open >= min_open_per_type {
                    continue;
                }
                let seed_key = campaign_seed_key(
                    &campaign,
                    phase,
                    proposal_type,
                    if objective_id.is_empty() {
                        "objective"
                    } else {
                        &objective_id
                    },
                );
                if existing_keys.contains(&seed_key) {
                    continue;
                }
                let id = campaign_seed_id(&seed_key);
                if existing_ids.contains(&id) {
                    continue;
                }

                let campaign_name = if campaign.name.is_empty() {
                    campaign.id.clone()
                } else {
                    campaign.name.clone()
                };
                let phase_name = if phase.name.is_empty() {
                    phase.id.clone()
                } else {
                    phase.name.clone()
                };
                let objective_clause = if objective_id.is_empty() {
                    String::new()
                } else {
                    format!(" objective {objective_id}")
                };
                let task = format!(
                    "Create one bounded, deterministic action for campaign \"{campaign_name}\" phase \"{phase_name}\" proposal type \"{proposal_type}\" aligned to{objective_clause}. Use low-risk reversible steps with explicit verification and rollback."
                );
                let verify = json!([
                    "Route execution plan succeeds in dry-run",
                    "Success criteria include measurable checks",
                    "Rollback path remains available"
                ]);
                additions.push(json!({
                    "id": id,
                    "type": proposal_type,
                    "title": format!("[Campaign] {campaign_name} :: {phase_name} :: {proposal_type}"),
                    "summary": format!("Campaign decomposition seed for {campaign_name}/{phase_name} ({proposal_type})."),
                    "expected_impact": default_impact,
                    "risk": default_risk,
                    "validation": verify,
                    "suggested_next_command": format!("node systems/routing/route_execute.js --task=\"{task}\" --tokens_est=650 --repeats_14d=1 --errors_30d=0 --dry-run"),
                    "action_spec": {
                        "version": 1,
                        "objective": format!("Generate concrete {proposal_type} action for campaign {campaign_name}/{phase_name}"),
                        "objective_id": if objective_id.is_empty() { Value::Null } else { Value::String(objective_id.clone()) },
                        "next_command": format!("node systems/routing/route_execute.js --task=\"{task}\" --tokens_est=650 --repeats_14d=1 --errors_30d=0 --dry-run"),
                        "verify": verify,
                        "rollback": "Drop generated campaign seed proposal if verification fails"
                    },
                    "meta": {
                        "source_eye": "strategy_campaign",
                        "campaign_generated": true,
                        "campaign_id": campaign.id,
                        "campaign_name": campaign_name,
                        "campaign_priority": campaign.priority,
                        "campaign_phase_id": phase.id,
                        "campaign_phase_name": phase_name,
                        "campaign_phase_order": phase.order,
                        "campaign_seed_key": seed_key,
                        "objective_id": if objective_id.is_empty() { Value::Null } else { Value::String(objective_id.clone()) },
                        "directive_objective_id": if objective_id.is_empty() { Value::Null } else { Value::String(objective_id.clone()) },
                        "generated_at": now_iso()
                    }
                }));
                existing_ids.insert(id);
                existing_keys.insert(seed_key);
                open_counts.insert(proposal_type.clone(), open + 1);
            }
        }
    }

    json!({
        "enabled": true,
        "additions": additions,
        "campaign_count": normalize_campaigns(strategy).len(),
        "min_open_per_type": min_open_per_type,
        "max_additions": max_additions
    })
}

fn run_command(command: &str, payload: &Map<String, Value>) -> Result<Value, String> {
    match command {
        "normalize-campaigns" => {
            let strategy = payload
                .get("strategy")
                .cloned()
                .unwrap_or_else(|| json!({}));
            Ok(json!({
                "ok": true,
                "campaigns": campaigns_as_value(&normalize_campaigns(&strategy))
            }))
        }
        "annotate-priority" => {
            let strategy = payload
                .get("strategy")
                .cloned()
                .unwrap_or_else(|| json!({}));
            let candidates = as_array(payload.get("candidates"))
                .iter()
                .cloned()
                .collect::<Vec<_>>();
            let out = annotate_campaign_priority(&candidates, &strategy);
            Ok(json!({
                "ok": true,
                "summary": out.get("summary").cloned().unwrap_or_else(|| json!({})),
                "candidates": out.get("candidates").cloned().unwrap_or_else(|| json!([]))
            }))
        }
        "build-decomposition-plans" => {
            let strategy = payload
                .get("strategy")
                .cloned()
                .unwrap_or_else(|| json!({}));
            let proposals = as_array(payload.get("proposals"))
                .iter()
                .cloned()
                .collect::<Vec<_>>();
            let opts = as_object(payload.get("opts")).cloned().unwrap_or_default();
            let out = build_campaign_decomposition_plans(&proposals, &strategy, &opts);
            Ok(json!({
                "ok": true,
                "plan": out
            }))
        }
        _ => Err("strategy_campaign_scheduler_kernel_unknown_command".to_string()),
    }
}

pub fn run(_root: &std::path::Path, argv: &[String]) -> i32 {
    let Some(command) = argv.first().map(|v| v.as_str()) else {
        usage();
        return 1;
    };
    if matches!(command, "help" | "--help" | "-h") {
        usage();
        return 0;
    }
    let payload = match payload_json(argv) {
        Ok(value) => value,
        Err(err) => {
            lane_utils::print_json_line(&lane_utils::cli_error(
                "strategy_campaign_scheduler_kernel",
                &err,
            ));
            return 1;
        }
    };
    let payload = payload_obj(&payload).clone();
    match run_command(command, &payload) {
        Ok(out) => {
            lane_utils::print_json_line(&lane_utils::cli_receipt(
                "strategy_campaign_scheduler_kernel",
                out,
            ));
            0
        }
        Err(err) => {
            lane_utils::print_json_line(&lane_utils::cli_error(
                "strategy_campaign_scheduler_kernel",
                &err,
            ));
            1
        }
    }
}

#[cfg(test)]
