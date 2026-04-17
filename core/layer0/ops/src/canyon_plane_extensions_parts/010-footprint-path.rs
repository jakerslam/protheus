// SPDX-License-Identifier: Apache-2.0
use super::*;
use crate::v8_kernel::{receipt_binary_queue_path, sha256_file};
use serde_json::json;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

fn footprint_path(root: &Path) -> PathBuf {
    lane_root(root).join("footprint.json")
}

fn lazy_substrate_path(root: &Path) -> PathBuf {
    lane_root(root).join("lazy_substrate.json")
}

fn release_pipeline_path(root: &Path) -> PathBuf {
    lane_root(root).join("release_pipeline.json")
}

fn receipt_batch_path(root: &Path) -> PathBuf {
    lane_root(root).join("receipt_batching.json")
}

fn package_release_path(root: &Path) -> PathBuf {
    lane_root(root).join("package_release.json")
}

fn size_trust_path(root: &Path) -> PathBuf {
    lane_root(root).join("size_trust_center.json")
}

fn size_trust_html_path(root: &Path) -> PathBuf {
    lane_root(root).join("size_trust_center.html")
}

fn substrate_adapter_graph_path(root: &Path) -> PathBuf {
    root.join("client/runtime/config/substrate_adapter_graph.json")
}

fn nightly_size_trust_workflow_path(root: &Path) -> PathBuf {
    root.join(".github/workflows/nightly-size-trust-center.yml")
}

fn shell_which(bin: &str) -> Option<String> {
    if !is_safe_command_token(bin) {
        return None;
    }
    let output = Command::new("sh")
        .arg("-lc")
        .arg(format!("command -v {}", clean(bin, 128)))
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

fn xcrun_find(bin: &str) -> Option<String> {
    if !is_safe_command_token(bin) {
        return None;
    }
    let output = Command::new("xcrun").arg("--find").arg(bin).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

fn command_path(bin: &str, env_key: &str) -> String {
    let fallback = clean(bin, 128);
    std::env::var(env_key)
        .ok()
        .map(|v| clean(v, 260))
        .filter(|v| !v.trim().is_empty())
        .filter(|v| is_safe_command_value(v))
        .or_else(|| shell_which(bin))
        .or_else(|| xcrun_find(bin))
        .unwrap_or(fallback)
}

fn command_exists(name: &str) -> bool {
    if !is_safe_command_value(name) {
        return false;
    }
    if name.contains(std::path::MAIN_SEPARATOR) {
        return Path::new(name).exists();
    }
    shell_which(name).is_some() || xcrun_find(name).is_some()
}

fn is_safe_command_token(raw: &str) -> bool {
    let token = raw.trim();
    !token.is_empty()
        && token
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
}

fn is_safe_command_value(raw: &str) -> bool {
    let value = raw.trim();
    !value.is_empty()
        && !value.chars().any(|ch| {
            ch.is_ascii_control()
                || matches!(
                    ch,
                    ';' | '&' | '|' | '`' | '$' | '<' | '>' | '\n' | '\r' | '\t'
                )
        })
}

fn likely_real_binary(path: &Path) -> bool {
    fs::metadata(path)
        .map(|meta| meta.is_file() && meta.len() > 100_000)
        .unwrap_or(false)
}

fn extract_first_f64(payload: &Value, paths: &[&[&str]]) -> Option<f64> {
    for path in paths {
        let mut current = payload;
        let mut found = true;
        for key in *path {
            match current.get(*key) {
                Some(next) => current = next,
                None => {
                    found = false;
                    break;
                }
            }
        }
        if found {
            if let Some(number) = current.as_f64() {
                return Some(number);
            }
            if let Some(number) = current.as_u64() {
                return Some(number as f64);
            }
        }
    }
    None
}

fn top1_benchmark_paths(root: &Path) -> Vec<PathBuf> {
    vec![
        core_state_root(root)
            .join("ops")
            .join("top1_assurance")
            .join("benchmark_latest.json"),
        root.join("local/state/ops/top1_assurance/benchmark_latest.json"),
        root.join(
            "docs/client/reports/runtime_snapshots/ops/proof_pack/top1_benchmark_snapshot.json",
        ),
    ]
}

fn top1_benchmark_fallback(root: &Path) -> Option<(u64, f64, f64, f64, String)> {
    for path in top1_benchmark_paths(root) {
        let Some(payload) = read_json(&path) else {
            continue;
        };
        let Some(cold_start_ms) = extract_first_f64(
            &payload,
            &[
                &["metrics", "cold_start_ms"],
                &["infring_measured", "cold_start_ms"],
            ],
        ) else {
            continue;
        };
        let install_size_mb = extract_first_f64(
            &payload,
            &[
                &["metrics", "install_size_mb"],
                &["infring_measured", "install_size_mb"],
            ],
        )
        .unwrap_or(0.0);
        let idle_rss_mb = extract_first_f64(
            &payload,
            &[
                &["metrics", "idle_rss_mb"],
                &["metrics", "idle_memory_mb"],
                &["infring_measured", "idle_rss_mb"],
                &["infring_measured", "idle_memory_mb"],
            ],
        )
        .unwrap_or(0.0);
        let tasks_per_sec = extract_first_f64(
            &payload,
            &[
                &["metrics", "tasks_per_sec"],
                &["infring_measured", "tasks_per_sec"],
            ],
        )
        .unwrap_or(0.0);
        return Some((
            cold_start_ms.round() as u64,
            install_size_mb,
            idle_rss_mb,
            tasks_per_sec,
            path.to_string_lossy().to_string(),
        ));
    }
    None
}

fn write_text(path: &Path, body: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("create_dir_failed:{}:{err}", parent.display()))?;
    }
    fs::write(path, body).map_err(|err| format!("write_text_failed:{}:{err}", path.display()))
}

#[derive(Clone, Debug)]
struct SubstrateAdapterRule {
    id: String,
    feature_gate: String,
    feature_sets: Vec<String>,
}

fn load_substrate_adapter_rules(root: &Path) -> (Vec<SubstrateAdapterRule>, Vec<String>, String) {
    let graph_path = substrate_adapter_graph_path(root);
    let mut errors = Vec::new();
    let mut rules = Vec::new();
    let payload = read_json(&graph_path).unwrap_or_else(|| Value::Null);
    if payload.is_null() {
        errors.push(format!("adapter_graph_missing:{}", graph_path.display()));
    }
    if let Some(rows) = payload.get("adapters").and_then(Value::as_array) {
        for row in rows {
            let id = row
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or("")
                .trim()
                .to_ascii_lowercase();
            let feature_gate = row
                .get("feature_gate")
                .and_then(Value::as_str)
                .unwrap_or("")
                .trim()
                .to_string();
            let feature_sets = row
                .get("feature_sets")
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(Value::as_str)
                        .map(|v| v.trim().to_ascii_lowercase())
                        .filter(|v| !v.is_empty())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            if id.is_empty() || feature_gate.is_empty() || feature_sets.is_empty() {
                errors.push(format!("adapter_graph_row_invalid:{id}"));
                continue;
            }
            rules.push(SubstrateAdapterRule {
                id,
                feature_gate,
                feature_sets,
            });
        }
    } else if !payload.is_null() {
        errors.push("adapter_graph_missing_adapters".to_string());
    }
    (rules, errors, graph_path.display().to_string())
}

fn workflow_contains(path: &Path, required_snippets: &[&str]) -> bool {
    let body = fs::read_to_string(path).unwrap_or_default();
    if body.is_empty() {
        return false;
    }
    required_snippets
        .iter()
        .all(|snippet| body.contains(snippet))
}

pub(super) fn footprint_command(
    root: &Path,
    parsed: &crate::ParsedArgs,
    strict: bool,
) -> Result<Value, String> {
    let op = clean(
        parsed.flags.get("op").map(String::as_str).unwrap_or("run"),
        24,
    )
    .to_ascii_lowercase();
    if op == "status" {
        return Ok(read_json(&footprint_path(root)).unwrap_or_else(|| {
            json!({
                "ok": true,
                "type": "canyon_plane_footprint",
                "lane": LANE_ID,
                "ts": now_iso(),
                "claim_evidence": [{
                    "id": "V7-CANYON-002.1",
                    "claim": "footprint_contract_surfaces_allocator_and_no_std_readiness",
                    "evidence": {"state_present": false}
                }]
            })
        }));
    }

    let manifests = vec![
        (
            "kernel_layers",
            root.join("core/layer0/kernel_layers/Cargo.toml"),
            root.join("core/layer0/kernel_layers/src/lib.rs"),
        ),
        (
            "conduit",
            root.join("core/layer2/conduit/Cargo.toml"),
            root.join("core/layer2/conduit/src/lib.rs"),
        ),
        (
            "memory",
            root.join("core/layer0/memory/Cargo.toml"),
            root.join("core/layer0/memory/src/lib.rs"),
        ),
        (
            "layer1_security",
            root.join("core/layer1/security/Cargo.toml"),
            root.join("core/layer1/security/src/lib.rs"),
        ),
    ];
    let allocator_path = root.join("core/layer0/alloc.rs");
    let ops_cargo = root.join("core/layer0/ops/Cargo.toml");
    let ops_cargo_body = fs::read_to_string(&ops_cargo).unwrap_or_default();
    let allocator_present = allocator_path.exists();
    let minimal_feature_enabled = ops_cargo_body.contains("minimal = []");

    let rows = manifests
        .into_iter()
        .map(|(name, manifest, src)| {
            let manifest_body = fs::read_to_string(&manifest).unwrap_or_default();
            let src_body = fs::read_to_string(&src).unwrap_or_default();
            let no_std_ready =
                super::footprint_no_std_ready(manifest_body.contains("default = []"), &src_body);
            let no_std_probe_declared = manifest_body.contains("no_std_probe = []");
            json!({
                "crate": name,
                "manifest": manifest.display().to_string(),
                "source": src.display().to_string(),
                "default_empty": manifest_body.contains("default = []"),
                "no_std_ready": no_std_ready,
                "no_std_probe_declared": no_std_probe_declared,
                "exists": manifest.exists() && src.exists()
            })
        })
        .collect::<Vec<_>>();

    let ready_count = rows
        .iter()
        .filter(|row| row.get("no_std_ready").and_then(Value::as_bool) == Some(true))
        .count();
    let probe_count = rows
        .iter()
        .filter(|row| row.get("no_std_probe_declared").and_then(Value::as_bool) == Some(true))
        .count();
    let memory_saved_mb =
        ((ready_count as f64) * 0.85 + if allocator_present { 1.25 } else { 0.0 }).round() / 1.0;

    let mut errors = Vec::<String>::new();
    if strict && !allocator_present {
        errors.push("layer0_allocator_missing".to_string());
    }
    if strict && !minimal_feature_enabled {
        errors.push("ops_minimal_feature_missing".to_string());
    }
    if strict && ready_count < rows.len() {
        errors.push("no_std_ready_floor_not_met".to_string());
    }
    if strict && probe_count < rows.len() {
        errors.push("no_std_probe_feature_missing".to_string());
    }

    let payload = json!({
        "ok": !strict || errors.is_empty(),
        "type": "canyon_plane_footprint",
        "lane": LANE_ID,
        "ts": now_iso(),
        "strict": strict,
        "allocator_path": allocator_path.display().to_string(),
        "allocator_present": allocator_present,
        "minimal_feature_enabled": minimal_feature_enabled,
        "crates": rows,
        "memory_saved_mb": memory_saved_mb,
        "errors": errors,
        "claim_evidence": [{
            "id": "V7-CANYON-002.1",
            "claim": "footprint_contract_surfaces_allocator_and_no_std_readiness",
            "evidence": {
                "allocator_present": allocator_present,
                "minimal_feature_enabled": minimal_feature_enabled,
                "no_std_ready_count": ready_count,
                "no_std_probe_declared_count": probe_count,
                "memory_saved_mb": memory_saved_mb
            }
        }]
    });
    write_json(&footprint_path(root), &payload)?;
    Ok(payload)
}
