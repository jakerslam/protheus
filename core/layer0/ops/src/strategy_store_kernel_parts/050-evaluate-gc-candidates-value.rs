fn normalize_gc_status(raw: &str) -> String {
    match raw.trim().to_ascii_lowercase().as_str() {
        "enabled" | "running" => "active".to_string(),
        "pause" | "paused" | "hold" => "blocked".to_string(),
        "retired" | "archive" | "archived" => "inactive".to_string(),
        "pin" => "pinned".to_string(),
        "protect" => "protected".to_string(),
        other => other.to_string(),
    }
}

fn evaluate_gc_candidates_value(state: &Value, opts: Option<&Map<String, Value>>) -> Value {
    let policy = state
        .get("policy")
        .cloned()
        .unwrap_or_else(|| default_strategy_state()["policy"].clone());
    let opts = opts.unwrap_or_else(|| {
        static EMPTY: std::sync::OnceLock<Map<String, Value>> = std::sync::OnceLock::new();
        EMPTY.get_or_init(Map::new)
    });
    let now_ms = Utc::now().timestamp_millis();
    let inactive_days = clamp_i64(
        opts.get("inactive_days"),
        1,
        365,
        clamp_i64(policy.get("gc_inactive_days"), 1, 365, 21),
    );
    let min_uses_30d = clamp_i64(
        opts.get("min_uses_30d"),
        0,
        1000,
        clamp_i64(policy.get("gc_min_uses_30d"), 0, 1000, 1),
    );
    let protect_new_days = clamp_i64(
        opts.get("protect_new_days"),
        0,
        90,
        clamp_i64(policy.get("gc_protect_new_days"), 0, 90, 3),
    );
    let mut candidates = Vec::new();
    let mut keepers = Vec::new();
    for profile in state
        .get("profiles")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
    {
        let profile = normalize_profile(profile.as_object(), &now_iso());
        let usage = profile.get("usage").cloned().unwrap_or_else(|| json!({}));
        let last_used = usage
            .get("last_used_ts")
            .and_then(|v| parse_ts_ms(&as_str(Some(v))));
        let created = profile
            .get("created_ts")
            .and_then(|v| parse_ts_ms(&as_str(Some(v))));
        let age_days = last_used
            .map(|ms| (now_ms - ms) as f64 / (24.0 * 60.0 * 60.0 * 1000.0))
            .map(|days| if days.is_finite() { days.max(0.0) } else { 0.0 });
        let new_age_days = created
            .map(|ms| (now_ms - ms) as f64 / (24.0 * 60.0 * 60.0 * 1000.0))
            .map(|days| if days.is_finite() { days.max(0.0) } else { 0.0 });
        let uses_30 = clamp_i64(usage.get("uses_30d"), 0, 1000, 0);
        let stale = age_days
            .map(|days| days > inactive_days as f64)
            .unwrap_or(true);
        let low_use = uses_30 < min_uses_30d;
        let status = normalize_gc_status(&as_str(profile.get("status")));
        let protected_status = matches!(status.as_str(), "active" | "pinned" | "protected");
        let protected_new = new_age_days
            .map(|days| days < protect_new_days as f64)
            .unwrap_or(false);
        let removable = stale && low_use && !protected_new && !protected_status;
        let row = json!({
            "id": profile.get("id").cloned().unwrap_or(Value::Null),
            "uid": profile.get("uid").cloned().unwrap_or(Value::Null),
            "status": status,
            "stage": profile.get("stage").cloned().unwrap_or(Value::Null),
            "age_days_since_last_use": age_days.map(|days| (days * 1000.0).round() / 1000.0),
            "age_days_since_created": new_age_days.map(|days| (days * 1000.0).round() / 1000.0),
            "uses_30d": uses_30,
            "removable": removable,
            "reason": if removable {
                format!("stale>{inactive_days}d and uses_30d<{min_uses_30d}")
            } else if protected_status {
                "status_protected".to_string()
            } else if protected_new {
                format!("protected_new<{protect_new_days}d")
            } else if stale {
                format!("uses_30d>={min_uses_30d}")
            } else {
                format!("last_used<={inactive_days}d")
            }
        });
        if removable {
            candidates.push(row);
        } else {
            keepers.push(row);
        }
    }
    candidates.sort_by(|a, b| {
        let age_a = a
            .get("age_days_since_last_use")
            .and_then(Value::as_f64)
            .unwrap_or(-1.0);
        let age_b = b
            .get("age_days_since_last_use")
            .and_then(Value::as_f64)
            .unwrap_or(-1.0);
        age_b
            .partial_cmp(&age_a)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| as_str(a.get("id")).cmp(&as_str(b.get("id"))))
    });
    keepers.sort_by(|a, b| as_str(a.get("id")).cmp(&as_str(b.get("id"))));
    json!({
        "policy": {
            "inactive_days": inactive_days,
            "min_uses_30d": min_uses_30d,
            "protect_new_days": protect_new_days,
        },
        "candidates": candidates,
        "keepers": keepers,
    })
}

fn gc_profiles(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let apply = payload.get("apply").and_then(Value::as_bool) == Some(true);
    let mut summary = Value::Null;
    let state = mutate_state(
        root,
        payload,
        if apply {
            "gc_profiles_apply"
        } else {
            "gc_profiles_preview"
        },
        |state| {
            let evals = evaluate_gc_candidates_value(
                state,
                payload
                    .get("opts")
                    .and_then(Value::as_object)
                    .or_else(|| as_object(payload.get("gc_opts")))
                    .or_else(|| as_object(payload.get("options")))
                    .or_else(|| as_object(payload.get("payload"))),
            );
            summary = evals.clone();
            if !apply {
                return Ok(());
            }
            let remove_ids = evals
                .get("candidates")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .map(|row| as_str(row.get("id")))
                .collect::<Vec<_>>();
            if remove_ids.is_empty() {
                return Ok(());
            }
            let profiles = state["profiles"]
                .as_array_mut()
                .ok_or_else(|| "strategy_store: profiles_missing".to_string())?;
            profiles.retain(|row| !remove_ids.iter().any(|id| id == &as_str(row.get("id"))));
            state["metrics"]["total_gc_deleted"] = Value::from(
                clamp_i64(
                    state.pointer("/metrics/total_gc_deleted"),
                    0,
                    100_000_000,
                    0,
                ) + remove_ids.len() as i64,
            );
            state["metrics"]["last_gc_ts"] = Value::String(now_iso());
            Ok(())
        },
    )?;
    Ok(json!({
        "state": state,
        "apply": apply,
        "policy": summary.get("policy").cloned().unwrap_or(Value::Null),
        "removed": summary.get("candidates").cloned().unwrap_or_else(|| json!([])),
        "kept": summary.get("keepers").cloned().unwrap_or_else(|| json!([])),
    }))
}

fn run_command(root: &Path, command: &str, payload: &Map<String, Value>) -> Result<Value, String> {
    match command {
        "paths" => Ok(json!({
            "default_rel_path": DEFAULT_REL_PATH,
            "default_abs_path": default_abs_path(root).to_string_lossy(),
            "store_abs_path": store_abs_path(root).to_string_lossy(),
        })),
        "default-state" => Ok(default_strategy_state()),
        "default-draft" => Ok(default_strategy_draft(
            payload
                .get("seed")
                .and_then(Value::as_object)
                .or_else(|| Some(payload)),
        )),
        "normalize-mode" => Ok(
            json!({"mode": normalize_mode(payload.get("value").or_else(|| payload.get("mode")), Some(&as_str(payload.get("fallback")).if_empty_then("hyper-creative")))}),
        ),
        "normalize-execution-mode" => Ok(
            json!({"mode": normalize_execution_mode(payload.get("value").or_else(|| payload.get("mode")), Some(&as_str(payload.get("fallback")).if_empty_then("score_only")))}),
        ),
        "normalize-profile" => {
            let now_ts = payload
                .get("now_ts")
                .and_then(|v| parse_ts_ms(&as_str(Some(v))).map(|_| as_str(Some(v))))
                .unwrap_or_else(now_iso);
            Ok(normalize_profile(
                payload
                    .get("profile")
                    .and_then(Value::as_object)
                    .or_else(|| Some(payload)),
                &now_ts,
            ))
        }
        "validate-profile" => validate_profile_input(
            payload
                .get("profile")
                .and_then(Value::as_object)
                .or_else(|| Some(payload)),
            payload
                .get("allow_elevated_mode")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        ),
        "normalize-queue-item" => {
            let now_ts = payload
                .get("now_ts")
                .and_then(|v| parse_ts_ms(&as_str(Some(v))).map(|_| as_str(Some(v))))
                .unwrap_or_else(now_iso);
            Ok(normalize_queue_item(
                payload
                    .get("item")
                    .and_then(Value::as_object)
                    .or_else(|| Some(payload)),
                &now_ts,
            ))
        }
        "recommend-mode" => Ok(
            json!({"mode": recommend_mode(&clean_text(payload.get("summary"), 220), &clean_text(payload.get("text"), 6000))}),
        ),
        "read-state" => read_state(root, payload),
        "ensure-state" => ensure_state(root, payload),
        "set-state" => set_state(root, payload),
        "upsert-profile" => upsert_profile(root, payload),
        "intake-signal" => intake_signal(root, payload),
        "materialize-from-queue" => materialize_from_queue(root, payload),
        "touch-profile-usage" => touch_profile_usage(root, payload),
        "evaluate-gc-candidates" => {
            let state = if let Some(raw_state) = payload.get("state") {
                normalize_state(Some(raw_state), Some(&default_strategy_state()))
            } else {
                let path = as_store_path(root, payload)?;
                let raw = read_json(&path);
                normalize_state(Some(&raw), Some(&default_strategy_state()))
            };
            Ok(evaluate_gc_candidates_value(
                &state,
                payload
                    .get("opts")
                    .and_then(Value::as_object)
                    .or_else(|| Some(payload)),
            ))
        }
        "gc-profiles" => gc_profiles(root, payload),
        _ => Err("strategy_store_kernel_unknown_command".to_string()),
    }
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let Some(command) = argv.first().map(|v| v.as_str()) else {
        usage();
        return 1;
    };
    if matches!(command, "help" | "--help" | "-h") {
        usage();
        return 0;
    }
    let payload = match payload_json(argv) {
        Ok(value) => value,
        Err(err) => {
            print_json_line(&cli_error("strategy_store_kernel", &err));
            return 1;
        }
    };
    let payload = payload_obj(&payload).clone();
    match run_command(root, command, &payload) {
        Ok(out) => {
            print_json_line(&cli_receipt("strategy_store_kernel", out));
            0
        }
        Err(err) => {
            print_json_line(&cli_error("strategy_store_kernel", &err));
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "strategy-store-kernel-{}-{}-{}",
            name,
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn normalize_profile_forces_score_only_without_override() {
        let normalized = normalize_profile(
            Some(payload_obj(&json!({
                "id": "ship_it",
                "execution_mode": "execute",
                "draft": {"objective": {"primary": "Ship it"}}
            }))),
            &now_iso(),
        );
        assert_eq!(
            normalized
                .pointer("/draft/execution_policy/mode")
                .and_then(Value::as_str),
            Some("score_only")
        );
        assert_eq!(
            normalized
                .get("elevated_mode_forced_down")
                .and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn intake_materialize_touch_and_gc_round_trip() {
        let root = temp_root("roundtrip");
        let intake = run_command(
            &root,
            "intake-signal",
            payload_obj(&json!({
                "intake": {
                    "source": "manual",
                    "kind": "signal",
                    "summary": "Investigate durable execution for strategy queue",
                    "text": "Need a durable strategy queue with clear ownership.",
                    "evidence_refs": ["doc://proof"]
                }
            })),
        )
        .unwrap();
        assert_eq!(intake.get("action").and_then(Value::as_str), Some("queued"));
        let qid = intake
            .pointer("/queue_item/uid")
            .and_then(Value::as_str)
            .unwrap()
            .to_string();

        let materialized = run_command(
            &root,
            "materialize-from-queue",
            payload_obj(&json!({
                "queue_uid": qid,
                "draft": {
                    "id": "durable_queue",
                    "name": "Durable Queue",
                    "draft": {"objective": {"primary": "Ship durable queue"}}
                }
            })),
        )
        .unwrap();
        assert_eq!(
            materialized.get("action").and_then(Value::as_str),
            Some("created")
        );
        let strategy_id = materialized
            .pointer("/profile/id")
            .and_then(Value::as_str)
            .unwrap()
            .to_string();

        let touched = run_command(
            &root,
            "touch-profile-usage",
            payload_obj(&json!({"strategy_id": strategy_id, "ts": "2026-03-17T12:00:00Z"})),
        )
        .unwrap();
        assert_eq!(
            touched
                .pointer("/profile/usage/uses_total")
                .and_then(Value::as_i64),
            Some(1)
        );

        let gc = run_command(&root, "evaluate-gc-candidates", payload_obj(&json!({}))).unwrap();
        assert_eq!(
            gc.get("candidates")
                .and_then(Value::as_array)
                .map(|rows| rows.len()),
            Some(0)
        );
    }

    #[test]
    fn ensure_state_writes_mutation_artifacts() {
        let root = temp_root("ensure");
        let state = run_command(&root, "ensure-state", payload_obj(&json!({}))).unwrap();
        assert!(state.get("policy").is_some());
        assert!(mutation_log_path(&root).exists());
        assert!(pointers_path(&root).exists());
        assert!(pointer_index_path(&root).exists());
    }
}
