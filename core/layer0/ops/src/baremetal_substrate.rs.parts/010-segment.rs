// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use serde_json::{json, Map, Value};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::contract_lane_utils::{
    self as lane_utils, clean_text, clean_token, cli_error, cli_receipt, json_bool as parse_bool,
    json_u64 as parse_u64, path_flag, payload_obj, print_json_line, string_set,
};
use crate::now_iso;

const DEFAULT_STATE_REL: &str = "local/state/ops/baremetal_substrate/latest.json";
const DEFAULT_HISTORY_REL: &str = "local/state/ops/baremetal_substrate/history.jsonl";
const DEFAULT_LEDGER_REL: &str = "local/state/ops/baremetal_substrate/fs_ledger.jsonl";

fn usage() {
    println!("baremetal-substrate commands:");
    println!("  protheus-ops baremetal-substrate status [--state-path=<path>]");
    println!("  protheus-ops baremetal-substrate boot-kernel [--payload-base64=<json>] [--state-path=<path>] [--history-path=<path>]");
    println!("  protheus-ops baremetal-substrate schedule [--payload-base64=<json>] [--state-path=<path>] [--history-path=<path>]");
    println!("  protheus-ops baremetal-substrate memory-manager [--payload-base64=<json>] [--state-path=<path>] [--history-path=<path>]");
    println!("  protheus-ops baremetal-substrate fs-driver [--payload-base64=<json>] [--state-path=<path>] [--history-path=<path>] [--ledger-path=<path>]");
    println!("  protheus-ops baremetal-substrate network-stack [--payload-base64=<json>] [--state-path=<path>] [--history-path=<path>]");
    println!("  protheus-ops baremetal-substrate security-model [--payload-base64=<json>] [--state-path=<path>] [--history-path=<path>]");
}

fn payload_json(argv: &[String]) -> Result<Value, String> {
    lane_utils::payload_json(argv, "baremetal_substrate")
}

fn state_path(root: &Path, argv: &[String], payload: &Map<String, Value>) -> PathBuf {
    path_flag(
        root,
        argv,
        payload,
        "state-path",
        "state_path",
        DEFAULT_STATE_REL,
    )
}

fn history_path(root: &Path, argv: &[String], payload: &Map<String, Value>) -> PathBuf {
    path_flag(
        root,
        argv,
        payload,
        "history-path",
        "history_path",
        DEFAULT_HISTORY_REL,
    )
}

fn ledger_path(root: &Path, argv: &[String], payload: &Map<String, Value>) -> PathBuf {
    path_flag(
        root,
        argv,
        payload,
        "ledger-path",
        "ledger_path",
        DEFAULT_LEDGER_REL,
    )
}

fn default_state() -> Value {
    json!({
        "schema_version": "baremetal_substrate_state_v1",
        "boot_events": {},
        "schedule_events": {},
        "memory_events": {},
        "fs_events": {},
        "network_events": {},
        "security_events": {},
        "ledger_head": "GENESIS",
        "last_receipt": null,
    })
}

fn ensure_state_shape(value: &mut Value) {
    if !value.is_object() {
        *value = default_state();
        return;
    }
    for key in [
        "boot_events",
        "schedule_events",
        "memory_events",
        "fs_events",
        "network_events",
        "security_events",
    ] {
        if !value.get(key).map(Value::is_object).unwrap_or(false) {
            value[key] = json!({});
        }
    }
    if value
        .get("schema_version")
        .and_then(Value::as_str)
        .is_none()
    {
        value["schema_version"] = json!("baremetal_substrate_state_v1");
    }
    if value.get("ledger_head").and_then(Value::as_str).is_none() {
        value["ledger_head"] = json!("GENESIS");
    }
}

fn load_state(path: &Path) -> Value {
    let mut state = lane_utils::read_json(path).unwrap_or_else(default_state);
    ensure_state_shape(&mut state);
    state
}

fn save_state(path: &Path, state: &Value) -> Result<(), String> {
    lane_utils::write_json(path, state)
}

fn append_history(path: &Path, row: &Value) -> Result<(), String> {
    lane_utils::append_jsonl(path, row)
}

fn as_object_mut<'a>(value: &'a mut Value, key: &str) -> &'a mut Map<String, Value> {
    if !value.get(key).map(Value::is_object).unwrap_or(false) {
        value[key] = json!({});
    }
    value
        .get_mut(key)
        .and_then(Value::as_object_mut)
        .expect("object")
}

fn json_f64(raw: Option<&Value>, fallback: f64, min: f64, max: f64) -> f64 {
    raw.and_then(Value::as_f64)
        .or_else(|| raw.and_then(Value::as_u64).map(|n| n as f64))
        .unwrap_or(fallback)
        .clamp(min, max)
}

fn bool_field(raw: Option<&Value>, fallback: bool) -> bool {
    parse_bool(raw, fallback)
}

fn object_field<'a>(payload: &'a Map<String, Value>, key: &str) -> &'a Map<String, Value> {
    payload
        .get(key)
        .and_then(Value::as_object)
        .unwrap_or_else(|| {
            static EMPTY: std::sync::OnceLock<Map<String, Value>> = std::sync::OnceLock::new();
            EMPTY.get_or_init(Map::new)
        })
}

fn now_millis() -> u128 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

fn to_base36(mut value: u128) -> String {
    if value == 0 {
        return "0".to_string();
    }
    let mut out = Vec::new();
    while value > 0 {
        let digit = (value % 36) as u8;
        out.push(if digit < 10 {
            (b'0' + digit) as char
        } else {
            (b'a' + digit - 10) as char
        });
        value /= 36;
    }
    out.iter().rev().collect()
}

fn stable_id(prefix: &str, basis: &Value) -> String {
    let digest = deterministic_receipt_hash(basis);
    format!("{prefix}_{}_{}", to_base36(now_millis()), &digest[..12])
}

fn boot_kernel(state: &mut Value, payload: &Map<String, Value>) -> Result<Value, String> {
    let arch = clean_token(payload.get("arch").and_then(Value::as_str), "x86_64");
    let arch_supported = matches!(arch.as_str(), "x86_64" | "arm64" | "riscv64");
    if !arch_supported {
        return Err("baremetal_substrate_arch_unsupported".to_string());
    }
    let firmware = clean_token(payload.get("firmware").and_then(Value::as_str), "uefi");
    let strict_boot = bool_field(payload.get("strict_boot"), true);
    let boot_ms = parse_u64(payload.get("boot_ms"), 3500, 100, 120_000);
    if strict_boot && boot_ms > 5000 {
        return Err("baremetal_substrate_boot_time_budget_exceeded".to_string());
    }

    let drivers = object_field(payload, "drivers");
    let cpu_driver = bool_field(drivers.get("cpu"), true);
    let gpu_driver = bool_field(drivers.get("gpu"), true);
    let storage_driver = bool_field(drivers.get("storage"), true);
    let network_driver = bool_field(drivers.get("network"), true);
    if !(cpu_driver && gpu_driver && storage_driver && network_driver) {
        return Err("baremetal_substrate_driver_probe_failed".to_string());
    }
    let hardware_year = parse_u64(payload.get("hardware_year"), 2020, 1995, 2028);
    let legacy_compatible =
        hardware_year <= 2001 || bool_field(payload.get("legacy_compat_mode"), false);

    let record = json!({
        "boot_id": stable_id("bmboot", &json!({"arch": arch, "firmware": firmware, "boot_ms": boot_ms})),
        "arch": arch,
        "firmware": firmware,
        "boot_ms": boot_ms,
        "agent_ready": true,
        "driver_probe": {
            "cpu": cpu_driver,
            "gpu": gpu_driver,
            "storage": storage_driver,
            "network": network_driver,
        },
        "hardware_year": hardware_year,
        "legacy_compatible": legacy_compatible,
        "recorded_at": now_iso(),
    });
    let boot_id = record["boot_id"].as_str().unwrap().to_string();
    as_object_mut(state, "boot_events").insert(boot_id, record.clone());
    Ok(json!({
        "ok": true,
        "boot_event": record,
        "claim_evidence": [{
            "id": "V10-BAREMETAL-001.1",
            "claim": "kernel_boot_path_and_direct_driver_probe_are_receipted_and_fail_closed"
        }]
    }))
}

fn schedule(state: &mut Value, payload: &Map<String, Value>) -> Result<Value, String> {
    let agent_count = parse_u64(payload.get("agent_count"), 100, 1, 10_000);
    let realtime_agents = parse_u64(payload.get("realtime_agents"), 0, 0, agent_count);
    let preemption_latency_us = parse_u64(payload.get("preemption_latency_us"), 900, 10, 60_000);
    if preemption_latency_us > 1000 {
        return Err("baremetal_substrate_preemption_latency_budget_exceeded".to_string());
    }
    let throughput_degradation_pct =
        json_f64(payload.get("throughput_degradation_pct"), 2.0, 0.0, 100.0);
    if throughput_degradation_pct > 5.0 {
        return Err("baremetal_substrate_throughput_degradation_budget_exceeded".to_string());
    }
    let thorn_cells = parse_u64(payload.get("thorn_cells"), 0, 0, 10_000);
    let thorn_cap = (agent_count / 10).max(1);
    if thorn_cells > thorn_cap {
        return Err("baremetal_substrate_thorn_cell_cap_exceeded".to_string());
    }
    let priorities = string_set(payload.get("priority_lanes"));
    let record = json!({
        "schedule_id": stable_id("bmsched", &json!({"agent_count": agent_count, "preemption_latency_us": preemption_latency_us})),
        "agent_count": agent_count,
        "realtime_agents": realtime_agents,
        "preemption_latency_us": preemption_latency_us,
        "throughput_degradation_pct": throughput_degradation_pct,
        "thorn_cells": thorn_cells,
        "thorn_cell_cap": thorn_cap,
        "priority_lanes": priorities,
        "scheduler_type": "preemptive_priority",
        "recorded_at": now_iso(),
    });
    let schedule_id = record["schedule_id"].as_str().unwrap().to_string();
    as_object_mut(state, "schedule_events").insert(schedule_id, record.clone());
    Ok(json!({
        "ok": true,
        "schedule_event": record,
        "claim_evidence": [{
            "id": "V10-BAREMETAL-001.2",
            "claim": "preemptive_priority_scheduler_enforces_latency_and_thorn_caps_receipted"
        }]
    }))
}

