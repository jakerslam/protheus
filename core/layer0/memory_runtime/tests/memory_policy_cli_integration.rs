use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::Command;

fn write_daily_node(root: &Path, day: &str, node_id: &str, tags: &str) {
    let memory_dir = root.join("memory");
    let client_memory_dir = root.join("client/memory");
    fs::create_dir_all(&memory_dir).expect("create memory dir");
    fs::create_dir_all(&client_memory_dir).expect("create client memory dir");
    let body = format!(
        r#"<!-- NODE -->
node_id: {node_id}
uid: UID{node_id}
tags: [{tags}]
# {node_id} summary
body
"#
    );
    let file = format!("{day}.md");
    fs::write(memory_dir.join(&file), &body).expect("write daily node");
    fs::write(client_memory_dir.join(&file), body).expect("write mirrored client node");
}

fn run_cli(args: &[&str]) -> Value {
    let bin = env!("CARGO_BIN_EXE_protheus-memory-core");
    let output = Command::new(bin)
        .args(args)
        .output()
        .expect("run memory core");
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    serde_json::from_str::<Value>(stdout.trim()).expect("json stdout")
}

#[test]
fn cli_query_rejects_over_budget_when_fail_closed() {
    let tmp = tempfile::tempdir().expect("tempdir");
    write_daily_node(tmp.path(), "2026-03-01", "node.alpha", "memory,policy");
    let out = run_cli(&[
        "query-index",
        &format!("--root={}", tmp.path().to_string_lossy()),
        "--q=memory",
        "--top=999",
        "--budget-mode=reject",
    ]);
    assert_eq!(out["ok"], false);
    assert_eq!(out["reason_code"], "recall_budget_exceeded");
}

#[test]
fn cli_get_node_is_node_scoped_and_fail_closed() {
    let tmp = tempfile::tempdir().expect("tempdir");
    write_daily_node(tmp.path(), "2026-03-01", "node.alpha", "memory,policy");
    let out = run_cli(&[
        "get-node",
        &format!("--root={}", tmp.path().to_string_lossy()),
        "--node-id=node.alpha",
    ]);
    assert_eq!(out["ok"], true);
    assert_eq!(out["node_id"], "node.alpha");

    let missing = run_cli(&[
        "get-node",
        &format!("--root={}", tmp.path().to_string_lossy()),
    ]);
    assert_eq!(missing["ok"], false);
    assert_eq!(missing["error"], "missing_node_or_uid");
}
