// SPDX-License-Identifier: Apache-2.0
use protheus_ops_core::{child_organ_runtime, continuity_runtime, memory_plane, runtime_systems};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

fn read_json(path: &std::path::Path) -> Value {
    let raw = fs::read_to_string(path).expect("read json");
    serde_json::from_str(&raw).expect("decode json")
}

fn workspace_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .ancestors()
        .nth(3)
        .expect("workspace ancestor")
        .to_path_buf()
}

fn collect_system_ids(dir: &Path, out: &mut Vec<String>) {
    let Ok(read) = fs::read_dir(dir) else {
        return;
    };
    for entry in read.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_system_ids(&path, out);
            continue;
        }
        if path.extension().and_then(|s| s.to_str()) != Some("ts") {
            continue;
        }
        let Ok(raw) = fs::read_to_string(&path) else {
            continue;
        };
        for line in raw.lines() {
            let l = line.trim();
            if !l.starts_with("const SYSTEM_ID = '") {
                continue;
            }
            if let Some(start) = l.find('\'') {
                if let Some(end) = l[start + 1..].find('\'') {
                    let id = l[start + 1..start + 1 + end].to_string();
                    if !id.is_empty() {
                        out.push(id);
                    }
                }
            }
        }
    }
}

#[test]
fn continuity_runtime_end_to_end_writes_checkpoint_and_vault() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();
    std::env::set_var("PROTHEUS_CONTINUITY_VAULT_KEY", "integration-secret");

    let checkpoint_exit = continuity_runtime::run(
        root,
        &[
            "resurrection-protocol".to_string(),
            "checkpoint".to_string(),
            "--session-id=intg".to_string(),
            "--state-json={\"attention_queue\":[\"a\"],\"memory_graph\":{\"n1\":{}},\"active_personas\":[\"planner\"]}".to_string(),
            "--apply=1".to_string(),
        ],
    );
    assert_eq!(checkpoint_exit, 0);

    let restore_exit = continuity_runtime::run(
        root,
        &[
            "resurrection-protocol".to_string(),
            "restore".to_string(),
            "--session-id=intg".to_string(),
            "--apply=1".to_string(),
        ],
    );
    assert_eq!(restore_exit, 0);

    let vault_put_exit = continuity_runtime::run(
        root,
        &[
            "session-continuity-vault".to_string(),
            "put".to_string(),
            "--session-id=intg".to_string(),
            "--state-json={\"attention_queue\":[\"a\"],\"memory_graph\":{},\"active_personas\":[]}"
                .to_string(),
            "--apply=1".to_string(),
        ],
    );
    assert_eq!(vault_put_exit, 0);

    let checkpoint_index = root
        .join("client")
        .join("local")
        .join("state")
        .join("continuity")
        .join("checkpoint_index.json");
    assert!(checkpoint_index.exists());

    let restored = root
        .join("client")
        .join("local")
        .join("state")
        .join("continuity")
        .join("restored")
        .join("latest.json");
    assert!(restored.exists());

    let vault_file = root
        .join("core")
        .join("local")
        .join("state")
        .join("continuity")
        .join("vault")
        .join("intg.json");
    assert!(vault_file.exists());
    let vault = read_json(&vault_file);
    assert!(vault.get("ciphertext_b64").is_some());

    std::env::remove_var("PROTHEUS_CONTINUITY_VAULT_KEY");
}

#[test]
fn memory_plane_end_to_end_writes_graph_and_federation_state() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();

    let e1 = memory_plane::run(
        root,
        &[
            "causal-temporal-graph".to_string(),
            "record".to_string(),
            "--event-id=e1".to_string(),
            "--summary=root".to_string(),
            "--actor=planner".to_string(),
            "--apply=1".to_string(),
        ],
    );
    assert_eq!(e1, 0);
    let e2 = memory_plane::run(
        root,
        &[
            "causal-temporal-graph".to_string(),
            "record".to_string(),
            "--event-id=e2".to_string(),
            "--summary=child".to_string(),
            "--actor=executor".to_string(),
            "--caused-by=e1".to_string(),
            "--apply=1".to_string(),
        ],
    );
    assert_eq!(e2, 0);
    let blame = memory_plane::run(
        root,
        &[
            "causal-temporal-graph".to_string(),
            "blame".to_string(),
            "--event-id=e2".to_string(),
        ],
    );
    assert_eq!(blame, 0);

    let sync = memory_plane::run(
        root,
        &[
            "memory-federation-plane".to_string(),
            "sync".to_string(),
            "--device-id=d1".to_string(),
            "--entries-json=[{\"key\":\"k1\",\"value\":{\"v\":1},\"counter\":1}]".to_string(),
            "--apply=1".to_string(),
        ],
    );
    assert_eq!(sync, 0);

    let graph_path = root
        .join("client")
        .join("local")
        .join("state")
        .join("memory")
        .join("causal_temporal_graph")
        .join("latest.json");
    assert!(graph_path.exists());
    let graph = read_json(&graph_path);
    assert_eq!(
        graph
            .get("nodes")
            .and_then(Value::as_object)
            .map(|m| m.len())
            .unwrap_or(0),
        2
    );

    let federation_state = root
        .join("client")
        .join("local")
        .join("state")
        .join("memory")
        .join("federation")
        .join("state.json");
    assert!(federation_state.exists());
    let fed = read_json(&federation_state);
    assert_eq!(
        fed.get("entries")
            .and_then(Value::as_object)
            .map(|m| m.len())
            .unwrap_or(0),
        1
    );
}

#[test]
fn child_organ_runtime_end_to_end_plans_and_spawns() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();

    let plan = child_organ_runtime::run(
        root,
        &[
            "plan".to_string(),
            "--organ-id=child-a".to_string(),
            "--budget-json={\"max_runtime_ms\":4000,\"max_output_bytes\":4096,\"allow_commands\":[\"echo\"]}".to_string(),
            "--apply=1".to_string(),
        ],
    );
    assert_eq!(plan, 0);

    let spawn = child_organ_runtime::run(
        root,
        &[
            "spawn".to_string(),
            "--organ-id=child-a".to_string(),
            "--command=echo".to_string(),
            "--arg=hello".to_string(),
            "--apply=1".to_string(),
        ],
    );
    assert_eq!(spawn, 0);

    let plans = root
        .join("client")
        .join("local")
        .join("state")
        .join("fractal")
        .join("child_organ_runtime")
        .join("plans.json");
    assert!(plans.exists());

    let runs = root
        .join("client")
        .join("local")
        .join("state")
        .join("fractal")
        .join("child_organ_runtime")
        .join("runs");
    let count = fs::read_dir(runs)
        .expect("runs dir")
        .flatten()
        .filter(|entry| entry.path().extension().and_then(|s| s.to_str()) == Some("json"))
        .count();
    assert!(count >= 1);
}

#[test]
fn all_migrated_runtime_system_wrappers_have_executable_core_lane() {
    let workspace = workspace_root();
    let systems = workspace.join("client").join("runtime").join("systems");
    let mut ids = Vec::new();
    collect_system_ids(&systems, &mut ids);
    ids.sort();
    ids.dedup();
    assert!(
        !ids.is_empty(),
        "expected migrated runtime system wrappers with SYSTEM_ID constants"
    );

    let root = tempfile::tempdir().expect("tempdir");
    for id in ids {
        let status_exit = runtime_systems::run(
            root.path(),
            &["status".to_string(), format!("--system-id={id}")],
        );
        assert_eq!(status_exit, 0, "status should run for {id}");
        let run_exit = runtime_systems::run(
            root.path(),
            &[
                "run".to_string(),
                format!("--system-id={id}"),
                "--apply=0".to_string(),
            ],
        );
        assert_eq!(run_exit, 0, "run should execute for {id}");
    }
}
