// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

use crate::contract_lane_utils as lane_utils;
use crate::now_iso;

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
    crate::contract_lane_utils::print_json_line(value);
}

fn cli_receipt(kind: &str, payload: Value) -> Value {
    crate::contract_lane_utils::cli_receipt(kind, payload)
}

fn cli_error(kind: &str, error: &str) -> Value {
    crate::contract_lane_utils::cli_error(kind, error)
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
