// SPDX-License-Identifier: Apache-2.0

use protheus_ops_core::{binary_vuln_plane, hermes_plane};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
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

fn latest_path(root: &Path) -> PathBuf {
    root.join("core")
        .join("local")
        .join("state")
        .join("ops")
        .join("binary_vuln_plane")
        .join("latest.json")
}

fn hermes_latest_path(root: &Path) -> PathBuf {
    root.join("core")
        .join("local")
        .join("state")
        .join("ops")
        .join("hermes_plane")
        .join("latest.json")
}

fn read_json(path: &Path) -> Value {
    let raw = fs::read_to_string(path).expect("read");
    serde_json::from_str(&raw).expect("parse")
}

fn canonicalize_json(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut keys = map.keys().cloned().collect::<Vec<_>>();
            keys.sort();
            let mut out = Map::new();
            for key in keys {
                if let Some(v) = map.get(&key) {
                    out.insert(key, canonicalize_json(v));
                }
            }
            Value::Object(out)
        }
        Value::Array(rows) => Value::Array(rows.iter().map(canonicalize_json).collect()),
        _ => value.clone(),
    }
}

fn canonical_json_string(value: &Value) -> String {
    serde_json::to_string(&canonicalize_json(value)).expect("canonical json")
}

fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}

fn assert_claim(payload: &Value, claim_id: &str) {
    let has = payload
        .get("claim_evidence")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .any(|row| row.get("id").and_then(Value::as_str) == Some(claim_id));
    assert!(has, "missing claim evidence id={claim_id}");
}

#[test]
fn v6_binvuln_batch11_core_lanes_execute_with_receipts() {
    let fixture = stage_fixture_root();
    let root = fixture.path();

    let sample = root.join("tmp").join("firmware.bin");
    fs::create_dir_all(sample.parent().expect("parent")).expect("mkdir");
    fs::write(
        &sample,
        b"firmware-start\npassword=supersecret\n/bin/sh\nhttp://example.local\n",
    )
    .expect("write sample");

    let scan_exit = binary_vuln_plane::run(
        root,
        &[
            "scan".to_string(),
            "--strict=1".to_string(),
            format!("--input={}", sample.display()),
            "--format=json".to_string(),
        ],
    );
    assert_eq!(scan_exit, 0);
    let scan_latest = read_json(&latest_path(root));
    assert_eq!(
        scan_latest.get("type").and_then(Value::as_str),
        Some("binary_vuln_plane_scan")
    );
    assert_eq!(scan_latest.get("ok").and_then(Value::as_bool), Some(true));
    assert!(
        scan_latest
            .get("output")
            .and_then(|v| v.get("finding_count"))
            .and_then(Value::as_u64)
            .unwrap_or(0)
            > 0
    );
    assert!(
        scan_latest
            .get("input")
            .and_then(|v| v.get("path_redacted"))
            .and_then(Value::as_bool)
            .unwrap_or(false),
        "strict scan should redact input paths by default"
    );
    assert_claim(&scan_latest, "V6-BINVULN-001.1");
    assert_claim(&scan_latest, "V6-BINVULN-001.3");
    assert_claim(&scan_latest, "V6-BINVULN-001.4");
    assert_claim(&scan_latest, "V6-BINVULN-001.6");

    let jsonl_exit = binary_vuln_plane::run(
        root,
        &[
            "scan".to_string(),
            "--strict=1".to_string(),
            format!("--input={}", sample.display()),
            "--format=jsonl".to_string(),
        ],
    );
    assert_eq!(jsonl_exit, 0);
    let jsonl_latest = read_json(&latest_path(root));
    assert_eq!(
        jsonl_latest
            .get("output")
            .and_then(|v| v.get("format"))
            .and_then(Value::as_str),
        Some("jsonl")
    );
    assert_claim(&jsonl_latest, "V6-BINVULN-001.3");

    let mcp_exit = binary_vuln_plane::run(
        root,
        &[
            "mcp-analyze".to_string(),
            "--strict=1".to_string(),
            format!("--input={}", sample.display()),
            "--transport=stdio".to_string(),
        ],
    );
    assert_eq!(mcp_exit, 0);
    let mcp_latest = read_json(&latest_path(root));
    assert_eq!(
        mcp_latest.get("type").and_then(Value::as_str),
        Some("binary_vuln_plane_mcp_analyze")
    );
    assert_eq!(mcp_latest.get("ok").and_then(Value::as_bool), Some(true));
    assert_claim(&mcp_latest, "V6-BINVULN-001.2");
    assert_claim(&mcp_latest, "V6-BINVULN-001.4");
}

#[test]
fn v6_binvuln_batch11_rejects_bypass_when_strict() {
    let fixture = stage_fixture_root();
    let root = fixture.path();
    let exit = binary_vuln_plane::run(
        root,
        &[
            "scan".to_string(),
            "--strict=1".to_string(),
            "--bypass=1".to_string(),
            "--input=missing.bin".to_string(),
        ],
    );
    assert_eq!(exit, 1);
    let latest = read_json(&latest_path(root));
    assert_eq!(
        latest.get("type").and_then(Value::as_str),
        Some("binary_vuln_plane_conduit_gate")
    );
    assert!(
        latest
            .get("conduit_enforcement")
            .and_then(|v| v.get("claim_evidence"))
            .and_then(Value::as_array)
            .map(|rows| rows
                .iter()
                .any(|row| row.get("id").and_then(Value::as_str) == Some("V6-BINVULN-001.4")))
            .unwrap_or(false),
        "conduit bypass rejection should emit sandbox safety claim evidence"
    );
}

#[test]
fn v6_binvuln_batch30_rulepack_install_enable_and_cockpit_observability() {
    let fixture = stage_fixture_root();
    let root = fixture.path();

    let provenance = "community://rulepack/test-batch30";
    let unsigned = serde_json::json!({
        "version": "v1",
        "kind": "binary_vuln_rulepack",
        "metadata": {
            "provenance": provenance
        },
        "rules": [
            {
                "id": "batch30_custom_pattern",
                "title": "Batch30 custom marker",
                "pattern": "batch30-danger",
                "severity": "high",
                "confidence": 0.93,
                "policy_labels": ["custom", "community"]
            }
        ]
    });
    let payload_digest = sha256_hex(&canonical_json_string(&unsigned));
    let signature = format!("sig:{}", sha256_hex(&format!("{provenance}:{payload_digest}")));
    let mut signed = unsigned.clone();
    signed["metadata"]["signature"] = Value::String(signature);

    let custom_rulepack = root.join("tmp").join("batch30_rulepack.json");
    fs::create_dir_all(custom_rulepack.parent().expect("parent")).expect("mkdir");
    fs::write(
        &custom_rulepack,
        serde_json::to_vec_pretty(&signed).expect("serialize"),
    )
    .expect("write rulepack");

    let install_exit = binary_vuln_plane::run(
        root,
        &[
            "rulepack-install".to_string(),
            "--strict=1".to_string(),
            format!("--rulepack={}", custom_rulepack.display()),
            "--name=batch30-community".to_string(),
            "--enable=1".to_string(),
        ],
    );
    assert_eq!(install_exit, 0);
    let install_latest = read_json(&latest_path(root));
    assert_eq!(
        install_latest.get("type").and_then(Value::as_str),
        Some("binary_vuln_plane_rulepack_install")
    );
    assert_eq!(install_latest.get("ok").and_then(Value::as_bool), Some(true));
    assert!(
        install_latest
            .get("rulepack")
            .and_then(|v| v.get("active_written"))
            .and_then(Value::as_bool)
            .unwrap_or(false),
        "rulepack install should activate by default for strict batch30 lane"
    );
    assert_claim(&install_latest, "V6-BINVULN-001.5");

    let sample = root.join("tmp").join("batch30-fw.bin");
    fs::write(&sample, b"header\nbatch30-danger\nfooter\n").expect("write sample");
    let scan_exit = binary_vuln_plane::run(
        root,
        &[
            "scan".to_string(),
            "--strict=1".to_string(),
            "--dx-source=scan-binary".to_string(),
            format!("--input={}", sample.display()),
        ],
    );
    assert_eq!(scan_exit, 0);
    let scan_latest = read_json(&latest_path(root));
    assert_claim(&scan_latest, "V6-BINVULN-001.6");
    assert!(
        scan_latest
            .get("findings")
            .and_then(Value::as_array)
            .map(|rows| {
                rows.iter()
                    .any(|row| row.get("id").and_then(Value::as_str) == Some("batch30_custom_pattern"))
            })
            .unwrap_or(false),
        "active custom rulepack should drive scan findings"
    );

    let cockpit_exit = hermes_plane::run(
        root,
        &["cockpit".to_string(), "--strict=1".to_string(), "--max-blocks=16".to_string()],
    );
    assert_eq!(cockpit_exit, 0);
    let cockpit_latest = read_json(&hermes_latest_path(root));
    assert!(
        cockpit_latest
            .get("cockpit")
            .and_then(|v| v.get("render"))
            .and_then(|v| v.get("stream_blocks"))
            .and_then(Value::as_array)
            .map(|rows| {
                rows.iter().any(|row| {
                    row.get("lane").and_then(Value::as_str) == Some("binary_vuln_plane")
                        && row.get("tool_call_class").and_then(Value::as_str) == Some("security")
                })
            })
            .unwrap_or(false),
        "protheus-top cockpit should surface binary-vuln scans as security blocks"
    );
}

#[test]
fn v6_binvuln_batch30_rejects_unsigned_rulepack_install_in_strict_mode() {
    let fixture = stage_fixture_root();
    let root = fixture.path();
    let unsigned_rulepack = root.join("tmp").join("unsigned_rulepack.json");
    fs::create_dir_all(unsigned_rulepack.parent().expect("parent")).expect("mkdir");
    fs::write(
        &unsigned_rulepack,
        serde_json::to_vec_pretty(&serde_json::json!({
            "version": "v1",
            "kind": "binary_vuln_rulepack",
            "metadata": {
                "provenance": "community://unsigned"
            },
            "rules": [
                {
                    "id": "unsigned_rule",
                    "title": "unsigned",
                    "pattern": "unsigned",
                    "severity": "low"
                }
            ]
        }))
        .expect("serialize"),
    )
    .expect("write");

    let exit = binary_vuln_plane::run(
        root,
        &[
            "rulepack-install".to_string(),
            "--strict=1".to_string(),
            format!("--rulepack={}", unsigned_rulepack.display()),
            "--name=unsigned".to_string(),
        ],
    );
    assert_eq!(exit, 1);
    let latest = read_json(&latest_path(root));
    assert_eq!(
        latest.get("type").and_then(Value::as_str),
        Some("binary_vuln_plane_rulepack_install")
    );
    assert!(
        latest
            .get("errors")
            .and_then(Value::as_array)
            .map(|rows| {
                rows.iter()
                    .filter_map(Value::as_str)
                    .any(|err| err == "rulepack_signature_required")
            })
            .unwrap_or(false),
        "strict rulepack intake must reject unsigned payloads"
    );
}
