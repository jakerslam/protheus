pub fn compute_parse_lane_decision(input: &ParseLaneDecisionInput) -> ParseLaneDecisionOutput {
    let args = input
        .args
        .as_ref()
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    let lane_raw = {
        let candidates = [
            value_to_string(args.get("brain_lane")),
            value_to_string(args.get("brain-lane")),
            value_to_string(args.get("generation_lane")),
            value_to_string(args.get("generation-lane")),
        ];
        candidates
            .into_iter()
            .find(|v| !v.trim().is_empty())
            .unwrap_or_default()
    };
    let lane = compute_normalize_token(&NormalizeTokenInput {
        value: Some(lane_raw),
        max_len: Some(120),
    })
    .value;
    if !lane.is_empty() {
        return ParseLaneDecisionOutput {
            selected_lane: lane,
            source: "arg".to_string(),
            route: None,
        };
    }
    ParseLaneDecisionOutput {
        selected_lane: String::new(),
        source: "none".to_string(),
        route: None,
    }
}

pub fn compute_sweep_expired_sessions(
    input: &SweepExpiredSessionsInput,
) -> SweepExpiredSessionsOutput {
    let paths = input
        .paths
        .as_ref()
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    let policy = input.policy.clone().unwrap_or_else(|| json!({}));
    let date_str = clean_text_runtime(input.date_str.as_deref().unwrap_or(""), 32);
    let now_iso = input.now_iso.clone().unwrap_or_else(now_iso_runtime);
    let now_ms = parse_ts_ms_runtime(&now_iso);
    let store = compute_load_active_sessions(&LoadActiveSessionsInput {
        file_path: paths
            .get("active_sessions_path")
            .and_then(|v| v.as_str())
            .map(|v| v.to_string()),
        now_iso: Some(now_iso.clone()),
    })
    .store;
    let sessions = store
        .get("sessions")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let mut expired = Vec::new();
    let mut keep = Vec::new();
    for session in sessions {
        let expires_ms = parse_ts_ms_runtime(&value_to_string(
            session.as_object().and_then(|m| m.get("expires_at")),
        ));
        if expires_ms > 0 && expires_ms <= now_ms {
            expired.push(session);
        } else {
            keep.push(session);
        }
    }
    if expired.is_empty() {
        return SweepExpiredSessionsOutput {
            expired_count: 0,
            sessions: keep,
        };
    }
    let _ = compute_save_active_sessions(&SaveActiveSessionsInput {
        file_path: paths
            .get("active_sessions_path")
            .and_then(|v| v.as_str())
            .map(|v| v.to_string()),
        store: Some(json!({ "sessions": keep.clone() })),
        now_iso: Some(now_iso.clone()),
    });
    for session in expired {
        let session_obj = session.as_object().cloned().unwrap_or_default();
        let row = json!({
            "ts": now_iso.clone(),
            "type": "inversion_auto_revert",
            "reason": "session_timeout",
            "session_id": clean_text_runtime(&value_to_string(session_obj.get("session_id")), 80),
            "objective": clean_text_runtime(&value_to_string(session_obj.get("objective")), 220),
            "target": compute_normalize_target(&NormalizeTargetInput {
                value: Some(value_to_string(session_obj.get("target")))
            }).value,
            "outcome_trit": 0,
            "result": "neutral",
            "certainty": js_number_for_extract(session_obj.get("certainty")).unwrap_or(0.0)
        });
        let _ = compute_append_jsonl(&AppendJsonlInput {
            file_path: paths
                .get("receipts_path")
                .and_then(|v| v.as_str())
                .map(|v| v.to_string()),
            row: Some(row.clone()),
        });
        let objective = value_to_string(session_obj.get("objective"));
        let signature = {
            let sig = value_to_string(session_obj.get("signature"));
            if sig.is_empty() {
                objective.clone()
            } else {
                sig
            }
        };
        let id_seed = format!(
            "{}|{}|timeout",
            value_to_string(row.as_object().and_then(|m| m.get("session_id"))),
            now_iso
        );
        let library_target = compute_normalize_target(&NormalizeTargetInput {
            value: Some(value_to_string(
                row.as_object().and_then(|m| m.get("target")),
            )),
        })
        .value;
        let library_impact = compute_normalize_impact(&NormalizeImpactInput {
            value: Some(value_to_string(session_obj.get("impact"))),
        })
        .value;
        let library_certainty = {
            let certainty = js_number_for_extract(row.as_object().and_then(|m| m.get("certainty")))
                .unwrap_or(0.0);
            (clamp_number(certainty, 0.0, 1.0) * 1_000_000.0).round() / 1_000_000.0
        };
        let library_filter_stack = compute_normalize_list(&NormalizeListInput {
            value: Some(
                session_obj
                    .get("filter_stack")
                    .cloned()
                    .unwrap_or(Value::Array(vec![])),
            ),
            max_len: Some(120),
        })
        .items;
        let library_maturity_band = compute_normalize_token(&NormalizeTokenInput {
            value: Some(value_to_string(session_obj.get("maturity_band"))),
            max_len: Some(24),
        })
        .value;
        let library_session_id = clean_text_runtime(
            &value_to_string(row.as_object().and_then(|m| m.get("session_id"))),
            80,
        );
        let _ = compute_append_jsonl(&AppendJsonlInput {
            file_path: paths
                .get("library_path")
                .and_then(|v| v.as_str())
                .map(|v| v.to_string()),
            row: Some(json!({
                "id": stable_id_runtime(&id_seed, "ifl"),
                "ts": now_iso.clone(),
                "objective": clean_text_runtime(&objective, 240),
                "objective_id": clean_text_runtime(&value_to_string(session_obj.get("objective_id")), 120),
                "signature": clean_text_runtime(&signature, 240),
                "signature_tokens": compute_tokenize_text(&TokenizeTextInput { value: Some(signature), max_tokens: Some(64) }).tokens,
                "target": library_target,
                "impact": library_impact,
                "certainty": library_certainty,
                "filter_stack": library_filter_stack,
                "outcome_trit": 0,
                "result": "neutral",
                "maturity_band": library_maturity_band,
                "session_id": library_session_id
            })),
        });
        if to_bool_like(
            value_path(Some(&policy), &["telemetry", "emit_events"]),
            false,
        ) {
            let _ = compute_emit_event(&EmitEventInput {
                events_dir: paths
                    .get("events_dir")
                    .and_then(|v| v.as_str())
                    .map(|v| v.to_string()),
                date_str: Some(date_str.clone()),
                event_type: Some("session_auto_revert".to_string()),
                payload: Some(row),
                emit_events: Some(true),
                now_iso: Some(now_iso.clone()),
            });
        }
    }
    let _ = compute_trim_library(&TrimLibraryInput {
        file_path: paths
            .get("library_path")
            .and_then(|v| v.as_str())
            .map(|v| v.to_string()),
        max_entries: value_path(Some(&policy), &["library", "max_entries"]).cloned(),
    });
    SweepExpiredSessionsOutput {
        expired_count: (store
            .get("sessions")
            .and_then(|v| v.as_array())
            .map(|rows| rows.len())
            .unwrap_or(0)
            .saturating_sub(keep.len())) as i64,
        sessions: keep,
    }
}

pub fn compute_load_impossibility_signals(
    input: &LoadImpossibilitySignalsInput,
) -> LoadImpossibilitySignalsOutput {
    let policy = input.policy.clone().unwrap_or_else(|| json!({}));
    let date_str = clean_text_runtime(input.date_str.as_deref().unwrap_or(""), 32);
    let root = input.root.clone().unwrap_or_default();
    let paths_cfg = value_path(Some(&policy), &["organ", "trigger_detection", "paths"])
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();

    let resolve_path = |raw: Option<&Value>| -> String {
        let p = clean_text_runtime(&value_to_string(raw), 420);
        if p.is_empty() {
            return String::new();
        }
        if Path::new(&p).is_absolute() || root.is_empty() {
            p
        } else {
            Path::new(&root).join(p).to_string_lossy().to_string()
        }
    };
    let regime_path = resolve_path(paths_cfg.get("regime_latest_path"));
    let mirror_path = resolve_path(paths_cfg.get("mirror_latest_path"));
    let simulation_dir = resolve_path(paths_cfg.get("simulation_dir"));
    let red_team_dir = resolve_path(paths_cfg.get("red_team_runs_dir"));
    let drift_governor_path = resolve_path(paths_cfg.get("drift_governor_path"));

    let regime = compute_read_json(&ReadJsonInput {
        file_path: Some(regime_path.clone()),
        fallback: Some(Value::Null),
    })
    .value;
    let mirror = compute_read_json(&ReadJsonInput {
        file_path: Some(mirror_path.clone()),
        fallback: Some(Value::Null),
    })
    .value;
    let simulation_by_date = if simulation_dir.is_empty() || date_str.is_empty() {
        String::new()
    } else {
        Path::new(&simulation_dir)
            .join(format!("{date_str}.json"))
            .to_string_lossy()
            .to_string()
    };
    let simulation_path =
        if !simulation_by_date.is_empty() && Path::new(&simulation_by_date).exists() {
            simulation_by_date
        } else {
            compute_latest_json_file_in_dir(&LatestJsonFileInDirInput {
                dir_path: Some(simulation_dir.clone()),
            })
            .file_path
            .unwrap_or_default()
        };
    let simulation = compute_read_json(&ReadJsonInput {
        file_path: Some(simulation_path.clone()),
        fallback: Some(Value::Null),
    })
    .value;
    let red_team_path = compute_latest_json_file_in_dir(&LatestJsonFileInDirInput {
        dir_path: Some(red_team_dir.clone()),
    })
    .file_path
    .unwrap_or_default();
    let red_team = compute_read_json(&ReadJsonInput {
        file_path: Some(red_team_path.clone()),
        fallback: Some(Value::Null),
    })
    .value;
    let drift_governor = compute_read_json(&ReadJsonInput {
        file_path: Some(drift_governor_path.clone()),
        fallback: Some(Value::Null),
    })
    .value;
    let trit_from_regime = normalize_trit_value(
        value_path(Some(&regime), &["context", "trit", "trit"]).unwrap_or(&Value::Null),
    );
    let trit_from_drift = normalize_trit_value(
        value_path(
            Some(&drift_governor),
            &["last_decision", "trit_shadow", "belief", "trit"],
        )
        .unwrap_or(&Value::Null),
    );
    let trit = if trit_from_regime != 0 {
        trit_from_regime
    } else {
        trit_from_drift
    };
    let trit_label = if trit > 0 {
        "ok"
    } else if trit < 0 {
        "pain"
    } else {
        "unknown"
    };
    let regime_name = clean_text_runtime(
        &value_to_string(value_path(Some(&regime), &["selected_regime"])),
        64,
    )
    .to_lowercase();
    let constrained_re = Regex::new("(constrained|emergency|defensive|degraded|critical)").unwrap();
    let mirror_reasons = value_path(Some(&mirror), &["reasons"])
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
        .iter()
        .map(|x| clean_text_runtime(&value_to_string(Some(x)), 120))
        .filter(|x| !x.is_empty())
        .take(8)
        .collect::<Vec<_>>();
    let rel = |p: &str| -> Option<String> {
        if p.is_empty() {
            return None;
        }
        if root.is_empty() {
            return Some(p.to_string());
        }
        let v = rel_path_runtime(&root, p);
        if v.is_empty() || v.starts_with("..") {
            Some(p.to_string())
        } else {
            Some(v)
        }
    };

    LoadImpossibilitySignalsOutput {
        signals: json!({
            "regime": {
                "path": rel(&regime_path),
                "selected_regime": if regime_name.is_empty() { "unknown".to_string() } else { regime_name.clone() },
                "confidence": clamp_number(js_number_for_extract(value_path(Some(&regime), &["candidate_confidence"])).unwrap_or(0.0), 0.0, 1.0),
                "constrained": constrained_re.is_match(&regime_name)
            },
            "mirror": {
                "path": rel(&mirror_path),
                "pressure_score": clamp_number(js_number_for_extract(value_path(Some(&mirror), &["pressure_score"])).unwrap_or(0.0), 0.0, 1.0),
                "confidence": clamp_number(js_number_for_extract(value_path(Some(&mirror), &["confidence"])).unwrap_or(0.0), 0.0, 1.0),
                "reasons": mirror_reasons
            },
            "simulation": {
                "path": rel(&simulation_path),
                "predicted_drift": clamp_number(js_number_for_extract(value_path(Some(&simulation), &["checks_effective", "drift_rate", "value"])).unwrap_or(0.0), 0.0, 1.0),
                "predicted_yield": clamp_number(js_number_for_extract(value_path(Some(&simulation), &["checks_effective", "yield_rate", "value"])).unwrap_or(0.0), 0.0, 1.0)
            },
            "red_team": {
                "path": rel(&red_team_path),
                "critical_fail_cases": clamp_int_value(value_path(Some(&red_team), &["summary", "critical_fail_cases"]), 0, 100000, 0),
                "pass_cases": clamp_int_value(value_path(Some(&red_team), &["summary", "pass_cases"]), 0, 100000, 0),
                "fail_cases": clamp_int_value(value_path(Some(&red_team), &["summary", "fail_cases"]), 0, 100000, 0)
            },
            "trit": {
                "value": trit,
                "label": trit_label
            }
        }),
    }
}
