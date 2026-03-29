#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RouteDecisionRequest {
    #[serde(default)]
    pub matched_habit_id: String,
    #[serde(default)]
    pub matched_habit_state: String,
    #[serde(default)]
    pub matched_reflex_id: String,
    #[serde(default)]
    pub reflex_eligible: bool,
    #[serde(default)]
    pub has_required_inputs: bool,
    #[serde(default)]
    pub required_input_count: i64,
    #[serde(default)]
    pub trusted_entrypoint: bool,
    #[serde(default)]
    pub any_trigger: bool,
    #[serde(default)]
    pub predicted_habit_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteDecisionResponse {
    pub ok: bool,
    pub decision: String,
    pub reason_code: String,
    pub suggested_habit_id: Option<String>,
    pub auto_habit_flow: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RouteHabitReadinessRequest {
    #[serde(default)]
    pub habit_state: String,
    #[serde(default)]
    pub entrypoint_resolved: String,
    #[serde(default)]
    pub trusted_entrypoints: Vec<String>,
    #[serde(default)]
    pub required_inputs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteHabitReadinessResponse {
    pub ok: bool,
    pub state: String,
    pub required_inputs: Vec<String>,
    pub trusted_entrypoint: bool,
    pub runnable: bool,
    pub reason_code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HeroicGateRequest {
    #[serde(default)]
    pub task_text: String,
    #[serde(default)]
    pub block_on_destructive: bool,
    #[serde(default)]
    pub purified_row: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeroicGateResponse {
    pub ok: bool,
    pub classification: String,
    pub decision: String,
    pub blocked: bool,
    pub reason_codes: Vec<String>,
}

#[derive(Debug, Clone)]
struct Segment {
    text: String,
    depth: usize,
    parent_id: Option<String>,
}

fn default_max_depth() -> usize {
    4
}
fn default_max_micro_tasks() -> usize {
    96
}
fn default_max_words_per_leaf() -> usize {
    18
}
fn default_min_minutes() -> usize {
    1
}
fn default_max_minutes() -> usize {
    5
}
fn default_max_groups() -> usize {
    8
}
fn default_lane() -> String {
    "autonomous_micro_agent".to_string()
}
fn default_storm_lane() -> String {
    "storm_human_lane".to_string()
}
fn default_autonomous_executor() -> String {
    "universal_execution_primitive".to_string()
}
fn default_storm_executor() -> String {
    "storm_human_lane".to_string()
}
fn default_min_storm_share() -> f64 {
    0.15
}
fn default_block_on_constitution_deny() -> bool {
    true
}

fn clean_text(raw: &str, max_len: usize) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut last_ws = false;
    for ch in raw.chars() {
        if ch.is_whitespace() {
            if !last_ws {
                out.push(' ');
                last_ws = true;
            }
        } else {
            out.push(ch);
            last_ws = false;
        }
    }
    let trimmed = out.trim();
    trimmed.chars().take(max_len).collect::<String>()
}

fn normalize_token(raw: &str, max_len: usize) -> String {
    let cleaned = clean_text(raw, max_len).to_lowercase();
    let mut out = String::with_capacity(cleaned.len());
    let mut last_underscore = false;
    for ch in cleaned.chars() {
        let allowed = ch.is_ascii_lowercase()
            || ch.is_ascii_digit()
            || ch == '_'
            || ch == '.'
            || ch == ':'
            || ch == '/'
            || ch == '-';
        if allowed {
            if ch == '_' {
                if !last_underscore {
                    out.push(ch);
                }
                last_underscore = true;
            } else {
                out.push(ch);
                last_underscore = false;
            }
        } else if !last_underscore {
            out.push('_');
            last_underscore = true;
        }
    }
    out.trim_matches('_').to_string()
}

fn sha16(seed: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(seed.as_bytes());
    let hex = hex::encode(hasher.finalize());
    hex.chars().take(16).collect()
}

fn word_count(text: &str) -> usize {
    text.split_whitespace().count()
}

fn split_candidates(text: &str) -> Vec<String> {
    let punct_re = Regex::new(r"[\n;]+").expect("valid punct regex");
    let connector_re =
        Regex::new(r"(?i)\b(?:and then|then|and|after|before|while|plus|also|with)\b")
            .expect("valid connector regex");

    let punct: Vec<String> = punct_re
        .split(text)
        .map(|row| clean_text(row, 800))
        .filter(|row| !row.is_empty())
        .collect();
    let rows = if punct.is_empty() {
        vec![text.to_string()]
    } else {
        punct
    };

    let mut out: Vec<String> = Vec::new();
    for row in rows {
        let split: Vec<String> = connector_re
            .split(row.as_str())
            .map(|part| clean_text(part, 600))
            .filter(|part| !part.is_empty())
            .collect();
        if split.len() > 1 {
            out.extend(split);
        } else if !row.is_empty() {
            out.push(row);
        }
    }
    out
}

fn recursive_decompose(
    text: &str,
    depth: usize,
    policy: &DecomposePolicy,
    parent: Option<String>,
) -> Vec<Segment> {
    let trimmed = clean_text(text, 1200);
    if trimmed.is_empty() {
        return Vec::new();
    }
    let words = word_count(trimmed.as_str());
    if depth >= policy.max_depth || words <= policy.max_words_per_leaf {
        return vec![Segment {
            text: trimmed,
            depth,
            parent_id: parent,
        }];
    }

    let candidates: Vec<String> = split_candidates(trimmed.as_str())
        .into_iter()
        .map(|row| clean_text(row.as_str(), 1000))
        .filter(|row| !row.is_empty() && row != &trimmed)
        .collect();
    if candidates.is_empty() {
        return vec![Segment {
            text: trimmed,
            depth,
            parent_id: parent,
        }];
    }

    let current_id = format!(
        "seg_{}",
        sha16(
            format!(
                "{}|{}",
                depth,
                trimmed.chars().take(120).collect::<String>()
            )
            .as_str()
        )
    );
    let mut out: Vec<Segment> = Vec::new();
    for candidate in candidates {
        out.extend(recursive_decompose(
            candidate.as_str(),
            depth + 1,
            policy,
            Some(current_id.clone()),
        ));
    }
    if out.is_empty() {
        vec![Segment {
            text: trimmed,
            depth,
            parent_id: parent,
        }]
    } else {
        out
    }
}

fn dedupe_segments(rows: Vec<Segment>, max_items: usize) -> Vec<Segment> {
    let mut out: Vec<Segment> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for row in rows {
        let key = normalize_token(row.text.as_str(), 220);
        if key.is_empty() || seen.contains(key.as_str()) {
            continue;
        }
        seen.insert(key);
        out.push(row);
        if out.len() >= max_items {
            break;
        }
    }
    out
}

fn estimate_minutes(text: &str, policy: &DecomposePolicy) -> usize {
    let words = word_count(text);
    let mut minutes = 1;
    if words > 8 {
        minutes = 2;
    }
    if words > 14 {
        minutes = 3;
    }
    if words > 24 {
        minutes = 4;
    }
    if words > 34 {
        minutes = 5;
    }
    minutes.clamp(policy.min_minutes, policy.max_minutes)
}

fn infer_capability(text: &str) -> Capability {
    let lower = text.to_lowercase();
    if Regex::new(r"\b(email|slack|discord|message|notify|outreach)\b")
        .expect("valid regex")
        .is_match(lower.as_str())
    {
        return Capability {
            capability_id: "comms_message".to_string(),
            adapter_kind: "email_message".to_string(),
            source_type: "comms".to_string(),
        };
    }
    if Regex::new(r"\b(browser|web|site|ui|form|click|navigate)\b")
        .expect("valid regex")
        .is_match(lower.as_str())
    {
        return Capability {
            capability_id: "browser_task".to_string(),
            adapter_kind: "browser_task".to_string(),
            source_type: "web_ui".to_string(),
        };
    }
    if Regex::new(r"\b(api|http|endpoint|request|json|graphql|webhook)\b")
        .expect("valid regex")
        .is_match(lower.as_str())
    {
        return Capability {
            capability_id: "api_request".to_string(),
            adapter_kind: "http_request".to_string(),
            source_type: "api".to_string(),
        };
    }
    if Regex::new(r"\b(file|document|write|save|edit|patch|code)\b")
        .expect("valid regex")
        .is_match(lower.as_str())
    {
        return Capability {
            capability_id: "filesystem_task".to_string(),
            adapter_kind: "filesystem_task".to_string(),
            source_type: "filesystem".to_string(),
        };
    }
    if Regex::new(r"\b(test|verify|assert|validate|check)\b")
        .expect("valid regex")
        .is_match(lower.as_str())
    {
        return Capability {
            capability_id: "quality_check".to_string(),
            adapter_kind: "shell_task".to_string(),
            source_type: "analysis".to_string(),
        };
    }
    if Regex::new(r"\b(research|analyze|summarize|read|investigate)\b")
        .expect("valid regex")
        .is_match(lower.as_str())
    {
        return Capability {
            capability_id: "analysis_task".to_string(),
            adapter_kind: "shell_task".to_string(),
            source_type: "analysis".to_string(),
        };
    }
    Capability {
        capability_id: "general_task".to_string(),
        adapter_kind: "shell_task".to_string(),
        source_type: "analysis".to_string(),
    }
}

fn title_for_task(text: &str) -> String {
    let words: Vec<&str> = text.split_whitespace().take(9).collect();
    if words.is_empty() {
        return "Micro Task".to_string();
    }
    let joined = words.join(" ");
    let mut chars = joined.chars();
    if let Some(first) = chars.next() {
        first.to_uppercase().collect::<String>() + chars.as_str()
    } else {
        "Micro Task".to_string()
    }
}

fn success_criteria(text: &str) -> Vec<String> {
    vec![
        format!("Execute: {}", clean_text(text, 180)),
        "Capture a receipt and link outcome to objective context.".to_string(),
    ]
}

fn normalize_keyword_rows(rows: &[String]) -> Vec<String> {
    rows.iter()
        .map(|row| normalize_token(row.as_str(), 80))
        .filter(|row| !row.is_empty())
        .collect()
}

fn lane_for_task(task_text: &str, policy: &DecomposePolicy) -> String {
    let lower = normalize_token(task_text, 500);
    let human_hits = normalize_keyword_rows(&policy.human_lane_keywords)
        .iter()
        .filter(|kw| lower.contains(kw.as_str()))
        .count();
    let auto_hits = normalize_keyword_rows(&policy.autonomous_lane_keywords)
        .iter()
        .filter(|kw| lower.contains(kw.as_str()))
        .count();
    if human_hits > auto_hits {
        policy.storm_lane.clone()
    } else {
        policy.default_lane.clone()
    }
}

fn normalize_capability(raw: &Capability) -> Capability {
    let capability_id = normalize_token(raw.capability_id.as_str(), 80);
    let adapter_kind = normalize_token(raw.adapter_kind.as_str(), 80);
    let source_type = normalize_token(raw.source_type.as_str(), 80);
    Capability {
        capability_id: if capability_id.is_empty() {
            "general_task".to_string()
        } else {
            capability_id
        },
        adapter_kind: if adapter_kind.is_empty() {
            "shell_task".to_string()
        } else {
            adapter_kind
        },
        source_type: if source_type.is_empty() {
            "analysis".to_string()
        } else {
            source_type
        },
    }
}
