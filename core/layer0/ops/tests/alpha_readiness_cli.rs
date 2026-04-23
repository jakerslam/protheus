use infring_ops_core::alpha_readiness;
use std::fs;
use std::path::{Path, PathBuf};

fn seed_workspace(root: &Path) {
    let files = [
        ("install.sh", "--pure\n--tiny-max\n--repair\n"),
        (
            "install.ps1",
            "param([switch]$Pure,[switch]$TinyMax,[switch]$Repair)\n",
        ),
        (
            "verify.sh",
            "INFRING_VERIFY_PROOF_TIMEOUT_SEC=${INFRING_VERIFY_PROOF_TIMEOUT_SEC:-420}\n",
        ),
        (
            "README.md",
            "## Alpha Readiness Checklist\ninfring alpha-check\n",
        ),
        (
            "package.json",
            r#"{"bin":{"infring":"a","infringctl":"b","infringd":"c"}}"#,
        ),
        (".github/workflows/release.yml", "name: release\n"),
        (".github/workflows/size-gate.yml", "name: size gate\n"),
        (
            ".github/workflows/infringd-static-size-gate.yml",
            "name: static size gate\n",
        ),
        ("docs/workspace/templates/assistant/SOUL.md", "seed\n"),
        ("docs/workspace/templates/assistant/USER.md", "seed\n"),
        ("docs/workspace/templates/assistant/HEARTBEAT.md", "seed\n"),
        ("docs/workspace/templates/assistant/IDENTITY.md", "seed\n"),
        ("docs/workspace/templates/assistant/TOOLS.md", "seed\n"),
    ];
    for (rel, contents) in files {
        let path = root.join(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("parent");
        }
        fs::write(path, contents).expect("write");
    }
}

fn latest_path(root: &Path) -> PathBuf {
    root.join("local/state/ops/alpha_readiness/latest.json")
}

fn assert_no_runtime_context_leak(raw: &str) {
    const FORBIDDEN: [&str; 6] = [
        "You are an expert Python programmer.",
        "[PATCH v2",
        "List Leaves (25",
        "BEGIN_OPENCLAW_INTERNAL_CONTEXT",
        "END_OPENCLAW_INTERNAL_CONTEXT",
        "UNTRUSTED_CHILD_RESULT_DELIMITER",
    ];
    for marker in FORBIDDEN {
        assert!(
            !raw.contains(marker),
            "runtime payload leaked forbidden marker `{marker}`: {raw}"
        );
    }
}

#[test]
fn alpha_readiness_run_persists_latest_snapshot() {
    let temp = tempfile::tempdir().expect("tempdir");
    seed_workspace(temp.path());

    let code = alpha_readiness::run(
        temp.path(),
        &[
            "run".to_string(),
            "--strict=1".to_string(),
            "--run-gates=0".to_string(),
        ],
    );
    assert_eq!(code, 0);
    assert!(latest_path(temp.path()).exists());
    let raw = fs::read_to_string(latest_path(temp.path())).expect("read latest snapshot");
    assert_no_runtime_context_leak(&raw);
}

#[test]
fn alpha_readiness_status_returns_zero_after_run() {
    let temp = tempfile::tempdir().expect("tempdir");
    seed_workspace(temp.path());
    let run_code = alpha_readiness::run(temp.path(), &["run".to_string()]);
    assert_eq!(run_code, 0);
    let status_code = alpha_readiness::run(temp.path(), &["status".to_string()]);
    assert_eq!(status_code, 0);
}
