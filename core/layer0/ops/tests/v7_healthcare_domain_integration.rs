// SPDX-License-Identifier: Apache-2.0

use protheus_ops_core::healthcare_plane;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

fn latest_path(root: &Path) -> PathBuf {
    root.join("core")
        .join("local")
        .join("state")
        .join("ops")
        .join("healthcare_plane")
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

#[test]
fn v7_health_001_1_to_001_10_runtime_contracts_proven() {
    let root = tempfile::tempdir().expect("tempdir");
    let root_path = root.path();

    let patient_exit = healthcare_plane::run(
        root_path,
        &[
            "patient".to_string(),
            "--op=register".to_string(),
            "--patient-id=p001".to_string(),
            "--mrn=123456".to_string(),
            "--consent-json={\"treatment\":true,\"research\":false}".to_string(),
            "--strict=1".to_string(),
        ],
    );
    assert_eq!(patient_exit, 0);
    let patient_latest = read_json(&latest_path(root_path));
    assert_claim(&patient_latest, "V7-HEALTH-001.1");

    let phi_exit = healthcare_plane::run(
        root_path,
        &[
            "phi-audit".to_string(),
            "--op=access".to_string(),
            "--user=dr.smith".to_string(),
            "--npi=1234567890".to_string(),
            "--patient-id=p001".to_string(),
            "--reason=treatment".to_string(),
            "--strict=1".to_string(),
        ],
    );
    assert_eq!(phi_exit, 0);
    let phi_latest = read_json(&latest_path(root_path));
    assert_claim(&phi_latest, "V7-HEALTH-001.2");

    let cds_exit = healthcare_plane::run(
        root_path,
        &[
            "cds".to_string(),
            "--op=evaluate".to_string(),
            "--patient-id=p001".to_string(),
            "--meds=warfarin,aspirin,penicillin".to_string(),
            "--allergies=penicillin".to_string(),
            "--dose-mg=1250".to_string(),
            "--strict=1".to_string(),
        ],
    );
    assert_eq!(cds_exit, 0);
    let cds_latest = read_json(&latest_path(root_path));
    assert_claim(&cds_latest, "V7-HEALTH-001.3");

    let devices_exit = healthcare_plane::run(
        root_path,
        &[
            "devices".to_string(),
            "--op=ingest".to_string(),
            "--protocol=fhir".to_string(),
            "--device-id=icu-monitor-1".to_string(),
            "--payload-json={\"spo2\":88}".to_string(),
            "--strict=1".to_string(),
        ],
    );
    assert_eq!(devices_exit, 0);
    let devices_latest = read_json(&latest_path(root_path));
    assert_claim(&devices_latest, "V7-HEALTH-001.4");

    let docs_exit = healthcare_plane::run(
        root_path,
        &[
            "documentation".to_string(),
            "--op=draft".to_string(),
            "--soap-json={\"subjective\":\"pain\",\"objective\":\"fever\",\"assessment\":\"infection\",\"plan\":\"labs\"}".to_string(),
            "--codes-json={\"ICD10\":\"A41.9\",\"SNOMED\":\"91302008\"}".to_string(),
            "--strict=1".to_string(),
        ],
    );
    assert_eq!(docs_exit, 0);
    let docs_latest = read_json(&latest_path(root_path));
    assert_claim(&docs_latest, "V7-HEALTH-001.5");

    let alert_exit = healthcare_plane::run(
        root_path,
        &[
            "alerts".to_string(),
            "--op=emit".to_string(),
            "--tier=critical".to_string(),
            "--key=sepsis-risk".to_string(),
            "--strict=1".to_string(),
        ],
    );
    assert_eq!(alert_exit, 0);
    let alerts_latest = read_json(&latest_path(root_path));
    assert_claim(&alerts_latest, "V7-HEALTH-001.6");

    let coord_exit = healthcare_plane::run(
        root_path,
        &[
            "coordination".to_string(),
            "--op=handoff".to_string(),
            "--sbar-json={\"situation\":\"sepsis\",\"background\":\"ed\",\"assessment\":\"stable\",\"recommendation\":\"icu\"}".to_string(),
            "--meds-json={\"home\":[\"metformin\"],\"inpatient\":[\"ceftriaxone\"]}".to_string(),
            "--strict=1".to_string(),
        ],
    );
    assert_eq!(coord_exit, 0);
    let coord_latest = read_json(&latest_path(root_path));
    assert_claim(&coord_latest, "V7-HEALTH-001.7");

    let trial_exit = healthcare_plane::run(
        root_path,
        &[
            "trials".to_string(),
            "--op=screen".to_string(),
            "--patient-id=p001".to_string(),
            "--trial=trial-a".to_string(),
            "--strict=1".to_string(),
        ],
    );
    assert_eq!(trial_exit, 0);
    let trial_latest = read_json(&latest_path(root_path));
    assert_claim(&trial_latest, "V7-HEALTH-001.8");

    let imaging_exit = healthcare_plane::run(
        root_path,
        &[
            "imaging".to_string(),
            "--op=critical-route".to_string(),
            "--study-id=ct-123".to_string(),
            "--finding=intracranial hemorrhage".to_string(),
            "--strict=1".to_string(),
        ],
    );
    assert_eq!(imaging_exit, 0);
    let imaging_latest = read_json(&latest_path(root_path));
    assert_claim(&imaging_latest, "V7-HEALTH-001.9");

    let emergency_exit = healthcare_plane::run(
        root_path,
        &[
            "emergency".to_string(),
            "--op=break-glass".to_string(),
            "--user=ed.attending".to_string(),
            "--patient-id=p001".to_string(),
            "--justification=unconscious trauma".to_string(),
            "--ttl-minutes=30".to_string(),
            "--strict=1".to_string(),
        ],
    );
    assert_eq!(emergency_exit, 0);
    let emergency_latest = read_json(&latest_path(root_path));
    assert_claim(&emergency_latest, "V7-HEALTH-001.10");

    let bypass_exit = healthcare_plane::run(
        root_path,
        &[
            "phi-audit".to_string(),
            "--op=status".to_string(),
            "--strict=1".to_string(),
            "--bypass=1".to_string(),
        ],
    );
    assert_eq!(bypass_exit, 1, "bypass must fail closed");
}
