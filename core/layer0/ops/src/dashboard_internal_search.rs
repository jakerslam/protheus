// Layer ownership: core/layer0/ops (authoritative)
// SPDX-License-Identifier: Apache-2.0
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

const AGENT_PROFILES_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/agent_profiles.json";
const AGENT_CONTRACTS_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/agent_contracts.json";
const AGENT_SESSIONS_DIR_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/agent_sessions";
const MAX_INDEXED_LINES_PER_AGENT: usize = 320;
const MAX_LINE_CHARS: usize = 520;

fn clean_text(value: &str, max_len: usize) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(max_len)
        .collect::<String>()
        .trim()
        .to_string()
}

fn normalize_agent_id(raw: &str) -> String {
    let mut out = String::new();
    for ch in clean_text(raw, 180).chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
            out.push(ch);
        }
    }
    out
}

fn parse_json_loose(body: &str) -> Option<Value> {
    if body.trim().is_empty() {
        return None;
    }
    if let Ok(value) = serde_json::from_str::<Value>(body) {
        return Some(value);
    }
    for line in body.lines().rev() {
        let row = line.trim();
        if row.is_empty() {
            continue;
        }
        if let Ok(value) = serde_json::from_str::<Value>(row) {
            return Some(value);
        }
    }
    None
}

fn read_json_file(path: &Path) -> Option<Value> {
    let body = fs::read_to_string(path).ok()?;
    parse_json_loose(&body)
}

fn profiles_path(root: &Path) -> PathBuf {
    root.join(AGENT_PROFILES_REL)
}

fn contracts_path(root: &Path) -> PathBuf {
    root.join(AGENT_CONTRACTS_REL)
}

fn sessions_dir(root: &Path) -> PathBuf {
    root.join(AGENT_SESSIONS_DIR_REL)
}

fn is_stop_word(token: &str) -> bool {
    matches!(
        token,
        "a" | "an"
            | "and"
            | "are"
            | "as"
            | "at"
            | "by"
            | "for"
            | "from"
            | "if"
            | "in"
            | "into"
            | "is"
            | "it"
            | "of"
            | "on"
            | "or"
            | "that"
            | "the"
            | "then"
            | "to"
            | "up"
            | "was"
            | "were"
            | "with"
    )
}

fn tokenize_for_search(value: &str) -> Vec<String> {
    let mut out = Vec::<String>::new();
    let mut current = String::new();
    let push_current = |row: &mut String, target: &mut Vec<String>| {
        if row.is_empty() {
            return;
        }
        let token = row.to_ascii_lowercase();
        row.clear();
        if token.len() < 2 || is_stop_word(&token) {
            return;
        }
        target.push(token);
    };
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            current.push(ch);
            continue;
        }
        push_current(&mut current, &mut out);
    }
    push_current(&mut current, &mut out);
    out
}

fn humanize_agent_name(agent_id: &str) -> String {
    let normalized = normalize_agent_id(agent_id);
    if normalized.is_empty() {
        return "Agent".to_string();
    }
    let mut out = String::new();
    let mut capitalize = true;
    for ch in normalized.chars() {
        if ch == '-' || ch == '_' {
            out.push(' ');
            capitalize = true;
            continue;
        }
        if capitalize {
            out.extend(ch.to_uppercase());
            capitalize = false;
        } else {
            out.extend(ch.to_lowercase());
        }
    }
    clean_text(&out, 140)
}

fn text_from_message(row: &Value) -> String {
    let text = row
        .get("text")
        .and_then(Value::as_str)
        .or_else(|| row.get("content").and_then(Value::as_str))
        .unwrap_or("");
    clean_text(text, MAX_LINE_CHARS)
}

fn profile_map(root: &Path) -> HashMap<String, Value> {
    let mut out = HashMap::<String, Value>::new();
    let state = read_json_file(&profiles_path(root)).unwrap_or_else(|| json!({}));
    if let Some(agents) = state.get("agents").and_then(Value::as_object) {
        for (raw_id, profile) in agents {
            let id = normalize_agent_id(raw_id);
            if id.is_empty() {
                continue;
            }
            out.insert(id, profile.clone());
        }
    }
    out
}

fn contract_map(root: &Path) -> HashMap<String, Value> {
    let mut out = HashMap::<String, Value>::new();
    let state = read_json_file(&contracts_path(root)).unwrap_or_else(|| json!({}));
    if let Some(contracts) = state.get("contracts").and_then(Value::as_object) {
        for (raw_id, contract) in contracts {
            let id = normalize_agent_id(raw_id);
            if id.is_empty() {
                continue;
            }
            out.insert(id, contract.clone());
        }
    }
    out
}

#[derive(Clone, Debug)]
struct ConversationDocument {
    agent_id: String,
    name: String,
    archived: bool,
    state: String,
    avatar_url: String,
    emoji: String,
    updated_at: String,
    lines: Vec<String>,
}

fn collect_documents(root: &Path) -> Vec<ConversationDocument> {
    let profiles = profile_map(root);
    let contracts = contract_map(root);
    let archived_ids = crate::dashboard_agent_state::archived_agent_ids(root);
    let mut out = Vec::<ConversationDocument>::new();
    let mut seen = HashSet::<String>::new();
    let dir = sessions_dir(root);

    if let Ok(read_dir) = fs::read_dir(&dir) {
        for entry in read_dir.flatten() {
            let path = entry.path();
            if path.extension().and_then(|value| value.to_str()) != Some("json") {
                continue;
            }
            let state = match read_json_file(&path) {
                Some(value) => value,
                None => continue,
            };
            let mut agent_id = normalize_agent_id(
                state
                    .get("agent_id")
                    .and_then(Value::as_str)
                    .unwrap_or_default(),
            );
            if agent_id.is_empty() {
                let stem = path
                    .file_stem()
                    .and_then(|value| value.to_str())
                    .unwrap_or_default();
                agent_id = normalize_agent_id(stem);
            }
            if agent_id.is_empty() || seen.contains(&agent_id) {
                continue;
            }
            seen.insert(agent_id.clone());

            let profile = profiles.get(&agent_id);
            let contract = contracts.get(&agent_id);
            let active_id = clean_text(
                state
                    .get("active_session_id")
                    .and_then(Value::as_str)
                    .unwrap_or("default"),
                120,
            );
            let sessions = state
                .get("sessions")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            let active = sessions
                .iter()
                .find(|row| {
                    row.get("session_id")
                        .and_then(Value::as_str)
                        .map(|value| value == active_id)
                        .unwrap_or(false)
                })
                .cloned()
                .unwrap_or_else(|| sessions.first().cloned().unwrap_or_else(|| json!({})));
            let updated_at = clean_text(
                active
                    .get("updated_at")
                    .and_then(Value::as_str)
                    .or_else(|| state.get("updated_at").and_then(Value::as_str))
                    .unwrap_or(""),
                80,
            );
            let messages = active
                .get("messages")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            let mut lines = Vec::<String>::new();
            for message in messages {
                let line = text_from_message(&message);
                if line.is_empty() {
                    continue;
                }
                lines.push(line);
                if lines.len() >= MAX_INDEXED_LINES_PER_AGENT {
                    break;
                }
            }
            let name = clean_text(
                profile
                    .and_then(|row| row.get("name").and_then(Value::as_str))
                    .unwrap_or(""),
                140,
            );
            let name = if name.is_empty() {
                humanize_agent_name(&agent_id)
            } else {
                name
            };
            let avatar_url = clean_text(
                profile
                    .and_then(|row| row.get("avatar_url").and_then(Value::as_str))
                    .unwrap_or(""),
                480,
            );
            let emoji = clean_text(
                profile
                    .and_then(|row| row.get("identity"))
                    .and_then(|row| row.get("emoji"))
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                16,
            );
            let status = clean_text(
                contract
                    .and_then(|row| row.get("status").and_then(Value::as_str))
                    .unwrap_or("active"),
                40,
            )
            .to_ascii_lowercase();
            let archived = archived_ids.contains(&agent_id) || status == "terminated";
            let state_label = if archived {
                "archived".to_string()
            } else {
                clean_text(
                    profile
                        .and_then(|row| row.get("state").and_then(Value::as_str))
                        .unwrap_or("running"),
                    40,
                )
            };
            out.push(ConversationDocument {
                agent_id,
                name,
                archived,
                state: state_label,
                avatar_url,
                emoji,
                updated_at,
                lines,
            });
        }
    }

    for (agent_id, profile) in profiles {
        if seen.contains(&agent_id) {
            continue;
        }
        let name = clean_text(
            profile.get("name").and_then(Value::as_str).unwrap_or(""),
            140,
        );
        let avatar_url = clean_text(
            profile
                .get("avatar_url")
                .and_then(Value::as_str)
                .unwrap_or(""),
            480,
        );
        let emoji = clean_text(
            profile
                .get("identity")
                .and_then(|row| row.get("emoji"))
                .and_then(Value::as_str)
                .unwrap_or(""),
            16,
        );
        let contract_status = clean_text(
            contracts
                .get(&agent_id)
                .and_then(|row| row.get("status").and_then(Value::as_str))
                .unwrap_or("active"),
            40,
        )
        .to_ascii_lowercase();
        let archived = archived_ids.contains(&agent_id) || contract_status == "terminated";
        out.push(ConversationDocument {
            agent_id: agent_id.clone(),
            name: if name.is_empty() {
                humanize_agent_name(&agent_id)
            } else {
                name
            },
            archived,
            state: if archived {
                "archived".to_string()
            } else {
                clean_text(
                    profile
                        .get("state")
                        .and_then(Value::as_str)
                        .unwrap_or("running"),
                    40,
                )
            },
            avatar_url,
            emoji,
            updated_at: clean_text(
                profile
                    .get("updated_at")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                80,
            ),
            lines: Vec::new(),
        });
    }
    out
}

fn snippet_for_line(line: &str, terms: &[String]) -> String {
    let value = clean_text(line, 260);
    if value.is_empty() {
        return String::new();
    }
    let lower = value.to_ascii_lowercase();
    let mut focus_start = None::<usize>;
    for term in terms {
        if term.is_empty() {
            continue;
        }
        if let Some(idx) = lower.find(term) {
            focus_start = Some(idx);
            break;
        }
    }
    let (start, end) = if let Some(idx) = focus_start {
        let mut left = idx.saturating_sub(42);
        let mut right = (idx + 78).min(value.len());
        while left > 0 && !value.is_char_boundary(left) {
            left -= 1;
        }
        while right < value.len() && !value.is_char_boundary(right) {
            right += 1;
        }
        (left, right.min(value.len()))
    } else {
        (0, value.len().min(120))
    };
    let excerpt = value
        .get(start..end)
        .map(|row| clean_text(row, 180))
        .unwrap_or_else(|| clean_text(&value, 180));
    if excerpt.is_empty() {
        return String::new();
    }
    let words = excerpt.split_whitespace().collect::<Vec<_>>();
    if words.is_empty() {
        return String::new();
    }
    let first_term_hit = words.iter().position(|word| {
        let lw = word.to_ascii_lowercase();
        terms
            .iter()
            .any(|term| !term.is_empty() && lw.contains(term))
    });
    let first_meaningful = first_term_hit.unwrap_or_else(|| {
        words
            .iter()
            .position(|word| !is_stop_word(&word.to_ascii_lowercase()))
            .unwrap_or(0)
    });
    let compact = words[first_meaningful..].join(" ");
    let clipped = clean_text(&compact, 176);
    if clipped.is_empty() {
        return String::new();
    }
    format!("...[{}]...", clipped)
}

pub fn search_conversations(root: &Path, query: &str, limit: usize) -> Value {
    let cleaned_query = clean_text(query, 260);
    let terms = tokenize_for_search(&cleaned_query);
    if cleaned_query.is_empty() || terms.is_empty() {
        return json!({
            "ok": true,
            "type": "dashboard_conversation_search",
            "query": cleaned_query,
            "results": []
        });
    }

    let mut scored = Vec::<(i64, String, Value)>::new();
    for doc in collect_documents(root) {
        let name_lc = doc.name.to_ascii_lowercase();
        let mut score: i64 = 0;
        if name_lc == cleaned_query.to_ascii_lowercase() {
            score += 180;
        }
        if name_lc.starts_with(&cleaned_query.to_ascii_lowercase()) {
            score += 120;
        }
        if name_lc.contains(&cleaned_query.to_ascii_lowercase()) {
            score += 84;
        }
        let mut best_line_score = 0i64;
        let mut best_line = String::new();
        for line in &doc.lines {
            let line_lc = line.to_ascii_lowercase();
            let mut line_score = 0i64;
            if line_lc.contains(&cleaned_query.to_ascii_lowercase()) {
                line_score += 42;
            }
            for term in &terms {
                if name_lc.contains(term) {
                    score += 16;
                }
                if line_lc.contains(term) {
                    line_score += 10;
                }
            }
            if line_score > best_line_score {
                best_line_score = line_score;
                best_line = line.clone();
            }
        }
        score += best_line_score;
        if score <= 0 {
            continue;
        }
        let snippet = if !best_line.is_empty() {
            snippet_for_line(&best_line, &terms)
        } else {
            format!("...[{}]...", clean_text(&doc.name, 96))
        };
        let payload = json!({
            "agent_id": doc.agent_id,
            "name": doc.name,
            "snippet": snippet,
            "score": score,
            "archived": doc.archived,
            "state": doc.state,
            "avatar_url": doc.avatar_url,
            "emoji": doc.emoji,
            "updated_at": doc.updated_at
        });
        scored.push((
            score,
            payload
                .get("updated_at")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            payload,
        ));
    }
    scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| b.1.cmp(&a.1)));
    let capped = limit.clamp(1, 120);
    let results = scored
        .into_iter()
        .take(capped)
        .map(|(_, _, payload)| payload)
        .collect::<Vec<_>>();
    json!({
        "ok": true,
        "type": "dashboard_conversation_search",
        "query": cleaned_query,
        "terms": terms,
        "results": results
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_json(path: &Path, value: &Value) {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(
            path,
            serde_json::to_string_pretty(value).unwrap_or_else(|_| "{}".to_string()),
        );
    }

    #[test]
    fn search_returns_ranked_rows_with_snippet() {
        let root = tempfile::tempdir().expect("tempdir");
        write_json(
            &root.path().join(AGENT_PROFILES_REL),
            &json!({
                "agents": {
                    "agent-alpha": { "name": "Lucas", "identity": { "emoji": "🔬" } }
                }
            }),
        );
        write_json(
            &root
                .path()
                .join(AGENT_SESSIONS_DIR_REL)
                .join("agent-alpha.json"),
            &json!({
                "agent_id": "agent-alpha",
                "active_session_id": "default",
                "sessions": [{
                    "session_id": "default",
                    "updated_at": "2026-04-01T01:02:03Z",
                    "messages": [
                        {"role": "user", "text": "Fix websocket reconnect stability"},
                        {"role": "agent", "text": "I patched reconnect jitter and retry cadence"}
                    ]
                }]
            }),
        );
        let out = search_conversations(root.path(), "reconnect jitter", 10);
        let rows = out
            .get("results")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(!rows.is_empty());
        let first = rows.first().cloned().unwrap_or_else(|| json!({}));
        assert_eq!(
            first.get("agent_id").and_then(Value::as_str),
            Some("agent-alpha")
        );
        let snippet = first
            .get("snippet")
            .and_then(Value::as_str)
            .unwrap_or_default();
        assert!(snippet.starts_with("...["));
        assert!(snippet.ends_with("]..."));
    }

    #[test]
    fn archived_agents_are_included() {
        let root = tempfile::tempdir().expect("tempdir");
        write_json(
            &root.path().join(AGENT_PROFILES_REL),
            &json!({
                "agents": {
                    "agent-zed": { "name": "Zed" }
                }
            }),
        );
        write_json(
            &root.path().join(AGENT_CONTRACTS_REL),
            &json!({
                "contracts": {
                    "agent-zed": { "status": "terminated" }
                }
            }),
        );
        write_json(
            &root
                .path()
                .join("client/runtime/local/state/ui/infring_dashboard/archived_agents.json"),
            &json!({
                "agents": { "agent-zed": { "reason": "user_archive" } }
            }),
        );
        write_json(
            &root
                .path()
                .join(AGENT_SESSIONS_DIR_REL)
                .join("agent-zed.json"),
            &json!({
                "agent_id": "agent-zed",
                "active_session_id": "default",
                "sessions": [{
                    "session_id": "default",
                    "updated_at": "2026-03-30T00:00:00Z",
                    "messages": [
                        {"role": "user", "text": "Review archived onboarding plan"}
                    ]
                }]
            }),
        );
        let out = search_conversations(root.path(), "onboarding", 8);
        let row = out
            .get("results")
            .and_then(Value::as_array)
            .and_then(|rows| rows.first())
            .cloned()
            .unwrap_or_else(|| json!({}));
        assert_eq!(
            row.get("agent_id").and_then(Value::as_str),
            Some("agent-zed")
        );
        assert_eq!(row.get("archived").and_then(Value::as_bool), Some(true));
    }

    #[test]
    fn snippet_prefers_query_terms_over_connectors() {
        let snippet = snippet_for_line(
            "and with the reconnect jitter fix now reduces retries",
            &["reconnect".to_string(), "jitter".to_string()],
        );
        assert!(snippet.contains("[reconnect jitter"));
    }
}
