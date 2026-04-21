
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

fn attach_migration_summary(
    payload: &mut Value,
    root_migration: &RootContinuityMigration,
    memory_migration: &MemoryMigrationState,
) {
    payload["migrated_root_files"] = json!(root_migration.migrated);
    payload["promoted_root_files"] = json!(root_migration.promoted);
    payload["archived_root_files"] = json!(root_migration.archived);
    payload["archived_assistant_template_files"] =
        json!(root_migration.archived_assistant_template_files);
    payload["archive_dir"] = json!(root_migration.archive_dir);
    payload["migrated_memory_files"] = json!(memory_migration.migrated);
    payload["archived_memory_files"] = json!(memory_migration.archived);
    payload["conflicted_memory_files"] = json!(memory_migration.conflicts);
    payload["duplicate_memory_files"] = json!(memory_migration.duplicates);
    payload["memory_archive_dir"] = json!(memory_migration.archive_dir);
    payload["removed_root_memory_dir"] = json!(memory_migration.removed_root_memory_dir);
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
    let mut payload = json!({
        "ok": missing_templates.is_empty(),
        "type": "local_runtime_partitioner",
        "command": "init",
        "workspace_root": workspace_root,
        "assistant_dir": paths.assistant_dir,
        "generated_files": generated,
        "missing_templates": missing_templates,
    });
    attach_migration_summary(&mut payload, &migrated, &memory_migration);
    Ok(payload)
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
    let mut payload = json!({
        "ok": missing_templates.is_empty(),
        "type": "local_runtime_partitioner",
        "command": "reset",
        "workspace_root": workspace_root,
        "assistant_dir": paths.assistant_dir,
        "assistant_archive_dir": reset_archive,
        "archived_assistant_files": archived_assistant_files,
        "generated_files": generated,
        "missing_templates": missing_templates,
    });
    attach_migration_summary(&mut payload, &migrated, &memory_migration);
    Ok(payload)
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
