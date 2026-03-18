// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

use crate::contract_lane_utils as lane_utils;
use crate::{deterministic_receipt_hash, now_iso};

const CONTINUITY_FILES: &[&str] = &[
    "SOUL.md",
    "USER.md",
    "HEARTBEAT.md",
    "IDENTITY.md",
    "TOOLS.md",
    "MEMORY.md",
];
const ROOT_DEPRECATED_FILES: &[&str] = CONTINUITY_FILES;
const LEGACY_MEMORY_ROOT_FILES: &[&str] = &["MEMORY_INDEX.md", "TAGS_INDEX.md"];
const LEGACY_ROOT_MEMORY_DIR: &str = "memory";
const RESET_CONFIRM: &str = "RESET_LOCAL";

#[derive(Debug, Clone)]
struct WorkspacePaths {
    workspace_root: PathBuf,
    template_dir: PathBuf,
    assistant_dir: PathBuf,
    reports_dir: PathBuf,
    memory_dir: PathBuf,
    private_dir: PathBuf,
    archive_root: PathBuf,
}

#[derive(Debug, Clone, Default)]
struct MemoryMigrationState {
    migrated: Vec<String>,
    archived: Vec<String>,
    conflicts: Vec<String>,
    duplicates: Vec<String>,
    archive_dir: Option<PathBuf>,
    removed_root_memory_dir: bool,
}

#[derive(Debug, Clone, Default)]
struct RootContinuityMigration {
    migrated: Vec<String>,
    promoted: Vec<String>,
    archived: Vec<String>,
    archived_assistant_template_files: Vec<String>,
    archive_dir: Option<PathBuf>,
}

fn iso_stamp() -> String {
    chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string()
}

fn print_json_line(value: &Value) {
    println!(
        "{}",
        serde_json::to_string(value)
            .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode_failed\"}".to_string())
    );
}

fn cli_receipt(kind: &str, payload: Value) -> Value {
    let ts = now_iso();
    let mut out = json!({
        "ok": payload.get("ok").and_then(Value::as_bool).unwrap_or(true),
        "type": kind,
        "ts": ts,
        "date": ts[..10].to_string(),
        "payload": payload,
    });
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    out
}

fn cli_error(kind: &str, error: &str) -> Value {
    let ts = now_iso();
    let mut out = json!({
        "ok": false,
        "type": kind,
        "ts": ts,
        "date": ts[..10].to_string(),
        "error": error,
        "fail_closed": true,
    });
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    out
}

fn ensure_dir(path: &Path) -> Result<(), String> {
    fs::create_dir_all(path).map_err(|err| format!("mkdir_failed:{}:{err}", path.display()))
}

fn copy_file(src: &Path, dst: &Path) -> Result<(), String> {
    if let Some(parent) = dst.parent() {
        ensure_dir(parent)?;
    }
    fs::copy(src, dst)
        .map_err(|err| format!("copy_failed:{}:{}:{err}", src.display(), dst.display()))?;
    Ok(())
}

fn move_file(src: &Path, dst: &Path) -> Result<(), String> {
    if let Some(parent) = dst.parent() {
        ensure_dir(parent)?;
    }
    fs::rename(src, dst)
        .map_err(|err| format!("rename_failed:{}:{}:{err}", src.display(), dst.display()))
}

fn files_equal(left: &Path, right: &Path) -> bool {
    let Ok(left_meta) = fs::metadata(left) else {
        return false;
    };
    let Ok(right_meta) = fs::metadata(right) else {
        return false;
    };
    if !left_meta.is_file() || !right_meta.is_file() || left_meta.len() != right_meta.len() {
        return false;
    }
    match (fs::read(left), fs::read(right)) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

fn list_files_recursive(root_dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::<PathBuf>::new();
    if !root_dir.is_dir() {
        return files;
    }
    for entry in walkdir::WalkDir::new(root_dir)
        .sort_by_file_name()
        .into_iter()
        .filter_map(Result::ok)
    {
        if entry.file_type().is_file() {
            files.push(entry.path().to_path_buf());
        }
    }
    files
}

fn prune_empty_dirs(root_dir: &Path) -> bool {
    if !root_dir.is_dir() {
        return false;
    }
    let mut dirs = walkdir::WalkDir::new(root_dir)
        .sort_by_file_name()
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_dir())
        .map(|entry| entry.into_path())
        .collect::<Vec<_>>();
    dirs.sort_by_key(|path| std::cmp::Reverse(path.components().count()));
    for dir in dirs {
        let _ = fs::remove_dir(&dir);
    }
    !root_dir.exists()
}

fn workspace_paths(workspace_root: &Path) -> WorkspacePaths {
    let local_workspace = workspace_root.join("local").join("workspace");
    WorkspacePaths {
        workspace_root: workspace_root.to_path_buf(),
        template_dir: workspace_root
            .join("docs")
            .join("workspace")
            .join("templates")
            .join("assistant"),
        assistant_dir: local_workspace.join("assistant"),
        reports_dir: local_workspace.join("reports"),
        memory_dir: local_workspace.join("memory"),
        private_dir: local_workspace.join("private"),
        archive_root: local_workspace.join("archive"),
    }
}

fn ensure_local_workspace_structure(paths: &WorkspacePaths) -> Result<(), String> {
    ensure_dir(&paths.assistant_dir)?;
    ensure_dir(&paths.reports_dir)?;
    ensure_dir(&paths.memory_dir)?;
    ensure_dir(&paths.private_dir)?;
    ensure_dir(&paths.archive_root)?;
    Ok(())
}

fn archive_deprecated_root_continuity(
    paths: &WorkspacePaths,
    migrate_missing: bool,
) -> Result<RootContinuityMigration, String> {
    let mut state = RootContinuityMigration::default();
    for name in ROOT_DEPRECATED_FILES {
        let root_path = paths.workspace_root.join(name);
        if !root_path.exists() {
            continue;
        }
        let assistant_path = paths.assistant_dir.join(name);
        if migrate_missing && !assistant_path.exists() {
            move_file(&root_path, &assistant_path)?;
            state.migrated.push((*name).to_string());
            continue;
        }
        let template_path = paths.template_dir.join(name);
        let assistant_is_template = migrate_missing
            && assistant_path.exists()
            && template_path.exists()
            && files_equal(&assistant_path, &template_path);
        if assistant_is_template {
            let archive_dir = state.archive_dir.get_or_insert_with(|| {
                paths
                    .archive_root
                    .join(format!("root-continuity-{}", iso_stamp()))
            });
            ensure_dir(archive_dir)?;
            move_file(
                &assistant_path,
                &archive_dir.join("assistant-template").join(name),
            )?;
            state
                .archived_assistant_template_files
                .push((*name).to_string());
            move_file(&root_path, &assistant_path)?;
            state.promoted.push((*name).to_string());
            continue;
        }
        let archive_dir = state.archive_dir.get_or_insert_with(|| {
            paths
                .archive_root
                .join(format!("root-continuity-{}", iso_stamp()))
        });
        ensure_dir(archive_dir)?;
        move_file(&root_path, &archive_dir.join("root-conflict").join(name))?;
        state.archived.push((*name).to_string());
    }
    Ok(state)
}

fn migrate_legacy_memory_path(
    paths: &WorkspacePaths,
    source_path: &Path,
    source_label: &str,
    destination_rel_path: &str,
    state: &mut MemoryMigrationState,
) -> Result<(), String> {
    if !source_path.exists() {
        return Ok(());
    }
    let destination_path = paths.memory_dir.join(destination_rel_path);
    if !destination_path.exists() {
        move_file(source_path, &destination_path)?;
        state.migrated.push(source_label.to_string());
        return Ok(());
    }
    let archive_dir = state.archive_dir.get_or_insert_with(|| {
        paths
            .archive_root
            .join(format!("root-memory-{}", iso_stamp()))
    });
    ensure_dir(archive_dir)?;
    if files_equal(source_path, &destination_path) {
        move_file(
            source_path,
            &archive_dir.join("duplicate").join(source_label),
        )?;
        state.archived.push(source_label.to_string());
        state.duplicates.push(source_label.to_string());
        return Ok(());
    }
    move_file(
        source_path,
        &archive_dir.join("conflict").join(source_label),
    )?;
    state.archived.push(source_label.to_string());
    state.conflicts.push(source_label.to_string());
    Ok(())
}

fn migrate_legacy_memory(paths: &WorkspacePaths) -> Result<MemoryMigrationState, String> {
    let mut state = MemoryMigrationState::default();
    for name in LEGACY_MEMORY_ROOT_FILES {
        migrate_legacy_memory_path(
            paths,
            &paths.workspace_root.join(name),
            name,
            name,
            &mut state,
        )?;
    }
    let root_memory_dir = paths.workspace_root.join(LEGACY_ROOT_MEMORY_DIR);
    for source_file in list_files_recursive(&root_memory_dir) {
        let rel = source_file
            .strip_prefix(&root_memory_dir)
            .ok()
            .map(|path| path.to_string_lossy().replace('\\', "/"))
            .unwrap_or_else(|| source_file.to_string_lossy().replace('\\', "/"));
        migrate_legacy_memory_path(
            paths,
            &source_file,
            &format!("memory/{rel}"),
            &rel,
            &mut state,
        )?;
    }
    state.removed_root_memory_dir = prune_empty_dirs(&root_memory_dir);
    Ok(state)
}

fn generate_missing_continuity(
    paths: &WorkspacePaths,
) -> Result<(Vec<String>, Vec<String>), String> {
    let mut generated = Vec::<String>::new();
    let mut missing_templates = Vec::<String>::new();
    for name in CONTINUITY_FILES {
        let dst = paths.assistant_dir.join(name);
        if dst.exists() {
            continue;
        }
        let template = paths.template_dir.join(name);
        if !template.exists() {
            missing_templates.push((*name).to_string());
            continue;
        }
        copy_file(&template, &dst)?;
        generated.push((*name).to_string());
    }
    Ok((generated, missing_templates))
}

fn continuity_status_value(workspace_root: &Path) -> Value {
    let paths = workspace_paths(workspace_root);
    let assistant_files = CONTINUITY_FILES
        .iter()
        .map(|name| {
            json!({
                "file": name,
                "exists": paths.assistant_dir.join(name).exists(),
                "template_exists": paths.template_dir.join(name).exists(),
            })
        })
        .collect::<Vec<_>>();
    json!({
        "ok": true,
        "type": "local_runtime_partitioner",
        "command": "status",
        "workspace_root": workspace_root,
        "assistant_dir": paths.assistant_dir,
        "templates_dir": paths.template_dir,
        "assistant_files": assistant_files,
        "deprecated_root_files": ROOT_DEPRECATED_FILES.iter().filter(|name| paths.workspace_root.join(name).exists()).collect::<Vec<_>>(),
        "deprecated_root_memory_files": LEGACY_MEMORY_ROOT_FILES.iter().filter(|name| paths.workspace_root.join(name).exists()).collect::<Vec<_>>(),
        "deprecated_root_memory_dir_exists": paths.workspace_root.join(LEGACY_ROOT_MEMORY_DIR).exists(),
    })
}

fn init_local_runtime_value(workspace_root: &Path) -> Result<Value, String> {
    let paths = workspace_paths(workspace_root);
    ensure_local_workspace_structure(&paths)?;
    let migrated = archive_deprecated_root_continuity(&paths, true)?;
    let memory_migration = migrate_legacy_memory(&paths)?;
    let (generated, missing_templates) = generate_missing_continuity(&paths)?;
    Ok(json!({
        "ok": missing_templates.is_empty(),
        "type": "local_runtime_partitioner",
        "command": "init",
        "workspace_root": workspace_root,
        "assistant_dir": paths.assistant_dir,
        "generated_files": generated,
        "migrated_root_files": migrated.migrated,
        "promoted_root_files": migrated.promoted,
        "archived_root_files": migrated.archived,
        "archived_assistant_template_files": migrated.archived_assistant_template_files,
        "archive_dir": migrated.archive_dir,
        "migrated_memory_files": memory_migration.migrated,
        "archived_memory_files": memory_migration.archived,
        "conflicted_memory_files": memory_migration.conflicts,
        "duplicate_memory_files": memory_migration.duplicates,
        "memory_archive_dir": memory_migration.archive_dir,
        "removed_root_memory_dir": memory_migration.removed_root_memory_dir,
        "missing_templates": missing_templates,
    }))
}

fn reset_local_runtime_value(workspace_root: &Path, confirm: &str) -> Result<Value, String> {
    if confirm != RESET_CONFIRM {
        return Ok(json!({
            "ok": false,
            "type": "local_runtime_partitioner",
            "command": "reset",
            "error": "missing_confirm_reset_local",
            "required_confirm": RESET_CONFIRM,
        }));
    }
    let paths = workspace_paths(workspace_root);
    ensure_local_workspace_structure(&paths)?;
    let reset_archive = paths
        .archive_root
        .join(format!("assistant-reset-{}", iso_stamp()));
    ensure_dir(&reset_archive)?;
    let mut archived_assistant_files = Vec::<String>::new();
    for name in CONTINUITY_FILES {
        let assistant_path = paths.assistant_dir.join(name);
        if !assistant_path.exists() {
            continue;
        }
        move_file(&assistant_path, &reset_archive.join(name))?;
        archived_assistant_files.push((*name).to_string());
    }
    let migrated = archive_deprecated_root_continuity(&paths, false)?;
    let memory_migration = migrate_legacy_memory(&paths)?;
    let (generated, missing_templates) = generate_missing_continuity(&paths)?;
    Ok(json!({
        "ok": missing_templates.is_empty(),
        "type": "local_runtime_partitioner",
        "command": "reset",
        "workspace_root": workspace_root,
        "assistant_dir": paths.assistant_dir,
        "assistant_archive_dir": reset_archive,
        "archived_assistant_files": archived_assistant_files,
        "generated_files": generated,
        "migrated_root_files": migrated.migrated,
        "promoted_root_files": migrated.promoted,
        "archived_root_files": migrated.archived,
        "archived_assistant_template_files": migrated.archived_assistant_template_files,
        "archive_dir": migrated.archive_dir,
        "migrated_memory_files": memory_migration.migrated,
        "archived_memory_files": memory_migration.archived,
        "conflicted_memory_files": memory_migration.conflicts,
        "duplicate_memory_files": memory_migration.duplicates,
        "memory_archive_dir": memory_migration.archive_dir,
        "removed_root_memory_dir": memory_migration.removed_root_memory_dir,
        "missing_templates": missing_templates,
    }))
}

pub fn run(cwd: &Path, argv: &[String]) -> i32 {
    let command = argv
        .first()
        .map(|value| value.to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());
    let workspace_root = lane_utils::parse_flag(argv, "workspace-root", false)
        .map(PathBuf::from)
        .unwrap_or_else(|| cwd.to_path_buf());
    let payload = match command.as_str() {
        "init" => init_local_runtime_value(&workspace_root),
        "reset" => {
            let confirm = lane_utils::parse_flag(argv, "confirm", false).unwrap_or_default();
            reset_local_runtime_value(&workspace_root, &confirm)
        }
        "status" => Ok(continuity_status_value(&workspace_root)),
        _ => Err(format!(
            "local_runtime_partitioner_unknown_command:{command}"
        )),
    };
    match payload {
        Ok(payload) => {
            let ok = payload.get("ok").and_then(Value::as_bool).unwrap_or(true);
            print_json_line(&cli_receipt(
                &format!("local_runtime_partitioner_{}", command.replace('-', "_")),
                payload,
            ));
            if ok {
                0
            } else {
                1
            }
        }
        Err(err) => {
            print_json_line(&cli_error("local_runtime_partitioner_error", &err));
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_file(path: &Path, body: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("mkdir");
        }
        fs::write(path, body).expect("write");
    }

    fn seed_templates(root: &Path) {
        let template_dir = root.join("docs/workspace/templates/assistant");
        for name in CONTINUITY_FILES {
            write_file(&template_dir.join(name), &format!("template:{name}\n"));
        }
    }

    #[test]
    fn init_migrates_root_and_generates_missing() {
        let tmp = tempfile::tempdir().expect("tmp");
        let root = tmp.path();
        seed_templates(root);
        write_file(&root.join("SOUL.md"), "root soul\n");
        write_file(
            &root.join("local/workspace/assistant/MEMORY.md"),
            "existing memory\n",
        );

        let out = init_local_runtime_value(root).expect("init");
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            out.get("migrated_root_files")
                .and_then(Value::as_array)
                .map(|rows| rows.iter().filter_map(Value::as_str).collect::<Vec<_>>()),
            Some(vec!["SOUL.md"])
        );
        assert!(root.join("local/workspace/assistant/SOUL.md").exists());
        assert!(root.join("local/workspace/assistant/USER.md").exists());
        assert!(!root.join("SOUL.md").exists());
        assert!(root.join("local/workspace/reports").exists());
    }

    #[test]
    fn reset_requires_confirm_token() {
        let tmp = tempfile::tempdir().expect("tmp");
        let root = tmp.path();
        seed_templates(root);

        let out = reset_local_runtime_value(root, "").expect("reset");
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(
            out.get("required_confirm").and_then(Value::as_str),
            Some(RESET_CONFIRM)
        );
    }

    #[test]
    fn init_migrates_memory_and_archives_conflicts() {
        let tmp = tempfile::tempdir().expect("tmp");
        let root = tmp.path();
        seed_templates(root);
        write_file(&root.join("memory/2026-03-13.md"), "legacy memory day\n");
        write_file(
            &root.join("memory/heartbeat-state.json"),
            "{\"lastChecks\":{\"email\":1}}\n",
        );
        write_file(&root.join("MEMORY_INDEX.md"), "legacy index\n");
        write_file(
            &root.join("local/workspace/memory/heartbeat-state.json"),
            "{\"lastChecks\":{\"email\":2}}\n",
        );

        let out = init_local_runtime_value(root).expect("init");
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert!(out
            .get("migrated_memory_files")
            .and_then(Value::as_array)
            .map(|rows| rows.iter().filter_map(Value::as_str).collect::<Vec<_>>())
            .unwrap_or_default()
            .contains(&"memory/2026-03-13.md"));
        assert!(out
            .get("migrated_memory_files")
            .and_then(Value::as_array)
            .map(|rows| rows.iter().filter_map(Value::as_str).collect::<Vec<_>>())
            .unwrap_or_default()
            .contains(&"MEMORY_INDEX.md"));
        assert!(out
            .get("conflicted_memory_files")
            .and_then(Value::as_array)
            .map(|rows| rows.iter().filter_map(Value::as_str).collect::<Vec<_>>())
            .unwrap_or_default()
            .contains(&"memory/heartbeat-state.json"));
        assert!(root.join("local/workspace/memory/2026-03-13.md").exists());
        assert!(!root.join("memory").exists());
    }
}
