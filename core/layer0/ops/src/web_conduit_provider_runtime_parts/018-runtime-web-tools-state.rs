const RUNTIME_WEB_TOOLS_METADATA_REL: &str =
    "client/runtime/local/state/web_conduit/runtime_web_tools_metadata.json";
const WEB_RECEIPTS_REL: &str = "client/runtime/local/state/web_conduit/receipts.jsonl";

fn runtime_web_tools_metadata_path(root: &Path) -> PathBuf {
    runtime_state_path(root, RUNTIME_WEB_TOOLS_METADATA_REL)
}

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

fn runtime_web_tools_exports_contract() -> Value {
    json!({
        "module_entrypoint": "src/agents/tools/web-tools.ts",
        "exports": ["createWebFetchTool", "extractReadableContent", "createWebSearchTool"],
        "web_fetch_factory": "createWebFetchTool",
        "readability_helper": "extractReadableContent",
        "web_search_factory": "createWebSearchTool"
    })
}

fn runtime_web_tools_default_enablement_contract() -> Value {
    json!({
        "web_fetch_enabled_by_default_non_sandbox": true,
        "web_fetch_explicit_disable_supported": true,
        "web_search_runtime_provider_override_supported": true,
        "web_search_runtime_only_provider_hydration": true,
        "runtime_web_search_metadata_fields": [
            "providerConfigured",
            "providerSource",
            "selectedProvider",
            "selectedProviderKeySource",
            "diagnostics",
            "toolSurfaceHealth"
        ],
        "runtime_metadata_provider_override_supported": true,
        "runtime_metadata_provider_override_field": "runtimeWebSearch.selectedProvider"
    })
}

fn runtime_web_fetch_unit_test_harness_contract() -> Value {
    json!({
        "headers_factory_entrypoint": "makeFetchHeaders",
        "headers_key_normalizer": "normalizeLowercaseStringOrEmpty",
        "headers_lookup_contract": "map[normalizeLowercaseStringOrEmpty(key)] ?? null",
        "base_test_config_entrypoint": "createBaseWebFetchToolConfig",
        "base_test_opts_supported": ["maxResponseBytes", "lookupFn"],
        "base_test_defaults": {
            "cache_ttl_minutes": 0,
            "firecrawl_enabled": false
        },
        "base_test_optional_overrides": {
            "max_response_bytes_config_path": "config.tools.web.fetch.maxResponseBytes",
            "lookup_fn_passthrough_field": "lookupFn",
            "max_response_bytes_added_only_when_truthy": true,
            "lookup_fn_added_only_when_present": true
        },
        "max_response_bytes_override_supported": true,
        "readability_test_mock_entrypoint": "web-fetch.test-mocks.ts",
        "readability_test_mock_behavior": "extractReadableContent returns deterministic title/text to avoid heavy dynamic imports"
    })
}

fn default_runtime_web_tools_metadata() -> Value {
    json!({
        "search": {
            "provider_configured": Value::Null,
            "provider_source": "none",
            "selected_provider": Value::Null,
            "selected_provider_key_source": Value::Null,
            "configured_surface_path": Value::Null,
            "config_surface": Value::Null,
            "manifest_contract_owner": Value::Null,
            "public_artifact_runtime": public_artifact_contract_for_family(WebProviderFamily::Search),
            "tool_surface_health": default_runtime_web_family_health(WebProviderFamily::Search),
            "diagnostics": []
        },
        "fetch": {
            "provider_configured": Value::Null,
            "provider_source": "none",
            "selected_provider": Value::Null,
            "selected_provider_key_source": Value::Null,
            "configured_surface_path": Value::Null,
            "config_surface": Value::Null,
            "manifest_contract_owner": Value::Null,
            "public_artifact_runtime": public_artifact_contract_for_family(WebProviderFamily::Fetch),
            "tool_surface_health": default_runtime_web_family_health(WebProviderFamily::Fetch),
            "diagnostics": []
        },
        "image_tool": default_image_tool_runtime_metadata(),
        "openclaw_web_tools_contract": {
            "exports": runtime_web_tools_exports_contract(),
            "default_enablement": runtime_web_tools_default_enablement_contract(),
            "fetch_unit_test_harness": runtime_web_fetch_unit_test_harness_contract()
        },
        "tool_surface_health": default_runtime_web_tools_health_summary(),
        "diagnostics": []
    })
}

pub(crate) fn load_active_runtime_web_tools_metadata(root: &Path) -> Value {
    read_json_or(
        &runtime_web_tools_metadata_path(root),
        default_runtime_web_tools_metadata(),
    )
}

fn store_active_runtime_web_tools_metadata(root: &Path, metadata: &Value) {
    let _ = write_json_atomic(&runtime_web_tools_metadata_path(root), metadata);
}

pub(crate) fn clear_active_runtime_web_tools_metadata(root: &Path) {
    let _ = std::fs::remove_file(runtime_web_tools_metadata_path(root));
}

fn raw_provider_tokens_from_value(raw: &Value) -> Vec<String> {
    let rows = if let Some(array) = raw.as_array() {
        array
            .iter()
            .filter_map(|row| row.as_str())
            .flat_map(|row| row.split(|ch: char| ch == ',' || ch.is_ascii_whitespace()))
            .map(str::trim)
            .filter(|row| !row.is_empty())
            .map(|row| clean_text(row, 60).to_ascii_lowercase())
            .collect::<Vec<_>>()
    } else if let Some(single) = raw.as_str() {
        single
            .split(|ch: char| ch == ',' || ch.is_ascii_whitespace())
            .map(str::trim)
            .filter(|row| !row.is_empty())
            .map(|row| clean_text(row, 60).to_ascii_lowercase())
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    dedupe_preserve(rows)
}

fn first_raw_provider_token_from_value(raw: &Value) -> Option<String> {
    raw_provider_tokens_from_value(raw).into_iter().next()
}

fn raw_provider_tokens_from_policy(policy: &Value, family: WebProviderFamily) -> Vec<String> {
    match family {
        WebProviderFamily::Search => policy
            .pointer("/web_conduit/search_provider_order")
            .or_else(|| policy.get("search_provider_order"))
            .map(raw_provider_tokens_from_value)
            .unwrap_or_default(),
        WebProviderFamily::Fetch => policy
            .pointer("/web_conduit/fetch_provider_order")
            .or_else(|| policy.get("fetch_provider_order"))
            .map(raw_provider_tokens_from_value)
            .unwrap_or_default(),
    }
}

fn configured_provider_input_from_policy(
    policy: &Value,
    family: WebProviderFamily,
) -> Option<String> {
    let explicit = match family {
        WebProviderFamily::Search => policy
            .pointer("/web_conduit/search_provider")
            .or_else(|| policy.get("search_provider")),
        WebProviderFamily::Fetch => policy
            .pointer("/web_conduit/fetch_provider")
            .or_else(|| policy.get("fetch_provider")),
    }
    .and_then(Value::as_str)
    .map(|raw| clean_text(raw, 60).to_ascii_lowercase())
    .filter(|value| !value.is_empty());
    explicit.or_else(|| match family {
        WebProviderFamily::Search => policy
            .pointer("/web_conduit/search_provider_order")
            .or_else(|| policy.get("search_provider_order"))
            .and_then(first_raw_provider_token_from_value),
        WebProviderFamily::Fetch => policy
            .pointer("/web_conduit/fetch_provider_order")
            .or_else(|| policy.get("fetch_provider_order"))
            .and_then(first_raw_provider_token_from_value),
    })
}

fn runtime_diagnostic(code: &str, message: String, path: &str) -> Value {
    json!({
        "code": code,
        "message": clean_text(&message, 260),
        "path": path
    })
}

fn default_runtime_web_family_health(family: WebProviderFamily) -> Value {
    json!({
        "status": "unavailable",
        "selected_provider_ready": false,
        "selected_provider_requires_credential": false,
        "selected_provider_credential_state": "unknown",
        "blocking_reason": "no_selected_provider",
        "available_provider_count": builtin_provider_descriptors(family).len(),
        "diagnostic_count": 0
    })
}

fn default_runtime_web_tools_health_summary() -> Value {
    json!({
        "status": "unavailable",
        "search_status": "unavailable",
        "fetch_status": "unavailable",
        "search_ready": false,
        "fetch_ready": false
    })
}

fn default_runtime_web_execution_gate() -> Value {
    json!({
        "should_execute": false,
        "mode": "blocked",
        "reason": "unknown",
        "retry_recommended": true,
        "retry_lane": "repair_tool_surface"
    })
}

pub(crate) fn runtime_web_execution_gate(
    tool_surface_status: &str,
    tool_surface_ready: bool,
    allow_fallback: bool,
    blocking_reason: &str,
) -> Value {
    let status = clean_text(tool_surface_status, 40).to_ascii_lowercase();
    let reason = clean_text(blocking_reason, 120).to_ascii_lowercase();
    if status == "ready" && (tool_surface_ready || allow_fallback) {
        return json!({
            "should_execute": true,
            "mode": "allow",
            "reason": "none",
            "retry_recommended": false,
            "retry_lane": "none"
        });
    }
    if status == "degraded" {
        if tool_surface_ready || allow_fallback {
            return json!({
                "should_execute": true,
                "mode": "degraded_allow",
                "reason": if reason.is_empty() { "degraded_but_fallback_available" } else { reason.as_str() },
                "retry_recommended": true,
                "retry_lane": "degraded_backoff"
            });
        }
        return json!({
            "should_execute": false,
            "mode": "blocked",
            "reason": if reason.is_empty() { "degraded_without_fallback" } else { reason.as_str() },
            "retry_recommended": true,
            "retry_lane": "repair_tool_surface"
        });
    }
    if status == "unavailable" {
        return json!({
            "should_execute": false,
            "mode": "blocked",
            "reason": if reason.is_empty() { "tool_surface_unavailable" } else { reason.as_str() },
            "retry_recommended": true,
            "retry_lane": "repair_tool_surface"
        });
    }
    json!({
        "should_execute": false,
        "mode": "blocked",
        "reason": if reason.is_empty() { "unknown" } else { reason.as_str() },
        "retry_recommended": true,
        "retry_lane": "repair_tool_surface"
    })
}

fn runtime_web_family_health(
    family: WebProviderFamily,
    selected_provider: Option<&str>,
    selected_provider_key_source: &Value,
    selection_fallback_reason: Option<&str>,
    diagnostics: &[Value],
) -> Value {
    let selected_provider_requires_credential = selected_provider
        .and_then(|provider| provider_descriptor(provider, family))
        .map(|descriptor| !descriptor.env_keys.is_empty())
        .unwrap_or(false);
    let selected_provider_credential_state = match (
        selected_provider_key_source.as_str(),
        selected_provider_requires_credential,
    ) {
        (Some("config" | "env" | "not_required"), _) => "resolved",
        (Some("missing"), true) => "missing",
        (Some("missing"), false) => "not_required",
        _ => "unknown",
    };
    let selected_provider_ready = selected_provider.is_some()
        && (selected_provider_credential_state == "resolved"
            || selected_provider_credential_state == "not_required");
    let status = if selected_provider.is_none() {
        "unavailable"
    } else if selection_fallback_reason == Some("credential_unresolved")
        || (selected_provider_requires_credential
            && selected_provider_credential_state == "missing")
    {
        "degraded"
    } else {
        "ready"
    };
    let blocking_reason = if selected_provider.is_none() {
        "no_selected_provider"
    } else if selection_fallback_reason == Some("credential_unresolved") {
        "configured_provider_credential_unresolved"
    } else if selected_provider_requires_credential
        && selected_provider_credential_state == "missing"
    {
        "selected_provider_credential_missing"
    } else {
        "none"
    };
    json!({
        "status": status,
        "selected_provider_ready": selected_provider_ready,
        "selected_provider_requires_credential": selected_provider_requires_credential,
        "selected_provider_credential_state": selected_provider_credential_state,
        "blocking_reason": blocking_reason,
        "available_provider_count": builtin_provider_descriptors(family).len(),
        "diagnostic_count": diagnostics.len()
    })
}

fn invalid_provider_code(family: WebProviderFamily) -> &'static str {
    match family {
        WebProviderFamily::Search => "WEB_SEARCH_PROVIDER_INVALID_AUTODETECT",
        WebProviderFamily::Fetch => "WEB_FETCH_PROVIDER_INVALID_AUTODETECT",
    }
}

fn auto_detect_code(family: WebProviderFamily) -> &'static str {
    match family {
        WebProviderFamily::Search => "WEB_SEARCH_AUTODETECT_SELECTED",
        WebProviderFamily::Fetch => "WEB_FETCH_AUTODETECT_SELECTED",
    }
}

fn fallback_used_code(family: WebProviderFamily) -> &'static str {
    match family {
        WebProviderFamily::Search => "WEB_SEARCH_KEY_UNRESOLVED_FALLBACK_USED",
        WebProviderFamily::Fetch => "WEB_FETCH_PROVIDER_KEY_UNRESOLVED_FALLBACK_USED",
    }
}

fn no_fallback_code(family: WebProviderFamily) -> &'static str {
    match family {
        WebProviderFamily::Search => "WEB_SEARCH_KEY_UNRESOLVED_NO_FALLBACK",
        WebProviderFamily::Fetch => "WEB_FETCH_PROVIDER_KEY_UNRESOLVED_NO_FALLBACK",
    }
}

fn configured_scope_path(provider: &str, family: WebProviderFamily) -> String {
    match family {
        WebProviderFamily::Search => format!("/web_conduit/search_provider_config/{provider}"),
        WebProviderFamily::Fetch => format!("/web_conduit/fetch_provider_config/{provider}"),
    }
}

fn config_surface_snapshot(
    policy: &Value,
    provider: Option<&str>,
    family: WebProviderFamily,
) -> Value {
    let Some(provider_id) = provider else {
        return Value::Null;
    };
    match family {
        WebProviderFamily::Search => {
            let section = search_provider_config_section(policy, provider_id);
            let inline_present = section
                .and_then(|row| row.get("api_key"))
                .and_then(Value::as_str)
                .map(|raw| !clean_text(raw, 600).is_empty())
                .unwrap_or(false);
            let env_name = section
                .and_then(|row| row.get("api_key_env"))
                .and_then(Value::as_str)
                .map(|raw| clean_text(raw, 160))
                .filter(|value| !value.is_empty());
            json!({
                "path": configured_scope_path(provider_id, family),
                "configured": section.is_some(),
                "has_inline_api_key": inline_present,
                "has_api_key_env": env_name.is_some(),
                "api_key_env": env_name
            })
        }
        WebProviderFamily::Fetch => json!({
            "path": configured_scope_path(provider_id, family),
            "configured": false
        }),
    }
}

fn manifest_contract_owner(provider: Option<&str>, family: WebProviderFamily) -> Value {
    provider
        .map(|provider_id| {
            json!({
                "kind": "built_in",
                "provider": provider_id,
                "family": provider_family_name(family)
            })
        })
        .unwrap_or(Value::Null)
}

fn selected_provider_key_source(
    policy: &Value,
    provider: Option<&str>,
    family: WebProviderFamily,
) -> Value {
    let Some(provider_id) = provider else {
        return Value::Null;
    };
    let raw = match family {
        WebProviderFamily::Search => {
            resolve_provider_credential_source_with_env(policy, provider_id, family, |key| {
                std::env::var(key).ok()
            })
        }
        WebProviderFamily::Fetch => "not_required".to_string(),
    };
    let normalized = match raw.as_str() {
        "policy_inline" => "config",
        "policy_env" | "env" => "env",
        "not_required" => "not_required",
        _ => "missing",
    };
    Value::String(normalized.to_string())
}

fn runtime_web_family_metadata(root: &Path, policy: &Value, family: WebProviderFamily) -> Value {
    let configured_path = match family {
        WebProviderFamily::Search => "/web_conduit/search_provider_order",
        WebProviderFamily::Fetch => "/web_conduit/fetch_provider_order",
    };
    let configured_provider_input = configured_provider_input_from_policy(policy, family);
    let configured_provider = configured_provider_input
        .as_ref()
        .and_then(|raw| normalize_provider_token_for_family(raw, family));
    let selected_provider = match family {
        WebProviderFamily::Search => resolved_search_provider_chain("", &json!({}), policy)
            .first()
            .cloned(),
        WebProviderFamily::Fetch => fetch_provider_chain_from_request("", &json!({}), policy)
            .first()
            .cloned(),
    };
    let mut diagnostics = Vec::<Value>::new();
    if let Some(raw) = configured_provider_input.as_ref() {
        if configured_provider.is_none() {
            diagnostics.push(runtime_diagnostic(
                invalid_provider_code(family),
                format!(
                    "{configured_path} contains unsupported provider token \"{raw}\"; falling back to auto-detect precedence."
                ),
                configured_path,
            ));
        }
    }
    for raw in raw_provider_tokens_from_policy(policy, family) {
        if normalize_provider_token_for_family(&raw, family).is_none()
            && configured_provider_input.as_deref() != Some(raw.as_str())
        {
            diagnostics.push(runtime_diagnostic(
                invalid_provider_code(family),
                format!(
                    "{configured_path} contains unsupported provider token \"{raw}\"; falling back to auto-detect precedence."
                ),
                configured_path,
            ));
        }
    }
    let provider_source = if let Some(configured) = configured_provider.as_ref() {
        if selected_provider.as_ref() == Some(configured) {
            "configured"
        } else if selected_provider.is_some() {
            let missing_credential =
                !provider_has_runtime_credential_with(configured, family, |key| {
                    std::env::var(key).ok()
                }) && provider_descriptor(configured, family)
                    .map(|descriptor| !descriptor.env_keys.is_empty())
                    .unwrap_or(false);
            if missing_credential {
                if let Some(selected) = selected_provider.as_ref() {
                    diagnostics.push(runtime_diagnostic(
                        fallback_used_code(family),
                        format!(
                            "{configured_path} prefers \"{configured}\", but its credential is unresolved; falling back to \"{selected}\"."
                        ),
                        &configured_scope_path(configured, family),
                    ));
                } else {
                    diagnostics.push(runtime_diagnostic(
                        no_fallback_code(family),
                        format!(
                            "{configured_path} prefers \"{configured}\", but no credential-backed or keyless fallback provider is available."
                        ),
                        &configured_scope_path(configured, family),
                    ));
                }
            }
            "auto-detect"
        } else {
            "none"
        }
    } else if let Some(selected) = selected_provider.as_ref() {
        diagnostics.push(runtime_diagnostic(
            auto_detect_code(family),
            format!(
                "{} auto-detected provider \"{selected}\".",
                provider_family_name(family)
            ),
            configured_path,
        ));
        "auto-detect"
    } else {
        "none"
    };
    let selection_fallback_reason = if configured_provider_input.is_some()
        && configured_provider.is_none()
        && selected_provider.is_some()
    {
        Some("invalid_configured_provider")
    } else if configured_provider.is_some()
        && selected_provider.is_some()
        && selected_provider != configured_provider
    {
        Some("credential_unresolved")
    } else {
        None
    };
    let owner_provider = selected_provider
        .as_deref()
        .or(configured_provider.as_deref());
    let selected_provider_key_source = selected_provider_key_source(policy, owner_provider, family);
    let tool_surface_health = runtime_web_family_health(
        family,
        selected_provider.as_deref(),
        &selected_provider_key_source,
        selection_fallback_reason,
        &diagnostics,
    );
    let allow_fallback_hint = provider_source != "configured";
    let execution_gate = runtime_web_execution_gate(
        tool_surface_health
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("unavailable"),
        tool_surface_health
            .get("selected_provider_ready")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        allow_fallback_hint,
        tool_surface_health
            .get("blocking_reason")
            .and_then(Value::as_str)
            .unwrap_or("none"),
    );
    json!({
        "configured_provider_input": configured_provider_input,
        "provider_configured": configured_provider,
        "provider_source": provider_source,
        "selected_provider": selected_provider,
        "selected_provider_key_source": selected_provider_key_source,
        "selection_fallback_reason": selection_fallback_reason,
        "configured_surface_path": configured_provider
            .as_deref()
            .map(|provider| configured_scope_path(provider, family)),
        "config_surface": config_surface_snapshot(policy, owner_provider, family),
        "manifest_contract_owner": manifest_contract_owner(owner_provider, family),
        "public_artifact_runtime": public_artifact_contract_for_family(family),
        "tool_surface_health": tool_surface_health,
        "execution_gate": execution_gate,
        "resolution_contract": runtime_resolution_contract(family),
        "state_path": runtime_web_tools_state_path(root).display().to_string(),
        "diagnostics": diagnostics
    })
}

pub(crate) fn runtime_web_tools_snapshot(root: &Path, policy: &Value) -> Value {
    let search = runtime_web_family_metadata(root, policy, WebProviderFamily::Search);
    let fetch = runtime_web_family_metadata(root, policy, WebProviderFamily::Fetch);
    let image_tool = image_tool_runtime_resolution_snapshot(root, policy, &json!({}));
    let search_status = search
        .pointer("/tool_surface_health/status")
        .and_then(Value::as_str)
        .unwrap_or("unavailable");
    let fetch_status = fetch
        .pointer("/tool_surface_health/status")
        .and_then(Value::as_str)
        .unwrap_or("unavailable");
    let search_ready = search
        .pointer("/tool_surface_health/selected_provider_ready")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let fetch_ready = fetch
        .pointer("/tool_surface_health/selected_provider_ready")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let search_execution_gate = search
        .get("execution_gate")
        .cloned()
        .unwrap_or_else(default_runtime_web_execution_gate);
    let fetch_execution_gate = fetch
        .get("execution_gate")
        .cloned()
        .unwrap_or_else(default_runtime_web_execution_gate);
    let overall_should_execute = search_execution_gate
        .get("should_execute")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        || fetch_execution_gate
            .get("should_execute")
            .and_then(Value::as_bool)
            .unwrap_or(false);
    let overall_status = if search_status == "unavailable" || fetch_status == "unavailable" {
        "unavailable"
    } else if search_status == "degraded" || fetch_status == "degraded" {
        "degraded"
    } else {
        "ready"
    };
    let diagnostics = search
        .get("diagnostics")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .cloned()
        .chain(
            fetch
                .get("diagnostics")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .cloned(),
        )
        .chain(
            image_tool
                .get("diagnostics")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .cloned(),
        )
        .collect::<Vec<_>>();
    let metadata = json!({
        "search": search,
        "fetch": fetch,
        "image_tool": image_tool,
        "openclaw_web_tools_contract": {
            "exports": runtime_web_tools_exports_contract(),
            "default_enablement": runtime_web_tools_default_enablement_contract(),
            "fetch_unit_test_harness": runtime_web_fetch_unit_test_harness_contract()
        },
        "tool_surface_health": {
            "status": overall_status,
            "search_status": search_status,
            "fetch_status": fetch_status,
            "search_ready": search_ready,
            "fetch_ready": fetch_ready
        },
        "tool_execution_gate": {
            "search": search_execution_gate,
            "fetch": fetch_execution_gate,
            "overall_should_execute": overall_should_execute,
            "overall_mode": if overall_should_execute { "allow_any" } else { "blocked_all" }
        },
        "diagnostics": diagnostics
    });
    store_active_runtime_web_tools_metadata(root, &metadata);
    metadata
}

#[cfg(test)]
mod openclaw_runtime_web_tools_tests {
    use super::*;

    #[test]
    fn runtime_web_tools_snapshot_persists_active_state() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let policy = json!({
            "web_conduit": {
                "search_provider_order": ["duckduckgo", "bing_rss"],
                "fetch_provider_order": ["direct_http"]
            }
        });
        let metadata = runtime_web_tools_snapshot(tmp.path(), &policy);
        assert_eq!(
            metadata
                .pointer("/search/selected_provider")
                .and_then(Value::as_str),
            Some("duckduckgo")
        );
        let loaded = load_active_runtime_web_tools_metadata(tmp.path());
        assert_eq!(
            loaded
                .pointer("/fetch/selected_provider")
                .and_then(Value::as_str),
            Some("direct_http")
        );
    }

    #[test]
    fn runtime_web_tools_snapshot_load_is_defensive_clone() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let policy = json!({
            "web_conduit": {
                "search_provider_order": ["duckduckgo", "bing_rss"],
                "fetch_provider_order": ["direct_http"]
            }
        });
        let _snapshot = runtime_web_tools_snapshot(tmp.path(), &policy);
        let mut loaded = load_active_runtime_web_tools_metadata(tmp.path());
        if let Some(search) = loaded.pointer_mut("/search").and_then(Value::as_object_mut) {
            search.insert("selected_provider".to_string(), json!("brave"));
            search.insert("provider_configured".to_string(), json!("brave"));
        }
        let reloaded = load_active_runtime_web_tools_metadata(tmp.path());
        assert_eq!(
            reloaded
                .pointer("/search/selected_provider")
                .and_then(Value::as_str),
            Some("duckduckgo")
        );
        assert_eq!(
            reloaded
                .pointer("/search/provider_configured")
                .and_then(Value::as_str),
            Some("duckduckgo")
        );
    }

    #[test]
    fn runtime_web_tools_snapshot_exposes_openclaw_contract_markers() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let policy = json!({
            "web_conduit": {
                "search_provider_order": ["duckduckgo", "bing_rss"],
                "fetch_provider_order": ["direct_http"]
            }
        });
        let metadata = runtime_web_tools_snapshot(tmp.path(), &policy);
        assert_eq!(
            metadata
                .pointer("/search/resolution_contract/runtime_mode")
                .and_then(Value::as_str),
            Some("built_in_only")
        );
        assert_eq!(
            metadata
                .pointer("/fetch/resolution_contract/runtime_mode")
                .and_then(Value::as_str),
            Some("built_in_only")
        );
        assert_eq!(
            metadata
                .pointer("/openclaw_web_tools_contract/exports/module_entrypoint")
                .and_then(Value::as_str),
            Some("src/agents/tools/web-tools.ts")
        );
        assert_eq!(
            metadata
                .pointer("/openclaw_web_tools_contract/exports/exports/0")
                .and_then(Value::as_str),
            Some("createWebFetchTool")
        );
        assert_eq!(
            metadata
                .pointer("/openclaw_web_tools_contract/exports/exports/1")
                .and_then(Value::as_str),
            Some("extractReadableContent")
        );
        assert_eq!(
            metadata
                .pointer("/openclaw_web_tools_contract/exports/readability_helper")
                .and_then(Value::as_str),
            Some("extractReadableContent")
        );
        assert_eq!(
            metadata
                .pointer("/openclaw_web_tools_contract/default_enablement/web_fetch_enabled_by_default_non_sandbox")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            metadata
                .pointer("/openclaw_web_tools_contract/default_enablement/web_search_runtime_provider_override_supported")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            metadata
                .pointer("/openclaw_web_tools_contract/default_enablement/runtime_web_search_metadata_fields/2")
                .and_then(Value::as_str),
            Some("selectedProvider")
        );
        assert_eq!(
            metadata
                .pointer("/openclaw_web_tools_contract/default_enablement/runtime_metadata_provider_override_field")
                .and_then(Value::as_str),
            Some("runtimeWebSearch.selectedProvider")
        );
        assert_eq!(
            metadata
                .pointer("/openclaw_web_tools_contract/fetch_unit_test_harness/headers_factory_entrypoint")
                .and_then(Value::as_str),
            Some("makeFetchHeaders")
        );
        assert_eq!(
            metadata
                .pointer("/openclaw_web_tools_contract/fetch_unit_test_harness/headers_lookup_contract")
                .and_then(Value::as_str),
            Some("map[normalizeLowercaseStringOrEmpty(key)] ?? null")
        );
        assert_eq!(
            metadata
                .pointer("/openclaw_web_tools_contract/fetch_unit_test_harness/base_test_defaults/cache_ttl_minutes")
                .and_then(Value::as_u64),
            Some(0)
        );
        assert_eq!(
            metadata
                .pointer("/openclaw_web_tools_contract/fetch_unit_test_harness/base_test_optional_overrides/max_response_bytes_added_only_when_truthy")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            metadata
                .pointer("/openclaw_web_tools_contract/fetch_unit_test_harness/readability_test_mock_entrypoint")
                .and_then(Value::as_str),
            Some("web-fetch.test-mocks.ts")
        );
        assert!(metadata
            .pointer("/diagnostics")
            .and_then(Value::as_array)
            .is_some());
    }

    #[test]
    fn runtime_web_tools_snapshot_flags_invalid_search_provider_tokens() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let policy = json!({
            "web_conduit": {
                "search_provider_order": ["perplexity", "duckduckgo"],
                "fetch_provider_order": ["direct_http"]
            }
        });
        let metadata = runtime_web_tools_snapshot(tmp.path(), &policy);
        assert_eq!(
            metadata
                .pointer("/search/provider_source")
                .and_then(Value::as_str),
            Some("auto-detect")
        );
        assert!(metadata
            .pointer("/search/diagnostics")
            .and_then(Value::as_array)
            .map(|rows| rows
                .iter()
                .any(|row| row.get("code").and_then(Value::as_str)
                    == Some("WEB_SEARCH_PROVIDER_INVALID_AUTODETECT")))
            .unwrap_or(false));
    }

    #[test]
    fn clear_active_runtime_web_tools_metadata_removes_persisted_snapshot() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let policy = json!({
            "web_conduit": {
                "search_provider_order": ["duckduckgo"],
                "fetch_provider_order": ["direct_http"]
            }
        });
        let _metadata = runtime_web_tools_snapshot(tmp.path(), &policy);
        assert!(runtime_web_tools_state_path(tmp.path()).exists());
        clear_active_runtime_web_tools_metadata(tmp.path());
        assert!(!runtime_web_tools_state_path(tmp.path()).exists());
        let loaded = load_active_runtime_web_tools_metadata(tmp.path());
        assert_eq!(
            loaded
                .pointer("/search/provider_source")
                .and_then(Value::as_str),
            Some("none")
        );
    }

    #[test]
    fn runtime_web_tools_snapshot_reports_missing_key_fallback() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let policy = json!({
            "web_conduit": {
                "search_provider_order": ["serperdev", "duckduckgo"],
                "fetch_provider_order": ["direct_http"]
            }
        });
        let metadata = runtime_web_tools_snapshot(tmp.path(), &policy);
        assert_eq!(
            metadata
                .pointer("/search/provider_configured")
                .and_then(Value::as_str),
            Some("serperdev")
        );
        assert_eq!(
            metadata
                .pointer("/search/selected_provider")
                .and_then(Value::as_str),
            Some("duckduckgo")
        );
        assert!(metadata
            .pointer("/search/diagnostics")
            .and_then(Value::as_array)
            .map(|rows| rows
                .iter()
                .any(|row| row.get("code").and_then(Value::as_str)
                    == Some("WEB_SEARCH_KEY_UNRESOLVED_FALLBACK_USED")))
            .unwrap_or(false));
        assert_eq!(
            metadata
                .pointer("/search/tool_surface_health/status")
                .and_then(Value::as_str),
            Some("degraded")
        );
        assert_eq!(
            metadata
                .pointer("/search/tool_surface_health/blocking_reason")
                .and_then(Value::as_str),
            Some("configured_provider_credential_unresolved")
        );
        assert_eq!(
            metadata
                .pointer("/search/tool_surface_health/selected_provider_ready")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            metadata
                .pointer("/tool_surface_health/search_status")
                .and_then(Value::as_str),
            Some("degraded")
        );
    }
}
