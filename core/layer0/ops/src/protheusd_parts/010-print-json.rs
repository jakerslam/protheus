// Layer ownership: core/layer0/ops (authoritative)
// SPDX-License-Identifier: Apache-2.0

use protheus_ops_core::v8_kernel::{parse_bool_str, parse_u64_str};
use protheus_ops_core::{
    client_state_root, configure_low_memory_allocator_env, daemon_control,
    deterministic_receipt_hash, now_iso, parse_os_args, status_runtime_efficiency_floor,
};
use serde_json::{json, Value};
use std::env;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::path::PathBuf;
use sysinfo::System;

#[cfg(feature = "embedded-minimal-core")]
type PlaneRunner = fn(&Path, &[String]) -> i32;

fn print_json(value: &Value) {
    println!(
        "{}",
        serde_json::to_string_pretty(value)
            .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode_failed\"}".to_string())
    );
}

fn usage() {
    println!("Usage:");
    println!("  infringd status");
    println!("  infringd start [--strict=1|0]");
    println!("  infringd stop [--strict=1|0]");
    println!("  infringd restart [--strict=1|0]");
    println!("  infringd attach [--strict=1|0]");
    println!("  infringd subscribe [--strict=1|0]");
    println!("  infringd tick [--strict=1|0]");
    println!("  infringd diagnostics [--strict=1|0]");
    println!("    start/restart optional dashboard flags:");
    println!("      --dashboard-autoboot=1|0 (default: 1)");
    println!("      --dashboard-open=1|0     (default: 1)");
    println!("      --dashboard-host=<ip> --dashboard-port=<n>");
    println!("  infringd daemon-control <...> (internal installer compat alias)");
    println!("  infringd dashboard-ui <...>   (internal installer compat alias)");
    println!("  infringd think --prompt=<text> [--session-id=<id>] [--memory-limit=<n>]");
    println!("  infringd research <status|fetch|diagnostics> [flags]");
    println!("  infringd memory <status|write|query> [flags]");
    println!("  infringd orchestration <invoke|help> [flags]");
    println!("  infringd swarm-runtime <status|spawn|sessions|results|tick|metrics|test> [flags]");
    println!("  infringd capability-profile [--hardware-class=<mcu|legacy|standard|high>] [--memory-mb=<n>] [--cpu-cores=<n>] [--tiny-max=1|0]");
    println!("  infringd efficiency-status");
    #[cfg(feature = "embedded-minimal-core")]
    println!("  infringd embedded-core-status");
    #[cfg(feature = "tiny")]
    println!("  infringd tiny-status");
    #[cfg(feature = "embedded-max")]
    println!("  infringd tiny-max-status");
}

fn cli_error(error: &str, command: &str) -> Value {
    let mut out = json!({
        "ok": false,
        "type": "protheusd_error",
        "command": command,
        "error": error,
        "ts": now_iso()
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}

fn parse_flag(argv: &[String], key: &str) -> Option<String> {
    let pref = format!("--{key}=");
    let key_token = format!("--{key}");
    let mut idx = 0usize;
    while idx < argv.len() {
        let token = argv[idx].trim();
        if let Some(value) = token.strip_prefix(&pref) {
            return Some(value.trim().to_string());
        }
        if token == key_token {
            if let Some(next) = argv.get(idx + 1) {
                let next_trimmed = next.trim();
                if !next_trimmed.starts_with("--") {
                    return Some(next_trimmed.to_string());
                }
            }
        }
        idx += 1;
    }
    None
}

fn clean_token(raw: Option<&str>, fallback: &str) -> String {
    let mut out = String::new();
    let source = raw.unwrap_or("").trim();
    for ch in source.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | ':' | '.') {
            out.push(ch);
        } else if ch.is_ascii_whitespace() && !out.ends_with('_') {
            out.push('_');
        }
        if out.len() >= 64 {
            break;
        }
    }
    let trimmed = out.trim_matches('_').to_string();
    if trimmed.is_empty() {
        fallback.to_string()
    } else {
        trimmed
    }
}

fn clean_text(raw: Option<&str>, max_len: usize) -> String {
    protheus_ops_core::contract_lane_utils::clean_text(raw, max_len)
}

fn parse_usize(raw: Option<&str>, fallback: usize, min: usize, max: usize) -> usize {
    raw.and_then(|v| v.trim().parse::<usize>().ok())
        .unwrap_or(fallback)
        .clamp(min, max)
}

fn parse_bool_switch(argv: &[String], key: &str, fallback: bool) -> bool {
    let token = format!("--{key}");
    if argv.iter().any(|arg| arg.trim() == token) {
        return true;
    }
    parse_bool_str(parse_flag(argv, key).as_deref(), fallback)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RuntimeHardwareClass {
    Microcontroller,
    Legacy,
    Standard,
    High,
}

impl RuntimeHardwareClass {
    fn as_str(self) -> &'static str {
        match self {
            Self::Microcontroller => "mcu",
            Self::Legacy => "legacy",
            Self::Standard => "standard",
            Self::High => "high",
        }
    }
}

#[derive(Clone, Debug)]
struct RuntimeCapabilityProfile {
    hardware_class: RuntimeHardwareClass,
    tiny_max: bool,
    sensed_memory_mb: u64,
    sensed_cpu_cores: usize,
    max_memory_hits: usize,
    allow_research_fetch: bool,
    allow_orchestration: bool,
    allow_swarm_spawn: bool,
    allow_persistent_swarm: bool,
    max_swarm_depth: u8,
}

impl RuntimeCapabilityProfile {
    fn allows_orchestration_op(&self, op: &str) -> bool {
        if !self.allow_orchestration {
            return false;
        }
        if self.hardware_class != RuntimeHardwareClass::Microcontroller {
            return true;
        }
        matches!(
            op,
            "scope.detect_overlaps"
                | "scope.classify"
                | "scratchpad.status"
                | "scratchpad.write"
                | "scratchpad.append_finding"
                | "scratchpad.append_checkpoint"
                | "checkpoint.should"
                | "checkpoint.tick"
                | "checkpoint.timeout"
                | "coordinator.partition"
                | "coordinator.merge_findings"
        )
    }

    fn as_json(&self) -> Value {
        let mut shed = Vec::<String>::new();
        if !self.allow_research_fetch {
            shed.push("research.fetch".to_string());
        }
        if !self.allow_persistent_swarm {
            shed.push("swarm.persistent".to_string());
        }
        if self.max_swarm_depth <= 1 {
            shed.push("swarm.max_depth>1".to_string());
        }
        if self.max_memory_hits <= 2 {
            shed.push("think.memory_hits>2".to_string());
        }
        json!({
            "hardware_class": self.hardware_class.as_str(),
            "tiny_max": self.tiny_max,
            "sensed_memory_mb": self.sensed_memory_mb,
            "sensed_cpu_cores": self.sensed_cpu_cores,
            "limits": {
                "max_memory_hits": self.max_memory_hits,
                "max_swarm_depth": self.max_swarm_depth
            },
            "capabilities": {
                "research_fetch": self.allow_research_fetch,
                "orchestration": self.allow_orchestration,
                "swarm_spawn": self.allow_swarm_spawn,
                "swarm_persistent": self.allow_persistent_swarm
            },
            "shed_capabilities": shed
        })
    }
}

fn parse_hardware_class(raw: Option<&str>) -> Option<RuntimeHardwareClass> {
    match raw.map(|v| v.trim().to_ascii_lowercase()) {
        Some(v) if matches!(v.as_str(), "mcu" | "microcontroller" | "embedded") => {
            Some(RuntimeHardwareClass::Microcontroller)
        }
        Some(v) if matches!(v.as_str(), "legacy" | "old" | "ancient") => {
            Some(RuntimeHardwareClass::Legacy)
        }
        Some(v) if matches!(v.as_str(), "standard" | "edge") => {
            Some(RuntimeHardwareClass::Standard)
        }
        Some(v) if matches!(v.as_str(), "high" | "desktop" | "server") => {
            Some(RuntimeHardwareClass::High)
        }
        _ => None,
    }
}

fn parse_u8_flag(argv: &[String], key: &str, default: u8) -> u8 {
    let parsed = parse_u64_str(parse_flag(argv, key).as_deref(), default as u64);
    parsed.clamp(0, u8::MAX as u64) as u8
}

fn tiny_max_requested(argv: &[String]) -> bool {
    parse_bool_switch(argv, "tiny-max", false)
        || parse_bool_switch(argv, "tiny_max", false)
        || parse_bool_str(env::var("PROTHEUS_EMBEDDED_MAX").ok().as_deref(), false)
        || cfg!(feature = "embedded-max")
}

fn sensed_memory_mb(argv: &[String]) -> u64 {
    let parsed_flag = parse_u64_str(parse_flag(argv, "memory-mb").as_deref(), 0);
    if parsed_flag > 0 {
        return parsed_flag.max(64);
    }
    let parsed_env = parse_u64_str(env::var("PROTHEUS_HW_MEMORY_MB").ok().as_deref(), 0);
    if parsed_env > 0 {
        return parsed_env.max(64);
    }
    let mut system = System::new_all();
    system.refresh_memory();
    let mb = (system.total_memory() as f64 / 1024.0).round() as u64;
    mb.max(64)
}

fn sensed_cpu_cores(argv: &[String]) -> usize {
    let parsed_flag = parse_u64_str(parse_flag(argv, "cpu-cores").as_deref(), 0);
    if parsed_flag > 0 {
        return parsed_flag.clamp(1, usize::MAX as u64) as usize;
    }
    let parsed_env = parse_u64_str(env::var("PROTHEUS_HW_CPU_CORES").ok().as_deref(), 0);
    if parsed_env > 0 {
        return parsed_env.clamp(1, usize::MAX as u64) as usize;
    }
    num_cpus::get().max(1)
}

#[derive(Clone, Debug)]
struct ProfiledRunArgs {
    profile: RuntimeCapabilityProfile,
    rest: Vec<String>,
}

fn profiled_run_args(argv: &[String], default_subcommand: &str) -> ProfiledRunArgs {
    let profile = runtime_capability_profile(argv);
    let mut rest = strip_runtime_profile_flags(argv);
    if rest.is_empty() {
        rest.push(default_subcommand.to_string());
    }
    ProfiledRunArgs { profile, rest }
}

fn enforce_profile<T>(
    profile: &RuntimeCapabilityProfile,
    argv: &[String],
    command: &str,
    validator: impl FnOnce(&RuntimeCapabilityProfile, &[String]) -> Result<T, String>,
) -> Result<T, i32> {
    match validator(profile, argv) {
        Ok(value) => Ok(value),
        Err(err) => {
            print_json(&cli_error(err.as_str(), command));
            Err(1)
        }
    }
}

fn infer_hardware_class(memory_mb: u64, cpu_cores: usize, tiny_max: bool) -> RuntimeHardwareClass {
    if memory_mb <= 768 || cpu_cores <= 1 {
        RuntimeHardwareClass::Microcontroller
    } else if memory_mb <= 4096 || cpu_cores <= 2 {
        RuntimeHardwareClass::Legacy
    } else if tiny_max && (memory_mb <= 8192 || cpu_cores <= 4) {
        RuntimeHardwareClass::Standard
    } else {
        RuntimeHardwareClass::High
    }
}

fn runtime_capability_profile(argv: &[String]) -> RuntimeCapabilityProfile {
    let tiny_max = tiny_max_requested(argv);
    let memory_mb = sensed_memory_mb(argv);
    let cpu_cores = sensed_cpu_cores(argv);
    let override_class = parse_hardware_class(
        parse_flag(argv, "hardware-class")
            .or_else(|| parse_flag(argv, "device-class"))
            .or_else(|| env::var("PROTHEUS_HW_CLASS").ok())
            .as_deref(),
    );
    let hardware_class =
        override_class.unwrap_or_else(|| infer_hardware_class(memory_mb, cpu_cores, tiny_max));

    match hardware_class {
        RuntimeHardwareClass::Microcontroller => RuntimeCapabilityProfile {
            hardware_class,
            tiny_max,
            sensed_memory_mb: memory_mb,
            sensed_cpu_cores: cpu_cores,
            max_memory_hits: 2,
            allow_research_fetch: false,
            allow_orchestration: true,
            allow_swarm_spawn: true,
            allow_persistent_swarm: false,
            max_swarm_depth: 1,
        },
        RuntimeHardwareClass::Legacy => RuntimeCapabilityProfile {
            hardware_class,
            tiny_max,
            sensed_memory_mb: memory_mb,
            sensed_cpu_cores: cpu_cores,
            max_memory_hits: 4,
            allow_research_fetch: true,
            allow_orchestration: true,
            allow_swarm_spawn: true,
            allow_persistent_swarm: false,
            max_swarm_depth: 2,
        },
        RuntimeHardwareClass::Standard => RuntimeCapabilityProfile {
            hardware_class,
            tiny_max,
            sensed_memory_mb: memory_mb,
            sensed_cpu_cores: cpu_cores,
            max_memory_hits: 8,
            allow_research_fetch: true,
            allow_orchestration: true,
            allow_swarm_spawn: true,
            allow_persistent_swarm: !tiny_max,
            max_swarm_depth: 4,
        },
        RuntimeHardwareClass::High => RuntimeCapabilityProfile {
            hardware_class,
            tiny_max,
            sensed_memory_mb: memory_mb,
            sensed_cpu_cores: cpu_cores,
            max_memory_hits: 20,
            allow_research_fetch: true,
            allow_orchestration: true,
            allow_swarm_spawn: true,
            allow_persistent_swarm: true,
            max_swarm_depth: 8,
        },
    }
}

fn strip_runtime_profile_flags(argv: &[String]) -> Vec<String> {
    let drop_next_for = [
        "--hardware-class",
        "--device-class",
        "--memory-mb",
        "--cpu-cores",
        "--tiny-max",
        "--tiny_max",
    ];
    let drop_prefixes = [
        "--hardware-class=",
        "--device-class=",
        "--memory-mb=",
        "--cpu-cores=",
        "--tiny-max=",
        "--tiny_max=",
    ];
    let mut out = Vec::<String>::new();
    let mut skip_next = false;
    for token in argv {
        if skip_next {
            skip_next = false;
            continue;
        }
        let trimmed = token.trim();
        if drop_next_for.contains(&trimmed) {
            skip_next = true;
            continue;
        }
        if drop_prefixes
            .iter()
            .any(|prefix| trimmed.starts_with(prefix))
        {
            continue;
        }
        out.push(token.clone());
    }
    out
}

fn capability_profile_payload(argv: &[String]) -> Value {
    let profile = runtime_capability_profile(argv);
    let mut out = json!({
        "ok": true,
        "type": "protheusd_capability_profile",
        "ts": now_iso(),
        "profile": profile.as_json()
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}

fn validate_orchestration_profile(
    profile: &RuntimeCapabilityProfile,
    argv: &[String],
) -> Result<(), String> {
    if !profile.allow_orchestration {
        return Err("hardware_profile_blocks_orchestration".to_string());
    }
    let sub = argv
        .first()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "help".to_string());
    if sub != "invoke" {
        return Ok(());
    }
    let op = clean_token(parse_flag(argv, "op").as_deref(), "");
    if op.is_empty() {
        return Err("orchestration_op_required".to_string());
    }
    if !profile.allows_orchestration_op(op.as_str()) {
        return Err(format!("hardware_profile_blocks_orchestration_op:{}", op));
    }
    Ok(())
}
