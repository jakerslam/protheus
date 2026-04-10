fn default_safe_apply_targets(control_runtime_root: &Path) -> Vec<PathBuf> {
    vec![
        control_runtime_root.join("control_runtime.json"),
        control_runtime_root.join("agents/main/agent/models.json"),
        control_runtime_root.join("agents/main/agent/routing-policy.json"),
        control_runtime_root.join("agents/main/agent/identity.md"),
    ]
}

fn resolve_safe_apply_target(control_runtime_root: &Path, path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        path
    } else {
        control_runtime_root.join(path)
    }
}

fn safe_apply_targets(control_runtime_root: &Path, payload: &Value) -> Vec<PathBuf> {
    if let Some(rows) = payload.get("targets").and_then(Value::as_array) {
        let selected = rows
            .iter()
            .filter_map(Value::as_str)
            .map(PathBuf::from)
            .map(|path| resolve_safe_apply_target(control_runtime_root, path))
            .collect::<Vec<_>>();
        if !selected.is_empty() {
            return selected;
        }
    }
    default_safe_apply_targets(control_runtime_root)
}

fn safe_apply_backup_path(
    control_runtime_root: &Path,
    backup_dir: &Path,
    target: &Path,
) -> PathBuf {
    if let Ok(rel) = target.strip_prefix(control_runtime_root) {
        return backup_dir.join(rel);
    }
    let file_name = target
        .file_name()
        .and_then(|v| v.to_str())
        .map(|v| clean_text(v, 120))
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "target".to_string());
    let digest = crate::deterministic_receipt_hash(&json!({
        "target": target.to_string_lossy().to_string()
    }));
    backup_dir
        .join("_external")
        .join(format!("{}-{file_name}", &digest[..12]))
}

fn rollback_from_backup(
    control_runtime_root: &Path,
    targets: &[PathBuf],
    backup_dir: &Path,
) -> Result<Vec<String>, String> {
    let mut restored = Vec::<String>::new();
    for target in targets {
        let backup = safe_apply_backup_path(control_runtime_root, backup_dir, target);
        if backup.exists() {
            fs::copy(&backup, target)
                .map_err(|err| format!("safe_apply_rollback_copy_failed:{err}"))?;
            restored.push(target.to_string_lossy().to_string());
        }
    }
    Ok(restored)
}
