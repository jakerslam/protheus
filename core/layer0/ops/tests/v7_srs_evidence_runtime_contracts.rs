use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

fn assert_path_contains(repo_root: &Path, rel: &str, needle: &str) {
    let path = repo_root.join(rel);
    assert!(path.exists(), "missing evidence path: {rel}");
    let content = fs::read_to_string(&path).unwrap_or_default();
    assert!(
        content.contains(needle),
        "expected marker `{needle}` in {rel}"
    );
}

fn assert_all_paths(repo_root: &Path, checks: &[(&str, &str)]) {
    for (rel, needle) in checks {
        assert_path_contains(repo_root, rel, needle);
    }
}

#[test]
fn pure_workspace_srs_rows_have_runtime_evidence_paths() {
    let repo = repo_root();
    let checks = [
        // V7-PURE-WORKSPACE-001.1
        ("client/pure-workspace/src/lib.rs", "pure-workspace"),
        ("client/pure-workspace/src/main.rs", "benchmark-ping"),
        // V7-PURE-WORKSPACE-001.2
        (
            "core/layer0/ops/src/protheusctl_routes_parts/010-command-routing.rs",
            "tiny-max",
        ),
        (
            "core/layer0/ops/src/canyon_plane_parts/050-ecosystem-command.rs",
            "--pure",
        ),
        (
            "core/layer0/ops/tests/v7_pure_workspace_integration.rs",
            "ecosystem_init_pure_dry_run_emits_pure_components",
        ),
        // V7-PURE-WORKSPACE-001.3
        ("install.sh", "--pure"),
        ("install.ps1", "InstallPure"),
        (
            "core/layer0/ops/src/benchmark_matrix_parts/040-run-impl.rs",
            "pure_workspace_measured",
        ),
        // V7-PURE-WORKSPACE-002.1
        ("core/layer0/ops/Cargo.toml", "embedded-max"),
        (
            "core/layer0/ops/src/protheusd_parts/030-embedded-minimal-core-status.rs",
            "tiny-max-status",
        ),
        // V7-PURE-WORKSPACE-002.2
        (
            "core/layer0/ops/src/benchmark_matrix_parts/040-run-impl.rs",
            "pure_workspace_tiny_max_measured",
        ),
        ("README.md", "Tiny-max"),
    ];
    assert_all_paths(&repo, &checks);
}

#[test]
fn bench_recovery_srs_rows_have_runtime_evidence_paths() {
    let repo = repo_root();
    let checks = [
        // V7-BENCH-RECOVERY-001.1
        ("core/layer0/ops/src/lib.rs", "core-lazy"),
        (
            "core/layer0/ops/tests/v7_bench_recovery_integration.rs",
            "runtime_efficiency_core_lazy_path_stays_receipted_and_live",
        ),
        // V7-BENCH-RECOVERY-001.2
        (
            "core/layer0/ops/src/lib.rs",
            "configure_low_memory_allocator_env",
        ),
        (
            "core/layer0/ops/src/protheusd_parts/010-print-json.rs",
            "configure_low_memory_allocator_env",
        ),
        // V7-BENCH-RECOVERY-001.3
        ("core/layer0/ops/src/lib.rs", "no-client-bloat"),
        ("install.sh", ".tar.zst"),
        ("install.ps1", ".tar.zst"),
        // V7-BENCH-RECOVERY-001.4
        (
            "core/layer0/tiny_runtime/Cargo.toml",
            "protheus-tiny-runtime",
        ),
        (
            "core/layer0/ops/src/protheusd_parts/030-embedded-minimal-core-status.rs",
            "tiny-status",
        ),
    ];
    assert_all_paths(&repo, &checks);
}
