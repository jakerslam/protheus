use super::{
    clean_lane_id, normalize_edge_prompt, summarize_for_edge_backend, validate_command,
    validate_structure, CommandEnvelope, TsCommand,
};
use super::{
    conduit_message_contract_count, process_command, run_stdio_once,
    validate_conduit_contract_budget, CommandHandler, ConduitPolicy, ConduitSecurityContext,
    EchoCommandHandler, KernelLaneCommandHandler, PolicyGate, RegistryPolicyGate, RustEvent,
    MAX_CONDUIT_MESSAGE_TYPES, RUST_EVENT_TYPES, TS_COMMAND_TYPES,
};
use protheus_nexus_core_v1::{
    CapabilityTokenAuthority, MessageSigner, RateLimitPolicy, RateLimiter,
};
use serde_json::Value;
use std::fs;
use std::io::{BufRead, BufReader, Cursor, Write};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

fn test_policy_paths() -> (PathBuf, PathBuf, tempfile::TempDir) {
    let temp = tempfile::tempdir().expect("tempdir");
    let constitution = temp.path().join("docs/workspace/AGENT-CONSTITUTION.md");
    let guard_registry = temp.path().join("guard_check_registry.json");
    if let Some(parent) = constitution.parent() {
        fs::create_dir_all(parent).expect("create constitution dir");
    }

    fs::write(&constitution, "Mind Sovereignty Covenant\nRSI Guardrails\n")
        .expect("write constitution");

    fs::write(
        &guard_registry,
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
    .expect("write guard registry");

    (constitution, guard_registry, temp)
}

fn test_policy() -> ConduitPolicy {
    let (constitution, guard_registry, temp) = test_policy_paths();
    std::mem::forget(temp);
    ConduitPolicy {
        constitution_path: constitution.to_string_lossy().to_string(),
        guard_registry_path: guard_registry.to_string_lossy().to_string(),
        rate_limit: RateLimitPolicy {
            window_ms: 5_000,
            per_client_max: 10,
            per_client_command_max: 10,
        },
        ..ConduitPolicy::default()
    }
}

fn test_security(policy: &ConduitPolicy) -> ConduitSecurityContext {
    ConduitSecurityContext::new(
        MessageSigner::new("msg-k1", "msg-secret"),
        CapabilityTokenAuthority::new("tok-k1", "tok-secret"),
        RateLimiter::new(policy.rate_limit.clone()),
        policy.command_required_capabilities.clone(),
    )
}

struct AlwaysDenyPolicy;

impl PolicyGate for AlwaysDenyPolicy {
    fn evaluate(&self, _command: &TsCommand) -> super::PolicyDecision {
        super::PolicyDecision::deny("deny_for_test")
    }
}

fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn signed_envelope(policy: &ConduitPolicy, command: TsCommand) -> CommandEnvelope {
    let security = test_security(policy);
    let request_id = "req-test";
    let ts_ms = super::now_ts_ms();
    let security_metadata =
        security.mint_security_metadata("client-a", request_id, ts_ms, &command, 60_000);
    CommandEnvelope {
        schema_id: super::CONDUIT_SCHEMA_ID.to_string(),
        schema_version: super::CONDUIT_SCHEMA_VERSION.to_string(),
        request_id: request_id.to_string(),
        ts_ms,
        command,
        security: security_metadata,
    }
}

#[test]
fn command_and_event_contract_counts_match_spec() {
    assert_eq!(TS_COMMAND_TYPES.len(), 7);
    assert_eq!(RUST_EVENT_TYPES.len(), 3);
    assert_eq!(conduit_message_contract_count(), MAX_CONDUIT_MESSAGE_TYPES);
    assert!(validate_conduit_contract_budget(MAX_CONDUIT_MESSAGE_TYPES).is_ok());
}

#[test]
fn conduit_policy_defaults_and_envelope_constructor_cover_contract_paths() {
    let serialized = serde_json::to_value(ConduitPolicy::default()).expect("policy value");
    let mut obj = serialized.as_object().cloned().expect("policy object");
    obj.remove("bridge_message_budget_max");
    let restored: ConduitPolicy =
        serde_json::from_value(Value::Object(obj)).expect("deserialize policy with defaults");
    assert_eq!(
        restored.bridge_message_budget_max, MAX_CONDUIT_MESSAGE_TYPES,
        "bridge message budget should use contract default when omitted"
    );

    let policy = test_policy();
    let signed = signed_envelope(&policy, TsCommand::GetSystemStatus);
    let constructed = CommandEnvelope::new(
        "req-new",
        TsCommand::GetSystemStatus,
        signed.security.clone(),
    );
    assert_eq!(constructed.schema_id, super::CONDUIT_SCHEMA_ID);
    assert_eq!(constructed.schema_version, super::CONDUIT_SCHEMA_VERSION);
    assert_eq!(constructed.request_id, "req-new");
    assert!(constructed.ts_ms > 0);

    let allow = super::AllowAllPolicy.evaluate(&TsCommand::GetSystemStatus);
    assert!(allow.allow);
    assert_eq!(allow.reason, "policy_allow");

    let deny = super::FailClosedPolicy.evaluate(&TsCommand::GetSystemStatus);
    assert!(!deny.allow);
    assert_eq!(deny.reason, "policy_gate_not_configured");
}

#[test]
fn secure_signed_command_passes_and_returns_receipts() {
    let policy = test_policy();
    let gate = RegistryPolicyGate::new(policy.clone());
    let mut security = test_security(&policy);
    let command = signed_envelope(
        &policy,
        TsCommand::StartAgent {
            agent_id: "agent-alpha".to_string(),
        },
    );

    let mut handler = EchoCommandHandler;
    let response = process_command(&command, &gate, &mut security, &mut handler);
    assert!(response.validation.ok);
    assert!(!response.validation.policy_receipt_hash.is_empty());
    assert!(!response.validation.security_receipt_hash.is_empty());
}

#[test]
fn bad_signature_fails_closed() {
    let policy = test_policy();
    let gate = RegistryPolicyGate::new(policy.clone());
    let mut security = test_security(&policy);
    let mut command = signed_envelope(&policy, TsCommand::GetSystemStatus);
    command.security.signature = "deadbeef".to_string();

    let mut handler = EchoCommandHandler;
    let response = process_command(&command, &gate, &mut security, &mut handler);
    assert!(!response.validation.ok);
    assert!(response.validation.fail_closed);
    assert_eq!(response.validation.reason, "message_signature_invalid");
}

#[test]
fn missing_scope_fails_closed() {
    let envelope_policy = test_policy();
    let command = signed_envelope(
        &envelope_policy,
        TsCommand::InstallExtension {
            extension_id: "ext-1".to_string(),
            wasm_sha256: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
                .to_string(),
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

    let mut runtime_policy = envelope_policy.clone();
    runtime_policy.command_required_capabilities.insert(
        "install_extension".to_string(),
        "extension.install.strict".to_string(),
    );

    let gate = RegistryPolicyGate::new(runtime_policy.clone());
    let mut security = test_security(&runtime_policy);

    let mut handler = EchoCommandHandler;
    let response = process_command(&command, &gate, &mut security, &mut handler);
    assert!(!response.validation.ok);
    assert!(response.validation.fail_closed);
    assert!(response
        .validation
        .reason
        .starts_with("capability_token_missing_scope"));
}

#[test]
fn schema_mismatch_fails_closed_before_policy_and_security() {
    let policy = test_policy();
    let gate = RegistryPolicyGate::new(policy.clone());
    let mut security = test_security(&policy);
    let mut command = signed_envelope(&policy, TsCommand::GetSystemStatus);
    command.schema_id = "conduit.command.envelope.v0".to_string();

    let receipt = validate_command(&command, &gate, &mut security);
    assert!(!receipt.ok);
    assert!(receipt.fail_closed);
    assert_eq!(receipt.reason, "conduit_schema_mismatch");
    assert_eq!(receipt.policy_receipt_hash, "policy_not_evaluated");
    assert_eq!(receipt.security_receipt_hash, "security_not_evaluated");
}

#[test]
fn policy_denial_fails_closed_before_security_validation() {
    let policy = test_policy();
    let mut security = test_security(&policy);
    let command = signed_envelope(&policy, TsCommand::GetSystemStatus);

    let receipt = validate_command(&command, &AlwaysDenyPolicy, &mut security);
    assert!(!receipt.ok);
    assert!(receipt.fail_closed);
    assert_eq!(receipt.reason, "deny_for_test");
    assert_eq!(receipt.security_receipt_hash, "security_not_evaluated");
}

#[test]
fn production_drift_violation_triggers_judicial_lock() {
    let _guard = env_lock().lock().expect("env lock");
    let temp = tempfile::tempdir().expect("tempdir");
    let config_path = temp.path().join("drift_policy.signed.json");
    let events_path = temp.path().join("drift_events.jsonl");
    let lock_path = temp.path().join("judicial_lock.json");

    std::env::set_var(
        "INFRING_VERITY_DRIFT_SIGNING_KEY",
        "verity-test-signing-key",
    );
    std::env::set_var("INFRING_VERITY_DRIFT_CONFIG_PATH", &config_path);
    std::env::set_var("INFRING_VERITY_DRIFT_EVENTS_PATH", &events_path);
    std::env::set_var("INFRING_VERITY_JUDICIAL_LOCK_PATH", &lock_path);

    let mut signed_config = super::VerityDriftSignedConfig {
        schema_id: super::VERITY_DRIFT_CONFIG_SCHEMA_ID.to_string(),
        schema_version: super::VERITY_DRIFT_CONFIG_SCHEMA_VERSION,
        policy_version: super::VERITY_DRIFT_CONFIG_POLICY_VERSION,
        mode: "production".to_string(),
        production_tolerance_ms: 500,
        simulation_tolerance_ms: 30_000,
        signature: String::new(),
    };
    signed_config.signature =
        super::sign_verity_config_payload(&super::verity_signature_payload(&signed_config));
    fs::write(
        &config_path,
        serde_json::to_string_pretty(&signed_config).expect("serialize config"),
    )
    .expect("write signed config");

    let policy = test_policy();
    let gate = RegistryPolicyGate::new(policy.clone());
    let mut security = test_security(&policy);
    let mut signer = test_security(&policy);
    let mut command = signed_envelope(&policy, TsCommand::GetSystemStatus);
    command.ts_ms = super::now_ts_ms().saturating_sub(2_000);
    command.security = signer.mint_security_metadata(
        "client-a",
        command.request_id.as_str(),
        command.ts_ms,
        &command.command,
        60_000,
    );

    let receipt = validate_command(&command, &gate, &mut security);
    assert!(!receipt.ok);
    assert!(receipt.fail_closed);
    assert_eq!(receipt.reason, "timestamp_drift_exceeded");
    assert_eq!(receipt.mode, "production");
    assert!(receipt.timestamp_drift_ms >= 2_000);

    let lock_raw = fs::read_to_string(&lock_path).expect("judicial lock payload");
    let lock_payload: Value = serde_json::from_str(&lock_raw).expect("parse lock payload");
    assert_eq!(
        lock_payload.get("active").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        lock_payload
            .pointer("/validation_receipt/reason")
            .and_then(Value::as_str),
        Some("timestamp_drift_exceeded")
    );

    let events_raw = fs::read_to_string(&events_path).expect("drift events");
    assert!(events_raw.contains("verity_drift_violation"));
    assert!(events_raw.contains("critical"));

    std::env::remove_var("INFRING_VERITY_DRIFT_SIGNING_KEY");
    std::env::remove_var("INFRING_VERITY_DRIFT_CONFIG_PATH");
    std::env::remove_var("INFRING_VERITY_DRIFT_EVENTS_PATH");
    std::env::remove_var("INFRING_VERITY_JUDICIAL_LOCK_PATH");
}

#[test]
fn structural_validation_rejects_invalid_limits_and_ids() {
    let empty_start = validate_structure(&TsCommand::StartAgent {
        agent_id: "   ".to_string(),
    });
    assert_eq!(empty_start.as_deref(), Some("agent_id_required"));

    let zero_limit = validate_structure(&TsCommand::QueryReceiptChain {
        from_hash: Some("abc123".to_string()),
        limit: Some(0),
    });
    assert_eq!(
        zero_limit.as_deref(),
        Some("receipt_query_limit_out_of_range")
    );

    let high_limit = validate_structure(&TsCommand::QueryReceiptChain {
        from_hash: Some("abc123".to_string()),
        limit: Some(1001),
    });
    assert_eq!(
        high_limit.as_deref(),
        Some("receipt_query_limit_out_of_range")
    );

    let missing_patch_id = validate_structure(&TsCommand::ApplyPolicyUpdate {
        patch_id: " ".to_string(),
        patch: serde_json::json!({"safe": true}),
    });
    assert_eq!(
        missing_patch_id.as_deref(),
        Some("policy_patch_id_required")
    );

    let unsafe_patch = validate_structure(&TsCommand::ApplyPolicyUpdate {
        patch_id: "runtime/unsafe".to_string(),
        patch: serde_json::json!({"safe": false}),
    });
    assert_eq!(
        unsafe_patch.as_deref(),
        Some("policy_update_must_be_constitution_safe")
    );
}

#[test]
fn structural_validation_rejects_bad_extensions() {
    let missing_id = validate_structure(&TsCommand::InstallExtension {
        extension_id: " ".to_string(),
        wasm_sha256: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
        capabilities: vec!["metrics.read".to_string()],
        plugin_type: Some("substrate_adapter".to_string()),
        version: Some("0.1.0".to_string()),
        wasm_component_path: Some("adapters/protocol/wasm_adapter_skeleton.wasm".to_string()),
        signature: None,
        provenance: None,
        recovery_max_attempts: None,
        recovery_backoff_ms: None,
    });
    assert_eq!(missing_id.as_deref(), Some("extension_id_required"));

    let bad_capabilities = validate_structure(&TsCommand::InstallExtension {
        extension_id: "ext-valid".to_string(),
        wasm_sha256: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
        capabilities: vec!["".to_string()],
        plugin_type: Some("substrate_adapter".to_string()),
        version: Some("0.1.0".to_string()),
        wasm_component_path: Some("adapters/protocol/wasm_adapter_skeleton.wasm".to_string()),
        signature: None,
        provenance: None,
        recovery_max_attempts: None,
        recovery_backoff_ms: None,
    });
    assert_eq!(
        bad_capabilities.as_deref(),
        Some("extension_capabilities_invalid")
    );

    let missing_path = validate_structure(&TsCommand::InstallExtension {
        extension_id: "ext-valid".to_string(),
        wasm_sha256: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
        capabilities: vec!["metrics.read".to_string()],
        plugin_type: Some("substrate_adapter".to_string()),
        version: Some("0.1.0".to_string()),
        wasm_component_path: None,
        signature: None,
        provenance: None,
        recovery_max_attempts: None,
        recovery_backoff_ms: None,
    });
    assert_eq!(
        missing_path.as_deref(),
        Some("extension_wasm_component_path_required")
    );

    let bad_plugin_type = validate_structure(&TsCommand::InstallExtension {
        extension_id: "ext-valid".to_string(),
        wasm_sha256: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
        capabilities: vec!["metrics.read".to_string()],
        plugin_type: Some("invalid_type".to_string()),
        version: Some("0.1.0".to_string()),
        wasm_component_path: Some("adapters/protocol/wasm_adapter_skeleton.wasm".to_string()),
        signature: None,
        provenance: None,
        recovery_max_attempts: None,
        recovery_backoff_ms: None,
    });
    assert_eq!(
        bad_plugin_type.as_deref(),
        Some("extension_plugin_type_invalid")
    );
}

#[test]
fn edge_prompt_helpers_normalize_and_cap_tokens() {
    assert_eq!(normalize_edge_prompt("   \n  "), "(empty_prompt)");
    assert_eq!(
        normalize_edge_prompt("hello   tiny   world"),
        "hello tiny world"
    );
    assert_eq!(
        summarize_for_edge_backend("a b c d e", 3),
        "a b c".to_string()
    );
    assert_eq!(summarize_for_edge_backend("a b", 3), "a b".to_string());
    assert_eq!(summarize_for_edge_backend("a b c", 0), "".to_string());
}

#[test]
fn clean_lane_id_keeps_allowed_chars_and_uppercases() {
    assert_eq!(clean_lane_id(" lane-1.alpha_beta!@# "), "LANE-1.ALPHA_BETA");
}

#[test]
fn echo_handler_covers_stop_and_query_paths() {
    let mut handler = EchoCommandHandler;
    match handler.handle(&TsCommand::StopAgent {
        agent_id: "agent-stop".to_string(),
    }) {
        RustEvent::AgentLifecycle { state, agent_id } => {
            assert_eq!(state, super::AgentLifecycleState::Stopped);
            assert_eq!(agent_id, "agent-stop");
        }
        other => panic!("expected stop lifecycle event, got {other:?}"),
    }

    match handler.handle(&TsCommand::QueryReceiptChain {
        from_hash: Some("abc".to_string()),
        limit: Some(1),
    }) {
        RustEvent::ReceiptAdded { receipt_hash } => {
            assert_eq!(receipt_hash, "query_receipt_chain_ack");
        }
        other => panic!("expected receipt-added event, got {other:?}"),
    }
}
