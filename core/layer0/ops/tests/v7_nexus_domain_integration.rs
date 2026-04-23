// SPDX-License-Identifier: Apache-2.0

use infring_ops_core::nexus_plane;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

fn latest_path(root: &Path) -> PathBuf {
    root.join("core")
        .join("local")
        .join("state")
        .join("ops")
        .join("nexus_plane")
        .join("latest.json")
}

fn read_json(path: &Path) -> Value {
    let raw = fs::read_to_string(path).expect("read");
    serde_json::from_str(&raw).expect("parse")
}

fn assert_claim(payload: &Value, id: &str) {
    let ok = payload
        .get("claim_evidence")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .any(|row| row.get("id").and_then(Value::as_str) == Some(id));
    assert!(ok, "missing claim {id}");
}

fn run_nexus(root: &Path, args: Vec<String>) -> Value {
    let exit = nexus_plane::run(root, &args);
    assert_eq!(exit, 0);
    read_json(&latest_path(root))
}

fn arg_set(rows: &[&str]) -> Vec<String> {
    rows.iter().map(|row| row.to_string()).collect()
}

#[test]
fn v7_nexus_001_1_to_001_7_runtime_contracts_proven() {
    let root = tempfile::tempdir().expect("tempdir");
    let root_path = root.path();

    let package_latest = run_nexus(
        root_path,
        arg_set(&["package-domain", "--domain=finance", "--strict=1"]),
    );
    assert_claim(&package_latest, "V7-NEXUS-001.1");

    let bridge_latest = run_nexus(
        root_path,
        arg_set(&[
            "bridge",
            "--from-domain=finance",
            "--to-domain=government",
            "--payload-json={\"event\":\"payment\"}",
            "--legal-contract-id=contract-77",
            "--sanitize=1",
            "--strict=1",
        ]),
    );
    assert_claim(&bridge_latest, "V7-NEXUS-001.2");

    let insurance_latest = run_nexus(
        root_path,
        arg_set(&[
            "insurance",
            "--op=quote",
            "--risk-json={\"risk_score\":0.3,\"compliance_score\":0.9}",
            "--strict=1",
        ]),
    );
    assert_claim(&insurance_latest, "V7-NEXUS-001.3");

    let human_latest = run_nexus(
        root_path,
        arg_set(&[
            "human-boundary",
            "--op=authorize",
            "--action=deploy_critical",
            "--human-a=SIG_A",
            "--human-b=SIG_B",
            "--strict=1",
        ]),
    );
    assert_claim(&human_latest, "V7-NEXUS-001.4");

    let receipt_latest = run_nexus(
        root_path,
        arg_set(&[
            "receipt-v2",
            "--op=validate",
            "--receipt-json={\"domain\":\"finance\",\"classifications\":[\"CUI\"],\"authorization\":{\"principal\":\"u\"},\"compliance\":{\"controls\":[\"x\"]},\"insurance\":{\"coverage\":\"approved\"}}",
            "--strict=1",
        ]),
    );
    assert_claim(&receipt_latest, "V7-NEXUS-001.5");

    let merkle_latest = run_nexus(
        root_path,
        arg_set(&["merkle-forest", "--op=build", "--strict=1"]),
    );
    assert_claim(&merkle_latest, "V7-NEXUS-001.6");

    let ledger_latest = run_nexus(
        root_path,
        vec![
            "compliance-ledger".to_string(),
            "--op=append".to_string(),
            "--chain-id=chain-1".to_string(),
            "--entry-json={\"from\":\"finance\",\"to\":\"government\",\"result\":\"ok\"}"
                .to_string(),
            "--strict=1".to_string(),
        ],
    );
    assert_claim(&ledger_latest, "V7-NEXUS-001.7");

    let bypass_exit = nexus_plane::run(
        root_path,
        &[
            "bridge".to_string(),
            "--from-domain=finance".to_string(),
            "--to-domain=government".to_string(),
            "--strict=1".to_string(),
            "--bypass=1".to_string(),
        ],
    );
    assert_eq!(bypass_exit, 1, "bypass must fail closed");
}
