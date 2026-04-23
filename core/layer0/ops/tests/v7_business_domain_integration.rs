// SPDX-License-Identifier: Apache-2.0

use infring_ops_core::business_plane;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

fn latest_path(root: &Path) -> PathBuf {
    root.join("core")
        .join("local")
        .join("state")
        .join("ops")
        .join("business_plane")
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
fn v7_business_001_1_to_001_8_runtime_contracts_proven() {
    let root = tempfile::tempdir().expect("tempdir");
    let root_path = root.path();

    let taxonomy_exit = business_plane::run(
        root_path,
        &[
            "taxonomy".to_string(),
            "--business-context=SUB_A".to_string(),
            "--topic=q1_strategy".to_string(),
            "--tier=tag2".to_string(),
            "--interaction-count=40".to_string(),
            "--strict=1".to_string(),
        ],
    );
    assert_eq!(taxonomy_exit, 0);
    let taxonomy_latest = read_json(&latest_path(root_path));
    assert_eq!(
        taxonomy_latest.get("type").and_then(Value::as_str),
        Some("business_plane_taxonomy")
    );
    assert_claim(&taxonomy_latest, "V7-BUSINESS-001.1");

    let persona_issue = business_plane::run(
        root_path,
        &[
            "persona".to_string(),
            "--op=issue".to_string(),
            "--persona=shadow-alpha".to_string(),
            "--business-context=SUB_A".to_string(),
            "--lease-hours=24".to_string(),
            "--strict=1".to_string(),
        ],
    );
    assert_eq!(persona_issue, 0);
    let persona_latest = read_json(&latest_path(root_path));
    assert_claim(&persona_latest, "V7-BUSINESS-001.2");

    let checkpoint_exit = business_plane::run(
        root_path,
        &[
            "continuity".to_string(),
            "--op=checkpoint".to_string(),
            "--business-context=SUB_A".to_string(),
            "--name=Q1_Deal_Review".to_string(),
            "--state-json={\"stage\":\"review\",\"approvals\":2}".to_string(),
            "--strict=1".to_string(),
        ],
    );
    assert_eq!(checkpoint_exit, 0);
    let resume_exit = business_plane::run(
        root_path,
        &[
            "continuity".to_string(),
            "--op=resume".to_string(),
            "--business-context=SUB_A".to_string(),
            "--name=Q1_Deal_Review".to_string(),
            "--strict=1".to_string(),
        ],
    );
    assert_eq!(resume_exit, 0);
    let handoff_exit = business_plane::run(
        root_path,
        &[
            "continuity".to_string(),
            "--op=handoff".to_string(),
            "--business-context=SUB_A".to_string(),
            "--to=stakeholder".to_string(),
            "--task=pending_approvals".to_string(),
            "--strict=1".to_string(),
        ],
    );
    assert_eq!(handoff_exit, 0);
    let continuity_latest = read_json(&latest_path(root_path));
    assert_claim(&continuity_latest, "V7-BUSINESS-001.3");

    let alert_emit = business_plane::run(
        root_path,
        &[
            "alerts".to_string(),
            "--op=emit".to_string(),
            "--alert-type=decision-required".to_string(),
            "--channel=slack".to_string(),
            "--strict=1".to_string(),
        ],
    );
    assert_eq!(alert_emit, 0);
    let alert_latest = read_json(&latest_path(root_path));
    assert_claim(&alert_latest, "V7-BUSINESS-001.4");

    let switch_create = business_plane::run(
        root_path,
        &[
            "switchboard".to_string(),
            "--op=create".to_string(),
            "--business-context=SUB_A".to_string(),
            "--strict=1".to_string(),
        ],
    );
    assert_eq!(switch_create, 0);
    let switch_write = business_plane::run(
        root_path,
        &[
            "switchboard".to_string(),
            "--op=write".to_string(),
            "--business-context=SUB_A".to_string(),
            "--entry-json={\"memo\":\"board note\"}".to_string(),
            "--strict=1".to_string(),
        ],
    );
    assert_eq!(switch_write, 0);
    let switch_cross = business_plane::run(
        root_path,
        &[
            "switchboard".to_string(),
            "--op=read".to_string(),
            "--business-context=SUB_A".to_string(),
            "--target-business=SUB_B".to_string(),
            "--strict=1".to_string(),
        ],
    );
    assert_eq!(switch_cross, 1, "cross business read must fail closed");
    let switch_latest = read_json(&latest_path(root_path));
    assert_claim(&switch_latest, "V7-BUSINESS-001.5");

    let sync_exit = business_plane::run(
        root_path,
        &[
            "external-sync".to_string(),
            "--business-context=SUB_A".to_string(),
            "--system=notion".to_string(),
            "--direction=bidirectional".to_string(),
            "--external-id=page_123".to_string(),
            "--content-json={\"title\":\"Q1\"}".to_string(),
            "--strict=1".to_string(),
        ],
    );
    assert_eq!(sync_exit, 0);
    let sync_latest = read_json(&latest_path(root_path));
    assert_claim(&sync_latest, "V7-BUSINESS-001.6");

    let audit_exit = business_plane::run(
        root_path,
        &[
            "continuity-audit".to_string(),
            "--days=7".to_string(),
            "--business-context=SUB_A".to_string(),
            "--strict=1".to_string(),
        ],
    );
    assert_eq!(audit_exit, 0);
    let audit_latest = read_json(&latest_path(root_path));
    assert_claim(&audit_latest, "V7-BUSINESS-001.7");

    let export_exit = business_plane::run(
        root_path,
        &[
            "archive".to_string(),
            "--op=export".to_string(),
            "--business-context=SUB_A".to_string(),
            "--date-range=:".to_string(),
            "--strict=1".to_string(),
        ],
    );
    assert_eq!(export_exit, 0);
    let archive_latest = read_json(&latest_path(root_path));
    assert_claim(&archive_latest, "V7-BUSINESS-001.8");
    assert!(
        archive_latest
            .get("export_path")
            .and_then(Value::as_str)
            .map(|p| PathBuf::from(p).exists())
            .unwrap_or(false),
        "archive export artifact should exist"
    );

    let bypass_exit = business_plane::run(
        root_path,
        &[
            "taxonomy".to_string(),
            "--business-context=SUB_A".to_string(),
            "--topic=blocked".to_string(),
            "--strict=1".to_string(),
            "--bypass=1".to_string(),
        ],
    );
    assert_eq!(bypass_exit, 1, "conduit bypass must fail closed");
}
