
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
