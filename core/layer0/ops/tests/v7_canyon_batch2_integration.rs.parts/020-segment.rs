#[test]
fn v7_canyon_batch2_contracts_are_behavior_proven() {
    let _guard = test_env_lock();
    let tmp = temp_root("canyon_batch2");
    let root = tmp.path();
    let canyon_state = root.join("local").join("state").join("canyon");
    std::env::set_var(ENV_KEY, &canyon_state);

    write_text(
        root,
        "core/layer0/kernel_layers/Cargo.toml",
        "[package]\nname='kernel_layers'\n[features]\ndefault = []\nno_std_probe = []\n",
    );
    write_text(root, "core/layer0/kernel_layers/src/lib.rs", "#![no_std]\n");
    write_text(
        root,
        "core/layer2/conduit/Cargo.toml",
        "[package]\nname='conduit'\n[features]\ndefault = []\nno_std_probe = []\n",
    );
    write_text(root, "core/layer2/conduit/src/lib.rs", "#![no_std]\n");
    write_text(
        root,
        "core/layer0/memory/Cargo.toml",
        "[package]\nname='memory'\n[features]\ndefault = []\nno_std_probe = []\n",
    );
    write_text(root, "core/layer0/memory/src/lib.rs", "pub fn x() {}\n");
    write_text(
        root,
        "core/layer1/security/Cargo.toml",
        "[package]\nname='security'\n[features]\ndefault = []\nno_std_probe = []\n",
    );
    write_text(root, "core/layer1/security/src/lib.rs", "pub fn y() {}\n");
    write_text(
        root,
        "core/layer0/ops/Cargo.toml",
        "[package]\nname='protheus-ops-core'\n[features]\nminimal = []\n",
    );
    write_text(
        root,
        "core/layer0/alloc.rs",
        "pub struct Layer0CountingAllocator;\n",
    );
    write_substrate_adapter_graph(root);
    write_release_security_workflow(root);
    write_size_trust_workflows(root);

    let stub_bin = install_stub_binary(root);
    let toolbin = install_tool_stubs(root);
    std::env::set_var("PROTHEUS_CARGO_BIN", toolbin.join("cargo"));
    std::env::set_var("PROTHEUS_STRIP_BIN", toolbin.join("strip"));
    std::env::set_var("PROTHEUS_LLVM_PROFDATA_BIN", toolbin.join("llvm-profdata"));
    std::env::set_var("PROTHEUS_LLVM_BOLT_BIN", toolbin.join("llvm-bolt"));

    assert_eq!(
        canyon_plane::run(root, &["footprint".to_string(), "--strict=1".to_string()]),
        0
    );
    let mut latest = read_json(&latest_path(&canyon_state));
    assert_eq!(
        latest.get("type").and_then(Value::as_str),
        Some("canyon_plane_footprint")
    );
    assert_claim(&latest, "V7-CANYON-002.1");

    assert_eq!(
        canyon_plane::run(
            root,
            &[
                "lazy-substrate".to_string(),
                "--op=enable".to_string(),
                "--feature-set=minimal".to_string(),
                "--strict=1".to_string(),
            ],
        ),
        0
    );
    assert_eq!(
        canyon_plane::run(
            root,
            &[
                "lazy-substrate".to_string(),
                "--op=load".to_string(),
                "--adapter=wifi-csi-engine".to_string(),
                "--strict=1".to_string(),
            ],
        ),
        0
    );
    latest = read_json(&latest_path(&canyon_state));
    assert_claim(&latest, "V7-CANYON-002.2");

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
        0
    );
    latest = read_json(&latest_path(&canyon_state));
    assert_claim(&latest, "V7-CANYON-002.3");

    assert_eq!(
        canyon_plane::run(
            root,
            &[
                "efficiency".to_string(),
                "--strict=1".to_string(),
                format!("--binary-path={}", stub_bin.display()),
                "--idle-memory-mb=10".to_string(),
                "--concurrent-agents=50".to_string(),
            ],
        ),
        0
    );
    write_text(root, "workspace/README.md", "# workspace\n");
    assert_eq!(
        canyon_plane::run(
            root,
            &[
                "workflow".to_string(),
                "--op=run".to_string(),
                "--goal=ship_end_to_end".to_string(),
                format!("--workspace={}", root.join("workspace").display()),
                "--strict=1".to_string(),
            ],
        ),
        0
    );
    assert_eq!(
        canyon_plane::run(
            root,
            &[
                "scheduler".to_string(),
                "--op=simulate".to_string(),
                "--agents=10000".to_string(),
                "--nodes=4".to_string(),
                "--modes=kubernetes,edge,distributed".to_string(),
                "--strict=1".to_string(),
            ],
        ),
        0
    );
    assert_eq!(
        canyon_plane::run(
            root,
            &[
                "control-plane".to_string(),
                "--op=snapshot".to_string(),
                "--rbac=1".to_string(),
                "--sso=1".to_string(),
                "--hitl=1".to_string(),
                "--strict=1".to_string(),
            ],
        ),
        0
    );
    assert_eq!(
        canyon_plane::run(
            root,
            &[
                "adoption".to_string(),
                "--op=run-demo".to_string(),
                "--tutorial=quickstart".to_string(),
                "--strict=1".to_string(),
            ],
        ),
        0
    );
    assert_eq!(
        canyon_plane::run(
            root,
            &[
                "benchmark-gate".to_string(),
                "--op=run".to_string(),
                "--milestone=day90".to_string(),
                "--strict=0".to_string(),
            ],
        ),
        0
    );

    assert_eq!(
        canyon_plane::run(
            root,
            &[
                "receipt-batching".to_string(),
                "--op=flush".to_string(),
                "--strict=1".to_string(),
            ],
        ),
        0
    );
    latest = read_json(&latest_path(&canyon_state));
    assert_claim(&latest, "V7-CANYON-002.4");

    assert_eq!(
        canyon_plane::run(
            root,
            &[
                "package-release".to_string(),
                "--op=build".to_string(),
                "--strict=1".to_string(),
            ],
        ),
        0
    );
    latest = read_json(&latest_path(&canyon_state));
    assert_claim(&latest, "V7-CANYON-002.5");

    assert_eq!(
        canyon_plane::run(root, &["size-trust".to_string(), "--strict=1".to_string()]),
        0
    );
    latest = read_json(&latest_path(&canyon_state));
    assert_eq!(
        latest.get("type").and_then(Value::as_str),
        Some("canyon_plane_size_trust_center")
    );
    assert_claim(&latest, "V7-CANYON-002.6");
}

#[test]
fn v7_canyon_release_pipeline_allows_missing_optional_llvm_tools_when_not_strict_and_size_trust_uses_top1_fallback(
) {
    let _guard = test_env_lock();
    let tmp = temp_root("canyon_batch2_optional_tools");
    let root = tmp.path();
    let canyon_state = root.join("local").join("state").join("canyon");
    std::env::set_var(ENV_KEY, &canyon_state);

    write_text(
        root,
        "core/layer0/ops/Cargo.toml",
        "[package]\nname='protheus-ops-core'\n[features]\nminimal = []\n",
    );
    write_release_security_workflow(root);
    write_size_trust_workflows(root);
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

    write_top1_benchmark(root, 28, 9.5, 2.7, 18_500);

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
    assert!(latest
        .get("optimization")
        .and_then(|v| v.get("missing_optional_tools"))
        .and_then(Value::as_array)
        .map(|rows| rows.iter().any(|row| row.as_str() == Some("llvm-profdata")))
        .unwrap_or(false));

    assert_eq!(
        canyon_plane::run(
            root,
            &[
                "package-release".to_string(),
                "--op=build".to_string(),
                "--strict=0".to_string(),
            ],
        ),
        0
    );
    assert_eq!(
        canyon_plane::run(root, &["size-trust".to_string(), "--strict=0".to_string()]),
        0
    );
    let latest = read_json(&latest_path(&canyon_state));
    assert_eq!(latest.get("ok").and_then(Value::as_bool), Some(true));
    assert_eq!(
        latest
            .get("metrics")
            .and_then(|v| v.get("cold_start_ms"))
            .and_then(Value::as_u64),
        Some(28)
    );
    assert_eq!(
        latest
            .get("metrics")
            .and_then(|v| v.get("idle_rss_mb"))
            .and_then(Value::as_f64),
        Some(9.5)
    );
}

