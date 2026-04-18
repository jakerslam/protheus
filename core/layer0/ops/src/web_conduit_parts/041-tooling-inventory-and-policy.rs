fn runtime_web_tooling_canonical_id(raw: &str) -> String {
    clean_text(raw, 120)
        .to_ascii_lowercase()
        .replace('_', ".")
        .replace('-', ".")
}

fn runtime_web_tooling_surface_error_code(
    surface_status: &str,
    gate_mode: &str,
    blocking_reason: &str,
) -> &'static str {
    if gate_mode == "allow" || gate_mode == "degraded_allow" {
        return "none";
    }
    if surface_status == "unavailable" {
        return "web_tool_surface_unavailable";
    }
    if surface_status == "degraded" {
        return "web_tool_surface_degraded";
    }
    if blocking_reason.contains("credential")
        || blocking_reason.contains("policy")
        || blocking_reason.contains("deny")
    {
        return "web_tool_policy_blocked";
    }
    "web_tool_blocked"
}

fn runtime_web_tooling_retry_policy_from_gate(gate: &Value) -> Value {
    let gate_mode = clean_text(
        gate.get("mode").and_then(Value::as_str).unwrap_or("blocked"),
        40,
    )
    .to_ascii_lowercase();
    let retry_recommended = gate
        .get("retry_recommended")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let retry_lane = clean_text(
        gate.get("retry_lane").and_then(Value::as_str).unwrap_or("none"),
        64,
    );
    let (attempts, base_delay_ms, max_delay_ms, jitter) = if !retry_recommended {
        (0_u64, 0_u64, 0_u64, false)
    } else if gate_mode == "blocked" {
        (3_u64, 450_u64, 2400_u64, true)
    } else if gate_mode == "degraded_allow" {
        (2_u64, 250_u64, 1200_u64, true)
    } else {
        (1_u64, 120_u64, 350_u64, false)
    };
    json!({
        "recommended": retry_recommended,
        "lane": if retry_lane.is_empty() { "none" } else { retry_lane.as_str() },
        "attempts": attempts,
        "base_delay_ms": base_delay_ms,
        "max_delay_ms": max_delay_ms,
        "jitter": jitter
    })
}

fn runtime_web_tooling_effective_row(
    tool_id: &str,
    family: &str,
    source: &str,
    surface_status: &str,
    selected_provider: &str,
    credential_state: &str,
    gate: &Value,
) -> Value {
    let gate_mode = clean_text(gate.get("mode").and_then(Value::as_str).unwrap_or("blocked"), 40)
        .to_ascii_lowercase();
    let should_execute = gate
        .get("should_execute")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let blocking_reason = clean_text(
        gate.get("reason").and_then(Value::as_str).unwrap_or("unknown"),
        120,
    )
    .to_ascii_lowercase();
    let blocking_error_code =
        runtime_web_tooling_surface_error_code(surface_status, &gate_mode, &blocking_reason);
    let retry_policy = runtime_web_tooling_retry_policy_from_gate(gate);
    json!({
        "tool_id": tool_id,
        "family": family,
        "source": source,
        "surface_status": surface_status,
        "selected_provider": selected_provider,
        "credential_state": credential_state,
        "should_execute": should_execute,
        "execution_mode": gate_mode,
        "blocking_reason": blocking_reason,
        "blocking_error_code": blocking_error_code,
        "retry_policy": retry_policy
    })
}

fn runtime_web_tooling_effective_inventory(
    runtime_web_tools_metadata: &Value,
    tool_catalog: &Value,
) -> Value {
    let search_gate = runtime_web_tools_metadata
        .pointer("/tool_execution_gate/search")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let fetch_gate = runtime_web_tools_metadata
        .pointer("/tool_execution_gate/fetch")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let search_surface_status = clean_text(
        runtime_web_tools_metadata
            .pointer("/tool_surface_health/search_status")
            .and_then(Value::as_str)
            .unwrap_or("unknown"),
        40,
    )
    .to_ascii_lowercase();
    let fetch_surface_status = clean_text(
        runtime_web_tools_metadata
            .pointer("/tool_surface_health/fetch_status")
            .and_then(Value::as_str)
            .unwrap_or("unknown"),
        40,
    )
    .to_ascii_lowercase();
    let mut rows = vec![
        runtime_web_tooling_effective_row(
            "web.search",
            "search",
            "runtime_web_tools_metadata",
            &search_surface_status,
            &clean_text(
                runtime_web_tools_metadata
                    .pointer("/search/selected_provider")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                80,
            ),
            &clean_text(
                runtime_web_tools_metadata
                    .pointer("/search/tool_surface_health/selected_provider_credential_state")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown"),
                40,
            )
            .to_ascii_lowercase(),
            &search_gate,
        ),
        runtime_web_tooling_effective_row(
            "web.fetch",
            "fetch",
            "runtime_web_tools_metadata",
            &fetch_surface_status,
            &clean_text(
                runtime_web_tools_metadata
                    .pointer("/fetch/selected_provider")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                80,
            ),
            &clean_text(
                runtime_web_tools_metadata
                    .pointer("/fetch/tool_surface_health/selected_provider_credential_state")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown"),
                40,
            )
            .to_ascii_lowercase(),
            &fetch_gate,
        ),
    ];
    let image_execution_mode = clean_text(
        runtime_web_tools_metadata
            .pointer("/image_tool/execution_mode")
            .and_then(Value::as_str)
            .unwrap_or("selection_only"),
        40,
    )
    .to_ascii_lowercase();
    let image_surface_status = clean_text(
        runtime_web_tools_metadata
            .pointer("/image_tool/tool_surface_health/status")
            .or_else(|| runtime_web_tools_metadata.pointer("/image_tool/status"))
            .and_then(Value::as_str)
            .unwrap_or("degraded"),
        40,
    )
    .to_ascii_lowercase();
    let image_gate = json!({
        "should_execute": image_execution_mode == "live_execution",
        "mode": if image_execution_mode == "live_execution" { "allow" } else { "blocked" },
        "reason": if image_execution_mode == "live_execution" { "none" } else { "image_tool_selection_only" },
        "retry_recommended": image_execution_mode != "live_execution",
        "retry_lane": if image_execution_mode == "live_execution" { "none" } else { "configure_image_transport" }
    });
    rows.push(runtime_web_tooling_effective_row(
        "web.image.query",
        "image",
        "runtime_web_tools_metadata",
        &image_surface_status,
        &clean_text(
            runtime_web_tools_metadata
                .pointer("/image_tool/selected_provider")
                .and_then(Value::as_str)
                .unwrap_or(""),
            80,
        ),
        &clean_text(
            runtime_web_tools_metadata
                .pointer("/image_tool/tool_surface_health/selected_provider_credential_state")
                .and_then(Value::as_str)
                .unwrap_or("unknown"),
            40,
        )
        .to_ascii_lowercase(),
        &image_gate,
    ));
    if let Some(catalog_rows) = tool_catalog.as_array() {
        for entry in catalog_rows {
            let raw_name = entry
                .get("tool")
                .or_else(|| entry.get("name"))
                .and_then(Value::as_str)
                .unwrap_or("");
            let tool_id = runtime_web_tooling_canonical_id(raw_name);
            if tool_id.is_empty() {
                continue;
            }
            if rows
                .iter()
                .any(|row| row.get("tool_id").and_then(Value::as_str) == Some(tool_id.as_str()))
            {
                continue;
            }
            let enabled = entry.get("enabled").and_then(Value::as_bool).unwrap_or(true);
            rows.push(json!({
                "tool_id": tool_id,
                "family": "catalog",
                "source": "tool_catalog",
                "surface_status": if enabled { "ready" } else { "unavailable" },
                "selected_provider": "",
                "credential_state": "unknown",
                "should_execute": enabled,
                "execution_mode": if enabled { "allow" } else { "blocked" },
                "blocking_reason": if enabled { "none" } else { "catalog_disabled" },
                "blocking_error_code": if enabled { "none" } else { "web_tool_catalog_disabled" },
                "retry_policy": {
                    "recommended": !enabled,
                    "lane": if enabled { "none" } else { "enable_tooling" },
                    "attempts": if enabled { 0 } else { 1 },
                    "base_delay_ms": if enabled { 0 } else { 200 },
                    "max_delay_ms": if enabled { 0 } else { 600 },
                    "jitter": false
                }
            }));
        }
    }
    let total_tools = rows.len() as u64;
    let enabled_tools = rows
        .iter()
        .filter(|row| row.get("should_execute").and_then(Value::as_bool) == Some(true))
        .count() as u64;
    let blocked_tools = total_tools.saturating_sub(enabled_tools);
    let degraded_tools = rows
        .iter()
        .filter(|row| row.get("surface_status").and_then(Value::as_str) == Some("degraded"))
        .count() as u64;
    let mut blocking_error_codes: Vec<String> = Vec::new();
    for row in &rows {
        let code = clean_text(
            row.get("blocking_error_code")
                .and_then(Value::as_str)
                .unwrap_or("none"),
            80,
        );
        if code.is_empty() || code == "none" {
            continue;
        }
        if !blocking_error_codes.iter().any(|existing| existing == &code) {
            blocking_error_codes.push(code);
        }
    }
    json!({
        "version": "effective_inventory_v2",
        "rows": rows,
        "summary": {
            "total_tools": total_tools,
            "enabled_tools": enabled_tools,
            "blocked_tools": blocked_tools,
            "degraded_tools": degraded_tools,
            "blocking_error_codes": blocking_error_codes
        }
    })
}

fn runtime_web_tooling_policy_pipeline(
    runtime_web_tools_metadata: &Value,
    effective_inventory: &Value,
) -> Value {
    let overall_status = clean_text(
        runtime_web_tools_metadata
            .pointer("/tool_surface_health/status")
            .and_then(Value::as_str)
            .unwrap_or("unknown"),
        40,
    )
    .to_ascii_lowercase();
    let overall_should_execute = runtime_web_tools_metadata
        .pointer("/tool_execution_gate/overall_should_execute")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let total_tools = effective_inventory
        .pointer("/summary/total_tools")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let blocked_tools = effective_inventory
        .pointer("/summary/blocked_tools")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let degraded_tools = effective_inventory
        .pointer("/summary/degraded_tools")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let stages = vec![
        json!({
            "stage": "surface_health",
            "status": if overall_status == "ready" { "pass" } else if overall_status == "degraded" { "warn" } else { "fail" },
            "detail": overall_status
        }),
        json!({
            "stage": "execution_gate",
            "status": if overall_should_execute { "pass" } else { "fail" },
            "detail": if overall_should_execute { "allow_any" } else { "blocked_all" }
        }),
        json!({
            "stage": "effective_inventory",
            "status": if total_tools > 0 { "pass" } else { "fail" },
            "detail": format!("rows={total_tools}")
        }),
        json!({
            "stage": "blocked_lanes",
            "status": if blocked_tools == 0 { "pass" } else { "warn" },
            "detail": format!("blocked={blocked_tools}")
        }),
        json!({
            "stage": "degraded_lanes",
            "status": if degraded_tools == 0 { "pass" } else { "warn" },
            "detail": format!("degraded={degraded_tools}")
        }),
    ];
    let overall =
        if total_tools == 0 || (!overall_should_execute && blocked_tools >= total_tools && total_tools > 0)
        {
            "fail"
        } else if overall_status == "degraded" || blocked_tools > 0 || degraded_tools > 0 {
            "warn"
        } else {
            "pass"
        };
    json!({
        "version": "tool_policy_pipeline_v2",
        "overall": overall,
        "stages": stages
    })
}

fn runtime_web_tooling_process_summary(
    runtime_web_tools_metadata: &Value,
    effective_inventory: &Value,
    policy_pipeline: &Value,
) -> Value {
    let total_tools = effective_inventory
        .pointer("/summary/total_tools")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let enabled_tools = effective_inventory
        .pointer("/summary/enabled_tools")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let blocked_tools = effective_inventory
        .pointer("/summary/blocked_tools")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let degraded_tools = effective_inventory
        .pointer("/summary/degraded_tools")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let diagnostic_count = runtime_web_tools_metadata
        .get("diagnostics")
        .and_then(Value::as_array)
        .map(|rows| rows.len() as u64)
        .unwrap_or(0);
    let stage_count = policy_pipeline
        .get("stages")
        .and_then(Value::as_array)
        .map(|rows| rows.len() as u64)
        .unwrap_or(0);
    let overall_status = clean_text(
        runtime_web_tools_metadata
            .pointer("/tool_surface_health/status")
            .and_then(Value::as_str)
            .unwrap_or("unknown"),
        40,
    )
    .to_ascii_lowercase();
    let overall_should_execute = runtime_web_tools_metadata
        .pointer("/tool_execution_gate/overall_should_execute")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let policy_blocked_count = effective_inventory
        .pointer("/summary/blocking_error_codes")
        .and_then(Value::as_array)
        .map(|codes| {
            codes
                .iter()
                .filter(|code| code.as_str() == Some("web_tool_policy_blocked"))
                .count() as u64
        })
        .unwrap_or(0);
    json!({
        "type": "runtime_web_tooling_process_summary",
        "generated_at": crate::now_iso(),
        "overall_status": overall_status,
        "overall_should_execute": overall_should_execute,
        "tool_counts": {
            "total": total_tools,
            "enabled": enabled_tools,
            "blocked": blocked_tools,
            "degraded": degraded_tools
        },
        "diagnostic_count": diagnostic_count,
        "pipeline_stage_count": stage_count,
        "policy_blocked_count": policy_blocked_count,
        "dashboard_hints": {
            "usage": {"enabled": enabled_tools, "blocked": blocked_tools},
            "logs": {"diagnostics": diagnostic_count},
            "comms": {"surface_status": overall_status},
            "sessions": {"overall_should_execute": overall_should_execute},
            "workflows": {"pipeline_stages": stage_count},
            "runtime": {"effective_inventory_version": "effective_inventory_v2"},
            "approvals": {"policy_blocked": policy_blocked_count}
        }
    })
}

fn runtime_web_tooling_decision_trace(
    runtime_web_tools_metadata: &Value,
    effective_inventory: &Value,
    policy_pipeline: &Value,
) -> Value {
    let mut trace_rows: Vec<Value> = Vec::new();
    let mut blocked_tools = 0_u64;
    let mut executable_tools = 0_u64;
    let mut retryable_tools = 0_u64;
    if let Some(rows) = effective_inventory.get("rows").and_then(Value::as_array) {
        for row in rows.iter().take(10) {
            let tool_id = clean_text(row.get("tool_id").and_then(Value::as_str).unwrap_or(""), 120);
            if tool_id.is_empty() {
                continue;
            }
            let family = clean_text(row.get("family").and_then(Value::as_str).unwrap_or(""), 64);
            let should_execute = row
                .get("should_execute")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let execution_mode = clean_text(
                row.get("execution_mode")
                    .and_then(Value::as_str)
                    .unwrap_or("blocked"),
                40,
            )
            .to_ascii_lowercase();
            let surface_status = clean_text(
                row.get("surface_status")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown"),
                40,
            )
            .to_ascii_lowercase();
            let selected_provider = clean_text(
                row.get("selected_provider")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                80,
            );
            let blocking_error_code = clean_text(
                row.get("blocking_error_code")
                    .and_then(Value::as_str)
                    .unwrap_or("none"),
                80,
            )
            .to_ascii_lowercase();
            let blocking_reason = clean_text(
                row.get("blocking_reason")
                    .and_then(Value::as_str)
                    .unwrap_or("none"),
                120,
            )
            .to_ascii_lowercase();
            let retry_recommended = row
                .pointer("/retry_policy/recommended")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let retry_attempts = row
                .pointer("/retry_policy/attempts")
                .and_then(Value::as_u64)
                .unwrap_or(0);
            let retry_lane = clean_text(
                row.pointer("/retry_policy/lane")
                    .and_then(Value::as_str)
                    .unwrap_or("none"),
                64,
            );
            let next_action = if should_execute {
                "execute_tool"
            } else if retry_recommended {
                "retry_then_report"
            } else {
                "report_blocked"
            };
            if should_execute {
                executable_tools += 1;
            } else {
                blocked_tools += 1;
            }
            if retry_recommended {
                retryable_tools += 1;
            }
            trace_rows.push(json!({
                "tool_id": tool_id,
                "family": family,
                "execution_mode": execution_mode,
                "surface_status": surface_status,
                "selected_provider": selected_provider,
                "should_execute": should_execute,
                "blocking_error_code": blocking_error_code,
                "blocking_reason": blocking_reason,
                "retry_recommended": retry_recommended,
                "retry_attempts": retry_attempts,
                "retry_lane": retry_lane,
                "next_action": next_action
            }));
        }
    }
    let overall_should_execute = runtime_web_tools_metadata
        .pointer("/tool_execution_gate/overall_should_execute")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let pipeline_overall = clean_text(
        policy_pipeline
            .get("overall")
            .and_then(Value::as_str)
            .unwrap_or("unknown"),
        24,
    )
    .to_ascii_lowercase();
    let finalization_status = if !overall_should_execute || executable_tools == 0 {
        "blocked"
    } else if blocked_tools > 0 || pipeline_overall == "warn" {
        "degraded"
    } else {
        "ready"
    };
    let completion_signal = if finalization_status == "ready" {
        "tooling_ready"
    } else if finalization_status == "degraded" {
        "tooling_degraded_continue"
    } else {
        "tooling_blocked_report"
    };
    json!({
        "type": "runtime_web_tooling_decision_trace_v1",
        "generated_at": crate::now_iso(),
        "pipeline_overall": pipeline_overall,
        "overall_should_execute": overall_should_execute,
        "trace_rows": trace_rows,
        "summary": {
            "rows": trace_rows.len(),
            "executable_tools": executable_tools,
            "blocked_tools": blocked_tools,
            "retryable_tools": retryable_tools
        },
        "synthesis_contract": {
            "must_emit_user_response": true,
            "retry_budget": 1,
            "fallback_message_mode": "reliable_failure_report"
        },
        "finalization": {
            "status": finalization_status,
            "completion_signal": completion_signal
        }
    })
}
