fn normalize_web_tooling_provider(raw: &str) -> String {
    let value = clean(raw, 80).to_ascii_lowercase();
    match value.as_str() {
        "google" => "google_search".to_string(),
        "xai" => "grok".to_string(),
        "moonshot" => "kimi".to_string(),
        "serp" => "serpapi".to_string(),
        _ => value,
    }
}

fn company_budget_runtime_web_tooling_auth_sources() -> Vec<String> {
    let env_candidates = [
        "BRAVE_API_KEY",
        "EXA_API_KEY",
        "TAVILY_API_KEY",
        "PERPLEXITY_API_KEY",
        "SERPAPI_API_KEY",
        "GOOGLE_SEARCH_API_KEY",
        "GOOGLE_CSE_ID",
        "FIRECRAWL_API_KEY",
        "XAI_API_KEY",
        "MOONSHOT_API_KEY",
        "OPENAI_API_KEY",
    ];
    let mut sources = Vec::<String>::new();
    for env_name in env_candidates {
        let present = std::env::var(env_name)
            .ok()
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false);
        if present {
            sources.push(format!("env:{env_name}"));
        }
    }
    sources
}

fn run_budget_enforce(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let contract = load_json_or(
        root,
        BUDGET_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "company_per_agent_budget_contract",
            "period_limits": {
                "daily": {"tokens": 200000, "cost_usd": 25.0, "compute_ms": 120000, "privacy_units": 1000},
                "weekly": {"tokens": 900000, "cost_usd": 100.0, "compute_ms": 500000, "privacy_units": 5000}
            }
        }),
    );
    let mut errors = Vec::<String>::new();
    if contract
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("company_budget_contract_version_must_be_v1".to_string());
    }
    if contract
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "company_per_agent_budget_contract"
    {
        errors.push("company_budget_contract_kind_invalid".to_string());
    }
    let agent = clean(
        parsed
            .flags
            .get("agent")
            .cloned()
            .or_else(|| parsed.positional.get(1).cloned())
            .unwrap_or_default(),
        120,
    );
    if agent.is_empty() {
        errors.push("company_budget_agent_required".to_string());
    }
    let period = clean(
        parsed
            .flags
            .get("period")
            .cloned()
            .unwrap_or_else(|| "daily".to_string()),
        20,
    )
    .to_ascii_lowercase();
    if strict && period != "daily" && period != "weekly" {
        errors.push("company_budget_period_invalid".to_string());
    }
    if !errors.is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "company_plane_budget_enforce",
            "errors": errors
        });
    }

    let tokens = parse_u64(parsed.flags.get("tokens"), 0);
    let compute_ms = parse_u64(parsed.flags.get("compute-ms"), 0);
    let privacy_units = parse_u64(parsed.flags.get("privacy-units"), 0);
    let cost_usd = parse_f64(parsed.flags.get("cost-usd"), 0.0);
    let web_requests = parse_u64(parsed.flags.get("web-requests"), 0);
    let web_cost_usd = parse_f64(parsed.flags.get("web-cost-usd"), 0.0);
    let web_provider = normalize_web_tooling_provider(
        parsed
            .flags
            .get("web-provider")
            .or_else(|| parsed.flags.get("web-tooling-provider"))
            .map(String::as_str)
            .unwrap_or(""),
    );
    let web_auth_sources = company_budget_runtime_web_tooling_auth_sources();
    let web_auth_present = !web_auth_sources.is_empty();
    let bucket = budget_bucket(&period);
    let period_limits = contract
        .get("period_limits")
        .and_then(|v| v.get(&period))
        .cloned()
        .unwrap_or_else(|| json!({}));

    let limits = json!({
        "tokens": period_limits.get("tokens").and_then(Value::as_u64).unwrap_or(0),
        "cost_usd": period_limits.get("cost_usd").and_then(Value::as_f64).unwrap_or(0.0),
        "compute_ms": period_limits.get("compute_ms").and_then(Value::as_u64).unwrap_or(0),
        "privacy_units": period_limits.get("privacy_units").and_then(Value::as_u64).unwrap_or(0)
    });
    let web_limits_by_period = contract
        .get("web_tooling_limits")
        .and_then(|value| value.get(&period))
        .cloned()
        .unwrap_or_else(|| json!({}));
    let web_limits_by_provider = web_limits_by_period
        .get("providers")
        .and_then(|providers| providers.get(&web_provider))
        .cloned()
        .unwrap_or_else(|| json!({}));
    let web_limits = json!({
        "requests": web_limits_by_provider
            .get("requests")
            .and_then(Value::as_u64)
            .or_else(|| web_limits_by_period.get("requests").and_then(Value::as_u64))
            .unwrap_or(0),
        "cost_usd": web_limits_by_provider
            .get("cost_usd")
            .and_then(Value::as_f64)
            .or_else(|| web_limits_by_period.get("cost_usd").and_then(Value::as_f64))
            .unwrap_or(0.0)
    });

    let ledger_path = state_root(root).join("budgets").join("ledger.json");
    let mut ledger = read_json(&ledger_path).unwrap_or_else(|| {
        json!({
            "version": "v1",
            "agents": {}
        })
    });
    if !ledger.get("agents").map(Value::is_object).unwrap_or(false) {
        ledger["agents"] = Value::Object(serde_json::Map::new());
    }
    if !ledger["agents"].get(&agent).is_some() {
        ledger["agents"][&agent] = json!({});
    }
    if !ledger["agents"][&agent].get(&period).is_some() {
        ledger["agents"][&agent][&period] = json!({});
    }
    if !ledger["agents"][&agent][&period].get(&bucket).is_some() {
        ledger["agents"][&agent][&period][&bucket] = json!({
            "tokens": 0,
            "cost_usd": 0.0,
            "compute_ms": 0,
            "privacy_units": 0,
            "web_requests": 0,
            "web_cost_usd": 0.0
        });
    }

    let current = ledger["agents"][&agent][&period][&bucket].clone();
    let projected = json!({
        "tokens": current.get("tokens").and_then(Value::as_u64).unwrap_or(0).saturating_add(tokens),
        "cost_usd": current.get("cost_usd").and_then(Value::as_f64).unwrap_or(0.0) + cost_usd,
        "compute_ms": current.get("compute_ms").and_then(Value::as_u64).unwrap_or(0).saturating_add(compute_ms),
        "privacy_units": current.get("privacy_units").and_then(Value::as_u64).unwrap_or(0).saturating_add(privacy_units),
        "web_requests": current.get("web_requests").and_then(Value::as_u64).unwrap_or(0).saturating_add(web_requests),
        "web_cost_usd": current.get("web_cost_usd").and_then(Value::as_f64).unwrap_or(0.0) + web_cost_usd
    });

    let mut reason_codes = Vec::<String>::new();
    if projected.get("tokens").and_then(Value::as_u64).unwrap_or(0)
        > limits.get("tokens").and_then(Value::as_u64).unwrap_or(0)
    {
        reason_codes.push("tokens_budget_exceeded".to_string());
    }
    if projected
        .get("cost_usd")
        .and_then(Value::as_f64)
        .unwrap_or(0.0)
        > limits
            .get("cost_usd")
            .and_then(Value::as_f64)
            .unwrap_or(0.0)
    {
        reason_codes.push("cost_budget_exceeded".to_string());
    }
    if projected
        .get("compute_ms")
        .and_then(Value::as_u64)
        .unwrap_or(0)
        > limits
            .get("compute_ms")
            .and_then(Value::as_u64)
            .unwrap_or(0)
    {
        reason_codes.push("compute_budget_exceeded".to_string());
    }
    if projected
        .get("privacy_units")
        .and_then(Value::as_u64)
        .unwrap_or(0)
        > limits
            .get("privacy_units")
            .and_then(Value::as_u64)
            .unwrap_or(0)
    {
        reason_codes.push("privacy_budget_exceeded".to_string());
    }
    if web_limits.get("requests").and_then(Value::as_u64).unwrap_or(0) > 0
        && projected
            .get("web_requests")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            > web_limits
                .get("requests")
                .and_then(Value::as_u64)
                .unwrap_or(0)
    {
        reason_codes.push("web_tooling_requests_budget_exceeded".to_string());
    }
    if web_limits
        .get("cost_usd")
        .and_then(Value::as_f64)
        .unwrap_or(0.0)
        > 0.0
        && projected
            .get("web_cost_usd")
            .and_then(Value::as_f64)
            .unwrap_or(0.0)
            > web_limits
                .get("cost_usd")
                .and_then(Value::as_f64)
                .unwrap_or(0.0)
    {
        reason_codes.push("web_tooling_cost_budget_exceeded".to_string());
    }
    if strict && (web_requests > 0 || !web_provider.is_empty()) && !web_auth_present {
        reason_codes.push("web_tooling_auth_missing".to_string());
    }
    let hard_stop = strict && !reason_codes.is_empty();
    if !hard_stop {
        ledger["agents"][&agent][&period][&bucket] = projected.clone();
        ledger["updated_at"] = Value::String(crate::now_iso());
        let _ = write_json(&ledger_path, &ledger);
    }
    let receipt = json!({
        "version": "v1",
        "agent": agent,
        "period": period,
        "bucket": bucket,
        "requested_delta": {
            "tokens": tokens,
            "cost_usd": cost_usd,
            "compute_ms": compute_ms,
            "privacy_units": privacy_units,
            "web_requests": web_requests,
            "web_cost_usd": web_cost_usd
        },
        "projected_usage": projected,
        "limits": limits,
        "web_tooling": {
            "provider": web_provider,
            "limits": web_limits,
            "auth_present": web_auth_present,
            "auth_sources": web_auth_sources
        },
        "hard_stop": hard_stop,
        "reason_codes": reason_codes,
        "ts": crate::now_iso()
    });
    let _ = append_jsonl(
        &state_root(root).join("budgets").join("history.jsonl"),
        &receipt,
    );

    let mut out = json!({
        "ok": !hard_stop,
        "strict": strict,
        "type": "company_plane_budget_enforce",
        "lane": "core/layer0/ops",
        "decision": receipt,
        "artifact": {
            "path": ledger_path.display().to_string(),
            "sha256": sha256_hex_str(&ledger.to_string())
        },
        "claim_evidence": [
            {
                "id": "V6-COMPANY-001.2",
                "claim": "per_agent_period_budget_enforcement_is_policy_backed_and_fail_closed_on_breaches",
                "evidence": {
                    "agent": agent,
                    "period": period,
                    "hard_stop": hard_stop,
                    "web_provider": web_provider,
                    "web_auth_present": web_auth_present
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}
