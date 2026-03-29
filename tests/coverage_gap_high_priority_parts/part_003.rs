
#[cfg(test)]
mod additional_coverage_tests {
    use super::*;

    /// Test: skills_plane_status_empty_state
    /// Validates status with no prior state
    #[test]
    fn skills_plane_status_empty_state() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        
        let status = skills_plane_status(root);
        assert!(status.get("ok").and_then(Value::as_bool).unwrap_or(false));
        assert_eq!(
            status.get("type").and_then(Value::as_str),
            Some("skills_plane_status")
        );
    }

    /// Test: skills_create_with_invalid_contract_version
    /// Validates error on contract version mismatch
    #[test]
    fn skills_create_with_invalid_contract_version() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        fs::create_dir_all(root.join("planes/contracts/skills")).unwrap();
        
        // Create contract with wrong version
        let contract = json!({
            "version": "v2",  // Wrong version
            "kind": "skill_scaffold_contract",
            "required_files": []
        });
        fs::write(
            root.join("planes/contracts/skills/skill_scaffold_contract_v1.json"),
            contract.to_string()
        ).unwrap();
        
        let parsed = crate::ParsedArgs {
            positional: vec!["create".to_string()],
            flags: {
                let mut m = std::collections::HashMap::new();
                m.insert("name".to_string(), "test-skill".to_string());
                m
            },
        };
        
        let result = run_create_skill(root, &parsed, true);
        assert!(!result.get("ok").and_then(Value::as_bool).unwrap_or(true));
        let errors = result.get("errors").and_then(Value::as_array).unwrap();
        assert!(errors.iter().any(|e| e.as_str() == Some("skill_scaffold_contract_version_must_be_v1")));
    }

    /// Test: skills_create_with_missing_name
    /// Validates error when skill name is missing
    #[test]
    fn skills_create_with_missing_name() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        fs::create_dir_all(root.join("planes/contracts/skills")).unwrap();
        
        // Create valid contract
        let contract = json!({
            "version": "v1",
            "kind": "skill_scaffold_contract",
            "required_files": []
        });
        fs::write(
            root.join("planes/contracts/skills/skill_scaffold_contract_v1.json"),
            contract.to_string()
        ).unwrap();
        
        let parsed = crate::ParsedArgs {
            positional: vec!["create".to_string()],
            flags: std::collections::HashMap::new(),
        };
        
        let result = run_create_skill(root, &parsed, true);
        assert!(!result.get("ok").and_then(Value::as_bool).unwrap_or(true));
        let errors = result.get("errors").and_then(Value::as_array).unwrap();
        assert!(errors.iter().any(|e| e.as_str() == Some("skill_name_required")));
    }

    /// Test: parse_skill_version_edge_cases
    /// Validates version parsing edge cases
    #[test]
    fn parse_skill_version_edge_cases() {
        // Empty string
        assert!(parse_skill_version("").is_none());
        
        // Just "v" prefix
        assert!(parse_skill_version("v").is_none());
        
        // Legacy format "v1"
        let v = parse_skill_version("v1").unwrap();
        assert_eq!(v.major, 1);
        assert!(v.legacy);
        
        // Standard semver "v1.2.3"
        let v = parse_skill_version("v1.2.3").unwrap();
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 2);
        assert_eq!(v.patch, 3);
        assert!(!v.legacy);
        
        // Without v prefix "2.0.0"
        let v = parse_skill_version("2.0.0").unwrap();
        assert_eq!(v.major, 2);
        assert!(!v.legacy);
        
        // Invalid formats
        assert!(parse_skill_version("abc").is_none());
        assert!(parse_skill_version("1.2.3.4").is_none());
    }

    // Helper structures and functions
    struct ParsedArgs {
        positional: Vec<String>,
        flags: std::collections::HashMap<String, String>,
    }

    struct SkillVersion {
        major: u64,
        minor: u64,
        patch: u64,
        legacy: bool,
    }

    fn skills_plane_status(_root: &Path) -> Value {
        todo!("Use actual ops crate function")
    }

    fn run_create_skill(_root: &Path, _parsed: &ParsedArgs, _strict: bool) -> Value {
        todo!("Use actual ops crate function")
    }

    fn parse_skill_version(_raw: &str) -> Option<SkillVersion> {
        todo!("Use actual ops crate function")
    }
}

// ============================================================================
// PRIORITY 7: PLUGIN REGISTRY AND HEALTH CHECKS
// Location: core/layer2/conduit/src/lib.rs
// Impact: +1.0% coverage
// ============================================================================

#[cfg(test)]
mod plugin_registry_tests {
    use super::*;

    /// Test: plugin_registry_load_empty
    /// Validates loading empty/non-existent registry
    #[test]
    fn plugin_registry_load_empty() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        
        let registry_path = root.join("empty_registry.json");
        let registry = load_plugin_registry(&registry_path);
        
        assert!(registry.plugins.is_empty());
    }

    /// Test: plugin_health_check_missing_file
    /// Validates health check when WASM file is missing
    #[test]
    fn plugin_health_check_missing_file() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        
        let plugin = PluginRegistryEntry {
            plugin_id: "test-plugin".to_string(),
            plugin_type: "substrate_adapter".to_string(),
            version: "1.0.0".to_string(),
            wasm_component_path: "nonexistent.wasm".to_string(),
            wasm_sha256: "abc123".to_string(),
            capabilities: vec!["read".to_string()],
            signature: None,
            provenance: None,
            enabled: true,
            status: "healthy".to_string(),
            failure_count: 0,
            max_recovery_attempts: 3,
            recovery_backoff_ms: 1000,
            next_retry_ts_ms: 0,
            last_healthcheck_ts_ms: 0,
            last_error: None,
            quarantined_reason: None,
            registered_ts_ms: 1234567890,
        };
        
        let result = plugin_health_check(root, &plugin);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "wasm_component_missing");
    }

    /// Test: plugin_health_check_sha_mismatch
    /// Validates health check when SHA256 doesn't match
    #[test]
    fn plugin_health_check_sha_mismatch() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        
        // Create WASM file with specific content
        let wasm_path = root.join("test.wasm");
        fs::write(&wasm_path, b"wasm content").unwrap();
        
        let plugin = PluginRegistryEntry {
            plugin_id: "test-plugin".to_string(),
            plugin_type: "substrate_adapter".to_string(),
            version: "1.0.0".to_string(),
            wasm_component_path: wasm_path.to_string_lossy().to_string(),
            wasm_sha256: "wrong_sha256_hash_1234567890abcdef".to_string(),
            capabilities: vec!["read".to_string()],
            signature: None,
            provenance: None,
            enabled: true,
            status: "healthy".to_string(),
            failure_count: 0,
            max_recovery_attempts: 3,
            recovery_backoff_ms: 1000,
            next_retry_ts_ms: 0,
            last_healthcheck_ts_ms: 0,
            last_error: None,
            quarantined_reason: None,
            registered_ts_ms: 1234567890,
        };
        
        let result = plugin_health_check(root, &plugin);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "wasm_component_sha_mismatch");
    }

    /// Test: mark_plugin_failure_quarantine
    /// Validates quarantine after max failures reached
    #[test]
    fn mark_plugin_failure_quarantine() {
        let mut plugin = PluginRegistryEntry {
            plugin_id: "test-plugin".to_string(),
            plugin_type: "substrate_adapter".to_string(),
            version: "1.0.0".to_string(),
            wasm_component_path: "/dev/null".to_string(),
            wasm_sha256: "abc123".to_string(),
            capabilities: vec!["read".to_string()],
            signature: None,
            provenance: None,
            enabled: true,
            status: "healthy".to_string(),
            failure_count: 2,  // One away from quarantine (max = 3)
            max_recovery_attempts: 3,
            recovery_backoff_ms: 1000,
            next_retry_ts_ms: 0,
            last_healthcheck_ts_ms: 0,
            last_error: None,
            quarantined_reason: None,
            registered_ts_ms: 1234567890,
        };
        
        let result = mark_plugin_failure(&mut plugin, "test failure", 1234567890);
        
        assert_eq!(plugin.status, "quarantined");
        assert!(!plugin.enabled);
        assert_eq!(plugin.quarantined_reason, Some("test failure".to_string()));
        
        let result_obj = result.as_object().unwrap();
        assert_eq!(result_obj.get("type").and_then(Value::as_str), Some("plugin_runtime_quarantined"));
    }

    /// Test: mark_plugin_healthy_recovery
    /// Validates recovery from failure state
    #[test]
    fn mark_plugin_healthy_recovery() {
        let mut plugin = PluginRegistryEntry {
            plugin_id: "test-plugin".to_string(),
            plugin_type: "substrate_adapter".to_string(),
            version: "1.0.0".to_string(),
            wasm_component_path: "/dev/null".to_string(),
            wasm_sha256: "abc123".to_string(),
            capabilities: vec!["read".to_string()],
            signature: None,
            provenance: None,
            enabled: true,
            status: "healing".to_string(),
            failure_count: 2,
            max_recovery_attempts: 3,
            recovery_backoff_ms: 1000,
            next_retry_ts_ms: 1000,
            last_healthcheck_ts_ms: 0,
            last_error: Some("previous error".to_string()),
            quarantined_reason: None,
            registered_ts_ms: 1234567890,
        };
        
        let result = mark_plugin_healthy(&mut plugin, 1234567890, "health_check");
        
        assert_eq!(plugin.status, "healthy");
        assert_eq!(plugin.failure_count, 0);
        assert_eq!(plugin.next_retry_ts_ms, 0);
        assert!(plugin.last_error.is_none());
        
        let result_obj = result.unwrap().as_object().unwrap();
        assert_eq!(result_obj.get("type").and_then(Value::as_str), Some("plugin_runtime_recovered"));
    }

    /// Test: normalize_plugin_type_invalid
    /// Validates plugin type normalization to default
    #[test]
    fn normalize_plugin_type_invalid() {
        let invalid_types = vec!["unknown", "malicious", "not_a_type", ""];
        
        for invalid in invalid_types {
            let normalized = normalize_plugin_type(Some(invalid));
            assert_eq!(normalized, "substrate_adapter");
        }
    }

    // Helper structures and functions
    struct PluginRegistryEntry {
        plugin_id: String,
        plugin_type: String,
        version: String,
        wasm_component_path: String,
        wasm_sha256: String,
        capabilities: Vec<String>,
        signature: Option<String>,
        provenance: Option<String>,
        enabled: bool,
        status: String,
        failure_count: u32,
        max_recovery_attempts: u32,
        recovery_backoff_ms: u64,
        next_retry_ts_ms: u64,
        last_healthcheck_ts_ms: u64,
        last_error: Option<String>,
        quarantined_reason: Option<String>,
        registered_ts_ms: u64,
    }

    struct PluginRegistryState {
        plugins: Vec<PluginRegistryEntry>,
    }

    impl Default for PluginRegistryState {
        fn default() -> Self {
            Self { plugins: Vec::new() }
        }
    }

    fn load_plugin_registry(_path: &Path) -> PluginRegistryState {
        todo!("Use actual conduit crate function")
    }

    fn plugin_health_check(_root: &Path, _plugin: &PluginRegistryEntry) -> Result<(), String> {
        todo!("Use actual conduit crate function")
    }

    fn mark_plugin_failure(_plugin: &mut PluginRegistryEntry, _reason: &str, _now_ms: u64) -> Value {
        todo!("Use actual conduit crate function")
    }

    fn mark_plugin_healthy(_plugin: &mut PluginRegistryEntry, _now_ms: u64, _source: &str) -> Option<Value> {
        todo!("Use actual conduit crate function")
    }

    fn normalize_plugin_type(_raw: Option<&str>) -> String {
        todo!("Use actual conduit crate function")
    }
}

// ============================================================================
// TEST SKELETON NOTES
// ============================================================================

// To integrate these tests into the actual codebase:
//
// 1. For Security Plane tests:
//    - Add to: core/layer1/security/src/lib.rs or core/layer1/security/tests/
//    - Import actual functions from the crate
//
// 2. For Skills Plane tests:
//    - Add to: core/layer0/ops/src/skills_plane.rs (#[cfg(test)] mod)
//    - Or core/layer0/ops/tests/skills_plane_tests.rs
//
// 3. For Conduit tests:
//    - Add to: core/layer2/conduit/tests/
//    - Import actual types and functions from conduit crate
//
// 4. Run tests:
//    cargo test --package <package-name> --test coverage_gap_high_priority
//
// Expected coverage gain: +8.5% (86.1% projected)
// Estimated implementation effort: 14.5 dev-days


