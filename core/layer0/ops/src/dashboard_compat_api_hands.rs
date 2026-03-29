// Layer ownership: core/layer0/ops (authoritative)
// SPDX-License-Identifier: Apache-2.0
use chrono::{DateTime, Utc};
use serde_json::{json, Map, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::CompatApiResponse;

const HANDS_STATE_REL: &str = "client/runtime/local/state/ui/infring_dashboard/hands_state.json";
const BROWSER_PLACEHOLDER_SCREENSHOT_BASE64: &str =
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO9f3n8AAAAASUVORK5CYII=";

fn clean_text(raw: &str, max_len: usize) -> String {
    raw.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .chars()
        .take(max_len)
        .collect::<String>()
}

fn clean_id(raw: &str, max_len: usize) -> String {
    let mut out = String::new();
    for ch in clean_text(raw, max_len).to_ascii_lowercase().chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
            out.push(ch);
        } else if ch == ' ' {
            out.push('-');
        }
    }
    while out.contains("--") {
        out = out.replace("--", "-");
    }
    out.trim_matches('-').to_string()
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

fn make_id(prefix: &str, seed: &Value) -> String {
    let hash = crate::deterministic_receipt_hash(seed);
    format!(
        "{}-{}",
        clean_id(prefix, 24),
        hash.chars().take(10).collect::<String>()
    )
}

fn as_bool(value: Option<&Value>, fallback: bool) -> bool {
    value.and_then(Value::as_bool).unwrap_or(fallback)
}

fn parse_rfc3339(raw: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(raw)
        .ok()
        .map(|v| v.with_timezone(&Utc))
}

fn server_platform() -> String {
    match std::env::consts::OS {
        "macos" => "macos".to_string(),
        "windows" => "windows".to_string(),
        _ => "linux".to_string(),
    }
}

fn command_available(command: &str) -> bool {
    let cmd = clean_id(command, 60);
    if cmd.is_empty() {
        return false;
    }
    if cfg!(windows) {
        Command::new("cmd")
            .args(["/C", "where", &cmd])
            .output()
            .map(|out| out.status.success())
            .unwrap_or(false)
    } else {
        Command::new("sh")
            .args(["-lc", &format!("command -v {cmd} >/dev/null 2>&1")])
            .output()
            .map(|out| out.status.success())
            .unwrap_or(false)
    }
}

fn env_present(key: &str) -> bool {
    let cleaned = clean_text(key, 120);
    if cleaned.is_empty() {
        return false;
    }
    std::env::var(cleaned)
        .map(|v| !v.trim().is_empty())
        .unwrap_or(false)
}

fn load_state(root: &Path) -> Value {
    read_json(&state_path(root, HANDS_STATE_REL)).unwrap_or_else(|| {
        json!({
            "type": "infring_dashboard_hands_state",
            "updated_at": crate::now_iso(),
            "instances": [],
            "hand_config": {}
        })
    })
}

fn save_state(root: &Path, mut state: Value) {
    state["updated_at"] = Value::String(crate::now_iso());
    write_json(&state_path(root, HANDS_STATE_REL), &state);
}

fn hand_config(state: &Value, hand_id: &str) -> Map<String, Value> {
    state
        .pointer(&format!("/hand_config/{}", clean_id(hand_id, 120)))
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default()
}

fn set_hand_config(state: &mut Value, hand_id: &str, config: &Map<String, Value>) {
    if !state
        .get("hand_config")
        .map(Value::is_object)
        .unwrap_or(false)
    {
        state["hand_config"] = Value::Object(Map::new());
    }
    if let Some(configs) = state.get_mut("hand_config").and_then(Value::as_object_mut) {
        configs.insert(clean_id(hand_id, 120), Value::Object(config.clone()));
    }
}

fn normalize_instance(instance: &Value) -> Value {
    let now = crate::now_iso();
    let instance_id = {
        let raw = clean_id(
            instance
                .get("instance_id")
                .and_then(Value::as_str)
                .unwrap_or(""),
            120,
        );
        if raw.is_empty() {
            make_id(
                "inst",
                &json!({"hand_id": instance.get("hand_id").cloned().unwrap_or(Value::Null), "ts": now}),
            )
        } else {
            raw
        }
    };
    let hand_id = clean_id(
        instance
            .get("hand_id")
            .and_then(Value::as_str)
            .unwrap_or(""),
        80,
    );
    let agent_id = clean_id(
        instance
            .get("agent_id")
            .and_then(Value::as_str)
            .unwrap_or(""),
        120,
    );
    let agent_name = clean_text(
        instance
            .get("agent_name")
            .and_then(Value::as_str)
            .unwrap_or(""),
        140,
    );
    let status = clean_text(
        instance
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("Active"),
        40,
    );
    let activated_at = clean_text(
        instance
            .get("activated_at")
            .and_then(Value::as_str)
            .unwrap_or(&now),
        80,
    );
    json!({
        "instance_id": instance_id,
        "hand_id": hand_id,
        "agent_id": agent_id,
        "agent_name": agent_name,
        "status": if status.is_empty() { "Active" } else { &status },
        "activated_at": if activated_at.is_empty() { &now } else { &activated_at },
        "updated_at": clean_text(instance.get("updated_at").and_then(Value::as_str).unwrap_or(&now), 80),
        "config": instance.get("config").cloned().unwrap_or_else(|| json!({}))
    })
}

fn load_instances(state: &Value) -> Vec<Value> {
    let mut rows = state
        .get("instances")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .map(normalize_instance)
        .collect::<Vec<_>>();
    rows.sort_by_key(|row| {
        std::cmp::Reverse(clean_text(
            row.get("activated_at")
                .and_then(Value::as_str)
                .unwrap_or(""),
            80,
        ))
    });
    rows
}

fn set_instances(state: &mut Value, instances: Vec<Value>) {
    state["instances"] = Value::Array(instances);
}

fn base_catalog(root: &Path, snapshot: &Value) -> Vec<Value> {
    let (provider, model) = super::effective_app_settings(root, snapshot);
    let fallback_provider = if provider.is_empty() {
        "auto".to_string()
    } else {
        provider
    };
    let fallback_model = if model.is_empty() {
        "auto".to_string()
    } else {
        model
    };
    vec![
        json!({
            "id": "browser",
            "name": "Browser Hand",
            "icon": "🌐",
            "category": "automation",
            "description": "Autonomous browser operator for navigation, extraction, and verification loops.",
            "tools": ["browser", "fetch", "terminal"],
            "agent": {
                "provider": fallback_provider,
                "model": fallback_model,
                "role": "browser-operator",
                "system_prompt": "You are Browser Hand. Use safe autonomous browsing and summarize findings with evidence."
            },
            "settings": [
                {"key": "start_url", "label": "Start URL", "setting_type": "text", "default": "https://example.com"},
                {"key": "goal", "label": "Primary Goal", "setting_type": "text", "default": "Collect high-signal page insights"},
                {"key": "headless", "label": "Headless Mode", "setting_type": "toggle", "default": "true"}
            ],
            "requirements": [
                {
                    "key": "node",
                    "label": "Node.js runtime",
                    "type": "Command",
                    "check_value": "node",
                    "install": {
                        "macos": "brew install node",
                        "linux_apt": "sudo apt-get install -y nodejs npm",
                        "windows": "winget install OpenJS.NodeJS",
                        "estimated_time": "2-5 min"
                    }
                }
            ],
            "dashboard": [
                {"memory_key": "browser_pages_visited", "label": "Pages Visited", "format": "number"},
                {"memory_key": "browser_last_url", "label": "Last URL", "format": "text"},
                {"memory_key": "browser_uptime", "label": "Uptime", "format": "duration"}
            ]
        }),
        json!({
            "id": "trader",
            "name": "Trader Hand",
            "icon": "📈",
            "category": "finance",
            "description": "Portfolio-aware execution hand for strategy monitoring, risk signals, and market actions.",
            "tools": ["market-data", "risk-monitor", "portfolio"],
            "agent": {
                "provider": fallback_provider,
                "model": fallback_model,
                "role": "trading-operator",
                "system_prompt": "You are Trader Hand. Prioritize risk controls, transparent assumptions, and measurable outcomes."
            },
            "settings": [
                {"key": "risk_mode", "label": "Risk Mode", "setting_type": "select", "default": "balanced", "options": [{"label":"Conservative","value":"conservative"}, {"label":"Balanced","value":"balanced"}, {"label":"Aggressive","value":"aggressive"}]},
                {"key": "watchlist", "label": "Watchlist", "setting_type": "text", "default": "AAPL,MSFT,NVDA"},
                {"key": "paper_mode", "label": "Paper Trading", "setting_type": "toggle", "default": "true"},
                {"key": "alpaca_api_key", "label": "Alpaca API Key", "setting_type": "password", "default": ""},
                {"key": "alpaca_secret_key", "label": "Alpaca Secret Key", "setting_type": "password", "default": ""}
            ],
            "requirements": [
                {
                    "key": "ALPACA_API_KEY",
                    "label": "Alpaca API Key",
                    "type": "ApiKey",
                    "check_value": "ALPACA_API_KEY"
                },
                {
                    "key": "ALPACA_SECRET_KEY",
                    "label": "Alpaca Secret Key",
                    "type": "ApiKey",
                    "check_value": "ALPACA_SECRET_KEY"
                }
            ],
            "dashboard": [
                {"memory_key": "trader_hand_portfolio_value", "label": "Portfolio Value", "format": "number"},
                {"memory_key": "trader_hand_total_pnl", "label": "Total P&L", "format": "number"},
                {"memory_key": "trader_hand_win_rate", "label": "Win Rate", "format": "text"},
                {"memory_key": "trader_hand_sharpe_ratio", "label": "Sharpe Ratio", "format": "number"},
                {"memory_key": "trader_hand_max_drawdown", "label": "Max Drawdown", "format": "text"},
                {"memory_key": "trader_hand_trades_count", "label": "Trades Executed", "format": "number"}
            ]
        }),
        json!({
            "id": "reliability",
            "name": "Reliability Hand",
            "icon": "🛡️",
            "category": "operations",
            "description": "Monitors system drift, queue pressure, and restart safety with actionable runbooks.",
            "tools": ["diagnostics", "queue", "receipts"],
            "agent": {
                "provider": fallback_provider,
                "model": fallback_model,
                "role": "ops-reliability",
                "system_prompt": "You are Reliability Hand. Focus on stability, alert triage, and fail-closed remediation steps."
            },
            "settings": [
                {"key": "monitor_interval_sec", "label": "Monitor Interval (sec)", "setting_type": "text", "default": "30"},
                {"key": "auto_remediate", "label": "Auto Remediate", "setting_type": "toggle", "default": "false"}
            ],
            "requirements": [],
            "dashboard": [
                {"memory_key": "runtime_queue_depth", "label": "Queue Depth", "format": "number"},
                {"memory_key": "runtime_conduit_signals", "label": "Conduit Signals", "format": "number"},
                {"memory_key": "runtime_cockpit_blocks", "label": "Cockpit Blocks", "format": "number"}
            ]
        }),
    ]
}

fn requirement_satisfied(requirement: &Value, config: &Map<String, Value>) -> bool {
    let req_type = clean_text(
        requirement
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or(""),
        40,
    );
    if req_type.eq_ignore_ascii_case("ApiKey") {
        let env_key = clean_text(
            requirement
                .get("check_value")
                .and_then(Value::as_str)
                .or_else(|| requirement.get("key").and_then(Value::as_str))
                .unwrap_or(""),
            120,
        );
        if env_present(&env_key) {
            return true;
        }
        let cfg_key = clean_id(&env_key.to_ascii_lowercase(), 120);
        if !cfg_key.is_empty() {
            let from_cfg = config
                .get(&cfg_key)
                .and_then(Value::as_str)
                .map(|v| !v.trim().is_empty())
                .unwrap_or(false);
            if from_cfg {
                return true;
            }
        }
        return false;
    }
    if req_type.eq_ignore_ascii_case("Command") {
        let command = clean_text(
            requirement
                .get("check_value")
                .and_then(Value::as_str)
                .or_else(|| requirement.get("key").and_then(Value::as_str))
                .unwrap_or(""),
            80,
        );
        return command_available(&command);
    }
    as_bool(requirement.get("satisfied"), false)
}

fn evaluate_requirements(
    requirements: &[Value],
    config: &Map<String, Value>,
) -> (Vec<Value>, bool) {
    let mut out = Vec::<Value>::new();
    let mut all_met = true;
    for req in requirements {
        let mut row = req.clone();
        let met = requirement_satisfied(req, config);
        if !met {
            all_met = false;
        }
        row["satisfied"] = Value::Bool(met);
        out.push(row);
    }
    (out, all_met)
}

fn catalog(root: &Path, snapshot: &Value, state: &Value) -> Vec<Value> {
    let platform = server_platform();
    let mut rows = base_catalog(root, snapshot);
    for row in &mut rows {
        let hand_id = clean_id(row.get("id").and_then(Value::as_str).unwrap_or(""), 80);
        let cfg = hand_config(state, &hand_id);
        let requirements = row
            .get("requirements")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let (evaluated, met) = evaluate_requirements(&requirements, &cfg);
        row["requirements"] = Value::Array(evaluated);
        row["requirements_met"] = Value::Bool(met);
        row["server_platform"] = Value::String(platform.clone());
        row["dashboard_metrics"] = Value::from(
            row.get("dashboard")
                .and_then(Value::as_array)
                .map(|v| v.len())
                .unwrap_or(0) as i64,
        );
    }
    rows
}

fn hand_from_catalog(catalog: &[Value], hand_id: &str) -> Option<Value> {
    let id = clean_id(hand_id, 80);
    catalog
        .iter()
        .find(|row| clean_id(row.get("id").and_then(Value::as_str).unwrap_or(""), 80) == id)
        .cloned()
}

fn uptime_seconds(activated_at: &str) -> i64 {
    let Some(ts) = parse_rfc3339(activated_at) else {
        return 0;
    };
    (Utc::now() - ts).num_seconds().max(0)
}

fn trader_default_metrics() -> Map<String, Value> {
    let mut out = Map::<String, Value>::new();
    out.insert(
        "Portfolio Value".to_string(),
        json!({"value": "100000", "format": "number"}),
    );
    out.insert(
        "Total P&L".to_string(),
        json!({"value": "0", "format": "number"}),
    );
    out.insert(
        "Win Rate".to_string(),
        json!({"value": "0%", "format": "text"}),
    );
    out.insert(
        "Sharpe Ratio".to_string(),
        json!({"value": "0.00", "format": "number"}),
    );
    out.insert(
        "Max Drawdown".to_string(),
        json!({"value": "0%", "format": "text"}),
    );
    out.insert(
        "Trades Executed".to_string(),
        json!({"value": 0, "format": "number"}),
    );
    out
}

fn stats_for_instance(instance: &Value) -> Value {
    let mut metrics = Map::<String, Value>::new();
    let status = clean_text(
        instance
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("Active"),
        40,
    );
    let activated_at = clean_text(
        instance
            .get("activated_at")
            .and_then(Value::as_str)
            .unwrap_or(""),
        80,
    );
    metrics.insert(
        "Status".to_string(),
        json!({"value": status, "format": "text"}),
    );
    metrics.insert(
        "Uptime".to_string(),
        json!({"value": uptime_seconds(&activated_at), "format": "duration"}),
    );
    metrics.insert(
        "Restarts".to_string(),
        json!({"value": 0, "format": "number"}),
    );
    metrics.insert(
        "Errors".to_string(),
        json!({"value": 0, "format": "number"}),
    );
    let hand_id = clean_id(
        instance
            .get("hand_id")
            .and_then(Value::as_str)
            .unwrap_or(""),
        80,
    );
    if hand_id == "trader" {
        for (key, value) in trader_default_metrics() {
            metrics.insert(key, value);
        }
    } else if hand_id == "browser" {
        metrics.insert(
            "Pages Visited".to_string(),
            json!({"value": 0, "format": "number"}),
        );
        metrics.insert(
            "Last URL".to_string(),
            json!({"value": clean_text(instance.pointer("/config/start_url").and_then(Value::as_str).unwrap_or(""), 300), "format": "text"}),
        );
    }
    json!({"ok": true, "metrics": metrics})
}

fn hands_segments(path_only: &str) -> Option<Vec<String>> {
    if path_only == "/api/hands" {
        return Some(Vec::new());
    }
    if let Some(rest) = path_only.strip_prefix("/api/hands/") {
        let segments = rest
            .split('/')
            .filter_map(|v| {
                let cleaned = clean_text(v, 200);
                if cleaned.is_empty() {
                    None
                } else {
                    Some(cleaned)
                }
            })
            .collect::<Vec<_>>();
        return Some(segments);
    }
    None
}

pub fn handle(
    root: &Path,
    method: &str,
    path_only: &str,
    body: &[u8],
    snapshot: &Value,
) -> Option<CompatApiResponse> {
    let Some(segments) = hands_segments(path_only) else {
        return None;
    };
    let mut state = load_state(root);
    let mut instances = load_instances(&state);
    let catalog_rows = catalog(root, snapshot, &state);

    if method == "GET" && segments.is_empty() {
        return Some(CompatApiResponse {
            status: 200,
            payload: json!({"ok": true, "hands": catalog_rows}),
        });
    }

    if method == "GET" && segments.len() == 1 && segments[0] == "active" {
        return Some(CompatApiResponse {
            status: 200,
            payload: json!({"ok": true, "instances": instances}),
        });
    }

    if method == "GET" && segments.len() == 1 && segments[0] != "active" {
        let hand_id = clean_id(&segments[0], 80);
        if let Some(detail) = hand_from_catalog(&catalog_rows, &hand_id) {
            return Some(CompatApiResponse {
                status: 200,
                payload: detail,
            });
        }
        return Some(CompatApiResponse {
            status: 404,
            payload: json!({"ok": false, "error": "hand_not_found"}),
        });
    }

    if method == "POST" && segments.len() == 2 {
        let hand_id = clean_id(&segments[0], 80);
        let action = clean_id(&segments[1], 40);
        if action == "check-deps" || action == "install-deps" {
            if let Some(mut detail) = hand_from_catalog(&catalog_rows, &hand_id) {
                let config = hand_config(&state, &hand_id);
                let requirements = detail
                    .get("requirements")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                let (evaluated, met) = evaluate_requirements(&requirements, &config);
                if action == "check-deps" {
                    detail["requirements"] = Value::Array(evaluated.clone());
                    detail["requirements_met"] = Value::Bool(met);
                    return Some(CompatApiResponse {
                        status: 200,
                        payload: json!({
                            "ok": true,
                            "hand_id": hand_id,
                            "requirements": evaluated,
                            "requirements_met": met
                        }),
                    });
                }
                let mut results = Vec::<Value>::new();
                for req in &evaluated {
                    let label = clean_text(
                        req.get("label")
                            .and_then(Value::as_str)
                            .unwrap_or("dependency"),
                        120,
                    );
                    let satisfied = as_bool(req.get("satisfied"), false);
                    if satisfied {
                        results.push(json!({
                            "key": clean_text(req.get("key").and_then(Value::as_str).unwrap_or(""), 120),
                            "label": label,
                            "status": "already_installed",
                            "message": "Dependency already satisfied."
                        }));
                    } else if clean_text(req.get("type").and_then(Value::as_str).unwrap_or(""), 40)
                        .eq_ignore_ascii_case("ApiKey")
                    {
                        results.push(json!({
                            "key": clean_text(req.get("key").and_then(Value::as_str).unwrap_or(""), 120),
                            "label": label,
                            "status": "error",
                            "message": "Provide API key in setup to satisfy this requirement."
                        }));
                    } else {
                        results.push(json!({
                            "key": clean_text(req.get("key").and_then(Value::as_str).unwrap_or(""), 120),
                            "label": label,
                            "status": "error",
                            "message": "Automatic install is disabled in compatibility mode. Use the install command shown in setup."
                        }));
                    }
                }
                return Some(CompatApiResponse {
                    status: 200,
                    payload: json!({
                        "ok": true,
                        "hand_id": hand_id,
                        "results": results,
                        "requirements": evaluated,
                        "requirements_met": met
                    }),
                });
            }
            return Some(CompatApiResponse {
                status: 404,
                payload: json!({"ok": false, "error": "hand_not_found"}),
            });
        }
        if action == "activate" {
            let request = parse_json(body);
            let config = request
                .get("config")
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default();
            if let Some(detail) = hand_from_catalog(&catalog_rows, &hand_id) {
                set_hand_config(&mut state, &hand_id, &config);
                let hand_name = clean_text(
                    detail.get("name").and_then(Value::as_str).unwrap_or("Hand"),
                    120,
                );
                let agent_name = clean_text(
                    config
                        .get("agent_name")
                        .and_then(Value::as_str)
                        .unwrap_or(&format!("{hand_name} Agent")),
                    140,
                );
                let agent_id = super::make_agent_id(root, &agent_name);
                let provider = clean_text(
                    detail
                        .pointer("/agent/provider")
                        .and_then(Value::as_str)
                        .unwrap_or("auto"),
                    80,
                );
                let model = clean_text(
                    detail
                        .pointer("/agent/model")
                        .and_then(Value::as_str)
                        .unwrap_or("auto"),
                    120,
                );
                let system_prompt = clean_text(
                    detail
                        .pointer("/agent/system_prompt")
                        .and_then(Value::as_str)
                        .unwrap_or("You are a specialized hand agent."),
                    4000,
                );
                let now = crate::now_iso();
                let _ = super::update_profile_patch(
                    root,
                    &agent_id,
                    &json!({
                        "agent_id": agent_id,
                        "name": agent_name,
                        "role": clean_text(detail.pointer("/agent/role").and_then(Value::as_str).unwrap_or("specialist"), 80),
                        "state": "Running",
                        "model_provider": provider,
                        "model_name": model,
                        "system_prompt": system_prompt,
                        "hand_id": hand_id,
                        "created_at": now,
                        "updated_at": now
                    }),
                );
                let _ = super::upsert_contract_patch(
                    root,
                    &agent_id,
                    &json!({
                        "status": "active",
                        "owner": "hands_runtime",
                        "mission": format!("Execute {} hand tasks.", hand_name),
                        "created_at": now,
                        "updated_at": now,
                        "termination_condition": "manual_or_timeout",
                        "expiry_seconds": 0,
                        "auto_terminate_allowed": false
                    }),
                );
                let _ = crate::dashboard_agent_state::memory_kv_set(
                    root,
                    &agent_id,
                    "active_hand_id",
                    &json!(hand_id),
                );
                if hand_id == "trader" {
                    let _ = crate::dashboard_agent_state::memory_kv_set(
                        root,
                        &agent_id,
                        "trader_hand_portfolio_value",
                        &json!("100000"),
                    );
                    let _ = crate::dashboard_agent_state::memory_kv_set(
                        root,
                        &agent_id,
                        "trader_hand_total_pnl",
                        &json!("0"),
                    );
                }
                super::append_turn_message(
                    root,
                    &agent_id,
                    "",
                    &format!("{hand_name} activated and linked to Rust runtime."),
                );
                let instance_id = make_id(
                    "handinst",
                    &json!({"hand_id": hand_id, "agent_id": agent_id, "ts": now}),
                );
                instances.push(normalize_instance(&json!({
                    "instance_id": instance_id,
                    "hand_id": hand_id,
                    "agent_id": agent_id,
                    "agent_name": agent_name,
                    "status": "Active",
                    "activated_at": now,
                    "updated_at": now,
                    "config": config
                })));
                set_instances(&mut state, instances.clone());
                save_state(root, state);
                return Some(CompatApiResponse {
                    status: 200,
                    payload: json!({
                        "ok": true,
                        "instance_id": instances.first().and_then(|v| v.get("instance_id")).cloned().unwrap_or(Value::Null),
                        "hand_id": hand_id,
                        "agent_id": agent_id,
                        "agent_name": agent_name
                    }),
                });
            }
            return Some(CompatApiResponse {
                status: 404,
                payload: json!({"ok": false, "error": "hand_not_found"}),
            });
        }
    }

    if segments.len() >= 2 && segments[0] == "instances" {
        let instance_id = clean_id(&segments[1], 120);
        if instance_id.is_empty() {
            return Some(CompatApiResponse {
                status: 400,
                payload: json!({"ok": false, "error": "instance_id_required"}),
            });
        }
        if method == "DELETE" && segments.len() == 2 {
            if let Some(idx) = instances.iter().position(|row| {
                clean_id(
                    row.get("instance_id").and_then(Value::as_str).unwrap_or(""),
                    120,
                ) == instance_id
            }) {
                let agent_id = clean_id(
                    instances[idx]
                        .get("agent_id")
                        .and_then(Value::as_str)
                        .unwrap_or(""),
                    120,
                );
                if !agent_id.is_empty() {
                    let _ = super::update_profile_patch(
                        root,
                        &agent_id,
                        &json!({"state": "Inactive", "updated_at": crate::now_iso()}),
                    );
                    let _ = super::upsert_contract_patch(
                        root,
                        &agent_id,
                        &json!({"status": "inactive", "updated_at": crate::now_iso()}),
                    );
                }
                instances.remove(idx);
                set_instances(&mut state, instances);
                save_state(root, state);
                return Some(CompatApiResponse {
                    status: 200,
                    payload: json!({"ok": true, "deleted": true, "instance_id": instance_id}),
                });
            }
            return Some(CompatApiResponse {
                status: 404,
                payload: json!({"ok": false, "error": "instance_not_found"}),
            });
        }
        if method == "GET" && segments.len() == 3 && segments[2] == "stats" {
            if let Some(instance) = instances.iter().find(|row| {
                clean_id(
                    row.get("instance_id").and_then(Value::as_str).unwrap_or(""),
                    120,
                ) == instance_id
            }) {
                let mut payload = stats_for_instance(instance);
                payload["instance_id"] = Value::String(instance_id);
                return Some(CompatApiResponse {
                    status: 200,
                    payload,
                });
            }
            return Some(CompatApiResponse {
                status: 404,
                payload: json!({"ok": false, "error": "instance_not_found"}),
            });
        }
        if method == "GET" && segments.len() == 3 && segments[2] == "browser" {
            if let Some(instance) = instances.iter().find(|row| {
                clean_id(
                    row.get("instance_id").and_then(Value::as_str).unwrap_or(""),
                    120,
                ) == instance_id
            }) {
                let hand_id = clean_id(
                    instance
                        .get("hand_id")
                        .and_then(Value::as_str)
                        .unwrap_or(""),
                    80,
                );
                if hand_id != "browser" {
                    return Some(CompatApiResponse {
                        status: 200,
                        payload: json!({"ok": true, "active": false}),
                    });
                }
                let url = clean_text(
                    instance
                        .pointer("/config/start_url")
                        .and_then(Value::as_str)
                        .unwrap_or("https://example.com"),
                    400,
                );
                return Some(CompatApiResponse {
                    status: 200,
                    payload: json!({
                        "ok": true,
                        "active": clean_text(instance.get("status").and_then(Value::as_str).unwrap_or(""), 40) == "Active",
                        "url": url,
                        "title": "Browser Hand Session",
                        "screenshot_base64": BROWSER_PLACEHOLDER_SCREENSHOT_BASE64,
                        "content": "Browser hand connected. Live browser telemetry can stream into this panel."
                    }),
                });
            }
            return Some(CompatApiResponse {
                status: 404,
                payload: json!({"ok": false, "error": "instance_not_found"}),
            });
        }
        if method == "POST"
            && segments.len() == 3
            && (segments[2] == "pause" || segments[2] == "resume")
        {
            if let Some(idx) = instances.iter().position(|row| {
                clean_id(
                    row.get("instance_id").and_then(Value::as_str).unwrap_or(""),
                    120,
                ) == instance_id
            }) {
                let is_pause = segments[2] == "pause";
                instances[idx]["status"] = Value::String(if is_pause {
                    "Paused".to_string()
                } else {
                    "Active".to_string()
                });
                instances[idx]["updated_at"] = Value::String(crate::now_iso());
                let agent_id = clean_id(
                    instances[idx]
                        .get("agent_id")
                        .and_then(Value::as_str)
                        .unwrap_or(""),
                    120,
                );
                if !agent_id.is_empty() {
                    let _ = super::update_profile_patch(
                        root,
                        &agent_id,
                        &json!({
                            "state": if is_pause { "Paused" } else { "Running" },
                            "updated_at": crate::now_iso()
                        }),
                    );
                }
                set_instances(&mut state, instances.clone());
                save_state(root, state);
                return Some(CompatApiResponse {
                    status: 200,
                    payload: json!({"ok": true, "instance": instances[idx].clone()}),
                });
            }
            return Some(CompatApiResponse {
                status: 404,
                payload: json!({"ok": false, "error": "instance_not_found"}),
            });
        }
    }

    Some(CompatApiResponse {
        status: 405,
        payload: json!({"ok": false, "error": "method_not_allowed"}),
    })
}
