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
            "privacy_units": 0
        });
    }

    let current = ledger["agents"][&agent][&period][&bucket].clone();
    let projected = json!({
        "tokens": current.get("tokens").and_then(Value::as_u64).unwrap_or(0).saturating_add(tokens),
        "cost_usd": current.get("cost_usd").and_then(Value::as_f64).unwrap_or(0.0) + cost_usd,
        "compute_ms": current.get("compute_ms").and_then(Value::as_u64).unwrap_or(0).saturating_add(compute_ms),
        "privacy_units": current.get("privacy_units").and_then(Value::as_u64).unwrap_or(0).saturating_add(privacy_units)
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
            "privacy_units": privacy_units
        },
        "projected_usage": projected,
        "limits": limits,
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
                    "hard_stop": hard_stop
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}
