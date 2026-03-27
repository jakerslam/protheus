# Test Coverage Gap Analysis - Implementation Plan
**Target:** Increase combined coverage from 77.63% to 85%+
**Status:** Current 77.63% + 8.5% = 86.13% projected
**Last Updated:** 2026-03-25

---

## Summary

| Metric | Current | Target | Gap | Impact of This Plan |
|--------|---------|--------|-----|---------------------|
| TypeScript | 71.95% | 90% | -18.05% | TypeScript focus separate |
| Rust | 83.31% | 90% | -6.69% | +8.5% → 91.81% |
| **Combined** | **77.63%** | **90%** | **-12.37%** | **+8.5% → 86.13%** |

*Note: Remaining ~4% to reach 90% requires TypeScript test infrastructure investment*

---

## Prioritized Test Implementation List (50 Tests)

### Priority 1: Security Fail-Closed Paths (15 tests) → +1.2%
**Location:** `core/layer1/security/src/lib.rs`
**Effort:** 3 dev-days
**Critical for:** Production security guarantees

| # | Test Name | File/Module | Lines | Est. Effort |
|---|-----------|-------------|-------|-------------|
| 1.1 | `emergency_stop_rejects_short_approval_note` | lib.rs | ~888-900 | 2h |
| 1.2 | `emergency_stop_normalizes_invalid_scopes` | lib.rs | ~810-830 | 2h |
| 1.3 | `emergency_stop_release_short_approval_note` | lib.rs | ~918-931 | 2h |
| 1.4 | `emergency_stop_unknown_command` | lib.rs | ~935-950 | 1h |
| 1.5 | `capability_lease_missing_key_error` | lib.rs | ~980-995 | 2h |
| 1.6 | `capability_lease_empty_scope_error` | lib.rs | ~1070-1075 | 2h |
| 1.7 | `capability_lease_malformed_token` | lib.rs | ~950-975 | 3h |
| 1.8 | `capability_lease_double_consume_fails_closed` | lib.rs | ~1150-1200 | 3h |
| 1.9 | `capability_lease_verify_consumed_fails` | lib.rs | ~1138-1148 | 2h |
| 1.10 | `startup_attestation_fails_without_key` | lib.rs | ~1300-1310 | 2h |
| 1.11 | `startup_attestation_unknown_command` | lib.rs | ~1480-1500 | 1h |
| 1.12 | `lease_unpack_token_invalid_base64` | lib.rs | ~960-975 | 2h |
| 1.13 | `capability_lease_status_empty_state` | lib.rs | ~1200+ | 1.5h |
| 1.14 | `startup_attestation_verify_expired` | lib.rs | ~1415-1425 | 2h |
| 1.15 | `startup_attestation_signature_mismatch` | lib.rs | ~1450-1470 | 2h |

**Files to modify:**
- `core/layer1/security/src/lib.rs` (add to `#[cfg(test)]` module)
- OR create `core/layer1/security/tests/fail_closed_tests.rs`

---

### Priority 2: Skills Backward Compatibility (8 tests) → +0.8%
**Location:** `core/layer0/ops/src/skills_plane.rs`
**Effort:** 2 dev-days
**Critical for:** SRS-V8-SKILL-002 compliance

| # | Test Name | Function | Lines | Est. Effort |
|---|-----------|----------|-------|-------------|
| 2.1 | `evaluate_skill_run_missing_skill_in_registry` | evaluate_skill_run_backward_compat | ~175-180 | 2h |
| 2.2 | `evaluate_skill_run_invalid_version_format` | parse_skill_version | ~185-190 | 2h |
| 2.3 | `evaluate_skill_run_version_below_minimum` | evaluate_skill_run_backward_compat | ~200-220 | 2.5h |
| 2.4 | `evaluate_skill_run_legacy_version_parsing` | parse_skill_version | ~65-85 | 2h |
| 2.5 | `evaluate_skill_run_strict_semver_policy` | load_backward_compat_policy | ~205-212 | 2h |
| 2.6 | `default_migration_lane_path_edge_cases` | default_migration_lane_path | ~245-275 | 1.5h |
| 2.7 | `load_backward_compat_policy_default` | load_backward_compat_policy | ~155-165 | 1.5h |
| 2.8 | `parse_skill_version_edge_cases` | parse_skill_version | ~45-85 | 2h |

**Files to modify:**
- `core/layer0/ops/src/skills_plane.rs`

---

### Priority 3: Conduit Strict Mode Enforcement (10 tests) → +1.5%
**Location:** `core/layer2/conduit/src/lib.rs`
**Effort:** 4 dev-days
**Critical for:** Cross-boundary security

| # | Test Name | Function | Lines | Est. Effort |
|---|-----------|----------|-------|-------------|
| 3.1 | `conduit_empty_agent_id_validation` | validate_structure | ~1750-1760 | 2h |
| 3.2 | `conduit_whitespace_agent_id_validation` | validate_structure | ~1753-1755 | 1h |
| 3.3 | `conduit_receipt_query_limit_zero` | validate_structure | ~1760-1770 | 2h |
| 3.4 | `conduit_receipt_query_limit_over_1000` | validate_structure | ~1765-1770 | 1h |
| 3.5 | `conduit_policy_update_empty_patch_id` | validate_structure | ~1770-1785 | 2h |
| 3.6 | `conduit_policy_update_unsafe_prefix` | validate_structure | ~1775-1782 | 2h |
| 3.7 | `conduit_extension_wasm_sha256_too_short` | validate_structure | ~1785-1800 | 2h |
| 3.8 | `conduit_extension_wasm_sha256_invalid_hex` | validate_structure | ~1788-1795 | 2h |
| 3.9 | `conduit_extension_empty_capabilities` | validate_structure | ~1790-1795 | 2h |
| 3.10 | `conduit_extension_whitespace_capabilities` | validate_structure | ~1793-1797 | 1.5h |
| 3.11 | `conduit_extension_invalid_plugin_type` | validate_structure | ~1795-1800 | 2h |
| 3.12 | `conduit_stop_agent_empty_validation` | validate_structure | ~1751-1755 | 1h |

**Files to modify:**
- `core/layer2/conduit/tests/strict_mode_tests.rs` (new file)

---

### Priority 4: Receipt Validation Logic (6 tests) → +1.0%
**Location:** `core/layer2/conduit/src/lib.rs`
**Effort:** 2.5 dev-days

| # | Test Name | Function | Lines | Est. Effort |
|---|-----------|----------|-------|-------------|
| 4.1 | `validation_receipt_deterministic_hashing` | fail_closed_receipt, success_receipt | ~1820-1850 | 2h |
| 4.2 | `validation_receipt_hash_uniqueness` | fail_closed_receipt | ~1850-1870 | 1.5h |
| 4.3 | `crossing_receipt_ts_to_rust_direction` | CrossingReceipt construction | ~1870-1890 | 2h |
| 4.4 | `crossing_receipt_rust_to_ts_direction` | CrossingReceipt construction | ~1890-1910 | 1.5h |
| 4.5 | `validation_receipt_different_policy_hashes` | fail_closed_receipt | ~1855-1865 | 1h |
| 4.6 | `validation_receipt_different_security_hashes` | fail_closed_receipt | ~1860-1870 | 1h |

**Files to modify:**
- Add to `core/layer2/conduit/tests/certification.rs` or new file

---

### Priority 5: Error Handling Branches (11 tests) → +2.0%
**Location:** Various
**Effort:** 4 dev-days

| # | Test Name | Module | Lines | Est. Effort |
|---|-----------|--------|-------|-------------|
| 5.1 | `startup_attestation_critical_hash_drift` | lib.rs | ~1455-1480 | 3h |
| 5.2 | `startup_attestation_missing_paths_handled` | lib.rs | ~1380-1390 | 2h |
| 5.3 | `startup_attestation_status_command` | lib.rs | ~1470-1485 | 1.5h |
| 5.4 | `startup_policy_ttl_clamping` | lib.rs | ~1280-1285 | 1h |
| 5.5 | `integrity_reseal_atomic_write_failure` | lib.rs | ~900-925 | 3h |
| 5.6 | `emergency_stop_state_load_malformed` | lib.rs | ~800-825 | 2h |
| 5.7 | `capability_lease_audit_append_failure` | lib.rs | ~1180-1195 | 2h |
| 5.8 | `capability_lease_state_corrupted` | lib.rs | ~930-945 | 2h |
| 5.9 | `lease_verify_wrong_key` | lib.rs | ~1120-1135 | 2h |
| 5.10 | `lease_verify_expired` | lib.rs | ~1105-1120 | 2h |
| 5.11 | `startup_resolve_secret_file_permission_denied` | lib.rs | ~1250-1270 | 2h |

**Files to modify:**
- `core/layer1/security/src/lib.rs`
- `core/layer1/security/tests/error_handling_tests.rs` (new)

---

### Priority 6: Skills State Management (6 tests) → +1.0%
**Location:** `core/layer0/ops/src/skills_plane.rs`
**Effort:** 3 dev-days

| # | Test Name | Function | Lines | Est. Effort |
|---|-----------|----------|-------|-------------|
| 6.1 | `skills_plane_status_empty_state` | status | ~300-320 | 1.5h |
| 6.2 | `skills_create_with_invalid_contract_version` | run_create | ~330-345 | 2h |
| 6.3 | `skills_create_with_invalid_contract_kind` | run_create | ~335-345 | 2h |
| 6.4 | `skills_create_with_missing_name` | run_create | ~360-370 | 1.5h |
| 6.5 | `rollback_checkpoint_path_generation` | rollback_checkpoint_path | ~270-280 | 1h |
| 6.6 | `quarantine_path_operations` | quarantine_path | ~285-295 | 1.5h |

**Files to modify:**
- `core/layer0/ops/src/skills_plane.rs` or `core/layer0/ops/tests/skills_tests.rs`

---

### Priority 7: Plugin Registry and Health Checks (6 tests) → +1.0%
**Location:** `core/layer2/conduit/src/lib.rs`
**Effort:** 3 dev-days

| # | Test Name | Function | Lines | Est. Effort |
|---|-----------|----------|-------|-------------|
| 7.1 | `plugin_registry_load_empty` | load_plugin_registry | ~1200-1215 | 1h |
| 7.2 | `plugin_registry_load_corrupted_json` | load_plugin_registry | ~1210-1220 | 1.5h |
| 7.3 | `plugin_health_check_missing_file` | plugin_health_check | ~1250-1260 | 2h |
| 7.4 | `plugin_health_check_not_a_file` | plugin_health_check | ~1255-1265 | 1.5h |
| 7.5 | `plugin_health_check_sha_mismatch` | plugin_health_check, hash_file_sha256 | ~1260-1275 | 2.5h |
| 7.6 | `mark_plugin_failure_quarantine` | mark_plugin_failure | ~1290-1310 | 2h |
| 7.7 | `mark_plugin_healthy_recovery` | mark_plugin_healthy | ~1315-1335 | 2h |
| 7.8 | `mark_plugin_healthy_no_change` | mark_plugin_healthy | ~1325-1330 | 1.5h |
| 7.9 | `normalize_plugin_type_invalid` | normalize_plugin_type | ~1240-1250 | 1h |
| 7.10 | `normalize_plugin_entry_clamping` | normalize_plugin_entry | ~1225-1240 | 2h |

**Files to modify:**
- `core/layer2/conduit/tests/plugin_tests.rs` (new)

---

## Implementation Timeline

### Week 1: Critical Security Paths (P1)
- **Day 1-2:** Emergency stop tests (3 tests)
- **Day 3-4:** Capability lease tests (5 tests)
- **Day 5:** Startup attestation tests (4 tests)
- **Day 6-7:** Integration and bug fixes

### Week 2: Skills and Conduit (P2-P3)
- **Day 1-2:** Skills backward compatibility (4 tests)
- **Day 3-5:** Conduit strict mode enforcement (8 tests)
- **Day 6-7:** Receipt validation (3 tests)

### Week 3: Errors and Plugin Registry (P4-P7 part)
- **Day 1-2:** Error handling branches (6 tests)
- **Day 3-4:** Plugin registry/health checks (5 tests)
- **Day 5:** Skills state management (3 tests)
- **Day 6-7:** Documentation and coverage verification

### Week 4: Remaining Tests and Polish
- **Day 1-3:** Complete remaining tests (12 tests)
- **Day 4-5:** Integration testing
- **Day 6:** Coverage verification
- **Day 7:** Documentation and handoff

---

## How to Run Tests

### Run all coverage gap tests:
```bash
cargo test --test coverage_gap_high_priority
```

### Run specific priority module:
```bash
# Security plane
cargo test security_fail_closed_tests

# Skills plane  
cargo test skills_backward_compat_tests

# Conduit
cargo test conduit_strict_mode_tests

# Receipts
cargo test receipt_validation_tests

# Error handling
cargo test error_handling_tests

# Additional
cargo test additional_coverage_tests
cargo test plugin_registry_tests
```

### Run with coverage:
```bash
# Using cargo-tarpaulin
cargo tarpaulin --workspace --output-dir coverage

# Generate HTML report
cargo tarpaulin --workspace --out Html
```

---

## Verification Checklist

Before marking complete:

- [ ] All 58 tests pass
- [ ] Coverage report shows +8.5% gain
- [ ] No flaky tests (run 5x in succession)
- [ ] Documentation updated
- [ ] CI pipeline passes
- [ ] Security review completed

---

## Coverage Gap Locations Detail

### Security Plane (core/layer1/security/src/lib.rs)
| Lines | Function | Coverage | Tests Added |
|-------|----------|----------|-------------|
| 810-840 | emergency_stop_normalize_scopes | Partial | 2 |
| 850-950 | run_emergency_stop | Partial | 4 |
| 930-980 | lease_unpack_token | None | 3 |
| 980-1070 | run_capability_lease issue | Partial | 4 |
| 1070-1200 | run_capability_lease verify/consume | Partial | 3 |
| 1270-1320 | load_startup_policy | Partial | 2 |
| 1320-1500 | run_startup_attestation | Partial | 4 |

### Conduit (core/layer2/conduit/src/lib.rs)
| Lines | Function | Coverage | Tests Added |
|-------|----------|----------|-------------|
| 1750-1800 | validate_structure | Partial | 12 |
| 1800-1920 | fail_closed_receipt, success_receipt | Partial | 6 |
| 1920-2050 | process_command error branches | Partial | 8 |
| 1200-1350 | Plugin registry functions | Partial | 10 |

### Skills Plane (core/layer0/ops/src/skills_plane.rs)
| Lines | Function | Coverage | Tests Added |
|-------|----------|----------|-------------|
| 45-90 | parse_skill_version | Partial | 4 |
| 155-180 | load_backward_compat_policy | Full | 1 |
| 180-230 | evaluate_skill_run_backward_compat | Partial | 4 |
| 220-280 | default_migration_lane_path | Partial | 2 |
| 280-330 | status, conduit_enforcement | Partial | 3 |
| 330-480 | run_create | Partial | 3 |

---

## Appendix: Quick Reference

### Test Template:
```rust
/// Test: [descriptive_name]
/// Validates [what behavior is tested]
/// Coverage: Lines [range] (current coverage status)
#[test]
fn test_name() {
    // Arrange
    let temp = TempDir::new().unwrap();
    let root = temp.path();
    
    // Act
    let result = function_to_test(root, args);
    
    // Assert
    assert!(result.condition());
    assert_eq!(result.get("key"), expected);
}
```

### Environment Setup:
```bash
# Ensure test dependencies
cargo add --dev tempfile

# For coverage
cargo install cargo-tarpaulin

# Verify installation
cargo tarpaulin --version
```

---

*Generated: 2026-03-25*
*Total Tests: 58*
*Total Estimated Effort: 14.5 dev-days*
*Projected Final Coverage: 86.13%*
