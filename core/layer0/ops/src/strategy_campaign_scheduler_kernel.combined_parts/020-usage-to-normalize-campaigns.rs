
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
