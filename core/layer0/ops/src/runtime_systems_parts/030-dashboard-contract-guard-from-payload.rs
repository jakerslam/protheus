
fn dashboard_contract_guard_from_payload(payload: &Value) -> Value {
    let input_text = payload_string(payload, "input_text", "");
    let lowered = input_text.to_ascii_lowercase();
    let recent_messages = payload_u64(payload, "recent_messages", 0).min(2_000_000);
    let max_per_min =
        payload_u64(payload, "rogue_message_rate_max_per_min", 20).clamp(1, 1_000_000);

    let contains_any = |terms: &[&str]| -> bool { terms.iter().any(|term| lowered.contains(term)) };

    let mut reason = String::new();
    let mut detail = String::new();
    if contains_any(&["ignore", "bypass", "disable", "override"])
        && contains_any(&["contract", "safety", "receipt", "policy"])
    {
        reason = "contract_override_attempt".to_string();
        detail = "input_requested_contract_bypass".to_string();
    } else if contains_any(&["exfiltrate", "steal", "dump secrets", "leak", "data exfil"]) {
        reason = "data_exfiltration_attempt".to_string();
        detail = "input_requested_exfiltration".to_string();
    } else if contains_any(&["extend", "increase"])
        && contains_any(&["expiry", "ttl", "time to live", "contract"])
    {
        reason = "self_extension_attempt".to_string();
        detail = "input_requested_expiry_extension".to_string();
    } else if recent_messages > max_per_min {
        reason = "message_rate_spike".to_string();
        detail = format!("recent_messages={recent_messages}");
    }

    json!({
        "authority": "rust_runtime_systems",
        "policy": "V6-DASHBOARD-007.3",
        "violation": !reason.is_empty(),
        "reason": reason,
        "detail": detail,
        "recent_messages": recent_messages,
        "rogue_message_rate_max_per_min": max_per_min,
        "input_sha256": sha256_hex(input_text.as_bytes())
    })
}

fn dashboard_auto_route_from_payload(payload: &Value) -> Value {
    let input_text = payload_string(payload, "input_text", "");
    let lowered = input_text.to_ascii_lowercase();
    let token_count = payload_u64(payload, "token_count", (input_text.len() as u64 / 4).max(1))
        .clamp(1, 8_000_000);
    let has_vision = payload_bool(payload, "has_vision", false);
    let asks_speed = payload_bool(
        payload,
        "asks_speed",
        lowered.contains("fast") || lowered.contains("speed"),
    );
    let asks_cost = payload_bool(
        payload,
        "asks_cost",
        lowered.contains("cheap") || lowered.contains("cost"),
    );
    let asks_quality = payload_bool(
        payload,
        "asks_quality",
        lowered.contains("quality") || lowered.contains("best"),
    );
    let asks_long_context = payload_bool(
        payload,
        "asks_long_context",
        token_count >= 100_000 || lowered.contains("long context"),
    );

    let preferred_provider = payload_string(payload, "preferred_provider", "ollama");
    let preferred_model = payload_string(payload, "preferred_model", "llama3.2:3b");
    let fallback_provider = payload_string(payload, "fallback_provider", "cloud");
    let fallback_model = payload_string(payload, "fallback_model", "kimi2.5:cloud");

    let mut raw_candidates = payload_array(payload, "candidates");
    if raw_candidates.is_empty() {
        raw_candidates.push(json!({
            "runtime_provider": preferred_provider,
            "runtime_model": preferred_model
        }));
        raw_candidates.push(json!({
            "runtime_provider": fallback_provider,
            "runtime_model": fallback_model
        }));
    }

    let runtime_success = payload_f64(payload, "spine_success_rate", 0.90).clamp(0.0, 1.0);
    let mut scored = Vec::<Value>::new();
    for candidate in raw_candidates {
        let provider = payload_string(
            &candidate,
            "runtime_provider",
            payload_string(&candidate, "provider", "ollama").as_str(),
        );
        let model = payload_string(
            &candidate,
            "runtime_model",
            payload_string(&candidate, "model", "llama3.2:3b").as_str(),
        );
        let model_lower = model.to_ascii_lowercase();
        let (prior_latency, prior_cost, prior_success): (f64, f64, f64) =
            match provider.to_ascii_lowercase().as_str() {
                "ollama" => (120.0_f64, 0.0_f64, 0.92_f64),
                "groq" => (65.0_f64, 0.2_f64, 0.90_f64),
                "openai" => (90.0_f64, 0.55_f64, 0.95_f64),
                "anthropic" => (105.0_f64, 0.7_f64, 0.95_f64),
                "google" => (95.0_f64, 0.6_f64, 0.94_f64),
                "cloud" => (80.0_f64, 0.3_f64, 0.93_f64),
                _ => (110.0_f64, 0.45_f64, 0.90_f64),
            };
        let model_is_small = model_lower.contains("3b")
            || model_lower.contains("mini")
            || model_lower.contains("small");
        let latency_ms =
            (prior_latency * if model_is_small { 0.85_f64 } else { 1.0_f64 }).max(1.0_f64);
        let cost_per_1k =
            (prior_cost * if model_is_small { 0.7_f64 } else { 1.0_f64 }).max(0.0_f64);
        let context_window = candidate
            .get("context_window")
            .and_then(Value::as_u64)
            .unwrap_or(8192)
            .clamp(1024, 8_000_000);
        let context_score = if token_count <= context_window {
            1.0
        } else {
            (context_window as f64 / token_count as f64).clamp(0.1, 1.0)
        };
        let latency_score = 1.0 / (1.0 + (latency_ms / 120.0));
        let cost_score = 1.0 / (1.0 + cost_per_1k);
        let success_rate = ((prior_success * 0.65) + (runtime_success * 0.35)).clamp(0.2, 0.99);
        let supports_vision = candidate
            .get("supports_vision")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let vision_penalty = if has_vision && !supports_vision {
            0.55
        } else {
            0.0
        };
        let speed_weight = if asks_speed { 1.55 } else { 1.05 };
        let cost_weight = if asks_cost { 1.35 } else { 0.75 };
        let quality_weight = if asks_quality { 1.8 } else { 1.3 };
        let context_weight = if asks_long_context { 1.45 } else { 1.1 };
        let score = (latency_score * speed_weight)
            + (cost_score * cost_weight)
            + (success_rate * quality_weight)
            + (context_score * context_weight)
            - vision_penalty;
        scored.push(json!({
            "provider": provider,
            "model": model,
            "score": (score * 1_000_000.0).round() / 1_000_000.0,
            "latency_ms": latency_ms.round() as u64,
            "cost_per_1k": ((cost_per_1k * 10_000.0).round()) / 10_000.0,
            "success_rate": (success_rate * 10_000.0).round() / 10_000.0,
            "context_window": context_window,
            "supports_vision": supports_vision
        }));
    }

    scored.sort_by(|a, b| {
        let lhs = b.get("score").and_then(Value::as_f64).unwrap_or(0.0);
        let rhs = a.get("score").and_then(Value::as_f64).unwrap_or(0.0);
        lhs.partial_cmp(&rhs).unwrap_or(std::cmp::Ordering::Equal)
    });

    let selected = scored.first().cloned().unwrap_or_else(|| {
        json!({
            "provider": preferred_provider,
            "model": preferred_model,
            "score": 0.0,
            "latency_ms": 120,
            "cost_per_1k": 0.0,
            "success_rate": 0.9,
            "context_window": 8192,
            "supports_vision": false
        })
    });
    let selected_provider = payload_string(&selected, "provider", "ollama");
    let selected_model = payload_string(&selected, "model", "llama3.2:3b");
    let selected_context_window = selected
        .get("context_window")
        .and_then(Value::as_u64)
        .unwrap_or(8192);
    let reason = format!(
        "rust auto-route selected {} / {} by weighted latency-cost-success-context scoring",
        selected_provider, selected_model
    );
    let fallback_chain = scored
        .iter()
        .skip(1)
        .take(3)
        .map(|row| {
            json!({
                "provider": payload_string(row, "provider", ""),
                "model": payload_string(row, "model", ""),
                "score": row.get("score").and_then(Value::as_f64).unwrap_or(0.0)
            })
        })
        .collect::<Vec<_>>();

    let mut decision = json!({
        "authority": "rust_runtime_systems",
        "policy": "V6-DASHBOARD-008.1",
        "route_lane": "runtime-systems.run",
        "selected_provider": selected_provider,
        "selected_model": selected_model,
        "selected_model_id": format!(
            "{}/{}",
            payload_string(&selected, "provider", "ollama"),
            payload_string(&selected, "model", "llama3.2:3b")
        ),
        "selected_context_window": selected_context_window,
        "reason": reason,
        "context": {
            "token_count": token_count,
            "has_vision": has_vision,
            "asks_speed": asks_speed,
            "asks_cost": asks_cost,
            "asks_quality": asks_quality,
            "asks_long_context": asks_long_context
        },
        "fallback_chain": fallback_chain,
        "candidates": scored,
        "runtime_sync": {
            "spine_success_rate": runtime_success,
            "receipt_latency_p99_ms": payload_f64(payload, "receipt_latency_p99_ms", 0.0).max(0.0)
        }
    });
    let hash = receipt_hash(&decision);
    decision["receipt_hash"] = Value::String(hash);
    decision
}

fn payload_string_list(payload: &Value, key: &str) -> Vec<String> {
    payload_array(payload, key)
        .into_iter()
        .map(|row| match row {
            Value::String(text) => text,
            other => other
                .as_str()
                .map(|text| text.to_string())
                .unwrap_or_default(),
        })
        .filter(|row| !row.trim().is_empty())
        .map(|row| row.to_ascii_lowercase())
        .collect::<Vec<_>>()
}

fn dashboard_message_stack_guard_from_payload(payload: &Value) -> (Value, Vec<String>) {
    let metadata_hover_scope = payload_string(payload, "metadata_hover_scope", "message_only");
    let hover_pushdown_layout_enabled = payload_bool(payload, "hover_pushdown_layout_enabled", true);
    let stack_interrupts_on_notifications =
        payload_bool(payload, "stack_interrupts_on_notifications", true);
    let messages = payload_array(payload, "messages");
    let mut previous_key = String::new();
    let mut source_runs = 0u64;
    let mut notification_rows = 0u64;
    for row in messages.iter() {
        let source = payload_string(
            row,
            "source",
            payload_string(row, "role", "unknown").as_str(),
        );
        let kind = payload_string(row, "kind", "message").to_ascii_lowercase();
        let is_notification = kind.contains("notification")
            || kind.contains("notice")
            || kind.contains("name_changed")
            || kind.contains("model_changed")
            || kind.contains("system_event");
        if is_notification {
            notification_rows = notification_rows.saturating_add(1);
        }
        let key = if is_notification {
            format!("notification::{source}")
        } else {
            format!("source::{source}")
        };
        if key != previous_key {
            source_runs = source_runs.saturating_add(1);
            previous_key = key;
        }
    }
    let expected_min_source_runs = payload_u64(
        payload,
        "expected_min_source_runs",
        if messages.is_empty() {
            0
        } else if notification_rows > 0 {
            2
        } else {
            1
        },
    );
    let mut violations = Vec::<String>::new();
    if metadata_hover_scope != "message_only" {
        violations.push(format!(
            "specific_dashboard_metadata_hover_scope_mismatch:{metadata_hover_scope}"
        ));
    }
    if !hover_pushdown_layout_enabled {
        violations.push("specific_dashboard_metadata_pushdown_disabled".to_string());
    }
    if notification_rows > 0 && !stack_interrupts_on_notifications {
        violations.push("specific_dashboard_notifications_must_interrupt_stack".to_string());
    }
    if source_runs < expected_min_source_runs {
        violations.push(format!(
            "specific_dashboard_source_run_count_too_low:{source_runs}<{}",
            expected_min_source_runs
        ));
    }

    (
        json!({
            "authority": "rust_runtime_systems",
            "policy": "V6-DASHBOARD-009.1",
            "metadata_hover_scope": metadata_hover_scope,
            "hover_pushdown_layout_enabled": hover_pushdown_layout_enabled,
            "stack_interrupts_on_notifications": stack_interrupts_on_notifications,
            "source_run_count": source_runs,
            "expected_min_source_runs": expected_min_source_runs,
            "notification_rows": notification_rows,
            "messages_seen": messages.len()
        }),
        violations,
    )
}

fn dashboard_boot_retry_guard_from_payload(payload: &Value) -> (Value, Vec<String>) {
    let boot_retry_enabled = payload_bool(payload, "boot_retry_enabled", true);
    let boot_retry_max_attempts = payload_u64(payload, "boot_retry_max_attempts", 5).clamp(1, 20);
    let boot_retry_backoff_ms = payload_u64(payload, "boot_retry_backoff_ms", 1000).clamp(1, 60_000);
    let startup_failed = payload_bool(payload, "startup_failed", false);
    let server_status_emitted = payload_bool(payload, "server_status_emitted", !startup_failed);
    let server_status_path = payload_string(
        payload,
        "server_status_path",
        "local/state/ops/daemon_control/server_status.json",
    );
    let status_error_code = payload_string(payload, "status_error_code", "");
    let mut violations = Vec::<String>::new();
    if !boot_retry_enabled {
        violations.push("specific_dashboard_boot_retry_disabled".to_string());
    }
    if boot_retry_max_attempts < 2 {
        violations.push(format!(
            "specific_dashboard_boot_retry_attempts_too_low:{boot_retry_max_attempts}"
        ));
    }
    if boot_retry_backoff_ms < 1000 {
        violations.push(format!(
            "specific_dashboard_boot_retry_backoff_too_low:{boot_retry_backoff_ms}"
        ));
    }
    if startup_failed && !server_status_emitted {
        violations.push("specific_dashboard_server_status_missing_on_failure".to_string());
    }
    if startup_failed && status_error_code.trim().is_empty() {
        violations.push("specific_dashboard_failure_missing_error_code".to_string());
    }
    if startup_failed && server_status_path.trim().is_empty() {
        violations.push("specific_dashboard_failure_missing_status_path".to_string());
    }

    (
        json!({
            "authority": "rust_runtime_systems",
            "policy": "V6-DASHBOARD-009.2",
            "boot_retry_enabled": boot_retry_enabled,
            "boot_retry_max_attempts": boot_retry_max_attempts,
            "boot_retry_backoff_ms": boot_retry_backoff_ms,
            "startup_failed": startup_failed,
            "server_status_emitted": server_status_emitted,
            "server_status_path": server_status_path,
            "status_error_code": status_error_code
        }),
        violations,
    )
}

fn infring_gap_guard_from_payload(contract_id: &str, payload: &Value) -> (Value, Vec<String>) {
    let mut violations = Vec::<String>::new();
    let mut check = json!({
        "authority": "rust_runtime_systems",
        "policy": contract_id
    });
    match contract_id {
        "V6-INFRING-GAP-001.1" => {
            let provider_agnostic_driver_enabled =
                payload_bool(payload, "provider_agnostic_driver_enabled", true);
            let context_budget_compaction_enabled =
                payload_bool(payload, "context_budget_compaction_enabled", true);
            let llm_driver_registry_count = payload_u64(payload, "llm_driver_registry_count", 1);
            if !provider_agnostic_driver_enabled {
                violations.push("specific_infring_gap_driver_layer_disabled".to_string());
            }
            if !context_budget_compaction_enabled {
                violations.push("specific_infring_gap_context_budget_compaction_disabled".to_string());
            }
            if llm_driver_registry_count < 1 {
                violations.push("specific_infring_gap_driver_registry_empty".to_string());
            }
            check["provider_agnostic_driver_enabled"] = json!(provider_agnostic_driver_enabled);
            check["context_budget_compaction_enabled"] = json!(context_budget_compaction_enabled);
            check["llm_driver_registry_count"] = json!(llm_driver_registry_count);
        }
        "V6-INFRING-GAP-001.2" => {
            let http_api_endpoints_count = payload_u64(payload, "http_api_endpoints_count", 0);
            let websocket_streaming_enabled =
                payload_bool(payload, "websocket_streaming_enabled", false);
            if http_api_endpoints_count < 1 {
                violations.push("specific_infring_gap_http_endpoint_count_too_low".to_string());
            }
            if !websocket_streaming_enabled {
                violations.push("specific_infring_gap_websocket_streaming_disabled".to_string());
            }
            check["http_api_endpoints_count"] = json!(http_api_endpoints_count);
            check["websocket_streaming_enabled"] = json!(websocket_streaming_enabled);
        }
        "V6-INFRING-GAP-001.3" => {
            let channel_adapters = payload_string_list(payload, "channel_adapters");
            let required = ["slack", "matrix", "email", "whatsapp"];
            let missing = required
                .iter()
                .filter(|target| !channel_adapters.iter().any(|row| row == *target))
                .cloned()
                .collect::<Vec<_>>();
            if !missing.is_empty() {
                violations.push(format!(
                    "specific_infring_gap_channel_adapters_missing:{}",
                    missing.join("|")
                ));
            }
            check["channel_adapters"] = json!(channel_adapters);
            check["required_adapters"] = json!(required);
            check["missing_adapters"] = json!(missing);
        }
        "V6-INFRING-GAP-001.4" => {
            let taint_tracking_enabled = payload_bool(payload, "taint_tracking_enabled", false);
            let merkle_audit_chain_enabled =
                payload_bool(payload, "merkle_audit_chain_enabled", false);
            let manifest_signing_enabled = payload_bool(payload, "manifest_signing_enabled", false);
            let ssrf_deny_paths_enabled = payload_bool(payload, "ssrf_deny_paths_enabled", false);
            if !taint_tracking_enabled {
                violations.push("specific_infring_gap_taint_tracking_disabled".to_string());
            }
            if !merkle_audit_chain_enabled {
                violations.push("specific_infring_gap_merkle_audit_chain_disabled".to_string());
            }
            if !manifest_signing_enabled {
                violations.push("specific_infring_gap_manifest_signing_disabled".to_string());
            }
            if !ssrf_deny_paths_enabled {
                violations.push("specific_infring_gap_ssrf_deny_paths_disabled".to_string());
            }
            check["taint_tracking_enabled"] = json!(taint_tracking_enabled);
            check["merkle_audit_chain_enabled"] = json!(merkle_audit_chain_enabled);
            check["manifest_signing_enabled"] = json!(manifest_signing_enabled);
            check["ssrf_deny_paths_enabled"] = json!(ssrf_deny_paths_enabled);
        }
        "V6-INFRING-GAP-001.5" => {
            let hands_registry_enabled = payload_bool(payload, "hands_registry_enabled", false);
            let skills_promotion_pipeline_enabled =
                payload_bool(payload, "skills_promotion_pipeline_enabled", false);
            let hands_fail_closed_enabled = payload_bool(payload, "hands_fail_closed_enabled", false);
            if !hands_registry_enabled {
                violations.push("specific_infring_gap_hands_registry_disabled".to_string());
            }
            if !skills_promotion_pipeline_enabled {
                violations.push("specific_infring_gap_skills_promotion_pipeline_disabled".to_string());
            }
            if !hands_fail_closed_enabled {
                violations.push("specific_infring_gap_hands_fail_closed_disabled".to_string());
            }
            check["hands_registry_enabled"] = json!(hands_registry_enabled);
            check["skills_promotion_pipeline_enabled"] = json!(skills_promotion_pipeline_enabled);
            check["hands_fail_closed_enabled"] = json!(hands_fail_closed_enabled);
        }
        _ => {}
    }
    (check, violations)
}

fn duality_guard_from_payload(contract_id: &str, payload: &Value) -> (Value, Vec<String>) {
    let mut violations = Vec::<String>::new();
    let mut check = json!({
        "authority": "rust_runtime_systems",
        "policy": contract_id
    });
    match contract_id {
        "V4-DUAL-CON-001" => {
            let duality_bundle_emitted = payload_bool(payload, "duality_bundle_emitted", true);
            let harmony_score = payload_f64(payload, "harmony_score", 0.92);
            if !duality_bundle_emitted {
                violations.push("specific_duality_bundle_missing".to_string());
            }
            if !(0.0..=1.0).contains(&harmony_score) {
                violations.push(format!("specific_duality_harmony_score_invalid:{harmony_score}"));
            }
            check["duality_bundle_emitted"] = json!(duality_bundle_emitted);
            check["harmony_score"] = json!(harmony_score);
        }
        "V4-DUAL-CON-002" => {
            let toll_prediction_enabled = payload_bool(payload, "toll_prediction_enabled", true);
            let imbalance_debt = payload_f64(payload, "imbalance_debt", 0.03);
            if !toll_prediction_enabled {
                violations.push("specific_duality_toll_prediction_disabled".to_string());
            }
            if !(0.0..=1.0).contains(&imbalance_debt) {
                violations.push(format!("specific_duality_imbalance_debt_invalid:{imbalance_debt}"));
            }
            check["toll_prediction_enabled"] = json!(toll_prediction_enabled);
            check["imbalance_debt"] = json!(imbalance_debt);
        }
        "V4-DUAL-CON-003" => {
            let fractal_balance_score = payload_f64(payload, "fractal_balance_score", 0.89);
            let macro_composition_enabled = payload_bool(payload, "macro_composition_enabled", true);
            if !macro_composition_enabled {
                violations.push("specific_duality_macro_composition_disabled".to_string());
            }
            if !(0.0..=1.0).contains(&fractal_balance_score) {
                violations.push(format!(
                    "specific_duality_fractal_balance_score_invalid:{fractal_balance_score}"
                ));
            }
            check["fractal_balance_score"] = json!(fractal_balance_score);
            check["macro_composition_enabled"] = json!(macro_composition_enabled);
        }
        "V4-DUAL-MEM-002" => {
            let dual_memory_tagging_enabled = payload_bool(payload, "dual_memory_tagging_enabled", true);
            let inversion_candidate_tagging_enabled =
                payload_bool(payload, "inversion_candidate_tagging_enabled", true);
            if !dual_memory_tagging_enabled {
                violations.push("specific_duality_memory_tagging_disabled".to_string());
            }
            if !inversion_candidate_tagging_enabled {
                violations.push("specific_duality_inversion_tagging_disabled".to_string());
            }
            check["dual_memory_tagging_enabled"] = json!(dual_memory_tagging_enabled);
            check["inversion_candidate_tagging_enabled"] = json!(inversion_candidate_tagging_enabled);
        }
        _ => {}
    }
    (check, violations)
}

fn perf_guard_from_payload(contract_id: &str, payload: &Value) -> (Value, Vec<String>) {
    let mut violations = Vec::<String>::new();
    let mut check = json!({
        "authority": "rust_runtime_systems",
        "policy": contract_id
    });
    match contract_id {
        "V10-PERF-001.1" => {
            let receipt_batching_enabled = payload_bool(payload, "receipt_batching_enabled", false);
            let receipt_batch_size = payload_u64(payload, "receipt_batch_size", 1);
            if !receipt_batching_enabled {
                violations.push("specific_perf_receipt_batching_disabled".to_string());
            }
            if !(8..=64).contains(&receipt_batch_size) {
                violations.push(format!(
                    "specific_perf_receipt_batch_size_out_of_range:{receipt_batch_size}"
                ));
            }
            check["receipt_batching_enabled"] = json!(receipt_batching_enabled);
            check["receipt_batch_size"] = json!(receipt_batch_size);
        }
        "V10-PERF-001.2" => {
            let simd_hotpaths_enabled = payload_bool(payload, "simd_hotpaths_enabled", false);
            let simd_profile = payload_string(payload, "simd_profile", "");
            let profile_valid = matches!(simd_profile.as_str(), "avx2" | "neon" | "portable");
            if !simd_hotpaths_enabled {
                violations.push("specific_perf_simd_hotpaths_disabled".to_string());
            }
            if !profile_valid {
                violations.push(format!("specific_perf_simd_profile_invalid:{simd_profile}"));
            }
            check["simd_hotpaths_enabled"] = json!(simd_hotpaths_enabled);
            check["simd_profile"] = json!(simd_profile);
        }
        "V10-PERF-001.3" => {
            let lock_free_coordination_enabled =
                payload_bool(payload, "lock_free_coordination_enabled", false);
            let coordination_contention_ratio =
                payload_f64(payload, "coordination_contention_ratio", 1.0).clamp(0.0, 1.0);
            if !lock_free_coordination_enabled {
                violations.push("specific_perf_lock_free_coordination_disabled".to_string());
            }
            if coordination_contention_ratio > 0.25 {
                violations.push(format!(
                    "specific_perf_coordination_contention_ratio_high:{coordination_contention_ratio:.4}"
                ));
            }
            check["lock_free_coordination_enabled"] = json!(lock_free_coordination_enabled);
            check["coordination_contention_ratio"] = json!(coordination_contention_ratio);
        }
        "V10-PERF-001.4" => {
            let pgo_enabled = payload_bool(payload, "pgo_enabled", false);
            let lto_enabled = payload_bool(payload, "lto_enabled", false);
            if !pgo_enabled {
                violations.push("specific_perf_pgo_disabled".to_string());
            }
            if !lto_enabled {
                violations.push("specific_perf_lto_disabled".to_string());
            }
            check["pgo_enabled"] = json!(pgo_enabled);
            check["lto_enabled"] = json!(lto_enabled);
        }
        "V10-PERF-001.5" => {
            let hierarchy_slab_allocator_enabled =
                payload_bool(payload, "hierarchy_slab_allocator_enabled", false);
            let memory_fragmentation_percent =
                payload_f64(payload, "memory_fragmentation_percent", 100.0).max(0.0);
            if !hierarchy_slab_allocator_enabled {
                violations.push("specific_perf_hierarchy_slab_allocator_disabled".to_string());
            }
            if memory_fragmentation_percent > 10.0 {
                violations.push(format!(
                    "specific_perf_memory_fragmentation_high:{memory_fragmentation_percent:.2}"
                ));
            }
            check["hierarchy_slab_allocator_enabled"] = json!(hierarchy_slab_allocator_enabled);
            check["memory_fragmentation_percent"] = json!(memory_fragmentation_percent);
        }
        "V10-PERF-001.6" => {
            let throughput_regression_guard_enabled =
                payload_bool(payload, "throughput_regression_guard_enabled", false);
            let throughput_drop_threshold_pct =
                payload_f64(payload, "throughput_drop_threshold_pct", 100.0).max(0.0);
            if !throughput_regression_guard_enabled {
                violations.push("specific_perf_throughput_regression_guard_disabled".to_string());
            }
            if throughput_drop_threshold_pct > 5.0 {
                violations.push(format!(
                    "specific_perf_throughput_drop_threshold_too_high:{throughput_drop_threshold_pct:.2}"
                ));
            }
            check["throughput_regression_guard_enabled"] = json!(throughput_regression_guard_enabled);
            check["throughput_drop_threshold_pct"] = json!(throughput_drop_threshold_pct);
        }
        _ => {}
    }
    (check, violations)
}

fn contract_specific_gates(
    profile: RuntimeSystemContractProfile,
    payload: &Value,
) -> (serde_json::Map<String, Value>, Vec<String>) {
    let mut checks = serde_json::Map::new();
    let mut violations = Vec::<String>::new();

    match profile.id {
        "V9-AUDIT-026.1" => {
            let targets = payload_string_array(
                payload,
                "audit_targets",
                &[
                    "origin_integrity",
                    "supply_chain_provenance_v2",
                    "alpha_readiness",
                ],
            );
            let missing = missing_required_tokens(
                &targets,
                &[
                    "origin_integrity",
                    "supply_chain_provenance_v2",
                    "alpha_readiness",
                ],
            );
            checks.insert("audit_targets".to_string(), json!(targets));
            checks.insert("audit_targets_missing".to_string(), json!(missing));
            if !missing.is_empty() {
                violations.push(format!(
                    "specific_missing_audit_targets:{}",
                    missing.join("|")
                ));
            }
        }
        "V9-AUDIT-026.2" => {
            let actions = payload_string_array(
                payload,
                "self_healing_actions",
                &[
                    "refresh_spine_receipt",
                    "rebuild_supply_chain_bundle",
                    "reconcile_workspace_churn",
                ],
            );
            let missing = missing_required_tokens(
                &actions,
                &[
                    "refresh_spine_receipt",
                    "rebuild_supply_chain_bundle",
                    "reconcile_workspace_churn",
                ],
            );
            checks.insert("self_healing_actions".to_string(), json!(actions));
            checks.insert("self_healing_actions_missing".to_string(), json!(missing));
            if !missing.is_empty() {
                violations.push(format!(
                    "specific_missing_self_healing_actions:{}",
                    missing.join("|")
                ));
            }
        }
        "V9-AUDIT-026.3" => {
            let range = payload_string(payload, "confidence_range", "0.0-1.0");
            checks.insert("confidence_range".to_string(), json!(range.clone()));
            if range != "0.0-1.0" {
                violations.push(format!("specific_confidence_range_mismatch:{range}"));
            }
        }
        "V9-AUDIT-026.4" => {
            let consensus = payload_string(payload, "consensus_mode", "strict_match");
            checks.insert("consensus_mode".to_string(), json!(consensus.clone()));
            if consensus != "strict_match" {
                violations.push(format!("specific_consensus_mode_mismatch:{consensus}"));
            }
        }
        "V6-DASHBOARD-007.3" => {
            checks.insert(
                "dashboard_contract_guard".to_string(),
                dashboard_contract_guard_from_payload(payload),
            );
        }
        _ if profile.id.starts_with("V6-DASHBOARD-007.") => {
            checks.insert(
                "dashboard_runtime_authority".to_string(),
                dashboard_runtime_authority_from_payload(payload),
            );
        }
        _ if profile.id.starts_with("V6-DASHBOARD-008.") => {
            checks.insert(
                "dashboard_auto_route_authority".to_string(),
                dashboard_auto_route_from_payload(payload),
            );
        }
        "V6-DASHBOARD-009.1" => {
            let (check, mut check_violations) = dashboard_message_stack_guard_from_payload(payload);
            checks.insert("dashboard_message_stack_guard".to_string(), check);
            violations.append(&mut check_violations);
        }
        "V6-DASHBOARD-009.2" => {
            let (check, mut check_violations) = dashboard_boot_retry_guard_from_payload(payload);
            checks.insert("dashboard_boot_retry_guard".to_string(), check);
            violations.append(&mut check_violations);
        }
        _ if profile.id.starts_with("V6-INFRING-GAP-001.") => {
            let (check, mut check_violations) = infring_gap_guard_from_payload(profile.id, payload);
            checks.insert("infring_gap_guard".to_string(), check);
            violations.append(&mut check_violations);
        }
        _ if profile.id.starts_with("V10-PERF-001.") => {
            let (check, mut check_violations) = perf_guard_from_payload(profile.id, payload);
            checks.insert("perf_guard".to_string(), check);
            violations.append(&mut check_violations);
        }
        _ if profile.id.starts_with("V4-DUAL-CON-") || profile.id.starts_with("V4-DUAL-MEM-") => {
            let (check, mut check_violations) = duality_guard_from_payload(profile.id, payload);
            checks.insert("duality_guard".to_string(), check);
            violations.append(&mut check_violations);
        }
        _ => {}
    }

    (checks, violations)
}

fn count_lines(path: &Path) -> u64 {
    fs::read_to_string(path)
        .ok()
        .map(|raw| raw.lines().count() as u64)
        .unwrap_or(0)
}

fn collect_repo_language_lines(dir: &Path, rs_lines: &mut u64, ts_lines: &mut u64) {
    let Ok(read) = fs::read_dir(dir) else {
        return;
    };
    for entry in read.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|v| v.to_str()) else {
            continue;
        };
        if path.is_dir() {
            if matches!(
                name,
                ".git"
                    | "target"
                    | "node_modules"
                    | "dist"
                    | "build"
                    | "coverage"
                    | "tmp"
                    | "local"
            ) {
                continue;
            }
            collect_repo_language_lines(&path, rs_lines, ts_lines);
            continue;
        }
        if name.ends_with(".rs") {
            *rs_lines += count_lines(&path);
        } else if name.ends_with(".ts") {
            *ts_lines += count_lines(&path);
        }
    }
}

fn repo_language_share(root: &Path) -> (u64, u64, f64) {
    let mut rs_lines = 0u64;
    let mut ts_lines = 0u64;
    collect_repo_language_lines(root, &mut rs_lines, &mut ts_lines);
    let total = rs_lines.saturating_add(ts_lines);
    let rust_share_pct = if total == 0 {
        0.0
    } else {
        (rs_lines as f64) * 100.0 / (total as f64)
    };
    (rs_lines, ts_lines, rust_share_pct)
}

#[derive(Debug, Clone)]
struct ContractExecution {
    summary: Value,
    claims: Vec<Value>,
    artifacts: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
struct FamilyContractRequirements {
    required_true: &'static [&'static str],
    min_values: &'static [(&'static str, f64)],
    max_values: &'static [(&'static str, f64)],
}

const EMPTY_REQUIRED_TRUE: &[&str] = &[];
const EMPTY_NUM_GATES: &[(&str, f64)] = &[];
