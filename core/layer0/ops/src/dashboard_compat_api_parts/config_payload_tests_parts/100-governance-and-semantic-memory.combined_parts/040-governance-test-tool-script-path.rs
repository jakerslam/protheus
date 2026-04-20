fn governance_test_tool_script_path(root: &Path) -> PathBuf {
    state_path(
        root,
        "client/runtime/local/state/ui/infring_dashboard/test_tool_script.json",
    )
}

#[test]
