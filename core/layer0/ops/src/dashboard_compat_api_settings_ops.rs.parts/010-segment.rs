// Layer ownership: core/layer0/ops (authoritative)
// SPDX-License-Identifier: Apache-2.0
use chrono::{DateTime, Duration, Utc};
use serde_json::{json, Map, Value};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use super::CompatApiResponse;

const OAUTH_STATE_REL: &str = "client/runtime/local/state/ui/infring_dashboard/copilot_oauth.json";
const MIGRATION_RECEIPT_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/migration_last_run.json";
const AGENT_PROFILES_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/agent_profiles.json";
const CHANNEL_REGISTRY_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/channel_registry.json";
const SKILLS_REGISTRY_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/skills_registry.json";
const CORE_SKILLS_REGISTRY_REL: &str = "core/local/state/ops/skills_plane/registry.json";

fn clean_text(raw: &str, max_len: usize) -> String {
    raw.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .chars()
        .take(max_len)
        .collect::<String>()
}

fn decode_segment(raw: &str) -> String {
    urlencoding::decode(raw)
        .ok()
        .map(|value| value.to_string())
        .unwrap_or_else(|| raw.to_string())
}

fn state_path(root: &Path, rel: &str) -> PathBuf {
    root.join(rel)
}

fn read_json(path: &Path) -> Option<Value> {
    fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
}

fn write_json(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(raw) = serde_json::to_string_pretty(value) {
        let _ = fs::write(path, format!("{raw}\n"));
    }
}

fn parse_json(body: &[u8]) -> Value {
    serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}))
}

fn as_i64(value: Option<&Value>, fallback: i64) -> i64 {
    value.and_then(Value::as_i64).unwrap_or(fallback)
}

fn as_bool(value: Option<&Value>, fallback: bool) -> bool {
    value.and_then(Value::as_bool).unwrap_or(fallback)
}

fn as_object_mut<'a>(value: &'a mut Value, key: &str) -> &'a mut Map<String, Value> {
    if !value.get(key).map(Value::is_object).unwrap_or(false) {
        value[key] = Value::Object(Map::new());
    }
    value
        .get_mut(key)
        .and_then(Value::as_object_mut)
        .expect("object key must exist")
}

fn parse_rfc3339(raw: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(raw)
        .ok()
        .map(|v| v.with_timezone(&Utc))
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::Prefix(prefix) => out.push(prefix.as_os_str()),
            std::path::Component::RootDir => out.push(component.as_os_str()),
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                let _ = out.pop();
            }
            std::path::Component::Normal(part) => out.push(part),
        }
    }
    out
}

fn expand_user_path(raw: &str) -> PathBuf {
    let cleaned = clean_text(raw, 4000);
    if cleaned == "~" {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home);
        }
    }
    if let Some(rest) = cleaned.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return normalize_path(&PathBuf::from(home).join(rest));
        }
    }
    normalize_path(&PathBuf::from(cleaned))
}

fn detect_workspace_for_source(source: &Path) -> Option<PathBuf> {
    let normalized = normalize_path(source);
    if normalized.join("client").is_dir() && normalized.join("core").is_dir() {
        return Some(normalized);
    }
    let workspace = normalized.join("workspace");
    if workspace.join("client").is_dir() && workspace.join("core").is_dir() {
        return Some(workspace);
    }
    None
}

fn strings_from_object_keys(value: Option<&Map<String, Value>>, max: usize) -> Vec<Value> {
    let mut rows = value
        .map(|map| {
            map.keys()
                .filter_map(|key| {
                    let cleaned = clean_text(key, 160);
                    if cleaned.is_empty() {
                        None
                    } else {
                        Some(Value::String(cleaned))
                    }
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    rows.sort_by(|a, b| {
        clean_text(a.as_str().unwrap_or(""), 160).cmp(&clean_text(b.as_str().unwrap_or(""), 160))
    });
    if rows.len() > max {
        rows.truncate(max);
    }
    rows
}

fn scan_workspace(workspace: &Path) -> Value {
    let profiles = read_json(&workspace.join(AGENT_PROFILES_REL)).unwrap_or_else(|| json!({}));
    let channels = read_json(&workspace.join(CHANNEL_REGISTRY_REL)).unwrap_or_else(|| json!({}));
    let skills_dashboard =
        read_json(&workspace.join(SKILLS_REGISTRY_REL)).unwrap_or_else(|| json!({}));
    let skills_core =
        read_json(&workspace.join(CORE_SKILLS_REGISTRY_REL)).unwrap_or_else(|| json!({}));

    let agent_rows =
        strings_from_object_keys(profiles.get("agents").and_then(Value::as_object), 500);
    let channel_rows =
        strings_from_object_keys(channels.get("channels").and_then(Value::as_object), 500);

    let mut skill_set = HashSet::<String>::new();
    for section in ["installed", "created"] {
        if let Some(rows) = skills_dashboard.get(section).and_then(Value::as_object) {
            for key in rows.keys() {
                let cleaned = clean_text(key, 160);
                if !cleaned.is_empty() {
                    skill_set.insert(cleaned);
                }
            }
        }
    }
    if let Some(rows) = skills_core.get("installed").and_then(Value::as_object) {
        for key in rows.keys() {
            let cleaned = clean_text(key, 160);
            if !cleaned.is_empty() {
                skill_set.insert(cleaned);
            }
        }
    }
    let mut skill_rows = skill_set.into_iter().map(Value::String).collect::<Vec<_>>();
    skill_rows.sort_by(|a, b| {
        clean_text(a.as_str().unwrap_or(""), 160).cmp(&clean_text(b.as_str().unwrap_or(""), 160))
    });

    json!({
        "path": workspace.to_string_lossy().to_string(),
        "agents": agent_rows,
        "channels": channel_rows,
        "skills": skill_rows,
        "counts": {
            "agents": profiles.get("agents").and_then(Value::as_object).map(|v| v.len()).unwrap_or(0),
            "channels": channels.get("channels").and_then(Value::as_object).map(|v| v.len()).unwrap_or(0),
            "skills": skill_rows.len()
        }
    })
}

fn scan_source_path(source_path: &str) -> Result<Value, String> {
    let source = expand_user_path(source_path);
    if !source.exists() {
        return Err(format!(
            "source_path_not_found: {}",
            source.to_string_lossy()
        ));
    }
    let Some(workspace) = detect_workspace_for_source(&source) else {
        return Err("infring_workspace_not_found".to_string());
    };
    let mut scan = scan_workspace(&workspace);
    scan["path"] = Value::String(source.to_string_lossy().to_string());
    scan["workspace_path"] = Value::String(workspace.to_string_lossy().to_string());
    Ok(scan)
}

fn detect_paths(root: &Path) -> Vec<PathBuf> {
    let mut rows = Vec::<PathBuf>::new();
    rows.push(root.to_path_buf());
    if let Some(parent) = root.parent() {
        rows.push(parent.to_path_buf());
    }
    if let Ok(home) = std::env::var("HOME") {
        let home_path = PathBuf::from(home);
        rows.push(home_path.join(".infring"));
        rows.push(home_path.join(".infring").join("workspace"));
    }
    let mut seen = HashSet::<String>::new();
    rows.into_iter()
        .filter_map(|path| {
            let normalized = normalize_path(&path);
            let key = normalized.to_string_lossy().to_string();
            if key.is_empty() || !seen.insert(key) {
                None
            } else {
                Some(normalized)
            }
        })
        .collect::<Vec<_>>()
}

fn detect_infring(root: &Path) -> Value {
    let mut best: Option<(usize, PathBuf, Value)> = None;
    for candidate in detect_paths(root) {
        let source = candidate.to_string_lossy().to_string();
        let Ok(scan) = scan_source_path(&source) else {
            continue;
        };
        let score = scan
            .get("counts")
            .and_then(Value::as_object)
            .map(|counts| {
                counts
                    .values()
                    .filter_map(Value::as_u64)
                    .map(|v| v as usize)
                    .sum::<usize>()
            })
            .unwrap_or(0);
        match &best {
            None => best = Some((score, candidate, scan)),
            Some((best_score, _, _)) if score > *best_score => {
                best = Some((score, candidate, scan))
            }
            _ => {}
        }
    }
    if let Some((_, path, scan)) = best {
        return json!({
            "ok": true,
            "detected": true,
            "path": path.to_string_lossy().to_string(),
            "scan": scan
        });
    }
    json!({"ok": true, "detected": false})
}

fn load_oauth_state(root: &Path) -> Value {
    read_json(&state_path(root, OAUTH_STATE_REL)).unwrap_or_else(|| {
        json!({
            "type": "infring_dashboard_copilot_oauth",
            "updated_at": crate::now_iso(),
            "sessions": {}
        })
    })
}

fn save_oauth_state(root: &Path, mut state: Value) {
    state["updated_at"] = Value::String(crate::now_iso());
    write_json(&state_path(root, OAUTH_STATE_REL), &state);
}

fn oauth_poll_id() -> String {
    let hash = crate::deterministic_receipt_hash(&json!({
        "ts": crate::now_iso(),
        "pid": std::process::id()
    }));
    format!("copilot-{}", hash.chars().take(12).collect::<String>())
}

fn oauth_user_code(poll_id: &str) -> String {
    let cleaned = poll_id
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .map(|ch| ch.to_ascii_uppercase())
        .collect::<String>();
    let left = cleaned.chars().skip(2).take(4).collect::<String>();
    let right = cleaned.chars().skip(6).take(4).collect::<String>();
    format!(
        "{}-{}",
        if left.is_empty() { "ABCD" } else { &left },
        if right.is_empty() { "WXYZ" } else { &right }
    )
}

fn start_copilot_oauth(root: &Path) -> Value {
    let mut state = load_oauth_state(root);
    let poll_id = oauth_poll_id();
    let user_code = oauth_user_code(&poll_id);
    let created_at = crate::now_iso();
    let expires_at = (Utc::now() + Duration::minutes(15)).to_rfc3339();
    let sessions = as_object_mut(&mut state, "sessions");
    sessions.insert(
        poll_id.clone(),
        json!({
            "provider": "github-copilot",
            "status": "pending",
            "user_code": user_code,
            "verification_uri": "https://github.com/login/device",
            "interval": 5,
            "poll_count": 0,
            "complete_after": 2,
            "created_at": created_at,
            "expires_at": expires_at
        }),
    );
    save_oauth_state(root, state);
    json!({
        "ok": true,
        "provider": "github-copilot",
        "poll_id": poll_id,
        "user_code": user_code,
        "verification_uri": "https://github.com/login/device",
        "interval": 5,
        "expires_in": 900,
        "status": "pending"
    })
}

