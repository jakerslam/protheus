fn runtime_web_tooling_provider_defaults() -> &'static [&'static str] {
    &[
        "brave",
        "gemini",
        "grok",
        "kimi",
        "perplexity",
        "firecrawl",
        "exa",
        "tavily",
        "duckduckgo",
    ]
}

fn runtime_web_tooling_snapshot() -> Value {
    let env_map = std::env::vars().collect::<std::collections::HashMap<String, String>>();
    let auth_sources = crate::contract_lane_utils::web_tooling_auth_sources_from_env(&env_map);
    let auth_any_present = !auth_sources.is_empty();
    let readiness = if auth_sources.is_empty() {
        "auth_missing"
    } else {
        "ready"
    };
    json!({
        "provider_order": runtime_web_tooling_provider_defaults(),
        "auth_sources": auth_sources,
        "auth_any_present": auth_any_present,
        "readiness": readiness
    })
}

fn run_heartbeat(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let contract = load_json_or(
        root,
        HEARTBEAT_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "company_team_heartbeat_contract",
            "default_interval_seconds": 300,
            "max_queue_depth_warn": 50,
            "status_levels": ["healthy", "degraded", "critical"]
        }),
    );
    let mut errors = Vec::<String>::new();
    if contract
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("company_heartbeat_contract_version_must_be_v1".to_string());
    }
    if contract
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "company_team_heartbeat_contract"
    {
        errors.push("company_heartbeat_contract_kind_invalid".to_string());
    }
    let op = clean(
        parsed
            .flags
            .get("op")
            .cloned()
            .or_else(|| parsed.positional.get(1).cloned())
            .unwrap_or_else(|| "tick".to_string()),
        30,
    )
    .to_ascii_lowercase();
    if strict && !matches!(op.as_str(), "tick" | "status" | "remote-feed") {
        errors.push("company_heartbeat_op_invalid".to_string());
    }
    let team = team_slug(
        parsed
            .flags
            .get("team")
            .map(String::as_str)
            .unwrap_or("default-team"),
    );
    if !errors.is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "company_plane_heartbeat",
            "errors": errors
        });
    }

    let state_path = heartbeat_state_path(root, &team);
    let mut state = read_json(&state_path).unwrap_or_else(|| {
        json!({
            "version": "v1",
            "team": team,
            "sequence": 0u64,
            "status": "healthy",
            "agents_online": 0u64,
            "queue_depth": 0u64,
            "last_beat_ts": Value::Null
        })
    });
    if !state.is_object() {
        state = json!({});
    }
    let mut remote_feed = read_json(&heartbeat_remote_feed_path(root)).unwrap_or_else(|| {
        json!({
            "version": "v1",
            "teams": {},
            "updated_at": Value::Null
        })
    });
    if !remote_feed
        .get("teams")
        .map(Value::is_object)
        .unwrap_or(false)
    {
        remote_feed["teams"] = Value::Object(serde_json::Map::new());
    }

    if op == "status" {
        let duality_state = load_duality_state_snapshot(root);
        let web_tooling = runtime_web_tooling_snapshot();
        let mut out = json!({
            "ok": true,
            "strict": strict,
            "type": "company_plane_heartbeat",
            "lane": "core/layer0/ops",
            "team": team,
            "op": op,
            "state": state,
            "duality_state": duality_state,
            "web_tooling": web_tooling,
            "remote_feed_path": heartbeat_remote_feed_path(root).display().to_string(),
            "claim_evidence": [
                {
                    "id": "V6-COMPANY-001.4",
                    "claim": "team_heartbeat_status_surfaces_always_on_monitoring_state",
                    "evidence": {
                        "team": team
                    }
                }
            ]
        });
        out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
        return out;
    }

    if op == "remote-feed" {
        let duality_state = load_duality_state_snapshot(root);
        let web_tooling = runtime_web_tooling_snapshot();
        let mut out = json!({
            "ok": true,
            "strict": strict,
            "type": "company_plane_heartbeat",
            "lane": "core/layer0/ops",
            "team": team,
            "op": op,
            "remote_feed": remote_feed,
            "duality_state": duality_state,
            "web_tooling": web_tooling,
            "artifact": {
                "path": heartbeat_remote_feed_path(root).display().to_string()
            },
            "claim_evidence": [
                {
                    "id": "V6-COMPANY-001.4",
                    "claim": "remote_mobile_safe_team_heartbeat_feed_is_available",
                    "evidence": {
                        "team_count": remote_feed
                            .get("teams")
                            .and_then(Value::as_object)
                            .map(|m| m.len())
                            .unwrap_or(0)
                    }
                }
            ]
        });
        out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
        return out;
    }

    let sequence = state
        .get("sequence")
        .and_then(Value::as_u64)
        .unwrap_or(0)
        .saturating_add(1);
    let status = clean(
        parsed
            .flags
            .get("status")
            .cloned()
            .unwrap_or_else(|| "healthy".to_string()),
        20,
    )
    .to_ascii_lowercase();
    let status_allowed = contract
        .get("status_levels")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(Value::as_str)
        .any(|s| s == status);
    if strict && !status_allowed {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "company_plane_heartbeat",
            "errors": ["company_heartbeat_status_invalid"]
        });
    }
    let agents_online = parse_u64(parsed.flags.get("agents-online"), 0);
    let queue_depth = parse_u64(parsed.flags.get("queue-depth"), 0);
    let warn_queue = contract
        .get("max_queue_depth_warn")
        .and_then(Value::as_u64)
        .unwrap_or(50);
    let web_tooling = runtime_web_tooling_snapshot();
    let web_tooling_missing_auth = !web_tooling
        .get("auth_any_present")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let duality = company_heartbeat_duality_snapshot(
        root,
        &team,
        sequence,
        &status,
        agents_online,
        queue_depth,
        true,
    );
    let duality_hard_block = duality
        .get("hard_block")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let mut effective_status = status.clone();
    if duality_hard_block {
        effective_status = "critical".to_string();
    }
    let degraded = effective_status == "degraded"
        || effective_status == "critical"
        || queue_depth > warn_queue
        || duality_hard_block
        || (strict && web_tooling_missing_auth);
    let recommended_clearance_tier = duality
        .get("recommended_clearance_tier")
        .and_then(Value::as_i64)
        .unwrap_or(3);

    state["version"] = Value::String("v1".to_string());
    state["team"] = Value::String(team.clone());
    state["sequence"] = Value::Number(serde_json::Number::from(sequence));
    state["status"] = Value::String(effective_status.clone());
    state["agents_online"] = Value::Number(serde_json::Number::from(agents_online));
    state["queue_depth"] = Value::Number(serde_json::Number::from(queue_depth));
    state["degraded"] = Value::Bool(degraded);
    state["web_tooling"] = web_tooling.clone();
    state["duality"] = duality.clone();
    state["recommended_clearance_tier"] = Value::Number(serde_json::Number::from(
        recommended_clearance_tier.clamp(1, 5),
    ));
    state["interval_seconds"] = Value::Number(serde_json::Number::from(
        contract
            .get("default_interval_seconds")
            .and_then(Value::as_u64)
            .unwrap_or(300),
    ));
    state["last_beat_ts"] = Value::String(crate::now_iso());
    let _ = write_json(&state_path, &state);

    remote_feed["version"] = Value::String("v1".to_string());
    remote_feed["teams"][&team] = json!({
        "status": effective_status,
        "agents_online": agents_online,
        "queue_depth": queue_depth,
        "degraded": degraded,
        "sequence": sequence,
        "last_beat_ts": state.get("last_beat_ts").cloned().unwrap_or(Value::Null),
        "web_tooling": web_tooling,
        "duality": {
            "hard_block": duality_hard_block,
            "recommended_clearance_tier": recommended_clearance_tier,
            "fractal_balance_score": duality.get("fractal_balance_score").cloned().unwrap_or(Value::Null)
        }
    });
    remote_feed["updated_at"] = Value::String(crate::now_iso());
    let feed_path = heartbeat_remote_feed_path(root);
    let _ = write_json(&feed_path, &remote_feed);

    let receipt = json!({
        "version": "v1",
        "team": team,
        "sequence": sequence,
        "status": effective_status,
        "agents_online": agents_online,
        "queue_depth": queue_depth,
        "degraded": degraded,
        "duality": duality,
        "recommended_clearance_tier": recommended_clearance_tier,
        "ts": crate::now_iso()
    });
    let _ = append_jsonl(
        &state_root(root).join("heartbeat").join("history.jsonl"),
        &receipt,
    );

    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "company_plane_heartbeat",
        "lane": "core/layer0/ops",
        "team": team,
        "op": op,
        "heartbeat": receipt,
        "artifact": {
            "state_path": state_path.display().to_string(),
            "remote_feed_path": feed_path.display().to_string(),
            "state_sha256": sha256_hex_str(&state.to_string()),
            "feed_sha256": sha256_hex_str(&remote_feed.to_string())
        },
        "claim_evidence": [
            {
                "id": "V6-COMPANY-001.4",
                "claim": "team_heartbeat_scheduler_emits_deterministic_receipts_and_remote_monitor_feed",
                "evidence": {
                    "team": team,
                    "sequence": sequence,
                    "degraded": degraded
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let parsed = parse_args(argv);
    let command = parsed
        .positional
        .first()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());
    if matches!(command.as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }

    let strict = parse_bool(parsed.flags.get("strict"), true);
    let conduit = if command != "status" {
        Some(conduit_enforcement(root, &parsed, strict, &command))
    } else {
        None
    };
    if strict
        && conduit
            .as_ref()
            .and_then(|v| v.get("ok"))
            .and_then(Value::as_bool)
            == Some(false)
    {
        return emit(
            root,
            json!({
                "ok": false,
                "strict": strict,
                "type": "company_plane_conduit_gate",
                "errors": ["conduit_bypass_rejected"],
                "conduit_enforcement": conduit
            }),
        );
    }

    let payload = match command.as_str() {
        "status" => status(root),
        "orchestrate-agency" | "orchestrate" => run_orchestrate_agency(root, &parsed, strict),
        "budget-enforce" | "budget" => run_budget_enforce(root, &parsed, strict),
        "ticket" => run_ticket(root, &parsed, strict),
        "heartbeat" => run_heartbeat(root, &parsed, strict),
        _ => json!({
            "ok": false,
            "type": "company_plane_error",
            "error": "unknown_command",
            "command": command
        }),
    };
    if command == "status" {
        print_json(&payload);
        return 0;
    }
    emit(root, attach_conduit(payload, conduit.as_ref()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn conduit_rejects_bypass() {
        let root = tempfile::tempdir().expect("tempdir");
        let parsed = crate::parse_args(&["orchestrate".to_string(), "--bypass=1".to_string()]);
        let out = conduit_enforcement(root.path(), &parsed, true, "orchestrate-agency");
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
    }

    #[test]
    fn heartbeat_tick_emits_duality_snapshot_and_clearance_hint() {
        let root = tempfile::tempdir().expect("tempdir");
        let config_dir = root.path().join("client/runtime/config");
        fs::create_dir_all(&config_dir).expect("config dir");
        fs::write(
            config_dir.join("duality_codex.txt"),
            "order/chaos harmonization\nzero point\n",
        )
        .expect("codex");
        fs::write(
            config_dir.join("duality_seed_policy.json"),
            serde_json::to_string_pretty(&json!({
                "enabled": true,
                "shadow_only": true,
                "advisory_only": true,
                "codex_path": "client/runtime/config/duality_codex.txt",
                "state": {
                    "latest_path": "local/state/autonomy/duality/latest.json",
                    "history_path": "local/state/autonomy/duality/history.jsonl"
                },
                "outputs": {"persist_shadow_receipts": true, "persist_observations": true}
            }))
            .expect("policy encode"),
        )
        .expect("policy");

        let parsed = crate::parse_args(&[
            "heartbeat".to_string(),
            "--op=tick".to_string(),
            "--team=ops".to_string(),
            "--status=healthy".to_string(),
            "--agents-online=2".to_string(),
            "--queue-depth=1".to_string(),
        ]);
        let out = run_heartbeat(root.path(), &parsed, false);
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert!(out.pointer("/heartbeat/duality").is_some());
        assert!(out
            .pointer("/heartbeat/recommended_clearance_tier")
            .is_some());
    }

    #[test]
    fn heartbeat_status_surfaces_duality_state_snapshot() {
        let root = tempfile::tempdir().expect("tempdir");
        let parsed = crate::parse_args(&[
            "heartbeat".to_string(),
            "--op=status".to_string(),
            "--team=ops".to_string(),
        ]);
        let out = run_heartbeat(root.path(), &parsed, false);
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert!(out.get("duality_state").is_some());
    }
}
