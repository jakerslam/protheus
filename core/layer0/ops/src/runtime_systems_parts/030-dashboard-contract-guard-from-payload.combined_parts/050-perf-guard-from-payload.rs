
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
