
fn runtime_web_receipts_path(root: &Path) -> PathBuf {
    runtime_state_path(root, WEB_RECEIPTS_REL)
}

pub(crate) fn runtime_web_tools_state_path(root: &Path) -> PathBuf {
    runtime_web_tools_metadata_path(root)
}

pub(crate) fn recent_tool_attempt_replay_guard(
    root: &Path,
    attempt_signature: &str,
    window: usize,
    block_threshold: usize,
    cooldown_base_seconds: u64,
    cooldown_step_seconds: u64,
    cooldown_max_seconds: u64,
) -> Value {
    let signature = clean_text(attempt_signature, 120);
    let window_size = window.clamp(1, 200);
    let threshold = block_threshold.clamp(2, window_size);
    let cooldown_base = cooldown_base_seconds.clamp(0, 3600);
    let cooldown_step = cooldown_step_seconds.clamp(0, 3600);
    let cooldown_max = cooldown_max_seconds.clamp(cooldown_base.max(1), 7200);
    if signature.is_empty() {
        return json!({
            "blocked": false,
            "reason": "no_attempt_signature",
            "attempt_signature": "",
            "window": window_size,
            "block_threshold": threshold,
            "matched_receipts": 0,
            "denied_receipts": 0,
            "scanned_receipts": 0,
            "retry_after_seconds": 0,
            "retry_lane": "none",
            "cooldown_policy": {
                "base_seconds": cooldown_base,
                "step_seconds": cooldown_step,
                "max_seconds": cooldown_max
            }
        });
    }
    let mut matched_receipts = 0_usize;
    let mut denied_receipts = 0_usize;
    let mut scanned_receipts = 0_usize;
    let receipts_raw = std::fs::read_to_string(runtime_web_receipts_path(root)).unwrap_or_default();
    for line in receipts_raw.lines().rev().take(window_size) {
        scanned_receipts += 1;
        let Ok(row) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if clean_text(
            row.get("attempt_signature")
                .and_then(Value::as_str)
                .unwrap_or(""),
            120,
        ) != signature
        {
            continue;
        }
        matched_receipts += 1;
        let denied = clean_text(row.get("decision").and_then(Value::as_str).unwrap_or(""), 30)
            .eq_ignore_ascii_case("deny")
            || row.get("error").is_some();
        if denied {
            denied_receipts += 1;
        }
    }
    let min_denied = if threshold > 1 { threshold - 1 } else { 1 };
    let blocked = matched_receipts >= threshold && denied_receipts >= min_denied;
    let replay_excess = matched_receipts.saturating_sub(threshold) as u64;
    let retry_after_seconds = if blocked {
        cooldown_base
            .saturating_add(replay_excess.saturating_mul(cooldown_step))
            .min(cooldown_max)
    } else {
        0
    };
    json!({
        "blocked": blocked,
        "reason": if blocked { "repeated_attempt_signature" } else { "none" },
        "attempt_signature": signature,
        "window": window_size,
        "block_threshold": threshold,
        "matched_receipts": matched_receipts,
        "denied_receipts": denied_receipts,
        "scanned_receipts": scanned_receipts,
        "retry_after_seconds": retry_after_seconds,
        "retry_lane": if blocked { "change_query_or_provider" } else { "none" },
        "cooldown_policy": {
            "base_seconds": cooldown_base,
            "step_seconds": cooldown_step,
            "max_seconds": cooldown_max
        }
    })
}

pub(crate) fn runtime_web_replay_policy(
    policy: &Value,
    request: &Value,
    tool_surface_status: &str,
    tool_surface_ready: bool,
) -> Value {
    let status = clean_text(tool_surface_status, 40).to_ascii_lowercase();
    let (default_window, default_threshold, default_base, default_step, default_max, lane) =
        if status == "ready" && tool_surface_ready {
            (24_u64, 4_u64, 20_u64, 10_u64, 120_u64, "normal")
        } else if status == "degraded" {
            (24_u64, 3_u64, 30_u64, 15_u64, 180_u64, "degraded")
        } else {
            (24_u64, 2_u64, 45_u64, 20_u64, 240_u64, "unavailable")
        };
    let policy_enabled = policy
        .pointer("/web_conduit/replay_policy/enabled")
        .or_else(|| policy.pointer("/web_conduit/replay_guard/enabled"))
        .and_then(Value::as_bool);
    let request_enabled = request
        .pointer("/replay_policy/enabled")
        .or_else(|| request.pointer("/replayPolicy/enabled"))
        .or_else(|| request.pointer("/replay_guard/enabled"))
        .or_else(|| request.pointer("/replayGuard/enabled"))
        .and_then(Value::as_bool);
    let policy_window = policy
        .pointer("/web_conduit/replay_policy/window")
        .or_else(|| policy.pointer("/web_conduit/replay_guard/window"))
        .and_then(Value::as_u64);
    let policy_threshold = policy
        .pointer("/web_conduit/replay_policy/block_threshold")
        .or_else(|| policy.pointer("/web_conduit/replay_guard/block_threshold"))
        .and_then(Value::as_u64);
    let policy_cooldown_base = policy
        .pointer("/web_conduit/replay_policy/cooldown_base_seconds")
        .or_else(|| policy.pointer("/web_conduit/replay_guard/cooldown_base_seconds"))
        .and_then(Value::as_u64);
    let policy_cooldown_step = policy
        .pointer("/web_conduit/replay_policy/cooldown_step_seconds")
        .or_else(|| policy.pointer("/web_conduit/replay_guard/cooldown_step_seconds"))
        .and_then(Value::as_u64);
    let policy_cooldown_max = policy
        .pointer("/web_conduit/replay_policy/cooldown_max_seconds")
        .or_else(|| policy.pointer("/web_conduit/replay_guard/cooldown_max_seconds"))
        .and_then(Value::as_u64);
    let request_window = request
        .pointer("/replay_policy/window")
        .or_else(|| request.pointer("/replayPolicy/window"))
        .or_else(|| request.pointer("/replay_guard/window"))
        .or_else(|| request.pointer("/replayGuard/window"))
        .and_then(Value::as_u64);
    let request_threshold = request
        .pointer("/replay_policy/block_threshold")
        .or_else(|| request.pointer("/replayPolicy/blockThreshold"))
        .or_else(|| request.pointer("/replay_guard/block_threshold"))
        .or_else(|| request.pointer("/replayGuard/blockThreshold"))
        .and_then(Value::as_u64);
    let request_cooldown_base = request
        .pointer("/replay_policy/cooldown_base_seconds")
        .or_else(|| request.pointer("/replayPolicy/cooldownBaseSeconds"))
        .or_else(|| request.pointer("/replay_guard/cooldown_base_seconds"))
        .or_else(|| request.pointer("/replayGuard/cooldownBaseSeconds"))
        .and_then(Value::as_u64);
    let request_cooldown_step = request
        .pointer("/replay_policy/cooldown_step_seconds")
        .or_else(|| request.pointer("/replayPolicy/cooldownStepSeconds"))
        .or_else(|| request.pointer("/replay_guard/cooldown_step_seconds"))
        .or_else(|| request.pointer("/replayGuard/cooldownStepSeconds"))
        .and_then(Value::as_u64);
    let request_cooldown_max = request
        .pointer("/replay_policy/cooldown_max_seconds")
        .or_else(|| request.pointer("/replayPolicy/cooldownMaxSeconds"))
        .or_else(|| request.pointer("/replay_guard/cooldown_max_seconds"))
        .or_else(|| request.pointer("/replayGuard/cooldownMaxSeconds"))
        .and_then(Value::as_u64);
    let source = if request_enabled.is_some()
        || request_window.is_some()
        || request_threshold.is_some()
        || request_cooldown_base.is_some()
        || request_cooldown_step.is_some()
        || request_cooldown_max.is_some()
    {
        "request_override"
    } else if policy_enabled.is_some()
        || policy_window.is_some()
        || policy_threshold.is_some()
        || policy_cooldown_base.is_some()
        || policy_cooldown_step.is_some()
        || policy_cooldown_max.is_some()
    {
        "policy_override"
    } else if status == "degraded" {
        "default"
    } else {
        "default"
    };
    let enabled = request_enabled.or(policy_enabled).unwrap_or(true);
    let resolved_window = request_window
        .or(policy_window)
        .unwrap_or(default_window)
        .clamp(1, 200);
    let resolved_threshold = request_threshold
        .or(policy_threshold)
        .unwrap_or(default_threshold)
        .clamp(2, resolved_window.max(2));
    let resolved_cooldown_base = request_cooldown_base
        .or(policy_cooldown_base)
        .unwrap_or(default_base)
        .clamp(0, 3600);
    let resolved_cooldown_step = request_cooldown_step
        .or(policy_cooldown_step)
        .unwrap_or(default_step)
        .clamp(0, 3600);
    let resolved_cooldown_max = request_cooldown_max
        .or(policy_cooldown_max)
        .unwrap_or(default_max)
        .clamp(resolved_cooldown_base.max(1), 7200);
    json!({
        "enabled": enabled,
        "window": resolved_window,
        "block_threshold": resolved_threshold,
        "lane": lane,
        "source": source,
        "cooldown_base_seconds": resolved_cooldown_base,
        "cooldown_step_seconds": resolved_cooldown_step,
        "cooldown_max_seconds": resolved_cooldown_max
    })
}

pub(crate) fn runtime_web_replay_bypass(
    policy: &Value,
    request: &Value,
    human_approved: bool,
) -> Value {
    let allow_force_bypass = policy
        .pointer("/web_conduit/replay_policy/allow_force_bypass")
        .or_else(|| policy.pointer("/web_conduit/replay_policy/allowForceBypass"))
        .or_else(|| policy.pointer("/web_conduit/replay_guard/allow_force_bypass"))
        .or_else(|| policy.pointer("/web_conduit/replay_guard/allowForceBypass"))
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let allow_human_approved_bypass = policy
        .pointer("/web_conduit/replay_policy/allow_human_approved_bypass")
        .or_else(|| policy.pointer("/web_conduit/replay_policy/allowHumanApprovedBypass"))
        .or_else(|| policy.pointer("/web_conduit/replay_guard/allow_human_approved_bypass"))
        .or_else(|| policy.pointer("/web_conduit/replay_guard/allowHumanApprovedBypass"))
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let request_force_allow = runtime_web_request_flag(
        request,
        &[
            "/force_allow",
            "/forceAllow",
            "/bypass",
            "/replay_policy/force_allow",
            "/replayPolicy/forceAllow",
            "/replay_guard/force_allow",
            "/replayGuard/forceAllow",
            "/replay_policy/bypass",
            "/replayPolicy/bypass",
            "/replay_guard/bypass",
            "/replayGuard/bypass",
        ],
    );
    let bypassed_by_force_allow = request_force_allow && allow_force_bypass;
    let bypassed_by_human_approval = human_approved && allow_human_approved_bypass;
    let bypassed = bypassed_by_force_allow || bypassed_by_human_approval;
    let reason = if bypassed_by_force_allow {
        "force_allow"
    } else if bypassed_by_human_approval {
        "human_approved"
    } else if request_force_allow && !allow_force_bypass {
        "force_allow_denied_by_policy"
    } else if human_approved && !allow_human_approved_bypass {
        "human_approval_bypass_denied_by_policy"
    } else {
        "none"
    };
    json!({
        "bypassed": bypassed,
        "reason": reason,
        "source": if bypassed_by_force_allow { "request" } else if bypassed_by_human_approval { "human_approval" } else { "none" },
        "allow_force_bypass": allow_force_bypass,
        "allow_human_approved_bypass": allow_human_approved_bypass,
        "request_force_allow": request_force_allow,
        "human_approved": human_approved
    })
}

pub(crate) fn runtime_web_replay_guard_passthrough(
    reason: &str,
    attempt_signature: &str,
    window: usize,
    block_threshold: usize,
    cooldown_base_seconds: u64,
    cooldown_step_seconds: u64,
    cooldown_max_seconds: u64,
    replay_bypass: &Value,
) -> Value {
    json!({
        "blocked": false,
        "reason": reason,
        "bypass": replay_bypass.clone(),
        "attempt_signature": attempt_signature,
        "window": window,
        "block_threshold": block_threshold,
        "matched_receipts": 0,
        "denied_receipts": 0,
        "scanned_receipts": 0,
        "retry_after_seconds": 0,
        "retry_lane": "none",
        "cooldown_policy": {
            "base_seconds": cooldown_base_seconds,
            "step_seconds": cooldown_step_seconds,
            "max_seconds": cooldown_max_seconds
        }
    })
}

pub(crate) fn runtime_web_truthy_flag(value: &Value) -> bool {
    value
        .as_bool()
        .or_else(|| value.as_i64().map(|n| n != 0))
        .or_else(|| {
            value.as_str().map(|raw| {
                matches!(
                    clean_text(raw, 12).to_ascii_lowercase().as_str(),
                    "1" | "true" | "yes" | "y" | "on"
                )
            })
        })
        .unwrap_or(false)
}

pub(crate) fn runtime_web_request_flag(request: &Value, pointers: &[&str]) -> bool {
    pointers
        .iter()
        .filter_map(|pointer| request.pointer(pointer))
        .any(runtime_web_truthy_flag)
}

pub(crate) fn runtime_web_process_summary(
    tool: &str,
    phase: &str,
    tool_execution_attempted: bool,
    tool_execution_gate: &Value,
    attempt_replay_guard: &Value,
    provider_chain: &Value,
    selected_provider: &str,
    error_code: Option<&str>,
) -> Value {
    let gate_should_execute = tool_execution_gate
        .get("should_execute")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let gate_mode = clean_text(
        tool_execution_gate
            .get("mode")
            .and_then(Value::as_str)
            .unwrap_or("unknown"),
        40,
    )
    .to_ascii_lowercase();
    let gate_reason = clean_text(
        tool_execution_gate
            .get("reason")
            .and_then(Value::as_str)
            .unwrap_or("unknown"),
        120,
    )
    .to_ascii_lowercase();
    let replay_blocked = attempt_replay_guard
        .get("blocked")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let replay_reason = clean_text(
        attempt_replay_guard
            .get("reason")
            .and_then(Value::as_str)
            .unwrap_or("none"),
        120,
    )
    .to_ascii_lowercase();
    let normalized_tool = clean_text(tool, 60).to_ascii_lowercase();
    let normalized_phase = clean_text(phase, 80).to_ascii_lowercase();
    let normalized_provider = clean_text(selected_provider, 80).to_ascii_lowercase();
    let normalized_error = error_code
        .map(|raw| clean_text(raw, 120))
        .filter(|value| !value.is_empty());
    json!({
        "workflow": "default_web_tooling_v2",
        "tool": if normalized_tool.is_empty() { "web_tool" } else { normalized_tool.as_str() },
        "phase": if normalized_phase.is_empty() { "unknown" } else { normalized_phase.as_str() },
        "tool_execution_attempted": tool_execution_attempted,
        "provider_chain": provider_chain.clone(),
        "selected_provider": if normalized_provider.is_empty() { "none" } else { normalized_provider.as_str() },
        "gate": {
            "should_execute": gate_should_execute,
            "mode": gate_mode,
            "reason": gate_reason
        },
        "replay_guard": {
            "blocked": replay_blocked,
            "reason": replay_reason
        },
        "result": {
            "ok": normalized_error.is_none(),
            "error_code": normalized_error
        }
    })
}
