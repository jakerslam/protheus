use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecomposePolicy {
    #[serde(default = "default_max_depth")]
    pub max_depth: usize,
    #[serde(default = "default_max_micro_tasks")]
    pub max_micro_tasks: usize,
    #[serde(default = "default_max_words_per_leaf")]
    pub max_words_per_leaf: usize,
    #[serde(default = "default_min_minutes")]
    pub min_minutes: usize,
    #[serde(default = "default_max_minutes")]
    pub max_minutes: usize,
    #[serde(default = "default_max_groups")]
    pub max_groups: usize,
    #[serde(default = "default_lane")]
    pub default_lane: String,
    #[serde(default = "default_storm_lane")]
    pub storm_lane: String,
    #[serde(default)]
    pub human_lane_keywords: Vec<String>,
    #[serde(default)]
    pub autonomous_lane_keywords: Vec<String>,
    #[serde(default = "default_min_storm_share")]
    pub min_storm_share: f64,
}

impl Default for DecomposePolicy {
    fn default() -> Self {
        Self {
            max_depth: default_max_depth(),
            max_micro_tasks: default_max_micro_tasks(),
            max_words_per_leaf: default_max_words_per_leaf(),
            min_minutes: default_min_minutes(),
            max_minutes: default_max_minutes(),
            max_groups: default_max_groups(),
            default_lane: default_lane(),
            storm_lane: default_storm_lane(),
            human_lane_keywords: Vec::new(),
            autonomous_lane_keywords: Vec::new(),
            min_storm_share: default_min_storm_share(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DecomposeRequest {
    #[serde(default)]
    pub run_id: String,
    #[serde(default)]
    pub goal_id: String,
    #[serde(default)]
    pub goal_text: String,
    #[serde(default)]
    pub objective_id: Option<String>,
    #[serde(default)]
    pub creator_id: Option<String>,
    #[serde(default)]
    pub policy: DecomposePolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Capability {
    pub capability_id: String,
    pub adapter_kind: String,
    pub source_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BaseTask {
    pub micro_task_id: String,
    pub goal_id: String,
    pub objective_id: Option<String>,
    pub parent_id: Option<String>,
    pub depth: usize,
    pub index: usize,
    pub title: String,
    pub task_text: String,
    pub estimated_minutes: usize,
    pub success_criteria: Vec<String>,
    pub required_capability: String,
    pub profile_id: String,
    pub capability: Capability,
    pub suggested_lane: String,
    pub parallel_group: usize,
    pub parallel_priority: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecomposeResponse {
    pub ok: bool,
    pub tasks: Vec<BaseTask>,
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
fn default_min_storm_share() -> f64 {
    0.15
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

pub fn decompose_goal(req: &DecomposeRequest) -> Vec<BaseTask> {
    let run_id = if req.run_id.trim().is_empty() {
        format!(
            "tdp_{}",
            sha16(format!("{}|{}", req.goal_id, req.goal_text).as_str())
        )
    } else {
        req.run_id.trim().to_string()
    };
    let max_items = req.policy.max_micro_tasks.max(1);
    let segments = dedupe_segments(
        recursive_decompose(req.goal_text.as_str(), 0, &req.policy, None),
        max_items,
    );

    let mut tasks: Vec<BaseTask> = Vec::new();
    for (i, seg) in segments.into_iter().enumerate() {
        let task_text = clean_text(seg.text.as_str(), 1000);
        if task_text.is_empty() {
            continue;
        }
        let micro_task_id = format!(
            "mt_{}",
            sha16(format!("{}|{}|{}", run_id, i, task_text).as_str())
        );
        let capability = infer_capability(task_text.as_str());
        let minutes = estimate_minutes(task_text.as_str(), &req.policy);
        let profile_id = format!(
            "task_micro_{}",
            sha16(format!("{}|{}", req.goal_id, micro_task_id).as_str())
        );
        let lane = lane_for_task(task_text.as_str(), &req.policy);
        tasks.push(BaseTask {
            micro_task_id,
            goal_id: req.goal_id.clone(),
            objective_id: req.objective_id.clone(),
            parent_id: seg.parent_id,
            depth: seg.depth,
            index: i,
            title: title_for_task(task_text.as_str()),
            task_text: task_text.clone(),
            estimated_minutes: minutes,
            success_criteria: success_criteria(task_text.as_str()),
            required_capability: capability.capability_id.clone(),
            profile_id,
            capability,
            suggested_lane: lane,
            parallel_group: i % req.policy.max_groups.max(1),
            parallel_priority: 1f64 / (minutes.max(1) as f64),
        });
    }

    let human_count = tasks
        .iter()
        .filter(|task| task.suggested_lane == req.policy.storm_lane)
        .count();
    let human_share = if tasks.is_empty() {
        0f64
    } else {
        human_count as f64 / tasks.len() as f64
    };
    if tasks.len() > 2 && human_share < req.policy.min_storm_share {
        if let Some(first) = tasks.first_mut() {
            first.suggested_lane = req.policy.storm_lane.clone();
        }
    }

    tasks
}

pub fn decompose_goal_json(payload: &str) -> Result<String, String> {
    let req = serde_json::from_str::<DecomposeRequest>(payload)
        .map_err(|err| format!("decompose_payload_parse_failed:{}", err))?;
    let resp = DecomposeResponse {
        ok: true,
        tasks: decompose_goal(&req),
    };
    serde_json::to_string(&resp)
        .map_err(|err| format!("decompose_payload_serialize_failed:{}", err))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decompose_generates_micro_tasks() {
        let req = DecomposeRequest {
            run_id: "tdp_test".to_string(),
            goal_id: "goal_test".to_string(),
            goal_text: "Design a creative onboarding campaign and test API endpoint health checks then summarize findings".to_string(),
            objective_id: Some("obj_test".to_string()),
            creator_id: None,
            policy: DecomposePolicy {
                human_lane_keywords: vec!["creative".to_string(), "design".to_string()],
                autonomous_lane_keywords: vec!["test".to_string(), "api".to_string()],
                ..DecomposePolicy::default()
            },
        };
        let out = decompose_goal(&req);
        assert!(!out.is_empty());
        assert!(out.iter().all(|row| !row.micro_task_id.is_empty()));
        assert!(out.iter().all(|row| !row.profile_id.is_empty()));
    }
}
