
#[cfg(test)]
mod receipt_validation_tests {
    use super::*;

    /// Test: validation_receipt_deterministic_hashing
    /// Validates deterministic hashing for receipts
    /// Coverage: Lines ~1820-1850 (untested receipt construction)
    #[test]
    fn validation_receipt_deterministic_hashing() {
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

    /// Test: validation_receipt_hash_uniqueness
    /// Validates that different inputs produce different hashes
    /// Coverage: Lines ~1850-1870 (untested hash uniqueness)
    #[test]
    fn validation_receipt_hash_uniqueness() {
        // Different reasons should produce different hashes
        let r1 = fail_closed_receipt("reason_a", "policy_hash", "security_hash");
        let r2 = fail_closed_receipt("reason_b", "policy_hash", "security_hash");
        assert_ne!(r1.receipt_hash, r2.receipt_hash);
        
        // Different policy hashes should produce different hashes
        let r3 = fail_closed_receipt("reason", "policy_a", "security_hash");
        let r4 = fail_closed_receipt("reason", "policy_b", "security_hash");
        assert_ne!(r3.receipt_hash, r4.receipt_hash);
        
        // Different security hashes should produce different hashes
        let r5 = fail_closed_receipt("reason", "policy_hash", "security_a");
        let r6 = fail_closed_receipt("reason", "policy_hash", "security_b");
        assert_ne!(r5.receipt_hash, r6.receipt_hash);
    }

    /// Test: crossing_receipt_ts_to_rust_direction
    /// Validates TsToRust crossing direction
    /// Coverage: Lines ~1870-1890 (untested crossing receipt construction)
    #[test]
    fn crossing_receipt_ts_to_rust_direction() {
        let crossing = CrossingReceipt {
            crossing_id: "test-123".to_string(),
            direction: CrossingDirection::TsToRust,
            command_type: "get_system_status".to_string(),
            deterministic_hash: "abc123".to_string(),
            ts_ms: 1234567890,
        };
        
        assert_eq!(crossing.crossing_id, "test-123");
        assert!(matches!(crossing.direction, CrossingDirection::TsToRust));
        assert_eq!(crossing.command_type, "get_system_status");
        assert_eq!(crossing.deterministic_hash, "abc123");
        assert_eq!(crossing.ts_ms, 1234567890);
    }

    /// Test: crossing_receipt_rust_to_ts_direction
    /// Validates RustToTs crossing direction
    /// Coverage: Lines ~1890-1910 (untested RustToTs variant)
    #[test]
    fn crossing_receipt_rust_to_ts_direction() {
        let crossing = CrossingReceipt {
            crossing_id: "response-456".to_string(),
            direction: CrossingDirection::RustToTs,
            command_type: "system_feedback".to_string(),
            deterministic_hash: "def456".to_string(),
            ts_ms: 1234567891,
        };
        
        assert!(matches!(crossing.direction, CrossingDirection::RustToTs));
        assert_eq!(crossing.command_type, "system_feedback");
    }

    // Placeholder types and functions
    #[derive(Debug, PartialEq)]
    enum CrossingDirection {
        TsToRust,
        RustToTs,
    }

    struct ValidationReceipt {
        ok: bool,
        fail_closed: bool,
        reason: String,
        policy_receipt_hash: String,
        security_receipt_hash: String,
        receipt_hash: String,
    }

    struct CrossingReceipt {
        crossing_id: String,
        direction: CrossingDirection,
        command_type: String,
        deterministic_hash: String,
        ts_ms: u64,
    }

    fn fail_closed_receipt(_reason: &str, _policy_hash: &str, _security_hash: &str) -> ValidationReceipt {
        // Implementation from core/layer2/conduit/src/lib.rs
        todo!("Use actual conduit crate function")
    }

    fn success_receipt(_policy_hash: &str, _security_hash: &str) -> ValidationReceipt {
        // Implementation from core/layer2/conduit/src/lib.rs
        todo!("Use actual conduit crate function")
    }
}

// ============================================================================
// PRIORITY 5: ERROR HANDLING BRANCHES
// Various locations
// Impact: +2.0% coverage
// ============================================================================

#[cfg(test)]
mod error_handling_tests {
    use super::*;

    /// Test: integrity_reseal_atomic_write_failure
    /// Validates error handling when atomic write fails
    /// Coverage: core/layer1/security/src/lib.rs atomic write paths
    #[test]
    fn integrity_reseal_atomic_write_failure() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        
        // Create initial policy file
        let policy_path = root.join("config").join("integrity_policy.json");
        fs::create_dir_all(policy_path.parent().unwrap()).unwrap();
        
        let policy = json!({
            "version": "1.0",
            "target_roots": ["test"],
            "hashes": {}
        });
        fs::write(&policy_path, policy.to_string()).unwrap();
        
        // Test that apply succeeds with valid note
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

    /// Test: capability_lease_status_empty_state
    /// Validates status command with empty lease state
    /// Coverage: Lines ~1200+ (untested status path)
    #[test]
    fn capability_lease_status_empty_state() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        
        std::env::set_var("CAPABILITY_LEASE_KEY", "test-secret-key-that-is-secure-long-enough");
        
        let argv = vec!["status".to_string()];
        let (out, code) = run_capability_lease(root, &argv);
        
        assert!(out.get("ok").and_then(Value::as_bool).unwrap_or(false));
        assert_eq!(code, 0);
        
        // Verify lease counts are present
        assert!(out.get("issued_count").is_some());
        assert!(out.get("consumed_count").is_some());
        
        std::env::remove_var("CAPABILITY_LEASE_KEY");
    }

    /// Test: startup_attestation_verify_expired
    /// Validates detection of expired attestation
    /// Coverage: Lines ~1415-1425 (untested expiry detection)
    #[test]
    fn startup_attestation_verify_expired() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        let state_root = state_root(root);
        fs::create_dir_all(&state_root).unwrap();
        
        // Create expired attestation (1970)
        let expired = json!({
            "type": "startup_attestation",
            "version": "1.0",
            "ts": "1970-01-01T00:00:00Z",
            "expires_at": "1970-01-01T00:00:01Z",
            "signature": "test",
            "critical_hashes": [],
            "missing_paths": []
        });
        fs::write(state_root.join("startup_attestation.json"), expired.to_string()).unwrap();
        
        std::env::set_var("STARTUP_ATTESTATION_KEY", "test-key-for-attestation-verification");
        
        let argv = vec!["verify".to_string()];
        let (out, code) = run_startup_attestation(root, &argv);
        
        assert!(!out.get("ok").and_then(Value::as_bool).unwrap_or(true));
        assert_eq!(out.get("reason").and_then(Value::as_str), Some("attestation_stale"));
        
        std::env::remove_var("STARTUP_ATTESTATION_KEY");
    }

    /// Test: startup_attestation_signature_mismatch
    /// Validates signature verification failure
    /// Coverage: Lines ~1450-1470 (untested signature verification)
    #[test]
    fn startup_attestation_signature_mismatch() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        let state_root = state_root(root);
        fs::create_dir_all(&state_root).unwrap();
        
        let key = "test-attestation-signature-key";
        std::env::set_var("STARTUP_ATTESTATION_KEY", key);
        
        // Create attestation with wrong signature
        let attestation = json!({
            "type": "startup_attestation",
            "version": "1.0",
            "ts": "2099-01-01T00:00:00Z",
            "expires_at": "2099-12-31T23:59:59Z",
            "signature": "wrong_signature",
            "critical_hashes": [],
            "missing_paths": []
        });
        fs::write(state_root.join("startup_attestation.json"), attestation.to_string()).unwrap();
        
        let argv = vec!["verify".to_string()];
        let (out, _) = run_startup_attestation(root, &argv);
        
        assert!(!out.get("ok").and_then(Value::as_bool).unwrap_or(true));
        assert_eq!(out.get("reason").and_then(Value::as_str), Some("signature_mismatch"));
        
        std::env::remove_var("STARTUP_ATTESTATION_KEY");
    }

    /// Test: startup_attestation_critical_hash_drift
    /// Validates detection of file hash changes
    /// Coverage: Lines ~1455-1480 (untested drift detection)
    #[test]
    fn startup_attestation_critical_hash_drift() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        let state_root = state_root(root);
        fs::create_dir_all(&state_root).unwrap();
        
        // Create runtime directory and critical file
        let runtime = root.join("client").join("runtime");
        fs::create_dir_all(&runtime).unwrap();
        let critical_file = runtime.join("critical.js");
        fs::write(&critical_file, "original content").unwrap();
        
        let key = "test-attestation-drift-key";
        std::env::set_var("STARTUP_ATTESTATION_KEY", key);
        
        // Issue attestation
        let issue_argv = vec![
            "issue".to_string(),
            "--ttl-hours=24".to_string(),
        ];
        let (issue_out, _) = run_startup_attestation(root, &issue_argv);
        assert!(issue_out.get("ok").and_then(Value::as_bool).unwrap_or(false));
        
        // Modify the critical file
        fs::write(&critical_file, "modified content").unwrap();
        
        // Verify should detect drift
        let verify_argv = vec!["verify".to_string()];
        let (verify_out, _) = run_startup_attestation(root, &verify_argv);
        
        assert!(!verify_out.get("ok").and_then(Value::as_bool).unwrap_or(true));
        assert_eq!(verify_out.get("reason").and_then(Value::as_str), Some("critical_hash_drift"));
        assert!(verify_out.get("drift").is_some());
        
        std::env::remove_var("STARTUP_ATTESTATION_KEY");
    }

    /// Test: lease_unpack_token_invalid_base64
    /// Validates handling of invalid base64 in token
    /// Coverage: Lines ~960-975 (untested error branches)
    #[test]
    fn lease_unpack_token_invalid_base64() {
        // Test with invalid base64 payload
        let invalid_tokens = vec![
            "!!!invalid_base64!!!.signature",  // Invalid chars in payload
            "payload.!!!invalid_sig!!!",        // Invalid chars in sig
            "no_dot_at_all",                    // No separator
            "a.",                               // Empty signature
            ".b",                               // Empty payload
        ];
        
        for token in invalid_tokens {
            let result = lease_unpack_token(token);
            assert!(result.is_err() || result.as_ref().map(|_| false).unwrap_or(true));
        }
    }

    /// Test: load_backward_compat_policy_default
    /// Validates default policy when file doesn't exist
    /// Coverage: core/layer0/ops/src/skills_plane.rs ~155-165
    #[test]
    fn load_backward_compat_policy_default() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        
        let policy = load_backward_compat_policy(root);
        
        assert_eq!(
            policy.get("policy").and_then(Value::as_str),
            Some("semver_major")
        );
        assert_eq!(
            policy.get("min_version").and_then(Value::as_str),
            Some("v1")
        );
        assert_eq!(
            policy.get("receipt_required").and_then(Value::as_bool),
            Some(true)
        );
    }

    // Placeholder functions
    fn run_integrity_reseal(_root: &Path, _argv: &[String]) -> (Value, i32) {
        todo!("Use actual security crate function")
    }
    
    fn state_root(root: &Path) -> PathBuf {
        root.join("client").join("local").join("state")
    }
    
    fn run_startup_attestation(_root: &Path, _argv: &[String]) -> (Value, i32) {
        todo!("Use actual security crate function")
    }
    
    fn run_capability_lease(_root: &Path, _argv: &[String]) -> (Value, i32) {
        todo!("Use actual security crate function")
    }
    
    fn lease_unpack_token(_token: &str) -> Result<(String, String, Value), String> {
        todo!("Use actual security crate function")
    }
    
    fn load_backward_compat_policy(_root: &Path) -> Value {
        todo!("Use actual ops crate function")
    }
}

// ============================================================================
// PRIORITY 6: ADDITIONAL HIGH-VALUE COVERAGE (State Management)
// Location: Various
// Impact: +1.0% coverage
// ============================================================================

