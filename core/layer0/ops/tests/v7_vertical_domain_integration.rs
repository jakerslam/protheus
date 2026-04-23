// SPDX-License-Identifier: Apache-2.0

use infring_ops_core::vertical_plane;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

fn latest_path(root: &Path) -> PathBuf {
    root.join("core")
        .join("local")
        .join("state")
        .join("ops")
        .join("vertical_plane")
        .join("latest.json")
}

fn read_json(path: &Path) -> Value {
    let raw = fs::read_to_string(path).expect("read");
    serde_json::from_str(&raw).expect("parse")
}

fn assert_claim(payload: &Value, id: &str) {
    assert_no_runtime_context_leak(&payload.to_string());
    let ok = payload
        .get("claim_evidence")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .any(|row| row.get("id").and_then(Value::as_str) == Some(id));
    assert!(ok, "missing claim {id}");
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

#[test]
fn v7_vertical_001_1_to_001_14_runtime_contracts_proven() {
    let root = tempfile::tempdir().expect("tempdir");
    let root_path = root.path();

    let domains = [
        ("industrial", "V7-VERTICAL-001.1"),
        ("grid", "V7-VERTICAL-001.2"),
        ("avionics", "V7-VERTICAL-001.3"),
        ("automotive", "V7-VERTICAL-001.4"),
        ("telecom", "V7-VERTICAL-001.5"),
        ("retail", "V7-VERTICAL-001.6"),
        ("education", "V7-VERTICAL-001.7"),
        ("legal", "V7-VERTICAL-001.8"),
        ("gaming", "V7-VERTICAL-001.9"),
        ("agriculture", "V7-VERTICAL-001.10"),
        ("construction", "V7-VERTICAL-001.11"),
        ("logistics", "V7-VERTICAL-001.12"),
        ("pharma", "V7-VERTICAL-001.13"),
    ];

    for (domain, claim) in domains {
        let exit = vertical_plane::run(
            root_path,
            &[
                "activate".to_string(),
                format!("--domain={domain}"),
                "--strict=1".to_string(),
            ],
        );
        assert_eq!(exit, 0, "activation failed for {domain}");
        let latest = read_json(&latest_path(root_path));
        assert_claim(&latest, claim);
    }

    let compile_exit = vertical_plane::run(
        root_path,
        &[
            "compile-profile".to_string(),
            "--domain=custom-domain".to_string(),
            "--profile-json={\"entity_model\":{\"primary\":\"x\"},\"compliance_mapping\":[\"A\"],\"protocols\":[\"p1\"],\"safety_class\":\"S1\",\"realtime_slo\":\"100ms\"}".to_string(),
            "--strict=1".to_string(),
        ],
    );
    assert_eq!(compile_exit, 0);
    let compile_latest = read_json(&latest_path(root_path));
    assert_claim(&compile_latest, "V7-VERTICAL-001.14");

    let bypass_exit = vertical_plane::run(
        root_path,
        &[
            "status".to_string(),
            "--strict=1".to_string(),
            "--bypass=1".to_string(),
        ],
    );
    assert_eq!(bypass_exit, 1, "bypass must fail closed");
}
