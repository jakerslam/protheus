// Layer ownership: core/layer0/ops (authoritative)
// SPDX-License-Identifier: Apache-2.0
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::contract_lane_utils as lane_utils;
use crate::dashboard_compat_api::CompatApiResponse;

const CORE_SKILLS_REGISTRY_REL: &str = "core/local/state/ops/skills_plane/registry.json";
const DASHBOARD_SKILLS_STATE_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/skills_registry.json";

fn clean_text(raw: &str, max_len: usize) -> String {
    lane_utils::clean_text(Some(raw), max_len.max(1))
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
    lane_utils::read_json(path)
}

fn write_json(path: &Path, value: &Value) {
    let _ = lane_utils::write_json(path, value);
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

fn load_core_registry(root: &Path) -> Value {
    read_json(&state_path(root, CORE_SKILLS_REGISTRY_REL)).unwrap_or_else(|| {
        json!({
            "kind": "skills_registry",
            "installed": {}
        })
    })
}

fn save_core_registry(root: &Path, mut state: Value) {
    if !state.get("kind").map(Value::is_string).unwrap_or(false) {
        state["kind"] = Value::String("skills_registry".to_string());
    }
    write_json(&state_path(root, CORE_SKILLS_REGISTRY_REL), &state);
}

fn core_record_from_skill_row(skill_id: &str, row: &Value) -> Value {
    let source = row
        .get("source")
        .cloned()
        .unwrap_or_else(|| json!({"type":"local"}));
    let source_type = clean_text(
        source
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("local"),
        40,
    );
    let source_slug = clean_text(
        source
            .get("slug")
            .and_then(Value::as_str)
            .unwrap_or(skill_id),
        120,
    );
    let fallback_path = if source_type.eq_ignore_ascii_case("clawhub") {
        format!("clawhub://{source_slug}")
    } else {
        format!("dashboard://{skill_id}")
    };
    json!({
        "name": clean_text(row.get("name").and_then(Value::as_str).unwrap_or(skill_id), 120),
        "description": clean_text(row.get("description").and_then(Value::as_str).unwrap_or(""), 300),
        "version": clean_text(row.get("version").and_then(Value::as_str).unwrap_or("v1"), 40),
        "author": clean_text(row.get("author").and_then(Value::as_str).unwrap_or("Unknown"), 120),
        "runtime": clean_text(row.get("runtime").and_then(Value::as_str).unwrap_or("prompt_only"), 40),
        "tools_count": parse_u64(row.get("tools_count"), 0),
        "tags": row.get("tags").cloned().filter(|v| v.is_array()).unwrap_or_else(|| Value::Array(default_tags())),
        "enabled": row.get("enabled").and_then(Value::as_bool).unwrap_or(true),
        "has_prompt_context": row.get("has_prompt_context").and_then(Value::as_bool).unwrap_or(false),
        "prompt_context": clean_text(row.get("prompt_context").and_then(Value::as_str).unwrap_or(""), 4000),
        "source": source,
        "path": clean_text(row.get("path").and_then(Value::as_str).unwrap_or(&fallback_path), 512),
        "installed_at": row
            .get("installed_at")
            .cloned()
            .unwrap_or_else(|| Value::String(crate::now_iso())),
    })
}

fn upsert_core_installed_skill(root: &Path, skill_id: &str, row: &Value) {
    let key = normalize_name(skill_id);
    if key.is_empty() {
        return;
    }
    let mut state = load_core_registry(root);
    let installed = as_object_mut(&mut state, "installed");
    installed.insert(key.clone(), core_record_from_skill_row(&key, row));
    save_core_registry(root, state);
}

fn remove_core_installed_skill(root: &Path, skill_id: &str) {
    let key = normalize_name(skill_id);
    if key.is_empty() {
        return;
    }
    let mut state = load_core_registry(root);
    let installed = as_object_mut(&mut state, "installed");
    installed.remove(&key);
    save_core_registry(root, state);
}

fn default_tags() -> Vec<Value> {
    vec![Value::String("general".to_string())]
}

fn normalize_skill_row(mut row: Value, fallback_name: &str, source: Value) -> Value {
    let name = clean_text(
        row.get("name")
            .and_then(Value::as_str)
            .unwrap_or(fallback_name),
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
        row.get("author")
            .and_then(Value::as_str)
            .unwrap_or("Unknown"),
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
    let registry =
        read_json(&state_path(root, CORE_SKILLS_REGISTRY_REL)).unwrap_or_else(|| json!({}));
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
        clean_text(a.get("name").and_then(Value::as_str).unwrap_or(""), 120).cmp(&clean_text(
            b.get("name").and_then(Value::as_str).unwrap_or(""),
            120,
        ))
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

pub(super) fn skills_prompt_context(root: &Path, max_skills: usize, max_chars: usize) -> String {
    let mut rows = merged_installed_rows(root);
    rows.sort_by(|a, b| {
        clean_text(a.get("name").and_then(Value::as_str).unwrap_or(""), 120).cmp(&clean_text(
            b.get("name").and_then(Value::as_str).unwrap_or(""),
            120,
        ))
    });
    let mut lines = Vec::<String>::new();
    for row in rows {
        if lines.len() >= max_skills {
            break;
        }
        let enabled = row.get("enabled").and_then(Value::as_bool).unwrap_or(true);
        if !enabled {
            continue;
        }
        let context = clean_text(
            row.get("prompt_context")
                .and_then(Value::as_str)
                .unwrap_or(""),
            1200,
        );
        if context.is_empty() {
            continue;
        }
        let name = clean_text(
            row.get("name").and_then(Value::as_str).unwrap_or("plugin"),
            120,
        );
        lines.push(format!("- {name}: {context}"));
    }
    if lines.is_empty() {
        return String::new();
    }
    let text = format!(
        "Installed plugin context (apply naturally when relevant):\n{}",
        lines.join("\n")
    );
    text.chars().take(max_chars).collect::<String>()
}

fn marketplace_catalog() -> Vec<Value> {
    vec![
        json!({"slug":"repo-architect","name":"repo-architect","title":"Repo Architect","description":"Deep repository navigation and refactor planning agent skill.","author":"Infring","runtime":"prompt_only","tags":["coding","architecture"],"downloads":7421,"stars":1294,"updated_at":"2026-03-20T00:00:00Z","source":{"type":"clawhub","slug":"repo-architect"},"prompt_context":"Plan safe, incremental repository refactors with risk and rollback awareness."}),
        json!({"slug":"incident-commander","name":"incident-commander","title":"Incident Commander","description":"Operational incident triage and mitigation playbook automation.","author":"Infring","runtime":"prompt_only","tags":["devops","reliability"],"downloads":6200,"stars":1112,"updated_at":"2026-03-22T00:00:00Z","source":{"type":"clawhub","slug":"incident-commander"},"prompt_context":"Triage incidents, prioritize mitigation, and maintain operator-ready runbooks."}),
        json!({"slug":"whatsapp-bridge","name":"whatsapp-bridge","title":"WhatsApp Bridge","description":"Channel bridge for WhatsApp workflows and escalation routing.","author":"Infring","runtime":"node","tags":["communication","messaging"],"downloads":5902,"stars":980,"updated_at":"2026-03-19T00:00:00Z","source":{"type":"clawhub","slug":"whatsapp-bridge"},"prompt_context":"Bridge inbound and outbound WhatsApp workflows with strict audit receipts."}),
        json!({"slug":"slack-warroom","name":"slack-warroom","title":"Slack Warroom","description":"Coordinate incidents and launches in Slack with structured updates.","author":"Infring","runtime":"node","tags":["communication","devops"],"downloads":5544,"stars":932,"updated_at":"2026-03-18T00:00:00Z","source":{"type":"clawhub","slug":"slack-warroom"},"prompt_context":"Drive war-room workflows in Slack with concise, timestamped decision logs."}),
        json!({"slug":"signal-sentry","name":"signal-sentry","title":"Signal Sentry","description":"Signal channel adapter and high-priority alert fanout skill.","author":"Infring","runtime":"node","tags":["communication","security"],"downloads":5120,"stars":854,"updated_at":"2026-03-17T00:00:00Z","source":{"type":"clawhub","slug":"signal-sentry"},"prompt_context":"Relay critical security and reliability alerts into Signal threads."}),
        json!({"slug":"model-router-pro","name":"model-router-pro","title":"Model Router Pro","description":"Provider-agnostic model routing based on scope, latency, and budget.","author":"Infring","runtime":"prompt_only","tags":["ai","routing"],"downloads":8420,"stars":1543,"updated_at":"2026-03-24T00:00:00Z","source":{"type":"clawhub","slug":"model-router-pro"},"prompt_context":"Route tasks across local/cloud models by complexity and cost ceilings."}),
        json!({"slug":"code-audit-pack","name":"code-audit-pack","title":"Code Audit Pack","description":"Security and regression audit checklist for large codebases.","author":"Infring","runtime":"prompt_only","tags":["coding","security"],"downloads":7310,"stars":1381,"updated_at":"2026-03-23T00:00:00Z","source":{"type":"clawhub","slug":"code-audit-pack"},"prompt_context":"Perform high-signal code audits with severity-ranked findings and remediation plans."}),
        json!({"slug":"release-captain","name":"release-captain","title":"Release Captain","description":"Release orchestration, changelog synthesis, and rollback planning.","author":"Infring","runtime":"prompt_only","tags":["devops","release"],"downloads":4888,"stars":811,"updated_at":"2026-03-21T00:00:00Z","source":{"type":"clawhub","slug":"release-captain"},"prompt_context":"Coordinate release readiness checks, rollout, and rollback plans."}),
        json!({"slug":"conduit-optimizer","name":"conduit-optimizer","title":"Conduit Optimizer","description":"Analyze queue pressure and recommend conduit scaling remediations.","author":"Infring","runtime":"prompt_only","tags":["ops","reliability"],"downloads":5333,"stars":902,"updated_at":"2026-03-22T00:00:00Z","source":{"type":"clawhub","slug":"conduit-optimizer"},"prompt_context":"Tune queue, conduit, and cockpit pressure with deterministic remediation guidance."}),
        json!({"slug":"docs-distiller","name":"docs-distiller","title":"Docs Distiller","description":"Condense long docs into implementation-grade summaries.","author":"Infring","runtime":"prompt_only","tags":["docs","productivity"],"downloads":4511,"stars":776,"updated_at":"2026-03-20T00:00:00Z","source":{"type":"clawhub","slug":"docs-distiller"},"prompt_context":"Extract requirements and decisions from long documentation without losing constraints."}),
        json!({"slug":"browser-runner","name":"browser-runner","title":"Browser Runner","description":"Browser automation workflows with policy gates and approvals.","author":"Infring","runtime":"node","tags":["browser","automation"],"downloads":6922,"stars":1205,"updated_at":"2026-03-23T00:00:00Z","source":{"type":"clawhub","slug":"browser-runner"},"prompt_context":"Automate browser tasks with receipts and explicit approval checkpoints."}),
        json!({"slug":"research-deepdive","name":"research-deepdive","title":"Research Deepdive","description":"Structured research synthesis with source confidence tagging.","author":"Infring","runtime":"prompt_only","tags":["research","ai"],"downloads":5833,"stars":1009,"updated_at":"2026-03-21T00:00:00Z","source":{"type":"clawhub","slug":"research-deepdive"},"prompt_context":"Deliver concise, source-grounded research outputs with explicit confidence labels."}),
    ]
}

fn paginate(mut rows: Vec<Value>, query: &Map<String, Value>) -> Value {
    let limit = parse_u64(query.get("limit"), 20).clamp(1, 50) as usize;
    let cursor = parse_u64(query.get("cursor"), 0) as usize;
    let total = rows.len();
    if cursor >= rows.len() {
        rows.clear();
    } else {
        rows = rows
            .into_iter()
            .skip(cursor)
            .take(limit)
            .collect::<Vec<_>>();
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
        let configured = raw
            .get("configured")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let connected = raw
            .get("connected")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let mut servers = connected.clone();
        servers.extend(configured.clone());
        let total_configured = raw
            .get("total_configured")
            .and_then(Value::as_u64)
            .unwrap_or(configured.len() as u64);
        let total_connected = raw
            .get("total_connected")
            .and_then(Value::as_u64)
            .unwrap_or(connected.len() as u64);
        return json!({
            "configured": configured,
            "connected": connected,
            "servers": servers,
            "total_configured": total_configured,
            "total_connected": total_connected
        });
    }
    let rows = raw.as_array().cloned().unwrap_or_default();
    let connected = rows
        .iter()
        .filter(|row| {
            row.get("connected")
                .and_then(Value::as_bool)
                .unwrap_or(false)
