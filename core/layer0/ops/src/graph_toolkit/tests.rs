// SPDX-License-Identifier: Apache-2.0

use super::{receipts::latest_path, run};
use crate::directive_kernel;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_root(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("protheus_graph_toolkit_{name}_{nonce}"));
    std::fs::create_dir_all(&root).expect("mkdir");
    root
}

fn env_guard() -> std::sync::MutexGuard<'static, ()> {
    crate::test_env_guard()
}

fn set_signing_key() {
    std::env::set_var("DIRECTIVE_KERNEL_SIGNING_KEY", "graph-test-signing");
}

fn cleanup(root: PathBuf) {
    std::env::remove_var("DIRECTIVE_KERNEL_SIGNING_KEY");
    let _ = std::fs::remove_dir_all(root);
}

fn allow_graph(root: &Path, pattern: &str) {
    set_signing_key();
    let exit = directive_kernel::run(
        root,
        &[
            "prime-sign".to_string(),
            format!("--directive=allow:{pattern}"),
            "--signer=tester".to_string(),
        ],
    );
    assert_eq!(exit, 0);
}

fn graph_b64() -> String {
    let payload = json!({
        "directed": false,
        "nodes": [{"id":"a"},{"id":"b"},{"id":"c"},{"id":"d"},{"id":"e"},{"id":"f"}],
        "edges": [
            {"from":"a","to":"b"},
            {"from":"b","to":"c"},
            {"from":"a","to":"c"},
            {"from":"d","to":"e"},
            {"from":"e","to":"f"},
            {"from":"d","to":"f"},
            {"from":"c","to":"d","weight":0.2}
        ]
    });
    BASE64_STANDARD.encode(serde_json::to_string(&payload).expect("encode"))
}

fn latest(root: &Path) -> Value {
    let raw = std::fs::read_to_string(latest_path(root)).expect("latest");
    serde_json::from_str(&raw).expect("json")
}

#[test]
fn pagerank_materializes_and_caches() {
    let _guard = env_guard();
    let root = temp_root("pagerank");
    allow_graph(&root, "graph:pagerank");
    let encoded = graph_b64();
    let args = vec![
        "pagerank".to_string(),
        format!("--graph-json-base64={encoded}"),
        "--iterations=16".to_string(),
    ];
    assert_eq!(run(&root, &args), 0);
    let first = latest(&root);
    assert_eq!(
        first.get("type").and_then(Value::as_str),
        Some("graph_toolkit_pagerank")
    );
    assert_eq!(first.get("cached").and_then(Value::as_bool), Some(false));

    assert_eq!(run(&root, &args), 0);
    let second = latest(&root);
    assert_eq!(second.get("cached").and_then(Value::as_bool), Some(true));
    cleanup(root);
}

#[test]
fn louvain_and_label_propagation_build_communities() {
    let _guard = env_guard();
    let root = temp_root("communities");
    allow_graph(&root, "graph:louvain");
    allow_graph(&root, "graph:label-propagation");
    let encoded = graph_b64();

    assert_eq!(
        run(
            &root,
            &[
                "louvain".to_string(),
                format!("--graph-json-base64={encoded}"),
                "--max-iter=16".to_string()
            ]
        ),
        0
    );
    let louvain = latest(&root);
    let communities = louvain
        .get("result")
        .and_then(|v| v.get("communities"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert!(communities.len() >= 2);

    assert_eq!(
        run(
            &root,
            &[
                "label-propagation".to_string(),
                format!("--graph-json-base64={encoded}"),
                "--max-iter=16".to_string()
            ]
        ),
        0
    );
    let lp = latest(&root);
    let groups = lp
        .get("result")
        .and_then(|v| v.get("groups"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert!(!groups.is_empty());
    cleanup(root);
}

#[test]
fn jaccard_betweenness_and_link_prediction_emit_expected_shapes() {
    let _guard = env_guard();
    let root = temp_root("similarity");
    allow_graph(&root, "graph:jaccard");
    allow_graph(&root, "graph:betweenness");
    allow_graph(&root, "graph:predict-links");
    let encoded = graph_b64();

    assert_eq!(
        run(
            &root,
            &[
                "jaccard".to_string(),
                format!("--graph-json-base64={encoded}"),
                "--top-k=5".to_string()
            ]
        ),
        0
    );
    let jaccard = latest(&root);
    assert!(jaccard
        .get("result")
        .and_then(|v| v.get("pairs"))
        .and_then(Value::as_array)
        .map(|rows| !rows.is_empty())
        .unwrap_or(false));

    assert_eq!(
        run(
            &root,
            &[
                "betweenness".to_string(),
                format!("--graph-json-base64={encoded}"),
                "--normalize=1".to_string()
            ]
        ),
        0
    );
    let betweenness = latest(&root);
    assert!(betweenness
        .get("result")
        .and_then(|v| v.get("scores"))
        .and_then(Value::as_array)
        .map(|rows| rows.len() == 6)
        .unwrap_or(false));

    assert_eq!(
        run(
            &root,
            &[
                "predict-links".to_string(),
                format!("--graph-json-base64={encoded}"),
                "--top-k=8".to_string()
            ]
        ),
        0
    );
    let predicted = latest(&root);
    assert!(predicted
        .get("result")
        .and_then(|v| v.get("predictions"))
        .and_then(Value::as_array)
        .map(|rows| !rows.is_empty())
        .unwrap_or(false));
    cleanup(root);
}

#[test]
fn command_fails_closed_without_directive() {
    let _guard = env_guard();
    let root = temp_root("fail_closed");
    let encoded = graph_b64();
    let exit = run(
        &root,
        &[
            "pagerank".to_string(),
            format!("--graph-json-base64={encoded}"),
            "--iterations=8".to_string(),
        ],
    );
    assert_eq!(exit, 2);
    let receipt = latest(&root);
    assert_eq!(
        receipt.get("error").and_then(Value::as_str),
        Some("directive_gate_denied")
    );
    cleanup(root);
}
