// Layer ownership: core/layer0/ops (authoritative)
// SPDX-License-Identifier: Apache-2.0
use chrono::{DateTime, Utc};
use serde_json::{json, Map, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::CompatApiResponse;
use crate::contract_lane_utils as lane_utils;

const HANDS_STATE_REL: &str = "client/runtime/local/state/ui/infring_dashboard/hands_state.json";
const BROWSER_PLACEHOLDER_SCREENSHOT_BASE64: &str =
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO9f3n8AAAAASUVORK5CYII=";

fn clean_text(raw: &str, max_len: usize) -> String {
    lane_utils::clean_text(Some(raw), max_len.max(1))
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
    lane_utils::read_json(path)
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
    let mut rows = vec![
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
    ];
    rows.extend(crate::reference_parity_catalog::extra_hands(
        &fallback_provider,
        &fallback_model,
    ));
    rows
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
