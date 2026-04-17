fn web_tooling_provider_contract_targets() -> [&'static str; 10] {
    [
        "brave",
        "duckduckgo",
        "exa",
        "firecrawl",
        "google",
        "minimax",
        "moonshot",
        "perplexity",
        "tavily",
        "xai",
    ]
}

fn normalize_web_tooling_provider(raw: &str) -> String {
    match raw.trim().to_ascii_lowercase().as_str() {
        "kimi" | "moonshot" => "moonshot".to_string(),
        "grok" | "xai" => "xai".to_string(),
        "duck_duck_go" | "duckduckgo" => "duckduckgo".to_string(),
        "brave_search" | "brave" => "brave".to_string(),
        other => other.to_string(),
    }
}

fn normalize_strategy_web_tooling_state(state: &mut Value) {
    if !state.is_object() {
        return;
    }
    if !state.get("web_tooling").map(Value::is_object).unwrap_or(false) {
        state["web_tooling"] = json!({});
    }
    let targets = web_tooling_provider_contract_targets();
    let configured = state["web_tooling"]
        .get("provider_priority")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| row.as_str().map(normalize_web_tooling_provider))
        .filter(|row| targets.iter().any(|target| target == row))
        .collect::<Vec<_>>();
    let mut deduped = Vec::<String>::new();
    for provider in configured {
        if !provider.is_empty() && !deduped.iter().any(|existing| existing == &provider) {
            deduped.push(provider);
        }
    }
    if deduped.is_empty() {
        deduped = targets.iter().map(|row| row.to_string()).collect::<Vec<_>>();
    }
    state["web_tooling"]["provider_priority"] =
        Value::Array(deduped.iter().map(|row| Value::String(row.clone())).collect());
    state["web_tooling"]["provider_contract_targets"] = Value::Array(
        targets
            .iter()
            .map(|target| Value::String((*target).to_string()))
            .collect(),
    );
    state["web_tooling"]["provider_runtime_contract"] = json!({
        "registry_coverage_required": true,
        "discovery_contract_required": true,
        "auth_contract_providers": ["openai_codex", "github_copilot"],
        "updated_ts": now_iso()
    });
}

fn set_state(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let path = as_store_path(root, payload)?;
    let state = payload
        .get("state")
        .cloned()
        .unwrap_or_else(|| Value::Object(payload.clone()));
    let mut normalized = normalize_state(Some(&state), Some(&default_strategy_state()));
    normalize_strategy_web_tooling_state(&mut normalized);
    write_json_atomic(&path, &normalized)?;
    let meta = as_object(payload.get("meta"));
    append_mutation_log(
        root,
        "set",
        DEFAULT_REL_PATH,
        Some(&normalized),
        meta,
        "set_strategy_state",
    )?;
    emit_strategy_pointer(root, meta)?;
    Ok(normalized)
}

fn mutate_state(
    root: &Path,
    payload: &Map<String, Value>,
    reason: &str,
    mutator: impl FnOnce(&mut Value) -> Result<(), String>,
) -> Result<Value, String> {
    let path = as_store_path(root, payload)?;
    let raw = read_json(&path);
    let mut state = normalize_state(Some(&raw), Some(&default_strategy_state()));
    mutator(&mut state)?;
    let mut normalized = normalize_state(Some(&state), Some(&default_strategy_state()));
    normalize_strategy_web_tooling_state(&mut normalized);
    write_json_atomic(&path, &normalized)?;
    let meta = as_object(payload.get("meta"));
    append_mutation_log(
        root,
        "set",
        DEFAULT_REL_PATH,
        Some(&normalized),
        meta,
        reason,
    )?;
    emit_strategy_pointer(root, meta)?;
    Ok(normalized)
}

fn upsert_profile(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let mut action = "none".to_string();
    let mut profile_out = Value::Null;
    let state = mutate_state(root, payload, "upsert_profile", |state| {
        let ts = now_iso();
        let allow_elevated = payload
            .get("meta")
            .and_then(Value::as_object)
            .and_then(|m| m.get("allow_elevated_mode"))
            .and_then(Value::as_bool)
            == Some(true);
        let incoming = validate_profile_input(
            payload.get("profile").and_then(Value::as_object),
            allow_elevated,
        )?;
        let incoming_id = as_str(incoming.get("id"));
        let max_profiles = state
            .pointer("/policy/max_profiles")
            .and_then(Value::as_i64)
            .unwrap_or(64) as usize;
        let mut updated_metric = 0_i64;
        let mut created_metric = 0_i64;
        {
            let profiles = state["profiles"]
                .as_array_mut()
                .ok_or_else(|| "strategy_store: profiles_missing".to_string())?;
            if let Some(idx) = profiles
                .iter()
                .position(|row| as_str(row.get("id")) == incoming_id)
            {
                let existing = normalize_profile(profiles[idx].as_object(), &ts);
                let mut merged = incoming.clone();
                merged["usage"] = if existing.get("usage").map(Value::is_object).unwrap_or(false) {
                    let mut usage = existing.get("usage").cloned().unwrap_or_else(|| json!({}));
                    if let Some(incoming_usage) = incoming.get("usage").and_then(Value::as_object) {
                        for (key, value) in incoming_usage {
                            usage[key] = value.clone();
                        }
                    }
                    usage
                } else {
                    incoming.get("usage").cloned().unwrap_or_else(|| json!({}))
                };
                merged["created_ts"] = existing
                    .get("created_ts")
                    .cloned()
                    .unwrap_or_else(|| Value::String(ts.clone()));
                merged["updated_ts"] = Value::String(ts.clone());
                profiles[idx] = normalize_profile(merged.as_object(), &ts);
                action = "updated".to_string();
                profile_out = profiles[idx].clone();
                updated_metric = 1;
            } else {
                let mut created_map = incoming.as_object().cloned().unwrap_or_default();
                created_map.insert("created_ts".to_string(), Value::String(ts.clone()));
                created_map.insert("updated_ts".to_string(), Value::String(ts.clone()));
                let created = normalize_profile(Some(&created_map), &ts);
                profiles.push(created.clone());
                action = "created".to_string();
                profile_out = created;
                created_metric = 1;
            }
            if profiles.len() > max_profiles {
                profiles.sort_by(|a, b| {
                    parse_ts_ms(&as_str(a.get("updated_ts")))
                        .unwrap_or(0)
                        .cmp(&parse_ts_ms(&as_str(b.get("updated_ts"))).unwrap_or(0))
                });
                let keep_from = profiles.len() - max_profiles;
                profiles.drain(0..keep_from);
            }
            profiles.sort_by(|a, b| as_str(a.get("id")).cmp(&as_str(b.get("id"))));
        }
        if updated_metric > 0 {
            state["metrics"]["total_profiles_updated"] = Value::from(
                clamp_i64(
                    state.pointer("/metrics/total_profiles_updated"),
                    0,
                    100_000_000,
                    0,
                ) + updated_metric,
            );
        }
        if created_metric > 0 {
            state["metrics"]["total_profiles_created"] = Value::from(
                clamp_i64(
                    state.pointer("/metrics/total_profiles_created"),
                    0,
                    100_000_000,
                    0,
                ) + created_metric,
            );
        }
        Ok(())
    })?;
    Ok(json!({"state": state, "action": action, "profile": profile_out}))
}

fn intake_signal(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let mut action = "none".to_string();
    let mut queue_item = Value::Null;
    let state = mutate_state(root, payload, "intake_signal", |state| {
        let ts = now_iso();
        let mut intake_map = payload
            .get("intake")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_else(|| payload.clone());
        intake_map.insert("created_ts".to_string(), Value::String(ts.clone()));
        intake_map.insert("updated_ts".to_string(), Value::String(ts.clone()));
        let mut item = normalize_queue_item(Some(&intake_map), &ts);
        let drops = queue_drop_reasons(&item, &state["policy"], Utc::now().timestamp_millis());
        if !drops.is_empty() {
            item["status"] = Value::String("dropped".to_string());
            item["drop_reason"] = Value::String(drops.join(","));
        }
        let max_queue = state
            .pointer("/policy/max_queue")
            .and_then(Value::as_i64)
            .unwrap_or(64) as usize;
        let intake_recorded = {
            let queue = state["intake_queue"]
                .as_array_mut()
                .ok_or_else(|| "strategy_store: intake_queue_missing".to_string())?;
            if let Some(existing) = queue.iter().find(|row| {
                as_str(row.get("fingerprint")) == as_str(item.get("fingerprint"))
                    && as_str(row.get("status")) == "queued"
            }) {
                action = "deduped".to_string();
                queue_item = existing.clone();
                return Ok(());
            }
            queue.push(item.clone());
            queue.sort_by(|a, b| {
                parse_ts_ms(&as_str(a.get("created_ts")))
                    .unwrap_or(0)
                    .cmp(&parse_ts_ms(&as_str(b.get("created_ts"))).unwrap_or(0))
            });
            if queue.len() > max_queue {
                let drop_count = queue.len() - max_queue;
                queue.drain(0..drop_count);
            }
            true
        };
        if intake_recorded {
            state["metrics"]["total_intakes"] = Value::from(
                clamp_i64(state.pointer("/metrics/total_intakes"), 0, 100_000_000, 0) + 1,
            );
        }
        action = if as_str(item.get("status")) == "dropped" {
            "dropped".to_string()
        } else {
            "queued".to_string()
        };
        queue_item = item;
        Ok(())
    })?;
    Ok(json!({"state": state, "action": action, "queue_item": queue_item}))
}

fn materialize_from_queue(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let qid = as_str(payload.get("queue_uid"));
    if qid.is_empty() {
        return Err("strategy_store: queue_uid_required".to_string());
    }
    let mut action = "none".to_string();
    let mut profile_out = Value::Null;
    let mut queue_out = Value::Null;
    let state = mutate_state(root, payload, "materialize_from_queue", |state| {
        let ts = now_iso();
        let queue_item = {
            let queue = state["intake_queue"]
                .as_array_mut()
                .ok_or_else(|| "strategy_store: intake_queue_missing".to_string())?;
            let idx = queue
                .iter()
                .position(|row| as_str(row.get("uid")) == qid)
                .ok_or_else(|| format!("strategy_store: queue_item_not_found:{qid}"))?;
            normalize_queue_item(queue[idx].as_object(), &ts)
        };
        if as_str(queue_item.get("status")) != "queued" {
            return Err(format!("strategy_store: queue_item_not_queued:{qid}"));
        }
        let mut draft_input = payload
            .get("draft")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        draft_input.insert(
            "source".to_string(),
            Value::String(clean_text(draft_input.get("source"), 80).if_empty_then(
                &clean_text(queue_item.get("source"), 80).if_empty_then("adaptive_intake"),
            )),
        );
        draft_input.insert("queue_ref".to_string(), Value::String(qid.clone()));
        draft_input.insert(
            "generated_mode".to_string(),
            Value::String(normalize_mode(
                draft_input
                    .get("generated_mode")
                    .or_else(|| draft_input.get("generation_mode"))
                    .or_else(|| queue_item.get("recommended_generation_mode")),
                Some("hyper-creative"),
            )),
        );
        let mut tags = draft_input
            .get("tags")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        tags.push(Value::String("adaptive".to_string()));
        tags.push(Value::String("strategy".to_string()));
        draft_input.insert("tags".to_string(), Value::Array(tags));
        if payload
            .get("meta")
            .and_then(Value::as_object)
            .and_then(|m| m.get("allow_elevated_mode"))
            .and_then(Value::as_bool)
            == Some(true)
        {
            draft_input.insert("allow_elevated_mode".to_string(), Value::Bool(true));
        }
        draft_input.insert("created_ts".to_string(), Value::String(ts.clone()));
        draft_input.insert("updated_ts".to_string(), Value::String(ts.clone()));
        let allow_elevated_mode = payload
            .get("meta")
            .and_then(Value::as_object)
            .and_then(|m| m.get("allow_elevated_mode"))
            .and_then(Value::as_bool)
            == Some(true);
        let upsert = validate_profile_input(Some(&draft_input), allow_elevated_mode)?;
        let upsert_id = as_str(upsert.get("id"));
        let mut created_metric = 0_i64;
        let mut updated_metric = 0_i64;
        {
            let profiles = state["profiles"]
                .as_array_mut()
                .ok_or_else(|| "strategy_store: profiles_missing".to_string())?;
            if let Some(existing_idx) = profiles
                .iter()
                .position(|row| as_str(row.get("id")) == upsert_id)
            {
                let prev = normalize_profile(profiles[existing_idx].as_object(), &ts);
                let mut merged = upsert.as_object().cloned().unwrap_or_default();
                merged.insert(
                    "created_ts".to_string(),
                    prev.get("created_ts")
                        .cloned()
                        .unwrap_or_else(|| Value::String(ts.clone())),
                );
                merged.insert(
                    "usage".to_string(),
                    prev.get("usage").cloned().unwrap_or_else(|| json!({})),
                );
                profiles[existing_idx] = normalize_profile(Some(&merged), &ts);
                action = "updated".to_string();
                profile_out = profiles[existing_idx].clone();
                updated_metric = 1;
            } else {
                profiles.push(upsert.clone());
                action = "created".to_string();
                profile_out = upsert.clone();
                created_metric = 1;
            }
            profiles.sort_by(|a, b| as_str(a.get("id")).cmp(&as_str(b.get("id"))));
        }
        let mut consumed = queue_item.clone();
        consumed["status"] = Value::String("consumed".to_string());
        consumed["updated_ts"] = Value::String(ts.clone());
        consumed["consumed_ts"] = Value::String(ts.clone());
        consumed["linked_strategy_id"] = profile_out.get("id").cloned().unwrap_or(Value::Null);
        consumed["attempts"] = Value::from(clamp_i64(queue_item.get("attempts"), 0, 1000, 0) + 1);
        consumed["work_packet"] = ensure_work_packet(&consumed);
        {
            let queue = state["intake_queue"]
                .as_array_mut()
                .ok_or_else(|| "strategy_store: intake_queue_missing".to_string())?;
            let idx = queue
                .iter()
                .position(|row| as_str(row.get("uid")) == qid)
                .ok_or_else(|| format!("strategy_store: queue_item_not_found:{qid}"))?;
            queue[idx] = consumed.clone();
        }
        if updated_metric > 0 {
            state["metrics"]["total_profiles_updated"] = Value::from(
                clamp_i64(
                    state.pointer("/metrics/total_profiles_updated"),
                    0,
                    100_000_000,
                    0,
                ) + updated_metric,
            );
        }
        if created_metric > 0 {
            state["metrics"]["total_profiles_created"] = Value::from(
                clamp_i64(
                    state.pointer("/metrics/total_profiles_created"),
                    0,
                    100_000_000,
                    0,
                ) + created_metric,
            );
        }
        state["metrics"]["total_queue_consumed"] = Value::from(
            clamp_i64(
                state.pointer("/metrics/total_queue_consumed"),
                0,
                100_000_000,
                0,
            ) + 1,
        );
        queue_out = consumed;
        Ok(())
    })?;
    Ok(json!({"state": state, "action": action, "profile": profile_out, "queue_item": queue_out}))
}

fn touch_profile_usage(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let sid = normalize_key(&as_str(payload.get("strategy_id")), 40);
    if sid.is_empty() {
        return Err("strategy_store: strategy_id_required".to_string());
    }
    let touch_ts = payload
        .get("ts")
        .and_then(|v| parse_ts_ms(&as_str(Some(v))).map(|_| as_str(Some(v))))
        .unwrap_or_else(now_iso);
    let mut profile_out = Value::Null;
    let state = mutate_state(root, payload, "touch_profile_usage", |state| {
        let profiles = state["profiles"]
            .as_array_mut()
            .ok_or_else(|| "strategy_store: profiles_missing".to_string())?;
        let idx = profiles
            .iter()
            .position(|row| as_str(row.get("id")) == sid)
            .ok_or_else(|| format!("strategy_store: strategy_not_found:{sid}"))?;
        let mut profile = normalize_profile(profiles[idx].as_object(), &touch_ts);
        let mut usage = normalize_usage(profile.get("usage").and_then(Value::as_object), &touch_ts);
        let mut events = usage
            .get("use_events")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        events.push(Value::String(touch_ts.clone()));
        if events.len() > 256 {
            events = events.split_off(events.len() - 256);
        }
        let cutoff = parse_ts_ms(&touch_ts).unwrap_or(0) - (30_i64 * 24 * 60 * 60 * 1000);
        let uses_30 = events
            .iter()
            .filter(|row| {
                parse_ts_ms(&as_str(Some(row)))
                    .map(|ms| ms >= cutoff)
                    .unwrap_or(false)
            })
            .count() as i64;
        usage["use_events"] = Value::Array(events);
        usage["uses_total"] =
            Value::from(clamp_i64(usage.get("uses_total"), 0, 100_000_000, 0) + 1);
        usage["uses_30d"] = Value::from(uses_30);
        usage["last_used_ts"] = Value::String(touch_ts.clone());
        usage["last_usage_sync_ts"] = Value::String(touch_ts.clone());
        profile["usage"] = usage;
        profile["updated_ts"] = Value::String(touch_ts.clone());
        profiles[idx] = profile.clone();
        profile_out = profile;
        Ok(())
    })?;
    Ok(json!({"state": state, "profile": profile_out}))
}
