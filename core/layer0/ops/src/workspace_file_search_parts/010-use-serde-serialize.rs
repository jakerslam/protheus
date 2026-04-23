// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)
// Capability intake source: cline/src/services/search/file-search.ts

use serde::Serialize;
use serde_json::{json, Value};
use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Clone, Debug)]
struct WorkspaceSpec {
    name: String,
    path: PathBuf,
}

#[derive(Clone, Debug, Serialize)]
struct SearchItem {
    workspace_name: String,
    path: String,
    item_type: String,
    label: String,
    score_span: Option<usize>,
    score_gaps: Option<usize>,
}

fn usage() {
    println!("workspace-file-search commands:");
    println!(
        "  infring-ops workspace-file-search search [--workspace=<path>] [--workspace-roots-json='[...]'] [--workspace-hint=<name>] [--q=<query>] [--type=file|folder] [--limit=<n>] [--fetch-limit=<n>] [--allow-external=1]"
    );
    println!(
        "  infring-ops workspace-file-search list [--workspace=<path>] [--workspace-roots-json='[...]'] [--workspace-hint=<name>] [--type=file|folder] [--limit=<n>] [--fetch-limit=<n>] [--allow-external=1]"
    );
    println!(
        "  infring-ops workspace-file-search mention [--workspace=<path>] [--workspace-roots-json='[...]'] [--workspace-hint=<name>] [--q=<query>] [--type=file|folder] [--mention-prefix=@] [--allow-external=1]"
    );
    println!("  infring-ops workspace-file-search status");
}

fn positional_query(parsed: &crate::ParsedArgs) -> String {
    parsed
        .positional
        .iter()
        .skip(1)
        .map(|row| crate::clean(row, 200))
        .collect::<Vec<_>>()
        .join(" ")
}

fn print_payload_and_exit(payload: &Value) -> i32 {
    println!(
        "{}",
        serde_json::to_string_pretty(payload)
            .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode_failed\"}".to_string())
    );
    if payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        0
    } else {
        1
    }
}

fn truthy(value: Option<&String>) -> bool {
    value
        .map(|row| crate::clean(row, 32).to_ascii_lowercase())
        .map(|row| matches!(row.as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(false)
}

fn parse_usize_flag(
    parsed: &crate::ParsedArgs,
    key: &str,
    default: usize,
    max_value: usize,
) -> usize {
    let parsed_value = parsed
        .flags
        .get(key)
        .and_then(|raw| crate::clean(raw, 32).parse::<usize>().ok())
        .unwrap_or(default);
    parsed_value.max(1).min(max_value.max(1))
}

fn canonical_dir(path: &Path) -> Result<PathBuf, String> {
    let canonical =
        fs::canonicalize(path).map_err(|err| format!("workspace_resolve_failed:{err}"))?;
    if !canonical.is_dir() {
        return Err(format!("workspace_not_directory:{}", canonical.display()));
    }
    Ok(canonical)
}

fn resolve_workspace_specs(
    root: &Path,
    parsed: &crate::ParsedArgs,
) -> Result<Vec<WorkspaceSpec>, String> {
    let root_canonical = canonical_dir(root)?;
    let allow_external = truthy(parsed.flags.get("allow-external"));
    let workspace_hint = parsed
        .flags
        .get("workspace-hint")
        .map(|raw| crate::clean(raw, 160))
        .unwrap_or_default();

    let mut specs = Vec::<WorkspaceSpec>::new();
    if let Some(raw) = parsed.flags.get("workspace-roots-json") {
        let value: Value = serde_json::from_str(raw)
            .map_err(|err| format!("workspace_roots_json_invalid:{err}"))?;
        let rows = value
            .as_array()
            .ok_or_else(|| "workspace_roots_json_invalid:not_array".to_string())?;
        for row in rows {
            let name = crate::clean(
                row.get("name")
                    .and_then(Value::as_str)
                    .unwrap_or("workspace"),
                160,
            );
            let path_raw = row
                .get("path")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .trim()
                .to_string();
            if path_raw.is_empty() {
                continue;
            }
            if !workspace_hint.is_empty() && !name.eq_ignore_ascii_case(&workspace_hint) {
                continue;
            }
            let candidate = PathBuf::from(path_raw);
            let resolved = if candidate.is_absolute() {
                candidate
            } else {
                root.join(candidate)
            };
            let canonical = canonical_dir(&resolved)?;
            if !allow_external && !canonical.starts_with(&root_canonical) {
                return Err(format!("workspace_outside_root:{}", canonical.display()));
            }
            specs.push(WorkspaceSpec {
                name,
                path: canonical,
            });
        }
    } else {
        let workspace_raw = parsed
            .flags
            .get("workspace")
            .map(|row| row.trim().to_string())
            .unwrap_or_else(|| ".".to_string());
        let workspace_name = parsed
            .flags
            .get("workspace-name")
            .map(|row| crate::clean(row, 160))
            .unwrap_or_else(|| "workspace".to_string());
        if workspace_hint.is_empty() || workspace_hint.eq_ignore_ascii_case(&workspace_name) {
            let candidate = PathBuf::from(workspace_raw);
            let resolved = if candidate.is_absolute() {
                candidate
            } else {
                root.join(candidate)
            };
            let canonical = canonical_dir(&resolved)?;
            if !allow_external && !canonical.starts_with(&root_canonical) {
                return Err(format!("workspace_outside_root:{}", canonical.display()));
            }
            specs.push(WorkspaceSpec {
                name: workspace_name,
                path: canonical,
            });
        }
    }

    if specs.is_empty() {
        return Err("workspace_not_found".to_string());
    }
    Ok(specs)
}

fn normalize_rel_path(raw: &str) -> String {
    crate::clean(raw.replace('\\', "/"), 1200)
        .trim_start_matches("./")
        .to_string()
}

fn ripgrep_install_hint() -> &'static str {
    if cfg!(target_os = "macos") {
        "brew install ripgrep"
    } else if cfg!(target_os = "linux") {
        "sudo apt-get install ripgrep (or use your distro package manager)"
    } else if cfg!(target_os = "windows") {
        "winget install BurntSushi.ripgrep"
    } else {
        "https://github.com/BurntSushi/ripgrep#installation"
    }
}
