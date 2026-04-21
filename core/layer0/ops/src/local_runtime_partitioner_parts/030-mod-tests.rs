
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
