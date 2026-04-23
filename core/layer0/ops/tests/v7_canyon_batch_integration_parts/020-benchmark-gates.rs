#[test]
fn v7_canyon_benchmark_gate_uses_adjacent_evidence_when_local_state_is_empty() {
    let _guard = test_env_lock();
    let tmp = temp_root("canyon_adjacent_benchmark");
    let root = tmp.path();
    let canyon_state = root.join("local").join("state").join("canyon");
    std::env::set_var(ENV_KEY, &canyon_state);

    write_json(
        root,
        "core/local/state/ops/top1_assurance/benchmark_latest.json",
        &serde_json::json!({
            "metrics": {
                "cold_start_ms": 72.0,
                "idle_rss_mb": 21.0,
                "install_size_mb": 22.4,
                "tasks_per_sec": 6400.0
            }
        }),
    );
    write_json(
        root,
        "core/local/state/ops/enterprise_hardening/f100/ops_bridge.json",
        &serde_json::json!({"providers": [{"provider": "splunk"}]}),
    );
    write_json(
        root,
        "core/local/state/ops/enterprise_hardening/f100/scale_ha_certification.json",
        &serde_json::json!({"airgap_agents": 10000, "regions": 3}),
    );
    write_json(
        root,
        "core/local/state/ops/enterprise_hardening/f100/adoption_bootstrap/bootstrap.json",
        &serde_json::json!({"profile": "enterprise", "compliance": true}),
    );
    write_json(
        root,
        "local/state/canyon/latest.json",
        &serde_json::json!({"ok": true}),
    );

    assert_eq!(
        canyon_plane::run(
            root,
            &[
                "benchmark-gate".to_string(),
                "--op=run".to_string(),
                "--milestone=day90".to_string(),
                "--strict=1".to_string(),
            ]
        ),
        0
    );
    let latest = read_json(&latest_path(&canyon_state));
    assert_eq!(
        latest.get("ok").and_then(Value::as_bool),
        Some(true),
        "{latest}"
    );
    assert_eq!(
        latest
            .get("claim_evidence")
            .and_then(Value::as_array)
            .and_then(|rows| rows.first())
            .and_then(|row| row.get("evidence"))
            .and_then(|row| row.get("performance_source"))
            .and_then(Value::as_str)
            .map(|v| v.contains("top1_assurance/benchmark_latest.json")),
        Some(true)
    );

    std::env::remove_var(ENV_KEY);
}

#[test]
fn v7_canyon_benchmark_gate_prefers_real_binary_and_materializes_missing_enterprise_evidence() {
    let _guard = test_env_lock();
    let tmp = temp_root("canyon_gate_materialize");
    let root = tmp.path();
    let canyon_state = root.join("local").join("state").join("canyon");
    std::env::set_var(ENV_KEY, &canyon_state);

    install_static_infringd_fixture(root, 3);

    write_json(
        root,
        "docs/client/reports/runtime_snapshots/ops/proof_pack/top1_benchmark_snapshot.json",
        &serde_json::json!({
            "metrics": {
                "cold_start_ms": 74.5,
                "idle_rss_mb": 22.1,
                "install_size_mb": 126.4,
                "tasks_per_sec": 7420.0
            }
        }),
    );
    write_json(
        root,
        "core/local/state/ops/enterprise_hardening/f100/scale_ha_certification.json",
        &serde_json::json!({"airgap_agents": 10000, "regions": 3}),
    );
    write_json(
        root,
        "local/state/canyon/latest.json",
        &serde_json::json!({"ok": true}),
    );

    assert_eq!(
        canyon_plane::run(
            root,
            &[
                "benchmark-gate".to_string(),
                "--op=run".to_string(),
                "--milestone=day90".to_string(),
                "--strict=1".to_string(),
            ]
        ),
        0
    );

    let latest = read_json(&latest_path(&canyon_state));
    assert_eq!(
        latest.get("ok").and_then(Value::as_bool),
        Some(true),
        "{latest}"
    );

    let evidence = latest
        .get("claim_evidence")
        .and_then(Value::as_array)
        .and_then(|rows| rows.first())
        .and_then(|row| row.get("evidence"))
        .cloned()
        .expect("claim evidence payload");
    assert_eq!(
        evidence
            .get("binary_size_source")
            .and_then(Value::as_str)
            .map(|v| v.contains("target/x86_64-unknown-linux-musl/release/infringd")),
        Some(true)
    );
    assert_eq!(
        evidence
            .get("audit_source")
            .and_then(Value::as_str)
            .map(|v| v.contains("enterprise_hardening/moat/explorer/index.json")),
        Some(true)
    );
    assert_eq!(
        evidence
            .get("adoption_source")
            .and_then(Value::as_str)
            .map(|v| v.contains("enterprise_hardening/f100/adoption_bootstrap/bootstrap.json")),
        Some(true)
    );
    assert!(root
        .join("core/local/state/ops/enterprise_hardening/moat/explorer/index.json")
        .exists());
    assert!(root
        .join("core/local/state/ops/enterprise_hardening/f100/adoption_bootstrap/bootstrap.json")
        .exists());

    std::env::remove_var(ENV_KEY);
}
