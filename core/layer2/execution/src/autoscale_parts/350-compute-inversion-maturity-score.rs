pub fn compute_inversion_maturity_score(
    input: &InversionMaturityScoreInput,
) -> InversionMaturityScoreOutput {
    let total = non_negative_number(Some(input.total_tests)).unwrap_or(0.0);
    let passed = non_negative_number(Some(input.passed_tests)).unwrap_or(0.0);
    let destructive = non_negative_number(Some(input.destructive_failures)).unwrap_or(0.0);
    let target_test_count = non_negative_number(Some(input.target_test_count)).unwrap_or(40.0);
    let weight_pass_rate = non_negative_number(Some(input.weight_pass_rate)).unwrap_or(0.0);
    let weight_non_destructive_rate =
        non_negative_number(Some(input.weight_non_destructive_rate)).unwrap_or(0.0);
    let weight_experience = non_negative_number(Some(input.weight_experience)).unwrap_or(0.0);
    let band_novice = non_negative_number(Some(input.band_novice)).unwrap_or(0.25);
    let band_developing = non_negative_number(Some(input.band_developing)).unwrap_or(0.45);
    let band_mature = non_negative_number(Some(input.band_mature)).unwrap_or(0.65);
    let band_seasoned = non_negative_number(Some(input.band_seasoned)).unwrap_or(0.82);

    let non_destructive_rate = if total > 0.0 {
        ((total - destructive) / total).max(0.0)
    } else {
        1.0
    };
    let pass_rate = if total > 0.0 {
        (passed / total).max(0.0)
    } else {
        0.0
    };
    let experience = (total / target_test_count.max(1.0)).min(1.0);

    let weight_total =
        (weight_pass_rate + weight_non_destructive_rate + weight_experience).max(0.0001);
    let raw_score = ((pass_rate * weight_pass_rate)
        + (non_destructive_rate * weight_non_destructive_rate)
        + (experience * weight_experience))
        / weight_total;
    let score = raw_score.clamp(0.0, 1.0);
    let band = if score < band_novice {
        "novice"
    } else if score < band_developing {
        "developing"
    } else if score < band_mature {
        "mature"
    } else if score < band_seasoned {
        "seasoned"
    } else {
        "legendary"
    };

    InversionMaturityScoreOutput {
        score: ((score * 1_000_000.0).round()) / 1_000_000.0,
        band: band.to_string(),
        pass_rate: ((pass_rate * 1_000_000.0).round()) / 1_000_000.0,
        non_destructive_rate: ((non_destructive_rate * 1_000_000.0).round()) / 1_000_000.0,
        experience: ((experience * 1_000_000.0).round()) / 1_000_000.0,
    }
}

pub fn compute_default_criteria_pattern_memory(
    _input: &DefaultCriteriaPatternMemoryInput,
) -> DefaultCriteriaPatternMemoryOutput {
    DefaultCriteriaPatternMemoryOutput {
        version: "1.0".to_string(),
        updated_at: None,
        patterns: std::collections::BTreeMap::new(),
    }
}

pub fn compute_strategy_execution_mode_effective(
    input: &StrategyExecutionModeEffectiveInput,
) -> StrategyExecutionModeEffectiveOutput {
    let mode_raw = input
        .strategy_mode
        .as_ref()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_default();
    let fallback_raw = input
        .fallback
        .as_ref()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_default();
    let fallback_mode = if fallback_raw == "score_only" {
        "score_only"
    } else if fallback_raw == "canary_execute" {
        "canary_execute"
    } else {
        "execute"
    };
    let mode = if mode_raw == "score_only" {
        "score_only"
    } else if mode_raw == "canary_execute" {
        "canary_execute"
    } else if mode_raw == "execute" {
        "execute"
    } else {
        fallback_mode
    };
    StrategyExecutionModeEffectiveOutput {
        mode: mode.to_string(),
    }
}

pub fn compute_strategy_canary_exec_limit_effective(
    input: &StrategyCanaryExecLimitEffectiveInput,
) -> StrategyCanaryExecLimitEffectiveOutput {
    let from_strategy = js_like_number(input.strategy_limit.as_ref());
    let from_fallback = input.fallback;
    let choose = from_strategy.or(from_fallback).and_then(|value| {
        if !value.is_finite() || value <= 0.0 {
            None
        } else {
            Some(value.round().clamp(1.0, 20.0))
        }
    });
    StrategyCanaryExecLimitEffectiveOutput { limit: choose }
}

pub fn compute_strategy_exploration_effective(
    input: &StrategyExplorationEffectiveInput,
) -> StrategyExplorationEffectiveOutput {
    let default_fraction = input.default_fraction.unwrap_or(0.25);
    let default_every_n = input.default_every_n.unwrap_or(3.0);
    let default_min_eligible = input.default_min_eligible.unwrap_or(3.0);
    let strategy_obj = input
        .strategy_exploration
        .as_ref()
        .and_then(|value| value.as_object());
    if strategy_obj.is_none() {
        return StrategyExplorationEffectiveOutput {
            fraction: default_fraction,
            every_n: default_every_n,
            min_eligible: default_min_eligible,
        };
    }
    let strategy_obj = strategy_obj.expect("checked is_some");
    StrategyExplorationEffectiveOutput {
        fraction: js_like_number(strategy_obj.get("fraction")).unwrap_or(default_fraction),
        every_n: js_like_number(strategy_obj.get("every_n")).unwrap_or(default_every_n),
        min_eligible: js_like_number(strategy_obj.get("min_eligible"))
            .unwrap_or(default_min_eligible),
    }
}

pub fn compute_strategy_budget_effective(
    input: &StrategyBudgetEffectiveInput,
) -> StrategyBudgetEffectiveOutput {
    let mut budget = input
        .caps
        .as_ref()
        .and_then(|value| value.as_object())
        .cloned()
        .unwrap_or_default();
    let hard_runs = input.hard_runs.filter(|v| v.is_finite() && *v > 0.0);
    let hard_tokens = input.hard_tokens.filter(|v| v.is_finite() && *v > 0.0);
    let hard_per_action = input.hard_per_action.filter(|v| v.is_finite() && *v > 0.0);

    if let Some(hard) = hard_runs {
        if let Some(current) = js_like_number(budget.get("daily_runs_cap")) {
            budget.insert(
                "daily_runs_cap".to_string(),
                serde_json::Value::from(current.min(hard)),
            );
        }
    }
    if let Some(hard) = hard_tokens {
        if let Some(current) = js_like_number(budget.get("daily_token_cap")) {
            budget.insert(
                "daily_token_cap".to_string(),
                serde_json::Value::from(current.min(hard)),
            );
        }
    }
    if let Some(hard) = hard_per_action {
        if let Some(current) = js_like_number(budget.get("max_tokens_per_action")) {
            budget.insert(
                "max_tokens_per_action".to_string(),
                serde_json::Value::from(current.min(hard)),
            );
        }
    }

    StrategyBudgetEffectiveOutput {
        budget: serde_json::Value::Object(budget),
    }
}

pub fn compute_preexec_verdict_from_signals(
    input: &PreexecVerdictFromSignalsInput,
) -> PreexecVerdictFromSignalsOutput {
    let blocker_rows = input
        .blockers
        .iter()
        .filter(|row| row.is_object())
        .collect::<Vec<_>>();
    let blocker_codes = blocker_rows
        .iter()
        .filter_map(|row| {
            row.as_object()
                .and_then(|obj| obj.get("code"))
                .map(js_like_string)
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
        })
        .take(16)
        .collect::<Vec<_>>();
    let manual_action_required = blocker_rows.iter().any(|row| {
        row.as_object()
            .and_then(|obj| obj.get("retryable"))
            .map(|value| value != &serde_json::Value::Bool(true))
            .unwrap_or(true)
    });
    let retryable_only = !blocker_rows.is_empty()
        && blocker_rows.iter().all(|row| {
            row.as_object()
                .and_then(|obj| obj.get("retryable"))
                .map(|value| value == &serde_json::Value::Bool(true))
                .unwrap_or(false)
        });
    let mut verdict = "proceed".to_string();
    if !blocker_rows.is_empty() {
        verdict = if manual_action_required {
            "reject".to_string()
        } else if retryable_only {
            "defer".to_string()
        } else {
            "reject".to_string()
        };
    }

    let signals = input
        .signals
        .clone()
        .filter(|value| value.is_object())
        .unwrap_or_else(|| serde_json::json!({}));
    let signal_rows = signals
        .as_object()
        .cloned()
        .unwrap_or_default()
        .into_values()
        .collect::<Vec<_>>();
    let mut fail_count = 0.0;
    let mut warn_count = 0.0;
    for row in signal_rows {
        let status = compute_normalized_signal_status(&NormalizedSignalStatusInput {
            value: row
                .as_object()
                .and_then(|obj| obj.get("status"))
                .map(js_like_string),
            fallback: Some("unknown".to_string()),
        })
        .status;
        if status == "fail" {
            fail_count += 1.0;
        } else if status == "warn" {
            warn_count += 1.0;
        }
    }
    let blocker_penalty = if blocker_rows.is_empty() {
        0.0
    } else {
        (blocker_rows.len() as f64 * 0.06).min(0.42)
    };
    let mut confidence = 1.0 - (fail_count * 0.22) - (warn_count * 0.08) - blocker_penalty;
    confidence = confidence.clamp(0.05, 1.0);
    if verdict == "reject" {
        confidence = confidence.min(0.49);
    }
    if verdict == "defer" {
        confidence = confidence.min(0.69);
    }
    let confidence = ((confidence * 1000.0).round()) / 1000.0;
    let next_runnable_at = if verdict == "proceed" {
        Some(
            compute_now_iso(&NowIsoInput {
                now_iso: input.now_iso.clone(),
            })
            .value,
        )
    } else {
        input
            .next_runnable_at
            .as_ref()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    };
    PreexecVerdictFromSignalsOutput {
        verdict,
        confidence,
        blocker_count: blocker_rows.len() as u32,
        blocker_codes,
        manual_action_required,
        next_runnable_at,
        signals,
    }
}

pub fn compute_score_only_proposal_churn(
    input: &ScoreOnlyProposalChurnInput,
) -> ScoreOnlyProposalChurnOutput {
    let proposal_id = input
        .proposal_id
        .as_ref()
        .map(|value| value.trim().to_string())
        .unwrap_or_default();
    if proposal_id.is_empty() {
        return ScoreOnlyProposalChurnOutput {
            count: 0,
            streak: 0,
            first_ts: None,
            last_ts: None,
        };
    }
    let now_ms = input
        .now_ms
        .filter(|value| value.is_finite())
        .unwrap_or_else(|| Utc::now().timestamp_millis() as f64);
    let window_ms = input
        .window_hours
        .filter(|value| value.is_finite())
        .unwrap_or(1.0)
        .max(1.0)
        * 3_600_000.0;
    let cutoff_ms = now_ms - window_ms;

    let mut matches = Vec::<(i64, serde_json::Value)>::new();
    for evt in &input.prior_runs {
        let Some(obj) = evt.as_object() else {
            continue;
        };
        let event_type = obj
            .get("type")
            .map(js_like_string)
            .unwrap_or_default()
            .trim()
            .to_string();
        if event_type != "autonomy_run" {
            continue;
        }
        let pid = obj
            .get("proposal_id")
            .map(js_like_string)
            .unwrap_or_default()
            .trim()
            .to_string();
        if pid != proposal_id {
            continue;
        }
        let ts_raw = obj
            .get("ts")
            .map(js_like_string)
            .unwrap_or_default()
            .trim()
            .to_string();
        if ts_raw.is_empty() {
            continue;
        }
        let parsed = compute_parse_iso_ts(&ParseIsoTsInput {
            ts: Some(ts_raw.clone()),
        });
        let Some(ts_ms) = parsed.timestamp_ms else {
            continue;
        };
        if ts_ms < cutoff_ms {
            continue;
        }
        let failure_like = compute_score_only_failure_like(&ScoreOnlyFailureLikeInput {
            event_type: Some(event_type),
            result: Some(obj.get("result").map(js_like_string).unwrap_or_default()),
            preview_verification_present: Some(obj.get("preview_verification").is_some()),
            preview_verification_passed: obj
                .get("preview_verification")
                .and_then(|row| row.as_object())
                .and_then(|map| map.get("passed"))
                .and_then(|value| value.as_bool()),
            preview_verification_outcome: obj
                .get("preview_verification")
                .and_then(|row| row.as_object())
                .and_then(|map| map.get("outcome"))
                .map(js_like_string),
        });
        if !failure_like.is_failure_like {
            continue;
        }
        matches.push((ts_ms as i64, evt.clone()));
    }
    matches.sort_by(|a, b| a.0.cmp(&b.0));
    let mut streak: u32 = 0;
    for (_, evt) in matches.iter().rev() {
        let Some(obj) = evt.as_object() else {
            break;
        };
        let failure_like = compute_score_only_failure_like(&ScoreOnlyFailureLikeInput {
            event_type: Some(obj.get("type").map(js_like_string).unwrap_or_default()),
            result: Some(obj.get("result").map(js_like_string).unwrap_or_default()),
            preview_verification_present: Some(obj.get("preview_verification").is_some()),
            preview_verification_passed: obj
                .get("preview_verification")
                .and_then(|row| row.as_object())
                .and_then(|map| map.get("passed"))
                .and_then(|value| value.as_bool()),
            preview_verification_outcome: obj
                .get("preview_verification")
                .and_then(|row| row.as_object())
                .and_then(|map| map.get("outcome"))
                .map(js_like_string),
        });
        if !failure_like.is_failure_like {
            break;
        }
        streak += 1;
    }
    let first_ts = matches
        .first()
        .and_then(|(ms, _)| DateTime::<Utc>::from_timestamp_millis(*ms))
        .map(|dt| dt.to_rfc3339_opts(SecondsFormat::Millis, true));
    let last_ts = matches
        .last()
        .and_then(|(ms, _)| DateTime::<Utc>::from_timestamp_millis(*ms))
        .map(|dt| dt.to_rfc3339_opts(SecondsFormat::Millis, true));
    ScoreOnlyProposalChurnOutput {
        count: matches.len() as u32,
        streak,
        first_ts,
        last_ts,
    }
}
