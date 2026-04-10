// SPDX-License-Identifier: Apache-2.0
use conduit::{
    deterministic_receipt_hash, process_command, CommandEnvelope, ConduitPolicy,
    ConduitSecurityContext, EchoCommandHandler, RegistryPolicyGate, TsCommand,
};
use std::time::{SystemTime, UNIX_EPOCH};

const TEST_REQUEST_ID: &str = "inv-req";

fn test_security(policy: &ConduitPolicy) -> ConduitSecurityContext {
    ConduitSecurityContext::from_policy(policy, "msg-k1", "msg-secret", "tok-k1", "tok-secret")
}

fn signed_envelope(policy: &ConduitPolicy, command: TsCommand) -> CommandEnvelope {
    let security = test_security(policy);
    let ts_ms = now_ts_ms();
    let security_metadata =
        security.mint_security_metadata("client-a", TEST_REQUEST_ID, ts_ms, &command, 120_000);

    CommandEnvelope {
        schema_id: conduit::CONDUIT_SCHEMA_ID.to_string(),
        schema_version: conduit::CONDUIT_SCHEMA_VERSION.to_string(),
        request_id: TEST_REQUEST_ID.to_string(),
        ts_ms,
        command,
        security: security_metadata,
    }
}

fn now_ts_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

fn invariant_policy() -> (ConduitPolicy, tempfile::TempDir) {
    let temp = tempfile::tempdir().expect("tempdir");
    let constitution = temp.path().join("constitution.md");
    let registry = temp.path().join("guard_registry.json");

    std::fs::write(&constitution, "Mind Sovereignty Covenant\nRSI Guardrails\n")
        .expect("write constitution");
    std::fs::write(
        &registry,
        serde_json::json!({
            "merge_guard": {
                "checks": [
                    {"id":"contract_check"},
                    {"id":"formal_invariant_engine"}
                ]
            }
        })
        .to_string(),
    )
    .expect("write registry");

    let policy = ConduitPolicy {
        constitution_path: constitution.to_string_lossy().to_string(),
        guard_registry_path: registry.to_string_lossy().to_string(),
        ..ConduitPolicy::default()
    };
    (policy, temp)
}

fn validation_summary(policy: &ConduitPolicy, envelope: &CommandEnvelope) -> (bool, bool, String) {
    let gate = RegistryPolicyGate::new(policy.clone());
    let mut security = test_security(policy);
    let mut handler = EchoCommandHandler;
    let response = process_command(envelope, &gate, &mut security, &mut handler);
    (
        response.validation.ok,
        response.validation.fail_closed,
        response.validation.reason,
    )
}

#[test]
fn deterministic_hashes_match_for_equal_envelopes() {
    let (policy, _tmp) = invariant_policy();
    let a = signed_envelope(&policy, TsCommand::ListActiveAgents);
    let b = signed_envelope(&policy, TsCommand::ListActiveAgents);

    assert_eq!(
        deterministic_receipt_hash(&a.command),
        deterministic_receipt_hash(&b.command)
    );
}

#[test]
fn install_extension_validation_is_fail_closed_for_invalid_sha() {
    let (policy, _tmp) = invariant_policy();
    let envelope = signed_envelope(
        &policy,
        TsCommand::InstallExtension {
            extension_id: "ext-1".to_string(),
            wasm_sha256: "deadbeef".to_string(),
            capabilities: vec!["metrics.read".to_string()],
            plugin_type: Some("substrate_adapter".to_string()),
            version: Some("0.1.0".to_string()),
            wasm_component_path: Some("adapters/protocol/wasm_adapter_skeleton.wasm".to_string()),
            signature: None,
            provenance: None,
            recovery_max_attempts: None,
            recovery_backoff_ms: None,
        },
    );

    let (ok, fail_closed, reason) = validation_summary(&policy, &envelope);
    assert!(!ok);
    assert!(fail_closed);
    assert_eq!(reason, "extension_wasm_sha256_invalid");
}

#[test]
fn policy_safe_patch_passes_validation() {
    let (policy, _tmp) = invariant_policy();
    let envelope = signed_envelope(
        &policy,
        TsCommand::ApplyPolicyUpdate {
            patch_id: "constitution_safe/allow-listed-change".to_string(),
            patch: serde_json::json!({"path":"/policy/test","value":true}),
        },
    );

    let (ok, fail_closed, _reason) = validation_summary(&policy, &envelope);
    assert!(ok);
    assert!(!fail_closed);
}
