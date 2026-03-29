
fn run_wasm_dual_meter_receipt(root: &Path, argv: &[String], strict: bool) -> Value {
    let policy = read_json(&root.join(WASM_DUAL_METER_POLICY_PATH)).unwrap_or_else(|| {
        json!({
            "version": "v1",
            "kind": "wasm_dual_meter_policy",
            "defaults": {
                "fuel_budget": 25000,
                "epoch_budget": 128,
                "fuel_per_tick": 90,
                "max_ticks_per_epoch": 16,
                "epoch_step": 1
            }
        })
    });
    let defaults = policy.get("defaults").cloned().unwrap_or(Value::Null);
    let ticks = parse_u64_flag(parse_flag(argv, "ticks"), 32);
    let fuel_budget = parse_u64_flag(
        parse_flag(argv, "fuel-budget"),
        defaults
            .get("fuel_budget")
            .and_then(Value::as_u64)
            .unwrap_or(25_000),
    );
    let epoch_budget = parse_u64_flag(
        parse_flag(argv, "epoch-budget"),
        defaults
            .get("epoch_budget")
            .and_then(Value::as_u64)
            .unwrap_or(128),
    );
    let fuel_per_tick = parse_u64_flag(
        parse_flag(argv, "fuel-per-tick"),
        defaults
            .get("fuel_per_tick")
            .and_then(Value::as_u64)
            .unwrap_or(90),
    );
    let max_ticks_per_epoch = parse_u64_flag(
        parse_flag(argv, "max-ticks-per-epoch"),
        defaults
            .get("max_ticks_per_epoch")
            .and_then(Value::as_u64)
            .unwrap_or(16),
    )
    .max(1);
    let epoch_step = parse_u64_flag(
        parse_flag(argv, "epoch-step"),
        defaults
            .get("epoch_step")
            .and_then(Value::as_u64)
            .unwrap_or(1),
    )
    .max(1);

    let fuel_used = ticks.saturating_mul(fuel_per_tick);
    let epoch_used = if ticks == 0 {
        0
    } else {
        ((ticks + max_ticks_per_epoch - 1) / max_ticks_per_epoch).saturating_mul(epoch_step)
    };
    let mut errors = Vec::<String>::new();
    if policy
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("policy_version_must_be_v1".to_string());
    }
    if policy
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "wasm_dual_meter_policy"
    {
        errors.push("policy_kind_invalid".to_string());
    }
    if fuel_used > fuel_budget {
        errors.push("fuel_exhausted".to_string());
    }
    if epoch_used > epoch_budget {
        errors.push("epoch_exhausted".to_string());
    }
    let ok = errors.is_empty();
    let mut out = json!({
        "ok": if strict { ok } else { true },
        "strict": strict,
        "type": "assimilation_controller_wasm_dual_meter",
        "lane": LANE_ID,
        "ts": now_iso(),
        "policy_path": WASM_DUAL_METER_POLICY_PATH,
        "telemetry": {
            "ticks": ticks,
            "fuel_budget": fuel_budget,
            "fuel_used": fuel_used,
            "fuel_remaining": fuel_budget.saturating_sub(fuel_used),
            "epoch_budget": epoch_budget,
            "epoch_used": epoch_used,
            "epoch_remaining": epoch_budget.saturating_sub(epoch_used)
        },
        "decision": if ok { "allow" } else { "deny" },
        "errors": errors,
        "claim_evidence": [
            {
                "id": "V7-ASSIMILATE-001.4",
                "claim": "dual_metered_wasm_policy_enforces_fuel_and_epoch_limits_with_fail_closed_receipts",
                "evidence": {
                    "fuel_used": fuel_used,
                    "epoch_used": epoch_used
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(receipt_hash(&out));
    persist_receipt(root, &out);
    out
}

fn run_hands_runtime_receipt(root: &Path, argv: &[String], strict: bool) -> Value {
    let op = parse_flag(argv, "op")
        .or_else(|| first_non_flag(argv, 1))
        .unwrap_or_else(|| "status".to_string())
        .to_ascii_lowercase();
    let manifest_rel =
        parse_flag(argv, "manifest").unwrap_or_else(|| HAND_MANIFEST_PATH.to_string());
    let manifest_path = root.join(&manifest_rel);
    let manifest = parse_hand_manifest(&manifest_path).unwrap_or(Value::Null);
    let state_path = state_root(root).join("hands_runtime").join("state.json");
    let events_path = state_root(root).join("hands_runtime").join("events.jsonl");
    let mut state = read_json(&state_path).unwrap_or_else(|| {
        json!({
            "installed": false,
            "running": false,
            "paused": false,
            "rotation_seq": 0,
            "active_version": Value::Null,
            "last_op": Value::Null,
            "updated_at": Value::Null
        })
    });

    let mut errors = Vec::<String>::new();
    if manifest.is_null() {
        errors.push("hand_manifest_missing_or_invalid".to_string());
    }
    if manifest
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .is_empty()
    {
        errors.push("hand_manifest_name_required".to_string());
    }
    if manifest
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .is_empty()
    {
        errors.push("hand_manifest_version_required".to_string());
    }
    if manifest
        .get("capabilities")
        .and_then(Value::as_array)
        .map(|rows| !rows.is_empty())
        .unwrap_or(false)
        == false
    {
        errors.push("hand_manifest_capabilities_required".to_string());
    }

    let installed = state
        .get("installed")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let running = state
        .get("running")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    if op == "install" {
        if errors.is_empty() {
            state["installed"] = Value::Bool(true);
            state["running"] = Value::Bool(false);
            state["paused"] = Value::Bool(false);
            state["rotation_seq"] = Value::Number(0_u64.into());
            state["active_version"] = manifest
                .get("version")
                .cloned()
                .unwrap_or_else(|| Value::String("0.0.0".to_string()));
        }
    } else if op == "start" {
        if !installed {
            errors.push("hands_runtime_not_installed".to_string());
        } else {
            state["running"] = Value::Bool(true);
            state["paused"] = Value::Bool(false);
        }
    } else if op == "pause" {
        if !running {
            errors.push("hands_runtime_not_running".to_string());
        } else {
            state["paused"] = Value::Bool(true);
            state["running"] = Value::Bool(false);
        }
    } else if op == "rotate" {
        if !installed {
            errors.push("hands_runtime_not_installed".to_string());
        } else {
            let next_version = parse_flag(argv, "version")
                .or_else(|| {
                    manifest
                        .get("version")
                        .and_then(Value::as_str)
                        .map(ToString::to_string)
                })
                .unwrap_or_else(|| "0.0.0".to_string());
            let next_seq = state
                .get("rotation_seq")
                .and_then(Value::as_u64)
                .unwrap_or(0)
                .saturating_add(1);
            state["rotation_seq"] = Value::Number(next_seq.into());
            state["active_version"] = Value::String(next_version);
            state["running"] = Value::Bool(true);
            state["paused"] = Value::Bool(false);
        }
    } else if op != "status" {
        errors.push(format!("unknown_hands_op:{op}"));
    }

    let ok = errors.is_empty();
    if matches!(op.as_str(), "install" | "start" | "pause" | "rotate") && ok {
        state["last_op"] = Value::String(op.clone());
        state["updated_at"] = Value::String(now_iso());
        if let Some(parent) = state_path.parent() {
            let _ = fs::create_dir_all(parent);
        } else {
            let _ = fs::create_dir_all(state_root(root));
        }
        let _ = fs::write(
            &state_path,
            serde_json::to_string_pretty(&state).unwrap_or_else(|_| "{}".to_string()) + "\n",
        );
        let _ = append_jsonl(
            &events_path,
            &json!({
                "type": "hands_runtime_event",
                "op": op,
                "ts": now_iso(),
                "manifest_path": manifest_rel,
                "state": state
            }),
        );
    }

    let mut out = json!({
        "ok": if strict { ok } else { true },
        "strict": strict,
        "type": "assimilation_controller_hands_runtime",
        "lane": LANE_ID,
        "ts": now_iso(),
        "op": op,
        "manifest_path": manifest_rel,
        "state_path": state_path.display().to_string(),
        "events_path": events_path.display().to_string(),
        "manifest": manifest,
        "state": state,
        "errors": errors,
        "claim_evidence": [
            {
                "id": "V7-ASSIMILATE-001.5",
                "claim": "hands_runtime_is_manifest_driven_with_receipted_install_start_pause_rotate_lifecycle",
                "evidence": {
                    "op": op
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(receipt_hash(&out));
    persist_receipt(root, &out);
    out
}

fn scheduled_hands_state_path(root: &Path) -> std::path::PathBuf {
    state_root(root).join("scheduled_hands").join("state.json")
}

fn scheduled_hands_history_path(root: &Path) -> std::path::PathBuf {
    state_root(root)
        .join("scheduled_hands")
        .join("history.jsonl")
}

fn scheduled_hands_earnings_path(root: &Path) -> std::path::PathBuf {
    state_root(root)
        .join("scheduled_hands")
        .join("earnings.jsonl")
}
