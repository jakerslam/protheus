// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use serde::Deserialize;
use serde_json::{json, Value};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::contract_lane_utils as lane_utils;
use crate::now_iso;

const DEFAULT_POLICY_REL: &str = "client/runtime/config/rust_hotpath_inventory_policy.json";

#[derive(Debug, Clone, Deserialize, Default)]
struct Policy {
    #[serde(default)]
    paths: PolicyPaths,
    #[serde(default)]
    scan: PolicyScan,
    #[serde(default)]
    report: PolicyReport,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct PolicyPaths {
    #[serde(default = "default_latest_path")]
    latest_path: String,
    #[serde(default = "default_history_path")]
    history_path: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct PolicyScan {
    #[serde(default)]
    roots: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct PolicyReport {
    #[serde(default = "default_top_directories")]
    top_directories: usize,
    #[serde(default = "default_top_files")]
    top_files: usize,
    #[serde(default = "default_milestones")]
    milestones: Vec<f64>,
}

#[derive(Debug, Clone)]
struct Record {
    path: String,
    lines: usize,
    ext: String,
    text: String,
}

fn default_latest_path() -> String {
    "local/state/ops/rust_hotpath_inventory/latest.json".to_string()
}

fn default_history_path() -> String {
    "local/state/ops/rust_hotpath_inventory/history.jsonl".to_string()
}

fn default_top_directories() -> usize {
    15
}

fn default_top_files() -> usize {
    30
}

fn default_milestones() -> Vec<f64> {
    vec![15.0, 25.0, 35.0, 50.0]
}

fn usage() {
    println!("rust-hotpath-inventory-kernel commands:");
    println!("  protheus-ops rust-hotpath-inventory-kernel <run|status|inventory> [--policy=<path>]");
}

fn print_json_line(value: &Value) {
    println!(
        "{}",
        serde_json::to_string(value)
            .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode_failed\"}".to_string())
    );
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

fn read_policy(path: &Path) -> Result<Policy, String> {
    let raw = fs::read_to_string(path).map_err(|err| format!("policy_read_failed:{err}"))?;
    serde_json::from_str::<Policy>(&raw).map_err(|err| format!("policy_decode_failed:{err}"))
}

fn git_tracked_files(root: &Path) -> Result<Vec<String>, String> {
    let output = Command::new("git")
        .arg("ls-files")
        .arg("*.ts")
        .arg("*.tsx")
        .arg("*.js")
        .arg("*.jsx")
        .arg("*.rs")
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

fn is_ts_ext(ext: &str) -> bool {
    ext == "ts" || ext == "tsx"
}

fn is_js_ext(ext: &str) -> bool {
    ext == "js" || ext == "jsx"
}

fn runtime_rel_path(rel_path: &str) -> &str {
    rel_path
        .strip_prefix("client/runtime/")
        .unwrap_or(rel_path)
}

fn in_scan_roots(rel_path: &str, roots: &[String]) -> bool {
    let runtime = runtime_rel_path(rel_path);
    roots.iter()
        .map(|row| row.trim())
        .filter(|row| !row.is_empty())
        .any(|root| runtime == root || runtime.starts_with(&format!("{root}/")))
}

fn dirname_posix(rel_path: &str) -> String {
    rel_path
        .rfind('/')
        .map(|idx| rel_path[..idx].to_string())
        .unwrap_or_else(|| ".".to_string())
}

fn sort_path_then_lines_desc(a_lines: usize, a_path: &str, b_lines: usize, b_path: &str) -> Ordering {
    b_lines
        .cmp(&a_lines)
        .then_with(|| a_path.cmp(b_path))
}

fn is_thin_bridge(path: &str, ext: &str, text: &str) -> bool {
    if ext != "ts" {
        return false;
    }
    let normalized = text;
    normalized.contains("createOpsLaneBridge")
        || normalized.contains("createManifestLaneBridge")
        || normalized.contains("runProtheusOps(args")
        || normalized.contains("runProtheusOps(['")
        || normalized.contains("require('./run_protheus_ops.ts')")
        || normalized.contains("Thin TypeScript wrapper only")
        || normalized.contains("Thin runtime wrapper:")
        || normalized.contains("thin CLI bridge")
        || normalized.contains("compatibility shim only")
        || (normalized.contains("Layer ownership: core/layer0/ops")
            && (normalized.contains("runProtheusOps(")
                || normalized.contains("createOpsLaneBridge(")))
        || (path.ends_with(".ts")
            && normalized.contains("module.exports")
            && normalized.contains("require('./")
            && normalized.contains(".ts"))
}

fn rust_milestones(
    tracked_rs_lines: usize,
    tracked_non_rust_code_lines: usize,
    milestones: &[f64],
) -> Vec<Value> {
    let total = tracked_rs_lines + tracked_non_rust_code_lines;
    milestones
        .iter()
        .map(|target| {
            let needed = (((target / 100.0) * total as f64) - tracked_rs_lines as f64).ceil();
            json!({
                "target_percent": *target,
                "additional_rs_lines_needed": if needed.is_sign_positive() { needed as i64 } else { 0i64 }
            })
        })
        .collect::<Vec<_>>()
}

fn build_inventory(root: &Path, policy_path: &Path, policy: &Policy) -> Result<Value, String> {
    let tracked_files = git_tracked_files(root)?;
    let mut tracked_ts_lines = 0usize;
    let mut tracked_js_lines = 0usize;
    let mut tracked_rs_lines = 0usize;
    let mut records = Vec::<Record>::new();

    for rel_path in tracked_files {
        let abs_path = root.join(&rel_path);
        if !abs_path.exists() {
            continue;
        }
        let text = fs::read_to_string(&abs_path).unwrap_or_default();
        let lines = count_lines_like_js(&text);
        let ext = ext_of(&rel_path);
        if is_ts_ext(&ext) {
            tracked_ts_lines += lines;
        }
        if is_js_ext(&ext) {
            tracked_js_lines += lines;
        }
        if ext == "rs" {
            tracked_rs_lines += lines;
        }
        records.push(Record {
            path: rel_path,
            lines,
            ext,
            text,
        });
    }

    let tracked_non_rust_code_lines = tracked_ts_lines + tracked_js_lines;
    let runtime_records = records
        .iter()
        .filter(|record| in_scan_roots(&record.path, &policy.scan.roots))
        .cloned()
        .collect::<Vec<_>>();
    let runtime_code_records = runtime_records
        .iter()
        .filter(|record| is_ts_ext(&record.ext) || is_js_ext(&record.ext))
        .cloned()
        .collect::<Vec<_>>();

    let mut dir_buckets = HashMap::<String, usize>::new();
    for record in &runtime_code_records {
        let dir = dirname_posix(&record.path);
        *dir_buckets.entry(dir).or_insert(0) += record.lines;
    }
    let mut top_directories = dir_buckets
        .iter()
        .map(|(dir, lines)| json!({"path": dir, "lines": *lines }))
        .collect::<Vec<_>>();
    top_directories.sort_by(|a, b| {
        let a_lines = a.get("lines").and_then(Value::as_u64).unwrap_or(0) as usize;
        let b_lines = b.get("lines").and_then(Value::as_u64).unwrap_or(0) as usize;
        let a_path = a.get("path").and_then(Value::as_str).unwrap_or("");
        let b_path = b.get("path").and_then(Value::as_str).unwrap_or("");
        sort_path_then_lines_desc(a_lines, a_path, b_lines, b_path)
    });
    top_directories.truncate(policy.report.top_directories.max(1));

    let mut top_files = runtime_code_records
        .iter()
        .map(|record| json!({"path": record.path, "lines": record.lines}))
        .collect::<Vec<_>>();
    top_files.sort_by(|a, b| {
        let a_lines = a.get("lines").and_then(Value::as_u64).unwrap_or(0) as usize;
        let b_lines = b.get("lines").and_then(Value::as_u64).unwrap_or(0) as usize;
        let a_path = a.get("path").and_then(Value::as_str).unwrap_or("");
        let b_path = b.get("path").and_then(Value::as_str).unwrap_or("");
        sort_path_then_lines_desc(a_lines, a_path, b_lines, b_path)
    });
    top_files.truncate(policy.report.top_files.max(1));

    let bridge_wrappers_excluded_from_queue = runtime_records
        .iter()
        .filter(|record| is_thin_bridge(&record.path, &record.ext, &record.text))
        .count();

    let rust_percent_code_scope = ((tracked_rs_lines as f64)
        / ((tracked_rs_lines + tracked_non_rust_code_lines).max(1) as f64)
        * 100.0 * 100.0)
        .round()
        / 100.0;
    let rust_percent_ts_scope = ((tracked_rs_lines as f64)
        / ((tracked_rs_lines + tracked_ts_lines).max(1) as f64)
        * 100.0 * 100.0)
        .round()
        / 100.0;

    Ok(json!({
        "ok": true,
        "type": "rust_hotpath_inventory",
        "ts": now_iso(),
        "policy_path": lane_utils::rel_path(root, policy_path),
        "tracked_ts_lines": tracked_ts_lines,
        "tracked_js_lines": tracked_js_lines,
        "tracked_rs_lines": tracked_rs_lines,
        "tracked_non_rust_code_lines": tracked_non_rust_code_lines,
        "rust_percent": rust_percent_code_scope,
        "rust_percent_ts_scope": rust_percent_ts_scope,
        "rust_percent_code_scope": rust_percent_code_scope,
        "runtime_scope": {
            "roots": policy.scan.roots,
            "ts_files": runtime_records.iter().filter(|record| is_ts_ext(&record.ext)).count(),
            "js_files": runtime_records.iter().filter(|record| is_js_ext(&record.ext)).count(),
            "rs_files": runtime_records.iter().filter(|record| record.ext == "rs").count(),
            "bridge_wrappers_excluded_from_queue": bridge_wrappers_excluded_from_queue
        },
        "top_directories": top_directories,
        "top_files": top_files,
        "milestones": rust_milestones(tracked_rs_lines, tracked_non_rust_code_lines, &policy.report.milestones)
    }))
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let command = argv
        .iter()
        .find(|token| !token.trim_start().starts_with("--"))
        .map(|token| token.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());

    if !matches!(command.as_str(), "run" | "status" | "inventory") {
        usage();
        let payload = json!({
            "ok": false,
            "type": "rust_hotpath_inventory",
            "error": "unknown_command",
            "command": command,
        });
        print_json_line(&payload);
        return 2;
    }

    let policy_path = parse_policy_path(root, argv);
    let policy = match read_policy(&policy_path) {
        Ok(value) => value,
        Err(err) => {
            let payload = json!({
                "ok": false,
                "type": "rust_hotpath_inventory",
                "error": err,
                "policy_path": lane_utils::rel_path(root, &policy_path),
            });
            print_json_line(&payload);
            return 1;
        }
    };
    let latest_path = root.join(policy.paths.latest_path.trim());
    let history_path = root.join(policy.paths.history_path.trim());

    if command == "status" && latest_path.exists() {
        let raw = fs::read_to_string(&latest_path).unwrap_or_default();
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            print_json_line(&json!({
                "ok": false,
                "type": "rust_hotpath_inventory",
                "error": "latest_receipt_empty",
                "latest_path": lane_utils::rel_path(root, &latest_path),
            }));
        } else {
            println!("{trimmed}");
        }
        return 0;
    }

    let payload = match build_inventory(root, &policy_path, &policy) {
        Ok(value) => value,
        Err(err) => {
            let out = json!({
                "ok": false,
                "type": "rust_hotpath_inventory",
                "error": err,
            });
            print_json_line(&out);
            return 1;
        }
    };

    if command == "run" {
        if let Err(err) = lane_utils::write_json(&latest_path, &payload) {
            let out = json!({
                "ok": false,
                "type": "rust_hotpath_inventory",
                "error": format!("write_latest_failed:{err}"),
            });
            print_json_line(&out);
            return 1;
        }
        if let Err(err) = lane_utils::append_jsonl(&history_path, &payload) {
            let out = json!({
                "ok": false,
                "type": "rust_hotpath_inventory",
                "error": format!("append_history_failed:{err}"),
            });
            print_json_line(&out);
            return 1;
        }
    }

    print_json_line(&payload);
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn count_lines_matches_js_split_behavior() {
        assert_eq!(count_lines_like_js(""), 0);
        assert_eq!(count_lines_like_js("a"), 1);
        assert_eq!(count_lines_like_js("a\n"), 2);
        assert_eq!(count_lines_like_js("a\nb"), 2);
    }

    #[test]
    fn in_scan_roots_handles_runtime_prefix() {
        let roots = vec!["systems".to_string(), "lib".to_string()];
        assert!(in_scan_roots("client/runtime/systems/ui/infring_dashboard.ts", &roots));
        assert!(in_scan_roots("client/runtime/lib/rust_lane_bridge.ts", &roots));
        assert!(!in_scan_roots("client/cognition/foo.ts", &roots));
    }
}
