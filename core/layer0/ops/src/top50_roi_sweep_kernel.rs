// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use serde_json::{json, Value};
use std::cmp::Ordering;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::contract_lane_utils as lane_utils;
use crate::now_iso;
#[path = "top50_roi_sweep_kernel_render.rs"]
mod render;
use render::{summary_payload, write_outputs, QUEUE_JSON_REL};

const DEFAULT_MAX: usize = 200;
const DEFAULT_TARGET_RUST_PERCENT: usize = 60;
const DEFAULT_POLICY_REL: &str = "client/runtime/config/rust_hotpath_inventory_policy.json";

#[derive(Debug, Clone)]
struct Record {
    path: String,
    lines: usize,
    text: String,
}
fn usage() {
    println!("top50-roi-sweep-kernel commands:");
    println!(
        "  protheus-ops top50-roi-sweep-kernel <run|queue|status> [--max=<n>] [--policy=<path>]"
    );
    println!(
        "  protheus-ops top200-roi-sweep-kernel <run|queue|status> [--max=<n>] [--policy=<path>]"
    );
}
fn print_json_line(value: &Value) {
    println!(
        "{}",
        serde_json::to_string(value)
            .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode_failed\"}".to_string())
    );
}
fn parse_usize_flag(value: Option<String>, fallback: usize) -> usize {
    value
        .as_deref()
        .and_then(|raw| raw.trim().parse::<usize>().ok())
        .map(|v| v.max(1))
        .unwrap_or(fallback)
}
fn parse_policy_path(root: &Path, argv: &[String]) -> PathBuf {
    if let Some(raw) = lane_utils::parse_flag(argv, "policy", false) {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            let candidate = PathBuf::from(trimmed);
            if candidate.is_absolute() {
                return candidate;
            }
            return root.join(candidate);
        }
    }
    root.join(DEFAULT_POLICY_REL)
}
fn parse_scan_roots(root: &Path, policy_path: &Path) -> Vec<String> {
    let raw = fs::read_to_string(policy_path).unwrap_or_default();
    if raw.trim().is_empty() {
        return vec!["systems".to_string(), "lib".to_string()];
    }
    let parsed = serde_json::from_str::<Value>(&raw).unwrap_or_else(|_| json!({}));
    let mut roots = parsed
        .get("scan")
        .and_then(|v| v.get("roots"))
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if roots.is_empty() {
        roots.push("systems".to_string());
        roots.push("lib".to_string());
    }
    let _ = root;
    roots
}
fn git_ls_files(root: &Path, patterns: &[&str]) -> Result<Vec<String>, String> {
    let mut cmd = Command::new("git");
    cmd.arg("ls-files");
    for pattern in patterns {
        cmd.arg(pattern);
    }
    let output = cmd
        .current_dir(root)
        .output()
        .map_err(|err| format!("git_ls_files_spawn_failed:{err}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        return Err(format!(
            "git_ls_files_failed:{}",
            if stderr.is_empty() { stdout } else { stderr }
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToString::to_string)
        .collect::<Vec<_>>())
}
fn count_lines_like_js(text: &str) -> usize {
    if text.is_empty() {
        0
    } else {
        text.split('\n').count()
    }
}
fn ext_of(path: &str) -> String {
    Path::new(path)
        .extension()
        .map(|ext| ext.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default()
}
fn read_records(root: &Path, files: &[String]) -> Vec<Record> {
    files
        .iter()
        .filter_map(|rel| {
            let abs = root.join(rel);
            if !abs.exists() {
                return None;
            }
            let text = fs::read_to_string(&abs).unwrap_or_default();
            Some(Record {
                path: rel.to_string(),
                lines: count_lines_like_js(&text),
                text,
            })
        })
        .collect::<Vec<_>>()
}
fn runtime_rel_path(rel_path: &str) -> &str {
    rel_path.strip_prefix("client/runtime/").unwrap_or(rel_path)
}
fn in_scan_roots(rel_path: &str, roots: &[String]) -> bool {
    let runtime = runtime_rel_path(rel_path);
    roots
        .iter()
        .map(|row| row.trim())
        .filter(|row| !row.is_empty())
        .any(|root| runtime == root || runtime.starts_with(&format!("{root}/")))
}
fn has_authority_marker(record: &Record) -> bool {
    let normalized = record.text.as_str();
    normalized.contains("Layer ownership:") && normalized.contains("(authoritative)")
}
fn is_thin_bridge(record: &Record) -> bool {
    if ext_of(&record.path) != "ts" {
        return false;
    }
    let normalized = record.text.as_str();
    normalized.contains("createOpsLaneBridge")
        || normalized.contains("createManifestLaneBridge")
        || normalized.contains("runProtheusOps(args")
        || normalized.contains("runProtheusOps(['")
        || normalized.contains("require('./run_protheus_ops.ts')")
        || normalized.contains("Thin TypeScript wrapper only")
        || normalized.contains("Thin runtime wrapper:")
        || normalized.contains("thin CLI bridge")
        || normalized.contains("compatibility shim only")
        || normalized.contains("authoritative runtime implementation lives in JS")
        || normalized.contains("legacy_retired_wrapper.ts")
        || normalized.contains("bindLegacyRetiredModule(__filename, module)")
        || (normalized.contains("Layer ownership: core/layer0/ops")
            && (normalized.contains("runProtheusOps(")
                || normalized.contains("createOpsLaneBridge(")))
        || (normalized.contains("module.exports")
            && normalized.contains("require('./")
            && (normalized.contains(".ts") || normalized.contains(".js")))
}
fn is_extension_surface(record: &Record) -> bool {
    let rel = record.path.as_str();
    let text = record.text.as_str();
    rel.starts_with("apps/")
        || rel.starts_with("packages/")
        || rel.starts_with("client/cognition/")
        || rel.starts_with("client/cli/bin/")
        || rel.starts_with("client/runtime/platform/")
        || rel.starts_with("adapters/importers/")
        || rel.starts_with("adapters/cognition/collectors/")
        || rel.contains("/skills/")
        || rel.starts_with("client/runtime/patches/")
        || rel.starts_with("client/runtime/systems/extensions/")
        || rel.starts_with("client/runtime/systems/marketplace/")
        || rel.ends_with("_demo.ts")
        || rel.contains("/demo/")
        || text.contains("thin demo shell only")
        || text.contains("optional REPL/demo ergonomics")
        || rel.ends_with(".d.ts")
        || rel.ends_with(".config.ts")
        || rel == "vitest.config.ts"
}
fn is_cognition_orchestration_thin_surface(record: &Record) -> bool {
    if !record.path.starts_with("client/cognition/orchestration/") {
        return false;
    }
    if has_authority_marker(record) {
        return false;
    }
    let text = record.text.as_str();
    text.contains("require('./core_bridge.ts')")
        || text.contains("invokeOrchestration(")
        || text.contains("runTaskGroupCli(")
}
fn route_weight(rel_path: &str) -> f64 {
    let weighted = [
        ("client/runtime/systems/web/", 4.5),
        ("client/runtime/systems/tooling/", 4.4),
        ("client/runtime/systems/search/", 4.2),
        ("client/runtime/systems/gateway/", 4.1),
        ("client/runtime/systems/autonomy/", 4.3),
        ("client/runtime/systems/security/", 4.3),
        ("client/runtime/systems/ops/", 4.3),
        ("client/runtime/systems/memory/", 4.3),
        ("client/runtime/systems/sensory/", 4.3),
        ("client/runtime/systems/assimilation/", 4.3),
        ("client/runtime/systems/routing/", 3.6),
        ("client/runtime/systems/workflow/", 3.4),
        ("client/runtime/systems/spine/", 3.4),
        ("client/runtime/systems/personas/", 3.2),
        ("client/runtime/lib/", 2.8),
        ("client/lib/", 2.2),
        ("adapters/", 2.1),
    ];
    for (prefix, weight) in weighted {
        if rel_path.starts_with(prefix) {
            return weight;
        }
    }
    1.5
}
fn should_exclude(record: &Record) -> bool {
    let rel = record.path.as_str();
    let base = Path::new(rel)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");
    if !rel.ends_with(".ts") {
        return true;
    }
    if rel.starts_with("apps/")
        || rel.starts_with("packages/")
        || rel.starts_with("client/cognition/")
        || rel.starts_with("client/cli/bin/")
        || rel.starts_with("client/runtime/platform/")
        || rel.starts_with("adapters/importers/")
        || rel.starts_with("adapters/cognition/collectors/")
        || rel.starts_with("adapters/cognition/skills/")
    {
        return true;
    }
    if rel.starts_with("tests/") || rel.contains("/tests/") {
        return true;
    }
    if rel.starts_with("client/lib/") && !has_authority_marker(record) {
        return true;
    }
    if (base.contains("benchmark") || rel.contains("/benchmarks/")) && !has_authority_marker(record)
    {
        return true;
    }
    if rel.starts_with("client/runtime/systems/ui/") {
        return true;
    }
    if rel.starts_with("adapters/cognition/skills/") {
        return true;
    }
    if is_extension_surface(record) && !has_authority_marker(record) {
        return true;
    }
    if rel == "client/runtime/lib/moltbook_api.ts"
        || rel == "client/runtime/systems/conduit/conduit-client.ts"
        || rel == "client/lib/conduit_full_lifecycle_probe.ts"
        || rel == "client/runtime/lib/rust_lane_bridge.ts"
        || rel == "client/runtime/lib/spine_conduit_bridge.ts"
        || rel == "client/runtime/systems/workflow/shannon_desktop_shell.ts"
        || rel == "client/runtime/lib/ts_bootstrap.ts"
        || rel == "client/runtime/lib/ts_entrypoint.ts"
        || rel == "client/lib/ts_entrypoint.ts"
        || rel == "client/runtime/lib/exec_compacted.ts"
        || rel == "client/runtime/lib/backlog_lane_cli.ts"
        || rel == "client/runtime/lib/tool_compactor_integration.ts"
        || rel == "client/runtime/lib/command_output_compactor.ts"
        || rel == "client/runtime/lib/eyes_catalog.ts"
        || rel == "client/lib/protheus_suite_tooling.ts"
    {
        return true;
    }
    if is_cognition_orchestration_thin_surface(record) {
        return true;
    }
    if base.ends_with("_bridge.ts") || base.ends_with("_client.ts") || base.ends_with("_cli.ts") {
        return true;
    }
    if rel.contains("/habits/") || rel.contains("/reflexes/") || rel.contains("/eyes/") {
        return true;
    }
    is_thin_bridge(record)
}
fn sort_candidates(a: &Value, b: &Value) -> Ordering {
    let a_impact = a.get("impact_score").and_then(Value::as_f64).unwrap_or(0.0);
    let b_impact = b.get("impact_score").and_then(Value::as_f64).unwrap_or(0.0);
    b_impact
        .partial_cmp(&a_impact)
        .unwrap_or(Ordering::Equal)
        .then_with(|| {
            let a_loc = a.get("loc").and_then(Value::as_u64).unwrap_or(0);
            let b_loc = b.get("loc").and_then(Value::as_u64).unwrap_or(0);
            b_loc.cmp(&a_loc)
        })
        .then_with(|| {
            let a_path = a.get("path").and_then(Value::as_str).unwrap_or("");
            let b_path = b.get("path").and_then(Value::as_str).unwrap_or("");
            a_path.cmp(b_path)
        })
}
fn format_impact(impact: f64) -> f64 {
    (impact * 10.0).round() / 10.0
}
fn build_queue(root: &Path, max: usize, policy_path: &Path) -> Result<Value, String> {
    let ts_files = git_ls_files(root, &["*.ts"])?;
    let records = read_records(root, &ts_files);

    let tracked_code_files = git_ls_files(root, &["*.ts", "*.tsx", "*.js", "*.jsx", "*.rs"])?;
    let code_records = read_records(root, &tracked_code_files);

    let mut tracked_ts_lines = 0usize;
    let mut tracked_js_lines = 0usize;
    let mut tracked_rs_lines = 0usize;
    for record in &code_records {
        match ext_of(&record.path).as_str() {
            "ts" | "tsx" => tracked_ts_lines += record.lines,
            "js" | "jsx" => tracked_js_lines += record.lines,
            "rs" => tracked_rs_lines += record.lines,
            _ => {}
        }
    }

    let roots = parse_scan_roots(root, policy_path);
    let runtime_records = code_records
        .iter()
        .filter(|record| in_scan_roots(&record.path, &roots))
        .cloned()
        .collect::<Vec<_>>();
    let bridge_wrappers_excluded = runtime_records
        .iter()
        .filter(|record| is_thin_bridge(record))
        .count();

    let extension_surfaces_excluded = records
        .iter()
        .filter(|record| {
            record.path.ends_with(".ts")
                && is_extension_surface(record)
                && !has_authority_marker(record)
        })
        .count();

    let mut candidates = records
        .iter()
        .filter(|record| !should_exclude(record))
        .map(|record| {
            let weight = route_weight(&record.path);
            json!({
                "path": record.path,
                "loc": record.lines,
                "weight": weight,
                "impact_score": format_impact(record.lines as f64 * weight),
            })
        })
        .collect::<Vec<_>>();

    candidates.sort_by(sort_candidates);

    let rust_percent = ((tracked_rs_lines as f64)
        / ((tracked_rs_lines + tracked_ts_lines + tracked_js_lines).max(1) as f64)
        * 100.0
        * 100.0)
        .round()
        / 100.0;

    let mut lanes = Vec::<Value>::with_capacity(candidates.len());
    let mut cumulative = 0usize;
    for (idx, candidate) in candidates.iter().enumerate() {
        let loc = candidate.get("loc").and_then(Value::as_u64).unwrap_or(0) as usize;
        cumulative += loc;
        let projected = ((tracked_rs_lines + cumulative) as f64
            / (tracked_rs_lines + tracked_ts_lines).max(1) as f64)
            * 100.0;
        lanes.push(json!({
            "lane_id": format!("R60-{:04}", idx + 1),
            "rank": idx + 1,
            "path": candidate.get("path").and_then(Value::as_str).unwrap_or(""),
            "loc": loc,
            "weight": candidate.get("weight").and_then(Value::as_f64).unwrap_or(0.0),
            "impact_score": candidate.get("impact_score").and_then(Value::as_f64).unwrap_or(0.0),
            "cumulative_migrated_ts_lines": cumulative,
            "projected_rust_percent_after_lane": ((projected * 1000.0).round() / 1000.0),
            "status": "queued"
        }));
    }

    let top = lanes.iter().take(max).cloned().collect::<Vec<_>>();

    Ok(json!({
        "ok": true,
        "type": "roi_top_queue",
        "ts": now_iso(),
        "target_rust_percent": DEFAULT_TARGET_RUST_PERCENT,
        "rust_percent": rust_percent,
        "current_rust_percent": rust_percent,
        "target_already_met": rust_percent >= DEFAULT_TARGET_RUST_PERCENT as f64,
        "queue_size": lanes.len(),
        "bridge_wrappers_excluded": bridge_wrappers_excluded,
        "extension_surfaces_excluded": extension_surfaces_excluded,
        "stale_reference_repair": true,
        "queue": lanes,
        "lanes": lanes,
        "top_candidates": top,
        "top": top,
    }))
}
pub fn run(root: &Path, argv: &[String]) -> i32 {
    let cmd = argv
        .iter()
        .find(|token| !token.trim_start().starts_with("--"))
        .map(|token| token.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "run".to_string());

    if !matches!(cmd.as_str(), "run" | "queue" | "status") {
        usage();
        let payload = json!({
            "ok": false,
            "type": "top50_roi_sweep",
            "error": "unknown_command",
            "command": cmd,
        });
        print_json_line(&payload);
        return 2;
    }

    let max = parse_usize_flag(lane_utils::parse_flag(argv, "max", false), DEFAULT_MAX);
    let policy_path = parse_policy_path(root, argv);

    if cmd == "status" {
        let queue_json = root.join(QUEUE_JSON_REL);
        if queue_json.exists() {
            let raw = fs::read_to_string(queue_json).unwrap_or_default();
            let trimmed = raw.trim();
            if !trimmed.is_empty() {
                println!("{trimmed}");
                return 0;
            }
        }
    }

    let queue = match build_queue(root, max, &policy_path) {
        Ok(value) => value,
        Err(err) => {
            let payload = json!({
                "ok": false,
                "type": "top50_roi_sweep",
                "error": err,
            });
            print_json_line(&payload);
            return 1;
        }
    };

    if cmd != "queue" {
        if let Err(err) = write_outputs(root, &queue) {
            let payload = json!({
                "ok": false,
                "type": "top50_roi_sweep",
                "error": err,
            });
            print_json_line(&payload);
            return 1;
        }
        print_json_line(&summary_payload(&queue));
        return 0;
    }

    print_json_line(&queue);
    0
}
