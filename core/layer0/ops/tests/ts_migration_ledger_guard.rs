// SPDX-License-Identifier: Apache-2.0

use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn workspace_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .ancestors()
        .nth(3)
        .expect("workspace ancestor")
        .to_path_buf()
}

fn tracked_ts_files(root: &Path) -> BTreeSet<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["ls-files", "*.ts", "*.tsx"])
        .output()
        .expect("run git ls-files");
    assert!(
        output.status.success(),
        "git ls-files failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|row| !row.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn authority_prefixes() -> [&'static str; 6] {
    [
        "client/runtime/systems/security/",
        "client/runtime/systems/ops/",
        "client/runtime/systems/memory/",
        "client/runtime/systems/sensory/",
        "client/runtime/systems/autonomy/",
        "client/runtime/systems/assimilation/",
    ]
}

fn authority_ts_files(root: &Path) -> Vec<String> {
    let tracked = tracked_ts_files(root);
    tracked
        .into_iter()
        .filter(|path| authority_prefixes().iter().any(|prefix| path.starts_with(prefix)))
        .collect()
}

fn rust_ts_exceptions_config(root: &Path) -> (Vec<String>, BTreeMap<String, String>) {
    let path = root
        .join("client")
        .join("runtime")
        .join("config")
        .join("rust_ts_exceptions.json");
    let raw = fs::read_to_string(path).expect("read rust_ts_exceptions.json");
    let parsed: Value = serde_json::from_str(&raw).expect("parse rust_ts_exceptions.json");

    let wrapper_tokens = parsed
        .get("wrapper_tokens")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| row.as_str().map(str::trim).map(ToString::to_string))
        .filter(|row| !row.is_empty())
        .collect::<Vec<String>>();

    let mut exceptions = BTreeMap::new();
    for row in parsed
        .get("exceptions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
    {
        let path = row
            .get("path")
            .and_then(Value::as_str)
            .map(str::trim)
            .unwrap_or_default();
        let reason = row
            .get("reason")
            .and_then(Value::as_str)
            .map(str::trim)
            .unwrap_or_default();
        if !path.is_empty() {
            exceptions.insert(path.to_string(), reason.to_string());
        }
    }

    (wrapper_tokens, exceptions)
}

fn has_wrapper_token(content: &str, tokens: &[String]) -> bool {
    tokens.iter().any(|token| !token.is_empty() && content.contains(token))
}

#[test]
fn authority_prefix_ts_count_is_bounded() {
    let root = workspace_root();
    let rows = authority_ts_files(&root);
    // Bound growth in Rust-authoritative prefixes.
    assert!(
        rows.len() <= 122,
        "authority-prefix TS count grew unexpectedly: {} > 122",
        rows.len()
    );
}

#[test]
fn authority_prefix_ts_require_wrapper_tokens_or_explicit_exception_reason() {
    let root = workspace_root();
    let rows = authority_ts_files(&root);
    let (wrapper_tokens, exceptions) = rust_ts_exceptions_config(&root);
    let mut violations = Vec::new();

    for rel in &rows {
        if rel.contains("/tests/") {
            continue;
        }
        let abs = root.join(rel);
        let raw = fs::read_to_string(&abs).unwrap_or_default();
        let has_wrapper = has_wrapper_token(&raw, &wrapper_tokens);
        let has_exception = exceptions.get(rel).is_some_and(|reason| reason.trim().len() >= 12);
        if !has_wrapper && !has_exception {
            violations.push(rel.clone());
        }
    }

    assert!(
        violations.is_empty(),
        "non-wrapper authority-prefix TS must be exception-listed with reason: {:?}",
        &violations[..violations.len().min(20)]
    );
}

#[test]
fn rust_ts_exception_manifest_paths_are_tracked_and_authority_scoped() {
    let root = workspace_root();
    let tracked = tracked_ts_files(&root);
    let (_wrapper_tokens, exceptions) = rust_ts_exceptions_config(&root);
    let mut missing = Vec::new();
    let mut out_of_scope = Vec::new();
    let mut weak_reason = Vec::new();

    for (path, reason) in &exceptions {
        if !tracked.contains(path) {
            missing.push(path.clone());
        }
        if !authority_prefixes()
            .iter()
            .any(|prefix| path.starts_with(prefix))
        {
            out_of_scope.push(path.clone());
        }
        if reason.trim().len() < 12 {
            weak_reason.push(path.clone());
        }
    }

    assert!(
        missing.is_empty(),
        "rust_ts_exceptions contains missing/untracked paths: {:?}",
        &missing[..missing.len().min(20)]
    );
    assert!(
        out_of_scope.is_empty(),
        "rust_ts_exceptions must only list authority-prefix paths: {:?}",
        &out_of_scope[..out_of_scope.len().min(20)]
    );
    assert!(
        weak_reason.is_empty(),
        "rust_ts_exceptions requires non-trivial reason text: {:?}",
        &weak_reason[..weak_reason.len().min(20)]
    );
}
