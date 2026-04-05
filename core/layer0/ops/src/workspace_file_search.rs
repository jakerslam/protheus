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
        "  protheus-ops workspace-file-search search [--workspace=<path>] [--workspace-roots-json='[...]'] [--workspace-hint=<name>] [--q=<query>] [--type=file|folder] [--limit=<n>] [--fetch-limit=<n>] [--allow-external=1]"
    );
    println!(
        "  protheus-ops workspace-file-search list [--workspace=<path>] [--workspace-roots-json='[...]'] [--workspace-hint=<name>] [--type=file|folder] [--limit=<n>] [--fetch-limit=<n>] [--allow-external=1]"
    );
    println!(
        "  protheus-ops workspace-file-search mention [--workspace=<path>] [--workspace-roots-json='[...]'] [--workspace-hint=<name>] [--q=<query>] [--type=file|folder] [--mention-prefix=@] [--allow-external=1]"
    );
    println!("  protheus-ops workspace-file-search status");
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

fn collect_workspace_items(
    workspace: &WorkspaceSpec,
    fetch_limit: usize,
) -> Result<Vec<SearchItem>, String> {
    let rg_binary = std::env::var("PROTHEUS_RG_BINARY")
        .ok()
        .map(|row| row.trim().to_string())
        .filter(|row| !row.is_empty())
        .unwrap_or_else(|| "rg".to_string());
    let output = Command::new(&rg_binary)
        .arg("--files")
        .arg("--follow")
        .arg("--hidden")
        .arg("-g")
        .arg("!**/{node_modules,.git,.github,out,dist,__pycache__,.venv,.env,venv,env,.cache,tmp,temp}/**")
        .current_dir(&workspace.path)
        .output()
        .map_err(|err| {
            if err.kind() == std::io::ErrorKind::NotFound {
                format!(
                    "workspace_file_scan_failed:rg_not_found:{}:install_hint={}",
                    rg_binary,
                    ripgrep_install_hint()
                )
            } else {
                format!("workspace_file_scan_failed:{err}")
            }
        })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(format!("workspace_file_scan_failed:{stderr}"));
    }

    let mut out = Vec::<SearchItem>::new();
    let mut dirs = BTreeSet::<String>::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        if out.len() >= fetch_limit {
            break;
        }
        let rel = normalize_rel_path(line);
        if rel.is_empty() {
            continue;
        }
        out.push(SearchItem {
            workspace_name: workspace.name.clone(),
            path: rel.clone(),
            item_type: "file".to_string(),
            label: Path::new(&rel)
                .file_name()
                .and_then(|row| row.to_str())
                .map(|row| crate::clean(row, 240))
                .unwrap_or_else(|| rel.clone()),
            score_span: None,
            score_gaps: None,
        });
        let mut cursor = Path::new(&rel).parent().map(|row| row.to_path_buf());
        while let Some(parent) = cursor {
            if parent.as_os_str().is_empty() || parent == Path::new(".") {
                break;
            }
            let parent_rel = normalize_rel_path(parent.to_string_lossy().as_ref());
            if parent_rel.is_empty() {
                break;
            }
            dirs.insert(parent_rel);
            cursor = parent.parent().map(|row| row.to_path_buf());
        }
    }

    out.extend(dirs.into_iter().map(|dir| {
        SearchItem {
            workspace_name: workspace.name.clone(),
            label: Path::new(&dir)
                .file_name()
                .and_then(|row| row.to_str())
                .map(|row| crate::clean(row, 240))
                .unwrap_or_else(|| dir.clone()),
            path: dir,
            item_type: "folder".to_string(),
            score_span: None,
            score_gaps: None,
        }
    }));
    Ok(out)
}

fn subsequence_gap_score(query: &str, candidate: &str) -> Option<(usize, usize)> {
    let q_chars = query.to_ascii_lowercase().chars().collect::<Vec<_>>();
    if q_chars.is_empty() {
        return Some((0, 0));
    }
    let c_chars = candidate.to_ascii_lowercase().chars().collect::<Vec<_>>();
    let mut positions = Vec::<usize>::with_capacity(q_chars.len());
    let mut cursor = 0usize;
    for q in q_chars {
        let mut found = None;
        while cursor < c_chars.len() {
            if c_chars[cursor] == q {
                found = Some(cursor);
                cursor += 1;
                break;
            }
            cursor += 1;
        }
        match found {
            Some(pos) => positions.push(pos),
            None => return None,
        }
    }
    let mut gaps = 0usize;
    for pair in positions.windows(2) {
        if pair[1] > pair[0] + 1 {
            gaps += 1;
        }
    }
    let span = positions.last().copied().unwrap_or(0) - positions.first().copied().unwrap_or(0) + 1;
    Some((gaps, span))
}

fn item_type_matches(item: &SearchItem, selected: &str) -> bool {
    if selected.is_empty() {
        return true;
    }
    item.item_type.eq_ignore_ascii_case(selected)
}

fn search_items(
    items: &[SearchItem],
    query: &str,
    selected_type: &str,
    limit: usize,
) -> Vec<SearchItem> {
    let query_clean = crate::clean(query, 200).to_ascii_lowercase();
    let mut filtered = Vec::<SearchItem>::new();
    for item in items {
        if !item_type_matches(item, selected_type) {
            continue;
        }
        if query_clean.is_empty() {
            filtered.push(item.clone());
            continue;
        }
        let candidate = format!("{} {} {}", item.label, item.label, item.path);
        if let Some((gaps, span)) = subsequence_gap_score(&query_clean, &candidate) {
            let mut row = item.clone();
            row.score_gaps = Some(gaps);
            row.score_span = Some(span);
            filtered.push(row);
        }
    }

    filtered.sort_by(|a, b| {
        let ag = a.score_gaps.unwrap_or(usize::MAX);
        let bg = b.score_gaps.unwrap_or(usize::MAX);
        let aspan = a.score_span.unwrap_or(usize::MAX);
        let bspan = b.score_span.unwrap_or(usize::MAX);
        match ag.cmp(&bg) {
            Ordering::Equal => match aspan.cmp(&bspan) {
                Ordering::Equal => match a.path.len().cmp(&b.path.len()) {
                    Ordering::Equal => a.path.cmp(&b.path),
                    other => other,
                },
                other => other,
            },
            other => other,
        }
    });
    filtered.truncate(limit);
    filtered
}

fn append_receipt(root: &Path, payload: &Value) {
    let path = root
        .join("core")
        .join("local")
        .join("state")
        .join("ops")
        .join("workspace_file_search_receipts.jsonl");
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = file.write_all(
            format!(
                "{}\n",
                serde_json::to_string(payload).unwrap_or_else(|_| "{}".to_string())
            )
            .as_bytes(),
        );
    }
}

fn run_search(root: &Path, parsed: &crate::ParsedArgs, default_query: &str) -> Value {
    let specs = match resolve_workspace_specs(root, parsed) {
        Ok(rows) => rows,
        Err(error) => return json!({"ok": false, "status": "blocked", "error": error}),
    };
    let selected_type = parsed
        .flags
        .get("type")
        .map(|row| crate::clean(row, 16))
        .unwrap_or_default();
    let query = parsed
        .flags
        .get("q")
        .or_else(|| parsed.flags.get("query"))
        .map(|row| crate::clean(row, 200))
        .unwrap_or_else(|| crate::clean(default_query, 200));
    let limit = parse_usize_flag(parsed, "limit", 20, 5000);
    let fetch_limit = parse_usize_flag(parsed, "fetch-limit", 5000, 20000);

    let mut all_items = Vec::<SearchItem>::new();
    let mut warnings = Vec::<String>::new();
    for workspace in &specs {
        match collect_workspace_items(workspace, fetch_limit) {
            Ok(rows) => all_items.extend(rows),
            Err(error) => warnings.push(format!("{}:{error}", workspace.name)),
        }
    }
    let results = search_items(&all_items, &query, &selected_type, limit);
    let receipt = json!({
        "type": "workspace_file_search_receipt",
        "ts": crate::now_iso(),
        "source": "cline/src/services/search/file-search.ts",
        "query": query,
        "selected_type": selected_type,
        "workspace_count": specs.len(),
        "item_count": all_items.len(),
        "result_count": results.len(),
        "warnings": warnings,
    });
    append_receipt(root, &receipt);
    json!({
        "ok": true,
        "type": "workspace_file_search",
        "source": "cline:file-search",
        "query": query,
        "selected_type": selected_type,
        "workspace_count": specs.len(),
        "results": results,
        "warnings": warnings,
    })
}

fn run_mention(root: &Path, parsed: &crate::ParsedArgs, default_query: &str) -> Value {
    let search_payload = run_search(root, parsed, default_query);
    if !search_payload
        .get("ok")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return json!({
            "ok": false,
            "status": "blocked",
            "type": "workspace_file_search_mention",
            "error": search_payload
                .get("error")
                .and_then(Value::as_str)
                .unwrap_or("workspace_file_search_failed"),
            "source": "cline:file-search-mention",
        });
    }

    let mention_prefix = parsed
        .flags
        .get("mention-prefix")
        .map(|row| crate::clean(row, 8))
        .filter(|row| !row.is_empty())
        .unwrap_or_else(|| "@".to_string());
    let query = search_payload
        .get("query")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let warnings = search_payload
        .get("warnings")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let results = search_payload
        .get("results")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    if let Some(first) = results.first() {
        let path = first
            .get("path")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let mention = format!("{mention_prefix}{path}");
        let receipt = json!({
            "type": "workspace_file_search_mention_receipt",
            "ts": crate::now_iso(),
            "source": "cline/src/utils/file-search.ts",
            "query": query,
            "mention": mention,
            "path": path,
            "workspace_name": first
                .get("workspace_name")
                .and_then(Value::as_str)
                .unwrap_or_default(),
            "result_item_type": first
                .get("item_type")
                .and_then(Value::as_str)
                .unwrap_or_default(),
            "warnings": warnings,
        });
        append_receipt(root, &receipt);
        return json!({
            "ok": true,
            "status": "ok",
            "type": "workspace_file_search_mention",
            "source": "cline:file-search-mention",
            "query": query,
            "mention": mention,
            "path": path,
            "workspace_name": first
                .get("workspace_name")
                .and_then(Value::as_str)
                .unwrap_or_default(),
            "item_type": first
                .get("item_type")
                .and_then(Value::as_str)
                .unwrap_or_default(),
            "warnings": warnings,
        });
    }

    json!({
        "ok": true,
        "status": "no_results",
        "type": "workspace_file_search_mention",
        "source": "cline:file-search-mention",
        "query": query,
        "mention": Value::Null,
        "path": Value::Null,
        "warnings": warnings,
    })
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let parsed = crate::parse_args(argv);
    let command = parsed
        .positional
        .first()
        .map(|row| row.to_ascii_lowercase())
        .unwrap_or_else(|| "search".to_string());
    let payload = match command.as_str() {
        "help" | "--help" | "-h" => {
            usage();
            json!({"ok": true, "type": "workspace_file_search_help"})
        }
        "status" => json!({
            "ok": true,
            "type": "workspace_file_search_status",
            "source": "cline:file-search",
            "ripgrep_binary": std::env::var("PROTHEUS_RG_BINARY").unwrap_or_else(|_| "rg".to_string()),
            "ripgrep_install_hint": ripgrep_install_hint()
        }),
        "list" => run_search(root, &parsed, ""),
        "search" => {
            let default_query = parsed
                .positional
                .iter()
                .skip(1)
                .map(|row| crate::clean(row, 200))
                .collect::<Vec<_>>()
                .join(" ");
            run_search(root, &parsed, &default_query)
        }
        "mention" => {
            let default_query = parsed
                .positional
                .iter()
                .skip(1)
                .map(|row| crate::clean(row, 200))
                .collect::<Vec<_>>()
                .join(" ");
            run_mention(root, &parsed, &default_query)
        }
        _ => {
            json!({"ok": false, "status": "blocked", "error": "workspace_file_search_unknown_command", "command": command})
        }
    };

    println!(
        "{}",
        serde_json::to_string_pretty(&payload)
            .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode_failed\"}".to_string())
    );
    if payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        0
    } else {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};

    fn test_env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn fuzzy_score_prefers_tighter_match() {
        let tight = subsequence_gap_score("abc", "abc file").expect("tight");
        let loose = subsequence_gap_score("abc", "a xx b yy c").expect("loose");
        assert!(tight.0 < loose.0);
    }

    #[test]
    fn workspace_outside_root_is_blocked_by_default() {
        let root =
            std::env::temp_dir().join(format!("workspace_file_search_root_{}", std::process::id()));
        let inside = root.join("inside");
        let outside = std::env::temp_dir().join(format!(
            "workspace_file_search_outside_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&outside);
        fs::create_dir_all(&inside).expect("inside dir");
        fs::create_dir_all(&outside).expect("outside dir");
        let args = crate::parse_args(&[format!("--workspace={}", outside.display())]);
        let result = resolve_workspace_specs(&inside, &args);
        assert!(result.is_err());
        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&outside);
    }

    #[test]
    fn run_search_returns_match_for_workspace_file() {
        let _guard = test_env_lock()
            .lock()
            .expect("workspace_file_search test lock");
        if Command::new("rg").arg("--version").output().is_err() {
            return;
        }
        let root =
            std::env::temp_dir().join(format!("workspace_file_search_run_{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("root dir");
        fs::write(root.join("context-stacks-proof.txt"), "proof").expect("fixture");
        let parsed = crate::parse_args(&[
            format!("--workspace={}", root.display()),
            "--q=context".to_string(),
            "--limit=5".to_string(),
        ]);
        let payload = run_search(&root, &parsed, "");
        assert_eq!(payload.get("ok").and_then(Value::as_bool), Some(true));
        let results = payload
            .get("results")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(!results.is_empty());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn run_search_reports_ripgrep_install_hint_when_missing() {
        let _guard = test_env_lock()
            .lock()
            .expect("workspace_file_search test lock");
        let root = std::env::temp_dir().join(format!(
            "workspace_file_search_missing_rg_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("root dir");
        fs::write(root.join("file.txt"), "fixture").expect("fixture");
        let previous_rg = std::env::var("PROTHEUS_RG_BINARY").ok();
        std::env::set_var(
            "PROTHEUS_RG_BINARY",
            "__missing_rg_binary_for_workspace_file_search__",
        );
        let parsed = crate::parse_args(&[
            format!("--workspace={}", root.display()),
            "--q=file".to_string(),
            "--limit=5".to_string(),
        ]);
        let payload = run_search(&root, &parsed, "");
        let warnings = payload
            .get("warnings")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(
            warnings
                .iter()
                .any(|row| row.as_str().unwrap_or("").contains("rg_not_found")),
            "expected rg_not_found warning with install hint"
        );
        if let Some(prev) = previous_rg {
            std::env::set_var("PROTHEUS_RG_BINARY", prev);
        } else {
            std::env::remove_var("PROTHEUS_RG_BINARY");
        }
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn run_mention_returns_insertable_path() {
        let _guard = test_env_lock()
            .lock()
            .expect("workspace_file_search test lock");
        if Command::new("rg").arg("--version").output().is_err() {
            return;
        }
        let root = std::env::temp_dir().join(format!(
            "workspace_file_search_mention_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("src")).expect("src dir");
        fs::write(root.join("src").join("main.rs"), "fn main() {}").expect("fixture");
        let parsed = crate::parse_args(&[
            format!("--workspace={}", root.display()),
            "--q=main".to_string(),
            "--limit=5".to_string(),
        ]);
        let payload = run_mention(&root, &parsed, "");
        assert_eq!(payload.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(payload.get("status").and_then(Value::as_str), Some("ok"));
        let mention = payload
            .get("mention")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        assert!(mention.starts_with('@'));
        assert!(mention.contains("main.rs"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn run_mention_reports_no_results_state() {
        let _guard = test_env_lock()
            .lock()
            .expect("workspace_file_search test lock");
        if Command::new("rg").arg("--version").output().is_err() {
            return;
        }
        let root = std::env::temp_dir().join(format!(
            "workspace_file_search_mention_no_results_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("root dir");
        fs::write(root.join("alpha.txt"), "fixture").expect("fixture");
        let parsed = crate::parse_args(&[
            format!("--workspace={}", root.display()),
            "--q=zzzzzz".to_string(),
            "--limit=5".to_string(),
        ]);
        let payload = run_mention(&root, &parsed, "");
        assert_eq!(payload.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            payload.get("status").and_then(Value::as_str),
            Some("no_results")
        );
        assert!(payload.get("mention").map(Value::is_null).unwrap_or(false));
        let _ = fs::remove_dir_all(&root);
    }
}
