// SPDX-License-Identifier: Apache-2.0

use infring_ops_core::{binary_blob_runtime, directive_kernel, security_plane};
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::sync::{Mutex, OnceLock};

fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn env_guard() -> std::sync::MutexGuard<'static, ()> {
    env_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner())
}

fn read_json(path: &Path) -> Value {
    let raw = fs::read_to_string(path).expect("read json");
    serde_json::from_str(&raw).expect("decode json")
}

fn has_claim(receipt: &Value, claim_id: &str) -> bool {
    receipt
        .get("claim_evidence")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .any(|row| row.get("id").and_then(Value::as_str) == Some(claim_id))
}

fn read_jsonl(path: &Path) -> Vec<Value> {
    let Ok(raw) = fs::read_to_string(path) else {
        return Vec::new();
    };
    raw.lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .collect()
}

fn latest(scope: &str, root: &Path) -> Value {
    read_json(
        &root
            .join("core")
            .join("local")
            .join("state")
            .join("ops")
            .join(scope)
            .join("latest.json"),
    )
}

fn allow(root: &Path, directive: &str) {
    assert_eq!(
        directive_kernel::run(
            root,
            &[
                "prime-sign".to_string(),
                format!("--directive={directive}"),
                "--signer=tester".to_string(),
            ],
        ),
        0
    );
}

#[test]
fn v8_batch23_supersession_enforces_conduit_gate_with_trace() {
    let _guard = env_guard();
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();
    std::env::set_var("DIRECTIVE_KERNEL_SIGNING_KEY", "batch23-signing-key");
    std::env::set_var("BINARY_BLOB_VAULT_SIGNING_KEY", "batch23-blob-key");

    allow(root, "allow:blob:*");
    assert!(has_claim(
        &latest("directive_kernel", root),
        "V8-DIRECTIVES-001.1"
    ));
    let vault_before = latest("directive_kernel", root)
        .get("entry")
        .cloned()
        .unwrap_or(Value::Null);
    let allow_entry_hash = vault_before
        .get("entry_hash")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();

    let module_path = root.join("demo_module.rs");
    fs::write(&module_path, "pub fn demo() -> u64 { 11 }\n").expect("write module");
    assert_eq!(
        binary_blob_runtime::run(
            root,
            &[
                "settle".to_string(),
                "--module=demo".to_string(),
                format!("--module-path={}", module_path.display()),
                "--apply=1".to_string(),
            ],
        ),
        0
    );
    let settled = latest("binary_blob_runtime", root);
    assert!(has_claim(&settled, "V8-BINARY-BLOB-001.1"));
    assert!(has_claim(&settled, "V8-BINARY-BLOB-001.2"));
    assert_eq!(
        binary_blob_runtime::run(root, &["load".to_string(), "--module=demo".to_string()]),
        0
    );
    let load_ok = latest("binary_blob_runtime", root);
    assert!(has_claim(&load_ok, "V8-BINARY-BLOB-001.1"));

    assert_eq!(
        directive_kernel::run(
            root,
            &[
                "supersede".to_string(),
                "--target=allow:blob:*".to_string(),
                "--directive=deny:blob:load:demo".to_string(),
                "--signer=tester".to_string(),
            ],
        ),
        0
    );
    assert!(has_claim(
        &latest("directive_kernel", root),
        "V8-DIRECTIVES-001.1"
    ));

    assert_eq!(
        binary_blob_runtime::run(root, &["load".to_string(), "--module=demo".to_string()]),
        2
    );
    let denied = latest("binary_blob_runtime", root);
    assert_eq!(
        denied.get("error").and_then(Value::as_str),
        Some("directive_gate_denied")
    );
    assert_eq!(
        denied
            .get("gate_evaluation")
            .and_then(|v| v.get("deny_hits"))
            .and_then(Value::as_array)
            .map(|rows| !rows.is_empty()),
        Some(true)
    );
    assert!(has_claim(&denied, "V8-BINARY-BLOB-001.1"));

    let vault_after = read_json(
        &root
            .join("core")
            .join("local")
            .join("state")
            .join("ops")
            .join("directive_kernel")
            .join("prime_directive_vault.json"),
    );
    let allow_hash_after = vault_after
        .get("prime")
        .and_then(Value::as_array)
        .and_then(|rows| rows.first())
        .and_then(|row| row.get("entry_hash"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    assert_eq!(allow_entry_hash, allow_hash_after);

    std::env::remove_var("DIRECTIVE_KERNEL_SIGNING_KEY");
    std::env::remove_var("BINARY_BLOB_VAULT_SIGNING_KEY");
}

#[test]
fn v8_batch23_blob_vault_chain_tamper_fails_closed() {
    let _guard = env_guard();
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();
    std::env::set_var("DIRECTIVE_KERNEL_SIGNING_KEY", "batch23-signing-key");
    std::env::set_var("BINARY_BLOB_VAULT_SIGNING_KEY", "batch23-blob-key");

    allow(root, "allow:blob:*");
    let module_path = root.join("demo_module.rs");
    fs::write(&module_path, "pub fn demo() -> u64 { 21 }\n").expect("write module");
    assert_eq!(
        binary_blob_runtime::run(
            root,
            &[
                "settle".to_string(),
                "--module=demo".to_string(),
                format!("--module-path={}", module_path.display()),
                "--apply=1".to_string(),
            ],
        ),
        0
    );

    let vault_path = root
        .join("core")
        .join("local")
        .join("state")
        .join("ops")
        .join("binary_blob_runtime")
        .join("prime_blob_vault.json");
    let mut vault = read_json(&vault_path);
    vault["chain_head"] = Value::String("tampered_chain_head".to_string());
    fs::write(
        &vault_path,
        serde_json::to_string_pretty(&vault).expect("serialize tampered vault"),
    )
    .expect("write tampered vault");

    assert_eq!(
        binary_blob_runtime::run(root, &["vault-status".to_string()]),
        2
    );
    assert_eq!(
        binary_blob_runtime::run(root, &["load".to_string(), "--module=demo".to_string()]),
        2
    );
    let latest_load = latest("binary_blob_runtime", root);
    assert_eq!(
        latest_load.get("error").and_then(Value::as_str),
        Some("prime_blob_vault_chain_invalid")
    );
    assert!(has_claim(&latest_load, "V8-BINARY-BLOB-001.1"));

    std::env::remove_var("DIRECTIVE_KERNEL_SIGNING_KEY");
    std::env::remove_var("BINARY_BLOB_VAULT_SIGNING_KEY");
}

#[test]
fn v8_batch23_directive_migrate_status_surface_integrity() {
    let _guard = env_guard();
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();
    std::env::set_var("DIRECTIVE_KERNEL_SIGNING_KEY", "batch23-signing-key");

    let legacy_path = root
        .join("docs")
        .join("workspace")
        .join("AGENT-CONSTITUTION.md");
    fs::create_dir_all(legacy_path.parent().expect("parent")).expect("mkdir");
    fs::write(&legacy_path, "- allow:blob:*\n- deny:rsi:unsafe\n")
        .expect("write legacy directives");

    assert_eq!(
        directive_kernel::run(root, &["migrate".to_string(), "--apply=1".to_string()]),
        0
    );
    assert_eq!(directive_kernel::run(root, &["status".to_string()]), 0);

    let integrity = directive_kernel::directive_vault_integrity(root);
    assert_eq!(integrity.get("ok").and_then(Value::as_bool), Some(true));
    let vault = read_json(
        &root
            .join("core")
            .join("local")
            .join("state")
            .join("ops")
            .join("directive_kernel")
            .join("prime_directive_vault.json"),
    );
    assert_eq!(
        vault
            .get("prime")
            .and_then(Value::as_array)
            .map(|rows| rows.len() >= 1),
        Some(true)
    );

    std::env::remove_var("DIRECTIVE_KERNEL_SIGNING_KEY");
}

