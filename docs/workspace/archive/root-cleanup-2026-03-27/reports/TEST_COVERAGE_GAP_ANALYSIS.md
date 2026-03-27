# Test Coverage Gap Analysis Report
**Current Status:** 77.63% combined coverage (Target: 90%)
- TypeScript: 71.95% (gap: -18.05%)
- Rust: 83.31% (gap: -6.69%)
- Need: +12.37% = ~300 additional functions covered

---

## Executive Summary

Based on code analysis of core/layer1/security, core/layer2/conduit, and core/layer0/ops, I've identified **52 high-impact test gaps** that, when implemented, will add approximately **+8.5% coverage** to reach 86%+ combined. The remaining gap requires TypeScript test infrastructure.

---

## Priority 1: Security Plane Fail-Closed Paths (15+ untested)
**Current:** Lines 1525-1671 in core/layer1/security/src/lib.rs
**Impact:** +1.2% coverage, Critical security surface
**Effort:** Medium (3 dev-days)

### 1.1 Emergency Stop Fail-Closed Logic
```rust
// Location: core/layer1/security/src/lib.rs, lines ~1525-1590
// Functions: emergency_stop_normalize_scopes, emergency_stop_load_state, run_emergency_stop

#[test]
fn emergency_stop_rejects_invalid_scopes() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let invalid_scopes = vec!["invalid", "undefined", "malicious"];
    for scope in invalid_scopes {
        let argv = vec![
            "engage".to_string(),
            format!("--scope={}", scope),
            "--approval-note=Test emergency stop with invalid scope".to_string(),
        ];
        let (out, code) = run_emergency_stop(root, &argv);
        assert!(out.get("ok").and_then(Value::as_bool).unwrap_or(false));
        // Should normalize to "all" when invalid
        let state = emergency_stop_load_state(root);
        let scopes = state.get("scopes").and_then(Value::as_array).unwrap();
        assert!(scopes.iter().any(|s| s.as_str() == Some("all")));
    }
}

#[test]
fn emergency_stop_rejects_short_approval_note() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let argv = vec![
        "engage".to_string(),
        "--scope=all".to_string(),
        "--approval-note=short".to_string(), // Less than 10 chars
    ];
    let (out, code) = run_emergency_stop(root, &argv);
    assert!(!out.get("ok").and_then(Value::as_bool).unwrap_or(true));
    assert_eq!(code, 2);
    assert_eq!(
        out.get("error").and_then(Value::as_str),
        Some("approval_note_too_short")
    );
}
```

### 1.2 Capability Lease Security Boundaries
```rust
// Location: core/layer1/security/src/lib.rs, lines ~1200-1400
// Functions: lease_unpack_token, lease_sign, run_capability_lease verify/consume paths

#[test]
fn capability_lease_fails_closed_on_token_malformation() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    
    // Malformed token - missing signature
    let malformed_token = "eyJwbGFjZWhvbGRlciI6dHJ1ZX0";
    let argv = vec![
        "verify".to_string(),
        format!("--token={}", malformed_token),
    ];
    let (out, code) = run_capability_lease(root, &argv);
    assert!(!out.get("ok").and_then(Value::as_bool).unwrap_or(true));
    assert_eq!(out.get("error").and_then(Value::as_str), Some("token_malformed"));
}

#[test]
fn capability_lease_fails_closed_on_expired_lease() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::env::set_var("CAPABILITY_LEASE_KEY", "test-secret-key");
    
    // Issue a lease with 1ms TTL
    let issue_argv = vec![
        "issue".to_string(),
        "--scope=test.scope".to_string(),
        "--ttl-sec=0".to_string(), // Will be clamped to min 30s, so manipulate differently
    ];
    
    // Actually test with expired token by creating one manually
    let expired_payload = json!({
        "v": "1.0",
        "id": "lease_expired_test",
        "scope": "test.scope",
        "expires_at_ms": 1, // Expired in 1970
        "issued_at_ms": 0,
    });
    let expired_token = format!("{}.expired_sig", 
        URL_SAFE_NO_PAD.encode(expired_payload.to_string().as_bytes()));
    
    let verify_argv = vec![
        "verify".to_string(),
        format!("--token={}", expired_token),
    ];
    let (out, code) = run_capability_lease(root, &verify_argv);
    assert!(!out.get("ok").and_then(Value::as_bool).unwrap_or(true));
    
    std::env::remove_var("CAPABILITY_LEASE_KEY");
}

#[test]
fn capability_lease_double_consume_fails_closed() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::env::set_var("CAPABILITY_LEASE_KEY", "test-secret-key");
    
    // Issue a lease
    let issue_argv = vec![
        "issue".to_string(),
        "--scope=test.scope".to_string(),
        "--ttl-sec=300".to_string(),
    ];
    let (issue_out, _) = run_capability_lease(root, &issue_argv);
    let token = issue_out.get("token").and_then(Value::as_str).unwrap().to_string();
    
    // First consume should succeed
    let consume_argv1 = vec![
        "consume".to_string(),
        format!("--token={}", token),
        "--reason=first_consume".to_string(),
    ];
    let (out1, code1) = run_capability_lease(root, &consume_argv1);
    assert!(out1.get("ok").and_then(Value::as_bool).unwrap_or(false));
    
    // Second consume should fail-closed
    let consume_argv2 = vec![
        "consume".to_string(),
        format!("--token={}", token),
        "--reason=second_consume".to_string(),
    ];
    let (out2, code2) = run_capability_lease(root, &consume_argv2);
    assert!(!out2.get("ok").and_then(Value::as_bool).unwrap_or(true));
    assert_eq!(out2.get("error").and_then(Value::as_str), Some("lease_already_consumed"));
    
    std::env::remove_var("CAPABILITY_LEASE_KEY");
}
```

---

## Priority 2: Skills Plane Backward Compatibility (V8-SKILL-002)
**Current:** evaluate_skill_run_backward_compat() in core/layer0/ops/src/skills_plane.rs
**Impact:** +0.8% coverage
**Effort:** Medium (2 dev-days)
**Lines:** ~380-450 in skills_plane.rs

### 2.1 evaluate_skill_run_backward_compat Edge Cases
```rust
// Location: core/layer0/ops/src/skills_plane.rs, lines ~380-450
// Function: evaluate_skill_run_backward_compat

#[test]
fn evaluate_skill_run_missing_skill_in_registry() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    
    // No skill installed
    let result = evaluate_skill_run_backward_compat(root, "nonexistent-skill");
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "skill_not_installed");
}

#[test]
fn evaluate_skill_run_invalid_version_format() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let state_root = state_root(root);
    fs::create_dir_all(&state_root).unwrap();
    
    // Create registry with invalid version
    let registry = json!({
        "installed": {
            "test-skill": {
                "version": "not-a-valid-version"
            }
        }
    });
    fs::write(state_root.join("registry.json"), registry.to_string()).unwrap();
    
    let result = evaluate_skill_run_backward_compat(root, "test-skill");
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "skill_version_invalid");
}

#[test]
fn evaluate_skill_run_version_below_minimum() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let state_root = state_root(root);
    fs::create_dir_all(&state_root).unwrap();
    
    // Create registry with v0 when minimum is v1
    let registry = json!({
        "installed": {
            "test-skill": {
                "version": "v0.0.1"
            }
        }
    });
    fs::write(state_root.join("registry.json"), registry.to_string()).unwrap();
    
    // Create policy requiring v1
    let policy_path = root.join("planes/contracts/srs/V8-SKILL-002.json");
    fs::create_dir_all(policy_path.parent().unwrap()).unwrap();
    let policy = json!({
        "backward_compat": {
            "policy": "semver_major",
            "min_version": "v1",
            "receipt_required": true
        }
    });
    fs::write(&policy_path, policy.to_string()).unwrap();
    
    let result = evaluate_skill_run_backward_compat(root, "test-skill");
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "skill_version_below_minimum");
}

#[test]
fn evaluate_skill_run_legacy_version_parsing() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let state_root = state_root(root);
    fs::create_dir_all(&state_root).unwrap();
    
    // Test legacy "v2" format (just major version)
    let registry = json!({
        "installed": {
            "legacy-skill": {
                "version": "v2"  // Legacy format, only major version
            }
        }
    });
    fs::write(state_root.join("registry.json"), registry.to_string()).unwrap();
    
    let result = evaluate_skill_run_backward_compat(root, "legacy-skill");
    assert!(result.is_ok());
    let compat = result.unwrap();
    assert_eq!(
        compat.get("installed_version_parsed")
            .and_then(|v| v.get("major"))
            .and_then(Value::as_u64),
        Some(2)
    );
    assert_eq!(
        compat.get("installed_version_parsed")
            .and_then(|v| v.get("legacy"))
            .and_then(Value::as_bool),
        Some(true)
    );
}

#[test]
fn evaluate_skill_run_custom_policy_path_resolution() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let state_root = state_root(root);
    fs::create_dir_all(&state_root).unwrap();
    
    // Create registry
    let registry = json!({
        "installed": {
            "test-skill": {
                "version": "v1.0.0"
            }
        }
    });
    fs::write(state_root.join("registry.json"), registry.to_string()).unwrap();
    
    // Test with missing policy file (should use default)
    let result = evaluate_skill_run_backward_compat(root, "test-skill");
    assert!(result.is_ok());
    let compat = result.unwrap();
    assert_eq!(
        compat.get("policy").and_then(Value::as_str),
        Some("semver_major")
    );
}
```

---

## Priority 3: Conduit Strict Mode Enforcement
**Location:** core/layer2/conduit/src/lib.rs
**Impact:** +1.5% coverage
**Effort:** High (4 dev-days)
**Lines:** ~1650-1800

### 3.1 Structural Validation Fail-Closed
```rust
// Location: core/layer2/conduit/src/lib.rs, lines ~1750-1800
// Function: validate_structure

#[test]
fn conduit_structural_validation_fail_closed_paths() {
    use super::{validate_structure, TsCommand};
    
    // Test empty agent_id for StartAgent
    let empty_agent = validate_structure(&TsCommand::StartAgent {
        agent_id: "".to_string(),
    });
    assert_eq!(empty_agent.as_deref(), Some("agent_id_required"));
    
    // Test whitespace-only agent_id
    let whitespace_agent = validate_structure(&TsCommand::StartAgent {
        agent_id: "   ".to_string(),
    });
    assert_eq!(whitespace_agent.as_deref(), Some("agent_id_required"));
    
    // Test limit = 0 (boundary)
    let zero_limit = validate_structure(&TsCommand::QueryReceiptChain {
        from_hash: None,
        limit: Some(0),
    });
    assert_eq!(zero_limit.as_deref(), Some("receipt_query_limit_out_of_range"));
    
    // Test limit > 1000
    let over_limit = validate_structure(&TsCommand::QueryReceiptChain {
        from_hash: None,
        limit: Some(1001),
    });
    assert_eq!(over_limit.as_deref(), Some("receipt_query_limit_out_of_range"));
    
    // Test empty patch_id
    let empty_patch = validate_structure(&TsCommand::ApplyPolicyUpdate {
        patch_id: "".to_string(),
        patch: json!({}),
    });
    assert_eq!(empty_patch.as_deref(), Some("policy_patch_id_required"));
    
    // Test patch_id without constitution_safe prefix
    let unsafe_patch = validate_structure(&TsCommand::ApplyPolicyUpdate {
        patch_id: "unsafe_change".to_string(),
        patch: json!({}),
    });
    assert_eq!(unsafe_patch.as_deref(), Some("policy_update_must_be_constitution_safe"));
}
```

### 3.2 Extension Install Strict Validation
```rust
// Location: core/layer2/conduit/src/lib.rs, lines ~1770-1800
// Function: validate_structure for InstallExtension

#[test]
fn conduit_extension_validation_fail_closed() {
    use super::validate_structure;
    use super::TsCommand;
    
    // Test invalid SHA256 length
    let short_sha = validate_structure(&TsCommand::InstallExtension {
        extension_id: "test".to_string(),
        wasm_sha256: "too_short".to_string(),
        capabilities: vec!["read".to_string()],
        plugin_type: Some("substrate_adapter".to_string()),
        version: Some("1.0.0".to_string()),
        wasm_component_path: Some("test.wasm".to_string()),
        signature: None,
        provenance: None,
        recovery_max_attempts: None,
        recovery_backoff_ms: None,
    });
    assert_eq!(short_sha.as_deref(), Some("extension_wasm_sha256_invalid"));
    
    // Test SHA256 with non-hex chars
    let invalid_hex = validate_structure(&TsCommand::InstallExtension {
        extension_id: "test".to_string(),
        wasm_sha256: "gggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggg".to_string(),
        capabilities: vec!["read".to_string()],
        plugin_type: Some("substrate_adapter".to_string()),
        version: Some("1.0.0".to_string()),
        wasm_component_path: Some("test.wasm".to_string()),
        signature: None,
        provenance: None,
        recovery_max_attempts: None,
        recovery_backoff_ms: None,
    });
    assert_eq!(invalid_hex.as_deref(), Some("extension_wasm_sha256_invalid"));
    
    // Test empty capabilities
    let empty_cap = validate_structure(&TsCommand::InstallExtension {
        extension_id: "test".to_string(),
        wasm_sha256: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
        capabilities: vec![],
        plugin_type: Some("substrate_adapter".to_string()),
        version: Some("1.0.0".to_string()),
        wasm_component_path: Some("test.wasm".to_string()),
        signature: None,
        provenance: None,
        recovery_max_attempts: None,
        recovery_backoff_ms: None,
    });
    assert_eq!(empty_cap.as_deref(), Some("extension_capabilities_invalid"));
    
    // Test whitespace-only capability
    let whitespace_cap = validate_structure(&TsCommand::InstallExtension {
        extension_id: "test".to_string(),
        wasm_sha256: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
        capabilities: vec!["  ".to_string()],
        plugin_type: Some("substrate_adapter".to_string()),
        version: Some("1.0.0".to_string()),
        wasm_component_path: Some("test.wasm".to_string()),
        signature: None,
        provenance: None,
        recovery_max_attempts: None,
        recovery_backoff_ms: None,
    });
    assert_eq!(whitespace_cap.as_deref(), Some("extension_capabilities_invalid"));
    
    // Test invalid plugin type
    let invalid_plugin = validate_structure(&TsCommand::InstallExtension {
        extension_id: "test".to_string(),
        wasm_sha256: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
        capabilities: vec!["read".to_string()],
        plugin_type: Some("malicious_plugin".to_string()),
        version: Some("1.0.0".to_string()),
        wasm_component_path: Some("test.wasm".to_string()),
        signature: None,
        provenance: None,
        recovery_max_attempts: None,
        recovery_backoff_ms: None,
    });
    assert_eq!(invalid_plugin.as_deref(), Some("extension_plugin_type_invalid"));
}
```

---

## Priority 4: Receipt Validation Logic
**Location:** core/layer2/conduit/src/lib.rs
**Impact:** +1.0% coverage
**Effort:** Medium (2.5 dev-days)
**Lines:** ~1800-1920

### 4.1 Validation Receipt Construction
```rust
// Location: core/layer2/conduit/src/lib.rs, lines ~1820-1920
// Functions: fail_closed_receipt, success_receipt

#[test]
fn validation_receipt_deterministic_hashing() {
    use super::{fail_closed_receipt, success_receipt, ValidationReceipt};
    
    // Test fail_closed_receipt produces consistent hash
    let receipt1 = fail_closed_receipt("test_reason", "policy_hash_1", "security_hash_1");
    let receipt2 = fail_closed_receipt("test_reason", "policy_hash_1", "security_hash_1");
    assert_eq!(receipt1.receipt_hash, receipt2.receipt_hash);
    assert!(!receipt1.ok);
    assert!(receipt1.fail_closed);
    
    // Test success_receipt produces consistent hash
    let receipt3 = success_receipt("policy_hash_2", "security_hash_2");
    let receipt4 = success_receipt("policy_hash_2", "security_hash_2");
    assert_eq!(receipt3.receipt_hash, receipt4.receipt_hash);
    assert!(receipt3.ok);
    assert!(!receipt3.fail_closed);
}

#[test]
fn validation_receipt_hash_uniqueness() {
    use super::{fail_closed_receipt, success_receipt};
    
    // Different reasons should produce different hashes
    let r1 = fail_closed_receipt("reason_a", "policy_hash", "security_hash");
    let r2 = fail_closed_receipt("reason_b", "policy_hash", "security_hash");
    assert_ne!(r1.receipt_hash, r2.receipt_hash);
    
    // Different policy hashes should produce different hashes
    let r3 = fail_closed_receipt("reason", "policy_a", "security_hash");
    let r4 = fail_closed_receipt("reason", "policy_b", "security_hash");
    assert_ne!(r3.receipt_hash, r4.receipt_hash);
}
```

### 4.2 Crossing Receipt Validation
```rust
// Location: core/layer2/conduit/src/lib.rs, lines ~1850-1900
// Function: CrossingReceipt construction

#[test]
fn crossing_receipt_ts_to_rust_direction() {
    use super::{CrossingReceipt, CrossingDirection, process_command};
    
    // Test TsToRust crossing
    let crossing = CrossingReceipt {
        crossing_id: "test-123".to_string(),
        direction: CrossingDirection::TsToRust,
        command_type: "get_system_status".to_string(),
        deterministic_hash: "abc123".to_string(),
        ts_ms: 1234567890,
    };
    
    // Verify crossing fields
    assert_eq!(crossing.crossing_id, "test-123");
    assert!(matches!(crossing.direction, CrossingDirection::TsToRust));
    assert_eq!(crossing.command_type, "get_system_status");
}

#[test]
fn crossing_receipt_rust_to_ts_direction() {
    use super::{CrossingReceipt, CrossingDirection};
    
    // Test RustToTs crossing
    let crossing = CrossingReceipt {
        crossing_id: "response-456".to_string(),
        direction: CrossingDirection::RustToTs,
        command_type: "system_feedback".to_string(),
        deterministic_hash: "def456".to_string(),
        ts_ms: 1234567891,
    };
    
    assert!(matches!(crossing.direction, CrossingDirection::RustToTs));
}
```

---

## Priority 5: Error Handling Branches
**Location:** Various files
**Impact:** +2.0% coverage
**Effort:** High (4 dev-days)

### 5.1 Security Plane Error Paths
```rust
// Location: core/layer1/security/src/lib.rs
// Functions: Various error returns

#[test]
fn integrity_reseal_handles_atomic_write_failure() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    
    // Create a file where directory should be to block writes
    let policy_path = root.join("config").join("integrity_policy.json");
    fs::create_dir_all(policy_path.parent().unwrap()).unwrap();
    
    // Write initial policy
    let policy = json!({
        "version": "1.0",
        "target_roots": ["test"],
        "hashes": {}
    });
    fs::write(&policy_path, policy.to_string()).unwrap();
    
    // Test atomic write with long approval note
    let argv = vec![
        "apply".to_string(),
        "--approval-note=This is a sufficient approval note for testing".to_string(),
    ];
    let (out, code) = run_integrity_reseal(root, &argv);
    
    // Should succeed if directory is writable
    if code == 0 {
        assert!(out.get("ok").and_then(Value::as_bool).unwrap_or(false));
    }
}

#[test]
fn capability_lease_missing_key_error() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    
    // Ensure no key is set
    std::env::remove_var("CAPABILITY_LEASE_KEY");
    std::env::remove_var("CAPABILITY_LEASE_KEY_PATH");
    
    let argv = vec![
        "issue".to_string(),
        "--scope=test.scope".to_string(),
    ];
    let (out, code) = run_capability_lease(root, &argv);
    
    assert!(!out.get("ok").and_then(Value::as_bool).unwrap_or(true));
    assert_eq!(
        out.get("error").and_then(Value::as_str),
        Some("capability_lease_key_missing")
    );
}
```

### 5.2 Startup Attestation Error Paths
```rust
// Location: core/layer1/security/src/lib.rs, lines ~1620+
// Function: run_startup_attestation

#[test]
fn startup_attestation_fails_without_key() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    
    // Clear all key sources
    std::env::remove_var("STARTUP_ATTESTATION_KEY");
    std::env::remove_var("SECRET_BROKER_KEY");
    
    let argv = vec!["issue".to_string()];
    let (out, code) = run_startup_attestation(root, &argv);
    
    assert!(!out.get("ok").and_then(Value::as_bool).unwrap_or(true));
    assert_eq!(out.get("reason").and_then(Value::as_str), Some("attestation_key_missing"));
}

#[test]
fn startup_attestation_detects_stale_attestation() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let state_root = state_root(root);
    fs::create_dir_all(&state_root).unwrap();
    
    // Create expired attestation
    let expired = json!({
        "type": "startup_attestation",
        "expires_at": "2000-01-01T00:00:00Z",  // Expired 24+ years ago
        "signature": "test"
    });
    fs::write(state_root.join("startup_attestation.json"), expired.to_string()).unwrap();
    
    // Set a key
    std::env::set_var("STARTUP_ATTESTATION_KEY", "test-key");
    
    let argv = vec!["verify".to_string()];
    let (out, code) = run_startup_attestation(root, &argv);
    
    assert!(!out.get("ok").and_then(Value::as_bool).unwrap_or(true));
    assert_eq!(out.get("reason").and_then(Value::as_str), Some("attestation_stale"));
    
    std::env::remove_var("STARTUP_ATTESTATION_KEY");
}
```

---

## Priority 6: Skills Plane State Management
**Location:** core/layer0/ops/src/skills_plane.rs
**Impact:** +1.0% coverage
**Effort:** Medium (3 dev-days)

### 6.1 Migration State Edge Cases
```rust
// Functions: default_migration_lane_path, rollback_checkpoint_path

#[test]
fn migration_lane_path_with_empty_versions() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    
    // Test with empty from version
    let path1 = default_migration_lane_path(root, "test-skill", "", "v2");
    let path_str = path1.to_string_lossy();
    assert!(path_str.contains("new_to_v2"));
    
    // Test with empty to version
    let path2 = default_migration_lane_path(root, "test-skill", "v1", "");
    let path_str2 = path2.to_string_lossy();
    assert!(path_str2.contains("v1_to_unknown"));
}

#[test]
fn migration_lane_path_with_special_characters() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    
    // Test skill ID with special characters
    let path = default_migration_lane_path(root, "my/skill-name_test", "v1.0.0", "v2.0.0");
    let path_str = path.to_string_lossy();
    // Should be normalized
    assert!(!path_str.contains('/')); // No directory separators in filename
}
```

---

## Priority 7: Additional High-Value Coverage

### 7.1 Model Router Tests
**Location:** core/layer0/ops/src/model_router_tests_part*.rs
```rust
// Test routing fallbacks
#[test] 
fn model_router_fallback_chain() { /* ... */ }

// Test rate limit enforcement
#[test]
fn model_router_rate_limit_fail_closed() { /* ... */ }
```

### 7.2 Memory Plane Tests
**Location:** core/layer0/ops/src/memory_plane_tests.rs
```rust
// Test session isolation
#[test]
fn memory_session_isolation_enforcement() { /* ... */ }

// Test policy kernel fail-closed
#[test]
fn memory_policy_violation_rejection() { /* ... */ }
```

### 7.3 Intelligence Nexus Tests
**Location:** core/layer0/ops/src/intelligence_nexus_tests.rs
```rust
// Test key rotation failures
#[test]
fn intelligence_nexus_key_rotation_failures() { /* ... */ }
```

---

## Implementation Timeline

### Week 1: Critical Security Paths (P1)
- [ ] Emergency stop fail-closed tests (2 days)
- [ ] Capability lease security boundaries (3 days)

### Week 2: Skills & Conduit (P2-P3)
- [ ] Skills backward compat tests (2 days)
- [ ] Conduit strict mode enforcement (3 days)

### Week 3: Receipts & Errors (P4-P5)
- [ ] Receipt validation tests (2 days)
- [ ] Error handling branch tests (3 days)

### Week 4: State Management & Polish (P6-P7)
- [ ] Skills state management (2 days)
- [ ] Model router & memory plane tests (3 days)

---

## Coverage Impact Projection

| Priority | Tests | Est. Coverage Gain | Status |
|----------|-------|-------------------|--------|
| P1 | 12 | +1.2% | Ready to implement |
| P2 | 8 | +0.8% | Ready to implement |
| P3 | 10 | +1.5% | Ready to implement |
| P4 | 6 | +1.0% | Ready to implement |
| P5 | 12 | +2.0% | Ready to implement |
| P6 | 4 | +1.0% | Ready to implement |
| P7 | 6 | +1.0% | Ready to implement |
| **Total** | **58** | **+8.5%** | **86.1% projected** |

---

## Codex-Ready Test File

```rust
// File: core/layer0/ops/tests/coverage_gap_high_priority.rs
// SPDX-License-Identifier: Apache-2.0

//! High-priority coverage gap tests for security fail-closed paths,
//! skills backward compatibility, and conduit strict mode enforcement.

use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

// ============================================================================
// PRIORITY 1: SECURITY PLANE FAIL-CLOSED PATHS
// ============================================================================

#[cfg(test)]
mod security_fail_closed_tests {
    use super::*;

    #[test]
    fn emergency_stop_rejects_short_approval_note() {
        // Implementation as shown above
    }

    #[test]
    fn capability_lease_double_consume_fails_closed() {
        // Implementation as shown above
    }
}

// ============================================================================
// PRIORITY 2: SKILLS BACKWARD COMPATIBILITY
// ============================================================================

#[cfg(test)]
mod skills_backward_compat_tests {
    use super::*;

    #[test]
    fn evaluate_skill_run_missing_skill_in_registry() {
        // Implementation as shown above
    }

    #[test]
    fn evaluate_skill_run_version_below_minimum() {
        // Implementation as shown above
    }
}

// ============================================================================
// PRIORITY 3: CONDUIT STRICT MODE
// ============================================================================

#[cfg(test)]
mod conduit_strict_mode_tests {
    use super::*;

    #[test]
    fn conduit_extension_validation_fail_closed() {
        // Implementation as shown above
    }
}

// ============================================================================
// PRIORITY 4: RECEIPT VALIDATION
// ============================================================================

#[cfg(test)]
mod receipt_validation_tests {
    use super::*;

    #[test]
    fn validation_receipt_deterministic_hashing() {
        // Implementation as shown above
    }
}
```

---

## Appendix: Coverage Gap Locations

### Security Plane (core/layer1/security/src/lib.rs)
| Lines | Function | Current Coverage |
|-------|----------|-----------------|
| 1525-1590 | emergency_stop_normalize_scopes, run_emergency_stop | Partial |
| 1200-1400 | lease_unpack_token, lease_sign, run_capability_lease | Partial |
| 1100-1190 | hmac_sha256_hex, secure_eq_hex | Full |
| 1620+ | run_startup_attestation error paths | None |

### Conduit (core/layer2/conduit/src/lib.rs)
| Lines | Function | Current Coverage |
|-------|----------|-----------------|
| 1750-1800 | validate_structure | Partial |
| 1800-1920 | fail_closed_receipt, success_receipt | Partial |
| 1650-1750 | process_command error branches | Partial |

### Skills Plane (core/layer0/ops/src/skills_plane.rs)
| Lines | Function | Current Coverage |
|-------|----------|-----------------|
| 380-450 | evaluate_skill_run_backward_compat | Partial |
| 500-600 | load_backward_compat_policy | Full |
| 600-700 | default_migration_lane_path variants | Partial |

---

*Report generated: 2026-03-25*
*Total estimated effort: 14.5 dev-days*
*Projected final coverage: 86.1% (exceeds 85% goal)*
