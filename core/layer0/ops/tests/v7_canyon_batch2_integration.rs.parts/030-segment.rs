#[test]
fn v7_canyon_release_pipeline_strict_fails_when_optional_llvm_tools_are_missing() {
    let _guard = test_env_lock();
    let tmp = temp_root("canyon_batch2_missing_llvm_strict");
    let root = tmp.path();
    let canyon_state = root.join("local").join("state").join("canyon");
    std::env::set_var(ENV_KEY, &canyon_state);

    write_text(
        root,
        "core/layer0/ops/Cargo.toml",
        "[package]\nname='protheus-ops-core'\n[features]\nminimal = []\n",
    );
    let toolbin = install_tool_stubs(root);
    std::env::set_var("PROTHEUS_CARGO_BIN", toolbin.join("cargo"));
    std::env::set_var("PROTHEUS_STRIP_BIN", toolbin.join("strip"));
    std::env::set_var(
        "PROTHEUS_LLVM_PROFDATA_BIN",
        root.join("missing").join("llvm-profdata"),
    );
    std::env::set_var(
        "PROTHEUS_LLVM_BOLT_BIN",
        root.join("missing").join("llvm-bolt"),
    );

    assert_eq!(
        canyon_plane::run(
            root,
            &[
                "release-pipeline".to_string(),
                "--op=run".to_string(),
                "--binary=protheusd".to_string(),
                "--target=x86_64-unknown-linux-musl".to_string(),
                "--profile=release-minimal".to_string(),
                "--strict=1".to_string(),
            ],
        ),
        1
    );
    let latest = read_json(&latest_path(&canyon_state));
    let errors = latest
        .get("errors")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert!(errors
        .iter()
        .any(|row| row.as_str() == Some("tool_missing:llvm-profdata")));
    if !cfg!(target_os = "macos") {
        assert!(errors
            .iter()
            .any(|row| row.as_str() == Some("tool_missing:llvm-bolt")));
    }
}

#[test]
fn v7_canyon_release_pipeline_reuses_real_release_artifact_when_minimal_profile_missing_non_strict()
{
    let _guard = test_env_lock();
    let tmp = temp_root("canyon_batch2_release_fallback");
    let root = tmp.path();
    let canyon_state = root.join("local").join("state").join("canyon");
    std::env::set_var(ENV_KEY, &canyon_state);

    write_text(
        root,
        "core/layer0/ops/Cargo.toml",
        "[package]\nname='protheus-ops-core'\n[features]\nminimal = []\n",
    );
    write_release_security_workflow(root);
    let toolbin = install_tool_stubs(root);
    std::env::set_var("PROTHEUS_CARGO_BIN", toolbin.join("cargo"));
    std::env::set_var("PROTHEUS_STRIP_BIN", toolbin.join("strip"));
    std::env::remove_var("PROTHEUS_LLVM_PROFDATA_BIN");
    std::env::remove_var("PROTHEUS_LLVM_BOLT_BIN");

    write_large_binary(
        root,
        "target/x86_64-unknown-linux-musl/release/protheusd",
        1_200_000,
    );

    assert_eq!(
        canyon_plane::run(
            root,
            &[
                "release-pipeline".to_string(),
                "--op=run".to_string(),
                "--binary=protheusd".to_string(),
                "--target=x86_64-unknown-linux-musl".to_string(),
                "--profile=release-minimal".to_string(),
                "--strict=0".to_string(),
            ],
        ),
        0
    );

    let latest = read_json(&latest_path(&canyon_state));
    assert_eq!(latest.get("ok").and_then(Value::as_bool), Some(true));
    assert_eq!(
        latest.get("artifact_source").and_then(Value::as_str),
        Some(
            root.join("target/x86_64-unknown-linux-musl/release/protheusd")
                .to_string_lossy()
                .as_ref()
        )
    );
    assert_eq!(
        latest.get("final_size_bytes").and_then(Value::as_u64),
        Some(1_200_000)
    );
}
