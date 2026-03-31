fn normalize_strategy(root: &Path, strategy_path: &Path, raw: &Value) -> Value {
    let src = raw.as_object().cloned().unwrap_or_default();
    let file_stem = strategy_path
        .file_stem()
        .map(|v| v.to_string_lossy().to_string())
        .unwrap_or_else(|| "default".to_string());
    let id = {
        let token = as_str(src.get("id"));
        if token.is_empty() {
            file_stem
        } else {
            token
        }
    };
    let name = {
        let token = as_str(src.get("name"));
        if token.is_empty() {
            id.clone()
        } else {
            token
        }
    };

    let mut allowed = as_string_array(src.get("allowed_risks"))
        .into_iter()
        .map(|v| v.to_ascii_lowercase())
        .collect::<Vec<_>>();
    if let Some(Value::Object(risk_obj)) = src.get("risk_policy") {
        for row in as_string_array(risk_obj.get("allowed_risks")) {
            let token = row.to_ascii_lowercase();
            if !allowed.iter().any(|v| v == &token) {
                allowed.push(token);
            }
        }
    }
    allowed.retain(|row| matches!(row.as_str(), "low" | "medium" | "high"));
    if allowed.is_empty() {
        allowed = vec!["low".to_string(), "medium".to_string()];
    }

    let max_risk_per_action = src
        .get("risk_policy")
        .and_then(Value::as_object)
        .and_then(|v| as_i64(v.get("max_risk_per_action")))
        .map(|v| clamp_i64(v, 0, 100));

    let execution_mode = {
        let raw_mode = src
            .get("execution_policy")
            .and_then(Value::as_object)
            .map(|v| as_str(v.get("mode")).to_ascii_lowercase())
            .unwrap_or_else(|| "score_only".to_string());
        match raw_mode.as_str() {
            "execute" => "execute".to_string(),
            "canary_execute" => "canary_execute".to_string(),
            _ => "score_only".to_string(),
        }
    };

    let generation_mode = {
        let raw_mode = src
            .get("generation_policy")
            .and_then(Value::as_object)
            .map(|v| as_str(v.get("mode")).to_ascii_lowercase())
            .unwrap_or_else(|| "hyper-creative".to_string());
        match raw_mode.as_str() {
            "normal" | "narrative" | "creative" | "hyper-creative" | "deep-thinker" => raw_mode,
            _ => "hyper-creative".to_string(),
        }
    };

    let canary_daily_exec_limit = src
        .get("execution_policy")
        .and_then(Value::as_object)
        .and_then(|v| as_i64(v.get("canary_daily_exec_limit")))
        .map(|v| clamp_i64(v, 1, 20));

    let budget_policy = {
        let obj = src
            .get("budget_policy")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        json!({
            "daily_runs_cap": as_i64(obj.get("daily_runs_cap")).map(|v| clamp_i64(v, 1, 500)),
            "daily_token_cap": as_i64(obj.get("daily_token_cap")).map(|v| clamp_i64(v, 100, 1_000_000)),
            "max_tokens_per_action": as_i64(obj.get("max_tokens_per_action")).map(|v| clamp_i64(v, 50, 1_000_000)),
            "token_cost_per_1k": as_f64(obj.get("token_cost_per_1k")),
            "daily_usd_cap": as_f64(obj.get("daily_usd_cap")),
            "per_action_avg_usd_cap": as_f64(obj.get("per_action_avg_usd_cap")),
            "monthly_usd_allocation": as_f64(obj.get("monthly_usd_allocation")),
            "monthly_credits_floor_pct": as_f64(obj.get("monthly_credits_floor_pct")).map(|v| clamp_f64(v, 0.0, 0.95)),
            "min_projected_tokens_for_burn_check": as_i64(obj.get("min_projected_tokens_for_burn_check")).map(|v| clamp_i64(v, 0, 1_000_000)),
            "per_capability_caps": obj.get("per_capability_caps").cloned().unwrap_or_else(|| json!({}))
        })
    };

    let exploration_policy = {
        let obj = src
            .get("exploration_policy")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let fraction = clamp_f64(as_f64(obj.get("fraction")).unwrap_or(0.25), 0.05, 0.8);
        let every_n = clamp_i64(as_i64(obj.get("every_n")).unwrap_or(3), 1, 20);
        let min_eligible = clamp_i64(as_i64(obj.get("min_eligible")).unwrap_or(3), 2, 20);
        json!({
            "fraction": ((fraction * 1000.0).round() / 1000.0),
            "every_n": every_n,
            "min_eligible": min_eligible
        })
    };

    let threshold_overrides = {
        let allowed = HashSet::from([
            "min_signal_quality",
            "min_sensory_signal_score",
            "min_sensory_relevance_score",
            "min_directive_fit",
            "min_actionability_score",
            "min_eye_score_ema",
            "min_composite_eligibility",
        ]);
        let mut out = Map::new();
        if let Some(Value::Object(overrides)) = src.get("threshold_overrides") {
            for (key, value) in overrides {
                if !allowed.contains(key.as_str()) {
                    continue;
                }
                if let Some(n) = as_f64(Some(value)) {
                    out.insert(key.clone(), json!(n));
                }
            }
        }
        Value::Object(out)
    };

    let admission_policy = {
        let obj = src
            .get("admission_policy")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        json!({
            "allowed_types": as_string_array(obj.get("allowed_types")).into_iter().map(|v| v.to_ascii_lowercase()).collect::<Vec<_>>(),
            "blocked_types": as_string_array(obj.get("blocked_types")).into_iter().map(|v| v.to_ascii_lowercase()).collect::<Vec<_>>(),
            "max_remediation_depth": as_i64(obj.get("max_remediation_depth")).map(|v| clamp_i64(v, 0, 12)),
            "duplicate_window_hours": clamp_i64(as_i64(obj.get("duplicate_window_hours")).unwrap_or(24), 1, 168)
        })
    };

    let objective = {
        let obj = src
            .get("objective")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let objective_metric = {
            let metric = as_str(obj.get("fitness_metric"));
            if metric.is_empty() {
                "verified_progress_rate".to_string()
            } else {
                metric
            }
        };
        json!({
            "primary": as_str(obj.get("primary")),
            "secondary": as_string_array(obj.get("secondary")),
            "fitness_metric": objective_metric,
            "target_window_days": clamp_i64(as_i64(obj.get("target_window_days")).unwrap_or(14), 1, 90)
        })
    };

    let ranking_weights = normalize_ranking_weights(src.get("ranking_weights"));

    let value_currency_policy = src
        .get("value_currency_policy")
        .cloned()
        .unwrap_or_else(|| json!({}));

    let strategy_rel = strategy_path
        .strip_prefix(root)
        .map(|v| v.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|_| strategy_path.to_string_lossy().replace('\\', "/"));

    let strategy_version = {
        let v = as_str(src.get("version"));
        if v.is_empty() {
            "1.0".to_string()
        } else {
            v
        }
    };

    json!({
        "id": id,
        "name": name,
        "status": normalize_status(src.get("status")),
        "file": strategy_rel,
        "version": strategy_version,
        "objective": objective,
        "campaigns": normalize_campaigns(src.get("campaigns"), false),
        "generation_policy": { "mode": generation_mode },
        "tags": as_string_array(src.get("tags")).into_iter().map(|v| v.to_ascii_lowercase()).collect::<Vec<_>>(),
        "risk_policy": {
            "allowed_risks": allowed,
            "max_risk_per_action": max_risk_per_action,
            "invalid_risks": []
        },
        "admission_policy": admission_policy,
        "ranking_weights": ranking_weights,
        "budget_policy": budget_policy,
        "exploration_policy": exploration_policy,
        "stop_policy": src.get("stop_policy").cloned().unwrap_or_else(|| json!({})),
        "promotion_policy": normalize_promotion_policy(src.get("promotion_policy")),
        "execution_policy": {
            "mode": execution_mode,
            "canary_daily_exec_limit": canary_daily_exec_limit
        },
        "threshold_overrides": threshold_overrides,
        "value_currency_policy": value_currency_policy,
        "validation": {
            "strict_ok": true,
            "errors": [],
            "warnings": []
        }
    })
}

fn default_strategy_dir(root: &Path) -> PathBuf {
    root.join(DEFAULT_STRATEGY_DIR_REL)
}

fn list_strategies(root: &Path, options: Option<&Value>) -> Vec<Value> {
    let strategy_dir = options
        .and_then(Value::as_object)
        .and_then(|obj| obj.get("dir"))
        .map(|v| as_str(Some(v)))
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .map(|p| if p.is_absolute() { p } else { root.join(p) })
        .unwrap_or_else(|| default_strategy_dir(root));

    let Ok(entries) = fs::read_dir(&strategy_dir) else {
        return Vec::new();
    };

    let mut files = entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().map(|v| v == "json").unwrap_or(false))
        .collect::<Vec<_>>();
    files.sort();

    let mut out = Vec::<Value>::new();
    for file_path in files {
        let Ok(text) = fs::read_to_string(&file_path) else {
            continue;
        };
        let Ok(raw) = serde_json::from_str::<Value>(&text) else {
            continue;
        };
        if !raw.is_object() {
            continue;
        }
        out.push(normalize_strategy(root, &file_path, &raw));
    }

    out.sort_by(|a, b| as_str(a.get("id")).cmp(&as_str(b.get("id"))));
    out
}

fn apply_weaver_overlay(root: &Path, strategy: Value) -> Value {
    let overlay_path = std::env::var("WEAVER_ACTIVE_OVERLAY_PATH")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join(DEFAULT_WEAVER_OVERLAY_REL));

    let Ok(text) = fs::read_to_string(&overlay_path) else {
        return strategy;
    };
    let Ok(overlay) = serde_json::from_str::<Value>(&text) else {
        return strategy;
    };
    let Some(overlay_obj) = overlay.as_object() else {
        return strategy;
    };
    if !as_bool(overlay_obj.get("enabled"), false) {
        return strategy;
    }

    let strategy_id_overlay = as_str(overlay_obj.get("strategy_id"));
    let strategy_id_current = as_str(strategy.get("id"));
    if !strategy_id_overlay.is_empty()
        && strategy_id_overlay != "*"
        && strategy_id_overlay != strategy_id_current
    {
        return strategy;
    }

    let strategy_policy = overlay_obj
        .get("strategy_policy")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let value_currency_overlay = strategy_policy
        .get("value_currency_policy_overrides")
        .cloned();

    let Some(mut strategy_obj) = strategy.as_object().cloned() else {
        return strategy;
    };

    if let Some(Value::Object(overlay_policy_obj)) = value_currency_overlay {
        let mut merged = strategy_obj
            .get("value_currency_policy")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        for (key, value) in overlay_policy_obj {
            merged.insert(key, value);
        }
        strategy_obj.insert("value_currency_policy".to_string(), Value::Object(merged));
    }

    strategy_obj.insert(
        "weaver_overlay".to_string(),
        json!({
            "ts": as_str(overlay_obj.get("ts")),
            "source": overlay_path
                .strip_prefix(root)
                .map(|v| v.to_string_lossy().replace('\\', "/"))
                .unwrap_or_else(|_| overlay_path.to_string_lossy().replace('\\', "/")),
            "objective_id": as_str(overlay_obj.get("objective_id")),
            "primary_metric_id": as_str(overlay_obj.get("primary_metric_id")),
            "reason_codes": as_string_array(overlay_obj.get("reason_codes"))
        }),
    );

    Value::Object(strategy_obj)
}

fn load_active_strategy(root: &Path, options: Option<&Value>) -> Result<Value, String> {
    let options_obj = options
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let allow_missing = as_bool(options_obj.get("allowMissing"), false);
    let strict = as_bool(options_obj.get("strict"), false)
        || matches!(
            std::env::var("AUTONOMY_STRATEGY_STRICT").ok().as_deref(),
            Some("1")
        );
    let requested_id = {
        let id = as_str(options_obj.get("id"));
        if id.is_empty() {
            std::env::var("AUTONOMY_STRATEGY_ID")
                .ok()
                .unwrap_or_default()
                .trim()
                .to_string()
        } else {
            id
        }
    };

    let listed = list_strategies(root, options);
    if listed.is_empty() {
        if allow_missing {
            return Ok(Value::Null);
        }
        return Err("strategy_not_found:no_profiles".to_string());
    }

    let mut pick = Value::Null;
    if !requested_id.is_empty() {
        if let Some(hit) = listed
            .iter()
            .find(|row| as_str(row.get("id")) == requested_id)
            .cloned()
        {
            pick = hit;
        } else if allow_missing {
            return Ok(Value::Null);
        } else {
            return Err(format!("strategy_not_found:{requested_id}"));
        }
    } else if let Some(active) = listed
        .iter()
        .find(|row| as_str(row.get("status")) == "active")
        .cloned()
    {
        pick = active;
    }

    if pick.is_null() {
        if allow_missing {
            return Ok(Value::Null);
        }
        return Err("strategy_not_found:no_active".to_string());
    }

    if strict {
        let strict_ok = pick
            .get("validation")
            .and_then(Value::as_object)
            .and_then(|obj| obj.get("strict_ok"))
            .and_then(Value::as_bool)
            .unwrap_or(true);
        if !strict_ok {
            return Err(format!("strategy_invalid:{}", as_str(pick.get("id"))));
        }
    }

    Ok(apply_weaver_overlay(root, pick))
}

