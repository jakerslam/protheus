fn load_state(root: &Path) -> BrokerState {
    let path = state_path(root);
    let Some(raw) = read_json(&path) else {
        return default_state();
    };
    let Some(obj) = raw.as_object() else {
        return default_state();
    };
    let mut state = default_state();
    state.version = obj.get("version").and_then(Value::as_i64).unwrap_or(1);
    state.ts = obj
        .get("ts")
        .and_then(Value::as_str)
        .unwrap_or(&state.ts)
        .trim()
        .to_string();
    if let Some(allocs) = obj.get("allocations").and_then(Value::as_object) {
        let mut out = BTreeMap::new();
        for (name, row) in allocs {
            if let Some(parsed) = parse_allocation(row) {
                out.insert(name.trim().to_ascii_lowercase(), parsed);
            }
        }
        state.allocations = out;
    }
    state
}

fn state_to_value(state: &BrokerState) -> Value {
    let mut allocs = serde_json::Map::new();
    for (name, row) in &state.allocations {
        allocs.insert(
            name.clone(),
            json!({
                "module": name,
                "cells": row.cells,
                "ts": row.ts,
                "reason": row.reason,
                "lease_expires_at": row.lease_expires_at
            }),
        );
    }
    json!({
        "version": state.version,
        "ts": state.ts,
        "allocations": Value::Object(allocs)
    })
}

fn save_state(root: &Path, state: &BrokerState) -> Result<(), String> {
    write_json_atomic(&state_path(root), &state_to_value(state))
}

fn parse_iso_ms(raw: &str) -> Option<i64> {
    let dt = DateTime::parse_from_rfc3339(raw).ok()?;
    Some(dt.timestamp_millis())
}

fn is_expired(iso: &Option<String>) -> bool {
    let Some(v) = iso else {
        return false;
    };
    let Some(ms) = parse_iso_ms(v.as_str()) else {
        return false;
    };
    ms <= now_ms()
}

fn prune_expired(mut state: BrokerState) -> (BrokerState, bool) {
    let before = state.allocations.len();
    state
        .allocations
        .retain(|_, row| !is_expired(&row.lease_expires_at));
    let changed = state.allocations.len() != before;
    if changed {
        state.ts = now_iso();
    }
    (state, changed)
}

fn parse_json_line_fallback(stdout: &str) -> Value {
    if let Ok(v) = serde_json::from_str::<Value>(stdout.trim()) {
        return v;
    }
    for line in stdout.lines().rev() {
        let s = line.trim();
        if !s.starts_with('{') {
            continue;
        }
        if let Ok(v) = serde_json::from_str::<Value>(s) {
            return v;
        }
    }
    Value::Null
}

fn router_hardware_plan(root: &Path) -> RouterPlan {
    let script = router_script_path(root);
    let cwd = root_client_runtime(root);
    let node_bin = std::env::var("PROTHEUS_NODE_BINARY")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "node".to_string());
    let run = Command::new(node_bin)
        .arg(script.to_string_lossy().to_string())
        .arg("hardware-plan")
        .current_dir(cwd)
        .output();
    let Ok(out) = run else {
        return RouterPlan {
            ok: false,
            payload: Value::Null,
            error: Some("spawn_router_exec_failed".to_string()),
            transport: Some("spawn_sync".to_string()),
        };
    };
    let status = out.status.code().unwrap_or(1);
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    if status != 0 {
        return RouterPlan {
            ok: false,
            payload: Value::Null,
            error: Some(format!("hardware_plan_failed:{}", stderr.trim())),
            transport: Some("spawn_sync".to_string()),
        };
    }
    let payload = parse_json_line_fallback(&stdout);
    RouterPlan {
        ok: true,
        payload,
        error: None,
        transport: Some("spawn_sync".to_string()),
    }
}

fn hardware_bounds(policy: &SpawnPolicy, payload: &Value) -> HardwareBounds {
    let profile = payload.get("profile").and_then(Value::as_object);
    let hw_class = profile
        .and_then(|p| p.get("hardware_class"))
        .and_then(Value::as_str)
        .map(|s| s.trim().to_ascii_lowercase())
        .filter(|s| !s.is_empty());
    let cpu_threads = profile
        .and_then(|p| p.get("cpu_threads"))
        .and_then(Value::as_f64);
    let ram_gb = profile
        .and_then(|p| p.get("ram_gb"))
        .and_then(Value::as_f64);

    let class_cap = hw_class
        .as_ref()
        .and_then(|c| policy.pool.max_cells_by_hardware.get(c))
        .copied()
        .unwrap_or(policy.pool.max_cells);

    let cap_by_cpu = if let Some(cpu) = cpu_threads {
        let free = (cpu - policy.pool.reserve_cpu_threads).max(0.0);
        ((free / policy.pool.estimated_cpu_threads_per_cell).floor() as i64).max(0)
    } else {
        policy.pool.max_cells
    };
    let cap_by_ram = if let Some(ram) = ram_gb {
        let free = (ram - policy.pool.reserve_ram_gb).max(0.0);
        ((free / policy.pool.estimated_ram_gb_per_cell).floor() as i64).max(0)
    } else {
        policy.pool.max_cells
    };

    let global_max = std::cmp::max(
        policy.pool.min_cells,
        std::cmp::min(
            policy.pool.max_cells,
            std::cmp::min(class_cap, std::cmp::min(cap_by_cpu, cap_by_ram)),
        ),
    );

    HardwareBounds {
        hardware_class: hw_class,
        cpu_threads,
        ram_gb,
        cap_by_class: class_cap,
        cap_by_cpu,
        cap_by_ram,
        global_max_cells: global_max,
    }
}

fn normalize_module_name(raw: Option<String>) -> String {
    let out = raw.unwrap_or_else(|| "reflex".to_string());
    let n = out.trim().to_ascii_lowercase();
    if n.is_empty() {
        "reflex".to_string()
    } else {
        n
    }
}

fn module_quota_max(policy: &SpawnPolicy, module: &str, global_max: i64) -> i64 {
    let raw = policy
        .quotas
        .modules
        .get(module)
        .copied()
        .unwrap_or(policy.quotas.default_max_cells);
    clamp_i64(raw, 0, global_max)
}

fn cells_for(state: &BrokerState, module: &str) -> i64 {
    state
        .allocations
        .get(module)
        .map(|r| r.cells)
        .unwrap_or(0)
        .max(0)
}

fn sum_allocations(state: &BrokerState, skip_module: &str) -> i64 {
    state
        .allocations
        .iter()
        .filter(|(name, _)| name.as_str() != skip_module)
        .map(|(_, row)| row.cells.max(0))
        .sum::<i64>()
}

fn compute_limits(
    policy: &SpawnPolicy,
    state: &BrokerState,
    module: &str,
    bounds: &HardwareBounds,
) -> Limits {
    let global_max = bounds.global_max_cells.max(0);
    let current = cells_for(state, module);
    let allocated_other = sum_allocations(state, module);
    let allocated_total = allocated_other + current;
    let free_with_current = (global_max - allocated_other).max(0);
    let free_global = (global_max - allocated_total).max(0);
    let module_quota = module_quota_max(policy, module, global_max);
    let max_cells = std::cmp::max(0, std::cmp::min(module_quota, free_with_current));
    Limits {
        module: module.to_string(),
        global_max_cells: global_max,
        module_quota_max_cells: module_quota,
        module_current_cells: current,
        allocated_other_cells: allocated_other,
        allocated_total_cells: allocated_total,
        free_global_cells: free_global,
        max_cells,
    }
}

fn summarize_allocations(state: &BrokerState) -> Value {
    let mut map = serde_json::Map::new();
    for (name, row) in &state.allocations {
        map.insert(
            name.clone(),
            json!({
                "cells": row.cells.max(0),
                "ts": row.ts,
                "reason": row.reason,
                "lease_expires_at": row.lease_expires_at
            }),
        );
    }
    Value::Object(map)
}

fn resolve_profile(argv: &[String]) -> String {
    let raw = parse_flag(argv, "profile")
        .or_else(|| parse_flag(argv, "spawn_profile"))
        .unwrap_or_else(|| "standard".to_string());
    let mut out = String::with_capacity(raw.len());
    let mut prev_us = false;
    for ch in raw.trim().to_ascii_lowercase().chars() {
        let keep = ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.' | ':' | '/' | '-');
        if keep {
            out.push(ch);
            prev_us = false;
        } else if !prev_us {
            out.push('_');
            prev_us = true;
        }
    }
    let cleaned = out.trim_matches('_').to_string();
    if cleaned.is_empty() {
        "standard".to_string()
    } else {
        cleaned
    }
}

fn resolve_lease_expiry(policy: &LeasePolicy, argv: &[String]) -> Option<String> {
    if !policy.enabled {
        return None;
    }
    let ttl = parse_i64(
        parse_flag(argv, "lease_sec")
            .or_else(|| parse_flag(argv, "lease"))
            .as_deref(),
        policy.default_ttl_sec,
    );
    let ttl = clamp_i64(ttl, 5, policy.max_ttl_sec);
    let ms = now_ms().saturating_add(ttl.saturating_mul(1000));
    DateTime::<Utc>::from_timestamp_millis(ms).map(|dt| dt.to_rfc3339())
}

fn load_autopause(root: &Path) -> AutopauseState {
    let path = autopause_path(root);
    let raw = read_json(&path);
    let mut active = false;
    let mut source = None;
    let mut reason = None;
    let mut until = None;
    if let Some(obj) = raw.and_then(|v| v.as_object().cloned()) {
        source = obj
            .get("source")
            .and_then(Value::as_str)
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        reason = obj
            .get("reason")
            .and_then(Value::as_str)
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        until = obj
            .get("until")
            .and_then(Value::as_str)
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let until_ms = obj.get("until_ms").and_then(Value::as_i64).unwrap_or(0);
        active = obj.get("active").and_then(Value::as_bool).unwrap_or(false) && until_ms > now_ms();
    }
    AutopauseState {
        active,
        source,
        reason,
        until,
    }
}

fn limits_to_value(limits: &Limits) -> Value {
    json!({
        "module": limits.module,
        "global_max_cells": limits.global_max_cells,
        "module_quota_max_cells": limits.module_quota_max_cells,
        "module_current_cells": limits.module_current_cells,
        "allocated_other_cells": limits.allocated_other_cells,
        "allocated_total_cells": limits.allocated_total_cells,
        "free_global_cells": limits.free_global_cells,
        "max_cells": limits.max_cells
    })
}

fn bounds_to_value(bounds: &HardwareBounds) -> Value {
    json!({
        "hardware_class": bounds.hardware_class,
        "cpu_threads": bounds.cpu_threads,
        "ram_gb": bounds.ram_gb,
        "cap_by_class": bounds.cap_by_class,
        "cap_by_cpu": bounds.cap_by_cpu,
        "cap_by_ram": bounds.cap_by_ram,
        "global_max_cells": bounds.global_max_cells
    })
}

fn policy_to_value(policy: &SpawnPolicy) -> Value {
    let mut modules = serde_json::Map::new();
    for (name, max_cells) in &policy.quotas.modules {
        modules.insert(name.clone(), json!({ "max_cells": max_cells }));
    }
    let mut by_hw = serde_json::Map::new();
    for (name, cap) in &policy.pool.max_cells_by_hardware {
        by_hw.insert(name.clone(), json!(cap));
    }
    json!({
        "version": policy.version,
        "pool": {
            "min_cells": policy.pool.min_cells,
            "max_cells": policy.pool.max_cells,
            "reserve_cpu_threads": policy.pool.reserve_cpu_threads,
            "reserve_ram_gb": policy.pool.reserve_ram_gb,
            "estimated_cpu_threads_per_cell": policy.pool.estimated_cpu_threads_per_cell,
            "estimated_ram_gb_per_cell": policy.pool.estimated_ram_gb_per_cell,
            "max_cells_by_hardware": by_hw
        },
        "quotas": {
            "default_max_cells": policy.quotas.default_max_cells,
            "modules": modules
        },
        "leases": {
            "enabled": policy.leases.enabled,
            "default_ttl_sec": policy.leases.default_ttl_sec,
            "max_ttl_sec": policy.leases.max_ttl_sec
        }
    })
}

fn cmd_status(root: &Path, argv: &[String]) -> i32 {
    let module = normalize_module_name(parse_flag(argv, "module"));
    let profile = resolve_profile(argv);
    let policy = load_policy(root);
    let (mut state, changed) = prune_expired(load_state(root));
    if changed {
        let _ = save_state(root, &state);
    }
    if state.ts.is_empty() {
        state.ts = now_iso();
    }

    let plan = router_hardware_plan(root);
    let bounds = hardware_bounds(&policy, &plan.payload);
    let limits = compute_limits(&policy, &state, &module, &bounds);
    let autopause = load_autopause(root);

    let mut out = json!({
        "ok": true,
        "ts": now_iso(),
        "module": module,
        "profile": profile,
        "policy": policy_to_value(&policy),
        "state": {
            "version": state.version,
            "ts": state.ts,
            "allocations": summarize_allocations(&state)
        },
        "limits": limits_to_value(&limits),
        "token_budget": {
            "enabled": false,
            "allow": true,
            "action": "allow",
            "reason": null
        },
        "budget_autopause": {
            "active": autopause.active,
            "source": autopause.source,
            "reason": autopause.reason,
            "until": autopause.until
        },
        "budget_guard": {
            "hard_stop": false,
            "hard_stop_reasons": [],
            "soft_pressure": false
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

