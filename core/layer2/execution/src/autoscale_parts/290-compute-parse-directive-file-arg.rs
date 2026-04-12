pub fn compute_parse_directive_file_arg(
    input: &ParseDirectiveFileArgInput,
) -> ParseDirectiveFileArgOutput {
    let text = input.command.as_deref().unwrap_or("").trim();
    if text.is_empty() {
        return ParseDirectiveFileArgOutput {
            file: String::new(),
        };
    }
    let re = Regex::new(
        r#"(?:^|\s)--file(?:=(?:"([^"]+)"|'([^']+)'|([^\s]+))|\s+(?:"([^"]+)"|'([^']+)'|([^\s]+)))"#,
    )
        .expect("valid directive file arg regex");
    let raw = re
        .captures(text)
        .and_then(|caps| {
            caps.get(1)
                .or_else(|| caps.get(2))
                .or_else(|| caps.get(3))
                .or_else(|| caps.get(4))
                .or_else(|| caps.get(5))
                .or_else(|| caps.get(6))
        })
        .map(|m| m.as_str().trim().replace('\\', "/"))
        .unwrap_or_default();
    if raw.is_empty() {
        return ParseDirectiveFileArgOutput {
            file: String::new(),
        };
    }
    let allow = Regex::new(r"(?i)^client/runtime/config/directives/[A-Za-z0-9_]+\.ya?ml$")
        .expect("valid directive file allow regex");
    if !allow.is_match(&raw) {
        return ParseDirectiveFileArgOutput {
            file: String::new(),
        };
    }
    ParseDirectiveFileArgOutput { file: raw }
}

pub fn compute_parse_directive_objective_arg(
    input: &ParseDirectiveObjectiveArgInput,
) -> ParseDirectiveObjectiveArgOutput {
    let text = normalize_spaces(input.command.as_deref().unwrap_or(""));
    if text.is_empty() {
        return ParseDirectiveObjectiveArgOutput {
            objective_id: String::new(),
        };
    }
    let re = Regex::new(r#"(?:^|\s)--id=(?:"([^"]+)"|'([^']+)'|([^\s]+))"#)
        .expect("valid directive objective arg regex");
    let raw = re
        .captures(&text)
        .and_then(|caps| caps.get(1).or_else(|| caps.get(2)).or_else(|| caps.get(3)))
        .map(|m| normalize_spaces(m.as_str()))
        .unwrap_or_default();
    let sanitized = compute_sanitize_directive_objective_id(&SanitizeDirectiveObjectiveIdInput {
        value: Some(raw),
    });
    ParseDirectiveObjectiveArgOutput {
        objective_id: sanitized.objective_id,
    }
}

pub fn compute_now_iso(input: &NowIsoInput) -> NowIsoOutput {
    if let Some(raw) = input.now_iso.as_deref() {
        let text = normalize_spaces(raw);
        if !text.is_empty() {
            if let Ok(dt) = DateTime::parse_from_rfc3339(&text) {
                return NowIsoOutput {
                    value: dt
                        .with_timezone(&Utc)
                        .to_rfc3339_opts(SecondsFormat::Millis, true),
                };
            }
        }
    }
    NowIsoOutput {
        value: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
    }
}

pub fn compute_today_str(input: &TodayStrInput) -> TodayStrOutput {
    if let Some(raw) = input.now_iso.as_deref() {
        let text = normalize_spaces(raw);
        if !text.is_empty() {
            if let Ok(dt) = DateTime::parse_from_rfc3339(&text) {
                return TodayStrOutput {
                    value: dt.with_timezone(&Utc).format("%Y-%m-%d").to_string(),
                };
            }
        }
    }
    TodayStrOutput {
        value: Utc::now().format("%Y-%m-%d").to_string(),
    }
}

pub fn compute_human_canary_override_approval_phrase(
    input: &HumanCanaryOverrideApprovalPhraseInput,
) -> HumanCanaryOverrideApprovalPhraseOutput {
    let prefix = input
        .prefix
        .as_deref()
        .map(normalize_spaces)
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "I_APPROVE_ONE_SHOT_CANARY_OVERRIDE".to_string());
    let date_str = input.date_str.as_deref().unwrap_or("");
    let nonce = input.nonce.as_deref().unwrap_or("");
    HumanCanaryOverrideApprovalPhraseOutput {
        phrase: format!("{prefix}:{date_str}:{nonce}"),
    }
}

pub fn compute_parse_human_canary_override_state(
    input: &ParseHumanCanaryOverrideStateInput,
) -> ParseHumanCanaryOverrideStateOutput {
    let Some(record) = input.record.as_ref().and_then(|v| v.as_object()) else {
        return ParseHumanCanaryOverrideStateOutput {
            active: false,
            reason: "missing".to_string(),
            expired: None,
            remaining: None,
            expires_at: None,
            date: None,
            require_execution_mode: None,
            id: None,
            r#type: None,
        };
    };
    let now_ms = input
        .now_ms
        .filter(|v| v.is_finite())
        .unwrap_or_else(|| Utc::now().timestamp_millis() as f64);
    let expires_at = record
        .get("expires_at")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let exp_ms = DateTime::parse_from_rfc3339(expires_at.trim())
        .map(|dt| dt.timestamp_millis() as f64)
        .ok();
    let remaining = match record.get("remaining_uses") {
        Some(serde_json::Value::Number(n)) => n.as_f64().unwrap_or(0.0),
        Some(serde_json::Value::String(s)) => s.trim().parse::<f64>().unwrap_or(0.0),
        Some(serde_json::Value::Bool(v)) => {
            if *v {
                1.0
            } else {
                0.0
            }
        }
        _ => 0.0,
    };
    let expired = exp_ms.map(|v| now_ms > v).unwrap_or(true);
    if remaining <= 0.0 {
        return ParseHumanCanaryOverrideStateOutput {
            active: false,
            reason: "depleted".to_string(),
            expired: Some(expired),
            remaining: Some(remaining),
            expires_at: None,
            date: None,
            require_execution_mode: None,
            id: None,
            r#type: None,
        };
    }
    if expired {
        return ParseHumanCanaryOverrideStateOutput {
            active: false,
            reason: "expired".to_string(),
            expired: Some(true),
            remaining: Some(remaining),
            expires_at: None,
            date: None,
            require_execution_mode: None,
            id: None,
            r#type: None,
        };
    }
    ParseHumanCanaryOverrideStateOutput {
        active: true,
        reason: "ok".to_string(),
        expired: Some(false),
        remaining: Some(remaining),
        expires_at: Some(expires_at),
        date: Some(
            record
                .get("date")
                .map(|v| v.as_str().unwrap_or("").to_string())
                .unwrap_or_default(),
        ),
        require_execution_mode: Some(
            record
                .get("require_execution_mode")
                .map(|v| v.as_str().unwrap_or("").to_string())
                .unwrap_or_default(),
        ),
        id: Some(
            record
                .get("id")
                .map(|v| v.as_str().unwrap_or("").to_string())
                .unwrap_or_default(),
        ),
        r#type: Some(
            record
                .get("type")
                .map(|v| v.as_str().unwrap_or("").to_string())
                .unwrap_or_default(),
        ),
    }
}

pub fn compute_daily_budget_path(input: &DailyBudgetPathInput) -> DailyBudgetPathOutput {
    let state_dir = input.state_dir.as_deref().unwrap_or("").trim();
    let date_str = input.date_str.as_deref().unwrap_or("").trim();
    let path = std::path::Path::new(state_dir)
        .join(format!("{date_str}.json"))
        .to_string_lossy()
        .to_string();
    DailyBudgetPathOutput { path }
}

pub fn compute_runs_path_for(input: &RunsPathForInput) -> RunsPathForOutput {
    let runs_dir = input.runs_dir.as_deref().unwrap_or("").trim();
    let date_str = input.date_str.as_deref().unwrap_or("").trim();
    let path = std::path::Path::new(runs_dir)
        .join(format!("{date_str}.jsonl"))
        .to_string_lossy()
        .to_string();
    RunsPathForOutput { path }
}

pub fn compute_effective_tier1_policy(
    input: &EffectiveTier1PolicyInput,
) -> EffectiveTier1PolicyOutput {
    let mode = normalize_spaces(input.execution_mode.as_deref().unwrap_or("")).to_ascii_lowercase();
    let canary_relaxed = mode == "canary_execute";
    EffectiveTier1PolicyOutput {
        execution_mode: if mode.is_empty() {
            None
        } else {
            Some(mode.clone())
        },
        canary_relaxed,
        burn_rate_multiplier: if canary_relaxed {
            input
                .tier1_burn_rate_multiplier
                .max(input.tier1_canary_burn_rate_multiplier)
        } else {
            input.tier1_burn_rate_multiplier
        },
        min_projected_tokens_for_burn_check: if canary_relaxed {
            input
                .tier1_min_projected_tokens_for_burn_check
                .max(input.tier1_canary_min_projected_tokens_for_burn_check)
        } else {
            input.tier1_min_projected_tokens_for_burn_check
        },
        drift_min_samples: if canary_relaxed {
            input
                .tier1_drift_min_samples
                .max(input.tier1_canary_drift_min_samples)
        } else {
            input.tier1_drift_min_samples
        },
        alignment_threshold: if canary_relaxed {
            input
                .tier1_alignment_threshold
                .min(input.tier1_canary_alignment_threshold)
        } else {
            input.tier1_alignment_threshold
        },
        suppress_alignment_blocker: canary_relaxed && input.tier1_canary_suppress_alignment_blocker,
    }
}

pub fn compute_compact_tier1_exception(
    input: &CompactTier1ExceptionInput,
) -> CompactTier1ExceptionOutput {
    if input.tracked != Some(true) {
        return CompactTier1ExceptionOutput {
            has_value: false,
            value: None,
        };
    }
    let recovery = input.recovery.as_ref().and_then(|v| v.as_object());
    let stage = input
        .stage
        .as_deref()
        .map(|v| v.to_string())
        .filter(|v| !v.is_empty());
    let error_code = input
        .error_code
        .as_deref()
        .map(|v| v.to_string())
        .filter(|v| !v.is_empty());
    let signature = input
        .signature
        .as_deref()
        .map(|v| v.to_string())
        .filter(|v| !v.is_empty());
    let recovery_action = recovery.and_then(|r| {
        r.get("action")
            .and_then(|v| v.as_str())
            .map(|v| v.to_string())
    });
    let recovery_cooldown_hours = recovery
        .and_then(|r| r.get("cooldown_hours"))
        .and_then(|v| {
            if let Some(n) = v.as_f64() {
                Some(n)
            } else if let Some(s) = v.as_str() {
                s.trim().parse::<f64>().ok()
            } else {
                None
            }
        });
    let recovery_playbook = recovery.and_then(|r| {
        r.get("playbook")
            .and_then(|v| v.as_str())
            .map(|v| v.to_string())
    });
    let recovery_reason = recovery.and_then(|r| {
        r.get("reason")
            .and_then(|v| v.as_str())
            .map(|v| v.to_string())
    });
    let recovery_should_escalate = recovery
        .and_then(|r| r.get("should_escalate"))
        .and_then(|v| v.as_bool());
    let value = serde_json::json!({
        "novel": input.novel == Some(true),
        "stage": stage,
        "error_code": error_code,
        "signature": signature,
        "count": input.count.unwrap_or(0.0),
        "recovery_action": recovery_action,
        "recovery_cooldown_hours": recovery_cooldown_hours,
        "recovery_playbook": recovery_playbook,
        "recovery_reason": recovery_reason,
        "recovery_should_escalate": recovery_should_escalate
    });
    CompactTier1ExceptionOutput {
        has_value: true,
        value: Some(value),
    }
}

pub fn compute_next_human_escalation_clear_at(
    input: &NextHumanEscalationClearAtInput,
) -> NextHumanEscalationClearAtOutput {
    let mut min_dt: Option<DateTime<Utc>> = None;
    for row in input.rows.iter() {
        let Some(obj) = row.as_object() else {
            continue;
        };
        let expires_at = obj
            .get("expires_at")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        if expires_at.is_empty() {
            continue;
        }
        let Ok(dt) = DateTime::parse_from_rfc3339(&expires_at) else {
            continue;
        };
        let dt_utc = dt.with_timezone(&Utc);
        min_dt = Some(match min_dt {
            Some(prev) => {
                if dt_utc < prev {
                    dt_utc
                } else {
                    prev
                }
            }
            None => dt_utc,
        });
    }
    NextHumanEscalationClearAtOutput {
        value: min_dt.map(|dt| dt.to_rfc3339_opts(SecondsFormat::Millis, true)),
    }
}

pub fn compute_model_catalog_canary_thresholds(
    input: &ModelCatalogCanaryThresholdsInput,
) -> ModelCatalogCanaryThresholdsOutput {
    let min_samples = input.min_samples.round().clamp(1.0, 50.0);
    let max_fail_rate = input.max_fail_rate.clamp(0.0, 1.0);
    let max_route_block_rate = input.max_route_block_rate.clamp(0.0, 1.0);
    ModelCatalogCanaryThresholdsOutput {
        min_samples,
        max_fail_rate,
        max_route_block_rate,
    }
}
