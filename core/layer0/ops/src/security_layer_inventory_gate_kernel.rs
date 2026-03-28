// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::contract_lane_utils as lane_utils;
use crate::now_iso;
#[path = "security_layer_inventory_gate_types.rs"]
mod security_layer_inventory_gate_types;
use security_layer_inventory_gate_types::{
    GuardRegistry, InventoryConfig, InventorySummary, LayerResult, MissingGuardCheck, MissingPath,
    RuntimeCheckResult, RuntimeCheckSpec,
};

const INVENTORY_CONFIG_REL: &str = "client/runtime/config/security_layer_inventory.json";
const GUARD_REGISTRY_REL: &str = "client/runtime/config/guard_check_registry.json";
const STATE_DIR_REL: &str = "client/runtime/local/state/ops/security_layer_inventory_gate";
const LATEST_REL: &str = "client/runtime/local/state/ops/security_layer_inventory_gate/latest.json";
const HISTORY_REL: &str = "client/runtime/local/state/ops/security_layer_inventory_gate/history.jsonl";
const DOC_REL: &str = "docs/client/security/SECURITY_LAYER_INVENTORY.md";
const REPORT_REL: &str = "local/workspace/reports/SECURITY_LAYER_INVENTORY_CURRENT.md";


fn usage() {
    println!("security-layer-inventory-gate-kernel commands:");
    println!(
        "  protheus-ops security-layer-inventory-gate-kernel <run|status> [--strict=1|0] [--write=1|0]"
    );
}

fn print_json(value: &Value) {
    println!(
        "{}",
        serde_json::to_string_pretty(value)
            .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode_failed\"}".to_string())
    );
}

fn truncate_chars(raw: &str, limit: usize) -> String {
    raw.chars().take(limit).collect::<String>()
}

fn parse_last_json(raw: &str) -> Option<Value> {
    let text = raw.trim();
    if text.is_empty() {
        return None;
    }
    if let Ok(value) = serde_json::from_str::<Value>(text) {
        return Some(value);
    }
    if let (Some(start), Some(end)) = (text.find('{'), text.rfind('}')) {
        if end > start {
            if let Ok(value) = serde_json::from_str::<Value>(&text[start..=end]) {
                return Some(value);
            }
        }
    }
    for line in text.lines().rev() {
        let line = line.trim();
        if line.starts_with('{') && line.ends_with('}') {
            if let Ok(value) = serde_json::from_str::<Value>(line) {
                return Some(value);
            }
        }
    }
    None
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path, kind: &str) -> Result<T, String> {
    let raw = fs::read_to_string(path).map_err(|err| format!("{kind}_read_failed:{err}"))?;
    serde_json::from_str::<T>(&raw).map_err(|err| format!("{kind}_decode_failed:{err}"))
}

fn parse_mode(argv: &[String]) -> String {
    argv.iter()
        .find(|token| !token.trim_start().starts_with("--"))
        .map(|token| token.trim().to_ascii_lowercase())
        .filter(|token| !token.is_empty())
        .unwrap_or_else(|| "run".to_string())
}

fn sha256_value(value: &Value) -> String {
    let encoded = serde_json::to_string(value).unwrap_or_else(|_| "{}".to_string());
    let mut hasher = Sha256::new();
    hasher.update(encoded.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn ops_binary(root: &Path) -> PathBuf {
    if let Ok(explicit) = std::env::var("PROTHEUS_OPS_BIN") {
        let trimmed = explicit.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    let bin_name = if cfg!(target_os = "windows") {
        "protheus-ops.exe"
    } else {
        "protheus-ops"
    };
    let debug = root.join("target").join("debug").join(bin_name);
    if debug.exists() {
        return debug;
    }
    let release = root.join("target").join("release").join(bin_name);
    if release.exists() {
        return release;
    }
    std::env::current_exe().unwrap_or_else(|_| PathBuf::from("protheus-ops"))
}

fn run_runtime_check(root: &Path, spec: &RuntimeCheckSpec) -> RuntimeCheckResult {
    let plane = if spec.plane.trim().is_empty() {
        "security-plane".to_string()
    } else {
        spec.plane.trim().to_string()
    };
    let command = if spec.command.trim().is_empty() {
        "status".to_string()
    } else {
        spec.command.trim().to_string()
    };
    let args = if spec.args.is_empty() {
        vec!["status".to_string(), "--strict=1".to_string()]
    } else {
        spec.args
            .iter()
            .map(|row| row.trim().to_string())
            .filter(|row| !row.is_empty())
            .collect::<Vec<_>>()
    };

    let bin = ops_binary(root);
    let mut cmd = Command::new(bin);
    cmd.current_dir(root).arg(&plane).arg(&command).args(&args);
    let output = cmd.output();
    let (status, stdout, stderr_raw) = match output {
        Ok(out) => (
            out.status.code().unwrap_or(1),
            String::from_utf8_lossy(&out.stdout).to_string(),
            String::from_utf8_lossy(&out.stderr).to_string(),
        ),
        Err(err) => (1, String::new(), format!("spawn_failed:{err}")),
    };

    let payload = parse_last_json(&stdout);
    let reachable = payload
        .as_ref()
        .and_then(Value::as_object)
        .map(|obj| {
            obj.get("authority").and_then(Value::as_str) == Some("rust_security_plane")
                || obj.get("lane").and_then(Value::as_str) == Some("state_kernel")
        })
        .unwrap_or(false);
    let ok = status == 0 || reachable;
    let stderr = {
        let clean = stderr_raw.trim();
        if clean.is_empty() {
            None
        } else {
            Some(truncate_chars(clean, 400))
        }
    };
    let output_preview = {
        let clean = stdout.trim();
        if clean.is_empty() {
            None
        } else {
            Some(truncate_chars(clean, 400))
        }
    };

    RuntimeCheckResult {
        plane,
        command,
        args,
        ok,
        reachable,
        policy_fail_closed: reachable && status != 0,
        status,
        stderr,
        output_preview,
    }
}

fn render_markdown(ts: &str, ok: bool, hash: &str, summary: &InventorySummary, layers: &[LayerResult]) -> String {
    let mut out = Vec::new();
    out.push("# Security Layer Inventory".to_string());
    out.push(String::new());
    out.push(format!("Generated: {ts}"));
    out.push(String::new());
    out.push(
        "This inventory maps each security layer to enforceable implementation paths, policy contracts, guard-check references, and live runtime checks."
            .to_string(),
    );
    out.push(String::new());
    out.push("| Layer | File/Guard Coverage | Runtime Checks |".to_string());
    out.push("|---|---|---|".to_string());
    for layer in layers {
        let runtime_summary = if layer.runtime_checks.is_empty() {
            "n/a".to_string()
        } else {
            layer
                .runtime_checks
                .iter()
                .map(|check| {
                    format!(
                        "{} {} {}",
                        check.plane,
                        check.command,
                        if check.ok { "ok" } else { "fail" }
                    )
                })
                .collect::<Vec<_>>()
                .join("<br>")
        };
        out.push(format!(
            "| `{}`<br>{} | missing paths: {}<br>missing guard ids: {} | {} |",
            layer.id,
            layer.title,
            layer.missing_paths.len(),
            layer.missing_guard_checks.len(),
            runtime_summary
        ));
    }
    out.push(String::new());
    out.push("## Verification Summary".to_string());
    out.push(String::new());
    out.push(format!("- Layers checked: {}", summary.layers_checked));
    out.push(format!("- Missing paths: {}", summary.missing_paths));
    out.push(format!(
        "- Missing guard checks: {}",
        summary.missing_guard_checks
    ));
    out.push(format!(
        "- Runtime check failures: {}",
        summary.runtime_check_failures
    ));
    out.push(format!(
        "- Contract status: {}",
        if ok { "PASS" } else { "FAIL" }
    ));
    out.push(format!("- Receipt hash: `{hash}`"));
    out.push(String::new());
    out.join("\n")
}

fn write_outputs(
    latest_path: &Path,
    history_path: &Path,
    doc_path: &Path,
    report_path: &Path,
    receipt: &Value,
    markdown: &str,
) -> Result<(), String> {
    lane_utils::write_json(latest_path, receipt)?;
    lane_utils::append_jsonl(history_path, receipt)?;
    lane_utils::ensure_parent(doc_path)?;
    lane_utils::ensure_parent(report_path)?;
    fs::write(doc_path, markdown).map_err(|err| format!("write_doc_failed:{err}"))?;
    fs::write(report_path, markdown).map_err(|err| format!("write_report_failed:{err}"))?;
    Ok(())
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let mode = parse_mode(argv);
    let strict = lane_utils::parse_bool(lane_utils::parse_flag(argv, "strict", true).as_deref(), false);
    let write = lane_utils::parse_bool(lane_utils::parse_flag(argv, "write", true).as_deref(), false);

    let inventory_path = root.join(INVENTORY_CONFIG_REL);
    let guard_registry_path = root.join(GUARD_REGISTRY_REL);
    let latest_path = root.join(LATEST_REL);
    let history_path = root.join(HISTORY_REL);
    let doc_path = root.join(DOC_REL);
    let report_path = root.join(REPORT_REL);
    let _state_dir = root.join(STATE_DIR_REL);

    if mode == "status" {
        let status_payload = lane_utils::read_json(&latest_path).unwrap_or_else(|| {
            json!({
                "ok": false,
                "type": "security_layer_inventory_gate",
                "error": "latest_receipt_missing",
                "latest_path": lane_utils::rel_path(root, &latest_path),
            })
        });
        let ok = status_payload
            .get("ok")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        print_json(&status_payload);
        return if strict && !ok { 2 } else { 0 };
    }

    if mode != "run" {
        usage();
        let payload = json!({
            "ok": false,
            "type": "security_layer_inventory_gate",
            "error": "unknown_command",
            "command": mode,
        });
        print_json(&payload);
        return 1;
    }

    let config = match read_json::<InventoryConfig>(&inventory_path, "inventory_config") {
        Ok(value) => value,
        Err(err) => {
            let payload = json!({
                "ok": false,
                "type": "security_layer_inventory_gate",
                "error": err,
                "config_path": lane_utils::rel_path(root, &inventory_path),
            });
            print_json(&payload);
            return 1;
        }
    };
    let registry = match read_json::<GuardRegistry>(&guard_registry_path, "guard_registry") {
        Ok(value) => value,
        Err(err) => {
            let payload = json!({
                "ok": false,
                "type": "security_layer_inventory_gate",
                "error": err,
                "guard_registry_path": lane_utils::rel_path(root, &guard_registry_path),
            });
            print_json(&payload);
            return 1;
        }
    };

    let guard_index = registry
        .merge_guard
        .checks
        .iter()
        .map(|check| check.id.trim())
        .filter(|id| !id.is_empty())
        .map(|id| id.to_string())
        .collect::<BTreeSet<_>>();

    let mut runtime_cache = HashMap::<String, RuntimeCheckResult>::new();
    let mut layers = Vec::<LayerResult>::new();
    for layer in &config.layers {
        let missing_paths = layer
            .implementation_paths
            .iter()
            .chain(layer.policy_paths.iter())
            .chain(layer.test_paths.iter())
            .filter(|rel| !root.join(rel.as_str()).exists())
            .map(|rel| MissingPath { path: rel.clone() })
            .collect::<Vec<_>>();
        let missing_guard_checks = layer
            .guard_check_ids
            .iter()
            .map(|id| id.trim().to_string())
            .filter(|id| !id.is_empty() && !guard_index.contains(id))
            .map(|id| MissingGuardCheck { id })
            .collect::<Vec<_>>();
        let runtime_checks = layer
            .runtime_checks
            .iter()
            .map(|spec| {
                let key = serde_json::to_string(spec).unwrap_or_else(|_| "runtime_check".to_string());
                if let Some(existing) = runtime_cache.get(&key) {
                    return existing.clone();
                }
                let computed = run_runtime_check(root, spec);
                runtime_cache.insert(key, computed.clone());
                computed
            })
            .collect::<Vec<_>>();

        layers.push(LayerResult {
            id: layer.id.clone(),
            title: layer.title.clone(),
            implementation_paths: layer.implementation_paths.clone(),
            policy_paths: layer.policy_paths.clone(),
            test_paths: layer.test_paths.clone(),
            guard_check_ids: layer.guard_check_ids.clone(),
            runtime_checks,
            missing_paths,
            missing_guard_checks,
        });
    }

    let summary = InventorySummary {
        layers_checked: layers.len(),
        missing_paths: layers.iter().map(|row| row.missing_paths.len()).sum(),
        missing_guard_checks: layers.iter().map(|row| row.missing_guard_checks.len()).sum(),
        runtime_check_failures: layers
            .iter()
            .map(|row| row.runtime_checks.iter().filter(|check| !check.ok).count())
            .sum(),
    };
    let ok = summary.missing_paths == 0
        && summary.missing_guard_checks == 0
        && summary.runtime_check_failures == 0;
    let ts = now_iso();
    let receipt_base = json!({
        "ok": ok,
        "type": "security_layer_inventory_gate",
        "ts": ts,
        "config_path": lane_utils::rel_path(root, &inventory_path),
        "guard_registry_path": lane_utils::rel_path(root, &guard_registry_path),
        "latest_path": lane_utils::rel_path(root, &latest_path),
        "doc_path": lane_utils::rel_path(root, &doc_path),
        "report_path": lane_utils::rel_path(root, &report_path),
        "summary": summary,
        "layers": layers,
    });
    let receipt_hash = sha256_value(&receipt_base);
    let mut receipt = receipt_base;
    receipt["receipt_hash"] = Value::String(receipt_hash.clone());
    let markdown = render_markdown(&ts, ok, &receipt_hash, &summary, &layers);

    if write {
        if let Err(err) = write_outputs(
            &latest_path,
            &history_path,
            &doc_path,
            &report_path,
            &receipt,
            &markdown,
        ) {
            let payload = json!({
                "ok": false,
                "type": "security_layer_inventory_gate",
                "error": err,
            });
            print_json(&payload);
            return 1;
        }
    }

    print_json(&receipt);
    if strict && !ok {
        2
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_last_json_recovers_from_log_prefix() {
        let raw = "trace line\n{\"ok\":true,\"type\":\"x\"}\n";
        let parsed = parse_last_json(raw).expect("json payload");
        assert_eq!(parsed.get("ok").and_then(Value::as_bool), Some(true));
    }

    #[test]
    fn render_markdown_reports_summary() {
        let summary = InventorySummary {
            layers_checked: 2,
            missing_paths: 1,
            missing_guard_checks: 0,
            runtime_check_failures: 1,
        };
        let markdown = render_markdown("2026-03-27T00:00:00Z", false, "abc", &summary, &[]);
        assert!(markdown.contains("Layers checked: 2"));
        assert!(markdown.contains("Contract status: FAIL"));
        assert!(markdown.contains("`abc`"));
    }
}
