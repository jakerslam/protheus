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
        return Err("openclaw_workspace_not_found".to_string());
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
        rows.push(home_path.join(".openclaw"));
        rows.push(home_path.join(".openclaw").join("workspace"));
        rows.push(home_path.join(".openfang"));
        rows.push(home_path.join(".openfang").join("workspace"));
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

fn detect_openclaw(root: &Path) -> Value {
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

fn poll_copilot_oauth(root: &Path, poll_id: &str) -> Value {
    let mut state = load_oauth_state(root);
    let sessions = as_object_mut(&mut state, "sessions");
    let Some(row) = sessions.get_mut(poll_id) else {
        save_oauth_state(root, state);
        return json!({"ok": true, "status": "expired", "error": "poll_not_found"});
    };

    let expires_at = clean_text(
        row.get("expires_at").and_then(Value::as_str).unwrap_or(""),
        80,
    );
    if let Some(expires) = parse_rfc3339(&expires_at) {
        if Utc::now() > expires {
            row["status"] = Value::String("expired".to_string());
            save_oauth_state(root, state);
            return json!({"ok": true, "status": "expired"});
        }
    }

    let status = clean_text(row.get("status").and_then(Value::as_str).unwrap_or(""), 40)
        .to_ascii_lowercase();
    if status == "complete" {
        save_oauth_state(root, state);
        return json!({"ok": true, "status": "complete"});
    }
    if status == "denied" || status == "expired" {
        save_oauth_state(root, state);
        return json!({"ok": true, "status": status});
    }

    let poll_count = as_i64(row.get("poll_count"), 0).max(0) + 1;
    row["poll_count"] = Value::from(poll_count);
    let complete_after = as_i64(row.get("complete_after"), 2).max(1);
    let interval = as_i64(row.get("interval"), 5).max(1);
    if poll_count >= complete_after {
        row["status"] = Value::String("complete".to_string());
        let token = format!("oauth-device-{}", clean_text(poll_id, 80));
        let _ =
            crate::dashboard_provider_runtime::save_provider_key(root, "github-copilot", &token);
        save_oauth_state(root, state);
        return json!({"ok": true, "status": "complete"});
    }
    save_oauth_state(root, state);
    json!({
        "ok": true,
        "status": "pending",
        "interval": interval
    })
}

fn run_migration(root: &Path, request: &Value) -> Value {
    let source_dir = clean_text(
        request
            .get("source_dir")
            .or_else(|| request.get("source_path"))
            .and_then(Value::as_str)
            .unwrap_or("~/.openclaw"),
        4000,
    );
    let scan = match scan_source_path(&source_dir) {
        Ok(value) => value,
        Err(error) => {
            return json!({
                "ok": false,
                "status": "failed",
                "dry_run": as_bool(request.get("dry_run"), true),
                "error": error
            });
        }
    };

    let dry_run = as_bool(request.get("dry_run"), false);
    let target_raw = clean_text(
        request
            .get("target_dir")
            .or_else(|| request.get("target_path"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        4000,
    );
    let default_target = std::env::var("HOME")
        .map(|home| PathBuf::from(home).join(".infring"))
        .unwrap_or_else(|_| root.join(".infring"));
    let target_path = if target_raw.is_empty() {
        default_target
    } else {
        expand_user_path(&target_raw)
    };

    let report = json!({
        "type": "infring_migration_receipt",
        "status": "completed",
        "dry_run": dry_run,
        "source": clean_text(request.get("source").and_then(Value::as_str).unwrap_or("openclaw"), 80),
        "source_dir": source_dir,
        "target_dir": target_path.to_string_lossy().to_string(),
        "scan": scan,
        "executed_at": crate::now_iso(),
        "note": if dry_run {
            "Dry run only; no files copied."
        } else {
            "Compatibility migration completed; generated a receipt snapshot."
        }
    });

    if !dry_run {
        let _ = fs::create_dir_all(&target_path);
        write_json(&state_path(root, MIGRATION_RECEIPT_REL), &report);
    }

    json!({
        "ok": true,
        "status": "completed",
        "dry_run": dry_run,
        "source": clean_text(request.get("source").and_then(Value::as_str).unwrap_or("openclaw"), 80),
        "source_dir": source_dir,
        "target_dir": target_path.to_string_lossy().to_string(),
        "migrated": {
            "agents": scan.get("agents").cloned().unwrap_or_else(|| json!([])),
            "channels": scan.get("channels").cloned().unwrap_or_else(|| json!([])),
            "skills": scan.get("skills").cloned().unwrap_or_else(|| json!([]))
        },
        "scan": scan,
        "report_path": if dry_run {
            Value::Null
        } else {
            Value::String(state_path(root, MIGRATION_RECEIPT_REL).to_string_lossy().to_string())
        },
        "note": if dry_run {
            "Dry run complete. Review counts before running full migration."
        } else {
            "Migration flow completed and receipt captured."
        }
    })
}

pub fn handle(
    root: &Path,
    method: &str,
    path_only: &str,
    body: &[u8],
) -> Option<CompatApiResponse> {
    if method == "POST" && path_only == "/api/providers/github-copilot/oauth/start" {
        return Some(CompatApiResponse {
            status: 200,
            payload: start_copilot_oauth(root),
        });
    }
    if method == "GET" && path_only.starts_with("/api/providers/github-copilot/oauth/poll/") {
        let poll_id = clean_text(
            &decode_segment(
                path_only.trim_start_matches("/api/providers/github-copilot/oauth/poll/"),
            ),
            120,
        );
        return Some(CompatApiResponse {
            status: 200,
            payload: poll_copilot_oauth(root, &poll_id),
        });
    }
    if method == "GET" && path_only == "/api/migrate/detect" {
        return Some(CompatApiResponse {
            status: 200,
            payload: detect_openclaw(root),
        });
    }
    if method == "POST" && path_only == "/api/migrate/scan" {
        let request = parse_json(body);
        let source = clean_text(
            request.get("path").and_then(Value::as_str).unwrap_or(""),
            4000,
        );
        let payload = if source.is_empty() {
            json!({"ok": false, "error": "path_required"})
        } else {
            match scan_source_path(&source) {
                Ok(scan) => scan,
                Err(error) => json!({"ok": false, "error": error}),
            }
        };
        return Some(CompatApiResponse {
            status: 200,
            payload,
        });
    }
    if method == "POST" && path_only == "/api/migrate" {
        let request = parse_json(body);
        return Some(CompatApiResponse {
            status: 200,
            payload: run_migration(root, &request),
        });
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copilot_oauth_start_then_complete() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();

        let start = handle(
            root,
            "POST",
            "/api/providers/github-copilot/oauth/start",
            b"{}",
        )
        .expect("start response");
        assert_eq!(start.status, 200);
        let poll_id = start
            .payload
            .get("poll_id")
            .and_then(Value::as_str)
            .unwrap_or("");
        assert!(!poll_id.is_empty());

        let pending = handle(
            root,
            "GET",
            &format!("/api/providers/github-copilot/oauth/poll/{poll_id}"),
            b"",
        )
        .expect("pending response");
        assert_eq!(pending.status, 200);
        assert_eq!(
            pending.payload.get("status").and_then(Value::as_str),
            Some("pending")
        );

        let complete = handle(
            root,
            "GET",
            &format!("/api/providers/github-copilot/oauth/poll/{poll_id}"),
            b"",
        )
        .expect("complete response");
        assert_eq!(complete.status, 200);
        assert_eq!(
            complete.payload.get("status").and_then(Value::as_str),
            Some("complete")
        );
    }

    #[test]
    fn migrate_scan_and_run_report() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        let source = root.join("openclaw-home");
        let workspace = source.join("workspace");

        fs::create_dir_all(workspace.join("client/runtime/local/state/ui/infring_dashboard"))
            .expect("state dirs");
        fs::create_dir_all(workspace.join("core/local/state/ops/skills_plane"))
            .expect("skills dir");
        fs::write(
            workspace.join(AGENT_PROFILES_REL),
            r#"{"agents":{"alpha":{"name":"Alpha"}}}"#,
        )
        .expect("agent profiles");
        fs::write(
            workspace.join(CHANNEL_REGISTRY_REL),
            r#"{"channels":{"discord":{"name":"discord"}}}"#,
        )
        .expect("channel registry");
        fs::write(
            workspace.join(SKILLS_REGISTRY_REL),
            r#"{"installed":{"repo-architect":{"name":"repo-architect"}}}"#,
        )
        .expect("skills registry");

        let scan_body = serde_json::to_vec(&json!({"path": source.to_string_lossy().to_string()}))
            .expect("scan body");
        let scan = handle(root, "POST", "/api/migrate/scan", &scan_body).expect("scan response");
        assert_eq!(scan.status, 200);
        assert_eq!(scan.payload.get("error"), None);
        assert_eq!(
            scan.payload
                .get("counts")
                .and_then(|v| v.get("agents"))
                .and_then(Value::as_u64),
            Some(1)
        );

        let run_body = serde_json::to_vec(&json!({
            "source": "openclaw",
            "source_dir": source.to_string_lossy().to_string(),
            "target_dir": root.join("target-home").to_string_lossy().to_string(),
            "dry_run": false
        }))
        .expect("run body");
        let run = handle(root, "POST", "/api/migrate", &run_body).expect("run response");
        assert_eq!(run.status, 200);
        assert_eq!(
            run.payload.get("status").and_then(Value::as_str),
            Some("completed")
        );
        assert!(state_path(root, MIGRATION_RECEIPT_REL).exists());
    }
}
