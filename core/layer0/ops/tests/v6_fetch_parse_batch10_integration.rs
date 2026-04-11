// SPDX-License-Identifier: Apache-2.0

use protheus_ops_core::{parse_plane, research_plane};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use walkdir::WalkDir;

fn workspace_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .ancestors()
        .nth(3)
        .expect("workspace ancestor")
        .to_path_buf()
}

fn copy_tree(src: &Path, dst: &Path) {
    for entry in WalkDir::new(src).into_iter().filter_map(Result::ok) {
        let rel = entry.path().strip_prefix(src).expect("strip prefix");
        let out = dst.join(rel);
        if entry.file_type().is_dir() {
            fs::create_dir_all(&out).expect("mkdir");
            continue;
        }
        if let Some(parent) = out.parent() {
            fs::create_dir_all(parent).expect("mkdir parent");
        }
        fs::copy(entry.path(), &out).expect("copy file");
    }
}

fn stage_fixture_root() -> TempDir {
    let workspace = workspace_root();
    let tmp = tempfile::tempdir().expect("tempdir");
    copy_tree(
        &workspace.join("planes").join("contracts"),
        &tmp.path().join("planes").join("contracts"),
    );
    tmp
}

fn read_json(path: &Path) -> Value {
    let raw = fs::read_to_string(path).expect("read");
    serde_json::from_str(&raw).expect("parse")
}

fn latest_research_path(root: &Path) -> PathBuf {
    root.join("core")
        .join("local")
        .join("state")
        .join("ops")
        .join("research_plane")
        .join("latest.json")
}

fn latest_parse_path(root: &Path) -> PathBuf {
    root.join("core")
        .join("local")
        .join("state")
        .join("ops")
        .join("parse_plane")
        .join("latest.json")
}

#[test]
fn v6_fetch_parse_batch10_research_and_parse_lanes_harden_edge_cases() {
    let fixture = stage_fixture_root();
    let root = fixture.path();

    let selector_exit = research_plane::run(
        root,
        &[
            "recover-selectors".to_string(),
            "--html=<section class=\"hero card\"><p>hello</p></section>".to_string(),
            "--selectors=.hero".to_string(),
            "--strict=1".to_string(),
        ],
    );
    assert_eq!(selector_exit, 0);
    let selector_latest = read_json(&latest_research_path(root));
    assert_eq!(
        selector_latest
            .get("recovered_selector")
            .and_then(Value::as_str),
        Some(".hero")
    );

    let file_path = root.join("fixtures").join("crawl-seed.html");
    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent).expect("mkdir fixtures");
    }
    fs::write(
        &file_path,
        "<html><head><title>Seed</title></head><body>crawl me</body></html>",
    )
    .expect("write crawl seed");
    let file_url = format!("file://{}", file_path.display());
    let crawl_exit = research_plane::run(
        root,
        &[
            "crawl".to_string(),
            format!("--seed-urls={0},{0}", file_url),
            "--max-pages=2".to_string(),
            "--strict=1".to_string(),
        ],
    );
    assert_eq!(crawl_exit, 0);
    let crawl_latest = read_json(&latest_research_path(root));
    assert_eq!(crawl_latest.get("seed_count").and_then(Value::as_u64), Some(1));
    assert_eq!(
        crawl_latest.get("visited_count").and_then(Value::as_u64),
        Some(1)
    );

    let extract_exit = research_plane::run(
        root,
        &[
            "mcp-extract".to_string(),
            "--strict=1".to_string(),
            "--payload=<html><title>Digest</title><body>beta beta beta alpha alpha gamma</body></html>"
                .to_string(),
            "--source=https://example.com/digest".to_string(),
        ],
    );
    assert_eq!(extract_exit, 0);
    let extract_latest = read_json(&latest_research_path(root));
    let entities = extract_latest
        .get("artifacts")
        .and_then(|v| v.get("entities"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert_eq!(
        entities
            .first()
            .and_then(|v| v.get("token"))
            .and_then(Value::as_str),
        Some("beta")
    );
    assert_eq!(
        entities
            .first()
            .and_then(|v| v.get("count"))
            .and_then(Value::as_u64),
        Some(3)
    );

    let source_path = root.join("fixtures").join("unknown_strategy.txt");
    fs::write(&source_path, "Company: Protheus Labs").expect("write parse source");
    let mapping_path = root.join("fixtures").join("unknown_strategy_mapping.json");
    fs::write(
        &mapping_path,
        "{\n  \"version\": \"v1\",\n  \"kind\": \"mapping_rule_set\",\n  \"rules\": [\n    {\"field\": \"company\", \"strategy\": \"mystery\"}\n  ]\n}\n",
    )
    .expect("write mapping");
    let parse_exit = parse_plane::run(
        root,
        &[
            "parse-doc".to_string(),
            format!("--file={}", source_path.display()),
            format!("--mapping-path={}", mapping_path.display()),
            "--strict=1".to_string(),
        ],
    );
    assert_eq!(parse_exit, 1);
    let parse_latest = read_json(&latest_parse_path(root));
    assert!(parse_latest
        .get("errors")
        .and_then(Value::as_array)
        .map(|rows| rows.iter().any(|row| row.as_str() == Some("unsupported_mapping_strategy:mystery")))
        .unwrap_or(false));

    let fake_table_exit = parse_plane::run(
        root,
        &[
            "postprocess-table".to_string(),
            "--strict=1".to_string(),
            "--table-json=[[\"---\",\"---\"],[\":::\",\"===\"]]".to_string(),
        ],
    );
    assert_eq!(fake_table_exit, 1);
    let fake_table_latest = read_json(&latest_parse_path(root));
    assert!(fake_table_latest
        .get("errors")
        .and_then(Value::as_array)
        .map(|rows| rows.iter().any(|row| row.as_str() == Some("table_empty_after_postprocess")))
        .unwrap_or(false));

    let post_exit = parse_plane::run(
        root,
        &[
            "postprocess-table".to_string(),
            "--strict=1".to_string(),
            "--table-json=[[\"Item\",\"Value\"],[\"Revenue [1]\",\"100\"],[\"\",\"USD\"]]".to_string(),
        ],
    );
    assert_eq!(post_exit, 0);
    let post_latest = read_json(&latest_parse_path(root));
    let post_artifact_path = post_latest
        .get("artifact")
        .and_then(|v| v.get("path"))
        .and_then(Value::as_str)
        .expect("postprocess artifact path")
        .to_string();

    let flatten_exit = parse_plane::run(
        root,
        &[
            "flatten".to_string(),
            "--strict=1".to_string(),
            format!("--from-path={post_artifact_path}"),
        ],
    );
    assert_eq!(flatten_exit, 0);
    let flatten_latest = read_json(&latest_parse_path(root));
    assert_eq!(
        flatten_latest
            .get("result")
            .and_then(|v| v.get("input_hint"))
            .and_then(Value::as_str),
        Some(post_artifact_path.as_str())
    );
    assert!(flatten_latest
        .get("result")
        .and_then(|v| v.get("flattened"))
        .and_then(|v| v.get("root.1"))
        .is_some());
    assert!(flatten_latest
        .get("result")
        .and_then(|v| v.get("metadata"))
        .and_then(|v| v.get("unnested_rows_sha256"))
        .and_then(Value::as_str)
        .is_some());
}
