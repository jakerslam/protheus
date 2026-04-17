use protheus_ops_core::canyon_plane;
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::sync::{Mutex, OnceLock};

fn latest_path(state_root: &Path) -> std::path::PathBuf {
    state_root.join("latest.json")
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

fn read_json(path: &Path) -> Value {
    let raw = fs::read_to_string(path).expect("read json");
    assert_no_runtime_context_leak(&raw);
    serde_json::from_str(&raw).expect("parse json")
}

fn test_guard() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(())).lock().expect("lock")
}

#[test]
fn ecosystem_init_pure_dry_run_emits_pure_components() {
    let _guard = test_guard();
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path();
    let state_root = root.join("state/canyon_pure");
    fs::create_dir_all(&state_root).expect("state root");

    std::env::set_var(
        "PROTHEUS_CANYON_PLANE_STATE_ROOT",
        state_root.to_string_lossy().as_ref(),
    );
    std::env::set_var("PROTHEUS_V8_CONDUIT_ENFORCE", "0");
    std::env::set_var("PROTHEUS_V8_CONDUIT_AUDIT_ONLY", "1");
    std::env::set_var("PROTHEUS_V8_CONDUIT_TRACE", "0");

    let code = canyon_plane::run(
        root,
        &[
            "ecosystem".to_string(),
            "--op=init".to_string(),
            "--pure=1".to_string(),
            "--dry-run=1".to_string(),
            format!("--target-dir={}", root.join("demo_pure").display()),
            "--strict=1".to_string(),
        ],
    );
    assert_eq!(code, 0);

    let latest = read_json(&latest_path(&state_root));
    let init = latest.get("init").expect("init summary");
    assert_eq!(
        init.get("workspace_mode").and_then(Value::as_str),
        Some("pure")
    );
    assert_eq!(init.get("dry_run").and_then(Value::as_bool), Some(true));
    let components = init
        .get("components")
        .and_then(Value::as_array)
        .expect("components");
    assert!(components.iter().any(|v| v.as_str() == Some("pure_client")));
    assert!(
        !root.join("demo_pure").exists(),
        "dry-run should not create files"
    );
}

#[test]
fn ecosystem_init_tiny_max_dry_run_sets_tiny_max_contract() {
    let _guard = test_guard();
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path();
    let state_root = root.join("state/canyon_pure_tiny_max");
    fs::create_dir_all(&state_root).expect("state root");

    std::env::set_var(
        "PROTHEUS_CANYON_PLANE_STATE_ROOT",
        state_root.to_string_lossy().as_ref(),
    );
    std::env::set_var("PROTHEUS_V8_CONDUIT_ENFORCE", "0");
    std::env::set_var("PROTHEUS_V8_CONDUIT_AUDIT_ONLY", "1");
    std::env::set_var("PROTHEUS_V8_CONDUIT_TRACE", "0");

    let code = canyon_plane::run(
        root,
        &[
            "ecosystem".to_string(),
            "--op=init".to_string(),
            "--tiny-max=1".to_string(),
            "--dry-run=1".to_string(),
            format!("--target-dir={}", root.join("demo_pure_tiny_max").display()),
            "--strict=1".to_string(),
        ],
    );
    assert_eq!(code, 0);

    let latest = read_json(&latest_path(&state_root));
    let init = latest.get("init").expect("init summary");
    assert_eq!(
        init.get("workspace_mode").and_then(Value::as_str),
        Some("pure")
    );
    assert_eq!(init.get("tiny_max").and_then(Value::as_bool), Some(true));
    assert_eq!(init.get("dry_run").and_then(Value::as_bool), Some(true));
    assert!(
        !root.join("demo_pure_tiny_max").exists(),
        "dry-run should not create files"
    );
}
