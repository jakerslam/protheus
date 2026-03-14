use protheus_ops_core::canyon_plane;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

const ENV_KEY: &str = "PROTHEUS_CANYON_PLANE_STATE_ROOT";

fn temp_root(prefix: &str) -> tempfile::TempDir {
    tempfile::Builder::new()
        .prefix(&format!("protheus_{prefix}_"))
        .tempdir()
        .expect("tempdir")
}

fn latest_path() -> PathBuf {
    PathBuf::from(
        std::env::var(ENV_KEY).expect("expected canyon state env in tests for latest path lookup"),
    )
    .join("latest.json")
}

fn read_json(path: &Path) -> Value {
    let raw = fs::read_to_string(path).expect("read json");
    serde_json::from_str::<Value>(&raw).expect("parse json")
}

fn write_text(root: &Path, rel: &str, body: &str) {
    let p = root.join(rel);
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent).expect("mkdir");
    }
    fs::write(p, body).expect("write");
}

fn assert_claim(payload: &Value, id: &str) {
    let claims = payload
        .get("claim_evidence")
        .and_then(Value::as_array)
        .expect("claim evidence array");
    assert!(
        claims
            .iter()
            .any(|row| row.get("id").and_then(Value::as_str) == Some(id)),
        "missing claim evidence {id}: {payload}"
    );
}

fn install_stub_binary(root: &Path) -> PathBuf {
    let bin = root.join("bin").join("protheus-ops");
    if let Some(parent) = bin.parent() {
        fs::create_dir_all(parent).expect("mkdir bin dir");
    }
    fs::write(
        &bin,
        "#!/bin/sh\n# stub cold-start probe for canyon test\nexit 0\n",
    )
    .expect("write stub binary");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&bin).expect("stat").permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&bin, perms).expect("chmod");
    }
    bin
}

#[test]
fn v7_canyon_contracts_are_behavior_proven() {
    let tmp = temp_root("canyon_batch");
    let root = tmp.path();
    let canyon_state = root.join("state").join("canyon");
    std::env::set_var(ENV_KEY, &canyon_state);

    let stub_bin = install_stub_binary(root);

    assert_eq!(
        canyon_plane::run(
            root,
            &[
                "efficiency".to_string(),
                "--strict=1".to_string(),
                format!("--binary-path={}", stub_bin.display()),
                "--idle-memory-mb=20".to_string(),
                "--concurrent-agents=50".to_string(),
            ]
        ),
        0
    );
    let mut latest = read_json(&latest_path());
    assert_eq!(
        latest.get("type").and_then(Value::as_str),
        Some("canyon_plane_efficiency")
    );
    assert_claim(&latest, "V7-CANYON-001.1");

    assert_eq!(
        canyon_plane::run(
            root,
            &[
                "hands-army".to_string(),
                "--op=bootstrap".to_string(),
                "--strict=1".to_string(),
            ]
        ),
        0
    );
    latest = read_json(&latest_path());
    assert_eq!(
        latest.get("type").and_then(Value::as_str),
        Some("canyon_plane_hands_army")
    );
    assert_claim(&latest, "V7-CANYON-001.2");
    assert!(
        latest
            .get("hands_count")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            >= 60
    );

    assert_eq!(
        canyon_plane::run(
            root,
            &[
                "evolution".to_string(),
                "--op=propose".to_string(),
                "--kind=code".to_string(),
                "--description=optimize scheduler".to_string(),
                "--strict=1".to_string(),
            ]
        ),
        0
    );
    latest = read_json(&latest_path());
    let proposal_id = latest
        .get("proposal_id")
        .and_then(Value::as_str)
        .expect("proposal id")
        .to_string();
    assert_claim(&latest, "V7-CANYON-001.3");

    assert_eq!(
        canyon_plane::run(
            root,
            &[
                "evolution".to_string(),
                "--op=shadow-simulate".to_string(),
                format!("--proposal-id={proposal_id}"),
                "--score=0.90".to_string(),
                "--strict=1".to_string(),
            ]
        ),
        0
    );
    assert_eq!(
        canyon_plane::run(
            root,
            &[
                "evolution".to_string(),
                "--op=review".to_string(),
                format!("--proposal-id={proposal_id}"),
                "--approved=1".to_string(),
                "--strict=1".to_string(),
            ]
        ),
        0
    );
    assert_eq!(
        canyon_plane::run(
            root,
            &[
                "evolution".to_string(),
                "--op=apply".to_string(),
                format!("--proposal-id={proposal_id}"),
                "--strict=1".to_string(),
            ]
        ),
        0
    );

    assert_eq!(
        canyon_plane::run(
            root,
            &[
                "sandbox".to_string(),
                "--op=run".to_string(),
                "--tier=native".to_string(),
                "--language=rust".to_string(),
                "--fuel=5000".to_string(),
                "--epoch=200".to_string(),
                "--strict=1".to_string(),
            ]
        ),
        0
    );
    latest = read_json(&latest_path());
    assert_claim(&latest, "V7-CANYON-001.4");

    assert_eq!(
        canyon_plane::run(
            root,
            &[
                "ecosystem".to_string(),
                "--op=bootstrap".to_string(),
                "--strict=1".to_string(),
            ]
        ),
        0
    );
    latest = read_json(&latest_path());
    assert_claim(&latest, "V7-CANYON-001.5");

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
            ]
        ),
        0
    );
    latest = read_json(&latest_path());
    assert_claim(&latest, "V7-CANYON-001.6");

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
            ]
        ),
        0
    );
    latest = read_json(&latest_path());
    assert_claim(&latest, "V7-CANYON-001.7");

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
            ]
        ),
        0
    );
    latest = read_json(&latest_path());
    assert_claim(&latest, "V7-CANYON-001.8");

    assert_eq!(
        canyon_plane::run(
            root,
            &[
                "adoption".to_string(),
                "--op=run-demo".to_string(),
                "--tutorial=guided_first_run".to_string(),
                "--strict=1".to_string(),
            ]
        ),
        0
    );
    latest = read_json(&latest_path());
    assert_claim(&latest, "V7-CANYON-001.9");

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
    latest = read_json(&latest_path());
    assert_eq!(
        latest.get("type").and_then(Value::as_str),
        Some("canyon_plane_benchmark_gate")
    );
    assert_claim(&latest, "V7-CANYON-001.10");
    assert_eq!(
        latest
            .get("state")
            .and_then(|v| v.get("release_blocked"))
            .and_then(Value::as_bool),
        Some(false)
    );

    std::env::remove_var(ENV_KEY);
}

#[test]
fn v7_canyon_fail_closed_paths_reject_bypass_and_failed_benchmarks() {
    let tmp = temp_root("canyon_fail_closed");
    let root = tmp.path();
    let canyon_state = root.join("state").join("canyon");
    std::env::set_var(ENV_KEY, &canyon_state);

    assert_eq!(
        canyon_plane::run(
            root,
            &[
                "efficiency".to_string(),
                "--strict=1".to_string(),
                "--bypass=1".to_string(),
            ]
        ),
        1
    );
    let mut latest = read_json(&latest_path());
    assert_eq!(
        latest.get("error").and_then(Value::as_str),
        Some("conduit_bypass_rejected")
    );

    assert_eq!(
        canyon_plane::run(
            root,
            &[
                "benchmark-gate".to_string(),
                "--op=run".to_string(),
                "--milestone=day180".to_string(),
                "--strict=1".to_string(),
            ]
        ),
        1
    );
    latest = read_json(&latest_path());
    assert_eq!(
        latest.get("type").and_then(Value::as_str),
        Some("canyon_plane_benchmark_gate")
    );
    assert_eq!(
        latest
            .get("state")
            .and_then(|v| v.get("release_blocked"))
            .and_then(Value::as_bool),
        Some(true)
    );

    std::env::remove_var(ENV_KEY);
}
