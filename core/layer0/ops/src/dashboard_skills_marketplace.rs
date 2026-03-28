// Layer ownership: core/layer0/ops (authoritative)
// SPDX-License-Identifier: Apache-2.0
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::dashboard_compat_api::CompatApiResponse;

const CORE_SKILLS_REGISTRY_REL: &str = "core/local/state/ops/skills_plane/registry.json";
const DASHBOARD_SKILLS_STATE_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/skills_registry.json";

fn clean_text(raw: &str, max_len: usize) -> String {
    raw.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .chars()
        .take(max_len)
        .collect::<String>()
}

fn normalize_name(raw: &str) -> String {
    clean_text(raw, 120)
        .to_lowercase()
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
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
        let _ = fs::write(path, raw);
    }
}

fn parse_json(raw: &[u8]) -> Value {
    serde_json::from_slice::<Value>(raw).unwrap_or_else(|_| json!({}))
}

fn parse_query(path: &str) -> Map<String, Value> {
    let mut out = Map::new();
    let query = path.split_once('?').map(|(_, q)| q).unwrap_or("");
    for pair in query.split('&') {
        if pair.is_empty() {
            continue;
        }
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        let key = clean_text(k, 80).to_lowercase();
        if key.is_empty() {
            continue;
        }
        let value = urlencoding::decode(v)
            .ok()
            .map(|s| s.to_string())
            .unwrap_or_default();
        out.insert(key, Value::String(clean_text(&value, 400)));
    }
    out
}

fn parse_u64(value: Option<&Value>, fallback: u64) -> u64 {
    value
        .and_then(|v| match v {
            Value::Number(_) => v.as_u64(),
            Value::String(s) => clean_text(s, 40).parse::<u64>().ok(),
            _ => None,
        })
        .unwrap_or(fallback)
}

fn as_object_mut<'a>(value: &'a mut Value, key: &str) -> &'a mut Map<String, Value> {
    if !value.get(key).map(Value::is_object).unwrap_or(false) {
        value[key] = Value::Object(Map::new());
    }
    value
        .get_mut(key)
        .and_then(Value::as_object_mut)
        .expect("object must exist")
}

fn load_dashboard_state(root: &Path) -> Value {
    read_json(&state_path(root, DASHBOARD_SKILLS_STATE_REL)).unwrap_or_else(|| {
        json!({
            "type": "infring_dashboard_skills_registry",
            "updated_at": crate::now_iso(),
            "installed": {},
            "created": {}
        })
    })
}

fn save_dashboard_state(root: &Path, mut state: Value) {
    state["updated_at"] = Value::String(crate::now_iso());
    write_json(&state_path(root, DASHBOARD_SKILLS_STATE_REL), &state);
}

fn default_tags() -> Vec<Value> {
    vec![Value::String("general".to_string())]
}

fn normalize_skill_row(mut row: Value, fallback_name: &str, source: Value) -> Value {
    let name = clean_text(
        row.get("name").and_then(Value::as_str).unwrap_or(fallback_name),
        120,
    );
    row["name"] = Value::String(name.clone());
    row["description"] = Value::String(clean_text(
        row.get("description")
            .and_then(Value::as_str)
            .unwrap_or("No description provided."),
        300,
    ));
    row["version"] = Value::String(clean_text(
        row.get("version").and_then(Value::as_str).unwrap_or("v1"),
        40,
    ));
    row["author"] = Value::String(clean_text(
        row.get("author").and_then(Value::as_str).unwrap_or("Unknown"),
        120,
    ));
    row["runtime"] = Value::String(clean_text(
        row.get("runtime")
            .and_then(Value::as_str)
            .unwrap_or("prompt_only"),
        40,
    ));
    row["tools_count"] = json!(parse_u64(row.get("tools_count"), 0));
    if !row.get("tags").map(Value::is_array).unwrap_or(false) {
        row["tags"] = Value::Array(default_tags());
    }
    row["enabled"] = Value::Bool(row.get("enabled").and_then(Value::as_bool).unwrap_or(true));
    row["has_prompt_context"] = Value::Bool(
        row.get("has_prompt_context")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    );
    row["source"] = source;
    row
}

fn installed_from_core(root: &Path) -> Vec<Value> {
    let registry = read_json(&state_path(root, CORE_SKILLS_REGISTRY_REL)).unwrap_or_else(|| json!({}));
    let installed = registry
        .get("installed")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let mut rows = installed
        .iter()
        .map(|(name, row)| {
            normalize_skill_row(
                row.clone(),
                name,
                json!({"type":"local","path": row.get("path").cloned().unwrap_or(Value::Null)}),
            )
        })
        .collect::<Vec<_>>();
    rows.sort_by(|a, b| {
        clean_text(a.get("name").and_then(Value::as_str).unwrap_or(""), 120)
            .cmp(&clean_text(b.get("name").and_then(Value::as_str).unwrap_or(""), 120))
    });
    rows
}

fn merged_installed_rows(root: &Path) -> Vec<Value> {
    let mut by_name = BTreeMap::<String, Value>::new();
    for row in installed_from_core(root) {
        let key = normalize_name(row.get("name").and_then(Value::as_str).unwrap_or(""));
        if !key.is_empty() {
            by_name.insert(key, row);
        }
    }

    let state = load_dashboard_state(root);
    for section in ["installed", "created"] {
        let rows = state
            .get(section)
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        for (name, row) in rows {
            let key = normalize_name(&name);
            if key.is_empty() {
                continue;
            }
            let source = row
                .get("source")
                .cloned()
                .unwrap_or_else(|| json!({"type":"local"}));
            by_name.insert(key, normalize_skill_row(row, &name, source));
        }
    }

    by_name.values().cloned().collect::<Vec<_>>()
}

fn marketplace_catalog() -> Vec<Value> {
    vec![
        json!({"slug":"repo-architect","name":"repo-architect","title":"Repo Architect","description":"Deep repository navigation and refactor planning agent skill.","author":"OpenFang","runtime":"prompt_only","tags":["coding","architecture"],"downloads":7421,"stars":1294,"updated_at":"2026-03-20T00:00:00Z","source":{"type":"clawhub","slug":"repo-architect"},"prompt_context":"Plan safe, incremental repository refactors with risk and rollback awareness."}),
        json!({"slug":"incident-commander","name":"incident-commander","title":"Incident Commander","description":"Operational incident triage and mitigation playbook automation.","author":"OpenFang","runtime":"prompt_only","tags":["devops","reliability"],"downloads":6200,"stars":1112,"updated_at":"2026-03-22T00:00:00Z","source":{"type":"clawhub","slug":"incident-commander"},"prompt_context":"Triage incidents, prioritize mitigation, and maintain operator-ready runbooks."}),
        json!({"slug":"whatsapp-bridge","name":"whatsapp-bridge","title":"WhatsApp Bridge","description":"Channel bridge for WhatsApp workflows and escalation routing.","author":"OpenFang","runtime":"node","tags":["communication","messaging"],"downloads":5902,"stars":980,"updated_at":"2026-03-19T00:00:00Z","source":{"type":"clawhub","slug":"whatsapp-bridge"},"prompt_context":"Bridge inbound and outbound WhatsApp workflows with strict audit receipts."}),
        json!({"slug":"slack-warroom","name":"slack-warroom","title":"Slack Warroom","description":"Coordinate incidents and launches in Slack with structured updates.","author":"OpenFang","runtime":"node","tags":["communication","devops"],"downloads":5544,"stars":932,"updated_at":"2026-03-18T00:00:00Z","source":{"type":"clawhub","slug":"slack-warroom"},"prompt_context":"Drive war-room workflows in Slack with concise, timestamped decision logs."}),
        json!({"slug":"signal-sentry","name":"signal-sentry","title":"Signal Sentry","description":"Signal channel adapter and high-priority alert fanout skill.","author":"OpenFang","runtime":"node","tags":["communication","security"],"downloads":5120,"stars":854,"updated_at":"2026-03-17T00:00:00Z","source":{"type":"clawhub","slug":"signal-sentry"},"prompt_context":"Relay critical security and reliability alerts into Signal threads."}),
        json!({"slug":"model-router-pro","name":"model-router-pro","title":"Model Router Pro","description":"Provider-agnostic model routing based on scope, latency, and budget.","author":"OpenFang","runtime":"prompt_only","tags":["ai","routing"],"downloads":8420,"stars":1543,"updated_at":"2026-03-24T00:00:00Z","source":{"type":"clawhub","slug":"model-router-pro"},"prompt_context":"Route tasks across local/cloud models by complexity and cost ceilings."}),
        json!({"slug":"code-audit-pack","name":"code-audit-pack","title":"Code Audit Pack","description":"Security and regression audit checklist for large codebases.","author":"OpenFang","runtime":"prompt_only","tags":["coding","security"],"downloads":7310,"stars":1381,"updated_at":"2026-03-23T00:00:00Z","source":{"type":"clawhub","slug":"code-audit-pack"},"prompt_context":"Perform high-signal code audits with severity-ranked findings and remediation plans."}),
        json!({"slug":"release-captain","name":"release-captain","title":"Release Captain","description":"Release orchestration, changelog synthesis, and rollback planning.","author":"OpenFang","runtime":"prompt_only","tags":["devops","release"],"downloads":4888,"stars":811,"updated_at":"2026-03-21T00:00:00Z","source":{"type":"clawhub","slug":"release-captain"},"prompt_context":"Coordinate release readiness checks, rollout, and rollback plans."}),
        json!({"slug":"conduit-optimizer","name":"conduit-optimizer","title":"Conduit Optimizer","description":"Analyze queue pressure and recommend conduit scaling remediations.","author":"OpenFang","runtime":"prompt_only","tags":["ops","reliability"],"downloads":5333,"stars":902,"updated_at":"2026-03-22T00:00:00Z","source":{"type":"clawhub","slug":"conduit-optimizer"},"prompt_context":"Tune queue, conduit, and cockpit pressure with deterministic remediation guidance."}),
        json!({"slug":"docs-distiller","name":"docs-distiller","title":"Docs Distiller","description":"Condense long docs into implementation-grade summaries.","author":"OpenFang","runtime":"prompt_only","tags":["docs","productivity"],"downloads":4511,"stars":776,"updated_at":"2026-03-20T00:00:00Z","source":{"type":"clawhub","slug":"docs-distiller"},"prompt_context":"Extract requirements and decisions from long documentation without losing constraints."}),
        json!({"slug":"browser-runner","name":"browser-runner","title":"Browser Runner","description":"Browser automation workflows with policy gates and approvals.","author":"OpenFang","runtime":"node","tags":["browser","automation"],"downloads":6922,"stars":1205,"updated_at":"2026-03-23T00:00:00Z","source":{"type":"clawhub","slug":"browser-runner"},"prompt_context":"Automate browser tasks with receipts and explicit approval checkpoints."}),
        json!({"slug":"research-deepdive","name":"research-deepdive","title":"Research Deepdive","description":"Structured research synthesis with source confidence tagging.","author":"OpenFang","runtime":"prompt_only","tags":["research","ai"],"downloads":5833,"stars":1009,"updated_at":"2026-03-21T00:00:00Z","source":{"type":"clawhub","slug":"research-deepdive"},"prompt_context":"Deliver concise, source-grounded research outputs with explicit confidence labels."}),
    ]
}

fn paginate(mut rows: Vec<Value>, query: &Map<String, Value>) -> Value {
    let limit = parse_u64(query.get("limit"), 20).clamp(1, 50) as usize;
    let cursor = parse_u64(query.get("cursor"), 0) as usize;
    let total = rows.len();
    if cursor >= rows.len() {
        rows.clear();
    } else {
        rows = rows.into_iter().skip(cursor).take(limit).collect::<Vec<_>>();
    }
    let next = cursor.saturating_add(limit);
    json!({
        "ok": true,
        "items": rows,
        "next_cursor": if next < total { Value::String(next.to_string()) } else { Value::Null }
    })
}

fn list_skills_payload(root: &Path) -> Value {
    json!({"ok": true, "skills": merged_installed_rows(root)})
}

fn mcp_servers_payload(snapshot: &Value) -> Value {
    let raw = snapshot
        .pointer("/skills/upstream/mcp_servers")
        .cloned()
        .unwrap_or_else(|| json!([]));
    if raw.get("configured").is_some() && raw.get("connected").is_some() {
        return raw;
    }
    let rows = raw.as_array().cloned().unwrap_or_default();
    let connected = rows
        .iter()
        .filter(|row| {
            row.get("connected").and_then(Value::as_bool).unwrap_or(false)
                || row
                    .get("status")
                    .and_then(Value::as_str)
                    .map(|v| v.eq_ignore_ascii_case("connected"))
                    .unwrap_or(false)
        })
        .cloned()
        .collect::<Vec<_>>();
    let configured = rows
        .iter()
        .filter(|row| {
            !row.get("connected").and_then(Value::as_bool).unwrap_or(false)
                && !row
                    .get("status")
                    .and_then(Value::as_str)
                    .map(|v| v.eq_ignore_ascii_case("connected"))
                    .unwrap_or(false)
        })
        .cloned()
        .collect::<Vec<_>>();
    json!({
        "configured": configured,
        "connected": connected,
        "total_configured": configured.len(),
        "total_connected": connected.len()
    })
}

fn browse_payload(path: &str) -> Value {
    let query = parse_query(path);
    let sort = clean_text(
        query
            .get("sort")
            .and_then(Value::as_str)
            .unwrap_or("trending"),
        40,
    )
    .to_lowercase();
    let mut rows = marketplace_catalog();
    rows.sort_by(|a, b| match sort.as_str() {
        "downloads" => parse_u64(b.get("downloads"), 0).cmp(&parse_u64(a.get("downloads"), 0)),
        "stars" => parse_u64(b.get("stars"), 0).cmp(&parse_u64(a.get("stars"), 0)),
        "updated" => clean_text(b.get("updated_at").and_then(Value::as_str).unwrap_or(""), 40)
            .cmp(&clean_text(
                a.get("updated_at").and_then(Value::as_str).unwrap_or(""),
                40,
            )),
        _ => {
            let score_a = parse_u64(a.get("downloads"), 0) + (parse_u64(a.get("stars"), 0) * 4);
            let score_b = parse_u64(b.get("downloads"), 0) + (parse_u64(b.get("stars"), 0) * 4);
            score_b.cmp(&score_a)
        }
    });
    paginate(rows, &query)
}

fn search_payload(path: &str) -> Value {
    let query = parse_query(path);
    let q = clean_text(query.get("q").and_then(Value::as_str).unwrap_or(""), 120).to_lowercase();
    let mut rows = marketplace_catalog();
    if !q.is_empty() {
        rows.retain(|row| {
            let tags = row
                .get("tags")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect::<Vec<_>>()
                .join(" ");
            let haystack = format!(
                "{} {} {} {}",
                row.get("slug").and_then(Value::as_str).unwrap_or(""),
                row.get("name").and_then(Value::as_str).unwrap_or(""),
                row.get("description").and_then(Value::as_str).unwrap_or(""),
                tags
            )
            .to_lowercase();
            haystack.contains(&q)
        });
    }
    paginate(rows, &query)
}

fn detail_payload(root: &Path, slug: &str) -> CompatApiResponse {
    let normalized = normalize_name(slug);
    let rows = marketplace_catalog();
    let Some(mut detail) = rows
        .into_iter()
        .find(|row| normalize_name(row.get("slug").and_then(Value::as_str).unwrap_or("")) == normalized)
    else {
        return CompatApiResponse {
            status: 404,
            payload: json!({"ok": false, "error": "skill_not_found"}),
        };
    };
    let installed = merged_installed_rows(root).into_iter().any(|row| {
        row.get("source")
            .and_then(Value::as_object)
            .and_then(|src| src.get("slug"))
            .and_then(Value::as_str)
            .map(|v| normalize_name(v) == normalized)
            .unwrap_or(false)
            || normalize_name(row.get("name").and_then(Value::as_str).unwrap_or("")) == normalized
    });
    detail["installed"] = Value::Bool(installed);
    CompatApiResponse {
        status: 200,
        payload: detail,
    }
}

fn detail_code_payload(slug: &str) -> CompatApiResponse {
    let normalized = normalize_name(slug);
    let rows = marketplace_catalog();
    let Some(detail) = rows
        .into_iter()
        .find(|row| normalize_name(row.get("slug").and_then(Value::as_str).unwrap_or("")) == normalized)
    else {
        return CompatApiResponse {
            status: 404,
            payload: json!({"ok": false, "error": "skill_not_found"}),
        };
    };
    let code = format!(
        "[skill]\nname = \"{}\"\nruntime = \"{}\"\ndescription = \"{}\"\n\n[prompt]\ncontext = \"{}\"\n",
        detail.get("name").and_then(Value::as_str).unwrap_or("unknown"),
        detail
            .get("runtime")
            .and_then(Value::as_str)
            .unwrap_or("prompt_only"),
        detail
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or(""),
        detail
            .get("prompt_context")
            .and_then(Value::as_str)
            .unwrap_or(""),
    );
    CompatApiResponse {
        status: 200,
        payload: json!({
            "ok": true,
            "filename": format!("{}.toml", normalized),
            "code": code
        }),
    }
}

fn install_payload(root: &Path, body: &[u8]) -> CompatApiResponse {
    let request = parse_json(body);
    let slug = normalize_name(request.get("slug").and_then(Value::as_str).unwrap_or(""));
    if slug.is_empty() {
        return CompatApiResponse {
            status: 400,
            payload: json!({"ok": false, "error": "slug_required"}),
        };
    }
    let Some(skill) = marketplace_catalog().into_iter().find(|row| {
        normalize_name(row.get("slug").and_then(Value::as_str).unwrap_or("")) == slug
    }) else {
        return CompatApiResponse {
            status: 404,
            payload: json!({"ok": false, "error": "skill_not_found"}),
        };
    };

    let mut state = load_dashboard_state(root);
    let installed = as_object_mut(&mut state, "installed");
    if installed.contains_key(&slug) {
        return CompatApiResponse {
            status: 409,
            payload: json!({"ok": false, "error": "already_installed"}),
        };
    }
    installed.insert(
        slug.clone(),
        json!({
            "name": skill.get("name").cloned().unwrap_or_else(|| Value::String(slug.clone())),
            "description": skill.get("description").cloned().unwrap_or_else(|| Value::String(String::new())),
            "version": "v1",
            "author": skill.get("author").cloned().unwrap_or_else(|| Value::String("OpenFang".to_string())),
            "runtime": skill.get("runtime").cloned().unwrap_or_else(|| Value::String("prompt_only".to_string())),
            "tools_count": 0,
            "tags": skill.get("tags").cloned().unwrap_or_else(|| Value::Array(default_tags())),
            "enabled": true,
            "has_prompt_context": true,
            "source": {"type":"clawhub","slug": slug},
            "installed_at": crate::now_iso()
        }),
    );
    save_dashboard_state(root, state);
    CompatApiResponse {
        status: 200,
        payload: json!({"ok": true, "name": skill.get("name").cloned().unwrap_or_else(|| Value::String(slug)), "warnings": []}),
    }
}

fn uninstall_payload(root: &Path, body: &[u8]) -> CompatApiResponse {
    let request = parse_json(body);
    let name = normalize_name(request.get("name").and_then(Value::as_str).unwrap_or(""));
    if name.is_empty() {
        return CompatApiResponse {
            status: 400,
            payload: json!({"ok": false, "error": "name_required"}),
        };
    }
    let mut state = load_dashboard_state(root);
    {
        let installed = as_object_mut(&mut state, "installed");
        installed.remove(&name);
    }
    {
        let created = as_object_mut(&mut state, "created");
        created.remove(&name);
    }
    save_dashboard_state(root, state);
    CompatApiResponse {
        status: 200,
        payload: json!({"ok": true}),
    }
}

fn create_payload(root: &Path, body: &[u8]) -> CompatApiResponse {
    let request = parse_json(body);
    let name_raw = request.get("name").and_then(Value::as_str).unwrap_or("");
    let name = normalize_name(name_raw);
    if name.is_empty() {
        return CompatApiResponse {
            status: 400,
            payload: json!({"ok": false, "error": "name_required"}),
        };
    }
    let mut state = load_dashboard_state(root);
    let created = as_object_mut(&mut state, "created");
    created.insert(
        name.clone(),
        json!({
            "name": name,
            "description": clean_text(request.get("description").and_then(Value::as_str).unwrap_or("User-created prompt skill"), 300),
            "version": "v1",
            "author": "User",
            "runtime": clean_text(request.get("runtime").and_then(Value::as_str).unwrap_or("prompt_only"), 40),
            "tools_count": 0,
            "tags": request.get("tags").cloned().filter(|v| v.is_array()).unwrap_or_else(|| Value::Array(default_tags())),
            "enabled": true,
            "has_prompt_context": true,
            "source": {"type":"local"},
            "prompt_context": clean_text(request.get("prompt_context").and_then(Value::as_str).unwrap_or(""), 4000),
            "created_at": crate::now_iso()
        }),
    );
    save_dashboard_state(root, state);
    CompatApiResponse {
        status: 200,
        payload: json!({"ok": true}),
    }
}

pub fn handle(
    root: &Path,
    method: &str,
    path: &str,
    snapshot: &Value,
    body: &[u8],
) -> Option<CompatApiResponse> {
    let path_only = path.split('?').next().unwrap_or(path);
    if method == "GET" {
        if path_only == "/api/skills" {
            return Some(CompatApiResponse {
                status: 200,
                payload: list_skills_payload(root),
            });
        }
        if path_only == "/api/mcp/servers" {
            return Some(CompatApiResponse {
                status: 200,
                payload: mcp_servers_payload(snapshot),
            });
        }
        if path_only == "/api/clawhub/browse" {
            return Some(CompatApiResponse {
                status: 200,
                payload: browse_payload(path),
            });
        }
        if path_only == "/api/clawhub/search" {
            return Some(CompatApiResponse {
                status: 200,
                payload: search_payload(path),
            });
        }
        if let Some(slug) = path_only.strip_prefix("/api/clawhub/skill/") {
            if let Some(clean_slug) = slug.strip_suffix("/code") {
                return Some(detail_code_payload(clean_slug));
            }
            return Some(detail_payload(root, slug));
        }
    }
    if method == "POST" {
        if path_only == "/api/clawhub/install" {
            return Some(install_payload(root, body));
        }
        if path_only == "/api/skills/uninstall" {
            return Some(uninstall_payload(root, body));
        }
        if path_only == "/api/skills/create" {
            return Some(create_payload(root, body));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn browse_and_search_are_paginated() {
        let root = tempfile::tempdir().expect("tempdir");
        let browse = handle(root.path(), "GET", "/api/clawhub/browse?sort=downloads&limit=5", &json!({}), &[])
            .expect("browse");
        let rows = browse
            .payload
            .get("items")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert_eq!(rows.len(), 5);
        let search = handle(root.path(), "GET", "/api/clawhub/search?q=router&limit=10", &json!({}), &[])
            .expect("search");
        let search_rows = search
            .payload
            .get("items")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(search_rows.iter().any(|row| {
            row.get("slug")
                .and_then(Value::as_str)
                .map(|v| v.contains("router"))
                .unwrap_or(false)
        }));
    }

    #[test]
    fn install_create_uninstall_round_trip() {
        let root = tempfile::tempdir().expect("tempdir");
        let installed = handle(
            root.path(),
            "POST",
            "/api/clawhub/install",
            &json!({}),
            br#"{"slug":"model-router-pro"}"#,
        )
        .expect("install");
        assert_eq!(installed.status, 200);
        let listed = handle(root.path(), "GET", "/api/skills", &json!({}), &[]).expect("skills");
        let rows = listed
            .payload
            .get("skills")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(rows.iter().any(|row| {
            row.get("name")
                .and_then(Value::as_str)
                .map(|v| v == "model-router-pro")
                .unwrap_or(false)
        }));

        let created = handle(
            root.path(),
            "POST",
            "/api/skills/create",
            &json!({}),
            br#"{"name":"my-demo-skill","description":"demo","runtime":"prompt_only","prompt_context":"ctx"}"#,
        )
        .expect("create");
        assert_eq!(created.status, 200);
        let removed = handle(
            root.path(),
            "POST",
            "/api/skills/uninstall",
            &json!({}),
            br#"{"name":"my-demo-skill"}"#,
        )
        .expect("uninstall");
        assert_eq!(removed.status, 200);
    }

    #[test]
    fn mcp_payload_normalizes_from_array() {
        let root = tempfile::tempdir().expect("tempdir");
        let snapshot = json!({
            "skills": {
                "upstream": {
                    "mcp_servers": [
                        {"name":"figma","connected":true},
                        {"name":"linear","connected":false}
                    ]
                }
            }
        });
        let out = handle(root.path(), "GET", "/api/mcp/servers", &snapshot, &[]).expect("mcp");
        assert_eq!(
            out.payload.get("total_connected").and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(
            out.payload.get("total_configured").and_then(Value::as_u64),
            Some(1)
        );
    }
}
