fn code_engineer_templates_path(root: &Path) -> PathBuf {
    state_root(root)
        .join("code_engineer")
        .join("builders_templates.json")
}
