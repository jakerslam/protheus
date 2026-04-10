// SPDX-License-Identifier: Apache-2.0
use super::*;

fn args(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

#[test]
fn plan_and_spawn_echo() {
    let dir = tempfile::tempdir().expect("tempdir");
    let policy = default_policy();
    plan_payload(
        dir.path(),
        &policy,
        &args(&[
            "--organ-id=o1",
            "--budget-json={\"max_runtime_ms\":5000,\"max_output_bytes\":2048,\"allow_commands\":[\"echo\"]}",
            "--apply=1",
        ]),
    )
    .expect("plan");
    let spawn = spawn_payload(
        dir.path(),
        &policy,
        &args(&[
            "--organ-id=o1",
            "--command=echo",
            "--arg=hello",
            "--apply=1",
        ]),
    )
    .expect("spawn");
    assert!(spawn.get("ok").and_then(Value::as_bool).unwrap_or(false));
}

#[test]
fn disallowed_command_fails_closed() {
    let dir = tempfile::tempdir().expect("tempdir");
    let policy = default_policy();
    let err = spawn_payload(
        dir.path(),
        &policy,
        &args(&[
            "--organ-id=o2",
            "--command=definitely_not_allowed",
            "--apply=0",
        ]),
    )
    .expect_err("blocked");
    assert!(err.contains("command_blocked_by_budget_policy"));
}
