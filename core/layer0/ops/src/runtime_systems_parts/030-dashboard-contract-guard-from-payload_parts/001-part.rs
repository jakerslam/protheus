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
            push_if_disabled(
                &mut violations,
                provider_agnostic_driver_enabled,
                "specific_infring_gap_driver_layer_disabled",
            );
            push_if_disabled(
                &mut violations,
                context_budget_compaction_enabled,
                "specific_infring_gap_context_budget_compaction_disabled",
            );
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
            push_if_disabled(
                &mut violations,
                websocket_streaming_enabled,
                "specific_infring_gap_websocket_streaming_disabled",
            );
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
            push_if_disabled(
                &mut violations,
                taint_tracking_enabled,
                "specific_infring_gap_taint_tracking_disabled",
            );
            push_if_disabled(
                &mut violations,
                merkle_audit_chain_enabled,
                "specific_infring_gap_merkle_audit_chain_disabled",
            );
            push_if_disabled(
                &mut violations,
                manifest_signing_enabled,
                "specific_infring_gap_manifest_signing_disabled",
            );
            push_if_disabled(
                &mut violations,
                ssrf_deny_paths_enabled,
                "specific_infring_gap_ssrf_deny_paths_disabled",
            );
            check["taint_tracking_enabled"] = json!(taint_tracking_enabled);
            check["merkle_audit_chain_enabled"] = json!(merkle_audit_chain_enabled);
            check["manifest_signing_enabled"] = json!(manifest_signing_enabled);
            check["ssrf_deny_paths_enabled"] = json!(ssrf_deny_paths_enabled);
        }
        "V6-INFRING-GAP-001.5" => {
            let hands_registry_enabled = payload_bool(payload, "hands_registry_enabled", false);
            let skills_promotion_pipeline_enabled =
                payload_bool(payload, "skills_promotion_pipeline_enabled", false);
            let hands_fail_closed_enabled =
                payload_bool(payload, "hands_fail_closed_enabled", false);
            push_if_disabled(
                &mut violations,
                hands_registry_enabled,
                "specific_infring_gap_hands_registry_disabled",
            );
            push_if_disabled(
                &mut violations,
                skills_promotion_pipeline_enabled,
                "specific_infring_gap_skills_promotion_pipeline_disabled",
            );
            push_if_disabled(
                &mut violations,
                hands_fail_closed_enabled,
                "specific_infring_gap_hands_fail_closed_disabled",
            );
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
            push_if_disabled(
                &mut violations,
                duality_bundle_emitted,
                "specific_duality_bundle_missing",
            );
            if !(0.0..=1.0).contains(&harmony_score) {
                violations.push(format!(
                    "specific_duality_harmony_score_invalid:{harmony_score}"
                ));
            }
            check["duality_bundle_emitted"] = json!(duality_bundle_emitted);
            check["harmony_score"] = json!(harmony_score);
        }
        "V4-DUAL-CON-002" => {
            let toll_prediction_enabled = payload_bool(payload, "toll_prediction_enabled", true);
            let imbalance_debt = payload_f64(payload, "imbalance_debt", 0.03);
            push_if_disabled(
                &mut violations,
                toll_prediction_enabled,
                "specific_duality_toll_prediction_disabled",
            );
            if !(0.0..=1.0).contains(&imbalance_debt) {
                violations.push(format!(
                    "specific_duality_imbalance_debt_invalid:{imbalance_debt}"
                ));
            }
            check["toll_prediction_enabled"] = json!(toll_prediction_enabled);
            check["imbalance_debt"] = json!(imbalance_debt);
        }
        "V4-DUAL-CON-003" => {
            let fractal_balance_score = payload_f64(payload, "fractal_balance_score", 0.89);
            let macro_composition_enabled =
                payload_bool(payload, "macro_composition_enabled", true);
            push_if_disabled(
                &mut violations,
                macro_composition_enabled,
                "specific_duality_macro_composition_disabled",
            );
            if !(0.0..=1.0).contains(&fractal_balance_score) {
                violations.push(format!(
                    "specific_duality_fractal_balance_score_invalid:{fractal_balance_score}"
                ));
            }
            check["fractal_balance_score"] = json!(fractal_balance_score);
            check["macro_composition_enabled"] = json!(macro_composition_enabled);
        }
        "V4-DUAL-MEM-002" => {
            let dual_memory_tagging_enabled =
                payload_bool(payload, "dual_memory_tagging_enabled", true);
            let inversion_candidate_tagging_enabled =
                payload_bool(payload, "inversion_candidate_tagging_enabled", true);
            push_if_disabled(
                &mut violations,
                dual_memory_tagging_enabled,
                "specific_duality_memory_tagging_disabled",
            );
            push_if_disabled(
                &mut violations,
                inversion_candidate_tagging_enabled,
                "specific_duality_inversion_tagging_disabled",
            );
            check["dual_memory_tagging_enabled"] = json!(dual_memory_tagging_enabled);
            check["inversion_candidate_tagging_enabled"] =
                json!(inversion_candidate_tagging_enabled);
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
            push_if_disabled(
                &mut violations,
                receipt_batching_enabled,
                "specific_perf_receipt_batching_disabled",
            );
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
            push_if_disabled(
                &mut violations,
                simd_hotpaths_enabled,
                "specific_perf_simd_hotpaths_disabled",
            );
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
            push_if_disabled(
                &mut violations,
                lock_free_coordination_enabled,
                "specific_perf_lock_free_coordination_disabled",
            );
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
            push_if_disabled(&mut violations, pgo_enabled, "specific_perf_pgo_disabled");
            push_if_disabled(&mut violations, lto_enabled, "specific_perf_lto_disabled");
            check["pgo_enabled"] = json!(pgo_enabled);
            check["lto_enabled"] = json!(lto_enabled);
        }
        "V10-PERF-001.5" => {
            let hierarchy_slab_allocator_enabled =
                payload_bool(payload, "hierarchy_slab_allocator_enabled", false);
            let memory_fragmentation_percent =
                payload_f64(payload, "memory_fragmentation_percent", 100.0).max(0.0);
            push_if_disabled(
                &mut violations,
                hierarchy_slab_allocator_enabled,
                "specific_perf_hierarchy_slab_allocator_disabled",
            );
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
            push_if_disabled(
                &mut violations,
                throughput_regression_guard_enabled,
                "specific_perf_throughput_regression_guard_disabled",
            );
            if throughput_drop_threshold_pct > 5.0 {
                violations.push(format!(
                    "specific_perf_throughput_drop_threshold_too_high:{throughput_drop_threshold_pct:.2}"
                ));
            }
            check["throughput_regression_guard_enabled"] =
                json!(throughput_regression_guard_enabled);
            check["throughput_drop_threshold_pct"] = json!(throughput_drop_threshold_pct);
        }
        _ => {}
    }
    (check, violations)
}

fn contract_specific_gates(
    profile: RuntimeSystemContractProfile,
    payload: &Value,
