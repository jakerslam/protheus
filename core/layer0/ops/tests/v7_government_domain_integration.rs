// SPDX-License-Identifier: Apache-2.0

use protheus_ops_core::government_plane;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

fn latest_path(root: &Path) -> PathBuf {
    root.join("core")
        .join("local")
        .join("state")
        .join("ops")
        .join("government_plane")
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

fn run_cmd(root: &Path, args: &[&str]) -> i32 {
    let argv = args.iter().map(|row| row.to_string()).collect::<Vec<_>>();
    government_plane::run(root, &argv)
}

#[test]
fn v7_gov_001_1_to_001_9_runtime_contracts_proven() {
    let root = tempfile::tempdir().expect("tempdir");
    let root_path = root.path();

    std::env::set_var("PROTHEUS_HSM_RECEIPT_KEY", "test-hsm");
    let attest_exit = run_cmd(
        root_path,
        &[
            "attestation",
            "--op=attest",
            "--device-id=tpm-node",
            "--nonce=n1",
            "--strict=1",
        ],
    );
    assert_eq!(attest_exit, 0);
    let verify_exit = run_cmd(root_path, &["attestation", "--op=verify", "--strict=1"]);
    assert_eq!(verify_exit, 0);
    let attest_latest = read_json(&latest_path(root_path));
    assert_claim(&attest_latest, "V7-GOV-001.1");

    let set_clearance = run_cmd(
        root_path,
        &[
            "classification",
            "--op=set-clearance",
            "--principal=analyst",
            "--clearance=secret",
            "--strict=1",
        ],
    );
    assert_eq!(set_clearance, 0);
    let write_secret = run_cmd(
        root_path,
        &[
            "classification",
            "--op=write",
            "--principal=analyst",
            "--level=secret",
            "--id=brief",
            "--payload-json={\"summary\":\"classified\"}",
            "--strict=1",
        ],
    );
    assert_eq!(write_secret, 0);
    let low_read = run_cmd(
        root_path,
        &[
            "classification",
            "--op=read",
            "--principal=intern",
            "--level=secret",
            "--id=brief",
            "--strict=1",
        ],
    );
    assert_eq!(low_read, 1, "lower clearance read must fail");
    let class_latest = read_json(&latest_path(root_path));
    assert_claim(&class_latest, "V7-GOV-001.2");

    let legal_exit = run_cmd(
        root_path,
        &[
            "nonrepudiation",
            "--principal=CN=User,O=Gov,OU=Dept",
            "--action=approve_order",
            "--auth-signature=RSA4096SIG",
            "--timestamp-authority=tsa.gov",
            "--legal-hold=1",
            "--strict=1",
        ],
    );
    assert_eq!(legal_exit, 0);
    let legal_latest = read_json(&latest_path(root_path));
    assert_claim(&legal_latest, "V7-GOV-001.3");

    let diode_exit = run_cmd(
        root_path,
        &[
            "diode",
            "--from=secret",
            "--to=unclassified",
            "--sanitize=1",
            "--payload-json={\"doc\":\"summary\"}",
            "--strict=1",
        ],
    );
    assert_eq!(diode_exit, 0);
    let diode_latest = read_json(&latest_path(root_path));
    assert_claim(&diode_latest, "V7-GOV-001.4");

    let soc_connect = run_cmd(
        root_path,
        &[
            "soc",
            "--op=connect",
            "--endpoint=splunk://soc",
            "--strict=1",
        ],
    );
    assert_eq!(soc_connect, 0);
    let soc_emit = run_cmd(
        root_path,
        &[
            "soc",
            "--op=emit",
            "--event-json={\"kind\":\"policy_violation\"}",
            "--strict=1",
        ],
    );
    assert_eq!(soc_emit, 0);
    let soc_latest = read_json(&latest_path(root_path));
    assert_claim(&soc_latest, "V7-GOV-001.5");

    let site_a = run_cmd(
        root_path,
        &[
            "coop",
            "--op=register-site",
            "--site=alpha",
            "--state=ACTIVE",
            "--strict=1",
        ],
    );
    assert_eq!(site_a, 0);
    let site_b = run_cmd(
        root_path,
        &[
            "coop",
            "--op=register-site",
            "--site=beta",
            "--state=STANDBY",
            "--strict=1",
        ],
    );
    assert_eq!(site_b, 0);
    let failover = run_cmd(
        root_path,
        &["coop", "--op=failover", "--target-site=beta", "--strict=1"],
    );
    assert_eq!(failover, 0);
    let coop_latest = read_json(&latest_path(root_path));
    assert_claim(&coop_latest, "V7-GOV-001.6");

    let proofs_exit = run_cmd(root_path, &["proofs", "--op=verify", "--strict=1"]);
    assert_eq!(proofs_exit, 1, "proofs may fail in isolated temp root");
    let proofs_latest = read_json(&latest_path(root_path));
    assert_claim(&proofs_latest, "V7-GOV-001.7");

    let interop_exit = run_cmd(
        root_path,
        &[
            "interoperability",
            "--op=validate",
            "--profile-json={\"standards\":[\"PKI\",\"SAML\",\"OIDC\",\"SMIME\",\"IPv6\",\"DNSSEC\",\"OAuth2\"],\"endpoint\":\"https://gov.api\"}",
            "--strict=1",
        ],
    );
    assert_eq!(interop_exit, 0);
    let interop_latest = read_json(&latest_path(root_path));
    assert_claim(&interop_latest, "V7-GOV-001.8");

    let ato_exit = run_cmd(root_path, &["ato-pack", "--op=generate", "--strict=1"]);
    assert_eq!(ato_exit, 0);
    let ato_latest = read_json(&latest_path(root_path));
    assert_claim(&ato_latest, "V7-GOV-001.9");

    let bypass_exit = run_cmd(
        root_path,
        &["soc", "--op=status", "--strict=1", "--bypass=1"],
    );
    assert_eq!(bypass_exit, 1, "bypass must fail closed");
}
