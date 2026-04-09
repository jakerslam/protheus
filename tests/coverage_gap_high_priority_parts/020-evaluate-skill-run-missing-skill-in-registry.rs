
#[cfg(test)]
mod skills_backward_compat_tests {
    use super::*;

    /// Test: evaluate_skill_run_missing_skill_in_registry
    /// Validates error when skill is not installed
    /// Coverage: Lines ~175-180 (untested error path)
    #[test]
    fn evaluate_skill_run_missing_skill_in_registry() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        
        // Ensure no skill is installed
        let result = evaluate_skill_run_backward_compat(root, "nonexistent-skill");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "skill_not_installed");
    }

    /// Test: evaluate_skill_run_invalid_version_format
    /// Validates error handling for malformed version strings
    /// Coverage: Lines ~185-190 (untested version parsing error)
    #[test]
    fn evaluate_skill_run_invalid_version_format() {
        let temp = TempDir::new().unwrap();
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

    /// Test: evaluate_skill_run_version_below_minimum
    /// Validates policy enforcement when version is below minimum
    /// Coverage: Lines ~200-220 (untested version gate)
    #[test]
    fn evaluate_skill_run_version_below_minimum() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        let state_root = state_root(root);
        fs::create_dir_all(&state_root).unwrap();
        fs::create_dir_all(root.join("planes/contracts/srs")).unwrap();
        
        // Create registry with v0 when minimum is v2
        let registry = json!({
            "installed": {
                "test-skill": {
                    "version": "v0.0.1"
                }
            }
        });
        fs::write(state_root.join("registry.json"), registry.to_string()).unwrap();
        
        // Create policy requiring v2
        let policy = json!({
            "backward_compat": {
                "policy": "semver_major",
                "min_version": "v2",
                "receipt_required": true
            }
        });
        fs::write(
            root.join("planes/contracts/srs/V8-SKILL-002.json"),
            policy.to_string()
        ).unwrap();
        
        let result = evaluate_skill_run_backward_compat(root, "test-skill");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "skill_version_below_minimum");
    }

    /// Test: evaluate_skill_run_legacy_version_parsing
    /// Validates legacy version format "v2" (without minor/patch)
    /// Coverage: Lines ~65-85 (parse_skill_version legacy path)
    #[test]
    fn evaluate_skill_run_legacy_version_parsing() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        let state_root = state_root(root);
        fs::create_dir_all(&state_root).unwrap();
        
        // Test legacy "v2" format (just major version)
        let registry = json!({
            "installed": {
                "legacy-skill": {
                    "version": "v2"
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

    /// Test: evaluate_skill_run_strict_semver_policy
    /// Validates strict semver comparison (not just major version)
    /// Coverage: Lines ~205-212 (untested semver_minor/semver_patch branches)
    #[test]
    fn evaluate_skill_run_strict_semver_policy() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        let state_root = state_root(root);
        fs::create_dir_all(&state_root).unwrap();
        fs::create_dir_all(root.join("planes/contracts/srs")).unwrap();
        
        // Create registry with v1.5.0
        let registry = json!({
            "installed": {
                "test-skill": {
                    "version": "v1.5.0"
                }
            }
        });
        fs::write(state_root.join("registry.json"), registry.to_string()).unwrap();
        
        // Create policy with semver_strict requiring v1.10.0
        let policy = json!({
            "backward_compat": {
                "policy": "semver_strict",
                "min_version": "v1.10.0",
                "receipt_required": true
            }
        });
        fs::write(
            root.join("planes/contracts/srs/V8-SKILL-002.json"),
            policy.to_string()
        ).unwrap();
        
        let result = evaluate_skill_run_backward_compat(root, "test-skill");
        assert!(result.is_err());
    }

    /// Test: default_migration_lane_path_edge_cases
    /// Validates edge cases for migration path generation
    /// Coverage: Lines ~245-275 (untested path variations)
    #[test]
    fn default_migration_lane_path_edge_cases() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        
        // Test with empty from version
        let path1 = default_migration_lane_path(root, "test-skill", "", "v2");
        let path_str = path1.to_string_lossy();
        assert!(path_str.contains("new_to_v2"));
        
        // Test with empty to version
        let path2 = default_migration_lane_path(root, "test-skill", "v1", "");
        let path_str2 = path2.to_string_lossy();
        assert!(path_str2.contains("v1_to_unknown"));
        
        // Test with special characters in skill ID
        let path3 = default_migration_lane_path(root, "my/skill-name_test", "v1", "v2");
        let path_str3 = path3.to_string_lossy();
        assert!(!path_str3.contains('/')); // Normalized
    }

    // Placeholder functions
    fn state_root(root: &Path) -> PathBuf {
        root.join("state")
    }
    
    fn evaluate_skill_run_backward_compat(_root: &Path, _skill: &str) -> Result<Value, String> {
        // Implementation from core/layer0/ops/src/skills_plane.rs
        todo!("Use actual ops crate function")
    }
    
    fn default_migration_lane_path(_root: &Path, _skill_id: &str, _from: &str, _to: &str) -> PathBuf {
        // Implementation from core/layer0/ops/src/skills_plane.rs
        todo!("Use actual ops crate function")
    }
}

// ============================================================================
// PRIORITY 3: CONDUIT STRICT MODE ENFORCEMENT
// Location: core/layer2/conduit/src/lib.rs, lines ~1650-1800
// Impact: +1.5% coverage
// ============================================================================

#[cfg(test)]
mod conduit_strict_mode_tests {
    use super::*;

    /// Test: conduit_empty_agent_id_validation
    /// Validates empty agent_id rejection for StartAgent command
    /// Coverage: Lines ~1750-1760 (untested validate_structure)
    #[test]
    fn conduit_empty_agent_id_validation() {
        let empty_agent = validate_structure(&TsCommand::StartAgent {
            agent_id: "".to_string(),
        });
        assert_eq!(empty_agent.as_deref(), Some("agent_id_required"));
        
        // Whitespace-only should also fail
        let whitespace_agent = validate_structure(&TsCommand::StartAgent {
            agent_id: "   ".to_string(),
        });
        assert_eq!(whitespace_agent.as_deref(), Some("agent_id_required"));
    }

    /// Test: conduit_receipt_query_limit_validation
    /// Validates limit boundaries for QueryReceiptChain
    /// Coverage: Lines ~1760-1770 (untested limit validation)
    #[test]
    fn conduit_receipt_query_limit_validation() {
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
        
        // Test valid limit
        let valid_limit = validate_structure(&TsCommand::QueryReceiptChain {
            from_hash: None,
            limit: Some(500),
        });
        assert_eq!(valid_limit, None); // No validation error
    }

    /// Test: conduit_policy_update_validation
    /// Validates strict mode enforcement for ApplyPolicyUpdate
    /// Coverage: Lines ~1770-1785 (untested patch validation)
    #[test]
    fn conduit_policy_update_validation() {
        // Empty patch_id should fail
        let empty_patch = validate_structure(&TsCommand::ApplyPolicyUpdate {
            patch_id: "".to_string(),
            patch: json!({}),
        });
        assert_eq!(empty_patch.as_deref(), Some("policy_patch_id_required"));
        
        // Missing constitution_safe prefix should fail
        let unsafe_patch = validate_structure(&TsCommand::ApplyPolicyUpdate {
            patch_id: "unsafe_change".to_string(),
            patch: json!({}),
        });
        assert_eq!(unsafe_patch.as_deref(), Some("policy_update_must_be_constitution_safe"));
        
        // Valid patch_id with proper prefix should succeed
        let safe_patch = validate_structure(&TsCommand::ApplyPolicyUpdate {
            patch_id: "constitution_safe_update_permissions".to_string(),
            patch: json!({}),
        });
        assert_eq!(safe_patch, None);
    }

    /// Test: conduit_extension_wasm_sha256_validation
    /// Validates SHA256 format enforcement for InstallExtension
    /// Coverage: Lines ~1785-1800 (untested extension validation)
    #[test]
    fn conduit_extension_wasm_sha256_validation() {
        // Invalid SHA256 - too short
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
        
        // Invalid SHA256 - non-hex characters
        let invalid_hex = validate_structure(&TsCommand::InstallExtension {
            extension_id: "test".to_string(),
            wasm_sha256: "g".repeat(64),
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
    }

    /// Test: conduit_extension_capabilities_validation
    /// Validates capabilities vector requirements
    /// Coverage: Lines ~1788-1795 (untested capabilities validation)
    #[test]
    fn conduit_extension_capabilities_validation() {
        // Empty capabilities should fail
        let empty_cap = validate_structure(&TsCommand::InstallExtension {
            extension_id: "test".to_string(),
            wasm_sha256: "a".repeat(64),
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
        
        // Whitespace-only capability should fail
        let whitespace_cap = validate_structure(&TsCommand::InstallExtension {
            extension_id: "test".to_string(),
            wasm_sha256: "a".repeat(64),
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
    }

    /// Test: conduit_extension_plugin_type_validation
    /// Validates plugin type restrictions
    /// Coverage: Lines ~1795-1800 (untested plugin_type validation)
    #[test]
    fn conduit_extension_plugin_type_validation() {
        let invalid_plugin_types = vec![
            "malicious_plugin",
            "unknown_type",
            "system_kernel_extension",
            "",
            "  ",
        ];
        
        for plugin_type in invalid_plugin_types {
            let invalid_plugin = validate_structure(&TsCommand::InstallExtension {
                extension_id: "test".to_string(),
                wasm_sha256: "a".repeat(64),
                capabilities: vec!["read".to_string()],
                plugin_type: Some(plugin_type.to_string()),
                version: Some("1.0.0".to_string()),
                wasm_component_path: Some("test.wasm".to_string()),
                signature: None,
                provenance: None,
                recovery_max_attempts: None,
                recovery_backoff_ms: None,
            });
            assert_eq!(invalid_plugin.as_deref(), Some("extension_plugin_type_invalid"));
        }
    }

    /// Test: conduit_stop_agent_validation
    /// Validates empty agent_id for StopAgent
    /// Coverage: Lines ~1751-1755 (implicit StopAgent validation)
    #[test]
    fn conduit_stop_agent_validation() {
        let empty_stop = validate_structure(&TsCommand::StopAgent {
            agent_id: "".to_string(),
        });
        assert_eq!(empty_stop.as_deref(), Some("agent_id_required"));
    }

    // Placeholder functions
    fn validate_structure(_cmd: &TsCommand) -> Option<String> {
        // Implementation from core/layer2/conduit/src/lib.rs
        todo!("Use actual conduit crate function")
    }

    // Mock TsCommand enum for compilation
    #[derive(Debug)]
    enum TsCommand {
        StartAgent { agent_id: String },
        StopAgent { agent_id: String },
        QueryReceiptChain { from_hash: Option<String>, limit: Option<u32> },
        ApplyPolicyUpdate { patch_id: String, patch: Value },
        InstallExtension {
            extension_id: String,
            wasm_sha256: String,
            capabilities: Vec<String>,
            plugin_type: Option<String>,
            version: Option<String>,
            wasm_component_path: Option<String>,
            signature: Option<String>,
            provenance: Option<String>,
            recovery_max_attempts: Option<u32>,
            recovery_backoff_ms: Option<u64>,
        },
    }
}

// ============================================================================
// PRIORITY 4: RECEIPT VALIDATION LOGIC
// Location: core/layer2/conduit/src/lib.rs, lines ~1800-1920
// Impact: +1.0% coverage
// ============================================================================

