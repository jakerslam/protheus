fn safe_apply_targets(openclaw_root: &Path, payload: &Value) -> Vec<PathBuf> {
    if let Some(rows) = payload.get("targets").and_then(Value::as_array) {
        let selected = rows
            .iter()
            .filter_map(Value::as_str)
            .map(PathBuf::from)
            .map(|path| {
                if path.is_absolute() {
                    path
                } else {
                    openclaw_root.join(path)
                }
            })
            .collect::<Vec<_>>();
        if !selected.is_empty() {
            return selected;
        }
    }
    vec![
        openclaw_root.join("openclaw.json"),
        openclaw_root.join("agents/main/agent/models.json"),
        openclaw_root.join("agents/main/agent/routing-policy.json"),
        openclaw_root.join("agents/main/agent/identity.md"),
    ]
}

fn safe_apply_backup_path(openclaw_root: &Path, backup_dir: &Path, target: &Path) -> PathBuf {
    if let Ok(rel) = target.strip_prefix(openclaw_root) {
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
    openclaw_root: &Path,
    targets: &[PathBuf],
    backup_dir: &Path,
) -> Result<Vec<String>, String> {
    let mut restored = Vec::<String>::new();
    for target in targets {
        let backup = safe_apply_backup_path(openclaw_root, backup_dir, target);
        if backup.exists() {
            fs::copy(&backup, target)
                .map_err(|err| format!("safe_apply_rollback_copy_failed:{err}"))?;
            restored.push(target.to_string_lossy().to_string());
        }
    }
    Ok(restored)
}
