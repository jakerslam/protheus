fn normalize_queue_item(raw: Option<&Map<String, Value>>, now_ts: &str) -> Value {
    let raw = raw.unwrap_or_else(|| {
        static EMPTY: std::sync::OnceLock<Map<String, Value>> = std::sync::OnceLock::new();
        EMPTY.get_or_init(Map::new)
    });
    let summary = {
        let value = clean_text(raw.get("summary"), 220);
        if !value.is_empty() {
            value
        } else {
            let text = clean_text(raw.get("text"), 220);
            if !text.is_empty() {
                text
            } else {
                clean_text(raw.get("payload"), 220).if_empty_then("strategy intake")
            }
        }
    };
    let text = as_str(raw.get("text")).if_empty_then(&as_str(raw.get("payload")));
    let text = text.chars().take(6000).collect::<String>();
    let evidence_refs = raw
        .get("evidence_refs")
        .and_then(Value::as_array)
        .map(|rows| {
            let mut uniq = Vec::<String>::new();
            for row in rows {
                let cleaned = clean_text(Some(row), 200);
                if cleaned.is_empty() || uniq.iter().any(|existing| existing == &cleaned) {
                    continue;
                }
                uniq.push(cleaned);
                if uniq.len() >= 24 {
                    break;
                }
            }
            uniq
        })
        .unwrap_or_default();
    let recommended_generation_mode = normalize_mode(
        raw.get("recommended_generation_mode")
            .or_else(|| raw.get("generation_mode"))
            .or_else(|| raw.get("mode")),
        Some(&recommend_mode(&summary, &text)),
    );
    let uid_candidate = clean_text(raw.get("uid"), 64);
    let uid = if is_alnum(&uid_candidate) {
        uid_candidate
    } else {
        random_uid("si", 24)
    };
    let fingerprint = {
        let raw_fingerprint = clean_text(raw.get("fingerprint"), 40);
        if !raw_fingerprint.is_empty() {
            raw_fingerprint
        } else {
            hash16(&json_string(&json!({
                "source": clean_text(raw.get("source"), 60).if_empty_then("unknown"),
                "kind": clean_text(raw.get("kind"), 40).if_empty_then("signal"),
                "summary": summary,
                "text": text,
                "evidence": evidence_refs,
            })))
        }
    };
    let status_raw = as_str(raw.get("status")).to_ascii_lowercase();
    let status = if matches!(status_raw.as_str(), "consumed" | "dropped") {
        status_raw
    } else {
        "queued".to_string()
    };
    let linked_strategy_id = {
        let value = clean_text(raw.get("linked_strategy_id"), 64);
        if value.is_empty() {
            Value::Null
        } else {
            Value::String(value)
        }
    };
    let mut item = json!({
        "uid": uid,
        "fingerprint": fingerprint,
        "source": clean_text(raw.get("source"), 80).if_empty_then("unknown"),
        "kind": clean_text(raw.get("kind"), 60).if_empty_then("signal"),
        "summary": summary,
        "text": text,
        "evidence_refs": evidence_refs,
        "recommended_generation_mode": recommended_generation_mode,
        "status": status,
        "attempts": clamp_i64(raw.get("attempts"), 0, 1000, 0),
        "created_ts": raw.get("created_ts").filter(|v| parse_ts_ms(&as_str(Some(v))).is_some()).cloned().unwrap_or_else(|| Value::String(now_ts.to_string())),
        "updated_ts": raw.get("updated_ts").filter(|v| parse_ts_ms(&as_str(Some(v))).is_some()).cloned().unwrap_or_else(|| Value::String(now_ts.to_string())),
        "consumed_ts": raw.get("consumed_ts").filter(|v| parse_ts_ms(&as_str(Some(v))).is_some()).cloned().unwrap_or(Value::Null),
        "linked_strategy_id": linked_strategy_id,
    });
    let trust_score = clamp_i64(raw.get("trust_score"), 0, 100, compute_trust_score(&item));
    item["trust_score"] = Value::from(trust_score);
    item["drop_reason"] = {
        let value = clean_text(raw.get("drop_reason"), 200);
        if value.is_empty() {
            Value::Null
        } else {
            Value::String(value)
        }
    };
    item["work_packet"] = ensure_work_packet(&item);
    item
}

fn normalize_profile(raw: Option<&Map<String, Value>>, now_ts: &str) -> Value {
    let raw = raw.unwrap_or_else(|| {
        static EMPTY: std::sync::OnceLock<Map<String, Value>> = std::sync::OnceLock::new();
        EMPTY.get_or_init(Map::new)
    });
    let draft_src = as_object(raw.get("draft")).unwrap_or(raw);
    let mut draft = default_strategy_draft(Some(draft_src));
    let draft_id = clean_text(draft.get("id"), 40);
    let draft_name = clean_text(draft.get("name"), 120);
    let id = normalize_key(
        &clean_text(raw.get("id"), 40).if_empty_then(&draft_id.clone().if_empty_then(&draft_name)),
        40,
    )
    .if_empty_then(&draft_id.clone().if_empty_then("strategy"));
    draft["id"] = Value::String(id.clone());
    draft["name"] = Value::String(
        clean_text(raw.get("name"), 120).if_empty_then(&draft_name.if_empty_then(&id)),
    );
    let objective_primary = {
        let current = as_object(draft.get("objective")).and_then(|v| v.get("primary"));
        clean_text(current.or_else(|| raw.get("objective_primary")), 220).if_empty_then(&format!(
            "Improve outcomes for {}",
            clean_text(draft.get("name"), 120).if_empty_then(&id)
        ))
    };
    draft["objective"] = json!({
        "primary": objective_primary,
        "secondary": draft.pointer("/objective/secondary").cloned().unwrap_or_else(|| json!([])),
        "fitness_metric": draft.pointer("/objective/fitness_metric").cloned().unwrap_or_else(|| Value::String("verified_progress_rate".to_string())),
        "target_window_days": clamp_i64(draft.pointer("/objective/target_window_days"), 1, 365, 14),
    });
    draft["risk_policy"] = json!({
        "allowed_risks": normalize_allowed_risks(draft.pointer("/risk_policy/allowed_risks")),
        "max_risk_per_action": clamp_number(draft.pointer("/risk_policy/max_risk_per_action"), 0.0, 100.0, 35.0),
    });
    let requested_execution_mode = normalize_execution_mode(
        raw.get("execution_mode")
            .or_else(|| {
                raw.get("execution_policy")
                    .and_then(Value::as_object)
                    .and_then(|v| v.get("mode"))
            })
            .or_else(|| draft.pointer("/execution_policy/mode")),
        Some("score_only"),
    );
    let allow_elevated_mode = raw.get("allow_elevated_mode").and_then(Value::as_bool) == Some(true);
    draft["execution_policy"] = json!({
        "mode": if allow_elevated_mode { requested_execution_mode.clone() } else { "score_only".to_string() },
    });
    draft["generation_policy"] = json!({
        "mode": normalize_mode(
            raw.get("generation_mode")
                .or_else(|| raw.get("generation_policy").and_then(Value::as_object).and_then(|v| v.get("mode")))
                .or_else(|| draft.pointer("/generation_policy/mode")),
            Some("hyper-creative"),
        )
    });
    let uid_candidate = clean_text(raw.get("uid"), 64);
    let uid = if is_alnum(&uid_candidate) {
        uid_candidate
    } else {
        stable_uid(&format!("adaptive_strategy_profile|{id}|v1"), "stp", 24)
    };
    let stage_raw = as_str(raw.get("stage")).to_ascii_lowercase();
    let stage = if matches!(stage_raw.as_str(), "trial" | "validated" | "scaled") {
        stage_raw
    } else {
        "theory".to_string()
    };
    let status_raw = as_str(raw.get("status")).to_ascii_lowercase();
    let status = if matches!(status_raw.as_str(), "disabled" | "archived") {
        status_raw
    } else {
        "active".to_string()
    };
    let tags = raw
        .get("tags")
        .and_then(Value::as_array)
        .map(|rows| {
            let mut uniq = Vec::<String>::new();
            for row in rows {
                let tag = normalize_key(&as_str(Some(row)), 32);
                if tag.is_empty() || uniq.iter().any(|existing| existing == &tag) {
                    continue;
                }
                uniq.push(tag);
                if uniq.len() >= 16 {
                    break;
                }
            }
            uniq
        })
        .unwrap_or_default();
    let queue_ref = {
        let value = clean_text(raw.get("queue_ref"), 64);
        if value.is_empty() {
            Value::Null
        } else {
            Value::String(value)
        }
    };
    json!({
        "uid": uid,
        "id": id,
        "name": clean_text(raw.get("name"), 120).if_empty_then(&clean_text(draft.get("name"), 120).if_empty_then("strategy")),
        "stage": stage,
        "status": status,
        "source": clean_text(raw.get("source"), 80).if_empty_then("adaptive_intake"),
        "queue_ref": queue_ref,
        "generated_mode": normalize_mode(
            raw.get("generated_mode")
                .or_else(|| raw.get("generation_mode"))
                .or_else(|| draft.pointer("/generation_policy/mode")),
            Some("hyper-creative"),
        ),
        "requested_execution_mode": requested_execution_mode,
        "elevated_mode_forced_down": !allow_elevated_mode && requested_execution_mode != "score_only",
        "tags": tags,
        "draft": draft,
        "usage": normalize_usage(raw.get("usage").and_then(Value::as_object), now_ts),
        "created_ts": raw.get("created_ts").filter(|v| parse_ts_ms(&as_str(Some(v))).is_some()).cloned().unwrap_or_else(|| Value::String(now_ts.to_string())),
        "updated_ts": raw.get("updated_ts").filter(|v| parse_ts_ms(&as_str(Some(v))).is_some()).cloned().unwrap_or_else(|| Value::String(now_ts.to_string())),
    })
}

fn validate_profile_input(
    raw_profile: Option<&Map<String, Value>>,
    allow_elevated_mode: bool,
) -> Result<Value, String> {
    let normalized = normalize_profile(raw_profile, &now_iso());
    let mut errors = Vec::new();
    if as_str(normalized.get("id")).is_empty() {
        errors.push("id_required");
    }
    if !normalized
        .get("draft")
        .map(Value::is_object)
        .unwrap_or(false)
    {
        errors.push("draft_required");
    }
    if clean_text(normalized.pointer("/draft/objective/primary"), 220).is_empty() {
        errors.push("objective_primary_required");
    }
    if normalized
        .pointer("/draft/risk_policy/allowed_risks")
        .and_then(Value::as_array)
        .map(|rows| rows.is_empty())
        .unwrap_or(true)
    {
        errors.push("risk_policy_allowed_risks_required");
    }
    let mode = normalize_execution_mode(
        normalized.pointer("/draft/execution_policy/mode"),
        Some("score_only"),
    );
    if !EXECUTION_MODES.contains(&mode.as_str()) {
        errors.push("execution_mode_invalid");
    }
    if !allow_elevated_mode && mode != "score_only" {
        errors.push("execution_mode_requires_explicit_override");
    }
    if errors.is_empty() {
        Ok(normalized)
    } else {
        Err(format!(
            "strategy_store: validation_failed:{}",
            errors.join(",")
        ))
    }
}

fn normalize_state(raw: Option<&Value>, fallback: Option<&Value>) -> Value {
    let now_ts = now_iso();
    let base = default_strategy_state();
    let src = raw
        .filter(|v| v.is_object())
        .unwrap_or_else(|| fallback.unwrap_or(&base));
    let src_obj = payload_obj(src);
    let policy = normalize_policy(as_object(src_obj.get("policy")));
    let max_profiles = policy
        .get("max_profiles")
        .and_then(Value::as_i64)
        .unwrap_or(64) as usize;
    let max_queue = policy
        .get("max_queue")
        .and_then(Value::as_i64)
        .unwrap_or(64) as usize;

    let mut profiles_by_id: BTreeMap<String, Value> = BTreeMap::new();
    for profile in src_obj
        .get("profiles")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
    {
        let normalized = normalize_profile(profile.as_object(), &now_ts);
        let id = as_str(normalized.get("id"));
        if id.is_empty() {
            continue;
        }
        let should_replace = profiles_by_id
            .get(&id)
            .map(|existing| {
                parse_ts_ms(&as_str(normalized.get("updated_ts"))).unwrap_or(0)
                    >= parse_ts_ms(&as_str(existing.get("updated_ts"))).unwrap_or(0)
            })
            .unwrap_or(true);
        if should_replace {
            profiles_by_id.insert(id, normalized);
        }
    }
    let mut profiles = profiles_by_id.into_values().collect::<Vec<_>>();
    profiles.sort_by(|a, b| as_str(a.get("id")).cmp(&as_str(b.get("id"))));
    if profiles.len() > max_profiles {
        profiles.truncate(max_profiles);
    }

    let mut queue_by_uid: BTreeMap<String, Value> = BTreeMap::new();
    let now_ms = Utc::now().timestamp_millis();
    for item in src_obj
        .get("intake_queue")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
    {
        let mut normalized = normalize_queue_item(item.as_object(), &now_ts);
        if as_str(normalized.get("status")) == "queued" {
            let drops = queue_drop_reasons(&normalized, &policy, now_ms);
            if !drops.is_empty() {
                normalized["status"] = Value::String("dropped".to_string());
                normalized["drop_reason"] = Value::String(drops.join(","));
                normalized["updated_ts"] = Value::String(now_ts.clone());
            }
        }
        let uid = as_str(normalized.get("uid"));
        if uid.is_empty() {
            continue;
        }
        let should_replace = queue_by_uid
            .get(&uid)
            .map(|existing| {
                parse_ts_ms(&as_str(normalized.get("updated_ts"))).unwrap_or(0)
                    >= parse_ts_ms(&as_str(existing.get("updated_ts"))).unwrap_or(0)
            })
            .unwrap_or(true);
        if should_replace {
            queue_by_uid.insert(uid, normalized);
        }
    }
    let mut intake_queue = queue_by_uid.into_values().collect::<Vec<_>>();
    intake_queue.sort_by(|a, b| {
        parse_ts_ms(&as_str(a.get("created_ts")))
            .unwrap_or(0)
            .cmp(&parse_ts_ms(&as_str(b.get("created_ts"))).unwrap_or(0))
    });
    if intake_queue.len() > max_queue {
        intake_queue = intake_queue.split_off(intake_queue.len() - max_queue);
    }

    let metrics_obj = as_object(src_obj.get("metrics"));
    json!({
        "version": clean_text(src_obj.get("version"), 40).if_empty_then("1.0"),
        "policy": policy,
        "profiles": profiles,
        "intake_queue": intake_queue,
        "metrics": {
            "total_intakes": clamp_i64(metrics_obj.and_then(|m| m.get("total_intakes")), 0, 100_000_000, 0),
            "total_profiles_created": clamp_i64(metrics_obj.and_then(|m| m.get("total_profiles_created")), 0, 100_000_000, 0),
            "total_profiles_updated": clamp_i64(metrics_obj.and_then(|m| m.get("total_profiles_updated")), 0, 100_000_000, 0),
            "total_queue_consumed": clamp_i64(metrics_obj.and_then(|m| m.get("total_queue_consumed")), 0, 100_000_000, 0),
            "total_gc_deleted": clamp_i64(metrics_obj.and_then(|m| m.get("total_gc_deleted")), 0, 100_000_000, 0),
            "last_gc_ts": metrics_obj.and_then(|m| m.get("last_gc_ts")).filter(|v| parse_ts_ms(&as_str(Some(v))).is_some()).cloned().unwrap_or(Value::Null),
            "last_usage_sync_ts": metrics_obj.and_then(|m| m.get("last_usage_sync_ts")).filter(|v| parse_ts_ms(&as_str(Some(v))).is_some()).cloned().unwrap_or(Value::Null),
        }
    })
}

fn resolve_requested_path(root: &Path, raw: &str) -> PathBuf {
    let candidate = PathBuf::from(raw.trim());
    if candidate.is_absolute() {
        candidate
    } else {
        workspace_root(root).join(candidate)
    }
}

fn as_store_path(root: &Path, payload: &Map<String, Value>) -> Result<PathBuf, String> {
    let canonical = store_abs_path(root);
    let raw = as_str(payload.get("file_path"));
    if raw.is_empty() {
        return Ok(canonical);
    }
    let requested = resolve_requested_path(root, &raw);
    if requested != canonical {
        return Err(format!(
            "strategy_store: path override denied (requested={})",
            requested.display()
        ));
    }
    Ok(canonical)
}

fn read_state(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let path = as_store_path(root, payload)?;
    let raw = read_json(&path);
    let fallback = payload
        .get("fallback")
        .cloned()
        .unwrap_or_else(default_strategy_state);
    Ok(normalize_state(Some(&raw), Some(&fallback)))
}

fn ensure_state(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let path = as_store_path(root, payload)?;
    if path.exists() {
        let raw = read_json(&path);
        if raw.is_object() {
            return Ok(normalize_state(Some(&raw), Some(&default_strategy_state())));
        }
    }
    let next = default_strategy_state();
    write_json_atomic(&path, &next)?;
    let meta = as_object(payload.get("meta"));
    append_mutation_log(
        root,
        "ensure",
        DEFAULT_REL_PATH,
        Some(&next),
        meta,
        "ensure_strategy_state",
    )?;
    emit_strategy_pointer(root, meta)?;
    Ok(normalize_state(
        Some(&next),
        Some(&default_strategy_state()),
    ))
}

