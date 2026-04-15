use protheus_ops_core::canyon_plane;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

const ENV_KEY: &str = "PROTHEUS_CANYON_PLANE_STATE_ROOT";

fn temp_root(prefix: &str) -> tempfile::TempDir {
    tempfile::Builder::new()
        .prefix(&format!("protheus_{prefix}_"))
        .tempdir()
        .expect("tempdir")
}

fn test_env_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(())).lock().expect("lock")
}

fn latest_path(state_root: &Path) -> PathBuf {
    state_root.join("latest.json")
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

fn install_stub_binary(root: &Path) -> PathBuf {
    let bin = root.join("bin").join("protheus-ops");
    if let Some(parent) = bin.parent() {
        fs::create_dir_all(parent).expect("mkdir bin dir");
    }
    fs::write(&bin, "#!/bin/sh\nexit 0\n").expect("write stub binary");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&bin).expect("stat").permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&bin, perms).expect("chmod");
    }
    bin
}

fn install_tool_stubs(root: &Path) -> PathBuf {
    let dir = root.join("toolbin");
    fs::create_dir_all(&dir).expect("mkdir toolbin");
    let cargo = dir.join("cargo");
    fs::write(
        &cargo,
        r#"#!/bin/sh
set -eu
TARGET=""
PROFILE="release-minimal"
BIN="protheusd"
while [ "$#" -gt 0 ]; do
  case "$1" in
    --target) TARGET="$2"; shift 2 ;;
    --profile) PROFILE="$2"; shift 2 ;;
    --bin) BIN="$2"; shift 2 ;;
    *) shift ;;
  esac
done
mkdir -p "target/${TARGET}/${PROFILE}"
printf '#!/bin/sh\necho built\n' > "target/${TARGET}/${PROFILE}/${BIN}"
chmod +x "target/${TARGET}/${PROFILE}/${BIN}"
"#,
    )
    .expect("write cargo stub");
    let strip = dir.join("strip");
    fs::write(&strip, "#!/bin/sh\nexit 0\n").expect("write strip stub");
    let prof = dir.join("llvm-profdata");
    fs::write(&prof, "#!/bin/sh\nexit 0\n").expect("write profdata stub");
    let bolt = dir.join("llvm-bolt");
    fs::write(&bolt, "#!/bin/sh\nexit 0\n").expect("write bolt stub");
    #[cfg(unix)]
    for p in [&cargo, &strip, &prof, &bolt] {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(p).expect("stat").permissions();
        perms.set_mode(0o755);
        fs::set_permissions(p, perms).expect("chmod");
    }
    dir
}

fn write_top1_benchmark(
    root: &Path,
    cold_start_ms: u64,
    idle_rss_mb: f64,
    install_size_mb: f64,
    tasks_per_sec: u64,
) {
    write_text(
        root,
        "core/local/state/ops/top1_assurance/benchmark_latest.json",
        &serde_json::json!({
            "metrics": {
                "cold_start_ms": cold_start_ms,
                "idle_rss_mb": idle_rss_mb,
                "install_size_mb": install_size_mb,
                "tasks_per_sec": tasks_per_sec
            }
        })
        .to_string(),
    );
}

fn write_large_binary(root: &Path, rel: &str, size_bytes: usize) {
    let path = root.join(rel);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("mkdir binary dir");
    }
    fs::write(path, vec![0u8; size_bytes]).expect("write large binary");
}

fn write_substrate_adapter_graph(root: &Path) {
    write_text(
        root,
        "client/runtime/config/substrate_adapter_graph.json",
        &serde_json::json!({
            "schema_id": "substrate_adapter_graph",
            "schema_version": "1.0",
            "adapters": [
                {"id": "wifi-csi-engine", "feature_gate": "embedded-minimal-core", "feature_sets": ["minimal", "full-substrate"]},
                {"id": "browser-sandbox", "feature_gate": "full-substrate", "feature_sets": ["full-substrate"]},
                {"id": "bio-adapter-template", "feature_gate": "full-substrate", "feature_sets": ["full-substrate"]},
                {"id": "vbrowser", "feature_gate": "full-substrate", "feature_sets": ["full-substrate"]},
                {"id": "binary-vuln", "feature_gate": "full-substrate", "feature_sets": ["full-substrate"]}
            ]
        })
        .to_string(),
    );
}

fn write_release_security_workflow(root: &Path) {
    write_text(
        root,
        ".github/workflows/release-security-artifacts.yml",
        "name: Release Security Artifacts\njobs:\n  release:\n    steps:\n      - uses: actions/attest-build-provenance@v2\n      - run: supply-chain-provenance-v2 run --strict=1\n      - run: echo reproducible_build_equivalence.json\n",
    );
}

fn write_size_trust_workflows(root: &Path) {
    write_text(
        root,
        ".github/workflows/size-gate.yml",
        "name: Size Gate\njobs:\n  gate:\n    steps:\n      - run: echo Build static protheusd\n      - run: echo Enforce full install size gate\n      - run: echo Enforce throughput gate\n",
    );
    write_text(
        root,
        ".github/workflows/protheusd-static-size-gate.yml",
        "name: Static Size Gate\njobs:\n  gate:\n    steps:\n      - run: echo Build static protheusd\n      - run: echo Enforce static size gate\n      - run: echo Verify reproducible static rebuild\n",
    );
    write_text(
        root,
        ".github/workflows/nightly-size-trust-center.yml",
        "name: Nightly Size Trust Center\non:\n  schedule:\n    - cron: \"17 7 * * *\"\njobs:\n  publish:\n    steps:\n      - run: echo upload-pages-artifact\n      - run: echo deploy-pages\n",
    );
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

