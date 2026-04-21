// SPDX-License-Identifier: Apache-2.0
use crate::contract_lane_utils as lane_utils;
use crate::runtime_system_contracts::profile_for as runtime_contract_profile_for;
use crate::runtime_systems;
use crate::{clean, now_iso, parse_args};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const INDEXED_LANE_SCRIPT: &str = "lane:run";
const INDEXED_TEST_LANE_SCRIPT: &str = "test:lane:run";

fn state_root(root: &Path) -> PathBuf {
    if let Ok(v) = std::env::var("BACKLOG_QUEUE_EXECUTOR_STATE_ROOT") {
        let s = v.trim();
        if !s.is_empty() {
            return PathBuf::from(s);
        }
    }
    root.join("client")
        .join("local")
        .join("state")
        .join("ops")
        .join("backlog_queue_executor")
}

fn latest_path(root: &Path) -> PathBuf {
    state_root(root).join("latest.json")
}

fn history_path(root: &Path) -> PathBuf {
    state_root(root).join("history.jsonl")
}

fn print_receipt(value: &Value) {
    println!(
        "{}",
        serde_json::to_string_pretty(value)
            .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode_failed\"}".to_string())
    );
}

fn lane_route(use_core_contract_lane: bool, use_dynamic_lane: bool) -> &'static str {
    if use_core_contract_lane {
        "core_runtime_systems"
    } else if use_dynamic_lane {
        "dynamic_legacy_adapter"
    } else {
        "indexed_npm_script"
    }
}

fn lane_script_value(
    id: &str,
    lane_script: &str,
    use_core_contract_lane: bool,
    use_dynamic_lane: bool,
) -> Value {
    if use_core_contract_lane {
        Value::String(format!("core:runtime-systems:{id}"))
    } else if use_dynamic_lane {
        Value::String(format!("dynamic:legacy_alias_adapter:{id}"))
    } else {
        Value::String(format!("{lane_script} -- --id={id}"))
    }
}

fn test_script_value(
    id: &str,
    test_exists: bool,
    use_dynamic_lane: bool,
    use_core_contract_lane: bool,
    test_script: &str,
) -> Value {
    if test_exists && !use_dynamic_lane && !use_core_contract_lane {
        Value::String(format!("{test_script} -- --id={id}"))
    } else {
        Value::Null
    }
}

fn row_scripts_and_route(
    id: &str,
    lane_script: &str,
    test_script: &str,
    test_exists: bool,
    use_core_contract_lane: bool,
    use_dynamic_lane: bool,
) -> (Value, Value, &'static str) {
    (
        lane_script_value(id, lane_script, use_core_contract_lane, use_dynamic_lane),
        test_script_value(
            id,
            test_exists,
            use_dynamic_lane,
            use_core_contract_lane,
            test_script,
        ),
        lane_route(use_core_contract_lane, use_dynamic_lane),
    )
}

fn parse_srs_rows(path: &Path) -> Vec<(String, String)> {
    let Ok(raw) = fs::read_to_string(path) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for line in raw.lines() {
        let l = line.trim();
        if !l.starts_with('|') {
            continue;
        }
        let cells: Vec<String> = l
            .trim_matches('|')
            .split('|')
            .map(|v| v.trim().to_string())
            .collect();
        if cells.len() < 2 {
            continue;
        }
        let id = cells[0].trim();
        if id == "ID" || id.starts_with("---") {
            continue;
        }
        if !id.starts_with('V') || !id.contains('-') {
            continue;
        }
        out.push((id.to_string(), cells[1].trim().to_ascii_lowercase()));
    }
    out
}

fn parse_ids_csv(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(|v| clean(v, 256).to_ascii_uppercase())
        .filter(|v| !v.is_empty())
        .collect()
}

fn lane_registry_path(root: &Path) -> PathBuf {
    root.join("client")
        .join("runtime")
        .join("config")
        .join("lane_command_registry.json")
}

fn load_lane_registry(root: &Path) -> Value {
    let path = lane_registry_path(root);
    let Ok(raw) = fs::read_to_string(path) else {
        return Value::Null;
    };
    serde_json::from_str::<Value>(&raw).unwrap_or(Value::Null)
}

fn lane_registry_command(registry: &Value, section: &str, id: &str) -> Option<String> {
    registry
        .get(section)
        .and_then(|value| value.get(id))
        .and_then(|value| value.get("command"))
        .and_then(Value::as_str)
        .map(|value| clean(value, 4000))
}

fn detect_missing_node_entrypoint(root: &Path, script_cmd: &str) -> Option<String> {
    let segment = script_cmd
        .split("&&")
        .next()
        .unwrap_or(script_cmd)
        .split("||")
        .next()
        .unwrap_or(script_cmd)
        .trim();
    let mut parts = segment.split_whitespace();
    let runner = parts.next()?;
    if runner != "node" {
        return None;
    }

    let mut entry = parts.next()?;
    while entry.starts_with('-') {
        entry = parts.next()?;
    }
    let entry = entry.trim_matches('"').trim_matches('\'');
    if entry.is_empty() || entry.starts_with('$') {
        return None;
    }

    let path = root.join(entry);
    if path.exists() {
        None
    } else {
        Some(entry.to_string())
    }
}

fn parse_npm_run_target(script_cmd: &str) -> Option<String> {
    let segment = script_cmd
        .split("&&")
        .next()
        .unwrap_or(script_cmd)
        .split("||")
        .next()
        .unwrap_or(script_cmd)
        .trim();
    let mut parts = segment.split_whitespace();
    if parts.next()? != "npm" {
        return None;
    }
    if parts.next()? != "run" {
        return None;
    }
    for token in parts {
        if token == "--" {
            continue;
        }
        if token.starts_with('-') {
            continue;
        }
        return Some(token.to_string());
    }
    None
}

fn detect_missing_entrypoint_for_script(
    root: &Path,
    scripts: &serde_json::Map<String, Value>,
    script_name: &str,
    depth: usize,
    seen: &mut std::collections::HashSet<String>,
) -> Option<String> {
    if depth > 6 {
        return None;
    }
    if !seen.insert(script_name.to_string()) {
        return None;
    }
    let cmd = scripts.get(script_name).and_then(|v| v.as_str())?;
    if let Some(missing) = detect_missing_node_entrypoint(root, cmd) {
        return Some(missing);
    }
    if let Some(nested) = parse_npm_run_target(cmd) {
        return detect_missing_entrypoint_for_script(root, scripts, &nested, depth + 1, seen);
    }
    None
}

fn load_npm_scripts(root: &Path) -> serde_json::Map<String, Value> {
    let pkg_path = root.join("package.json");
    let Ok(raw) = fs::read_to_string(pkg_path) else {
        return serde_json::Map::new();
    };
    let Ok(val) = serde_json::from_str::<Value>(&raw) else {
        return serde_json::Map::new();
    };
    val.get("scripts")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default()
}
