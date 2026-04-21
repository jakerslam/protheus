fn capped_signal_u64(payload: &Value, key: &str, fallback: u64) -> u64 {
    payload_u64(payload, key, fallback).min(2_000_000)
}

fn clamped_u64(payload: &Value, key: &str, fallback: u64, min: u64, max: u64) -> u64 {
    payload_u64(payload, key, fallback).clamp(min, max)
}

fn optional_metric(known: bool, value: f64) -> Value {
    if known {
        json!(value)
    } else {
        Value::Null
    }
}

fn dashboard_runtime_authority_from_payload(payload: &Value) -> Value {
    let queue_depth = capped_signal_u64(payload, "queue_depth", 0);
    let critical_attention_total = capped_signal_u64(payload, "critical_attention_total", 0);
    let cockpit_blocks = capped_signal_u64(payload, "cockpit_blocks", 0);
    let conduit_signals = capped_signal_u64(payload, "conduit_signals", 0);
    let stale_blocks = capped_signal_u64(payload, "cockpit_stale_blocks", 0);
    let stale_blocks_raw = capped_signal_u64(payload, "cockpit_stale_blocks_raw", stale_blocks);
    let stale_blocks_dormant = capped_signal_u64(payload, "cockpit_stale_blocks_dormant", 0);
    let stale_ratio = payload_f64(payload, "cockpit_stale_ratio", 0.0).clamp(0.0, 1.0);
    let health_coverage_gap_count = capped_signal_u64(payload, "health_coverage_gap_count", 0);
    let attention_unacked_depth = capped_signal_u64(payload, "attention_unacked_depth", 0);
    let attention_cursor_offset = capped_signal_u64(payload, "attention_cursor_offset", 0);
    let memory_ingest_paused = payload_bool(payload, "memory_ingest_paused", false);
    let collab_handoff_count = capped_signal_u64(payload, "collab_handoff_count", 0);
    let active_swarm_agents = capped_signal_u64(payload, "active_swarm_agents", 0);
    let spine_success_rate = payload_f64(payload, "spine_success_rate", 1.0).clamp(0.0, 1.0);
    let human_escalation_open_rate =
        payload_f64(payload, "human_escalation_open_rate", 0.0).clamp(0.0, 1.0);
    let receipt_latency_p95_ms = payload_f64(payload, "receipt_latency_p95_ms", 0.0).max(0.0);
    let receipt_latency_p99_ms = payload_f64(payload, "receipt_latency_p99_ms", 0.0).max(0.0);
    let spine_metrics_stale = payload_bool(payload, "spine_metrics_stale", false);
    let receipt_latency_metrics_stale =
        payload_bool(payload, "receipt_latency_metrics_stale", false);
    let spine_metrics_latest_age_seconds =
        capped_signal_u64(payload, "spine_metrics_latest_age_seconds", 0);
    let spine_metrics_fresh_window_seconds = clamped_u64(
        payload,
        "spine_metrics_fresh_window_seconds",
        900,
        1,
        2_000_000,
    );
    let benchmark_sanity_age_seconds =
        capped_signal_u64(payload, "benchmark_sanity_age_seconds", 0);
    let benchmark_refresh_max_age_seconds = clamped_u64(
        payload,
        "benchmark_refresh_max_age_seconds",
        1200,
        1,
        2_000_000,
    );
    let benchmark_cockpit_status =
        payload_string(payload, "benchmark_sanity_cockpit_status", "unknown");
    let benchmark_mirror_status = payload_string(payload, "benchmark_sanity_status", "unknown");
    let stable_agent_cap_base = clamped_u64(payload, "stable_agent_cap_base", 512, 16, 2_000_000);
    let min_doubled_cap = stable_agent_cap_base.saturating_mul(2);
    let stable_agent_cap = clamped_u64(
        payload,
        "stable_agent_cap",
        min_doubled_cap,
        min_doubled_cap,
        2_000_000,
    );
    let max_agents_per_cell = clamped_u64(payload, "max_agents_per_cell", 32, 4, 1_000);
    let director_fanout_cells = clamped_u64(payload, "director_fanout_cells", 16, 1, 1_000);
    let max_directors = clamped_u64(payload, "max_directors", 256, 1, 10_000);
    let decentralized_floor = clamped_u64(payload, "decentralized_floor", 24, 1, 1_000_000);

    let dampen_depth = payload_u64(payload, "ingress_dampen_depth", 40).clamp(1, 10_000);
    let shed_depth = payload_u64(payload, "ingress_shed_depth", 80).clamp(dampen_depth, 10_000);
    let circuit_depth =
        payload_u64(payload, "ingress_circuit_depth", 100).clamp(shed_depth, 20_000);
    let ingress_delay_ms = payload_u64(payload, "ingress_delay_ms", 100).clamp(0, 10_000);
    let critical_threshold =
        payload_u64(payload, "critical_escalation_threshold", 7).clamp(1, 1_000);
    let throttle_max_depth = payload_u64(payload, "throttle_max_depth", 80).clamp(40, 10_000);
    let attention_drain_trigger_depth =
        payload_u64(payload, "attention_drain_trigger_depth", 60).clamp(1, 10_000);
    let attention_drain_min_batch =
        payload_u64(payload, "attention_drain_min_batch", 20).clamp(1, 1_000);
    let attention_drain_max_batch = payload_u64(
        payload,
        "attention_drain_max_batch",
        attention_drain_min_batch.max(20),
    )
    .clamp(attention_drain_min_batch, 20_000);
    let attention_compact_depth =
        payload_u64(payload, "attention_compact_depth", 80).clamp(1, 10_000);
    let attention_compact_retain =
        payload_u64(payload, "attention_compact_retain", 256).clamp(1, 200_000);
    let attention_compact_min_acked =
        payload_u64(payload, "attention_compact_min_acked", 64).clamp(1, 200_000);
    let queue_resume_depth = payload_u64(payload, "queue_resume_depth", 40).clamp(1, 10_000);
    let stale_autoheal_min_blocks =
        payload_u64(payload, "stale_autoheal_min_blocks", 10).clamp(1, 10_000);
    let predictive_drain_clear_depth =
        payload_u64(payload, "predictive_drain_clear_depth", 40).clamp(0, 10_000);
    let spine_success_target = payload_f64(payload, "spine_success_target", 0.90).clamp(0.0, 1.0);
    let handoffs_per_agent_min = payload_f64(payload, "handoffs_per_agent_min", 2.0).max(0.0);
    let escalation_open_rate_min = payload_f64(payload, "escalation_open_rate_min", 0.01).max(0.0);
    let slo_latency_p95_max_ms =
        payload_f64(payload, "slo_receipt_latency_p95_max_ms", 250.0).max(1.0);
    let slo_latency_p99_max_ms =
        payload_f64(payload, "slo_receipt_latency_p99_max_ms", 450.0).max(1.0);
    let slo_queue_depth_max = payload_u64(payload, "slo_queue_depth_max", 60).clamp(1, 200_000);

    let growth_risk = queue_depth >= shed_depth && critical_attention_total >= critical_threshold;
    let (ingress_level, reject_non_critical, reason) =
        if queue_depth >= circuit_depth || growth_risk {
            (
                "circuit".to_string(),
                true,
                if queue_depth >= circuit_depth {
                    "queue_circuit_breaker".to_string()
                } else {
                    "critical_growth_risk".to_string()
                },
            )
        } else if queue_depth >= shed_depth {
            ("shed".to_string(), true, "priority_shed".to_string())
        } else if queue_depth >= dampen_depth {
            ("dampen".to_string(), false, "predictive_dampen".to_string())
        } else {
            ("normal".to_string(), false, "steady_state".to_string())
        };
    let delay_ms = if ingress_level == "normal" {
        0
    } else {
        ingress_delay_ms
    };

    let backpressure_level = if queue_depth >= circuit_depth {
        "critical"
    } else if queue_depth >= shed_depth {
        "high"
    } else if queue_depth >= dampen_depth {
        "elevated"
    } else {
        "normal"
    };
    let sync_mode = if queue_depth >= 75 {
        "batch_sync"
    } else if queue_depth >= 50 {
        "delta_sync"
    } else {
        "live_sync"
    };

    let target_conduit_signals = {
        let mut target = 6u64;
        if queue_depth >= 100 {
            target = 16;
        } else if queue_depth >= 80 {
            target = 12;
        } else if queue_depth >= dampen_depth {
            target = 8;
        }
        if cockpit_blocks >= 30 && queue_depth >= dampen_depth {
            target += 2;
        }
        if stale_blocks >= 10 || stale_ratio >= 0.5 {
            target += 2;
        }
        target.clamp(4, 32)
    };
    let conduit_autobalance_required = conduit_signals < target_conduit_signals;
    let signal_quality = if stale_ratio >= 0.5 {
        "coarse"
    } else if stale_ratio >= 0.3 {
        "degraded"
    } else {
        "good"
    };
    let stream_coarse = signal_quality == "coarse";

    let throttle_required = queue_depth >= 75
        || critical_attention_total >= critical_threshold
        || ingress_level == "shed"
        || ingress_level == "circuit"
        || stale_blocks >= 10
        || stale_ratio >= 0.5;
    let throttle_depth = if queue_depth >= circuit_depth {
        throttle_max_depth.saturating_sub(20).max(40)
    } else if queue_depth >= shed_depth {
        throttle_max_depth.saturating_sub(10).max(50)
    } else {
        throttle_max_depth
    };

    let attention_drain_required = queue_depth >= attention_drain_trigger_depth
        || attention_unacked_depth >= attention_compact_min_acked.saturating_mul(2)
        || ingress_level == "circuit";
    let attention_drain_limit = std::cmp::min(
        attention_drain_max_batch,
        std::cmp::max(
            attention_drain_min_batch,
            (queue_depth.saturating_add(2)) / 3,
        ),
    );
    let attention_compact_required = queue_depth >= attention_compact_depth
        && attention_cursor_offset >= attention_compact_min_acked;

    let adaptive_health_required = queue_depth >= 80 || health_coverage_gap_count > 0;
    let memory_resume_eligible = memory_ingest_paused && queue_depth <= queue_resume_depth;
    let stale_autoheal_required = stale_blocks >= stale_autoheal_min_blocks;
    let predictive_drain_required = queue_depth >= attention_drain_trigger_depth;
    let predictive_drain_release = queue_depth <= predictive_drain_clear_depth;

    let cell_count_estimate = if active_swarm_agents == 0 {
        0
    } else {
        active_swarm_agents.div_ceil(max_agents_per_cell)
    };
    let director_target = if cell_count_estimate == 0 {
        0
    } else {
        cell_count_estimate
            .div_ceil(director_fanout_cells)
            .min(max_directors)
    };
    let stable_cap_utilization_pct = if stable_agent_cap == 0 {
        0.0
    } else {
        ((active_swarm_agents as f64 / stable_agent_cap as f64) * 100.0).clamp(0.0, 1000.0)
    };
    let cap_doubled = stable_agent_cap >= stable_agent_cap_base.saturating_mul(2);
    let decentralized_management_required = active_swarm_agents >= decentralized_floor
        || queue_depth >= attention_drain_trigger_depth
        || conduit_autobalance_required
        || stale_ratio >= 0.3;
    let chokepoint_risk = !cap_doubled
        || stable_cap_utilization_pct >= 90.0
        || (queue_depth >= dampen_depth && conduit_signals < target_conduit_signals);

    let handoffs_per_agent = if active_swarm_agents > 0 {
        collab_handoff_count as f64 / active_swarm_agents as f64
    } else {
        0.0
    };
    let handoff_coverage_weak = active_swarm_agents
        >= payload_u64(payload, "handoffs_agent_floor", 24)
        && handoffs_per_agent < handoffs_per_agent_min;
    let spine_degraded = !spine_metrics_stale && spine_success_rate < spine_success_target;
    let escalation_starved = spine_degraded && human_escalation_open_rate <= 0.0;
    let reliability_degraded = spine_degraded || handoff_coverage_weak;
    let facade_response_p95_ms = if receipt_latency_p95_ms > 0.0 {
        receipt_latency_p95_ms.round() as u64
    } else if receipt_latency_p99_ms > 0.0 {
        receipt_latency_p99_ms.round() as u64
    } else {
        0
    };
    let mut facade_confidence: i64 = 100;
    if queue_depth > 20 {
        facade_confidence -= (((queue_depth - 20) / 2).min(20)) as i64;
    }
    if stale_blocks > 0 {
        facade_confidence -= ((stale_blocks.saturating_mul(2)).min(20)) as i64;
    }
    if health_coverage_gap_count > 0 {
        facade_confidence -= ((health_coverage_gap_count.saturating_mul(6)).min(20)) as i64;
    }
    let min_signal_floor = std::cmp::max(3, target_conduit_signals / 2);
    if conduit_signals < min_signal_floor {
        facade_confidence -= 12;
    }
    let benchmark_status_lower = benchmark_cockpit_status.to_ascii_lowercase();
    let benchmark_mirror_lower = benchmark_mirror_status.to_ascii_lowercase();
    if benchmark_status_lower == "warn" || benchmark_mirror_lower == "warn" {
        facade_confidence -= 8;
    }
    if benchmark_status_lower == "fail"
        || benchmark_status_lower == "error"
        || benchmark_mirror_lower == "fail"
        || benchmark_mirror_lower == "error"
    {
        facade_confidence -= 20;
    }
    if !spine_metrics_stale {
        if spine_success_rate < 0.9 {
            facade_confidence -= 15;
        }
        if spine_success_rate < 0.6 {
            facade_confidence -= 10;
        }
    }
    let facade_confidence_percent = facade_confidence.clamp(10, 100) as u64;
    let facade_eta_seconds = if queue_depth == 0 {
        0
    } else {
        queue_depth.div_ceil(8).clamp(1, 300)
    };

    let mut check_rows = Vec::<Value>::new();
    let mut failed_checks = Vec::<String>::new();
    let push_check = |failed_checks: &mut Vec<String>,
                      check_rows: &mut Vec<Value>,
                      id: &str,
                      status: &str,
                      value: Value,
                      target: Value,
                      operator: &str,
                      known: bool| {
        if status == "fail" {
            failed_checks.push(id.to_string());
        }
        check_rows.push(json!({
            "id": id,
            "status": status,
            "value": value,
            "target": target,
            "operator": operator,
            "known": known
        }));
    };
    let spine_status = if spine_metrics_stale {
        "unknown"
    } else if spine_success_rate < spine_success_target {
        "fail"
    } else {
        "pass"
    };
    push_check(
        &mut failed_checks,
        &mut check_rows,
        "spine_success_rate",
        spine_status,
        optional_metric(!spine_metrics_stale, spine_success_rate),
        json!(spine_success_target),
        ">=",
        !spine_metrics_stale,
    );

    let p95_status = if receipt_latency_metrics_stale {
        "unknown"
    } else if receipt_latency_p95_ms > slo_latency_p95_max_ms {
        "fail"
    } else {
        "pass"
    };
    push_check(
        &mut failed_checks,
        &mut check_rows,
        "receipt_latency_p95_ms",
        p95_status,
        optional_metric(!receipt_latency_metrics_stale, receipt_latency_p95_ms),
        json!(slo_latency_p95_max_ms),
        "<=",
        !receipt_latency_metrics_stale,
    );
    let p99_status = if receipt_latency_metrics_stale {
        "unknown"
    } else if receipt_latency_p99_ms > slo_latency_p99_max_ms {
        "fail"
    } else {
        "pass"
    };
    push_check(
        &mut failed_checks,
        &mut check_rows,
        "receipt_latency_p99_ms",
        p99_status,
        optional_metric(!receipt_latency_metrics_stale, receipt_latency_p99_ms),
        json!(slo_latency_p99_max_ms),
        "<=",
        !receipt_latency_metrics_stale,
    );
    let queue_status = if queue_depth > slo_queue_depth_max {
        "fail"
    } else {
        "pass"
    };
    push_check(
        &mut failed_checks,
        &mut check_rows,
        "queue_depth",
        queue_status,
        json!(queue_depth),
        json!(slo_queue_depth_max),
        "<=",
        true,
    );
    let stale_metrics = spine_metrics_stale || receipt_latency_metrics_stale;
    push_check(
        &mut failed_checks,
        &mut check_rows,
        "spine_metrics_freshness",
        if stale_metrics { "warn" } else { "pass" },
        json!(spine_metrics_latest_age_seconds),
        json!(spine_metrics_fresh_window_seconds),
        "<=",
        true,
    );
    let escalation_required = !failed_checks.is_empty() || reliability_degraded;
    let escalation_status = if !escalation_required {
        "pass"
    } else if human_escalation_open_rate > escalation_open_rate_min {
        "pass"
    } else {
        "fail"
    };
    push_check(
        &mut failed_checks,
        &mut check_rows,
        "human_escalation_open_rate",
        escalation_status,
        json!(human_escalation_open_rate),
        json!(escalation_open_rate_min),
        if escalation_required { ">" } else { "n/a" },
        true,
    );

    let severe_latency = !receipt_latency_metrics_stale
        && (receipt_latency_p99_ms > (slo_latency_p99_max_ms * 1.5)
            || receipt_latency_p95_ms > (slo_latency_p95_max_ms * 1.5));
    let severe_spine = !spine_metrics_stale && spine_success_rate < (spine_success_target * 0.75);
    let severe_backlog = queue_depth >= circuit_depth;
    let severity = if failed_checks.is_empty() {
        "ok"
    } else if severe_latency || severe_spine || severe_backlog || failed_checks.len() >= 2 {
        "critical"
    } else {
        "warn"
    };
    let containment_required = !failed_checks.is_empty();
    let block_scale = severity == "critical";
    let slo_required = !failed_checks.is_empty();
    let slo_summary = if failed_checks.is_empty() {
        "runtime_slo_within_bounds".to_string()
    } else {
        format!("runtime_slo_degraded:{}", failed_checks.join("|"))
    };

    let canary_required = spine_metrics_stale
        && spine_metrics_latest_age_seconds >= spine_metrics_fresh_window_seconds;
    let benchmark_refresh_required = benchmark_cockpit_status.eq_ignore_ascii_case("fail")
        || benchmark_mirror_status.eq_ignore_ascii_case("fail")
        || benchmark_sanity_age_seconds > benchmark_refresh_max_age_seconds;
    let conduit_watchdog_required = conduit_signals < target_conduit_signals
        && (queue_depth >= attention_drain_trigger_depth
            || stale_blocks_raw >= stale_autoheal_min_blocks
            || stale_blocks >= stale_autoheal_min_blocks);

    let coarse_signal_remediation_required = stream_coarse
        || stale_autoheal_required
        || stale_blocks_raw >= stale_autoheal_min_blocks
        || (target_conduit_signals > conduit_signals && queue_depth >= dampen_depth);

    let contract_rows = payload_array(payload, "contracts");
    let mut termination_decisions = Vec::<Value>::new();
    let mut idle_candidates = Vec::<(String, u64, i64)>::new();
    let idle_termination_ms =
        payload_u64(payload, "idle_termination_ms", 20 * 60 * 1000).max(1_000);
    let idle_threshold = payload_u64(payload, "idle_threshold", 5).max(1);
    let idle_batch = payload_u64(payload, "idle_batch", 12).max(1);
    let idle_batch_max = payload_u64(payload, "idle_batch_max", 96).max(idle_batch);
    let idle_cooldown_ms = payload_u64(payload, "idle_cooldown_ms", 120_000).max(1_000);
    let idle_since_last_ms = payload_u64(payload, "idle_since_last_ms", idle_cooldown_ms);
    for row in contract_rows {
        let agent_id = payload_string(&row, "agent_id", "");
        if agent_id.is_empty() {
            continue;
        }
        let auto_terminate_allowed = payload_bool(&row, "auto_terminate_allowed", true);
        if !auto_terminate_allowed {
            continue;
        }
        let status = payload_string(&row, "status", "active").to_ascii_lowercase();
        if status != "active" {
            continue;
        }
        let condition =
            payload_string(&row, "termination_condition", "task_or_timeout").to_ascii_lowercase();
        let revoked_at = payload_string(&row, "revoked_at", "");
        let completed_at = payload_string(&row, "completed_at", "");
        let remaining_ms = row
            .get("remaining_ms")
            .and_then(Value::as_i64)
            .or_else(|| {
                row.get("remaining_ms")
                    .and_then(Value::as_u64)
                    .map(|v| v as i64)
            })
            .unwrap_or(i64::MAX);
        let mut reason = String::new();
        if !revoked_at.is_empty() {
            reason = "manual_revocation".to_string();
        } else if (condition == "task_complete" || condition == "task_or_timeout")
            && !completed_at.is_empty()
        {
            reason = "task_complete".to_string();
        } else if (condition == "timeout" || condition == "task_or_timeout") && remaining_ms <= 0 {
            reason = "timeout".to_string();
        }
        if !reason.is_empty() {
            termination_decisions.push(json!({
                "agent_id": agent_id,
                "reason": reason,
                "authority": "rust_runtime_systems"
            }));
        }
        let idle_for_ms = row
            .get("idle_for_ms")
            .and_then(Value::as_u64)
            .or_else(|| {
                row.get("idle_for_ms")
                    .and_then(Value::as_i64)
                    .map(|v| v.max(0) as u64)
            })
            .unwrap_or(0);
        if idle_for_ms >= idle_termination_ms {
            idle_candidates.push((agent_id, idle_for_ms, remaining_ms));
        }
    }
    idle_candidates.sort_by(|a, b| b.1.cmp(&a.1));
    let idle_excess = idle_candidates
        .len()
        .saturating_sub(idle_threshold as usize) as u64;
    let idle_sweep_ready = idle_excess > 0 && idle_since_last_ms >= idle_cooldown_ms;
    let idle_batch_size = if idle_excess == 0 {
        0
    } else {
        std::cmp::min(
            idle_batch_max,
            std::cmp::min(
                idle_excess,
                std::cmp::max(idle_batch, (idle_excess + 5) / 6),
            ),
        )
    };
    let idle_candidates_json: Vec<Value> = idle_candidates
        .iter()
        .map(|(agent_id, idle_for_ms, remaining_ms)| {
            json!({
                "agent_id": agent_id,
                "idle_for_ms": idle_for_ms,
                "remaining_ms": if *remaining_ms == i64::MAX { Value::Null } else { json!(remaining_ms) }
            })
        })
        .collect();

    let mut role_plan = Vec::<Value>::new();
    if throttle_required || adaptive_health_required {
        role_plan.push(json!({ "role": "coordinator", "required": true }));
    }
    if conduit_autobalance_required || critical_attention_total >= 5 {
        role_plan.push(json!({ "role": "researcher", "required": true }));
    }
    if cockpit_blocks >= 30 || stale_blocks > 0 {
        role_plan.push(json!({ "role": "builder", "required": true }));
    }
    if queue_depth >= 80 || critical_attention_total >= critical_threshold {
        role_plan.push(json!({ "role": "analyst", "required": true }));
    }
    if queue_depth >= 80 && stale_ratio >= 0.3 {
        role_plan.push(json!({ "role": "reviewer", "required": true }));
    }
    if decentralized_management_required {
        role_plan.push(json!({
            "role": "director",
            "required": true,
            "target_count": director_target.max(1),
            "reason": "decentralized_hierarchy_scale_guard"
        }));
        role_plan.push(json!({
            "role": "cell_coordinator",
            "required": true,
            "target_count": cell_count_estimate.max(1),
            "reason": "decentralized_cell_routing"
        }));
    }

    json!({
        "authority": "rust_runtime_systems",
        "contract_id": "V6-DASHBOARD-007.1",
        "runtime": {
            "queue_depth": queue_depth,
            "critical_attention_total": critical_attention_total,
            "cockpit_blocks": cockpit_blocks,
            "cockpit_stale_blocks": stale_blocks,
            "cockpit_stale_blocks_raw": stale_blocks_raw,
            "cockpit_stale_blocks_dormant": stale_blocks_dormant,
            "cockpit_stale_ratio": stale_ratio,
            "conduit_signals": conduit_signals,
            "attention_unacked_depth": attention_unacked_depth,
            "attention_cursor_offset": attention_cursor_offset,
            "memory_ingest_paused": memory_ingest_paused,
            "collab_handoff_count": collab_handoff_count,
            "active_swarm_agents": active_swarm_agents,
            "spine_success_rate": spine_success_rate,
            "human_escalation_open_rate": human_escalation_open_rate,
            "receipt_latency_p95_ms": receipt_latency_p95_ms,
            "receipt_latency_p99_ms": receipt_latency_p99_ms,
            "facade_response_p95_ms": facade_response_p95_ms,
            "facade_confidence_percent": facade_confidence_percent,
            "facade_eta_seconds": facade_eta_seconds,
            "spine_metrics_stale": spine_metrics_stale,
            "receipt_latency_metrics_stale": receipt_latency_metrics_stale,
            "spine_metrics_latest_age_seconds": spine_metrics_latest_age_seconds
        },
        "cockpit_signal": {
            "quality": signal_quality,
            "coarse": stream_coarse
        },
        "ingress_control": {
            "level": ingress_level,
            "reject_non_critical": reject_non_critical,
            "delay_ms": delay_ms,
            "reason": reason,
            "dampen_depth": dampen_depth,
            "shed_depth": shed_depth,
            "circuit_depth": circuit_depth
        },
        "recommendations": {
            "sync_mode": sync_mode,
            "backpressure_level": backpressure_level,
            "target_conduit_signals": target_conduit_signals,
            "conduit_autobalance_required": conduit_autobalance_required,
            "adaptive_health_required": adaptive_health_required,
            "throttle_required": throttle_required,
            "throttle_max_depth": throttle_depth,
            "attention_drain_required": attention_drain_required,
            "attention_drain_limit": attention_drain_limit,
            "attention_compact_required": attention_compact_required,
            "attention_compact_retain": attention_compact_retain,
            "attention_compact_min_acked": attention_compact_min_acked,
            "memory_resume_eligible": memory_resume_eligible,
            "stale_autoheal_required": stale_autoheal_required,
            "coarse_signal_remediation_required": coarse_signal_remediation_required,
            "conduit_watchdog_required": conduit_watchdog_required,
            "spine_canary_required": canary_required,
            "benchmark_refresh_required": benchmark_refresh_required,
            "predictive_drain_required": predictive_drain_required,
            "predictive_drain_release": predictive_drain_release,
            "decentralized_management_required": decentralized_management_required,
            "stable_agent_cap": stable_agent_cap,
            "cell_count_estimate": cell_count_estimate,
            "director_target": director_target
        },
        "scale_model": {
            "stable_agent_cap_base": stable_agent_cap_base,
            "stable_agent_cap": stable_agent_cap,
            "cap_doubled": cap_doubled,
            "active_swarm_agents": active_swarm_agents,
            "stable_cap_utilization_pct": stable_cap_utilization_pct,
            "max_agents_per_cell": max_agents_per_cell,
            "director_fanout_cells": director_fanout_cells,
            "max_directors": max_directors,
            "decentralized_floor": decentralized_floor,
            "decentralized_management_required": decentralized_management_required,
            "cell_count_estimate": cell_count_estimate,
            "director_target": director_target,
            "chokepoint_risk": chokepoint_risk
        },
        "reliability_posture": {
            "degraded": reliability_degraded,
            "spine_degraded": spine_degraded,
            "spine_success_rate": spine_success_rate,
            "spine_success_target": spine_success_target,
            "spine_metrics_stale": spine_metrics_stale,
            "escalation_open_rate": human_escalation_open_rate,
            "escalation_starved": escalation_starved,
            "handoff_count": collab_handoff_count,
            "handoffs_per_agent": handoffs_per_agent,
            "handoffs_per_agent_min": handoffs_per_agent_min,
            "handoff_coverage_weak": handoff_coverage_weak,
            "active_swarm_agents": active_swarm_agents
        },
        "slo_gate": {
            "required": slo_required,
            "severity": severity,
            "block_scale": block_scale,
            "containment_required": containment_required,
            "failed_checks": failed_checks,
            "checks": check_rows,
            "summary": slo_summary,
            "stale_metrics": stale_metrics,
            "thresholds": {
                "spine_success_rate_min": spine_success_target,
                "receipt_latency_p95_max_ms": slo_latency_p95_max_ms,
                "receipt_latency_p99_max_ms": slo_latency_p99_max_ms,
                "queue_depth_max": slo_queue_depth_max,
                "escalation_open_rate_min": escalation_open_rate_min
            }
        },
        "contract_enforcement": {
            "termination_decisions": termination_decisions,
            "idle_candidates": idle_candidates_json,
            "idle_threshold": idle_threshold,
            "idle_excess": idle_excess,
            "idle_batch_size": idle_batch_size,
            "idle_sweep_ready": idle_sweep_ready,
            "idle_termination_ms": idle_termination_ms
        },
        "role_plan": role_plan
    })
}
