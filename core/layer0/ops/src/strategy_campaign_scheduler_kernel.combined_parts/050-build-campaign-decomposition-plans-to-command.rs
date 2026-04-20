
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
