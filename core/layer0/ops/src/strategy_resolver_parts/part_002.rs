fn resolve_ranking_context(strategy: Option<&Value>, context: Option<&Value>) -> Value {
    let strategy_obj = strategy
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let context_obj = context
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    let base_weights = normalize_ranking_weights(strategy_obj.get("ranking_weights"));
    let mut weights_map = base_weights.as_object().cloned().unwrap_or_default();

    let policy_obj = strategy_obj
        .get("value_currency_policy")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    let objective_id = as_str(context_obj.get("objective_id"));
    let mut selected_currency = as_str(context_obj.get("value_currency")).to_ascii_lowercase();
    let mut applied = Vec::<String>::new();

    if let Some(objective_overrides) = policy_obj
        .get("objective_overrides")
        .and_then(Value::as_object)
    {
        if let Some(objective_hit) = objective_overrides
            .get(&objective_id)
            .and_then(Value::as_object)
        {
            if let Some(ranking_overlay) = objective_hit.get("ranking_weights") {
                if let Some(overlay_obj) = ranking_overlay.as_object() {
                    for (key, value) in overlay_obj {
                        if let Some(n) = as_f64(Some(value)) {
                            if n >= 0.0 {
                                weights_map.insert(key.clone(), json!(n));
                            }
                        }
                    }
                    applied.push(format!("objective:{objective_id}"));
                }
            }
            if selected_currency.is_empty() {
                selected_currency =
                    as_str(objective_hit.get("primary_currency")).to_ascii_lowercase();
            }
        }
    }

    if selected_currency.is_empty() {
        selected_currency = as_str(policy_obj.get("default_currency")).to_ascii_lowercase();
    }

    if let Some(currency_overrides) = policy_obj
        .get("currency_overrides")
        .and_then(Value::as_object)
    {
        if let Some(currency_hit) = currency_overrides
            .get(&selected_currency)
            .and_then(Value::as_object)
        {
            if let Some(ranking_overlay) = currency_hit.get("ranking_weights") {
                if let Some(overlay_obj) = ranking_overlay.as_object() {
                    for (key, value) in overlay_obj {
                        if let Some(n) = as_f64(Some(value)) {
                            if n >= 0.0 {
                                weights_map.insert(key.clone(), json!(n));
                            }
                        }
                    }
                    if !selected_currency.is_empty() {
                        applied.push(format!("currency:{selected_currency}"));
                    }
                }
            }
        }
    }

    let normalized_weights = normalize_ranking_weights(Some(&Value::Object(weights_map)));

    json!({
        "objective_id": if objective_id.is_empty() { Value::Null } else { Value::String(objective_id) },
        "value_currency": if selected_currency.is_empty() { Value::Null } else { Value::String(selected_currency) },
        "weights": normalized_weights,
        "applied_overrides": applied
    })
}

fn op_dispatch(root: &Path, op: &str, args: Option<&Value>) -> Result<Value, String> {
    match op {
        "listStrategies" => Ok(Value::Array(list_strategies(root, args))),
        "loadActiveStrategy" => load_active_strategy(root, args),
        "effectiveAllowedRisks" => {
            let args_obj = args.and_then(Value::as_object).cloned().unwrap_or_default();
            let defaults = as_string_array(args_obj.get("defaultSet"))
                .into_iter()
                .map(|v| v.to_ascii_lowercase())
                .collect::<Vec<_>>();
            let strategy_allowed = args_obj
                .get("strategy")
                .and_then(Value::as_object)
                .and_then(|obj| obj.get("risk_policy"))
                .and_then(Value::as_object)
                .map(|obj| as_string_array(obj.get("allowed_risks")))
                .unwrap_or_default()
                .into_iter()
                .map(|v| v.to_ascii_lowercase())
                .collect::<Vec<_>>();
            let selected = if strategy_allowed.is_empty() {
                defaults
            } else {
                strategy_allowed
            };
            let mut dedupe = BTreeSet::<String>::new();
            let out = selected
                .into_iter()
                .filter(|v| !v.is_empty())
                .filter(|v| dedupe.insert(v.clone()))
                .collect::<Vec<_>>();
            Ok(json!(out))
        }
        "applyThresholdOverrides" => {
            let args_obj = args.and_then(Value::as_object).cloned().unwrap_or_default();
            let mut base = args_obj
                .get("baseThresholds")
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default();
            let overrides = args_obj
                .get("strategy")
                .and_then(Value::as_object)
                .and_then(|obj| obj.get("threshold_overrides"))
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default();
            let allowed = HashSet::from([
                "min_signal_quality",
                "min_sensory_signal_score",
                "min_sensory_relevance_score",
                "min_directive_fit",
                "min_actionability_score",
                "min_eye_score_ema",
                "min_composite_eligibility",
            ]);
            for (key, value) in overrides {
                if !allowed.contains(key.as_str()) {
                    continue;
                }
                if as_f64(Some(&value)).is_some() {
                    base.insert(key, value);
                }
            }
            Ok(Value::Object(base))
        }
        "strategyExecutionMode" => {
            let args_obj = args.and_then(Value::as_object).cloned().unwrap_or_default();
            let strategy = args_obj
                .get("strategy")
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default();
            let fallback = as_str(args_obj.get("fallback")).to_ascii_lowercase();
            let fallback_mode = match fallback.as_str() {
                "score_only" => "score_only",
                "canary_execute" => "canary_execute",
                "execute" => "execute",
                _ => "execute",
            };
            let mode = strategy
                .get("execution_policy")
                .and_then(Value::as_object)
                .map(|obj| as_str(obj.get("mode")).to_ascii_lowercase())
                .unwrap_or_default();
            let out = match mode.as_str() {
                "score_only" => "score_only",
                "canary_execute" => "canary_execute",
                "execute" => "execute",
                _ => fallback_mode,
            };
            Ok(Value::String(out.to_string()))
        }
        "strategyGenerationMode" => {
            let args_obj = args.and_then(Value::as_object).cloned().unwrap_or_default();
            let strategy = args_obj
                .get("strategy")
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default();
            let fallback = as_str(args_obj.get("fallback")).to_ascii_lowercase();
            let mode = strategy
                .get("generation_policy")
                .and_then(Value::as_object)
                .map(|obj| as_str(obj.get("mode")).to_ascii_lowercase())
                .unwrap_or_default();
            let allowed = HashSet::from([
                "normal",
                "narrative",
                "creative",
                "hyper-creative",
                "deep-thinker",
            ]);
            if allowed.contains(mode.as_str()) {
                Ok(Value::String(mode))
            } else if allowed.contains(fallback.as_str()) {
                Ok(Value::String(fallback))
            } else {
                Ok(Value::String("hyper-creative".to_string()))
            }
        }
        "strategyCanaryDailyExecLimit" => {
            let args_obj = args.and_then(Value::as_object).cloned().unwrap_or_default();
            let strategy = args_obj
                .get("strategy")
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default();
            let from_strategy = strategy
                .get("execution_policy")
                .and_then(Value::as_object)
                .and_then(|obj| as_i64(obj.get("canary_daily_exec_limit")));
            let fallback = as_i64(args_obj.get("fallback"));
            let value = from_strategy.or(fallback).map(|v| clamp_i64(v, 1, 20));
            Ok(value.map(Value::from).unwrap_or(Value::Null))
        }
        "strategyBudgetCaps" => {
            let args_obj = args.and_then(Value::as_object).cloned().unwrap_or_default();
            let strategy = args_obj
                .get("strategy")
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default();
            let defaults = args_obj
                .get("defaults")
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default();
            let policy = strategy
                .get("budget_policy")
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default();

            let choose_i64 = |key: &str, lo: i64, hi: i64| -> Option<i64> {
                as_i64(policy.get(key))
                    .or_else(|| as_i64(defaults.get(key)))
                    .map(|v| clamp_i64(v, lo, hi))
            };
            let choose_f64 = |key: &str| -> Option<f64> {
                as_f64(policy.get(key)).or_else(|| as_f64(defaults.get(key)))
            };

            Ok(json!({
                "daily_runs_cap": choose_i64("daily_runs_cap", 1, 500),
                "daily_token_cap": choose_i64("daily_token_cap", 100, 1_000_000),
                "max_tokens_per_action": choose_i64("max_tokens_per_action", 50, 1_000_000),
                "token_cost_per_1k": choose_f64("token_cost_per_1k"),
                "daily_usd_cap": choose_f64("daily_usd_cap"),
                "per_action_avg_usd_cap": choose_f64("per_action_avg_usd_cap"),
                "monthly_usd_allocation": choose_f64("monthly_usd_allocation"),
                "monthly_credits_floor_pct": choose_f64("monthly_credits_floor_pct").map(|v| clamp_f64(v, 0.0, 0.95)),
                "min_projected_tokens_for_burn_check": choose_i64("min_projected_tokens_for_burn_check", 0, 1_000_000),
                "per_capability_caps": policy.get("per_capability_caps").cloned().unwrap_or_else(|| json!({}))
            }))
        }
        "strategyExplorationPolicy" => {
            let args_obj = args.and_then(Value::as_object).cloned().unwrap_or_default();
            let strategy = args_obj
                .get("strategy")
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default();
            let defaults = args_obj
                .get("defaults")
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default();
            let policy = strategy
                .get("exploration_policy")
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default();
            let fraction = as_f64(policy.get("fraction"))
                .or_else(|| as_f64(defaults.get("fraction")))
                .unwrap_or(0.25);
            let every_n = as_i64(policy.get("every_n"))
                .or_else(|| as_i64(defaults.get("every_n")))
                .unwrap_or(3);
            let min_eligible = as_i64(policy.get("min_eligible"))
                .or_else(|| as_i64(defaults.get("min_eligible")))
                .unwrap_or(3);
            Ok(json!({
                "fraction": clamp_f64(fraction, 0.05, 0.8),
                "every_n": clamp_i64(every_n, 1, 20),
                "min_eligible": clamp_i64(min_eligible, 2, 20)
            }))
        }
        "resolveStrategyRankingContext" => {
            let args_obj = args.and_then(Value::as_object).cloned().unwrap_or_default();
            Ok(resolve_ranking_context(
                args_obj.get("strategy"),
                args_obj.get("context"),
            ))
        }
        "strategyRankingWeights" => {
            let args_obj = args.and_then(Value::as_object).cloned().unwrap_or_default();
            let resolved =
                resolve_ranking_context(args_obj.get("strategy"), args_obj.get("context"));
            Ok(resolved
                .get("weights")
                .cloned()
                .unwrap_or_else(|| json!({})))
        }
        "strategyCampaigns" => {
            let args_obj = args.and_then(Value::as_object).cloned().unwrap_or_default();
            let strategy = args_obj
                .get("strategy")
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default();
            let active_only = as_bool(args_obj.get("activeOnly"), false);
            Ok(normalize_campaigns(strategy.get("campaigns"), active_only))
        }
        "strategyAllowsProposalType" => {
            let args_obj = args.and_then(Value::as_object).cloned().unwrap_or_default();
            let strategy = args_obj
                .get("strategy")
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default();
            let proposal_type = as_str(args_obj.get("proposalType")).to_ascii_lowercase();
            let admission_policy = strategy
                .get("admission_policy")
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default();
            let allowed = as_string_array(admission_policy.get("allowed_types"))
                .into_iter()
                .map(|v| v.to_ascii_lowercase())
                .collect::<Vec<_>>();
            let blocked = as_string_array(admission_policy.get("blocked_types"))
                .into_iter()
                .map(|v| v.to_ascii_lowercase())
                .collect::<Vec<_>>();

            let out = if proposal_type.is_empty() {
                allowed.is_empty()
            } else if blocked.iter().any(|v| v == &proposal_type) {
                false
            } else if allowed.is_empty() {
                true
            } else {
                allowed.iter().any(|v| v == &proposal_type)
            };
            Ok(Value::Bool(out))
        }
        "strategyPromotionPolicy" => {
            let args_obj = args.and_then(Value::as_object).cloned().unwrap_or_default();
            let defaults = args_obj
                .get("defaults")
                .cloned()
                .unwrap_or_else(|| json!({}));
            let strategy = args_obj
                .get("strategy")
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default();
            let merged = {
                let mut out = defaults.as_object().cloned().unwrap_or_default();
                if let Some(obj) = strategy.get("promotion_policy").and_then(Value::as_object) {
                    for (k, v) in obj {
                        out.insert(k.clone(), v.clone());
                    }
                }
                Value::Object(out)
            };
            Ok(normalize_promotion_policy(Some(&merged)))
        }
        "strategyMaxRiskPerAction" => {
            let args_obj = args.and_then(Value::as_object).cloned().unwrap_or_default();
            let strategy = args_obj
                .get("strategy")
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default();
            let raw = strategy
                .get("risk_policy")
                .and_then(Value::as_object)
                .and_then(|obj| as_i64(obj.get("max_risk_per_action")))
                .or_else(|| as_i64(args_obj.get("fallback")));
            Ok(raw
                .map(|v| Value::from(clamp_i64(v, 0, 100)))
                .unwrap_or(Value::Null))
        }
        "strategyDuplicateWindowHours" => {
            let args_obj = args.and_then(Value::as_object).cloned().unwrap_or_default();
            let strategy = args_obj
                .get("strategy")
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default();
            let from_strategy = strategy
                .get("admission_policy")
                .and_then(Value::as_object)
                .and_then(|obj| as_i64(obj.get("duplicate_window_hours")));
            let fallback = as_i64(args_obj.get("fallback")).unwrap_or(24);
            let out = clamp_i64(from_strategy.unwrap_or(fallback), 1, 168);
            Ok(Value::from(out))
        }
        _ => Err(format!("strategy_resolver_unknown_op:{op}")),
    }
}

