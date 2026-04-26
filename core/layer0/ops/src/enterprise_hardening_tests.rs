// SPDX-License-Identifier: Apache-2.0
use super::*;

fn write_text(root: &Path, rel: &str, body: &str) {
    let p = root.join(rel);
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent).expect("mkdir");
    }
    fs::write(p, body).expect("write");
}

fn write_regulated_readiness_fixture(root: &Path, required_env: &str, include_docs_tokens: bool) {
    write_text(
        root,
        "client/runtime/config/multi_tenant_isolation_contract.json",
        r#"{
  "schema_id": "multi_tenant_isolation_contract",
  "schema_version": "1.0",
  "invariants": {
    "cross_tenant_leaks": 0,
    "classification_enforced": true,
    "delete_export_contract": true
  },
  "namespace": {
    "mode": "strict",
    "tenant_id_pattern": "^[a-z0-9][a-z0-9-]{2,63}$",
    "enforce_per_tenant_state_roots": true,
    "per_tenant_state_roots": [
      "local/state/tenants/{tenant_id}"
    ],
    "cross_namespace_reads": "deny_by_default"
  }
}"#,
    );
    write_text(
        root,
        "client/runtime/config/enterprise_access_policy.json",
        r#"{
  "operations": {
    "op.a": {
      "tenant_scoped": true,
      "require_mfa": true
    }
  }
}"#,
    );
    write_text(
        root,
        "core/layer1/security/config/abac_policy_plane.json",
        r#"{
  "default_effect": "deny",
  "rules": [
    {
      "id": "allow",
      "effect": "allow"
    }
  ],
  "flight_recorder": {
    "immutable": true,
    "hash_chain": true
  }
}"#,
    );
    write_text(
        root,
        "client/runtime/config/enterprise_secret_kms_policy.json",
        r#"{
  "handle_mode": "opaque_handle_only",
  "cmek": {
    "key_uri": "kms://infring/test/key"
  },
  "backends": [
    {
      "id": "aws_kms_primary"
    }
  ]
}"#,
    );
    write_text(
        root,
        "client/runtime/config/signed_receipt_policy.json",
        &format!(
            r#"{{
  "algorithms": ["hmac_sha256"],
  "required_env": ["{required_env}"],
  "receipt_chain": {{
    "hmac_required_for_regulated_exports": true
  }}
}}"#
        ),
    );
    write_text(
        root,
        "client/runtime/config/retention_policy_pack.json",
        r#"{
  "enabled": true,
  "runtime_policy_path": "client/runtime/config/runtime_retention_policy.json",
  "compliance_policy_path": "client/runtime/config/compliance_retention_policy.json"
}"#,
    );
    write_text(
        root,
        "client/runtime/config/runtime_retention_policy.json",
        r#"{
  "jsonl_targets": [{"path":"local/state/x.jsonl","keep_lines":100}],
  "directory_targets": [{"path":"local/state/x","max_files":10}]
}"#,
    );
    write_text(
        root,
        "client/runtime/config/compliance_retention_policy.json",
        r#"{
  "tiers": {
    "hot_days": 90,
    "warm_days": 180,
    "cold_days": 365
  },
  "scopes": ["local/state/ops"]
}"#,
    );
    write_text(
        root,
        "client/runtime/config/audit_log_export_policy.json",
        r#"{
  "outputs": {
    "latest_path": "local/state/ops/audit/latest.json",
    "history_path": "local/state/ops/audit/history.jsonl"
  }
}"#,
    );
    write_text(
        root,
        "client/runtime/config/evidence_audit_dashboard_policy.json",
        r#"{
  "paths": {
    "export_json_path": "local/state/ops/evidence/export.json",
    "export_md_path": "local/state/ops/evidence/export.md",
    "receipts_path": "local/state/ops/evidence/receipts.jsonl"
  }
}"#,
    );
    write_text(
        root,
        "core/local/state/ops/enterprise_hardening/f100/zero_trust_profile.json",
        r#"{
  "signed_jwt": true,
  "cmek_key": "kms://infring/test/key",
  "private_link": "vpce-123",
  "egress": "deny"
}"#,
    );
    if include_docs_tokens {
        write_text(
            root,
            "docs/client/DEPLOYMENT_PACKAGING.md",
            r#"Multi-Tenant Deployment
RBAC/ABAC
KMS-backed Secret Handling
Signed Receipts
Retention Policy Packs
Exportable Audit Trails"#,
        );
    } else {
        write_text(root, "docs/client/DEPLOYMENT_PACKAGING.md", "placeholder");
    }
}

fn assert_non_silent_outcome(payload: &Value, expected_type: &str) {
    assert_eq!(
        payload.get("type").and_then(Value::as_str),
        Some(expected_type)
    );
    assert!(payload.get("ok").and_then(Value::as_bool).is_some());
    assert!(
        payload.get("receipt_hash").and_then(Value::as_str).is_some()
            || payload.get("claim_evidence").and_then(Value::as_array).is_some()
            || payload.get("error").is_some()
            || payload.get("reason").is_some()
    );
}

#[test]
fn cron_integrity_rejects_none_delivery_mode() {
    let tmp = tempfile::tempdir().expect("tmp");
    write_text(
        tmp.path(),
        "client/runtime/config/cron_jobs.json",
        r#"{"jobs":[{"id":"j1","name":"x","enabled":true,"sessionTarget":"isolated","delivery":{"mode":"none","channel":"last"}}]}"#,
    );
    let (ok, details) =
        check_cron_delivery_integrity(tmp.path(), "client/runtime/config/cron_jobs.json")
            .expect("audit");
    assert!(!ok);
    assert!(details.to_string().contains("delivery_mode_none_forbidden"));
}

#[test]
fn cron_integrity_rejects_missing_delivery_for_enabled_jobs() {
    let tmp = tempfile::tempdir().expect("tmp");
    write_text(
        tmp.path(),
        "client/runtime/config/cron_jobs.json",
        r#"{"jobs":[{"id":"j1","name":"x","enabled":true,"sessionTarget":"main"}]}"#,
    );
    let (ok, details) =
        check_cron_delivery_integrity(tmp.path(), "client/runtime/config/cron_jobs.json")
            .expect("audit");
    assert!(!ok);
    assert!(details
        .to_string()
        .contains("missing_delivery_for_enabled_job"));
}

#[test]
fn run_control_json_fields_detects_missing_field() {
    let tmp = tempfile::tempdir().expect("tmp");
    write_text(
        tmp.path(),
        "client/runtime/config/x.json",
        r#"{"a":{"b":1}}"#,
    );
    let control = json!({
        "id": "c1",
        "title": "json",
        "type": "json_fields",
        "path": "client/runtime/config/x.json",
        "required_fields": ["a.b", "a.c"]
    });
    let out = run_control(tmp.path(), control.as_object().expect("obj"));
    assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
    assert!(out.to_string().contains("a.c"));
}

#[test]
fn enable_bedrock_produces_sigv4_private_profile() {
    let tmp = tempfile::tempdir().expect("tmp");
    write_text(
        tmp.path(),
        DEFAULT_BEDROCK_POLICY_REL,
        r#"{
  "version": "v1",
  "kind": "enterprise_bedrock_proxy_contract",
  "provider": "bedrock",
  "region": "us-west-2",
  "auth": {
    "mode": "sigv4_instance_profile",
    "require_sigv4": true
  },
  "network": {
    "vpc": "vpc-prod",
    "subnet": "subnet-private-a",
    "require_private_subnet": true
  },
  "secrets": {
    "ssm_path": "/infring/bedrock/proxy",
    "require_ssm": true
  }
}"#,
    );
    let out = run_enable_bedrock(tmp.path(), true, &std::collections::HashMap::new())
        .expect("enable bedrock");
    assert_eq!(
        out.get("type").and_then(Value::as_str),
        Some("enterprise_hardening_enable_bedrock")
    );
    assert_non_silent_outcome(&out, "enterprise_hardening_enable_bedrock");
    assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
    assert!(out
        .pointer("/profile/auth/mode")
        .and_then(Value::as_str)
        .map(|row| row == "sigv4_instance_profile")
        .unwrap_or(false));
    let claim_ok = out
        .get("claim_evidence")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .any(|row| row.get("id").and_then(Value::as_str) == Some("V7-ASSIMILATE-001.5.1"));
    assert!(claim_ok, "missing bedrock claim evidence");
}

#[test]
fn regulated_readiness_passes_with_complete_contracts() {
    let tmp = tempfile::tempdir().expect("tmp");
    let env_key = "INFRING_TEST_RECEIPT_HMAC_KEY_OK";
    write_regulated_readiness_fixture(tmp.path(), env_key, true);

    let previous = std::env::var(env_key).ok();
    std::env::set_var(env_key, "signed-secret");

    let out = run_regulated_readiness(tmp.path(), true, &std::collections::HashMap::new())
        .expect("regulated readiness");
    assert_eq!(
        out.get("type").and_then(Value::as_str),
        Some("enterprise_hardening_regulated_readiness")
    );
    assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
    assert_eq!(out.get("controls_failed").and_then(Value::as_u64), Some(0));

    if let Some(value) = previous {
        std::env::set_var(env_key, value);
    } else {
        std::env::remove_var(env_key);
    }
}

#[test]
fn regulated_readiness_fails_when_signed_env_and_docs_are_missing() {
    let tmp = tempfile::tempdir().expect("tmp");
    let env_key = "INFRING_TEST_RECEIPT_HMAC_KEY_FAIL";
    write_regulated_readiness_fixture(tmp.path(), env_key, false);
    std::env::remove_var(env_key);

    let out = run_regulated_readiness(tmp.path(), true, &std::collections::HashMap::new())
        .expect("regulated readiness");
    assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
    assert!(
        out.get("controls_failed")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            >= 2
    );

    let controls = out
        .get("controls")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let signed_receipts_failed = controls.iter().any(|row| {
        row.get("id").and_then(Value::as_str) == Some("signed_receipts")
            && row.get("ok").and_then(Value::as_bool) == Some(false)
    });
    let docs_failed = controls.iter().any(|row| {
        row.get("id").and_then(Value::as_str) == Some("deployment_docs_regulated_surface")
            && row.get("ok").and_then(Value::as_bool) == Some(false)
    });
    assert!(signed_receipts_failed);
    assert!(docs_failed);
}
