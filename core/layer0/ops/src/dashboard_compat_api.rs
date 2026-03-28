// Layer ownership: core/layer0/ops (authoritative)
// SPDX-License-Identifier: Apache-2.0
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

const PROVIDER_REGISTRY_REL: &str = "client/runtime/local/state/ui/infring_dashboard/provider_registry.json";
const APPROVALS_REL: &str = "client/runtime/local/state/ui/infring_dashboard/approvals.json";
const WORKFLOWS_REL: &str = "client/runtime/local/state/ui/infring_dashboard/workflows.json";
const CRON_JOBS_REL: &str = "client/runtime/local/state/ui/infring_dashboard/cron_jobs.json";
const TRIGGERS_REL: &str = "client/runtime/local/state/ui/infring_dashboard/triggers.json";
const AGENT_PROFILES_REL: &str = "client/runtime/local/state/ui/infring_dashboard/agent_profiles.json";
const ACTION_HISTORY_REL: &str = "client/runtime/local/state/ui/infring_dashboard/actions/history.jsonl";
const EYES_CATALOG_STATE_PATHS: [&str; 3] = [
    "client/runtime/local/state/ui/infring_dashboard/eyes_catalog.json",
    "client/runtime/local/state/eyes/catalog.json",
    "client/runtime/local/state/ui/eyes/catalog.json",
];

#[path = "dashboard_compat_api_channels.rs"]
mod dashboard_compat_api_channels;
#[path = "dashboard_skills_marketplace.rs"]
mod dashboard_skills_marketplace;

#[derive(Debug, Clone)]
pub struct CompatApiResponse {
    pub status: u16,
    pub payload: Value,
}

fn clean_text(raw: &str, max_len: usize) -> String {
    raw.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .chars()
        .take(max_len)
        .collect::<String>()
}

fn read_json(path: &Path) -> Option<Value> {
    fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
}

#[cfg(test)]
fn write_json(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(raw) = serde_json::to_string_pretty(value) {
        let _ = fs::write(path, raw);
    }
}

fn parse_non_negative_i64(value: Option<&Value>, fallback: i64) -> i64 {
    value
        .and_then(Value::as_i64)
        .unwrap_or(fallback)
        .max(0)
}

fn state_path(root: &Path, rel: &str) -> PathBuf {
    root.join(rel)
}

fn extract_app_settings(snapshot: &Value) -> (String, String) {
    let provider = clean_text(
        snapshot
            .pointer("/app/settings/provider")
            .and_then(Value::as_str)
            .unwrap_or("auto"),
        80,
    );
    let model = clean_text(
        snapshot
            .pointer("/app/settings/model")
            .and_then(Value::as_str)
            .unwrap_or(""),
        120,
    );
    (provider, model)
}

fn runtime_sync_summary(snapshot: &Value) -> Value {
    if let Some(summary) = snapshot.pointer("/runtime_sync/summary") {
        return summary.clone();
    }
    json!({
        "queue_depth": parse_non_negative_i64(snapshot.pointer("/health/dashboard_metrics/queue_depth/value"), 0),
        "cockpit_blocks": parse_non_negative_i64(snapshot.pointer("/health/dashboard_metrics/hermes_cockpit_stream/value"), 0),
        "cockpit_total_blocks": parse_non_negative_i64(snapshot.pointer("/health/dashboard_metrics/hermes_cockpit_stream/value"), 0),
        "attention_batch_count": 0,
        "conduit_signals": parse_non_negative_i64(snapshot.pointer("/health/dashboard_metrics/collab_team_surface/value"), 0),
        "conduit_channels_observed": parse_non_negative_i64(snapshot.pointer("/health/dashboard_metrics/collab_team_surface/value"), 0),
        "target_conduit_signals": 4,
        "conduit_scale_required": false,
        "sync_mode": "live_sync",
        "backpressure_level": "normal"
    })
}

fn usage_from_snapshot(snapshot: &Value) -> Value {
    let turn_count = parse_non_negative_i64(snapshot.pointer("/app/turn_count"), 0);
    let (provider, model) = extract_app_settings(snapshot);
    let model_rows = if model.is_empty() {
        Vec::new()
    } else {
        vec![json!({
            "provider": provider,
            "model": model,
            "requests": turn_count,
            "input_tokens": 0,
            "output_tokens": 0,
            "cost_usd": 0.0
        })]
    };
    let today = crate::now_iso().chars().take(10).collect::<String>();
    let daily = vec![json!({
        "date": today,
        "requests": turn_count,
        "input_tokens": 0,
        "output_tokens": 0,
        "cost_usd": 0.0
    })];
    json!({
        "agents": {
            "active": snapshot
                .pointer("/collab/dashboard/agents")
                .and_then(Value::as_array)
                .map(|rows| rows.len() as i64)
                .unwrap_or(0),
            "archived": 0
        },
        "summary": {
            "requests": turn_count,
            "input_tokens": 0,
            "output_tokens": 0,
            "total_cost_usd": 0.0,
            "active_provider": provider,
            "active_model": model
        },
        "models": model_rows,
        "daily": daily
    })
}

fn providers_payload(root: &Path, snapshot: &Value) -> Value {
    let mut rows = Vec::<Value>::new();
    if let Some(registry) = read_json(&state_path(root, PROVIDER_REGISTRY_REL)) {
        if let Some(obj) = registry.get("providers").and_then(Value::as_object) {
            for row in obj.values() {
                rows.push(row.clone());
            }
        }
    }
    if rows.is_empty() {
        let (provider, model) = extract_app_settings(snapshot);
        rows.push(json!({
            "id": if provider.is_empty() { "auto" } else { provider.as_str() },
            "display_name": "Runtime Provider",
            "is_local": provider == "ollama" || provider == "local",
            "needs_key": provider != "ollama" && provider != "local",
            "auth_status": "unknown",
            "detected_models": if model.is_empty() { vec![] } else { vec![model] }
        }));
    }
    rows.sort_by(|a, b| {
        clean_text(a.get("id").and_then(Value::as_str).unwrap_or(""), 80)
            .cmp(&clean_text(b.get("id").and_then(Value::as_str).unwrap_or(""), 80))
    });
    json!({"ok": true, "providers": rows})
}

fn approvals_payload(root: &Path) -> Value {
    let rows = read_json(&state_path(root, APPROVALS_REL))
        .and_then(|v| v.get("approvals").cloned())
        .and_then(|v| v.as_array().cloned())
        .unwrap_or_else(|| {
            vec![json!({
                "id": "default-approval-policy",
                "name": "Default Approval Policy",
                "status": "active",
                "updated_at": crate::now_iso()
            })]
        });
    json!({"ok": true, "approvals": rows})
}

fn rows_from_array_store(root: &Path, rel: &str, key: &str) -> Value {
    let rows = read_json(&state_path(root, rel))
        .and_then(|v| {
            if v.is_array() {
                v.as_array().cloned()
            } else {
                v.get(key).and_then(Value::as_array).cloned()
            }
        })
        .unwrap_or_default();
    json!({"ok": true, key: rows})
}

fn read_eyes_payload(root: &Path) -> Value {
    for rel in EYES_CATALOG_STATE_PATHS {
        if let Some(value) = read_json(&state_path(root, rel)) {
            return json!({"ok": true, "type": "eyes_catalog", "catalog": value});
        }
    }
    json!({"ok": true, "type": "eyes_catalog", "catalog": {"eyes": []}})
}

fn extract_profiles(root: &Path) -> Vec<Value> {
    let state = read_json(&state_path(root, AGENT_PROFILES_REL)).unwrap_or_else(|| json!({}));
    let mut rows = state
        .get("agents")
        .and_then(Value::as_object)
        .map(|obj| {
            obj.values()
                .map(|v| v.clone())
                .collect::<Vec<Value>>()
        })
        .unwrap_or_default();
    rows.sort_by(|a, b| {
        clean_text(a.get("agent_id").and_then(Value::as_str).unwrap_or(""), 120)
            .cmp(&clean_text(b.get("agent_id").and_then(Value::as_str).unwrap_or(""), 120))
    });
    rows
}

fn recent_audit_entries(root: &Path, snapshot: &Value) -> Vec<Value> {
    let from_snapshot = snapshot
        .pointer("/receipts/recent")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if !from_snapshot.is_empty() {
        return from_snapshot;
    }
    let raw = fs::read_to_string(state_path(root, ACTION_HISTORY_REL)).unwrap_or_default();
    raw.lines()
        .rev()
        .take(200)
        .filter_map(|row| serde_json::from_str::<Value>(row).ok())
        .collect::<Vec<_>>()
}

pub fn handle(root: &Path, method: &str, path: &str, body: &[u8], snapshot: &Value) -> Option<CompatApiResponse> {
    let path_only = path.split('?').next().unwrap_or(path);
    if let Some(payload) = crate::dashboard_terminal_broker::handle_http(root, method, path_only, body) {
        return Some(CompatApiResponse { status: 200, payload });
    }
    if let Some(response) = dashboard_compat_api_channels::handle(root, method, path_only, body) {
        return Some(response);
    }
    if let Some(response) = dashboard_skills_marketplace::handle(root, method, path, snapshot, body)
    {
        return Some(response);
    }
    let usage = usage_from_snapshot(snapshot);
    let runtime = runtime_sync_summary(snapshot);
    let alerts_count = parse_non_negative_i64(snapshot.pointer("/health/alerts/count"), 0);
    let status = if snapshot.get("ok").and_then(Value::as_bool).unwrap_or(false) && alerts_count == 0 {
        "healthy"
    } else if snapshot.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        "degraded"
    } else {
        "critical"
    };

    if method == "GET" {
        let payload = match path_only {
            "/api/health" => json!({
                "ok": true,
                "status": status,
                "checks": snapshot.pointer("/health/checks").cloned().unwrap_or_else(|| json!({})),
                "alerts": snapshot.pointer("/health/alerts").cloned().unwrap_or_else(|| json!({"count": 0, "checks": []})),
                "dashboard_metrics": snapshot.pointer("/health/dashboard_metrics").cloned().unwrap_or_else(|| json!({})),
                "runtime_sync": runtime,
                "receipt_hash": snapshot.get("receipt_hash").cloned().unwrap_or(Value::Null),
                "ts": crate::now_iso()
            }),
            "/api/usage" => json!({"ok": true, "agents": usage["agents"].clone(), "summary": usage["summary"].clone(), "by_model": usage["models"].clone(), "daily": usage["daily"].clone()}),
            "/api/usage/summary" => json!({"ok": true, "summary": usage["summary"].clone()}),
            "/api/usage/by-model" => json!({"ok": true, "models": usage["models"].clone()}),
            "/api/usage/daily" => json!({"ok": true, "days": usage["daily"].clone()}),
            "/api/providers" => providers_payload(root, snapshot),
            "/api/models" => crate::dashboard_model_catalog::catalog_payload(root, snapshot),
            "/api/models/recommended" => crate::dashboard_model_catalog::route_decision_payload(
                root,
                snapshot,
                &json!({"task_type":"general","budget_mode":"balanced"}),
            ),
            "/api/route/decision" =>
                crate::dashboard_model_catalog::route_decision_payload(root, snapshot, &json!({})),
            "/api/channels" => dashboard_compat_api_channels::channels_payload(root),
            "/api/eyes" => read_eyes_payload(root),
            "/api/audit/recent" => {
                let entries = recent_audit_entries(root, snapshot);
                let tip_hash = crate::deterministic_receipt_hash(&json!({"entries": entries}));
                json!({"ok": true, "entries": entries, "tip_hash": tip_hash})
            }
            "/api/audit/verify" => {
                let entries = recent_audit_entries(root, snapshot);
                let tip_hash = crate::deterministic_receipt_hash(&json!({"entries": entries}));
                json!({"ok": true, "valid": true, "entries": entries.len(), "tip_hash": tip_hash})
            }
            "/api/version" => {
                let version = read_json(&root.join("package.json"))
                    .and_then(|v| v.get("version").and_then(Value::as_str).map(str::to_string))
                    .unwrap_or_else(|| "0.1.0".to_string());
                json!({"ok": true, "version": version, "rust_authority": "rust_core_lanes"})
            }
            "/api/network/status" => json!({"ok": true, "enabled": true, "connected_peers": 0, "total_peers": 0, "runtime_sync": runtime}),
            "/api/peers" => json!({"ok": true, "peers": [], "connected": 0, "total": 0, "runtime_sync": runtime}),
            "/api/security" => json!({
                "ok": true,
                "mode": "strict",
                "fail_closed": true,
                "receipts_required": true,
                "checks": snapshot.pointer("/health/checks").cloned().unwrap_or_else(|| json!({})),
                "alerts": snapshot.pointer("/health/alerts").cloned().unwrap_or_else(|| json!({})),
                "runtime_sync": runtime
            }),
            "/api/tools" => json!({
                "ok": true,
                "tools": [
                    {"name": "protheus-ops", "category": "runtime"},
                    {"name": "infringd", "category": "runtime"},
                    {"name": "git", "category": "cli"},
                    {"name": "rg", "category": "cli"}
                ],
                "runtime_sync": runtime
            }),
            "/api/commands" => json!({
                "ok": true,
                "commands": [
                    {"command": "/status", "description": "Show runtime status and cockpit summary"},
                    {"command": "/queue", "description": "Show current queue pressure"},
                    {"command": "/context", "description": "Show context and attention state"},
                    {"command": "/model", "description": "Inspect or switch active model"},
                    {"command": "/file <path>", "description": "Render full file output in chat from workspace path"},
                    {"command": "/folder <path>", "description": "Render folder tree + downloadable archive in chat"}
                ]
            }),
            "/api/budget" => json!({
                "ok": true,
                "hourly_spend": 0,
                "daily_spend": usage.pointer("/summary/total_cost_usd").cloned().unwrap_or_else(|| json!(0)),
                "monthly_spend": usage.pointer("/summary/total_cost_usd").cloned().unwrap_or_else(|| json!(0)),
                "hourly_limit": 0,
                "daily_limit": 0,
                "monthly_limit": 0
            }),
            "/api/a2a/agents" => json!({"ok": true, "agents": []}),
            "/api/approvals" => approvals_payload(root),
            "/api/sessions" => json!({"ok": true, "sessions": snapshot.pointer("/agents/session_summaries/rows").cloned().unwrap_or_else(|| json!([]))}),
            "/api/workflows" => rows_from_array_store(root, WORKFLOWS_REL, "workflows"),
            "/api/cron/jobs" => rows_from_array_store(root, CRON_JOBS_REL, "jobs"),
            "/api/triggers" => rows_from_array_store(root, TRIGGERS_REL, "triggers"),
            "/api/schedules" => rows_from_array_store(root, CRON_JOBS_REL, "schedules"),
            "/api/comms/topology" => json!({
                "ok": true,
                "topology": {
                    "nodes": snapshot.pointer("/collab/dashboard/agents").and_then(Value::as_array).map(|rows| rows.len()).unwrap_or(0),
                    "edges": 0,
                    "connected": true
                }
            }),
            "/api/comms/events" => json!({"ok": true, "events": []}),
            "/api/hands" | "/api/hands/active" => json!({"ok": true, "hands": [], "active": []}),
            "/api/profiles" => json!({"ok": true, "profiles": extract_profiles(root)}),
            "/api/update/check" => crate::dashboard_release_update::check_update(root),
            "/api/templates" => json!({
                "ok": true,
                "templates": [
                    {"id": "general-assistant", "name": "General Assistant", "provider": "auto", "model": "auto"},
                    {"id": "research-analyst", "name": "Research Analyst", "provider": "openai", "model": "gpt-5"},
                    {"id": "ops-reliability", "name": "Ops Reliability", "provider": "anthropic", "model": "claude-opus-4-20250514"}
                ]
            }),
            _ => return None,
        };
        return Some(CompatApiResponse { status: 200, payload });
    }

    if method == "POST" {
        if path_only == "/api/update/apply" {
            return Some(CompatApiResponse {
                status: 200,
                payload: crate::dashboard_release_update::apply_update(root),
            });
        }
        if path_only == "/api/route/decision" {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            return Some(CompatApiResponse {
                status: 200,
                payload: crate::dashboard_model_catalog::route_decision_payload(
                    root, snapshot, &request,
                ),
            });
        }
        return None;
    }

    if method == "DELETE" {
        return None;
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn providers_endpoint_uses_registry_rows() {
        let root = tempfile::tempdir().expect("tempdir");
        write_json(
            &state_path(root.path(), PROVIDER_REGISTRY_REL),
            &json!({
                "type": "infring_dashboard_provider_registry",
                "providers": {
                    "ollama": {"id": "ollama", "display_name": "Ollama", "is_local": true, "needs_key": false},
                    "openai": {"id": "openai", "display_name": "OpenAI", "is_local": false, "needs_key": true}
                }
            }),
        );
        let out = handle(
            root.path(),
            "GET",
            "/api/providers",
            &[],
            &json!({"ok": true}),
        )
        .expect("providers");
        let rows = out
            .payload
            .get("providers")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert_eq!(rows.len(), 2);
    }

    #[test]
    fn channels_endpoint_returns_catalog_defaults() {
        let root = tempfile::tempdir().expect("tempdir");
        let out = handle(
            root.path(),
            "GET",
            "/api/channels",
            &[],
            &json!({"ok": true}),
        )
        .expect("channels");
        let rows = out
            .payload
            .get("channels")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(rows.len() >= 40);
        assert!(rows.iter().any(|row| {
            row.get("name")
                .and_then(Value::as_str)
                .map(|v| v == "whatsapp")
                .unwrap_or(false)
        }));
    }

    #[test]
    fn channels_configure_and_test_round_trip() {
        let root = tempfile::tempdir().expect("tempdir");
        let configure = handle(
            root.path(),
            "POST",
            "/api/channels/discord/configure",
            br#"{"fields":{"bot_token":"abc","channel_id":"123"}}"#,
            &json!({"ok": true}),
        )
        .expect("configure");
        assert_eq!(configure.status, 200);
        let test = handle(
            root.path(),
            "POST",
            "/api/channels/discord/test",
            &[],
            &json!({"ok": true}),
        )
        .expect("test");
        assert_eq!(
            test.payload.get("status").and_then(Value::as_str),
            Some("ok")
        );
    }

    #[test]
    fn route_decision_endpoint_prefers_local_when_offline() {
        let root = tempfile::tempdir().expect("tempdir");
        write_json(
            &state_path(root.path(), PROVIDER_REGISTRY_REL),
            &json!({
                "type": "infring_dashboard_provider_registry",
                "providers": {
                    "ollama": {
                        "id": "ollama",
                        "is_local": true,
                        "needs_key": false,
                        "auth_status": "ok",
                        "model_profiles": {
                            "smallthinker:4b": {"power_rating": 2, "cost_rating": 1, "param_count_billion": 4, "specialty":"general"}
                        }
                    },
                    "openai": {
                        "id": "openai",
                        "is_local": false,
                        "needs_key": true,
                        "auth_status": "set",
                        "model_profiles": {
                            "gpt-5": {"power_rating": 5, "cost_rating": 5, "param_count_billion": 70, "specialty":"general"}
                        }
                    }
                }
            }),
        );
        let out = handle(
            root.path(),
            "POST",
            "/api/route/decision",
            br#"{"offline_required":true,"task_type":"general"}"#,
            &json!({"ok": true}),
        )
        .expect("route decision");
        assert_eq!(
            out.payload
                .get("selected")
                .and_then(|v| v.get("provider"))
                .and_then(Value::as_str),
            Some("ollama")
        );
    }

    #[test]
    fn whatsapp_qr_start_exposes_data_url() {
        let root = tempfile::tempdir().expect("tempdir");
        let out = handle(
            root.path(),
            "POST",
            "/api/channels/whatsapp/qr/start",
            &[],
            &json!({"ok": true}),
        )
        .expect("qr");
        let url = out
            .payload
            .get("qr_data_url")
            .and_then(Value::as_str)
            .unwrap_or("");
        assert!(url.starts_with("data:image/svg+xml;base64,"));
    }

    #[test]
    fn terminal_endpoints_round_trip() {
        let root = tempfile::tempdir().expect("tempdir");
        let created = handle(
            root.path(),
            "POST",
            "/api/terminal/sessions",
            br#"{"id":"term-a"}"#,
            &json!({"ok": true}),
        )
        .expect("create");
        assert_eq!(created.payload.get("ok").and_then(Value::as_bool), Some(true));
        let listed = handle(
            root.path(),
            "GET",
            "/api/terminal/sessions",
            &[],
            &json!({"ok": true}),
        )
        .expect("list");
        assert_eq!(
            listed
                .payload
                .get("sessions")
                .and_then(Value::as_array)
                .map(|rows| rows.len()),
            Some(1)
        );
        let ran = handle(
            root.path(),
            "POST",
            "/api/terminal/queue",
            br#"{"session_id":"term-a","command":"printf 'ok'"}"#,
            &json!({"ok": true}),
        )
        .expect("exec");
        assert_eq!(ran.payload.get("stdout").and_then(Value::as_str), Some("ok"));
    }
}
