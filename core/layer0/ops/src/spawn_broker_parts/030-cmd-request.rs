fn cmd_request(root: &Path, argv: &[String]) -> i32 {
    let module = normalize_module_name(parse_flag(argv, "module"));
    let profile = resolve_profile(argv);
    let requested_cells = clamp_i64(
        parse_i64(
            parse_flag(argv, "requested_cells")
                .or_else(|| parse_flag(argv, "requested"))
                .as_deref(),
            0,
        ),
        0,
        4096,
    );
    let reason = parse_flag(argv, "reason")
        .unwrap_or_default()
        .trim()
        .chars()
        .take(160)
        .collect::<String>();
    let apply = parse_bool(parse_flag(argv, "apply").as_deref(), true);
    let policy = load_policy(root);
    let (mut state, changed) = prune_expired(load_state(root));
    if changed {
        let _ = save_state(root, &state);
    }

    let plan = router_hardware_plan(root);
    let bounds = hardware_bounds(&policy, &plan.payload);
    let limits = compute_limits(&policy, &state, &module, &bounds);
    let autopause = load_autopause(root);
    let reason_value = if reason.is_empty() {
        Value::Null
    } else {
        Value::String(reason.clone())
    };
    if autopause.active {
        let mut blocked = json!({
            "ok": true,
            "ts": now_iso(),
            "module": module,
            "profile": profile,
            "apply": apply,
            "requested_cells": requested_cells,
            "granted_cells": 0,
            "requested_tokens_est": 0,
            "reason": "budget_autopause_active",
            "blocked_by_budget": true,
            "lineage_contract": null,
            "lineage_error": null,
            "lease_expires_at": null,
            "limits": limits_to_value(&limits),
            "token_budget": {
              "enabled": true,
              "allow": false,
              "action": "escalate",
              "reason": "budget_autopause_active"
            },
            "budget_autopause": {
                "active": true,
                "source": autopause.source,
                "reason": autopause.reason,
                "until": autopause.until
            },
            "budget_guard": {
              "hard_stop": true,
              "hard_stop_reasons": ["budget_autopause_active"],
              "soft_pressure": false
            },
            "hardware_plan_ok": plan.ok,
            "hardware_plan_error": plan.error,
            "hardware_plan_transport": plan.transport,
            "hardware_bounds": bounds_to_value(&bounds)
        });
        blocked["receipt_hash"] = Value::String(receipt_hash(&blocked));
        let _ = append_jsonl(
            &events_path(root),
            &json!({
                "ts": now_iso(),
                "type": "spawn_request_blocked_budget",
                "module": module,
                "profile": profile,
                "requested_cells": requested_cells,
                "reason": "budget_autopause_active"
            }),
        );
        print_json_line(&blocked);
        return 0;
    }

    let granted_cells = std::cmp::max(0, std::cmp::min(requested_cells, limits.max_cells));
    let lease_expires_at = resolve_lease_expiry(&policy.leases, argv);

    if apply {
        if granted_cells <= 0 {
            state.allocations.remove(&module);
        } else {
            state.allocations.insert(
                module.clone(),
                Allocation {
                    cells: granted_cells,
                    ts: now_iso(),
                    reason: reason.clone(),
                    lease_expires_at: lease_expires_at.clone(),
                },
            );
        }
        state.version = 1;
        state.ts = now_iso();
        let _ = save_state(root, &state);
        let _ = append_jsonl(
            &events_path(root),
            &json!({
                "ts": now_iso(),
                "type": "spawn_request",
                "module": module,
                "profile": profile,
                "requested_cells": requested_cells,
                "granted_cells": granted_cells,
                "reason": reason_value.clone(),
                "lease_expires_at": lease_expires_at
            }),
        );
    }

    let mut out = json!({
        "ok": true,
        "ts": now_iso(),
        "module": module,
        "profile": profile,
        "apply": apply,
        "requested_cells": requested_cells,
        "granted_cells": granted_cells,
        "requested_tokens_est": 0,
        "reason": reason_value,
        "lineage_contract": null,
        "lineage_error": null,
        "lease_expires_at": lease_expires_at,
        "limits": limits_to_value(&limits),
        "token_budget": {
          "enabled": false,
          "allow": true,
          "action": "allow",
          "reason": null
        },
        "hardware_plan_ok": plan.ok,
        "hardware_plan_error": plan.error,
        "hardware_plan_transport": plan.transport,
        "hardware_bounds": bounds_to_value(&bounds)
    });
    out["receipt_hash"] = Value::String(receipt_hash(&out));
    print_json_line(&out);
    0
}

fn cmd_release(root: &Path, argv: &[String]) -> i32 {
    let module = normalize_module_name(parse_flag(argv, "module"));
    let reason = parse_flag(argv, "reason")
        .unwrap_or_else(|| "release".to_string())
        .trim()
        .chars()
        .take(160)
        .collect::<String>();

    let mut state = load_state(root);
    let prev = state.allocations.get(&module).map(|r| r.cells).unwrap_or(0);
    state.allocations.remove(&module);
    state.version = 1;
    state.ts = now_iso();
    let _ = save_state(root, &state);
    let _ = append_jsonl(
        &events_path(root),
        &json!({
            "ts": now_iso(),
            "type": "spawn_release",
            "module": module,
            "previous_cells": prev,
            "reason": reason
        }),
    );

    let mut out = json!({
        "ok": true,
        "ts": now_iso(),
        "module": module,
        "released_cells": prev,
        "reason": reason
    });
    out["receipt_hash"] = Value::String(receipt_hash(&out));
    print_json_line(&out);
    0
}

fn cli_error(argv: &[String], err: &str, exit_code: i32) -> Value {
    let mut out = json!({
        "ok": false,
        "type": "spawn_broker_cli_error",
        "ts": now_iso(),
        "argv": argv,
        "error": err,
        "exit_code": exit_code
    });
    out["receipt_hash"] = Value::String(receipt_hash(&out));
    out
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let cmd = argv
        .iter()
        .find(|arg| !arg.trim().starts_with("--"))
        .map(|s| s.trim().to_ascii_lowercase())
        .unwrap_or_default();
    if cmd.is_empty() || matches!(cmd.as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }
    match cmd.as_str() {
        "status" => cmd_status(root, argv),
        "request" => cmd_request(root, argv),
        "release" => cmd_release(root, argv),
        _ => {
            usage();
            print_json_line(&cli_error(argv, "unknown_command", 2));
            2
        }
    }
}
