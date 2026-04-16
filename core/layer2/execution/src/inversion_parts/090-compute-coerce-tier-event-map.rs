pub fn compute_coerce_tier_event_map(input: &CoerceTierEventMapInput) -> CoerceTierEventMapOutput {
    let src = input.map.as_ref().and_then(|v| v.as_object()).cloned();
    let mut map = serde_json::Map::new();
    for target in TIER_TARGETS {
        let legacy_suffix = format!("{target}_events");
        let mut rows = Vec::<String>::new();
        let mut seen = std::collections::BTreeSet::<String>::new();
        for key in [target.to_string(), legacy_suffix] {
            let source_rows = src
                .as_ref()
                .and_then(|obj| obj.get(&key))
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            for row in source_rows {
                let normalized = value_to_string(Some(&row)).trim().to_string();
                if normalized.is_empty() {
                    continue;
                }
                if seen.insert(normalized.clone()) {
                    rows.push(normalized);
                }
                if rows.len() >= 10_000 {
                    break;
                }
            }
            if rows.len() >= 10_000 {
                break;
            }
        }
        map.insert(
            target.to_string(),
            Value::Array(rows.into_iter().map(Value::String).collect::<Vec<_>>()),
        );
    }
    CoerceTierEventMapOutput {
        map: Value::Object(map),
    }
}

pub fn compute_get_tier_scope(input: &GetTierScopeInput) -> GetTierScopeOutput {
    let safe_version = clean_text_runtime(input.policy_version.as_deref().unwrap_or("1.0"), 24);
    let policy_version = if safe_version.is_empty() {
        "1.0".to_string()
    } else {
        safe_version
    };
    let mut state = input
        .state
        .as_ref()
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();

    let mut scopes = state
        .get("scopes")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    if !scopes
        .get(&policy_version)
        .map(|v| v.is_object())
        .unwrap_or(false)
    {
        scopes.insert(
            policy_version.clone(),
            compute_default_tier_scope(&DefaultTierScopeInput::default()).scope,
        );
    }
    let scope = scopes
        .get(&policy_version)
        .cloned()
        .unwrap_or_else(|| compute_default_tier_scope(&DefaultTierScopeInput::default()).scope);
    state.insert("scopes".to_string(), Value::Object(scopes));
    GetTierScopeOutput {
        state: Value::Object(state),
        scope,
    }
}

pub fn compute_load_tier_governance_state(
    input: &LoadTierGovernanceStateInput,
) -> LoadTierGovernanceStateOutput {
    let now_iso = input.now_iso.clone().unwrap_or_else(now_iso_runtime);
    let safe_version = clean_text_runtime(input.policy_version.as_deref().unwrap_or("1.0"), 24);
    let policy_version = if safe_version.is_empty() {
        "1.0".to_string()
    } else {
        safe_version
    };
    let src = compute_read_json(&ReadJsonInput {
        file_path: input.file_path.clone(),
        fallback: Some(Value::Null),
    })
    .value;
    let payload = src.as_object();
    let updated_at = {
        let value = value_to_string(payload.and_then(|m| m.get("updated_at")));
        if value.is_empty() {
            now_iso.clone()
        } else {
            value
        }
    };
    let legacy_scope = compute_default_tier_scope(&DefaultTierScopeInput {
        legacy: Some(json!({
            "live_apply_counts": payload.and_then(|m| m.get("live_apply_counts")).cloned().unwrap_or_else(|| json!({})),
            "shadow_pass_counts": payload.and_then(|m| m.get("shadow_pass_counts")).cloned().unwrap_or_else(|| json!({})),
            "live_apply_safe_aborts": payload.and_then(|m| m.get("live_apply_safe_aborts")).cloned().unwrap_or_else(|| json!({})),
            "shadow_critical_failures": payload.and_then(|m| m.get("shadow_critical_failures")).cloned().unwrap_or_else(|| json!({}))
        })),
        legacy_ts: Some(updated_at.clone()),
    })
    .scope;
    let scopes_src = payload
        .and_then(|m| m.get("scopes"))
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    let mut scopes = serde_json::Map::new();
    for (version, scope) in scopes_src {
        scopes.insert(
            version.to_string(),
            compute_normalize_tier_scope(&NormalizeTierScopeInput {
                scope: Some(scope),
                legacy: None,
                legacy_ts: Some(updated_at.clone()),
            })
            .scope,
        );
    }
    if !scopes
        .get(&policy_version)
        .map(|v| v.is_object())
        .unwrap_or(false)
    {
        scopes.insert(
            policy_version.clone(),
            compute_normalize_tier_scope(&NormalizeTierScopeInput {
                scope: Some(legacy_scope),
                legacy: None,
                legacy_ts: Some(updated_at.clone()),
            })
            .scope,
        );
    }
    let mut out = serde_json::Map::new();
    out.insert(
        "schema_id".to_string(),
        Value::String("inversion_tier_governance_state".to_string()),
    );
    out.insert(
        "schema_version".to_string(),
        Value::String("1.0".to_string()),
    );
    out.insert(
        "active_policy_version".to_string(),
        Value::String(policy_version.clone()),
    );
    out.insert("updated_at".to_string(), Value::String(updated_at));
    out.insert("scopes".to_string(), Value::Object(scopes));

    let got = compute_get_tier_scope(&GetTierScopeInput {
        state: Some(Value::Object(out)),
        policy_version: Some(policy_version),
    });
    let mut state_out = got.state.as_object().cloned().unwrap_or_default();
    state_out.insert("active_scope".to_string(), got.scope);
    LoadTierGovernanceStateOutput {
        state: Value::Object(state_out),
    }
}

pub fn compute_save_tier_governance_state(
    input: &SaveTierGovernanceStateInput,
) -> SaveTierGovernanceStateOutput {
    let now_iso = input.now_iso.clone().unwrap_or_else(now_iso_runtime);
    let safe_version = clean_text_runtime(input.policy_version.as_deref().unwrap_or("1.0"), 24);
    let policy_version = if safe_version.is_empty() {
        "1.0".to_string()
    } else {
        safe_version
    };
    let retention_days = input.retention_days.unwrap_or(365).clamp(1, 3650);
    let src = input
        .state
        .as_ref()
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    let scopes_src = src
        .get("scopes")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    let mut scopes = serde_json::Map::new();
    for (version, scope) in scopes_src {
        scopes.insert(
            version.to_string(),
            compute_prune_tier_scope_events(&PruneTierScopeEventsInput {
                scope: Some(scope),
                retention_days: Some(retention_days),
            })
            .scope,
        );
    }
    if !scopes
        .get(&policy_version)
        .map(|v| v.is_object())
        .unwrap_or(false)
    {
        scopes.insert(
            policy_version.clone(),
            compute_default_tier_scope(&DefaultTierScopeInput::default()).scope,
        );
    }
    let out = json!({
        "schema_id": "inversion_tier_governance_state",
        "schema_version": "1.0",
        "active_policy_version": policy_version,
        "updated_at": now_iso,
        "scopes": scopes
    });
    let _ = compute_write_json_atomic(&WriteJsonAtomicInput {
        file_path: input.file_path.clone(),
        value: Some(out.clone()),
    });
    let active_policy = value_to_string(value_path(Some(&out), &["active_policy_version"]));
    let got = compute_get_tier_scope(&GetTierScopeInput {
        state: Some(out),
        policy_version: Some(active_policy),
    });
    let mut state_out = got.state.as_object().cloned().unwrap_or_default();
    state_out.insert("active_scope".to_string(), got.scope);
    SaveTierGovernanceStateOutput {
        state: Value::Object(state_out),
    }
}

pub fn compute_push_tier_event(input: &PushTierEventInput) -> PushTierEventOutput {
    let mut map = input
        .scope_map
        .as_ref()
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    let target = compute_normalize_target(&NormalizeTargetInput {
        value: input.target.clone(),
    })
    .value;
    let mut rows = map
        .get(&target)
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    rows.push(Value::String(
        input.ts.clone().unwrap_or_else(now_iso_runtime),
    ));
    let normalized = compute_normalize_iso_events(&NormalizeIsoEventsInput {
        src: rows,
        max_rows: Some(10000),
    })
    .events;
    map.insert(
        target,
        Value::Array(
            normalized
                .into_iter()
                .map(Value::String)
                .collect::<Vec<_>>(),
        ),
    );
    PushTierEventOutput {
        map: Value::Object(map),
    }
}

pub fn compute_add_tier_event(input: &AddTierEventInput) -> AddTierEventOutput {
    let now_iso = input.now_iso.clone().unwrap_or_else(now_iso_runtime);
    let policy = input.policy.as_ref();
    let policy_version = {
        let value = clean_text_runtime(
            value_path(policy, &["version"])
                .and_then(|v| v.as_str())
                .unwrap_or("1.0"),
            24,
        );
        if value.is_empty() {
            "1.0".to_string()
        } else {
            value
        }
    };
    let mut state = compute_load_tier_governance_state(&LoadTierGovernanceStateInput {
        file_path: input.file_path.clone(),
        policy_version: Some(policy_version.clone()),
        now_iso: Some(now_iso.clone()),
    })
    .state;
    let got_scope = compute_get_tier_scope(&GetTierScopeInput {
        state: Some(state.clone()),
        policy_version: Some(policy_version.clone()),
    });
    state = got_scope.state;
    let mut scope = got_scope.scope;

    let metric = clean_text_runtime(input.metric.as_deref().unwrap_or(""), 80);
    if matches!(
        metric.as_str(),
        "live_apply_attempts"
            | "live_apply_successes"
            | "live_apply_safe_aborts"
            | "shadow_passes"
            | "shadow_critical_failures"
    ) {
        let map_src = value_path(Some(&scope), &[metric.as_str()])
            .cloned()
            .unwrap_or_else(default_tier_event_map_value);
        let pushed = compute_push_tier_event(&PushTierEventInput {
            scope_map: Some(map_src),
            target: input.target.clone(),
            ts: Some(input.ts.clone().unwrap_or_else(|| now_iso.clone())),
        })
        .map;
        if let Some(scope_obj) = scope.as_object_mut() {
            scope_obj.insert(metric, pushed);
        }
    }

    let mut state_obj = state.as_object().cloned().unwrap_or_default();
    let mut scopes = state_obj
        .get("scopes")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    scopes.insert(policy_version.clone(), scope);
    state_obj.insert("scopes".to_string(), Value::Object(scopes));

    let retention_days = compute_tier_retention_days(&TierRetentionDaysInput {
        policy: input.policy.clone(),
    })
    .days;
    let saved = compute_save_tier_governance_state(&SaveTierGovernanceStateInput {
        file_path: input.file_path.clone(),
        state: Some(Value::Object(state_obj)),
        policy_version: Some(policy_version),
        retention_days: Some(retention_days),
        now_iso: Some(now_iso),
    });
    AddTierEventOutput { state: saved.state }
}

pub fn compute_increment_live_apply_attempt(
    input: &IncrementLiveApplyAttemptInput,
) -> IncrementLiveApplyAttemptOutput {
    let out = compute_add_tier_event(&AddTierEventInput {
        file_path: input.file_path.clone(),
        policy: input.policy.clone(),
        metric: Some("live_apply_attempts".to_string()),
        target: input.target.clone(),
        ts: Some(input.now_iso.clone().unwrap_or_else(now_iso_runtime)),
        now_iso: input.now_iso.clone(),
    });
    IncrementLiveApplyAttemptOutput { state: out.state }
}

pub fn compute_increment_live_apply_success(
    input: &IncrementLiveApplySuccessInput,
) -> IncrementLiveApplySuccessOutput {
    let out = compute_add_tier_event(&AddTierEventInput {
        file_path: input.file_path.clone(),
        policy: input.policy.clone(),
        metric: Some("live_apply_successes".to_string()),
        target: input.target.clone(),
        ts: Some(input.now_iso.clone().unwrap_or_else(now_iso_runtime)),
        now_iso: input.now_iso.clone(),
    });
    IncrementLiveApplySuccessOutput { state: out.state }
}

pub fn compute_increment_live_apply_safe_abort(
    input: &IncrementLiveApplySafeAbortInput,
) -> IncrementLiveApplySafeAbortOutput {
    let out = compute_add_tier_event(&AddTierEventInput {
        file_path: input.file_path.clone(),
        policy: input.policy.clone(),
        metric: Some("live_apply_safe_aborts".to_string()),
        target: input.target.clone(),
        ts: Some(input.now_iso.clone().unwrap_or_else(now_iso_runtime)),
        now_iso: input.now_iso.clone(),
    });
    IncrementLiveApplySafeAbortOutput { state: out.state }
}

pub fn compute_update_shadow_trial_counters(
    input: &UpdateShadowTrialCountersInput,
) -> UpdateShadowTrialCountersOutput {
    let session = input
        .session
        .as_ref()
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    let mode = compute_normalize_mode(&NormalizeModeInput {
        value: Some(value_to_string(session.get("mode"))),
    })
    .value;
    let apply_requested = to_bool_like(session.get("apply_requested"), false);
    let is_shadow_trial = mode == "test" || !apply_requested;
    if !is_shadow_trial {
        return UpdateShadowTrialCountersOutput { state: None };
    }
    let target = compute_normalize_target(&NormalizeTargetInput {
        value: Some(value_to_string(session.get("target"))),
    })
    .value;
    let result = compute_normalize_result(&NormalizeResultInput {
        value: input.result.clone(),
    })
    .value;
    let now_iso = input.now_iso.clone().unwrap_or_else(now_iso_runtime);
    let mut state = compute_load_tier_governance_state(&LoadTierGovernanceStateInput {
        file_path: input.file_path.clone(),
        policy_version: Some(clean_text_runtime(
            value_path(input.policy.as_ref(), &["version"])
                .and_then(|v| v.as_str())
                .unwrap_or("1.0"),
            24,
        )),
        now_iso: Some(now_iso.clone()),
    })
    .state;
    if result == "success" {
        state = compute_add_tier_event(&AddTierEventInput {
            file_path: input.file_path.clone(),
            policy: input.policy.clone(),
            metric: Some("shadow_passes".to_string()),
            target: Some(target.clone()),
            ts: Some(now_iso.clone()),
            now_iso: Some(now_iso.clone()),
        })
        .state;
    }
    if input.destructive == Some(true) || result == "destructive" {
        state = compute_add_tier_event(&AddTierEventInput {
            file_path: input.file_path.clone(),
            policy: input.policy.clone(),
            metric: Some("shadow_critical_failures".to_string()),
            target: Some(target),
            ts: Some(now_iso.clone()),
            now_iso: Some(now_iso),
        })
        .state;
    }
    UpdateShadowTrialCountersOutput { state: Some(state) }
}

pub fn compute_default_harness_state(
    _input: &DefaultHarnessStateInput,
) -> DefaultHarnessStateOutput {
    DefaultHarnessStateOutput {
        state: json!({
            "schema_id": "inversion_maturity_harness_state",
            "schema_version": "1.0",
            "updated_at": now_iso_runtime(),
            "last_run_ts": Value::Null,
            "cursor": 0
        }),
    }
}
