fn run_cockpit(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let contract = load_json_or(
        root,
        COCKPIT_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "premium_cockpit_contract",
            "max_blocks": 64,
            "stale_block_threshold_ms": 30_000,
            "conduit_signal_active_window_ms": 90_000,
            "auto_reclaim_stale_blocks": true,
            "auto_reclaim_max_per_run": 16,
            "allowed_status_colors": ["green", "amber", "red", "blue"]
        }),
    );
    let mut errors = Vec::<String>::new();
    if contract
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("premium_cockpit_contract_version_must_be_v1".to_string());
    }
    if contract
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "premium_cockpit_contract"
    {
        errors.push("premium_cockpit_contract_kind_invalid".to_string());
    }
    if !errors.is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "hermes_plane_cockpit",
            "errors": errors
        });
    }

    let max_blocks = parse_u64(parsed.flags.get("max-blocks"), 0).max(1).min(
        contract
            .get("max_blocks")
            .and_then(Value::as_u64)
            .unwrap_or(64),
    ) as usize;
    let contract_stale_threshold_ms = contract
        .get("stale_block_threshold_ms")
        .and_then(Value::as_u64)
        .unwrap_or(30_000);
    let stale_block_threshold_ms = parse_u64(
        parsed.flags.get("stale-threshold-ms"),
        parse_u64(parsed.flags.get("threshold-ms"), contract_stale_threshold_ms),
    )
    .max(1);
    let conduit_signal_active_window_ms = parse_u64(
        parsed.flags.get("conduit-signal-window-ms"),
        contract
            .get("conduit_signal_active_window_ms")
            .and_then(Value::as_u64)
            .unwrap_or(90_000),
    )
    .max(stale_block_threshold_ms);
    let auto_reclaim_stale_blocks = contract
        .get("auto_reclaim_stale_blocks")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let reclaim_threshold_ms = parse_u64(
        parsed.flags.get("reclaim-threshold-ms"),
        stale_block_threshold_ms,
    )
    .max(1);
    let reclaim_max_per_run = parse_u64(
        parsed.flags.get("max-reclaims"),
        contract
            .get("auto_reclaim_max_per_run")
            .and_then(Value::as_u64)
            .unwrap_or(16),
    )
    .clamp(1, 10_000) as usize;
    let protected_lanes = reclaim_protected_lanes(&contract);
    let reclaim = if auto_reclaim_stale_blocks {
        reclaim_stale_latest(
            root,
            reclaim_threshold_ms,
            reclaim_max_per_run,
            &protected_lanes,
            false,
        )
    } else {
        json!({
            "ok": true,
            "type": "hermes_plane_reclaim_stale",
            "dry_run": true,
            "stale_threshold_ms": reclaim_threshold_ms,
            "candidate_count": 0,
            "reclaimed_count": 0,
            "skipped_protected_count": protected_lanes.len()
        })
    };
    let reclaimed_count = reclaim
        .get("reclaimed_count")
        .and_then(Value::as_u64)
        .unwrap_or(0) as usize;
    let latest_rows = collect_recent_ops_latest(root, max_blocks);

    let mut blocks = Vec::<Value>::new();
    let mut stale_block_count: usize = 0;
    let mut active_block_count: usize = 0;
    let mut conduit_signals_total: usize = 0;
    let mut conduit_signals_active: usize = 0;
    for (idx, row) in latest_rows.iter().enumerate() {
        let lane = clean(
            row.get("lane").and_then(Value::as_str).unwrap_or("unknown"),
            120,
        );
        let ty = clean(
            row.get("type").and_then(Value::as_str).unwrap_or("unknown"),
            120,
        );
        let row_ts = row.get("ts").and_then(Value::as_str).unwrap_or("");
        let latest_mtime_ms = row
            .get("latest_mtime_ms")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let conduit_history_mtime_ms = row
            .get("conduit_history_mtime_ms")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let ok = row.get("ok").and_then(Value::as_bool).unwrap_or(false);
        let class = classify_tool_call(&ty).to_string();
        let payload = row.get("payload").cloned().unwrap_or(Value::Null);
        let has_conduit_history = row
            .get("has_conduit_history")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let conduit_enforced = payload.get("conduit_enforcement").is_some()
            || payload
                .get("routed_via")
                .and_then(Value::as_str)
                .map(|v| v.eq_ignore_ascii_case("conduit"))
                .unwrap_or(false)
            || has_conduit_history;
        let (duration_ms, duration_source) = duration_from_ts_or_mtime_ms(row_ts, latest_mtime_ms);
        let conduit_history_age_ms = if conduit_history_mtime_ms > 0 {
            duration_from_epoch_ms(conduit_history_mtime_ms)
        } else {
            0
        };
        let is_stale = duration_ms >= stale_block_threshold_ms;
        if is_stale {
            stale_block_count += 1;
        } else {
            active_block_count += 1;
        }
        if conduit_enforced {
            conduit_signals_total += 1;
            if duration_ms < conduit_signal_active_window_ms {
                conduit_signals_active += 1;
            }
        }
        let block = json!({
            "index": idx + 1,
            "lane": lane,
            "event_type": ty,
            "tool_call_class": class,
            "status": if ok { "ok" } else { "fail" },
            "status_color": status_color(ok, classify_tool_call(&ty)),
            "conduit_enforced": conduit_enforced,
            "duration_ms": duration_ms,
            "duration_source": duration_source,
            "is_stale": is_stale,
            "stale_block_threshold_ms": stale_block_threshold_ms,
            "latest_mtime_ms": latest_mtime_ms,
            "conduit_history_age_ms": conduit_history_age_ms,
            "ts": row.get("ts").cloned().unwrap_or(Value::Null),
            "path": row.get("latest_path").cloned().unwrap_or(Value::Null)
        });
        blocks.push(block);
    }

    let cockpit = json!({
        "version": "v1",
        "mode": "premium",
        "render": {
            "ascii_header": "PROTHEUS TOP",
            "stream_blocks": blocks,
            "total_blocks": blocks.len()
        },
        "metrics": {
            "active_block_count": active_block_count,
            "stale_block_count": stale_block_count,
            "stale_block_threshold_ms": stale_block_threshold_ms,
            "stale_reclaimed_count": reclaimed_count,
            "conduit_signal_active_window_ms": conduit_signal_active_window_ms,
            "conduit_signals_active": conduit_signals_active,
            "conduit_signals_total": conduit_signals_total,
            "conduit_channels_observed": conduit_signals_active
        },
        "generated_at": crate::now_iso()
    });
    let artifact_path = state_root(root).join("cockpit").join("latest.json");
    let _ = write_json(&artifact_path, &cockpit);

    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "hermes_plane_cockpit",
        "lane": "core/layer0/ops",
        "artifact": {
            "path": artifact_path.display().to_string(),
            "sha256": sha256_hex_str(&cockpit.to_string())
        },
        "cockpit": cockpit,
        "reclaim": reclaim,
        "claim_evidence": [
            {
                "id": "V6-HERMES-001.2",
                "claim": "premium_realtime_cockpit_stream_exposes_timings_tool_classes_and_status_colors",
                "evidence": {
                    "blocks": blocks.len(),
                    "max_blocks": max_blocks
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}

fn run_reclaim_stale(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let contract = load_json_or(
        root,
        COCKPIT_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "premium_cockpit_contract",
            "max_blocks": 64,
            "stale_block_threshold_ms": 30_000,
            "conduit_signal_active_window_ms": 90_000,
            "auto_reclaim_stale_blocks": true,
            "auto_reclaim_max_per_run": 16,
            "allowed_status_colors": ["green", "amber", "red", "blue"]
        }),
    );
    let default_threshold_ms = contract
        .get("stale_block_threshold_ms")
        .and_then(Value::as_u64)
        .unwrap_or(30_000);
    let threshold_ms = parse_u64(
        parsed.flags.get("stale-threshold-ms"),
        parse_u64(parsed.flags.get("threshold-ms"), default_threshold_ms),
    )
    .max(1);
    let max_reclaims = parse_u64(
        parsed.flags.get("max-reclaims"),
        contract
            .get("auto_reclaim_max_per_run")
            .and_then(Value::as_u64)
            .unwrap_or(16),
    )
    .clamp(1, 10_000) as usize;
    let dry_run = parse_bool(parsed.flags.get("dry-run"), false);
    let protected_lanes = reclaim_protected_lanes(&contract);
    let reclaim = reclaim_stale_latest(root, threshold_ms, max_reclaims, &protected_lanes, dry_run);
    json!({
        "ok": reclaim.get("ok").and_then(Value::as_bool).unwrap_or(false),
        "strict": strict,
        "type": "hermes_plane_reclaim_stale",
        "lane": "core/layer0/ops",
        "reclaim": reclaim,
        "claim_evidence": [
            {
                "id": "V6-HERMES-001.2",
                "claim": "premium_realtime_cockpit_stream_exposes_timings_tool_classes_and_status_colors",
                "evidence": {
                    "stale_threshold_ms": threshold_ms,
                    "max_reclaims": max_reclaims,
                    "dry_run": dry_run
                }
            }
        ]
    })
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
                "type": "hermes_plane_conduit_gate",
                "errors": ["conduit_bypass_rejected"],
                "conduit_enforcement": conduit
            }),
        );
    }

    let payload = match command.as_str() {
        "status" => status(root),
        "discover" => run_discover(root, &parsed, strict),
        "continuity" => run_continuity(root, &parsed, strict),
        "delegate" => run_delegate(root, &parsed, strict),
        "cockpit" | "top" | "dashboard" => run_cockpit(root, &parsed, strict),
        "reclaim-stale" | "reclaim-blocks" => run_reclaim_stale(root, &parsed, strict),
        _ => json!({
            "ok": false,
            "type": "hermes_plane_error",
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

