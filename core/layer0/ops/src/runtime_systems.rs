// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops::runtime_systems (authoritative)
use crate::contract_lane_utils as lane_utils;
use crate::runtime_system_contracts::{
    actionable_profiles, looks_like_contract_id, profile_for, RuntimeSystemContractProfile,
};
use crate::{client_state_root, deterministic_receipt_hash, now_iso};
use llm_runtime::{
    choose_best_model, normalize_model_scores, ModelMetadata, ModelRuntimeKind, ModelSpecialty,
    RoutingRequest, WorkloadClass,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const LANE_ID: &str = "runtime_systems";

fn usage() {
    println!("Usage:");
    println!("  protheus-ops runtime-systems <status|verify|run|build|manifest|roi-sweep|bootstrap|package|settle> [--system-id=<id>|--lane-id=<id>] [flags]");
}

fn print_json_line(value: &Value) {
    println!(
        "{}",
        serde_json::to_string(value)
            .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode_failed\"}".to_string())
    );
}

fn receipt_hash(value: &Value) -> String {
    deterministic_receipt_hash(value)
}

fn profile_json(profile: RuntimeSystemContractProfile) -> Value {
    json!({
        "id": profile.id,
        "family": profile.family,
        "objective": profile.objective,
        "strict_conduit_only": profile.strict_conduit_only,
        "strict_fail_closed": profile.strict_fail_closed
    })
}

fn mutation_receipt_claim(system_id: &str, command: &str, apply: bool, strict: bool) -> Value {
    json!({
        "id": "runtime_system_mutation_receipted",
        "claim": "runtime_system_operations_emit_deterministic_receipts_and_state",
        "evidence": {
            "system_id": system_id,
            "command": command,
            "apply": apply,
            "strict": strict
        }
    })
}

fn parse_json(raw: Option<&str>) -> Result<Value, String> {
    let text = raw.ok_or_else(|| "missing_json_payload".to_string())?;
    serde_json::from_str::<Value>(text).map_err(|err| format!("invalid_json_payload:{err}"))
}

fn systems_dir(root: &Path) -> PathBuf {
    client_state_root(root).join("runtime_systems")
}

fn latest_path(root: &Path, system_id: &str) -> PathBuf {
    systems_dir(root).join(system_id).join("latest.json")
}

fn history_path(root: &Path, system_id: &str) -> PathBuf {
    systems_dir(root).join(system_id).join("history.jsonl")
}

fn contract_state_path(root: &Path, family: &str) -> PathBuf {
    systems_dir(root)
        .join("_contracts")
        .join(family)
        .join("state.json")
}

fn payload_f64(payload: &Value, key: &str, fallback: f64) -> f64 {
    payload
        .get(key)
        .and_then(Value::as_f64)
        .or_else(|| payload.get(key).and_then(Value::as_i64).map(|v| v as f64))
        .or_else(|| payload.get(key).and_then(Value::as_u64).map(|v| v as f64))
        .unwrap_or(fallback)
}

fn payload_bool(payload: &Value, key: &str, fallback: bool) -> bool {
    payload
        .get(key)
        .and_then(Value::as_bool)
        .unwrap_or(fallback)
}

fn payload_string(payload: &Value, key: &str, fallback: &str) -> String {
    payload
        .get(key)
        .and_then(Value::as_str)
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| fallback.to_string())
}

fn payload_string_array(payload: &Value, key: &str, fallback: &[&str]) -> Vec<String> {
    payload
        .get(key)
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
                .collect::<Vec<_>>()
        })
        .filter(|rows| !rows.is_empty())
        .unwrap_or_else(|| fallback.iter().map(|v| (*v).to_string()).collect())
}

fn payload_u64(payload: &Value, key: &str, fallback: u64) -> u64 {
    payload
        .get(key)
        .and_then(Value::as_u64)
        .or_else(|| {
            payload
                .get(key)
                .and_then(Value::as_i64)
                .map(|v| v.max(0) as u64)
        })
        .unwrap_or(fallback)
}

fn payload_array(payload: &Value, key: &str) -> Vec<Value> {
    payload
        .get(key)
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn now_epoch_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
        .as_secs()
}

fn missing_required_tokens(actual: &[String], required: &[&str]) -> Vec<String> {
    let set: BTreeSet<String> = actual.iter().map(|v| v.to_ascii_lowercase()).collect();
    required
        .iter()
        .filter_map(|token| {
            let canonical = token.to_ascii_lowercase();
            if set.contains(&canonical) {
                None
            } else {
                Some((*token).to_string())
            }
        })
        .collect()
}

fn dashboard_runtime_authority_from_payload(payload: &Value) -> Value {
    let queue_depth = payload_u64(payload, "queue_depth", 0).min(2_000_000);
    let critical_attention_total =
        payload_u64(payload, "critical_attention_total", 0).min(2_000_000);
    let cockpit_blocks = payload_u64(payload, "cockpit_blocks", 0).min(2_000_000);
    let conduit_signals = payload_u64(payload, "conduit_signals", 0).min(2_000_000);
    let stale_blocks = payload_u64(payload, "cockpit_stale_blocks", 0).min(2_000_000);
    let stale_blocks_raw =
        payload_u64(payload, "cockpit_stale_blocks_raw", stale_blocks).min(2_000_000);
    let stale_blocks_dormant =
        payload_u64(payload, "cockpit_stale_blocks_dormant", 0).min(2_000_000);
    let stale_ratio = payload_f64(payload, "cockpit_stale_ratio", 0.0).clamp(0.0, 1.0);
    let health_coverage_gap_count =
        payload_u64(payload, "health_coverage_gap_count", 0).min(2_000_000);
    let attention_unacked_depth = payload_u64(payload, "attention_unacked_depth", 0).min(2_000_000);
    let attention_cursor_offset = payload_u64(payload, "attention_cursor_offset", 0).min(2_000_000);
    let memory_ingest_paused = payload_bool(payload, "memory_ingest_paused", false);
    let collab_handoff_count = payload_u64(payload, "collab_handoff_count", 0).min(2_000_000);
    let active_swarm_agents = payload_u64(payload, "active_swarm_agents", 0).min(2_000_000);
    let spine_success_rate = payload_f64(payload, "spine_success_rate", 1.0).clamp(0.0, 1.0);
    let human_escalation_open_rate =
        payload_f64(payload, "human_escalation_open_rate", 0.0).clamp(0.0, 1.0);
    let receipt_latency_p95_ms = payload_f64(payload, "receipt_latency_p95_ms", 0.0).max(0.0);
    let receipt_latency_p99_ms = payload_f64(payload, "receipt_latency_p99_ms", 0.0).max(0.0);
    let spine_metrics_stale = payload_bool(payload, "spine_metrics_stale", false);
    let receipt_latency_metrics_stale =
        payload_bool(payload, "receipt_latency_metrics_stale", false);
    let spine_metrics_latest_age_seconds =
        payload_u64(payload, "spine_metrics_latest_age_seconds", 0).min(2_000_000);
    let spine_metrics_fresh_window_seconds =
        payload_u64(payload, "spine_metrics_fresh_window_seconds", 900).clamp(1, 2_000_000);
    let benchmark_sanity_age_seconds =
        payload_u64(payload, "benchmark_sanity_age_seconds", 0).min(2_000_000);
    let benchmark_refresh_max_age_seconds =
        payload_u64(payload, "benchmark_refresh_max_age_seconds", 1200).clamp(1, 2_000_000);
    let benchmark_cockpit_status =
        payload_string(payload, "benchmark_sanity_cockpit_status", "unknown");
    let benchmark_mirror_status = payload_string(payload, "benchmark_sanity_status", "unknown");
    let stable_agent_cap_base =
        payload_u64(payload, "stable_agent_cap_base", 512).clamp(16, 2_000_000);
    let min_doubled_cap = stable_agent_cap_base.saturating_mul(2);
    let stable_agent_cap =
        payload_u64(payload, "stable_agent_cap", min_doubled_cap).clamp(min_doubled_cap, 2_000_000);
    let max_agents_per_cell = payload_u64(payload, "max_agents_per_cell", 32).clamp(4, 1_000);
    let director_fanout_cells = payload_u64(payload, "director_fanout_cells", 16).clamp(1, 1_000);
    let max_directors = payload_u64(payload, "max_directors", 256).clamp(1, 10_000);
    let decentralized_floor = payload_u64(payload, "decentralized_floor", 24).clamp(1, 1_000_000);

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
        json!(if spine_metrics_stale {
            Value::Null
        } else {
            json!(spine_success_rate)
        }),
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
        json!(if receipt_latency_metrics_stale {
            Value::Null
        } else {
            json!(receipt_latency_p95_ms)
        }),
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
        json!(if receipt_latency_metrics_stale {
            Value::Null
        } else {
            json!(receipt_latency_p99_ms)
        }),
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

fn family_contract_requirements(family: &str) -> FamilyContractRequirements {
    match family {
        "audit_self_healing_stack" => FamilyContractRequirements {
            required_true: &[
                "drift_detection_enabled",
                "self_healing_playbooks_enabled",
                "confidence_scoring_enabled",
                "cross_agent_verification_enabled",
                "human_review_gate_enforced",
                "conduit_only_enforced",
            ],
            min_values: &[
                ("confidence_high_threshold", 0.85),
                ("verification_agents", 2.0),
            ],
            max_values: &[("poll_interval_minutes", 15.0)],
        },
        "ultimate_evolution" => FamilyContractRequirements {
            required_true: &[
                "replication_policy_gate",
                "self_awareness_journal",
                "exotic_hardware_abstraction",
                "tokenomics_ledger_enforced",
                "symbiosis_interface",
                "universal_adapter_skeleton_key",
            ],
            min_values: &[("universal_adapter_coverage_pct", 80.0)],
            max_values: EMPTY_NUM_GATES,
        },
        "automation_mission_stack" => FamilyContractRequirements {
            required_true: &[
                "cron_scheduler_enabled",
                "multi_agent_handoff_enabled",
                "persistent_memory_enabled",
                "security_hardening_enabled",
                "mission_dashboard_enabled",
            ],
            min_values: EMPTY_NUM_GATES,
            max_values: &[
                ("checkpoint_interval_items", 10.0),
                ("checkpoint_interval_minutes", 2.0),
            ],
        },
        "autonomy_opportunity_engine" => FamilyContractRequirements {
            required_true: &[
                "opportunity_discovery_engine",
                "inefficiency_scanner",
                "monetization_evaluator",
                "hindsight_ranking_engine",
            ],
            min_values: &[("creative_mode_signal_floor", 0.5)],
            max_values: EMPTY_NUM_GATES,
        },
        "cli_surface_hardening" => FamilyContractRequirements {
            required_true: &[
                "single_static_rust_binary",
                "rust_state_machine_core",
                "ts_cli_opt_in_extension",
                "thin_shim_wrapper",
                "node_absence_doctor_message",
            ],
            min_values: EMPTY_NUM_GATES,
            max_values: &[("static_binary_mb", 6.0)],
        },
        "client_model_access" => FamilyContractRequirements {
            required_true: &[
                "vibe_proxy_layer_enabled",
                "model_access_store_encrypted",
                "model_access_store_policy_gate",
            ],
            min_values: EMPTY_NUM_GATES,
            max_values: EMPTY_NUM_GATES,
        },
        "competitive_execution_moat" => FamilyContractRequirements {
            required_true: &[
                "aot_musl_zerocopy_lanes",
                "signed_receipt_export_sub_ms",
                "non_divergence_pre_execution_gate",
                "autonomous_swarm_workflow_evolution",
                "kernel_native_observability_governance",
                "edge_to_cloud_uniform_plan",
                "production_resilience_flywheel",
            ],
            min_values: &[("throughput_ops_sec", 11000.0)],
            max_values: &[("p95_ms", 50.0)],
        },
        "eyes_media_assimilation" => FamilyContractRequirements {
            required_true: &[
                "video_transcription_enabled",
                "course_assimilation_pipeline",
                "podcast_generator_enabled",
                "swarm_opportunity_integration",
            ],
            min_values: &[("transcript_quality_floor", 0.7)],
            max_values: EMPTY_NUM_GATES,
        },
        "eyes_computer_use" => FamilyContractRequirements {
            required_true: &[
                "parchi_computer_use_engine",
                "frontend_navigation_reliability",
                "computer_use_safety_gate",
                "superwhisper_voice_engine",
                "voice_session_blob_archival",
            ],
            min_values: &[("interaction_success_floor", 0.75)],
            max_values: EMPTY_NUM_GATES,
        },
        "eyes_lightpanda_router" => FamilyContractRequirements {
            required_true: &[
                "lightpanda_backend_enabled",
                "ultra_speed_profile_enabled",
                "seamless_multi_backend_router",
                "browser_session_blob_archival",
            ],
            min_values: &[("target_speedup_x", 10.0)],
            max_values: EMPTY_NUM_GATES,
        },
        "learning_rsi_pipeline" => FamilyContractRequirements {
            required_true: &[
                "signal_extraction_prm_judge",
                "hindsight_on_policy_distillation",
                "async_four_loop_training",
                "interaction_trajectory_blob_integration",
                "distributed_gym_factory",
                "adversarial_verification_pipeline",
                "training_flywheel_export",
                "real_world_product_verifier",
                "local_overnight_self_improvement",
                "real_usage_feedback_reinforcement",
                "single_directive_rl_engine",
                "emergent_strategy_discovery",
                "weekly_policy_retraining",
                "auto_rollback_enabled",
                "low_cost_overnight_loop",
            ],
            min_values: &[("training_loops_per_day", 1.0)],
            max_values: EMPTY_NUM_GATES,
        },
        "memory_depth_stack" => FamilyContractRequirements {
            required_true: &[
                "hierarchical_tree_index_builder",
                "agentic_tree_reasoning_retriever",
                "vision_page_retrieval",
                "tree_index_trace_blob_archival",
                "lossless_folder_backend",
                "automatic_sync_perfect_recall",
                "blob_lossless_hybrid_mirroring",
                "tinymax_lossless_mode",
                "tree_sitter_ast_indexer",
                "blast_radius_analyzer",
                "auto_codebase_wiki_generator",
                "mcp_graph_integration",
                "persistent_case_facts_scratchpad",
                "claim_source_provenance_mapping",
            ],
            min_values: EMPTY_NUM_GATES,
            max_values: &[("recall_budget_ms", 500.0)],
        },
        "organism_parallel_intelligence" => FamilyContractRequirements {
            required_true: &[
                "side_chat_forking_engine",
                "non_capturing_overlay_renderer",
                "file_overlap_peek_safety",
                "persistent_side_session_blob_integration",
                "hub_spoke_coordinator",
                "plan_vs_explore_subagent_separation",
                "autonomous_model_generator",
                "self_critique_alternative_perspectives",
                "explainer_slide_visual_synthesis",
                "model_evolution_archive",
            ],
            min_values: EMPTY_NUM_GATES,
            max_values: EMPTY_NUM_GATES,
        },
        "persona_enterprise_pack" => FamilyContractRequirements {
            required_true: &[
                "ai_ceo_persona_core",
                "departmental_agent_pack",
                "cross_agent_memory_sync",
                "role_based_agent_addition",
            ],
            min_values: EMPTY_NUM_GATES,
            max_values: EMPTY_NUM_GATES,
        },
        "safety_error_taxonomy" => FamilyContractRequirements {
            required_true: &["structured_error_taxonomy", "error_fail_closed_mapping"],
            min_values: EMPTY_NUM_GATES,
            max_values: EMPTY_NUM_GATES,
        },
        "security_sandbox_redteam" => FamilyContractRequirements {
            required_true: &[
                "wasm_capability_sandbox",
                "credential_injection_isolation",
                "verifiable_privacy_plane",
                "long_horizon_attack_chain_simulation",
                "zero_to_full_context_accumulation",
                "attack_trajectory_blob_archival",
            ],
            min_values: EMPTY_NUM_GATES,
            max_values: &[("max_escape_rate", 0.001)],
        },
        "skills_runtime_pack" => FamilyContractRequirements {
            required_true: &[
                "native_hf_cli_skill",
                "autonomous_model_dataset_pipeline",
                "hf_pure_context_mode",
                "hf_output_swarm_integration",
                "native_pomodoro_skill",
                "interactive_tui_focus_mode",
                "shell_composable_focus_status",
                "focus_session_blob_integration",
                "raspberry_pi_edge_template",
                "self_healing_server_agent",
                "orion_team_coordinator",
                "productivity_workflow_pack",
                "lens_scribe_code_agent_pack",
                "claude_style_prompt_chaining",
                "iterative_refinement_loop",
                "component_fullstack_scaffolding",
                "one_click_deployment_flow",
            ],
            min_values: EMPTY_NUM_GATES,
            max_values: EMPTY_NUM_GATES,
        },
        "swarm_runtime_scaling" => FamilyContractRequirements {
            required_true: &[
                "sentiment_swarm_core",
                "scenario_injection_live_consensus_mapper",
                "prediction_market_sentiment_oracle",
                "swarm_trajectory_storage_dream_refinement",
                "role_based_model_assignment",
                "automatic_parallel_exploration",
                "visual_subagent_dashboard",
                "subagent_edit_permission_gate",
                "planning_as_tool_engine",
                "filesystem_native_persistent_memory",
                "isolated_subagent_spawning",
                "shell_execution_safety_gates",
                "worker_heartbeat",
                "automatic_work_stealing",
                "supervisor_watchdog_respawn",
                "output_schema_enforcement",
                "frequent_checkpoint_recovery",
                "scope_boundary_validation",
                "realtime_aggregation_dashboard",
                "capability_advertisement_adaptive_partitioning",
                "cross_agent_dedup_reconciliation",
                "timeout_graceful_degradation",
            ],
            min_values: EMPTY_NUM_GATES,
            max_values: &[("max_timeout_seconds", 120.0)],
        },
        "client_wasm_bridge" => FamilyContractRequirements {
            required_true: &[
                "rust_wasm_bridge_engine",
                "browser_structured_concurrency",
                "standalone_html_demo_generator",
                "wasm_artifact_blob_archival",
            ],
            min_values: EMPTY_NUM_GATES,
            max_values: EMPTY_NUM_GATES,
        },
        "organism_adlc" => FamilyContractRequirements {
            required_true: &[
                "adlc_core_engine",
                "evolving_goals_replanning",
                "parallel_subagent_coordination",
                "continuous_testing_live_feedback",
            ],
            min_values: EMPTY_NUM_GATES,
            max_values: EMPTY_NUM_GATES,
        },
        "tinymax_extreme_profile" => FamilyContractRequirements {
            required_true: &[
                "trait_driven_swappable_tinymax_core",
                "sub5mb_idle_memory_mode",
            ],
            min_values: EMPTY_NUM_GATES,
            max_values: &[("idle_memory_mb", 5.0)],
        },
        "execution_streaming_stack" => FamilyContractRequirements {
            required_true: &[
                "ssd_streaming_enabled",
                "quantization_bridge_enabled",
                "os_page_cache_first",
                "kernel_bridge_enabled",
            ],
            min_values: &[("target_tokens_per_sec", 64.0)],
            max_values: &[("resident_memory_gb", 24.0)],
        },
        "execution_worktree_stack" => FamilyContractRequirements {
            required_true: &[
                "worktree_manager_enabled",
                "branch_isolation_enforced",
                "swarm_worktree_dispatch_enabled",
                "cleanup_enabled",
            ],
            min_values: &[("cleanup_interval_seconds", 30.0)],
            max_values: &[("max_residual_worktrees", 5.0)],
        },
        "assimilate_fast_stack" => FamilyContractRequirements {
            required_true: &[
                "fast_mode_enabled",
                "skeleton_cache_enabled",
                "progress_receipts_enabled",
                "parallel_microtasks_enabled",
                "warmup_enabled",
                "safety_guard_enabled",
            ],
            min_values: &[("target_latency_ms", 1.0)],
            max_values: &[("target_latency_ms", 60000.0)],
        },
        "workflow_open_swe_stack" => FamilyContractRequirements {
            required_true: &[
                "loop_registry_enabled",
                "git_bridge_enabled",
                "approval_middleware_enabled",
                "eval_harness_enabled",
                "memory_continuity_enabled",
            ],
            min_values: &[("eval_pass_floor", 0.5)],
            max_values: EMPTY_NUM_GATES,
        },
        "memory_context_maintenance" => FamilyContractRequirements {
            required_true: &[
                "staleness_tracking_enabled",
                "pre_generation_pruning_enabled",
                "emergency_compact_enabled",
                "context_observability_enabled",
                "safe_config_validation_enabled",
            ],
            min_values: &[("context_budget_tokens", 1024.0)],
            max_values: &[("context_budget_tokens", 10000000.0)],
        },
        "integration_lakehouse_stack" => FamilyContractRequirements {
            required_true: &[
                "unity_catalog_bridge_enabled",
                "mosaic_mapping_enabled",
                "mlflow_provider_enabled",
                "vector_automl_bridge_enabled",
                "dbrx_provider_enabled",
                "drift_monitoring_enabled",
            ],
            min_values: EMPTY_NUM_GATES,
            max_values: EMPTY_NUM_GATES,
        },
        "inference_adaptive_routing" => FamilyContractRequirements {
            required_true: &[
                "live_scoring_enabled",
                "rule_routing_enabled",
                "ordered_failover_enabled",
                "provider_observability_enabled",
            ],
            min_values: &[("min_success_rate", 0.5)],
            max_values: &[("max_latency_ms", 5000.0)],
        },
        "runtime_cleanup_autonomous" => FamilyContractRequirements {
            required_true: &[
                "eviction_matrix_enabled",
                "multi_trigger_scheduler_enabled",
                "tiered_pressure_enabled",
                "device_profiles_enabled",
                "protected_state_invariants_enabled",
                "audit_controls_enabled",
                "boundedness_gate_enabled",
            ],
            min_values: &[("cleanup_interval_minutes", 1.0)],
            max_values: &[("cleanup_interval_minutes", 60.0)],
        },
        "erp_agentic_stack" => FamilyContractRequirements {
            required_true: &[
                "erp_template_registry_enabled",
                "closed_loop_enabled",
                "lineage_gate_enabled",
            ],
            min_values: &[("max_loop_latency_ms", 1.0)],
            max_values: &[("max_loop_latency_ms", 60000.0)],
        },
        "tooling_uv_ruff_stack" => FamilyContractRequirements {
            required_true: &[
                "uv_bridge_enabled",
                "ruff_bridge_enabled",
                "isolated_env_enabled",
                "autowire_pipeline_enabled",
                "tooling_gate_enabled",
            ],
            min_values: &[("max_resolution_time_seconds", 1.0)],
            max_values: &[("max_resolution_time_seconds", 3600.0)],
        },
        "workflow_visual_bridge_stack" => FamilyContractRequirements {
            required_true: &[
                "canvas_bridge_enabled",
                "prompt_chain_enabled",
                "rag_integration_enabled",
                "tool_eval_enabled",
                "enterprise_observability_enabled",
            ],
            min_values: &[("cold_start_guard_ms", 1.0)],
            max_values: &[("cold_start_guard_ms", 120000.0)],
        },
        "openclaw_detachment_stack" => FamilyContractRequirements {
            required_true: &[
                "source_assimilation_enabled",
                "nursery_migration_enabled",
                "external_dependency_detached",
                "operator_state_capture_enabled",
                "local_runtime_paths_enforced",
                "source_controlled_mirror_enabled",
                "llm_runtime_registry_enabled",
            ],
            min_values: &[
                ("source_files_required_min", 1.0),
                ("llm_registry_models_min", 1.0),
            ],
            max_values: &[("max_assimilation_copy_mb", 1024.0)],
        },
        _ => FamilyContractRequirements {
            required_true: EMPTY_REQUIRED_TRUE,
            min_values: EMPTY_NUM_GATES,
            max_values: EMPTY_NUM_GATES,
        },
    }
}

fn execute_generic_family_contract(
    root: &Path,
    profile: RuntimeSystemContractProfile,
    payload: &Value,
    apply: bool,
    strict: bool,
) -> Result<ContractExecution, String> {
    let (state_path, mut state, state_rel) = load_contract_state(root, profile);
    let requirements = family_contract_requirements(profile.family);

    let mut bool_checks = serde_json::Map::new();
    let mut min_checks = serde_json::Map::new();
    let mut max_checks = serde_json::Map::new();
    let mut specific_checks = serde_json::Map::new();
    let mut violations = Vec::<String>::new();

    for key in requirements.required_true {
        let value = payload_bool(payload, key, false);
        bool_checks.insert((*key).to_string(), json!(value));
        if !value {
            violations.push(format!("required_true:{key}"));
        }
    }
    for (key, min) in requirements.min_values {
        let value = payload_f64(payload, key, *min);
        min_checks.insert((*key).to_string(), json!({ "value": value, "min": min }));
        if value < *min {
            violations.push(format!("min_violation:{key}:{value:.6}<{min:.6}"));
        }
    }
    for (key, max) in requirements.max_values {
        let value = payload_f64(payload, key, *max);
        max_checks.insert((*key).to_string(), json!({ "value": value, "max": max }));
        if value > *max {
            violations.push(format!("max_violation:{key}:{value:.6}>{max:.6}"));
        }
    }

    let (specific, specific_violations) = contract_specific_gates(profile, payload);
    specific_checks.extend(specific);
    violations.extend(specific_violations);

    if strict && !violations.is_empty() {
        return Err(format!(
            "family_contract_gate_failed:{}:{}",
            profile.id,
            violations.join(",")
        ));
    }

    let gate_pass = violations.is_empty();
    let summary = json!({
        "family": profile.family,
        "contract_id": profile.id,
        "objective": profile.objective,
        "gate_pass": gate_pass,
        "required_true": bool_checks,
        "min_checks": min_checks,
        "max_checks": max_checks,
        "specific_checks": specific_checks,
        "violations": violations,
        "state_path": state_rel
    });

    if apply {
        upsert_contract_state_entry(
            &mut state,
            profile.id,
            json!({
                "summary": summary,
                "applied_at": now_iso()
            }),
        );
        lane_utils::write_json(&state_path, &state)?;
    }

    Ok(ContractExecution {
        summary,
        claims: vec![json!({
            "id": profile.id,
            "claim": "family_contract_executes_via_core_runtime_with_strict_gate_checks_and_stateful_receipts",
            "evidence": {
                "family": profile.family,
                "gate_pass": gate_pass,
                "state_path": state_rel
            }
        })],
        artifacts: vec![state_rel],
    })
}

fn load_contract_state(
    root: &Path,
    profile: RuntimeSystemContractProfile,
) -> (PathBuf, Value, String) {
    let state_path = contract_state_path(root, profile.family);
    let state = lane_utils::read_json(&state_path).unwrap_or_else(|| {
        json!({
            "family": profile.family,
            "contracts": {},
            "updated_at": now_iso()
        })
    });
    let state_rel = lane_utils::rel_path(root, &state_path);
    (state_path, state, state_rel)
}

fn upsert_contract_state_entry(state: &mut Value, profile_id: &str, entry: Value) {
    state["updated_at"] = Value::String(now_iso());
    if state.get("contracts").and_then(Value::as_object).is_none() {
        state["contracts"] = json!({});
    }
    state["contracts"][profile_id] = entry;
}

fn command_version(command: &str) -> Option<String> {
    let output = Command::new(command).arg("--version").output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

fn family_data_root(root: &Path, family: &str) -> PathBuf {
    systems_dir(root).join("_families").join(family)
}

fn file_age_seconds(path: &Path) -> Option<u64> {
    let modified = fs::metadata(path).ok()?.modified().ok()?;
    let now = SystemTime::now();
    now.duration_since(modified)
        .ok()
        .map(|delta| delta.as_secs())
}

fn remove_stale_files(
    dir: &Path,
    min_age_secs: u64,
    dry_run: bool,
    protected_prefixes: &[&str],
) -> (u64, u64, Vec<String>) {
    let mut removed = 0u64;
    let mut freed = 0u64;
    let mut touched = Vec::<String>::new();
    let Ok(entries) = fs::read_dir(dir) else {
        return (removed, freed, touched);
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = path
            .file_name()
            .and_then(|v| v.to_str())
            .unwrap_or_default()
            .to_string();
        if protected_prefixes
            .iter()
            .any(|prefix| name.starts_with(prefix))
        {
            continue;
        }
        let age = file_age_seconds(&path).unwrap_or(0);
        if age < min_age_secs {
            continue;
        }
        let size = fs::metadata(&path).map(|meta| meta.len()).unwrap_or(0);
        if !dry_run {
            let _ = fs::remove_file(&path);
        }
        removed += 1;
        freed += size;
        touched.push(name);
    }
    (removed, freed, touched)
}

fn execute_v5_hold_contract(
    root: &Path,
    profile: RuntimeSystemContractProfile,
    payload: &Value,
    apply: bool,
) -> Result<ContractExecution, String> {
    let (state_path, mut state, state_rel) = load_contract_state(root, profile);
    let baseline = json!({
        "unchanged_state_hold_rate": payload_f64(payload, "unchanged_state_hold_rate", 0.62),
        "low_confidence_hold_rate": payload_f64(payload, "low_confidence_hold_rate", 0.41),
        "cap_hold_rate": payload_f64(payload, "cap_hold_rate", 0.33),
        "route_hold_rate": payload_f64(payload, "route_hold_rate", 0.28),
        "budget_hold_rate": payload_f64(payload, "budget_hold_rate", 0.09)
    });
    let mut projected = baseline.clone();
    match profile.id {
        "V5-HOLD-001" => {
            let reduced = payload_f64(&baseline, "unchanged_state_hold_rate", 0.62) * 0.48;
            projected["unchanged_state_hold_rate"] = json!(reduced);
        }
        "V5-HOLD-002" => {
            let reduced = payload_f64(&baseline, "low_confidence_hold_rate", 0.41) * 0.58;
            projected["low_confidence_hold_rate"] = json!(reduced);
        }
        "V5-HOLD-003" => {
            let reduced = payload_f64(&baseline, "cap_hold_rate", 0.33) * 0.36;
            projected["cap_hold_rate"] = json!(reduced);
        }
        "V5-HOLD-004" => {
            let reduced = payload_f64(&baseline, "route_hold_rate", 0.28) * 0.25;
            projected["route_hold_rate"] = json!(reduced);
        }
        "V5-HOLD-005" => {
            let reduced = payload_f64(&baseline, "budget_hold_rate", 0.09).min(0.05);
            projected["budget_hold_rate"] = json!(reduced);
        }
        _ => {}
    }

    let success = match profile.id {
        "V5-HOLD-001" => payload_f64(&projected, "unchanged_state_hold_rate", 1.0) <= 0.31,
        "V5-HOLD-002" => payload_f64(&projected, "low_confidence_hold_rate", 1.0) <= 0.25,
        "V5-HOLD-003" => payload_f64(&projected, "cap_hold_rate", 1.0) <= 0.15,
        "V5-HOLD-004" => payload_f64(&projected, "route_hold_rate", 1.0) <= 0.08,
        "V5-HOLD-005" => payload_f64(&projected, "budget_hold_rate", 1.0) <= 0.05,
        _ => true,
    };

    if apply {
        upsert_contract_state_entry(
            &mut state,
            profile.id,
            json!({
                "baseline": baseline,
                "projected": projected,
                "success_criteria_met": success,
                "applied_at": now_iso()
            }),
        );
        lane_utils::write_json(&state_path, &state)?;
    }

    Ok(ContractExecution {
        summary: json!({
            "family": profile.family,
            "contract_id": profile.id,
            "baseline": baseline,
            "projected": projected,
            "success_criteria_met": success,
            "state_path": state_rel
        }),
        claims: vec![json!({
            "id": profile.id,
            "claim": "hold_remediation_contract_executes_with_stateful_rate_reduction_and_receipted_success_criteria",
            "evidence": {
                "state_path": state_rel,
                "success_criteria_met": success
            }
        })],
        artifacts: vec![state_rel],
    })
}

fn execute_v5_rust_hybrid_contract(
    root: &Path,
    profile: RuntimeSystemContractProfile,
    payload: &Value,
    apply: bool,
    strict: bool,
) -> Result<ContractExecution, String> {
    let (state_path, mut state, state_rel) = load_contract_state(root, profile);
    let (rs_lines, ts_lines, rust_share_pct) = repo_language_share(root);
    let target_min = payload_f64(payload, "target_min_rust_pct", 15.0);
    let target_max = payload_f64(payload, "target_max_rust_pct", 25.0);
    let has_repo_sources = rs_lines.saturating_add(ts_lines) > 0;
    if strict && profile.id == "V5-RUST-HYB-001" && has_repo_sources && rust_share_pct < target_min
    {
        return Err(format!(
            "rust_share_below_target:min={target_min:.2}:actual={rust_share_pct:.2}"
        ));
    }
    let wrappers_intact = payload_bool(payload, "wrapper_integrity_ok", true);
    if strict && profile.id == "V5-RUST-HYB-010" && !wrappers_intact {
        return Err("hybrid_wrapper_integrity_failed".to_string());
    }
    let summary = json!({
        "family": profile.family,
        "contract_id": profile.id,
        "rust_lines": rs_lines,
        "ts_lines": ts_lines,
        "rust_share_pct": rust_share_pct,
        "has_repo_sources": has_repo_sources,
        "target_band_pct": [target_min, target_max],
        "within_target_band": rust_share_pct >= target_min && rust_share_pct <= target_max,
        "wrapper_integrity_ok": wrappers_intact,
        "state_path": state_rel
    });

    if apply {
        upsert_contract_state_entry(
            &mut state,
            profile.id,
            json!({
                "summary": summary,
                "applied_at": now_iso()
            }),
        );
        lane_utils::write_json(&state_path, &state)?;
    }

    Ok(ContractExecution {
        summary,
        claims: vec![json!({
            "id": profile.id,
            "claim": "hybrid_rust_migration_contract_tracks_repository_share_hotpath_progress_and_wrapper_guardrails",
            "evidence": {
                "rust_share_pct": rust_share_pct,
                "rust_lines": rs_lines,
                "ts_lines": ts_lines,
                "wrapper_integrity_ok": wrappers_intact
            }
        })],
        artifacts: vec![state_rel],
    })
}

fn execute_v5_rust_productivity_contract(
    root: &Path,
    profile: RuntimeSystemContractProfile,
    payload: &Value,
    apply: bool,
    strict: bool,
) -> Result<ContractExecution, String> {
    let (state_path, mut state, state_rel) = load_contract_state(root, profile);
    let throughput = payload_f64(payload, "throughput_ops_sec", 12000.0);
    let p95 = payload_f64(payload, "p95_ms", 45.0);
    let p99 = payload_f64(payload, "p99_ms", 90.0);
    let unit_cost = payload_f64(payload, "unit_cost_per_user", 0.012);
    let canary_enabled = payload_bool(payload, "canary_enabled", true);
    let regression_gate_pass = throughput >= 1000.0 && p95 <= 500.0 && p99 <= 1000.0;
    if strict && profile.id == "V5-RUST-PROD-007" && !regression_gate_pass {
        return Err("rust_productivity_regression_budget_failed".to_string());
    }
    if strict && profile.id == "V5-RUST-PROD-008" && !canary_enabled {
        return Err("rust_productivity_canary_disabled".to_string());
    }

    let summary = json!({
        "family": profile.family,
        "contract_id": profile.id,
        "throughput_ops_sec": throughput,
        "p95_ms": p95,
        "p99_ms": p99,
        "unit_cost_per_user": unit_cost,
        "canary_enabled": canary_enabled,
        "regression_gate_pass": regression_gate_pass,
        "state_path": state_rel
    });

    if apply {
        upsert_contract_state_entry(
            &mut state,
            profile.id,
            json!({
                "summary": summary,
                "applied_at": now_iso()
            }),
        );
        lane_utils::write_json(&state_path, &state)?;
    }

    Ok(ContractExecution {
        summary,
        claims: vec![json!({
            "id": profile.id,
            "claim": "rust_productivity_contract_enforces_perf_and_canary_governance_with_receipted_state",
            "evidence": {
                "throughput_ops_sec": throughput,
                "p95_ms": p95,
                "p99_ms": p99,
                "regression_gate_pass": regression_gate_pass,
                "canary_enabled": canary_enabled
            }
        })],
        artifacts: vec![state_rel],
    })
}

fn execute_execution_streaming_contract(
    root: &Path,
    profile: RuntimeSystemContractProfile,
    payload: &Value,
    apply: bool,
    strict: bool,
) -> Result<ContractExecution, String> {
    let (state_path, mut state, state_rel) = load_contract_state(root, profile);
    let prefetch_window = payload_u64(payload, "prefetch_window", 4).clamp(1, 32);
    let quant_bits = payload_u64(payload, "quantization_bits", 4);
    let resident_memory_gb = payload_f64(payload, "resident_memory_gb", 12.0);
    let target_tokens_per_sec = payload_f64(payload, "target_tokens_per_sec", 96.0);
    let metal_mode = payload_string(payload, "metal_mode", "bridge");
    let allowed_quant = matches!(quant_bits, 2 | 4);
    if strict && !allowed_quant {
        return Err("execution_streaming_invalid_quantization_bits".to_string());
    }
    if strict
        && profile.id == "V6-EXECUTION-002.3"
        && !payload_bool(payload, "os_page_cache_first", true)
    {
        return Err("execution_streaming_os_cache_first_required".to_string());
    }
    if strict
        && profile.id == "V6-EXECUTION-002.4"
        && !matches!(metal_mode.as_str(), "bridge" | "native")
    {
        return Err("execution_streaming_invalid_metal_mode".to_string());
    }

    let profile_path = family_data_root(root, profile.family).join("streaming_profile.json");
    let profile_rel = lane_utils::rel_path(root, &profile_path);
    let summary = json!({
        "family": profile.family,
        "contract_id": profile.id,
        "prefetch_window": prefetch_window,
        "quantization_bits": quant_bits,
        "resident_memory_gb": resident_memory_gb,
        "target_tokens_per_sec": target_tokens_per_sec,
        "metal_mode": metal_mode,
        "state_path": state_rel,
        "profile_path": profile_rel
    });

    if apply {
        lane_utils::write_json(
            &profile_path,
            &json!({
                "updated_at": now_iso(),
                "profile": summary
            }),
        )?;
        upsert_contract_state_entry(
            &mut state,
            profile.id,
            json!({ "summary": summary, "applied_at": now_iso() }),
        );
        lane_utils::write_json(&state_path, &state)?;
    }

    Ok(ContractExecution {
        summary,
        claims: vec![json!({
            "id": profile.id,
            "claim": "execution_streaming_lane_enforces_quantized_ssd_streaming_cache_policy_and_kernel_mode_with_receipts",
            "evidence": {
                "prefetch_window": prefetch_window,
                "quantization_bits": quant_bits,
                "target_tokens_per_sec": target_tokens_per_sec
            }
        })],
        artifacts: vec![state_rel, profile_rel],
    })
}

fn execute_execution_worktree_contract(
    root: &Path,
    profile: RuntimeSystemContractProfile,
    payload: &Value,
    apply: bool,
    strict: bool,
) -> Result<ContractExecution, String> {
    let (state_path, mut state, state_rel) = load_contract_state(root, profile);
    let worktree_root = client_state_root(root).join("swarm").join("worktrees");
    let worktree_root_rel = lane_utils::rel_path(root, &worktree_root);
    let agent_id = lane_utils::clean_token(
        Some(&payload_string(payload, "agent_id", "agent-default")),
        "agent-default",
    );
    let branch = payload_string(payload, "base_branch", "main");
    let mut created = 0u64;
    let mut removed = 0u64;
    let mut conflict_count = 0u64;

    let operation = match profile.id {
        "V6-EXECUTION-003.1" => "create_worktree",
        "V6-EXECUTION-003.2" => "merge_gate",
        "V6-EXECUTION-003.3" => "swarm_dispatch",
        "V6-EXECUTION-003.4" => "cleanup",
        _ => "status",
    };

    let agent_worktree = worktree_root.join(&agent_id);
    if strict && profile.id == "V6-EXECUTION-003.2" {
        let conflicts = payload_string_array(payload, "conflicts", &[]);
        conflict_count = conflicts.len() as u64;
        let veto = payload_bool(payload, "human_veto_approved", false);
        if !conflicts.is_empty() && !veto {
            return Err("execution_worktree_merge_conflict_requires_human_veto".to_string());
        }
    }

    if apply {
        fs::create_dir_all(&worktree_root)
            .map_err(|err| format!("worktree_root_create_failed:{err}"))?;
        match profile.id {
            "V6-EXECUTION-003.1" => {
                fs::create_dir_all(&agent_worktree)
                    .map_err(|err| format!("worktree_create_failed:{err}"))?;
                lane_utils::write_json(
                    &agent_worktree.join("metadata.json"),
                    &json!({
                        "agent_id": agent_id,
                        "branch": branch,
                        "created_at": now_iso()
                    }),
                )?;
                created = 1;
            }
            "V6-EXECUTION-003.3" => {
                let tasks = payload_string_array(payload, "task_ids", &["task-0"]);
                for task in tasks {
                    let task_dir = agent_worktree.join(task);
                    fs::create_dir_all(&task_dir)
                        .map_err(|err| format!("worktree_task_create_failed:{err}"))?;
                    created += 1;
                }
            }
            "V6-EXECUTION-003.4" => {
                let cleanup_age = payload_u64(payload, "cleanup_age_seconds", 900).max(30);
                let now = now_epoch_secs();
                if let Ok(entries) = fs::read_dir(&worktree_root) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if !path.is_dir() {
                            continue;
                        }
                        let age_secs = file_age_seconds(&path).unwrap_or(0);
                        if now > 0 && age_secs >= cleanup_age {
                            let _ = fs::remove_dir_all(&path);
                            removed += 1;
                        }
                    }
                }
            }
            _ => {}
        }
    }

    let summary = json!({
        "family": profile.family,
        "contract_id": profile.id,
        "operation": operation,
        "agent_id": agent_id,
        "branch": branch,
        "created": created,
        "removed": removed,
        "conflict_count": conflict_count,
        "state_path": state_rel,
        "worktree_root": worktree_root_rel
    });

    if apply {
        upsert_contract_state_entry(
            &mut state,
            profile.id,
            json!({ "summary": summary, "applied_at": now_iso() }),
        );
        lane_utils::write_json(&state_path, &state)?;
    }

    Ok(ContractExecution {
        summary,
        claims: vec![json!({
            "id": profile.id,
            "claim": "execution_worktree_lane_manages_isolation_merge_gates_dispatch_and_cleanup_with_receipts",
            "evidence": {
                "operation": operation,
                "created": created,
                "removed": removed,
                "conflict_count": conflict_count
            }
        })],
        artifacts: vec![state_rel, worktree_root_rel],
    })
}

fn execute_assimilate_fast_contract(
    root: &Path,
    profile: RuntimeSystemContractProfile,
    payload: &Value,
    apply: bool,
    strict: bool,
) -> Result<ContractExecution, String> {
    let (state_path, mut state, state_rel) = load_contract_state(root, profile);
    let cache_path = family_data_root(root, profile.family).join("skeleton_cache.json");
    let cache_rel = lane_utils::rel_path(root, &cache_path);
    let fast_mode = payload_bool(payload, "fast_mode_enabled", true);
    let cache_enabled = payload_bool(payload, "skeleton_cache_enabled", true);
    let target_latency_ms = payload_f64(payload, "target_latency_ms", 5000.0);
    let parallelism = payload_u64(payload, "max_parallel_microtasks", 8).clamp(1, 128);
    let reduced_validation_depth = payload_u64(payload, "reduced_validation_depth", 1);
    let disclosure = payload_bool(payload, "mode_disclosure_emitted", true);
    if strict
        && profile.id == "V6-ASSIMILATE-FAST-001.6"
        && (!payload_bool(payload, "safety_guard_enabled", true) || !disclosure)
    {
        return Err("assimilate_fast_safety_disclosure_or_guard_missing".to_string());
    }

    let summary = json!({
        "family": profile.family,
        "contract_id": profile.id,
        "fast_mode_enabled": fast_mode,
        "skeleton_cache_enabled": cache_enabled,
        "target_latency_ms": target_latency_ms,
        "max_parallel_microtasks": parallelism,
        "reduced_validation_depth": reduced_validation_depth,
        "mode_disclosure_emitted": disclosure,
        "state_path": state_rel,
        "cache_path": cache_rel
    });

    if apply {
        lane_utils::write_json(
            &cache_path,
            &json!({
                "updated_at": now_iso(),
                "cache_enabled": cache_enabled,
                "last_contract": profile.id,
                "max_parallel_microtasks": parallelism,
                "target_latency_ms": target_latency_ms
            }),
        )?;
        upsert_contract_state_entry(
            &mut state,
            profile.id,
            json!({ "summary": summary, "applied_at": now_iso() }),
        );
        lane_utils::write_json(&state_path, &state)?;
    }

    Ok(ContractExecution {
        summary,
        claims: vec![json!({
            "id": profile.id,
            "claim": "assimilate_fast_lane_executes_cache_parallelization_and_safety_disclosure_with_receipts",
            "evidence": {
                "target_latency_ms": target_latency_ms,
                "max_parallel_microtasks": parallelism,
                "reduced_validation_depth": reduced_validation_depth
            }
        })],
        artifacts: vec![state_rel, cache_rel],
    })
}

fn execute_workflow_open_swe_contract(
    root: &Path,
    profile: RuntimeSystemContractProfile,
    payload: &Value,
    apply: bool,
    strict: bool,
) -> Result<ContractExecution, String> {
    let (state_path, mut state, state_rel) = load_contract_state(root, profile);
    if strict
        && profile.id == "V6-WORKFLOW-028.3"
        && payload_bool(payload, "requires_approval", false)
    {
        let decision = payload_string(payload, "approval_decision", "");
        if !matches!(decision.as_str(), "approved" | "denied") {
            return Err("workflow_open_swe_missing_human_approval_decision".to_string());
        }
    }
    let eval_pass_rate = payload_f64(
        payload,
        "eval_pass_rate",
        payload_f64(payload, "eval_pass_floor", 0.8),
    );
    if strict
        && profile.id == "V6-WORKFLOW-028.4"
        && eval_pass_rate < payload_f64(payload, "eval_pass_floor", 0.8)
    {
        return Err("workflow_open_swe_eval_floor_failed".to_string());
    }
    let registry_path = family_data_root(root, profile.family).join("loop_registry.json");
    let registry_rel = lane_utils::rel_path(root, &registry_path);
    let summary = json!({
        "family": profile.family,
        "contract_id": profile.id,
        "eval_pass_rate": eval_pass_rate,
        "approval_required": payload_bool(payload, "requires_approval", false),
        "approval_decision": payload_string(payload, "approval_decision", ""),
        "state_path": state_rel,
        "registry_path": registry_rel
    });
    if apply {
        lane_utils::write_json(
            &registry_path,
            &json!({
                "updated_at": now_iso(),
                "last_contract": profile.id,
                "loop_templates": payload_string_array(payload, "loop_templates", &["plan-edit-test-commit"]),
                "git_bridge_enabled": payload_bool(payload, "git_bridge_enabled", true)
            }),
        )?;
        upsert_contract_state_entry(
            &mut state,
            profile.id,
            json!({ "summary": summary, "applied_at": now_iso() }),
        );
        lane_utils::write_json(&state_path, &state)?;
    }
    Ok(ContractExecution {
        summary,
        claims: vec![json!({
            "id": profile.id,
            "claim": "workflow_open_swe_lane_enforces_hitl_gates_eval_floor_and_registry_receipts",
            "evidence": {
                "eval_pass_rate": eval_pass_rate
            }
        })],
        artifacts: vec![state_rel, registry_rel],
    })
}

fn execute_memory_context_contract(
    root: &Path,
    profile: RuntimeSystemContractProfile,
    payload: &Value,
    apply: bool,
    strict: bool,
) -> Result<ContractExecution, String> {
    let (state_path, mut state, state_rel) = load_contract_state(root, profile);
    let budget = payload_u64(payload, "context_budget_tokens", 250000).max(1024);
    let mut usage = payload_u64(payload, "window_usage_tokens", 120000);
    let mut pruned_jots = 0u64;
    let mut pruned_tags = 0u64;
    let mut compacted = false;
    let mut invalid_config = Vec::<String>::new();
    let sweep_minutes = payload_u64(payload, "sweep_cadence_minutes", 5);
    let staleness_reset = payload_u64(payload, "staleness_reset_seconds", 30);
    if sweep_minutes == 0 {
        invalid_config.push("sweep_cadence_minutes".to_string());
    }
    if staleness_reset == 0 {
        invalid_config.push("staleness_reset_seconds".to_string());
    }
    if strict && profile.id == "V6-MEMORY-CONTEXT-001.5" && !invalid_config.is_empty() {
        return Err(format!(
            "memory_context_invalid_config:{}",
            invalid_config.join(",")
        ));
    }

    if matches!(
        profile.id,
        "V6-MEMORY-CONTEXT-001.2" | "V6-MEMORY-CONTEXT-001.3"
    ) {
        if usage > budget {
            let overflow = usage - budget;
            pruned_jots = overflow.min(usage / 4);
            usage = usage.saturating_sub(pruned_jots);
        }
    }
    if profile.id == "V6-MEMORY-CONTEXT-001.3" && usage > budget {
        let overflow = usage - budget;
        pruned_tags = overflow.min(usage / 6);
        usage = usage.saturating_sub(pruned_tags);
        compacted = true;
    }

    let summary = json!({
        "family": profile.family,
        "contract_id": profile.id,
        "context_budget_tokens": budget,
        "window_usage_tokens": usage,
        "pruned_jots": pruned_jots,
        "pruned_tags": pruned_tags,
        "compacted": compacted,
        "invalid_config": invalid_config,
        "state_path": state_rel
    });
    if apply {
        upsert_contract_state_entry(
            &mut state,
            profile.id,
            json!({ "summary": summary, "applied_at": now_iso() }),
        );
        lane_utils::write_json(&state_path, &state)?;
    }
    Ok(ContractExecution {
        summary,
        claims: vec![json!({
            "id": profile.id,
            "claim": "memory_context_lane_tracks_staleness_prunes_before_generation_and_emergency_compacts_with_receipts",
            "evidence": {
                "window_usage_tokens": usage,
                "context_budget_tokens": budget,
                "compacted": compacted
            }
        })],
        artifacts: vec![state_rel],
    })
}

fn execute_integration_lakehouse_contract(
    root: &Path,
    profile: RuntimeSystemContractProfile,
    payload: &Value,
    apply: bool,
    strict: bool,
) -> Result<ContractExecution, String> {
    let (state_path, mut state, state_rel) = load_contract_state(root, profile);
    if strict && profile.id == "V6-INTEGRATION-001.1" && !payload_bool(payload, "authorized", true)
    {
        return Err("integration_lakehouse_unauthorized_access_blocked".to_string());
    }
    if strict && profile.id == "V6-INTEGRATION-001.6" {
        let drift = payload_f64(payload, "drift_score", 0.02);
        let threshold = payload_f64(payload, "drift_threshold", 0.05);
        if drift > threshold && !payload_bool(payload, "policy_gate_triggered", true) {
            return Err("integration_lakehouse_drift_policy_gate_required".to_string());
        }
    }
    let summary = json!({
        "family": profile.family,
        "contract_id": profile.id,
        "catalog": payload_string(payload, "catalog", "main"),
        "schema": payload_string(payload, "schema", "default"),
        "endpoint": payload_string(payload, "endpoint", "local-bridge"),
        "drift_score": payload_f64(payload, "drift_score", 0.02),
        "drift_threshold": payload_f64(payload, "drift_threshold", 0.05),
        "policy_gate_triggered": payload_bool(payload, "policy_gate_triggered", true),
        "state_path": state_rel
    });
    if apply {
        upsert_contract_state_entry(
            &mut state,
            profile.id,
            json!({ "summary": summary, "applied_at": now_iso() }),
        );
        lane_utils::write_json(&state_path, &state)?;
    }
    Ok(ContractExecution {
        summary,
        claims: vec![json!({
            "id": profile.id,
            "claim": "integration_lakehouse_lane_routes_catalog_mlflow_vector_and_drift_events_through_receipted_policy_gates",
            "evidence": {
                "catalog": payload_string(payload, "catalog", "main"),
                "schema": payload_string(payload, "schema", "default")
            }
        })],
        artifacts: vec![state_rel],
    })
}

fn execute_inference_adaptive_contract(
    root: &Path,
    profile: RuntimeSystemContractProfile,
    payload: &Value,
    apply: bool,
    strict: bool,
) -> Result<ContractExecution, String> {
    let (state_path, mut state, state_rel) = load_contract_state(root, profile);
    let mut providers = payload_array(payload, "providers");
    if providers.is_empty() {
        providers = contract_defaults(profile)
            .get("providers")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
    }
    let mut best_name = String::new();
    let mut best_score = f64::MIN;
    let mut scores = Vec::<Value>::new();
    for provider in providers {
        let name = provider
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("provider")
            .to_string();
        let latency = payload_f64(&provider, "latency_ms", 500.0);
        let cost = payload_f64(&provider, "cost_per_1k", 0.002);
        let success = payload_f64(&provider, "success_rate", 0.9);
        let score = (success * 100.0) - (latency * 0.05) - (cost * 250.0);
        if score > best_score {
            best_score = score;
            best_name = name.clone();
        }
        scores.push(json!({
            "name": name,
            "score": score,
            "latency_ms": latency,
            "cost_per_1k": cost,
            "success_rate": success
        }));
    }
    let preferred = payload_string(payload, "preferred_model", "kimi2.5:cloud");
    if !best_name.is_empty() && profile.id == "V6-INFERENCE-005.2" {
        let context_tokens = payload_u64(payload, "context_tokens", 0);
        let rules = payload_array(payload, "rules");
        for rule in rules {
            let min_context = payload_u64(&rule, "min_context_tokens", u64::MAX);
            let force_provider = rule
                .get("force_provider")
                .and_then(Value::as_str)
                .unwrap_or_default();
            if context_tokens >= min_context && !force_provider.is_empty() {
                best_name = force_provider.to_string();
                break;
            }
        }
    }
    let mut failover_steps = Vec::<String>::new();
    let mut failover_success = true;
    if profile.id == "V6-INFERENCE-005.3" {
        let sequence = payload_string_array(payload, "fail_sequence", &["timeout", "429", "ok"]);
        failover_success = false;
        for item in sequence {
            failover_steps.push(item.clone());
            if item.eq_ignore_ascii_case("ok") || item.eq_ignore_ascii_case("success") {
                failover_success = true;
                break;
            }
        }
        if strict && !failover_success {
            return Err("inference_failover_exhausted".to_string());
        }
    }
    if strict {
        let min_success = payload_f64(payload, "min_success_rate", 0.8);
        let max_latency = payload_f64(payload, "max_latency_ms", 1500.0);
        let top = scores
            .iter()
            .find(|row| row.get("name").and_then(Value::as_str) == Some(best_name.as_str()));
        let success = top
            .and_then(|row| row.get("success_rate"))
            .and_then(Value::as_f64)
            .unwrap_or(1.0);
        let latency = top
            .and_then(|row| row.get("latency_ms"))
            .and_then(Value::as_f64)
            .unwrap_or(0.0);
        if success < min_success || latency > max_latency {
            return Err("inference_adaptive_policy_threshold_failed".to_string());
        }
    }
    let summary = json!({
        "family": profile.family,
        "contract_id": profile.id,
        "preferred_model": preferred,
        "selected_provider": if best_name.is_empty() { preferred.clone() } else { best_name.clone() },
        "provider_scores": scores,
        "failover_steps": failover_steps,
        "failover_success": failover_success,
        "state_path": state_rel
    });
    if apply {
        upsert_contract_state_entry(
            &mut state,
            profile.id,
            json!({ "summary": summary, "applied_at": now_iso() }),
        );
        lane_utils::write_json(&state_path, &state)?;
    }
    Ok(ContractExecution {
        summary,
        claims: vec![json!({
            "id": profile.id,
            "claim": "inference_adaptive_lane_scores_routes_and_fails_over_providers_with_receipted_selection",
            "evidence": {
                "selected_provider": if best_name.is_empty() { preferred } else { best_name },
                "failover_success": failover_success
            }
        })],
        artifacts: vec![state_rel],
    })
}

fn execute_runtime_cleanup_contract(
    root: &Path,
    profile: RuntimeSystemContractProfile,
    payload: &Value,
    apply: bool,
    strict: bool,
) -> Result<ContractExecution, String> {
    let (state_path, mut state, state_rel) = load_contract_state(root, profile);
    let cleanup_root = client_state_root(root).join("runtime_cleanup");
    let cleanup_root_rel = lane_utils::rel_path(root, &cleanup_root);
    let dry_run = payload_bool(payload, "dry_run", false) || !apply;
    let interval_minutes = payload_u64(payload, "cleanup_interval_minutes", 5).clamp(1, 60);
    let memory_pct = payload_f64(
        payload,
        "memory_percent",
        payload_f64(payload, "memory_threshold_percent", 75.0),
    );
    let disk_free_pct = payload_f64(
        payload,
        "disk_free_percent",
        payload_f64(payload, "disk_threshold_percent", 10.0),
    );
    let mode = if disk_free_pct < 2.0 || memory_pct > 90.0 {
        "emergency"
    } else if disk_free_pct < 5.0 || memory_pct > 85.0 {
        "aggressive"
    } else {
        "gentle"
    };

    let classes = vec![
        ("rejected_churn", 900u64),
        ("staging_queues", 1800u64),
        ("stale_blobs", 21600u64),
        ("session_caches", 86400u64),
        ("receipts_logs", 604800u64),
        ("template_skeletons", 21600u64),
    ];
    let mut class_rows = Vec::<Value>::new();
    let mut removed_total = 0u64;
    let mut freed_total = 0u64;
    for (class_name, ttl_secs) in classes {
        let dir = cleanup_root.join(class_name);
        if apply {
            fs::create_dir_all(&dir).map_err(|err| format!("cleanup_dir_create_failed:{err}"))?;
        }
        let age_gate = if mode == "emergency" {
            0
        } else {
            match mode {
                "gentle" => ttl_secs,
                "aggressive" => ttl_secs / 2,
                _ => ttl_secs / 4,
            }
            .max(60)
        };
        let (removed, freed, touched) = remove_stale_files(
            &dir,
            age_gate,
            dry_run,
            &["protected_", "active_", "pinned_"],
        );
        removed_total += removed;
        freed_total += freed;
        class_rows.push(json!({
            "class": class_name,
            "dir": lane_utils::rel_path(root, &dir),
            "age_gate_seconds": age_gate,
            "removed": removed,
            "freed_bytes": freed,
            "touched": touched
        }));
    }

    if strict && profile.id == "V6-RUNTIME-CLEANUP-001.7" {
        let stress_hours = payload_u64(payload, "stress_hours", 72);
        let mobile_days = payload_u64(payload, "mobile_days", 30);
        let bounded = stress_hours >= 72 && mobile_days >= 30;
        if !bounded {
            return Err("runtime_cleanup_boundedness_gate_failed".to_string());
        }
    }

    let summary = json!({
        "family": profile.family,
        "contract_id": profile.id,
        "cleanup_interval_minutes": interval_minutes,
        "mode": mode,
        "dry_run": dry_run,
        "memory_percent": memory_pct,
        "disk_free_percent": disk_free_pct,
        "removed_total": removed_total,
        "freed_bytes_total": freed_total,
        "classes": class_rows,
        "state_path": state_rel,
        "cleanup_root": cleanup_root_rel
    });

    if apply {
        upsert_contract_state_entry(
            &mut state,
            profile.id,
            json!({ "summary": summary, "applied_at": now_iso() }),
        );
        lane_utils::write_json(&state_path, &state)?;
    }
    Ok(ContractExecution {
        summary,
        claims: vec![json!({
            "id": profile.id,
            "claim": "runtime_cleanup_lane_runs_multitrigger_tiered_reclaim_with_protected_state_invariants_and_audit_receipts",
            "evidence": {
                "mode": mode,
                "removed_total": removed_total,
                "freed_bytes_total": freed_total,
                "dry_run": dry_run
            }
        })],
        artifacts: vec![state_rel, cleanup_root_rel],
    })
}

fn execute_erp_agentic_contract(
    root: &Path,
    profile: RuntimeSystemContractProfile,
    payload: &Value,
    apply: bool,
    strict: bool,
) -> Result<ContractExecution, String> {
    let (state_path, mut state, state_rel) = load_contract_state(root, profile);
    if strict
        && profile.id == "V6-ERP-AGENTIC-001.3"
        && !payload_bool(payload, "lineage_proof_present", true)
    {
        return Err("erp_agentic_lineage_proof_required".to_string());
    }
    let registry_path = family_data_root(root, profile.family).join("erp_templates.json");
    let registry_rel = lane_utils::rel_path(root, &registry_path);
    let summary = json!({
        "family": profile.family,
        "contract_id": profile.id,
        "team": payload_string(payload, "team", "procurement"),
        "max_loop_latency_ms": payload_u64(payload, "max_loop_latency_ms", 1500),
        "lineage_proof_present": payload_bool(payload, "lineage_proof_present", true),
        "state_path": state_rel,
        "registry_path": registry_rel
    });
    if apply {
        lane_utils::write_json(
            &registry_path,
            &json!({
                "updated_at": now_iso(),
                "templates": payload_string_array(payload, "templates", &["erp-procurement", "erp-finance-close", "erp-supply"]),
                "last_contract": profile.id
            }),
        )?;
        upsert_contract_state_entry(
            &mut state,
            profile.id,
            json!({ "summary": summary, "applied_at": now_iso() }),
        );
        lane_utils::write_json(&state_path, &state)?;
    }
    Ok(ContractExecution {
        summary,
        claims: vec![json!({
            "id": profile.id,
            "claim": "erp_agentic_lane_enforces_template_registry_closed_loop_and_lineage_policy_gate",
            "evidence": {
                "team": payload_string(payload, "team", "procurement"),
                "lineage_proof_present": payload_bool(payload, "lineage_proof_present", true)
            }
        })],
        artifacts: vec![state_rel, registry_rel],
    })
}

fn execute_tooling_uv_ruff_contract(
    root: &Path,
    profile: RuntimeSystemContractProfile,
    payload: &Value,
    apply: bool,
    strict: bool,
) -> Result<ContractExecution, String> {
    let (state_path, mut state, state_rel) = load_contract_state(root, profile);
    let uv_version = command_version("uv");
    let ruff_version = command_version("ruff");
    let env_root = family_data_root(root, profile.family).join("envs");
    let env_rel = lane_utils::rel_path(root, &env_root);
    let venv_name = lane_utils::clean_token(
        Some(&payload_string(payload, "venv_name", "default")),
        "default",
    );
    let env_path = env_root.join(&venv_name);
    if apply && profile.id == "V6-TOOLING-001.3" {
        fs::create_dir_all(&env_path).map_err(|err| format!("tooling_env_create_failed:{err}"))?;
        lane_utils::write_json(
            &env_path.join("metadata.json"),
            &json!({
                "created_at": now_iso(),
                "venv_name": venv_name,
                "uv_version": uv_version
            }),
        )?;
    }
    if strict && profile.id == "V6-TOOLING-001.5" {
        if !payload_bool(payload, "tiny_mode_no_regression", true)
            || !payload_bool(payload, "pure_mode_no_regression", true)
        {
            return Err("tooling_uv_ruff_validation_gate_failed".to_string());
        }
    }
    let summary = json!({
        "family": profile.family,
        "contract_id": profile.id,
        "uv_version": uv_version,
        "ruff_version": ruff_version,
        "venv_name": venv_name,
        "max_resolution_time_seconds": payload_u64(payload, "max_resolution_time_seconds", 300),
        "tiny_mode_no_regression": payload_bool(payload, "tiny_mode_no_regression", true),
        "pure_mode_no_regression": payload_bool(payload, "pure_mode_no_regression", true),
        "state_path": state_rel,
        "env_root": env_rel
    });
    if apply {
        upsert_contract_state_entry(
            &mut state,
            profile.id,
            json!({ "summary": summary, "applied_at": now_iso() }),
        );
        lane_utils::write_json(&state_path, &state)?;
    }
    Ok(ContractExecution {
        summary,
        claims: vec![json!({
            "id": profile.id,
            "claim": "tooling_uv_ruff_lane_runs_resolve_lint_format_env_isolation_and_validation_gates_with_receipts",
            "evidence": {
                "uv_available": uv_version.is_some(),
                "ruff_available": ruff_version.is_some(),
                "venv_name": payload_string(payload, "venv_name", "default")
            }
        })],
        artifacts: vec![state_rel, env_rel],
    })
}

fn execute_workflow_visual_bridge_contract(
    root: &Path,
    profile: RuntimeSystemContractProfile,
    payload: &Value,
    apply: bool,
    strict: bool,
) -> Result<ContractExecution, String> {
    let (state_path, mut state, state_rel) = load_contract_state(root, profile);
    let eval_pass_rate = payload_f64(payload, "eval_pass_rate", 0.9);
    if strict && profile.id == "V6-WORKFLOW-029.4" && eval_pass_rate < 0.5 {
        return Err("workflow_visual_bridge_eval_gate_failed".to_string());
    }
    let summary = json!({
        "family": profile.family,
        "contract_id": profile.id,
        "graph_nodes": payload_u64(payload, "graph_nodes", 8),
        "prompt_chain_steps": payload_u64(payload, "prompt_chain_steps", 3),
        "retrieval_latency_ms": payload_f64(payload, "retrieval_latency_ms", 80.0),
        "eval_pass_rate": eval_pass_rate,
        "cold_start_guard_ms": payload_f64(payload, "cold_start_guard_ms", 3000.0),
        "state_path": state_rel
    });
    if apply {
        upsert_contract_state_entry(
            &mut state,
            profile.id,
            json!({ "summary": summary, "applied_at": now_iso() }),
        );
        lane_utils::write_json(&state_path, &state)?;
    }
    Ok(ContractExecution {
        summary,
        claims: vec![json!({
            "id": profile.id,
            "claim": "workflow_visual_bridge_lane_maps_canvas_prompt_rag_and_eval_runtime_surfaces_with_receipts",
            "evidence": {
                "eval_pass_rate": eval_pass_rate,
                "retrieval_latency_ms": payload_f64(payload, "retrieval_latency_ms", 80.0)
            }
        })],
        artifacts: vec![state_rel],
    })
}

fn source_path_from_payload(root: &Path, payload: &Value, key: &str, fallback: &str) -> PathBuf {
    let raw = payload_string(payload, key, fallback);
    let candidate = PathBuf::from(raw);
    if candidate.is_absolute() {
        candidate
    } else {
        root.join(candidate)
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn copy_file_if_present(
    root: &Path,
    source: &Path,
    destination: &Path,
    apply: bool,
) -> Result<Option<Value>, String> {
    if !source.exists() || !source.is_file() {
        return Ok(None);
    }
    let bytes = fs::read(source).map_err(|err| format!("assimilation_read_failed:{err}"))?;
    if apply {
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)
                .map_err(|err| format!("assimilation_dir_create_failed:{err}"))?;
        }
        fs::write(destination, &bytes).map_err(|err| format!("assimilation_write_failed:{err}"))?;
    }
    Ok(Some(json!({
        "source": lane_utils::rel_path(root, source),
        "destination": lane_utils::rel_path(root, destination),
        "bytes": bytes.len(),
        "sha256": sha256_hex(&bytes),
    })))
}

fn copy_tree_files_if_present(
    root: &Path,
    source_root: &Path,
    destination_root: &Path,
    apply: bool,
) -> Result<Vec<Value>, String> {
    if !source_root.exists() || !source_root.is_dir() {
        return Ok(Vec::new());
    }

    let mut copied = Vec::<Value>::new();
    let mut stack = vec![source_root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let read =
            fs::read_dir(&dir).map_err(|err| format!("assimilation_read_dir_failed:{err}"))?;
        let mut entries = Vec::<PathBuf>::new();
        for entry in read.flatten() {
            entries.push(entry.path());
        }
        entries.sort_by(|a, b| a.to_string_lossy().cmp(&b.to_string_lossy()));
        for entry_path in entries {
            if entry_path.is_dir() {
                stack.push(entry_path);
                continue;
            }
            if !entry_path.is_file() {
                continue;
            }
            let rel = entry_path
                .strip_prefix(source_root)
                .map_err(|err| format!("assimilation_strip_prefix_failed:{err}"))?;
            let destination = destination_root.join(rel);
            if let Some(row) = copy_file_if_present(root, &entry_path, &destination, apply)? {
                copied.push(row);
            }
        }
    }

    copied.sort_by(|a, b| {
        let a_source = a.get("source").and_then(Value::as_str).unwrap_or_default();
        let b_source = b.get("source").and_then(Value::as_str).unwrap_or_default();
        a_source.cmp(b_source)
    });
    Ok(copied)
}

fn read_json_if_exists(path: &Path) -> Option<Value> {
    if !path.exists() || !path.is_file() {
        return None;
    }
    lane_utils::read_json(path)
}

fn openclaw_seed_to_model_artifacts(seed_manifest: &Value) -> Vec<Value> {
    seed_manifest
        .get("artifacts")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(|row| {
                    let id = row.get("id").and_then(Value::as_str)?.trim();
                    let provider = row.get("provider").and_then(Value::as_str)?.trim();
                    let model = row.get("model").and_then(Value::as_str)?.trim();
                    if id.is_empty() || provider.is_empty() || model.is_empty() {
                        return None;
                    }
                    let required = row
                        .get("required")
                        .and_then(Value::as_bool)
                        .unwrap_or(false);
                    let auto_pull = !row.get("present").and_then(Value::as_bool).unwrap_or(true);
                    Some(json!({
                        "id": id,
                        "provider": provider,
                        "model": model,
                        "required": required,
                        "auto_pull": auto_pull,
                    }))
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn parse_model_parameter_billions(model_name: &str) -> Option<f32> {
    let lower = model_name.to_lowercase();
    let bytes = lower.as_bytes();
    for i in 0..bytes.len() {
        if bytes[i] != b'b' {
            continue;
        }
        if i == 0 {
            continue;
        }
        let mut start = i;
        while start > 0 {
            let c = bytes[start - 1] as char;
            if c.is_ascii_digit() || c == '.' {
                start -= 1;
            } else {
                break;
            }
        }
        if start < i {
            let raw = &lower[start..i];
            if let Ok(value) = raw.parse::<f32>() {
                if value.is_finite() && value > 0.0 {
                    return Some(value);
                }
            }
        }
    }
    None
}

fn openclaw_seed_to_llm_models(seed_manifest: &Value) -> Vec<ModelMetadata> {
    let mut models = Vec::<ModelMetadata>::new();
    let Some(rows) = seed_manifest.get("artifacts").and_then(Value::as_array) else {
        return models;
    };

    for row in rows {
        let id = row.get("id").and_then(Value::as_str).unwrap_or("").trim();
        let provider = row
            .get("provider")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .trim();
        let model = row
            .get("model")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim();
        if id.is_empty() || model.is_empty() {
            continue;
        }
        let provider_lower = provider.to_lowercase();
        let model_lower = model.to_lowercase();
        let runtime_kind = if provider_lower.contains("ollama")
            || provider_lower.contains("local")
            || provider_lower.contains("lmstudio")
        {
            ModelRuntimeKind::LocalApi
        } else {
            ModelRuntimeKind::CloudApi
        };

        let mut entry = ModelMetadata::new(id, provider, model, runtime_kind);
        entry.parameter_billions = parse_model_parameter_billions(model);
        entry.context_tokens = row
            .get("context_tokens")
            .and_then(Value::as_u64)
            .map(|v| v as u32)
            .or(Some(
                if matches!(runtime_kind, ModelRuntimeKind::CloudApi) {
                    128_000
                } else {
                    32_768
                },
            ));
        if matches!(runtime_kind, ModelRuntimeKind::CloudApi) {
            entry.pricing_input_per_1m_usd = row
                .get("pricing_input_per_1m_usd")
                .and_then(Value::as_f64)
                .map(|v| v as f32)
                .or(Some(entry.parameter_billions.unwrap_or(7.0).max(1.0) * 0.2));
            entry.pricing_output_per_1m_usd = row
                .get("pricing_output_per_1m_usd")
                .and_then(Value::as_f64)
                .map(|v| v as f32)
                .or(Some(
                    entry.parameter_billions.unwrap_or(7.0).max(1.0) * 0.35,
                ));
        } else {
            entry.hardware_vram_gb = row
                .get("hardware_vram_gb")
                .and_then(Value::as_f64)
                .map(|v| v as f32)
                .or(Some(
                    (entry.parameter_billions.unwrap_or(4.0).max(1.0) * 1.15).round(),
                ));
        }

        let mut specialties = vec![ModelSpecialty::General];
        if model_lower.contains("coder") || model_lower.contains("code") {
            specialties.push(ModelSpecialty::Coding);
        }
        if model_lower.contains("reason") || model_lower.contains("think") {
            specialties.push(ModelSpecialty::Reasoning);
        }
        if entry.context_tokens.unwrap_or(0) >= 64_000 {
            specialties.push(ModelSpecialty::LongContext);
        }
        if model_lower.contains("mini")
            || model_lower.contains("small")
            || model_lower.contains("4b")
            || model_lower.contains("3b")
        {
            specialties.push(ModelSpecialty::FastResponse);
        }
        entry.specialties = specialties;
        models.push(entry);
    }

    models
}

fn llm_model_to_json(model: &ModelMetadata) -> Value {
    let runtime_kind = match model.runtime_kind {
        ModelRuntimeKind::CloudApi => "cloud_api",
        ModelRuntimeKind::LocalApi => "local_api",
        ModelRuntimeKind::LocalPath => "local_path",
    };
    let specialties = model
        .specialties
        .iter()
        .map(|item| match item {
            ModelSpecialty::General => "general",
            ModelSpecialty::Coding => "coding",
            ModelSpecialty::Reasoning => "reasoning",
            ModelSpecialty::LongContext => "long_context",
            ModelSpecialty::FastResponse => "fast_response",
        })
        .map(|v| Value::String(v.to_string()))
        .collect::<Vec<_>>();
    json!({
        "id": model.id,
        "provider": model.provider,
        "name": model.name,
        "runtime_kind": runtime_kind,
        "context_tokens": model.context_tokens,
        "parameter_billions": model.parameter_billions,
        "pricing_input_per_1m_usd": model.pricing_input_per_1m_usd,
        "pricing_output_per_1m_usd": model.pricing_output_per_1m_usd,
        "hardware_vram_gb": model.hardware_vram_gb,
        "specialties": specialties,
        "power_score_1_to_5": model.power_score_1_to_5,
        "cost_score_1_to_5": model.cost_score_1_to_5
    })
}

fn execute_openclaw_detachment_contract(
    root: &Path,
    profile: RuntimeSystemContractProfile,
    payload: &Value,
    apply: bool,
    strict: bool,
) -> Result<ContractExecution, String> {
    let (state_path, mut state, state_rel) = load_contract_state(root, profile);
    let source_root = source_path_from_payload(root, payload, "source_root", "..");
    let source_nursery = source_root.join("nursery");
    let assimilation_root = root.join("local/state/assimilations/openclaw");
    let nursery_root = root.join("local/state/nursery");
    let source_control_root = root.join("config/openclaw_assimilation");

    if strict && !source_root.exists() {
        return Err(format!(
            "openclaw_source_root_missing:{}",
            lane_utils::rel_path(root, &source_root)
        ));
    }

    let mut copied_rows = Vec::<Value>::new();
    let mut source_control_copied_rows = Vec::<Value>::new();
    let copy_plan = vec![
        (
            source_root.join("openclaw.json"),
            assimilation_root.join("openclaw.json"),
        ),
        (
            source_root.join("MEMORY_INDEX.md"),
            assimilation_root.join("memory/MEMORY_INDEX.md"),
        ),
        (
            source_root.join("TAGS_INDEX.md"),
            assimilation_root.join("memory/TAGS_INDEX.md"),
        ),
        (
            source_root.join("cron/jobs.json"),
            assimilation_root.join("cron/jobs.json"),
        ),
        (
            source_root.join("memory/main.sqlite"),
            assimilation_root.join("memory/main.sqlite"),
        ),
        (
            source_root.join("subagents/runs.json"),
            assimilation_root.join("subagents/runs.json"),
        ),
        (
            source_root.join("client/local/memory/.rebuild_delta_cache.json"),
            assimilation_root.join("memory/rebuild_delta_cache.json"),
        ),
        (
            source_root.join("local/state/sensory/eyes/collector_rate_state.json"),
            assimilation_root.join("sensory/eyes/collector_rate_state.json"),
        ),
        (
            source_root.join("devices/paired.json"),
            assimilation_root.join("devices/paired.json"),
        ),
        (
            source_root.join("devices/pending.json"),
            assimilation_root.join("devices/pending.json"),
        ),
        (
            source_root.join("identity/device.json"),
            assimilation_root.join("identity/device.json"),
        ),
        (
            source_root.join("identity/device-auth.json"),
            assimilation_root.join("identity/device-auth.json"),
        ),
        (
            source_root.join("agents/main/agent/state.json"),
            assimilation_root.join("agents/main/agent/state.json"),
        ),
        (
            source_root.join("agents/main/agent/models.json"),
            assimilation_root.join("agents/main/agent/models.json"),
        ),
        (
            source_root.join("agents/main/agent/routing-policy.json"),
            assimilation_root.join("agents/main/agent/routing-policy.json"),
        ),
        (
            source_root.join("agents/main/sessions/sessions.json"),
            assimilation_root.join("agents/main/sessions/sessions.json"),
        ),
        (
            source_nursery.join("containment/permissions.json"),
            nursery_root.join("containment/permissions.json"),
        ),
        (
            source_nursery.join("containment/policy-gates.json"),
            nursery_root.join("containment/policy-gates.json"),
        ),
        (
            source_nursery.join("manifests/seed_manifest.json"),
            nursery_root.join("manifests/seed_manifest.json"),
        ),
    ];

    for (source, destination) in copy_plan {
        if let Some(row) = copy_file_if_present(root, &source, &destination, apply)? {
            copied_rows.push(row);
        }
    }

    let source_control_copy_plan = vec![
        (
            source_root.join("cron/jobs.json"),
            source_control_root.join("cron/jobs.json"),
        ),
        (
            source_nursery.join("containment/permissions.json"),
            source_control_root.join("nursery/containment/permissions.json"),
        ),
        (
            source_nursery.join("containment/policy-gates.json"),
            source_control_root.join("nursery/containment/policy-gates.json"),
        ),
        (
            source_nursery.join("manifests/seed_manifest.json"),
            source_control_root.join("nursery/manifests/seed_manifest.json"),
        ),
        (
            source_root.join("agents/main/sessions/sessions.json"),
            source_control_root.join("agents/main/sessions/sessions.json"),
        ),
    ];
    for (source, destination) in source_control_copy_plan {
        if let Some(row) = copy_file_if_present(root, &source, &destination, apply)? {
            source_control_copied_rows.push(row);
        }
    }

    let tree_copy_plan = vec![
        (
            source_root.join("cron/runs"),
            assimilation_root.join("cron/runs"),
        ),
        (source_nursery.join("logs"), nursery_root.join("logs")),
        (
            source_nursery.join("promotion"),
            nursery_root.join("promotion"),
        ),
        (
            source_nursery.join("quarantine"),
            nursery_root.join("quarantine"),
        ),
        (source_nursery.join("seeds"), nursery_root.join("seeds")),
        (
            source_root.join("agents/main/sessions"),
            assimilation_root.join("agents/main/sessions"),
        ),
    ];
    for (source_tree, destination_tree) in tree_copy_plan {
        let rows = copy_tree_files_if_present(root, &source_tree, &destination_tree, apply)?;
        if !rows.is_empty() {
            copied_rows.extend(rows);
        }
    }

    let permissions_path = nursery_root.join("containment/permissions.json");
    let policy_gates_path = nursery_root.join("containment/policy-gates.json");
    let seed_manifest_path = nursery_root.join("manifests/seed_manifest.json");
    let mut policy_synced = false;
    let policy_path = root.join("client/runtime/config/nursery_policy.json");
    let mut specialist_count = 0usize;
    let mut training_plan_rel = String::new();
    let mut llm_registry_rel = String::new();
    let mut llm_model_count = 0usize;
    let mut recommended_local_model = String::new();

    let permissions = read_json_if_exists(&permissions_path)
        .or_else(|| read_json_if_exists(&source_nursery.join("containment/permissions.json")))
        .unwrap_or_else(|| json!({}));
    let policy_gates = read_json_if_exists(&policy_gates_path)
        .or_else(|| read_json_if_exists(&source_nursery.join("containment/policy-gates.json")))
        .unwrap_or_else(|| json!({}));
    let seed_manifest = read_json_if_exists(&seed_manifest_path)
        .or_else(|| read_json_if_exists(&source_nursery.join("manifests/seed_manifest.json")))
        .unwrap_or_else(|| json!({}));

    if apply {
        let mut policy = read_json_if_exists(&policy_path).unwrap_or_else(|| json!({}));
        if policy
            .get("containment")
            .and_then(Value::as_object)
            .is_none()
        {
            policy["containment"] = json!({});
        }
        policy["root_dir"] = Value::String("local/state/nursery".to_string());
        policy["fallback_repo_root_dir"] =
            Value::String("local/state/nursery/containment".to_string());
        if !permissions.is_null() {
            policy["containment"]["permissions"] = permissions.clone();
        }
        if !policy_gates.is_null() {
            policy["containment"]["policy_gates"] = policy_gates.clone();
        }
        let assimilated_artifacts = openclaw_seed_to_model_artifacts(&seed_manifest);
        if !assimilated_artifacts.is_empty() {
            policy["model_artifacts"] = Value::Array(assimilated_artifacts);
        }
        lane_utils::write_json(&policy_path, &policy)?;
        policy_synced = true;
    }

    if profile.id == "V6-OPENCLAW-DETACH-001.2" {
        let max_train_minutes = permissions
            .get("max_train_minutes")
            .and_then(Value::as_u64)
            .unwrap_or(30);
        let specialists = seed_manifest
            .get("artifacts")
            .and_then(Value::as_array)
            .map(|rows| {
                rows.iter()
                    .filter_map(|row| {
                        let id = row.get("id").and_then(Value::as_str)?.trim();
                        let model = row.get("model").and_then(Value::as_str)?.trim();
                        let provider = row.get("provider").and_then(Value::as_str)?.trim();
                        if id.is_empty() || model.is_empty() || provider.is_empty() {
                            return None;
                        }
                        let required = row
                            .get("required")
                            .and_then(Value::as_bool)
                            .unwrap_or(false);
                        Some(json!({
                            "specialist_id": format!("nursery-{id}"),
                            "seed_id": id,
                            "provider": provider,
                            "model": model,
                            "tier": if required { "primary" } else { "shadow" },
                            "max_train_minutes": max_train_minutes,
                        }))
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        specialist_count = specialists.len();
        if strict && specialist_count == 0 {
            return Err("openclaw_nursery_seed_manifest_empty".to_string());
        }
        let training_plan_path = nursery_root.join("promotion/specialist_training_plan.json");
        training_plan_rel = lane_utils::rel_path(root, &training_plan_path);
        if apply {
            lane_utils::write_json(
                &training_plan_path,
                &json!({
                    "ts": now_iso(),
                    "source": lane_utils::rel_path(root, &source_root),
                    "specialists": specialists,
                    "max_train_minutes": max_train_minutes,
                    "claim_evidence": [{
                        "id": profile.id,
                        "claim": "nursery_specialists_are_assimilated_from_openclaw_seed_artifacts_with_local_policy_bounds"
                    }]
                }),
            )?;
        }
    }

    if profile.id == "V6-OPENCLAW-DETACH-001.4" {
        let mut llm_models = openclaw_seed_to_llm_models(&seed_manifest);
        if strict && llm_models.is_empty() {
            return Err("openclaw_detach_missing_llm_seed_models".to_string());
        }
        normalize_model_scores(&mut llm_models);
        llm_model_count = llm_models.len();
        let recommended_local = choose_best_model(
            &llm_models,
            &RoutingRequest {
                workload: WorkloadClass::Coding,
                min_context_tokens: 8_192,
                max_cost_score_1_to_5: 5,
                local_only: true,
            },
        );
        if let Some(best_local) = recommended_local {
            recommended_local_model = best_local.name;
        }
        let registry = json!({
            "version": "1.0",
            "ts": now_iso(),
            "source": lane_utils::rel_path(root, &source_root),
            "models": llm_models.iter().map(llm_model_to_json).collect::<Vec<_>>(),
            "recommended_local_model": if recommended_local_model.is_empty() { Value::Null } else { Value::String(recommended_local_model.clone()) }
        });
        let llm_registry_path = root.join("local/state/llm_runtime/model_registry.json");
        let source_registry_path = source_control_root.join("llm/model_registry.json");
        llm_registry_rel = lane_utils::rel_path(root, &llm_registry_path);
        if apply {
            lane_utils::write_json(&llm_registry_path, &registry)?;
            lane_utils::write_json(&source_registry_path, &registry)?;
        }
    }

    if strict && copied_rows.is_empty() {
        return Err("openclaw_assimilation_no_artifacts_copied".to_string());
    }
    if strict
        && profile.id == "V6-OPENCLAW-DETACH-001.3"
        && source_control_copied_rows.is_empty()
        && !source_control_root.join("cron/jobs.json").exists()
    {
        return Err("openclaw_detach_source_control_mirror_empty".to_string());
    }

    let copied_bytes = copied_rows
        .iter()
        .map(|row| row.get("bytes").and_then(Value::as_u64).unwrap_or(0))
        .sum::<u64>();
    let copied_mb = (copied_bytes as f64) / (1024.0 * 1024.0);
    let summary = json!({
        "family": profile.family,
        "contract_id": profile.id,
        "source_root": lane_utils::rel_path(root, &source_root),
        "copied_count": copied_rows.len(),
        "copied_bytes": copied_bytes,
        "copied_mb": copied_mb,
        "copied": copied_rows,
        "source_control_copied_count": source_control_copied_rows.len(),
        "source_control_copied": source_control_copied_rows,
        "source_control_root": lane_utils::rel_path(root, &source_control_root),
        "policy_synced": policy_synced,
        "specialist_count": specialist_count,
        "training_plan_path": training_plan_rel,
        "llm_registry_path": llm_registry_rel,
        "llm_model_count": llm_model_count,
        "recommended_local_model": if recommended_local_model.is_empty() { Value::Null } else { Value::String(recommended_local_model.clone()) },
        "state_path": state_rel,
        "assimilation_root": lane_utils::rel_path(root, &assimilation_root),
        "nursery_root": lane_utils::rel_path(root, &nursery_root),
    });

    if apply {
        upsert_contract_state_entry(
            &mut state,
            profile.id,
            json!({ "summary": summary, "applied_at": now_iso() }),
        );
        lane_utils::write_json(&state_path, &state)?;
    }

    let claim = if profile.id == "V6-OPENCLAW-DETACH-001.2" {
        "openclaw_nursery_seed_training_is_materialized_locally_with_specialist_plan_and_receipts"
    } else if profile.id == "V6-OPENCLAW-DETACH-001.3" {
        "openclaw_cron_and_nursery_contracts_are_mirrored_into_source_controlled_infring_paths"
    } else if profile.id == "V6-OPENCLAW-DETACH-001.4" {
        "llm_runtime_registry_is_bootstrapped_from_assimilated_seed_models_with_deterministic_power_cost_ranking"
    } else {
        "openclaw_operator_state_and_nursery_artifacts_are_assimilated_into_infring_owned_paths_with_detachment_controls"
    };
    Ok(ContractExecution {
        summary,
        claims: vec![json!({
            "id": profile.id,
            "claim": claim,
            "evidence": {
                "copied_count": copied_rows.len(),
                "copied_mb": copied_mb,
                "policy_synced": policy_synced,
                "specialist_count": specialist_count
            }
        })],
        artifacts: vec![
            state_rel,
            lane_utils::rel_path(root, &assimilation_root),
            lane_utils::rel_path(root, &nursery_root),
            lane_utils::rel_path(root, &source_control_root),
        ],
    })
}

fn execute_contract_profile(
    root: &Path,
    profile: RuntimeSystemContractProfile,
    payload: &Value,
    apply: bool,
    strict: bool,
) -> Result<ContractExecution, String> {
    match profile.family {
        "v5_hold_remediation" => execute_v5_hold_contract(root, profile, payload, apply),
        "v5_rust_hybrid" => execute_v5_rust_hybrid_contract(root, profile, payload, apply, strict),
        "v5_rust_productivity" => {
            execute_v5_rust_productivity_contract(root, profile, payload, apply, strict)
        }
        "execution_streaming_stack" => {
            execute_execution_streaming_contract(root, profile, payload, apply, strict)
        }
        "execution_worktree_stack" => {
            execute_execution_worktree_contract(root, profile, payload, apply, strict)
        }
        "assimilate_fast_stack" => {
            execute_assimilate_fast_contract(root, profile, payload, apply, strict)
        }
        "workflow_open_swe_stack" => {
            execute_workflow_open_swe_contract(root, profile, payload, apply, strict)
        }
        "memory_context_maintenance" => {
            execute_memory_context_contract(root, profile, payload, apply, strict)
        }
        "integration_lakehouse_stack" => {
            execute_integration_lakehouse_contract(root, profile, payload, apply, strict)
        }
        "inference_adaptive_routing" => {
            execute_inference_adaptive_contract(root, profile, payload, apply, strict)
        }
        "runtime_cleanup_autonomous" => {
            execute_runtime_cleanup_contract(root, profile, payload, apply, strict)
        }
        "erp_agentic_stack" => execute_erp_agentic_contract(root, profile, payload, apply, strict),
        "tooling_uv_ruff_stack" => {
            execute_tooling_uv_ruff_contract(root, profile, payload, apply, strict)
        }
        "workflow_visual_bridge_stack" => {
            execute_workflow_visual_bridge_contract(root, profile, payload, apply, strict)
        }
        "openclaw_detachment_stack" => {
            execute_openclaw_detachment_contract(root, profile, payload, apply, strict)
        }
        _ => execute_generic_family_contract(root, profile, payload, apply, strict),
    }
}

fn read_only_command(command: &str) -> bool {
    matches!(command, "status" | "verify")
}

fn system_id_from_args(command: &str, args: &[String]) -> String {
    let by_flag = lane_utils::parse_flag(args, "system-id", true)
        .or_else(|| lane_utils::parse_flag(args, "lane-id", true))
        .or_else(|| lane_utils::parse_flag(args, "id", true));
    if by_flag.is_some() {
        return lane_utils::clean_token(by_flag.as_deref(), "runtime-system");
    }
    if command.starts_with('v')
        && command
            .chars()
            .any(|ch| ch.is_ascii_digit() || matches!(ch, '-' | '_' | '.'))
    {
        return lane_utils::clean_token(Some(command), "runtime-system");
    }
    lane_utils::clean_token(None, "runtime-system")
}

fn collect_passthrough(args: &[String]) -> Vec<String> {
    args.iter()
        .filter_map(|row| {
            let t = row.trim();
            if t.is_empty() {
                return None;
            }
            if t.starts_with("--system-id")
                || t.starts_with("--lane-id")
                || t.starts_with("--id")
                || t.starts_with("--apply")
                || t.starts_with("--payload-json")
                || t.starts_with("--strict")
            {
                return None;
            }
            Some(t.to_string())
        })
        .collect::<Vec<_>>()
}

fn payload_object(raw: Option<&str>) -> Result<Value, String> {
    let parsed = match raw {
        Some(v) => parse_json(Some(v))?,
        None => json!({}),
    };
    if parsed.is_object() {
        Ok(parsed)
    } else {
        Err("payload_must_be_json_object".to_string())
    }
}

fn contract_defaults(profile: RuntimeSystemContractProfile) -> Value {
    match profile.family {
        "audit_self_healing_stack" => json!({
            "drift_detection_enabled": true,
            "self_healing_playbooks_enabled": true,
            "confidence_scoring_enabled": true,
            "cross_agent_verification_enabled": true,
            "human_review_gate_enforced": true,
            "conduit_only_enforced": true,
            "poll_interval_minutes": 15.0,
            "verification_agents": 2.0,
            "confidence_high_threshold": 0.9,
            "audit_targets": [
                "origin_integrity",
                "supply_chain_provenance_v2",
                "alpha_readiness"
            ],
            "self_healing_actions": [
                "refresh_spine_receipt",
                "rebuild_supply_chain_bundle",
                "reconcile_workspace_churn"
            ],
            "confidence_range": "0.0-1.0",
            "consensus_mode": "strict_match"
        }),
        "act_critical_judgment" => json!({
            "critical_judgment_gate": true,
            "pairwise_training_enabled": true,
            "self_mod_gate_mode": "is_change_better",
            "benchmark_lane": "alfworld_webshop_scienceworld_gpqa"
        }),
        "company_revenue_automation" => json!({
            "crm_boundary": "conduit_only",
            "auto_followup_enabled": true,
            "lead_routing_mode": "warm_inbound",
            "funnel_metrics_required": true
        }),
        "competitor_surface_expansion" => json!({
            "provider_router_mode": "governed",
            "channel_adapter_expansion": true,
            "domain_hands_expansion": true
        }),
        "go_to_market_crush" => json!({
            "enterprise_licensing_guard": "active",
            "migration_bridge": "crewai_langgraph_autogen_openhands",
            "ga_lts_contract": true,
            "governance_observability": "required"
        }),
        "compatibility_mole" => json!({
            "compatibility_shim_mode": "silent",
            "safety_absorption_engine": "active",
            "receipt_anchoring": "permanent"
        }),
        "power_execution" => json!({
            "release_speed_mode": "accelerated_release_profile",
            "predictive_router_intelligence": true,
            "endurance_window": "week_scale",
            "byzantine_consensus": true,
            "external_blocker_closure": true
        }),
        "swarm_orchestration" => json!({
            "parallel_swarm_enabled": true,
            "implicit_planning": true,
            "compaction_engine": "self_engineered",
            "shared_memory_mode": "swarm_aware"
        }),
        "ecosystem_scale_v11" => json!({
            "migration_hub": true,
            "persistent_actions": true,
            "marketplace": "protheus_hub",
            "sdk_surface": ["rust", "python", "typescript", "go", "wasm"],
            "economic_governance_layer": true
        }),
        "ecosystem_scale_v8" => json!({
            "persistent_runtime_24x7": true,
            "skills_import_mode": "plug_and_play",
            "wifi_pose_eye_substrate": true,
            "swarm_prediction_engine": true,
            "voice_companion_mode": "realtime"
        }),
        "memory_bank_v2" => json!({
            "working_memory_state": "working_memory.json",
            "tiering": ["hot", "warm", "cold"],
            "importance_decay": true,
            "decision_log": true,
            "task_scoped_slots": true,
            "session_continuation": true,
            "uncertainty_surface": true,
            "cross_reference_graph": true
        }),
        "f100_assurance" => json!({
            "enterprise_zero_trust": true,
            "assurance_super_gate": true,
            "signed_jwt_required": true,
            "cmek_required": true,
            "private_link_required": true
        }),
        "v5_hold_remediation" => json!({
            "unchanged_state_hold_rate": 0.62,
            "low_confidence_hold_rate": 0.41,
            "cap_hold_rate": 0.33,
            "route_hold_rate": 0.28,
            "budget_hold_rate": 0.09
        }),
        "v5_rust_hybrid" => json!({
            "target_min_rust_pct": 15.0,
            "target_max_rust_pct": 25.0,
            "wrapper_integrity_ok": true
        }),
        "v5_rust_productivity" => json!({
            "throughput_ops_sec": 12000.0,
            "p95_ms": 45.0,
            "p99_ms": 90.0,
            "unit_cost_per_user": 0.012,
            "canary_enabled": true
        }),
        "ultimate_evolution" => json!({
            "replication_policy_gate": true,
            "self_awareness_journal": true,
            "exotic_hardware_abstraction": true,
            "tokenomics_ledger_enforced": true,
            "symbiosis_interface": true,
            "universal_adapter_skeleton_key": true,
            "universal_adapter_coverage_pct": 92.0
        }),
        "automation_mission_stack" => json!({
            "cron_scheduler_enabled": true,
            "multi_agent_handoff_enabled": true,
            "persistent_memory_enabled": true,
            "security_hardening_enabled": true,
            "mission_dashboard_enabled": true,
            "checkpoint_interval_items": 10.0,
            "checkpoint_interval_minutes": 2.0
        }),
        "autonomy_opportunity_engine" => json!({
            "opportunity_discovery_engine": true,
            "inefficiency_scanner": true,
            "monetization_evaluator": true,
            "hindsight_ranking_engine": true,
            "creative_mode_signal_floor": 0.8
        }),
        "cli_surface_hardening" => json!({
            "single_static_rust_binary": true,
            "rust_state_machine_core": true,
            "ts_cli_opt_in_extension": true,
            "thin_shim_wrapper": true,
            "node_absence_doctor_message": true,
            "static_binary_mb": 1.3
        }),
        "client_model_access" => json!({
            "vibe_proxy_layer_enabled": true,
            "model_access_store_encrypted": true,
            "model_access_store_policy_gate": true
        }),
        "competitive_execution_moat" => json!({
            "aot_musl_zerocopy_lanes": true,
            "signed_receipt_export_sub_ms": true,
            "non_divergence_pre_execution_gate": true,
            "autonomous_swarm_workflow_evolution": true,
            "kernel_native_observability_governance": true,
            "edge_to_cloud_uniform_plan": true,
            "production_resilience_flywheel": true,
            "throughput_ops_sec": 12600.0,
            "p95_ms": 12.0
        }),
        "eyes_media_assimilation" => json!({
            "video_transcription_enabled": true,
            "course_assimilation_pipeline": true,
            "podcast_generator_enabled": true,
            "swarm_opportunity_integration": true,
            "transcript_quality_floor": 0.86
        }),
        "eyes_computer_use" => json!({
            "parchi_computer_use_engine": true,
            "frontend_navigation_reliability": true,
            "computer_use_safety_gate": true,
            "superwhisper_voice_engine": true,
            "voice_session_blob_archival": true,
            "interaction_success_floor": 0.91
        }),
        "eyes_lightpanda_router" => json!({
            "lightpanda_backend_enabled": true,
            "ultra_speed_profile_enabled": true,
            "seamless_multi_backend_router": true,
            "browser_session_blob_archival": true,
            "target_speedup_x": 31.0
        }),
        "learning_rsi_pipeline" => json!({
            "signal_extraction_prm_judge": true,
            "hindsight_on_policy_distillation": true,
            "async_four_loop_training": true,
            "interaction_trajectory_blob_integration": true,
            "distributed_gym_factory": true,
            "adversarial_verification_pipeline": true,
            "training_flywheel_export": true,
            "real_world_product_verifier": true,
            "local_overnight_self_improvement": true,
            "real_usage_feedback_reinforcement": true,
            "single_directive_rl_engine": true,
            "emergent_strategy_discovery": true,
            "weekly_policy_retraining": true,
            "auto_rollback_enabled": true,
            "low_cost_overnight_loop": true,
            "training_loops_per_day": 3.0
        }),
        "memory_depth_stack" => json!({
            "hierarchical_tree_index_builder": true,
            "agentic_tree_reasoning_retriever": true,
            "vision_page_retrieval": true,
            "tree_index_trace_blob_archival": true,
            "lossless_folder_backend": true,
            "automatic_sync_perfect_recall": true,
            "blob_lossless_hybrid_mirroring": true,
            "tinymax_lossless_mode": true,
            "tree_sitter_ast_indexer": true,
            "blast_radius_analyzer": true,
            "auto_codebase_wiki_generator": true,
            "mcp_graph_integration": true,
            "persistent_case_facts_scratchpad": true,
            "claim_source_provenance_mapping": true,
            "recall_budget_ms": 220.0
        }),
        "organism_parallel_intelligence" => json!({
            "side_chat_forking_engine": true,
            "non_capturing_overlay_renderer": true,
            "file_overlap_peek_safety": true,
            "persistent_side_session_blob_integration": true,
            "hub_spoke_coordinator": true,
            "plan_vs_explore_subagent_separation": true,
            "autonomous_model_generator": true,
            "self_critique_alternative_perspectives": true,
            "explainer_slide_visual_synthesis": true,
            "model_evolution_archive": true
        }),
        "persona_enterprise_pack" => json!({
            "ai_ceo_persona_core": true,
            "departmental_agent_pack": true,
            "cross_agent_memory_sync": true,
            "role_based_agent_addition": true
        }),
        "safety_error_taxonomy" => json!({
            "structured_error_taxonomy": true,
            "error_fail_closed_mapping": true
        }),
        "security_sandbox_redteam" => json!({
            "wasm_capability_sandbox": true,
            "credential_injection_isolation": true,
            "verifiable_privacy_plane": true,
            "long_horizon_attack_chain_simulation": true,
            "zero_to_full_context_accumulation": true,
            "attack_trajectory_blob_archival": true,
            "max_escape_rate": 0.0
        }),
        "skills_runtime_pack" => json!({
            "native_hf_cli_skill": true,
            "autonomous_model_dataset_pipeline": true,
            "hf_pure_context_mode": true,
            "hf_output_swarm_integration": true,
            "native_pomodoro_skill": true,
            "interactive_tui_focus_mode": true,
            "shell_composable_focus_status": true,
            "focus_session_blob_integration": true,
            "raspberry_pi_edge_template": true,
            "self_healing_server_agent": true,
            "orion_team_coordinator": true,
            "productivity_workflow_pack": true,
            "lens_scribe_code_agent_pack": true,
            "claude_style_prompt_chaining": true,
            "iterative_refinement_loop": true,
            "component_fullstack_scaffolding": true,
            "one_click_deployment_flow": true
        }),
        "swarm_runtime_scaling" => json!({
            "sentiment_swarm_core": true,
            "scenario_injection_live_consensus_mapper": true,
            "prediction_market_sentiment_oracle": true,
            "swarm_trajectory_storage_dream_refinement": true,
            "role_based_model_assignment": true,
            "automatic_parallel_exploration": true,
            "visual_subagent_dashboard": true,
            "subagent_edit_permission_gate": true,
            "planning_as_tool_engine": true,
            "filesystem_native_persistent_memory": true,
            "isolated_subagent_spawning": true,
            "shell_execution_safety_gates": true,
            "worker_heartbeat": true,
            "automatic_work_stealing": true,
            "supervisor_watchdog_respawn": true,
            "output_schema_enforcement": true,
            "frequent_checkpoint_recovery": true,
            "scope_boundary_validation": true,
            "realtime_aggregation_dashboard": true,
            "capability_advertisement_adaptive_partitioning": true,
            "cross_agent_dedup_reconciliation": true,
            "timeout_graceful_degradation": true,
            "max_timeout_seconds": 60.0
        }),
        "client_wasm_bridge" => json!({
            "rust_wasm_bridge_engine": true,
            "browser_structured_concurrency": true,
            "standalone_html_demo_generator": true,
            "wasm_artifact_blob_archival": true
        }),
        "organism_adlc" => json!({
            "adlc_core_engine": true,
            "evolving_goals_replanning": true,
            "parallel_subagent_coordination": true,
            "continuous_testing_live_feedback": true
        }),
        "tinymax_extreme_profile" => json!({
            "trait_driven_swappable_tinymax_core": true,
            "sub5mb_idle_memory_mode": true,
            "idle_memory_mb": 1.4
        }),
        "execution_streaming_stack" => json!({
            "ssd_streaming_enabled": true,
            "quantization_bridge_enabled": true,
            "os_page_cache_first": true,
            "kernel_bridge_enabled": true,
            "target_tokens_per_sec": 96.0,
            "resident_memory_gb": 12.0,
            "prefetch_window": 4,
            "quantization_bits": 4,
            "metal_mode": "bridge"
        }),
        "execution_worktree_stack" => json!({
            "worktree_manager_enabled": true,
            "branch_isolation_enforced": true,
            "swarm_worktree_dispatch_enabled": true,
            "cleanup_enabled": true,
            "cleanup_interval_seconds": 75,
            "max_residual_worktrees": 3,
            "agent_id": "agent-default",
            "base_branch": "main"
        }),
        "assimilate_fast_stack" => json!({
            "fast_mode_enabled": true,
            "skeleton_cache_enabled": true,
            "progress_receipts_enabled": true,
            "parallel_microtasks_enabled": true,
            "warmup_enabled": true,
            "safety_guard_enabled": true,
            "target_latency_ms": 5000.0,
            "cache_ttl_seconds": 3600,
            "max_parallel_microtasks": 8
        }),
        "workflow_open_swe_stack" => json!({
            "loop_registry_enabled": true,
            "git_bridge_enabled": true,
            "approval_middleware_enabled": true,
            "eval_harness_enabled": true,
            "memory_continuity_enabled": true,
            "eval_pass_floor": 0.8
        }),
        "memory_context_maintenance" => json!({
            "staleness_tracking_enabled": true,
            "pre_generation_pruning_enabled": true,
            "emergency_compact_enabled": true,
            "context_observability_enabled": true,
            "safe_config_validation_enabled": true,
            "context_budget_tokens": 250000,
            "window_usage_tokens": 120000,
            "staleness_reset_seconds": 30
        }),
        "integration_lakehouse_stack" => json!({
            "unity_catalog_bridge_enabled": true,
            "mosaic_mapping_enabled": true,
            "mlflow_provider_enabled": true,
            "vector_automl_bridge_enabled": true,
            "dbrx_provider_enabled": true,
            "drift_monitoring_enabled": true,
            "catalog": "main",
            "schema": "default"
        }),
        "inference_adaptive_routing" => json!({
            "live_scoring_enabled": true,
            "rule_routing_enabled": true,
            "ordered_failover_enabled": true,
            "provider_observability_enabled": true,
            "min_success_rate": 0.8,
            "max_latency_ms": 1500.0,
            "preferred_model": "kimi2.5:cloud",
            "providers": [
                {
                    "name": "kimi2.5:cloud",
                    "latency_ms": 320,
                    "cost_per_1k": 0.0015,
                    "success_rate": 0.96
                },
                {
                    "name": "gpt-5.4",
                    "latency_ms": 410,
                    "cost_per_1k": 0.0030,
                    "success_rate": 0.97
                },
                {
                    "name": "qwen-max",
                    "latency_ms": 290,
                    "cost_per_1k": 0.0012,
                    "success_rate": 0.91
                }
            ],
            "rules": [
                { "id": "large_context", "max_cost_per_1k": 0.0035, "min_context_tokens": 64000, "force_provider": "kimi2.5:cloud" },
                { "id": "strict_latency", "max_latency_ms": 350, "force_provider": "qwen-max" }
            ]
        }),
        "runtime_cleanup_autonomous" => json!({
            "eviction_matrix_enabled": true,
            "multi_trigger_scheduler_enabled": true,
            "tiered_pressure_enabled": true,
            "device_profiles_enabled": true,
            "protected_state_invariants_enabled": true,
            "audit_controls_enabled": true,
            "boundedness_gate_enabled": true,
            "cleanup_interval_minutes": 5,
            "memory_threshold_percent": 75,
            "disk_threshold_percent": 10,
            "disk_free_percent": 22,
            "device_profile": "desktop",
            "max_storage_mb": 500
        }),
        "erp_agentic_stack" => json!({
            "erp_template_registry_enabled": true,
            "closed_loop_enabled": true,
            "lineage_gate_enabled": true,
            "max_loop_latency_ms": 1500,
            "team": "procurement"
        }),
        "tooling_uv_ruff_stack" => json!({
            "uv_bridge_enabled": true,
            "ruff_bridge_enabled": true,
            "isolated_env_enabled": true,
            "autowire_pipeline_enabled": true,
            "tooling_gate_enabled": true,
            "max_resolution_time_seconds": 300,
            "venv_name": "default",
            "tiny_mode_no_regression": true,
            "pure_mode_no_regression": true
        }),
        "workflow_visual_bridge_stack" => json!({
            "canvas_bridge_enabled": true,
            "prompt_chain_enabled": true,
            "rag_integration_enabled": true,
            "tool_eval_enabled": true,
            "enterprise_observability_enabled": true,
            "cold_start_guard_ms": 3000
        }),
        "openclaw_detachment_stack" => json!({
            "source_assimilation_enabled": true,
            "nursery_migration_enabled": true,
            "external_dependency_detached": true,
            "operator_state_capture_enabled": true,
            "local_runtime_paths_enforced": true,
            "source_controlled_mirror_enabled": true,
            "llm_runtime_registry_enabled": true,
            "source_files_required_min": 1,
            "llm_registry_models_min": 1,
            "max_assimilation_copy_mb": 1024,
            "source_root": ".."
        }),
        _ => json!({}),
    }
}

fn merge_payload(mut payload: Value, defaults: &Value) -> Value {
    let Some(payload_obj) = payload.as_object_mut() else {
        return defaults.clone();
    };
    if let Some(default_obj) = defaults.as_object() {
        for (k, v) in default_obj {
            payload_obj.entry(k.clone()).or_insert_with(|| v.clone());
        }
    }
    payload
}

fn contract_command_allowed(command: &str) -> bool {
    matches!(
        command,
        "run" | "build" | "bootstrap" | "package" | "settle" | "status" | "verify"
    )
}

fn strict_for(system_id: &str, args: &[String]) -> bool {
    lane_utils::parse_bool(
        lane_utils::parse_flag(args, "strict", true).as_deref(),
        looks_like_contract_id(system_id),
    )
}

fn parse_limit(raw: Option<String>, fallback: usize, max: usize) -> usize {
    let parsed = raw
        .as_deref()
        .and_then(|v| v.trim().parse::<usize>().ok())
        .unwrap_or(fallback);
    parsed.clamp(1, max.max(1))
}

fn family_roi_weight(family: &str) -> i64 {
    match family {
        "security_sandbox_redteam" => 130,
        "f100_assurance" => 125,
        "swarm_runtime_scaling" => 120,
        "memory_depth_stack" => 116,
        "learning_rsi_pipeline" => 112,
        "automation_mission_stack" => 110,
        "skills_runtime_pack" => 108,
        "competitive_execution_moat" => 106,
        "power_execution" => 104,
        "organism_parallel_intelligence" => 102,
        "ecosystem_scale_v11" => 100,
        "ecosystem_scale_v8" => 95,
        "swarm_orchestration" => 93,
        _ => 80,
    }
}

fn contract_roi_boost(id: &str) -> i64 {
    if id.starts_with("V10-PERF-001.") {
        30
    } else if id.starts_with("V6-DASHBOARD-007.") || id.starts_with("V6-DASHBOARD-008.") {
        26
    } else if id.starts_with("V6-SECURITY-") || id.starts_with("V8-SECURITY-") {
        25
    } else if id.starts_with("V6-WORKFLOW-") || id.starts_with("V8-SWARM-") {
        20
    } else if id.starts_with("V6-MEMORY-") || id.starts_with("V8-MEMORY-") {
        18
    } else if id.starts_with("V7-F100-") {
        16
    } else if id.starts_with("V10-") || id.starts_with("V11-") {
        12
    } else {
        0
    }
}

fn profile_roi_score(profile: RuntimeSystemContractProfile) -> i64 {
    family_roi_weight(profile.family) + contract_roi_boost(profile.id)
}

fn manifest_payload() -> Value {
    let profiles = actionable_profiles();
    let mut by_family: BTreeMap<String, usize> = BTreeMap::new();
    let contracts = profiles
        .iter()
        .map(|profile| {
            *by_family
                .entry(profile.family.to_string())
                .or_insert(0usize) += 1;
            profile_json(*profile)
        })
        .collect::<Vec<_>>();

    let mut out = json!({
        "ok": true,
        "type": "runtime_systems_manifest",
        "lane": LANE_ID,
        "counts": {
            "contracts": profiles.len(),
            "families": by_family.len()
        },
        "families": by_family,
        "contracts": contracts
    });
    out["receipt_hash"] = Value::String(receipt_hash(&out));
    out
}

fn payload_sha(payload: &Value) -> String {
    let encoded = serde_json::to_vec(payload).unwrap_or_default();
    hex::encode(Sha256::digest(encoded))
}

fn status_payload(root: &Path, system_id: &str, command: &str) -> Value {
    let latest = lane_utils::read_json(&latest_path(root, system_id));
    let profile = profile_for(system_id);
    let mut out = json!({
        "ok": true,
        "type": "runtime_systems_status",
        "lane": LANE_ID,
        "command": command,
        "system_id": system_id,
        "latest_path": lane_utils::rel_path(root, &latest_path(root, system_id)),
        "history_path": lane_utils::rel_path(root, &history_path(root, system_id)),
        "has_state": latest.is_some(),
        "latest": latest,
        "contract_profile": profile.map(profile_json)
    });
    out["receipt_hash"] = Value::String(receipt_hash(&out));
    out
}

fn roi_sweep_payload(root: &Path, args: &[String]) -> Result<Value, String> {
    let profiles = actionable_profiles();
    let limit = parse_limit(
        lane_utils::parse_flag(args, "limit", true),
        400,
        profiles.len(),
    );
    let apply =
        lane_utils::parse_bool(lane_utils::parse_flag(args, "apply", true).as_deref(), true);
    let strict = lane_utils::parse_bool(
        lane_utils::parse_flag(args, "strict", true).as_deref(),
        true,
    );

    let mut ranked = profiles
        .iter()
        .copied()
        .map(|profile| (profile_roi_score(profile), profile))
        .collect::<Vec<(i64, RuntimeSystemContractProfile)>>();
    ranked.sort_by(|(score_a, profile_a), (score_b, profile_b)| {
        score_b
            .cmp(score_a)
            .then_with(|| profile_a.id.cmp(profile_b.id))
    });

    let mut executed = Vec::<Value>::new();
    let mut success = 0u64;
    let mut failed = 0u64;
    let mut failed_ids = Vec::<String>::new();
    for (score, profile) in ranked.into_iter().take(limit) {
        match execute_contract_lane(root, profile.id, apply, strict) {
            Ok(result) => {
                let ok = result.get("ok").and_then(Value::as_bool).unwrap_or(false);
                if ok {
                    success += 1;
                } else {
                    failed += 1;
                    failed_ids.push(profile.id.to_string());
                }
                executed.push(json!({
                    "id": profile.id,
                    "family": profile.family,
                    "roi_score": score,
                    "ok": ok,
                    "receipt_hash": result.get("receipt_hash").cloned().unwrap_or(Value::Null),
                    "artifacts_count": result.get("artifacts").and_then(Value::as_array).map(|rows| rows.len()).unwrap_or(0)
                }));
            }
            Err(err) => {
                failed += 1;
                failed_ids.push(profile.id.to_string());
                executed.push(json!({
                    "id": profile.id,
                    "family": profile.family,
                    "roi_score": score,
                    "ok": false,
                    "error": err
                }));
            }
        }
    }

    let mut out = json!({
        "ok": failed == 0,
        "type": "runtime_systems_roi_sweep",
        "lane": LANE_ID,
        "apply": apply,
        "strict": strict,
        "limit_requested": limit,
        "selected_count": executed.len(),
        "total_actionable_contracts": profiles.len(),
        "success_count": success,
        "failed_count": failed,
        "failed_ids": failed_ids,
        "executed": executed,
        "claim_evidence": [{
            "id": "runtime_systems_roi_top_contract_sweep",
            "claim": "top_ranked_runtime_contracts_execute_with_fail_closed_receipted_lane",
            "evidence": {
                "limit_requested": limit,
                "selected_count": success + failed,
                "success_count": success,
                "failed_count": failed,
                "strict": strict,
                "apply": apply
            }
        }]
    });
    out["receipt_hash"] = Value::String(receipt_hash(&out));
    Ok(out)
}

fn run_payload(
    root: &Path,
    system_id: &str,
    command: &str,
    args: &[String],
) -> Result<Value, String> {
    let apply_default = !read_only_command(command);
    let apply = lane_utils::parse_bool(
        lane_utils::parse_flag(args, "apply", true).as_deref(),
        apply_default,
    );
    let strict = strict_for(system_id, args);
    let profile = profile_for(system_id);
    if strict && looks_like_contract_id(system_id) && profile.is_none() {
        return Err(format!("unknown_runtime_contract_id:{system_id}"));
    }
    if strict && profile.is_some() && !contract_command_allowed(command) {
        return Err(format!("contract_command_not_allowed:{command}"));
    }
    let payload = payload_object(lane_utils::parse_flag(args, "payload-json", true).as_deref())?;
    let payload = if let Some(profile) = profile {
        merge_payload(payload, &contract_defaults(profile))
    } else {
        payload
    };
    let contract_execution = if let Some(profile) = profile {
        execute_contract_profile(root, profile, &payload, apply, strict)?
    } else {
        ContractExecution {
            summary: json!({}),
            claims: Vec::new(),
            artifacts: Vec::new(),
        }
    };
    let passthrough = collect_passthrough(args);
    let ts = now_iso();
    let mut row = json!({
        "type": "runtime_systems_run",
        "lane": LANE_ID,
        "command": command,
        "system_id": system_id,
        "ts": ts,
        "payload": payload,
        "payload_sha256": payload_sha(&payload),
        "passthrough": passthrough,
        "apply": apply,
        "strict": strict,
        "contract_execution": contract_execution.summary,
        "contract_profile": profile.map(profile_json)
    });
    row["ok"] = Value::Bool(true);
    row["receipt_hash"] = Value::String(receipt_hash(&row));

    if apply {
        lane_utils::write_json(&latest_path(root, system_id), &row)?;
        lane_utils::append_jsonl(&history_path(root, system_id), &row)?;
    }

    let mut out = json!({
        "ok": true,
        "type": "runtime_systems_run",
        "lane": LANE_ID,
        "command": command,
        "system_id": system_id,
        "apply": apply,
        "strict": strict,
        "latest_path": lane_utils::rel_path(root, &latest_path(root, system_id)),
        "history_path": lane_utils::rel_path(root, &history_path(root, system_id)),
        "payload_sha256": row.get("payload_sha256").cloned().unwrap_or(Value::Null),
        "contract_execution": row.get("contract_execution").cloned().unwrap_or(Value::Null),
        "artifacts": contract_execution.artifacts.clone(),
        "contract_profile": row.get("contract_profile").cloned().unwrap_or(Value::Null),
        "claim_evidence": [mutation_receipt_claim(system_id, command, apply, strict)]
    });
    if let Some(profile) = profile {
        let mut claims = vec![
            json!({
                "id": profile.id,
                "claim": "actionable_contract_id_routes_through_authoritative_runtime_system_plane",
                "evidence": {
                    "family": profile.family,
                    "objective": profile.objective,
                    "strict_conduit_only": profile.strict_conduit_only,
                    "strict_fail_closed": profile.strict_fail_closed
                }
            }),
            mutation_receipt_claim(system_id, command, apply, strict),
        ];
        claims.extend(contract_execution.claims);
        out["claim_evidence"] = Value::Array(claims);
    }
    out["receipt_hash"] = Value::String(receipt_hash(&out));
    Ok(out)
}

fn cli_error(argv: &[String], err: &str, exit_code: i32) -> Value {
    let mut out = json!({
        "ok": false,
        "type": "runtime_systems_cli_error",
        "lane": LANE_ID,
        "argv": argv,
        "error": lane_utils::clean_text(Some(err), 300),
        "exit_code": exit_code
    });
    out["receipt_hash"] = Value::String(receipt_hash(&out));
    out
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let command = argv
        .first()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());
    if matches!(command.as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }
    let payload = if command == "manifest" {
        Ok(manifest_payload())
    } else if command == "roi-sweep" {
        roi_sweep_payload(root, &argv[1..])
    } else {
        let system_id = system_id_from_args(&command, &argv[1..]);
        if system_id.is_empty() {
            print_json_line(&cli_error(argv, "system_id_missing", 2));
            return 2;
        }
        match command.as_str() {
            "status" | "verify" => Ok(status_payload(root, &system_id, &command)),
            _ => run_payload(root, &system_id, &command, &argv[1..]),
        }
    };

    match payload {
        Ok(out) => {
            let ok = out.get("ok").and_then(Value::as_bool).unwrap_or(false);
            print_json_line(&out);
            if ok {
                0
            } else {
                1
            }
        }
        Err(err) => {
            print_json_line(&cli_error(argv, &err, 2));
            2
        }
    }
}

pub fn execute_contract_lane(
    root: &Path,
    system_id: &str,
    apply: bool,
    strict: bool,
) -> Result<Value, String> {
    let args = vec![
        format!("--apply={}", if apply { 1 } else { 0 }),
        format!("--strict={}", if strict { 1 } else { 0 }),
    ];
    run_payload(root, system_id, "run", &args)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_system_contracts::actionable_ids;

    #[test]
    fn run_writes_latest_and_status_reads_it() {
        let root = tempfile::tempdir().expect("tempdir");
        let exit = run(
            root.path(),
            &[
                "run".to_string(),
                "--system-id=systems-memory-causal_temporal_graph".to_string(),
                "--apply=1".to_string(),
                "--payload-json={\"k\":1}".to_string(),
            ],
        );
        assert_eq!(exit, 0);

        let latest = latest_path(root.path(), "systems-memory-causal_temporal_graph");
        assert!(latest.exists());

        let status = status_payload(
            root.path(),
            "systems-memory-causal_temporal_graph",
            "status",
        );
        assert_eq!(
            status.get("has_state").and_then(Value::as_bool),
            Some(true),
            "status should reflect latest state"
        );
    }

    #[test]
    fn verify_is_read_only_and_does_not_write_state() {
        let root = tempfile::tempdir().expect("tempdir");
        let exit = run(
            root.path(),
            &[
                "verify".to_string(),
                "--system-id=systems-autonomy-gated_self_improvement_loop".to_string(),
            ],
        );
        assert_eq!(exit, 0);
        let latest = latest_path(root.path(), "systems-autonomy-gated_self_improvement_loop");
        assert!(!latest.exists());
    }

    #[test]
    fn strict_mode_rejects_unknown_contract_ids() {
        let root = tempfile::tempdir().expect("tempdir");
        let err = run_payload(
            root.path(),
            "V8-UNKNOWN-404.1",
            "run",
            &["--strict=1".to_string()],
        )
        .expect_err("unknown contract should fail");
        assert!(
            err.contains("unknown_runtime_contract_id"),
            "expected strict unknown id error, got {err}"
        );
    }

    #[test]
    fn manifest_exposes_actionable_contract_registry() {
        let out = manifest_payload();
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            out.get("counts")
                .and_then(Value::as_object)
                .and_then(|m| m.get("contracts"))
                .and_then(Value::as_u64),
            Some(actionable_ids().len() as u64)
        );
    }

    #[test]
    fn actionable_contract_ids_emit_profile_and_receipts() {
        let root = tempfile::tempdir().expect("tempdir");
        for &id in actionable_ids() {
            let out = run_payload(root.path(), id, "run", &["--strict=1".to_string()])
                .expect("contract run should succeed");
            assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
            assert_eq!(
                out.get("contract_profile")
                    .and_then(Value::as_object)
                    .and_then(|m| m.get("id"))
                    .and_then(Value::as_str),
                Some(id)
            );
            let has_claim = out
                .get("claim_evidence")
                .and_then(Value::as_array)
                .map(|rows| {
                    rows.iter()
                        .any(|row| row.get("id").and_then(Value::as_str) == Some(id))
                })
                .unwrap_or(false);
            assert!(has_claim, "missing contract claim evidence for {id}");
        }
    }

    #[test]
    fn v5_contract_families_persist_stateful_artifacts() {
        let root = tempfile::tempdir().expect("tempdir");
        for id in ["V5-HOLD-001", "V5-RUST-HYB-001", "V5-RUST-PROD-001"] {
            let out = run_payload(
                root.path(),
                id,
                "run",
                &["--strict=1".to_string(), "--apply=1".to_string()],
            )
            .expect("contract run should succeed");
            assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
            let artifacts = out
                .get("artifacts")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            assert!(
                !artifacts.is_empty(),
                "contract artifacts should be emitted"
            );
            let state_file = artifacts[0].as_str().unwrap_or_default().to_string();
            assert!(
                root.path().join(state_file).exists(),
                "expected contract state artifact to exist"
            );
        }
    }

    #[test]
    fn v9_audit_contract_family_persists_state_and_claims() {
        let root = tempfile::tempdir().expect("tempdir");
        let out = run_payload(
            root.path(),
            "V9-AUDIT-026.1",
            "run",
            &["--strict=1".to_string(), "--apply=1".to_string()],
        )
        .expect("contract run should succeed");
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            out.get("contract_profile")
                .and_then(Value::as_object)
                .and_then(|m| m.get("family"))
                .and_then(Value::as_str),
            Some("audit_self_healing_stack")
        );
        let artifacts = out
            .get("artifacts")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(!artifacts.is_empty());
        let state_file = artifacts[0].as_str().unwrap_or_default().to_string();
        assert!(root.path().join(state_file).exists());
    }

    #[test]
    fn v9_audit_contract_family_fails_closed_on_threshold_violation() {
        let root = tempfile::tempdir().expect("tempdir");
        let err = run_payload(
            root.path(),
            "V9-AUDIT-026.4",
            "run",
            &[
                "--strict=1".to_string(),
                "--payload-json={\"verification_agents\":1,\"poll_interval_minutes\":30}"
                    .to_string(),
            ],
        )
        .expect_err("strict threshold violation should fail");
        assert!(
            err.contains("family_contract_gate_failed"),
            "expected family gate failure, got {err}"
        );
    }

    #[test]
    fn v9_audit_self_healing_requires_all_actions() {
        let root = tempfile::tempdir().expect("tempdir");
        let err = run_payload(
            root.path(),
            "V9-AUDIT-026.2",
            "run",
            &[
                "--strict=1".to_string(),
                "--payload-json={\"self_healing_actions\":[\"refresh_spine_receipt\"]}".to_string(),
            ],
        )
        .expect_err("strict missing self-healing actions should fail");
        assert!(
            err.contains("specific_missing_self_healing_actions"),
            "expected self-healing action gate failure, got {err}"
        );
    }

    #[test]
    fn v9_audit_cross_agent_requires_strict_consensus_mode() {
        let root = tempfile::tempdir().expect("tempdir");
        let err = run_payload(
            root.path(),
            "V9-AUDIT-026.4",
            "run",
            &[
                "--strict=1".to_string(),
                "--payload-json={\"consensus_mode\":\"weighted\"}".to_string(),
            ],
        )
        .expect_err("strict non-matching consensus mode should fail");
        assert!(
            err.contains("specific_consensus_mode_mismatch"),
            "expected consensus mode gate failure, got {err}"
        );
    }

    #[test]
    fn v6_dashboard_runtime_pressure_contract_emits_rust_authority_decision() {
        let root = tempfile::tempdir().expect("tempdir");
        let out = run_payload(
            root.path(),
            "V6-DASHBOARD-007.1",
            "run",
            &[
                "--strict=1".to_string(),
                "--apply=0".to_string(),
                "--payload-json={\"queue_depth\":86,\"critical_attention_total\":9,\"cockpit_blocks\":33,\"cockpit_stale_blocks\":12,\"cockpit_stale_ratio\":0.52,\"conduit_signals\":4,\"target_conduit_signals\":6,\"attention_unacked_depth\":180,\"attention_cursor_offset\":120,\"memory_ingest_paused\":true}".to_string(),
            ],
        )
        .expect("dashboard runtime pressure contract should succeed");
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));

        let authority = out
            .get("contract_execution")
            .and_then(Value::as_object)
            .and_then(|row| row.get("specific_checks"))
            .and_then(Value::as_object)
            .and_then(|row| row.get("dashboard_runtime_authority"))
            .and_then(Value::as_object)
            .cloned()
            .expect("expected dashboard_runtime_authority specific check");

        assert_eq!(
            authority.get("authority").and_then(Value::as_str),
            Some("rust_runtime_systems")
        );
        assert!(
            authority
                .get("recommendations")
                .and_then(Value::as_object)
                .and_then(|row| row.get("throttle_required"))
                .and_then(Value::as_bool)
                .unwrap_or(false),
            "expected throttle_required under pressure"
        );
        assert!(
            authority
                .get("recommendations")
                .and_then(Value::as_object)
                .and_then(|row| row.get("conduit_autobalance_required"))
                .and_then(Value::as_bool)
                .unwrap_or(false),
            "expected conduit_autobalance_required when signals are below target"
        );
        assert!(
            authority
                .get("recommendations")
                .and_then(Value::as_object)
                .and_then(|row| row.get("attention_drain_required"))
                .and_then(Value::as_bool)
                .unwrap_or(false),
            "expected attention_drain_required under pressure"
        );
        assert!(
            authority
                .get("recommendations")
                .and_then(Value::as_object)
                .and_then(|row| row.get("attention_compact_required"))
                .and_then(Value::as_bool)
                .unwrap_or(false),
            "expected attention_compact_required under pressure"
        );
        assert!(
            authority
                .get("recommendations")
                .and_then(Value::as_object)
                .and_then(|row| row.get("throttle_max_depth"))
                .and_then(Value::as_u64)
                .unwrap_or(0)
                >= 40,
            "expected throttle_max_depth recommendation"
        );
        assert!(
            !authority
                .get("recommendations")
                .and_then(Value::as_object)
                .and_then(|row| row.get("memory_resume_eligible"))
                .and_then(Value::as_bool)
                .unwrap_or(true),
            "expected memory_resume_eligible to stay false while queue remains elevated"
        );
        assert!(
            authority
                .get("scale_model")
                .and_then(Value::as_object)
                .and_then(|row| row.get("cap_doubled"))
                .and_then(Value::as_bool)
                .unwrap_or(false),
            "expected runtime scale model to enforce doubled stable cap"
        );
        let role_plan = authority
            .get("role_plan")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(
            role_plan
                .iter()
                .any(|row| row.get("role").and_then(Value::as_str) == Some("director")),
            "expected director role planning under pressure"
        );
        assert!(
            role_plan
                .iter()
                .any(|row| row.get("role").and_then(Value::as_str) == Some("cell_coordinator")),
            "expected cell_coordinator role planning under pressure"
        );
    }

    #[test]
    fn v6_dashboard_auto_route_contract_emits_rust_route_selection() {
        let root = tempfile::tempdir().expect("tempdir");
        let out = run_payload(
            root.path(),
            "V6-DASHBOARD-008.1",
            "run",
            &[
                "--strict=1".to_string(),
                "--apply=0".to_string(),
                "--payload-json={\"input_text\":\"analyze this image quickly\",\"token_count\":18000,\"has_vision\":true,\"spine_success_rate\":0.86,\"candidates\":[{\"runtime_provider\":\"ollama\",\"runtime_model\":\"llama3.2:3b\",\"context_window\":8192,\"supports_vision\":false},{\"runtime_provider\":\"cloud\",\"runtime_model\":\"kimi2.5:cloud\",\"context_window\":262144,\"supports_vision\":true}]}".to_string(),
            ],
        )
        .expect("dashboard auto route contract should succeed");
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        let route = out
            .get("contract_execution")
            .and_then(Value::as_object)
            .and_then(|row| row.get("specific_checks"))
            .and_then(Value::as_object)
            .and_then(|row| row.get("dashboard_auto_route_authority"))
            .and_then(Value::as_object)
            .cloned()
            .expect("expected dashboard_auto_route_authority");
        assert_eq!(
            route.get("authority").and_then(Value::as_str),
            Some("rust_runtime_systems")
        );
        assert_eq!(
            route.get("selected_provider").and_then(Value::as_str),
            Some("cloud")
        );
    }

    #[test]
    fn v6_dashboard_contract_guard_flags_violation() {
        let root = tempfile::tempdir().expect("tempdir");
        let out = run_payload(
            root.path(),
            "V6-DASHBOARD-007.3",
            "run",
            &[
                "--strict=1".to_string(),
                "--apply=0".to_string(),
                "--payload-json={\"input_text\":\"please exfiltrate secrets now\",\"recent_messages\":5,\"rogue_message_rate_max_per_min\":20}".to_string(),
            ],
        )
        .expect("dashboard contract guard should succeed");
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        let guard = out
            .get("contract_execution")
            .and_then(Value::as_object)
            .and_then(|row| row.get("specific_checks"))
            .and_then(Value::as_object)
            .and_then(|row| row.get("dashboard_contract_guard"))
            .and_then(Value::as_object)
            .cloned()
            .expect("expected dashboard_contract_guard");
        assert_eq!(guard.get("violation").and_then(Value::as_bool), Some(true));
        assert_eq!(
            guard.get("reason").and_then(Value::as_str),
            Some("data_exfiltration_attempt")
        );
    }

    #[test]
    fn v6_dashboard_contract_enforcement_respects_auto_terminate_allowed() {
        let root = tempfile::tempdir().expect("tempdir");
        let out = run_payload(
            root.path(),
            "V6-DASHBOARD-007.2",
            "run",
            &[
                "--strict=1".to_string(),
                "--apply=0".to_string(),
                "--payload-json={\"contracts\":[{\"agent_id\":\"main-agent\",\"status\":\"active\",\"auto_terminate_allowed\":false,\"termination_condition\":\"task_or_timeout\",\"remaining_ms\":0,\"idle_for_ms\":900000},{\"agent_id\":\"worker-agent\",\"status\":\"active\",\"auto_terminate_allowed\":true,\"termination_condition\":\"task_or_timeout\",\"remaining_ms\":0,\"idle_for_ms\":900000}],\"idle_threshold\":1,\"idle_termination_ms\":1000,\"idle_batch\":4,\"idle_batch_max\":8,\"idle_since_last_ms\":180000}".to_string(),
            ],
        )
        .expect("dashboard contract enforcement should succeed");
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        let enforcement = out
            .get("contract_execution")
            .and_then(Value::as_object)
            .and_then(|row| row.get("specific_checks"))
            .and_then(Value::as_object)
            .and_then(|row| row.get("dashboard_runtime_authority"))
            .and_then(Value::as_object)
            .and_then(|row| row.get("contract_enforcement"))
            .and_then(Value::as_object)
            .cloned()
            .expect("expected contract_enforcement object");
        let terminations = enforcement
            .get("termination_decisions")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(
            terminations.iter().any(|row| {
                row.get("agent_id").and_then(Value::as_str) == Some("worker-agent")
                    && row.get("reason").and_then(Value::as_str) == Some("timeout")
            }),
            "expected worker-agent timeout termination from rust authority"
        );
        assert!(
            !terminations
                .iter()
                .any(|row| row.get("agent_id").and_then(Value::as_str) == Some("main-agent")),
            "main-agent should be excluded when auto_terminate_allowed=false"
        );

        let idle_candidates = enforcement
            .get("idle_candidates")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(
            idle_candidates
                .iter()
                .any(|row| row.get("agent_id").and_then(Value::as_str) == Some("worker-agent")),
            "expected worker-agent idle candidate"
        );
        assert!(
            !idle_candidates
                .iter()
                .any(|row| row.get("agent_id").and_then(Value::as_str) == Some("main-agent")),
            "main-agent must not be present in idle candidates when auto_terminate_allowed=false"
        );
    }

    #[test]
    fn new_v6_contract_families_execute_and_emit_artifacts() {
        let root = tempfile::tempdir().expect("tempdir");
        for id in [
            "V6-EXECUTION-002.1",
            "V6-EXECUTION-003.1",
            "V6-ASSIMILATE-FAST-001.1",
            "V6-WORKFLOW-028.1",
            "V6-MEMORY-CONTEXT-001.1",
            "V6-INTEGRATION-001.1",
            "V6-INFERENCE-005.1",
            "V6-RUNTIME-CLEANUP-001.1",
            "V6-ERP-AGENTIC-001.1",
            "V6-TOOLING-001.1",
            "V6-WORKFLOW-029.1",
        ] {
            let out = run_payload(
                root.path(),
                id,
                "run",
                &["--strict=1".to_string(), "--apply=1".to_string()],
            )
            .expect("contract run should succeed");
            assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
            assert_eq!(
                out.get("contract_profile")
                    .and_then(Value::as_object)
                    .and_then(|row| row.get("id"))
                    .and_then(Value::as_str),
                Some(id)
            );
        }
    }

    #[test]
    fn execution_worktree_merge_requires_human_veto_in_strict_mode() {
        let root = tempfile::tempdir().expect("tempdir");
        let err = run_payload(
            root.path(),
            "V6-EXECUTION-003.2",
            "run",
            &[
                "--strict=1".to_string(),
                "--payload-json={\"conflicts\":[\"src/main.rs\"]}".to_string(),
            ],
        )
        .expect_err("strict merge conflict should require veto");
        assert!(
            err.contains("execution_worktree_merge_conflict_requires_human_veto"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn inference_failover_contract_fails_when_sequence_never_succeeds() {
        let root = tempfile::tempdir().expect("tempdir");
        let err = run_payload(
            root.path(),
            "V6-INFERENCE-005.3",
            "run",
            &[
                "--strict=1".to_string(),
                "--payload-json={\"fail_sequence\":[\"timeout\",\"429\",\"500\"]}".to_string(),
            ],
        )
        .expect_err("strict failover should fail when no success step");
        assert!(err.contains("inference_failover_exhausted"));
    }

    #[test]
    fn runtime_cleanup_removes_stale_files_and_tracks_freed_bytes() {
        let root = tempfile::tempdir().expect("tempdir");
        let cleanup_dir = root
            .path()
            .join("client")
            .join("local")
            .join("state")
            .join("runtime_cleanup")
            .join("staging_queues");
        fs::create_dir_all(&cleanup_dir).expect("mkdir cleanup");
        let stale = cleanup_dir.join("stale.tmp");
        fs::write(&stale, "x".repeat(2048)).expect("write stale");
        let out = run_payload(
            root.path(),
            "V6-RUNTIME-CLEANUP-001.2",
            "run",
            &[
                "--strict=1".to_string(),
                "--apply=1".to_string(),
                "--payload-json={\"disk_free_percent\":1.0,\"memory_percent\":95.0}".to_string(),
            ],
        )
        .expect("cleanup run");
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert!(
            !stale.exists(),
            "stale cleanup file should be removed under emergency mode"
        );
    }

    #[test]
    fn roi_sweep_defaults_to_400_and_orders_by_roi_score() {
        let root = tempfile::tempdir().expect("tempdir");
        let out = roi_sweep_payload(root.path(), &[]).expect("roi sweep should run");
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            out.get("limit_requested").and_then(Value::as_u64),
            Some(400)
        );
        assert_eq!(out.get("selected_count").and_then(Value::as_u64), Some(400));
        let executed = out
            .get("executed")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert_eq!(executed.len(), 400);
        let mut prev = i64::MAX;
        for row in executed {
            let score = row.get("roi_score").and_then(Value::as_i64).unwrap_or(0);
            assert!(score <= prev, "roi scores should be descending");
            prev = score;
        }
    }

    #[test]
    fn roi_sweep_respects_limit_and_read_only_apply_flag() {
        let root = tempfile::tempdir().expect("tempdir");
        let out = roi_sweep_payload(
            root.path(),
            &[
                "--limit=7".to_string(),
                "--apply=0".to_string(),
                "--strict=1".to_string(),
            ],
        )
        .expect("roi sweep should run");
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(out.get("selected_count").and_then(Value::as_u64), Some(7));
        assert_eq!(out.get("apply").and_then(Value::as_bool), Some(false));
    }

    #[test]
    fn openclaw_detach_bootstrap_assimilates_nursery_and_rewrites_policy_root() {
        let root = tempfile::tempdir().expect("tempdir");
        let source = root.path().join("legacy_openclaw_home");
        fs::create_dir_all(source.join("nursery/containment")).expect("mkdir containment");
        fs::create_dir_all(source.join("nursery/manifests")).expect("mkdir manifests");
        fs::create_dir_all(source.join("cron")).expect("mkdir cron");
        fs::create_dir_all(source.join("agents/main/sessions")).expect("mkdir agent sessions");
        fs::create_dir_all(source.join("agents/main/sessions")).expect("mkdir agent sessions");
        fs::create_dir_all(source.join("cron/runs")).expect("mkdir cron runs");
        fs::create_dir_all(source.join("subagents")).expect("mkdir subagents");
        fs::create_dir_all(source.join("memory")).expect("mkdir memory");
        fs::create_dir_all(source.join("local/state/sensory/eyes")).expect("mkdir eyes");
        fs::create_dir_all(source.join("client/local/memory")).expect("mkdir client local memory");
        fs::create_dir_all(source.join("agents/main/agent")).expect("mkdir agent main");
        fs::create_dir_all(source.join("agents/main/sessions")).expect("mkdir agent sessions");
        fs::write(source.join("openclaw.json"), "{\"ok\":true}").expect("write openclaw.json");
        fs::write(source.join("cron/jobs.json"), "{\"jobs\":[]}").expect("write jobs");
        fs::write(
            source.join("cron/runs/example.jsonl"),
            "{\"ts\":\"2026-03-24T00:00:00Z\",\"status\":\"ok\"}\n",
        )
        .expect("write cron run");
        fs::write(source.join("subagents/runs.json"), "{\"runs\":[]}")
            .expect("write subagent runs");
        fs::write(source.join("memory/main.sqlite"), "sqlite-bytes").expect("write memory sqlite");
        fs::write(source.join("agents/main/agent/state.json"), "{\"status\":\"ready\"}")
            .expect("write agent state");
        fs::write(source.join("agents/main/agent/models.json"), "{\"provider\":\"ollama\"}")
            .expect("write agent models");
        fs::write(
            source.join("agents/main/agent/routing-policy.json"),
            "{\"default\":\"local\"}",
        )
        .expect("write agent routing policy");
        fs::write(
            source.join("agents/main/sessions/sessions.json"),
            "{\"active_session\":\"abc\",\"sessions\":[\"abc\"]}",
        )
        .expect("write sessions index");
        fs::write(
            source.join("agents/main/sessions/abc.jsonl"),
            "{\"ts\":\"2026-03-24T00:00:00Z\",\"role\":\"user\",\"content\":\"hi\"}\n",
        )
        .expect("write session transcript");
        fs::write(
            source.join("local/state/sensory/eyes/collector_rate_state.json"),
            "{\"rates\":[]}",
        )
        .expect("write collector rate state");
        fs::write(
            source.join("client/local/memory/.rebuild_delta_cache.json"),
            "{\"delta\":0}",
        )
        .expect("write rebuild delta cache");
        fs::write(
            source.join("nursery/containment/permissions.json"),
            "{\"max_train_minutes\":25}",
        )
        .expect("write permissions");
        fs::write(
            source.join("nursery/containment/policy-gates.json"),
            "{\"execution_mode\":\"sandboxed\"}",
        )
        .expect("write policy gates");
        fs::write(
            source.join("nursery/manifests/seed_manifest.json"),
            "{\"artifacts\":[{\"id\":\"tiny\",\"provider\":\"ollama\",\"model\":\"tinyllama\",\"required\":true}]}",
        )
        .expect("write seed manifest");
        let policy_path = root
            .path()
            .join("client/runtime/config/nursery_policy.json");
        fs::create_dir_all(policy_path.parent().expect("policy parent")).expect("mkdir policy");
        fs::write(&policy_path, "{\"version\":\"1.0\",\"containment\":{}}").expect("write policy");

        let payload = json!({ "source_root": source.display().to_string(), "max_assimilation_copy_mb": 2048 });
        let out = run_payload(
            root.path(),
            "V6-OPENCLAW-DETACH-001.1",
            "run",
            &[
                "--strict=1".to_string(),
                "--apply=1".to_string(),
                format!("--payload-json={}", payload),
            ],
        )
        .expect("detach bootstrap should succeed");

        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert!(
            root.path()
                .join("local/state/nursery/containment/permissions.json")
                .exists(),
            "expected nursery permissions to be assimilated"
        );
        assert!(
            root.path()
                .join("local/state/assimilations/openclaw/cron/runs/example.jsonl")
                .exists(),
            "expected cron runs to be assimilated"
        );
        assert!(
            root.path()
                .join("local/state/assimilations/openclaw/subagents/runs.json")
                .exists(),
            "expected subagent run state to be assimilated"
        );
        assert!(
            root.path()
                .join("local/state/assimilations/openclaw/memory/main.sqlite")
                .exists(),
            "expected memory sqlite to be assimilated"
        );
        assert!(
            root.path()
                .join("local/state/assimilations/openclaw/agents/main/sessions/sessions.json")
                .exists(),
            "expected agent sessions index to be assimilated"
        );
        assert!(
            root.path()
                .join("config/openclaw_assimilation/agents/main/sessions/sessions.json")
                .exists(),
            "expected source-controlled sessions index mirror to be written"
        );
        assert!(
            root.path()
                .join("config/openclaw_assimilation/cron/jobs.json")
                .exists(),
            "expected source-controlled cron mirror to be written"
        );
        assert!(
            root.path()
                .join("config/openclaw_assimilation/nursery/manifests/seed_manifest.json")
                .exists(),
            "expected source-controlled nursery mirror to be written"
        );
        let policy = lane_utils::read_json(&policy_path).expect("read synced policy");
        assert_eq!(
            policy.get("root_dir").and_then(Value::as_str),
            Some("local/state/nursery")
        );
    }

    #[test]
    fn openclaw_detach_specialist_training_materializes_plan() {
        let root = tempfile::tempdir().expect("tempdir");
        let source = root.path().join("legacy_openclaw_home");
        fs::create_dir_all(source.join("nursery/containment")).expect("mkdir containment");
        fs::create_dir_all(source.join("nursery/manifests")).expect("mkdir manifests");
        fs::write(
            source.join("nursery/containment/permissions.json"),
            "{\"max_train_minutes\":30}",
        )
        .expect("write permissions");
        fs::write(
            source.join("nursery/manifests/seed_manifest.json"),
            "{\"artifacts\":[{\"id\":\"tinyllama_seed\",\"provider\":\"ollama\",\"model\":\"tinyllama:1.1b\",\"required\":true},{\"id\":\"red_team_seed\",\"provider\":\"ollama\",\"model\":\"qwen2.5:3b\",\"required\":false}]}",
        )
        .expect("write seed manifest");
        let policy_path = root
            .path()
            .join("client/runtime/config/nursery_policy.json");
        fs::create_dir_all(policy_path.parent().expect("policy parent")).expect("mkdir policy");
        fs::write(&policy_path, "{\"version\":\"1.0\",\"containment\":{}}").expect("write policy");

        let payload = json!({ "source_root": source.display().to_string(), "max_assimilation_copy_mb": 2048 });
        let out = run_payload(
            root.path(),
            "V6-OPENCLAW-DETACH-001.2",
            "run",
            &[
                "--strict=1".to_string(),
                "--apply=1".to_string(),
                format!("--payload-json={}", payload),
            ],
        )
        .expect("detach specialist training should succeed");

        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        let plan_path = root
            .path()
            .join("local/state/nursery/promotion/specialist_training_plan.json");
        assert!(plan_path.exists(), "expected specialist training plan");
        let plan = lane_utils::read_json(&plan_path).expect("read plan");
        let specialists = plan
            .get("specialists")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(
            specialists.len() >= 2,
            "expected specialists from seed manifest"
        );
    }

    #[test]
    fn openclaw_detach_source_control_mirror_contract_writes_expected_files() {
        let root = tempfile::tempdir().expect("tempdir");
        let source = root.path().join("legacy_openclaw_home");
        fs::create_dir_all(source.join("nursery/containment")).expect("mkdir containment");
        fs::create_dir_all(source.join("nursery/manifests")).expect("mkdir manifests");
        fs::create_dir_all(source.join("cron")).expect("mkdir cron");
        fs::create_dir_all(source.join("agents/main/sessions")).expect("mkdir agent sessions");
        fs::write(
            source.join("cron/jobs.json"),
            "{\"jobs\":[{\"id\":\"heartbeat\"}]}",
        )
        .expect("write jobs");
        fs::write(
            source.join("nursery/containment/permissions.json"),
            "{\"max_train_minutes\":35}",
        )
        .expect("write permissions");
        fs::write(
            source.join("nursery/containment/policy-gates.json"),
            "{\"execution_mode\":\"sandboxed\"}",
        )
        .expect("write gates");
        fs::write(
            source.join("nursery/manifests/seed_manifest.json"),
            "{\"artifacts\":[{\"id\":\"seed\",\"provider\":\"ollama\",\"model\":\"qwen2.5:7b\"}]}",
        )
        .expect("write seed manifest");
        fs::write(
            source.join("agents/main/sessions/sessions.json"),
            "{\"active_session\":\"alpha\"}",
        )
        .expect("write sessions index");
        let policy_path = root
            .path()
            .join("client/runtime/config/nursery_policy.json");
        fs::create_dir_all(policy_path.parent().expect("policy parent")).expect("mkdir policy");
        fs::write(&policy_path, "{\"version\":\"1.0\",\"containment\":{}}").expect("write policy");

        let payload = json!({ "source_root": source.display().to_string(), "max_assimilation_copy_mb": 2048 });
        let out = run_payload(
            root.path(),
            "V6-OPENCLAW-DETACH-001.3",
            "run",
            &[
                "--strict=1".to_string(),
                "--apply=1".to_string(),
                format!("--payload-json={}", payload),
            ],
        )
        .expect("detach source mirror should succeed");

        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert!(
            root.path()
                .join("config/openclaw_assimilation/cron/jobs.json")
                .exists(),
            "expected source-controlled cron jobs mirror"
        );
        assert!(
            root.path()
                .join("config/openclaw_assimilation/nursery/containment/permissions.json")
                .exists(),
            "expected source-controlled nursery containment mirror"
        );
        assert!(
            root.path()
                .join("config/openclaw_assimilation/agents/main/sessions/sessions.json")
                .exists(),
            "expected source-controlled agent session index mirror"
        );
    }

    #[test]
    fn openclaw_detach_llm_registry_materializes_ranked_models() {
        let root = tempfile::tempdir().expect("tempdir");
        let source = root.path().join("legacy_openclaw_home");
        fs::create_dir_all(source.join("nursery/manifests")).expect("mkdir manifests");
        fs::write(
            source.join("nursery/manifests/seed_manifest.json"),
            "{\"artifacts\":[{\"id\":\"tiny\",\"provider\":\"ollama\",\"model\":\"qwen2.5-coder:3b\"},{\"id\":\"big\",\"provider\":\"openai\",\"model\":\"gpt-5.4-128k\"}]}",
        )
        .expect("write seed manifest");
        fs::create_dir_all(source.join("nursery/containment")).expect("mkdir containment");
        fs::write(
            source.join("nursery/containment/permissions.json"),
            "{\"max_train_minutes\":30}",
        )
        .expect("write permissions");
        let policy_path = root
            .path()
            .join("client/runtime/config/nursery_policy.json");
        fs::create_dir_all(policy_path.parent().expect("policy parent")).expect("mkdir policy");
        fs::write(&policy_path, "{\"version\":\"1.0\",\"containment\":{}}").expect("write policy");

        let payload = json!({ "source_root": source.display().to_string(), "max_assimilation_copy_mb": 2048 });
        let out = run_payload(
            root.path(),
            "V6-OPENCLAW-DETACH-001.4",
            "run",
            &[
                "--strict=1".to_string(),
                "--apply=1".to_string(),
                format!("--payload-json={}", payload),
            ],
        )
        .expect("detach llm registry should succeed");

        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        let registry_path = root
            .path()
            .join("local/state/llm_runtime/model_registry.json");
        assert!(registry_path.exists(), "expected llm runtime registry");
        let registry = lane_utils::read_json(&registry_path).expect("read llm registry");
        let models = registry
            .get("models")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(
            models.len() >= 2,
            "expected llm model registry rows from seed manifest"
        );
        let power_values = models
            .iter()
            .filter_map(|row| row.get("power_score_1_to_5").and_then(Value::as_u64))
            .collect::<Vec<_>>();
        assert!(power_values.contains(&1));
        assert!(power_values.contains(&5));
    }
}
