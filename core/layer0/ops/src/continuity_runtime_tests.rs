// SPDX-License-Identifier: Apache-2.0
use super::*;
use std::collections::BTreeMap;

fn root() -> tempfile::TempDir {
    tempfile::tempdir().expect("tempdir")
}

fn args(rows: &[&str]) -> Vec<String> {
    rows.iter().map(|row| row.to_string()).collect()
}

fn policy_with_vault_key(policy: &ContinuityPolicy, env_key: &str) -> ContinuityPolicy {
    ContinuityPolicy {
        vault_key_env: env_key.to_string(),
        ..policy.clone()
    }
}

#[test]
fn checkpoint_and_restore_roundtrip() {
    let dir = root();
    let checkpoint = checkpoint_payload(
        dir.path(),
        &default_policy(),
        &args(&[
            "--session-id=session-a",
            "--state-json={\"attention_queue\":[\"a\"],\"memory_graph\":{\"n1\":{}},\"active_personas\":[\"planner\"]}",
            "--apply=1",
        ]),
    )
    .expect("checkpoint");
    assert!(checkpoint
        .get("ok")
        .and_then(Value::as_bool)
        .unwrap_or(false));

    let restored = restore_payload(
        dir.path(),
        &default_policy(),
        &args(&["--session-id=session-a", "--apply=1"]),
    )
    .expect("restore");
    assert!(restored.get("ok").and_then(Value::as_bool).unwrap_or(false));
    assert_eq!(
        restored
            .get("restored_state")
            .and_then(|v| v.get("active_personas"))
            .and_then(Value::as_array)
            .map(|rows| rows.len()),
        Some(1)
    );
}

#[test]
fn degraded_restore_is_blocked_without_override() {
    let dir = root();
    let policy = default_policy();
    let ckpt_path = checkpoints_dir(dir.path()).join("s1_manual_degraded.json");
    write_json(
        &ckpt_path,
        &json!({
            "session_id": "s1",
            "ts": now_iso(),
            "state": { "attention_queue": ["a"] },
            "degraded": true
        }),
    )
    .expect("write degraded checkpoint");
    let mut index = BTreeMap::new();
    index.insert("s1".to_string(), rel_path(dir.path(), &ckpt_path));
    write_checkpoint_index(dir.path(), &index).expect("write index");

    let err = restore_payload(
        dir.path(),
        &policy,
        &args(&["--session-id=s1", "--apply=0"]),
    )
    .expect_err("blocked");
    assert!(err.contains("degraded_restore_blocked_by_policy"));
}

#[test]
fn vault_encrypts_and_decrypts_state() {
    let dir = root();
    let policy = default_policy();
    std::env::set_var("TEST_CONTINUITY_KEY", "s3cr3t");

    let put = vault_put_payload(
        dir.path(),
        &policy_with_vault_key(&policy, "TEST_CONTINUITY_KEY"),
        &args(&[
            "--session-id=s2",
            "--state-json={\"attention_queue\":[\"a\"],\"memory_graph\":{},\"active_personas\":[]}",
            "--apply=1",
        ]),
    )
    .expect("vault put");
    assert!(put
        .get("encrypted")
        .and_then(Value::as_bool)
        .unwrap_or(false));

    let get = vault_get_payload(
        dir.path(),
        &policy_with_vault_key(&policy, "TEST_CONTINUITY_KEY"),
        &args(&["--session-id=s2", "--emit-state=1"]),
    )
    .expect("vault get");

    assert_eq!(
        get.get("state")
            .and_then(|v| v.get("attention_queue"))
            .and_then(Value::as_array)
            .map(|rows| rows.len()),
        Some(1)
    );

    std::env::remove_var("TEST_CONTINUITY_KEY");
}
