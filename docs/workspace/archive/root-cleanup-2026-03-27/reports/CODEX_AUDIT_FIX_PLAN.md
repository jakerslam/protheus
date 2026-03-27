# CODEX AUDIT FIX PLAN
## Comprehensive Remediation Guide for A+ Readiness

**Generated:** March 19, 2026  
**Target Grade:** A+ (95%+)  
**Current Grade:** A- (91%)  
**Gap to Close:** 4-5 percentage points

---

## EXECUTIVE SUMMARY

This document provides a prioritized, actionable fix plan to achieve A+ readiness. All items are organized by priority (P0-P3), effort estimate, and blocking dependencies.

**Current Blockers to A+:**
1. HMAN actions (13) - Requires human authority
2. Security command stubs (~47) - Requires implementation
3. Test coverage (77% → 90%) - Requires ~300 functions
4. V8-SKILL-002 backward compatibility - Requires gates

---

## PRIORITY MATRIX

| Priority | Items | Effort | Impact on Grade |
|----------|-------|--------|-----------------|
| P0 | 3 | 4-6 weeks | +3-5% |
| P1 | 4 | 2-4 weeks | +2-3% |
| P2 | 5 | 1-2 weeks | +1-2% |
| P3 | 6 | Ongoing | +0-1% |

---

## P0 CRITICAL (Blocks A+)

### P0-1: Implement Security Command Stubs
**Contract:** V6-SEC-010 through V6-SEC-016  
**Location:** `core/layer0/ops/src/security_plane.rs`  
**Effort:** 3-4 weeks  
**Impact:** +3-4% grade

#### Commands to Implement

| Command | Contract | Status | Implementation Notes |
|---------|----------|--------|---------------------|
| `scan` | V6-SEC-010 | 🔄 Stub | Injection/MCP poisoning scanner |
| `auto-remediate` | V6-SEC-011 | 🔄 Stub | Automated remediation loop |
| `blast-radius-sentinel` | V6-SEC-012 | 🔄 Partial | Blast radius containment |
| `verify-proofs` | V6-SEC-013 | 🔄 Stub | Formal proof verification runner |
| `audit-logs` | V6-SEC-014 | 🔄 Stub | Security audit log analysis |
| `threat-model` | V6-SEC-015 | 🔄 Stub | Threat modeling automation |
| `secrets-federation` | V6-SEC-016 | 🔄 Stub | Cross-system secrets sync |

#### Implementation Template

```rust
// Example: V6-SEC-010 scan command
#[derive(Debug, Serialize, Deserialize)]
pub struct ScanCommand {
    pub target: ScanTarget,
    pub scan_type: ScanType,
    pub depth: ScanDepth,
}

impl SecurityCommand for ScanCommand {
    fn execute(&self, ctx: &SecurityContext) -> Result<SecurityReceipt, SecurityError> {
        // 1. Validate scan scope against ABAC policy
        // 2. Run injection detection patterns
        // 3. Check MCP poisoning vectors
        // 4. Generate deterministic receipt
        // 5. Log to security events
        
        let findings = self.run_injection_scan(ctx)?;
        let mcp_findings = self.run_mcp_poisoning_scan(ctx)?;
        
        Ok(SecurityReceipt::new()
            .with_findings(findings)
            .with_mcp_findings(mcp_findings)
            .with_hash())
    }
}
```

#### Acceptance Criteria
- [ ] All 7 commands fully implemented
- [ ] Unit tests for each command (90%+ coverage)
- [ ] Integration tests with security plane
- [ ] Documentation in `docs/security/commands.md`
- [ ] Receipt generation verified

---

### P0-2: Complete HMAN Actions
**Contracts:** HMAN-001, 026, 027, 028, 032, 040, 043, 086, 087  
**Location:** `hman-prep/` (prepared materials)  
**Effort:** 30-90 days (human timeline)  
**Impact:** +4-7% grade

#### Action Checklist

| HMAN | Action | Prepared In | Owner | Status |
|------|--------|-------------|-------|--------|
| HMAN-001 | Security audit authority | `auditor-outreach/` | Legal | ⏳ Ready to send |
| HMAN-026 | SOC2 Type II | `soc2-readiness-bundle/` | Compliance | ⏳ Ready to engage |
| HMAN-027 | ISO 27001 | `iso27001-readiness-bundle/` | Compliance | ⏳ Ready to engage |
| HMAN-028 | Commercial contracts | `legal-contracts/msa-template.md` | Legal | ⏳ Ready to execute |
| HMAN-032 | Legal packet | `legal-contracts/dpa-template.md` | Legal | ⏳ Ready to deliver |
| HMAN-040 | Legal documentation | `legal-contracts/` | Legal | ⏳ Ready to complete |
| HMAN-043 | Security audit authority | `auditor-outreach/` | Security | ⏳ Ready to send |
| HMAN-086 | High-assurance profile | `high-assurance-profile/` | CEO/CTO | ⏳ Ready for review |
| HMAN-087 | Third-party verification | `auditor-outreach/` | Security | ⏳ Ready to send |

#### Next Steps
1. Review all materials in `hman-prep/`
2. Fill in company-specific details
3. Legal review of contracts
4. Send auditor outreach emails
5. Schedule scoping calls
6. Execute contracts

---

### P0-3: Fix V8-SKILL-002 Backward Compatibility
**Contract:** V8-SKILL-002  
**Location:** `core/layer0/ops/src/skill_runtime.rs`  
**Effort:** 1-2 weeks  
**Impact:** +1-2% grade

#### Required Implementation

```rust
// Backward compatibility gates
pub struct SkillCompatibilityGate {
    pub semver: Semver,
    pub migration_lane: MigrationLane,
    pub deprecation_policy: DeprecationPolicy,
}

impl SkillCompatibilityGate {
    pub fn validate_backward_compat(&self, skill: &Skill) -> Result<(), CompatibilityError> {
        // 1. Check semver compatibility
        // 2. Validate migration lane exists
        // 3. Verify deprecation timeline
        // 4. Generate compatibility receipt
        
        if !self.semver.is_compatible(&skill.version) {
            return Err(CompatibilityError::VersionMismatch);
        }
        
        if !self.migration_lane.has_path(&skill.version) {
            return Err(CompatibilityError::NoMigrationPath);
        }
        
        Ok(())
    }
}
```

#### Acceptance Criteria
- [ ] Semver validation implemented
- [ ] Migration lanes defined for all skills
- [ ] Deprecation policy enforced
- [ ] Compatibility receipts generated
- [ ] Tests for breaking changes

---

## P1 HIGH PRIORITY

### P1-1: Increase Test Coverage to 90%
**Current:** 77%  
**Target:** 90%  
**Gap:** ~300 functions  
**Effort:** 2-3 weeks  
**Impact:** +2-3% grade

#### Coverage by Module

| Module | Current | Target | Gap |
|--------|---------|--------|-----|
| `core/layer0/ops` | 82% | 90% | 8% |
| `core/layer1/cognition` | 75% | 90% | 15% |
| `core/layer2/substrate` | 70% | 90% | 20% |
| `client/runtime` | 80% | 90% | 10% |

#### Implementation Strategy

```bash
# Generate coverage report
cargo tarpaulin --out Html --output-dir coverage/

# Identify uncovered functions
grep -r "uncovered" coverage/tarpaulin-report.html | head -50

# Focus on:
# 1. Error handling paths
# 2. Edge cases in security commands
# 3. Conduit strict mode enforcement
# 4. Receipt validation logic
```

#### Priority Functions to Cover

1. **Security Plane:**
   - `security_plane::execute_command` (all branches)
   - `security_plane::validate_receipt`
   - `security_plane::audit_log_write`

2. **Conduit:**
   - `conduit::strict_mode_enforce`
   - `conduit::validate_data_isolation`
   - `conduit::cross_plane_check`

3. **Receipt System:**
   - `receipt::generate_hash`
   - `receipt::verify_integrity`
   - `receipt::chain_validation`

---

### P1-2: Create Root-Level GOVERNANCE.md
**Location:** `GOVERNANCE.md` (root)  
**Effort:** 1 day  
**Impact:** +0.5% grade

#### Required Content

```markdown
# Governance

## Decision Making
- RFC process for major changes
- Security review for all changes
- Formal verification for critical paths

## Code Review
- All changes require PR review
- Security team approval for security changes
- CI gates must pass

## Release Process
- Semantic versioning
- Changelog maintenance
- Security advisory process

## Security Response
- Incident response team
- Disclosure policy
- Patch timeline
```

---

### P1-3: Implement OpenAI Swarm Bridge
**Location:** `core/layer0/ops/src/bridge/`  
**Effort:** 1 week  
**Impact:** +0.5% grade

#### Implementation

```rust
// OpenAI Swarm bridge implementation
pub struct OpenAISwarmBridge {
    client: SwarmClient,
    config: BridgeConfig,
}

impl Bridge for OpenAISwarmBridge {
    fn execute(&self, request: BridgeRequest) -> Result<BridgeResponse, BridgeError> {
        // 1. Validate request against policy
        // 2. Transform to Swarm format
        // 3. Execute via Swarm API
        // 4. Transform response
        // 5. Generate receipt
        
        let swarm_request = self.transform_request(request)?;
        let swarm_response = self.client.execute(swarm_request).await?;
        let response = self.transform_response(swarm_response)?;
        
        Ok(response.with_receipt())
    }
}
```

---

### P1-4: Complete V6-SEC Security Features
**Contracts:** V6-SEC-001 through V6-SEC-009  
**Effort:** 1-2 weeks  
**Impact:** +1% grade

#### Verify Completion

| Contract | Feature | Status | Verification |
|----------|---------|--------|--------------|
| V6-SEC-001 | Authentication | ✅ | Check `startup_attestation_boot_gate` |
| V6-SEC-002 | Authorization | ✅ | Check ABAC policy enforcement |
| V6-SEC-003 | Audit logging | ✅ | Check `local/state/ops/security_plane/events.jsonl` |
| V6-SEC-004 | Encryption | ✅ | Check TLS/encryption in transit |
| V6-SEC-005 | Input validation | ✅ | Check command validation |
| V6-SEC-006 | Error handling | ✅ | Check secure error responses |
| V6-SEC-007 | Session management | ✅ | Check session lifecycle |
| V6-SEC-008 | Secure configuration | ✅ | Check config validation |
| V6-SEC-009 | Dependency management | ✅ | Check SBOM and Sigstore |

---

## P2 MEDIUM PRIORITY

### P2-1: Formal Proof Expansion
**Current:** 60% coverage  
**Target:** 90% coverage  
**Effort:** 2-3 weeks  
**Impact:** +1-2% grade

#### Priority Proofs

1. **Safety Invariants:**
   - Memory safety for all unsafe blocks
   - Conduit data isolation
   - Receipt integrity

2. **Security Properties:**
   - ABAC policy enforcement
   - Command authorization
   - Audit log immutability

3. **Liveness Properties:**
   - Eventual consistency
   - Progress guarantees
   - Termination proofs

---

### P2-2: Documentation Completion
**Effort:** 1 week  
**Impact:** +0.5% grade

#### Missing Documentation

| Document | Location | Priority |
|----------|----------|----------|
| API Reference | `docs/api/` | High |
| Security Runbook | `docs/security/runbook.md` | High |
| Deployment Guide | `docs/deployment/` | Medium |
| Troubleshooting | `docs/troubleshooting/` | Medium |
| Architecture Decision Records | `docs/adr/` | Low |

---

### P2-3: Performance Optimization
**Effort:** 1 week  
**Impact:** +0.5% grade

#### Targets

| Metric | Current | Target | Action |
|--------|---------|--------|--------|
| Cold start | 74.5ms | <50ms | Optimize initialization |
| Idle RSS | 22.1MB | <20MB | Reduce memory footprint |
| Throughput | 174,995/s | 200,000/s | Optimize hot paths |

---

### P2-4: Client Runtime Migration
**Current:** 65% Rust  
**Target:** 90% Rust  
**Effort:** 2-3 weeks  
**Impact:** +1% grade

#### Migration Priority

1. High-value TypeScript modules
2. Bridge pattern completion
3. Remove legacy wrappers
4. Update documentation

---

### P2-5: Monitoring & Alerting
**Effort:** 1 week  
**Impact:** +0.5% grade

#### Implementation

```yaml
# alerts.yml
alerts:
  - name: security_event_rate
    threshold: 100/minute
    action: notify_security_team
    
  - name: receipt_validation_failure
    threshold: 1
    action: emergency_stop
    
  - name: dopamine_threshold_breach
    threshold: warn
    action: notify_ops_team
```

---

## P3 LOW PRIORITY

### P3-1: Code Quality Improvements
- Linting rule enforcement
- Clippy warnings cleanup
- Dead code removal

### P3-2: Dependency Updates
- Security patch application
- Major version upgrades
- License compliance check

### P3-3: Developer Experience
- Better error messages
- Debug tooling
- Local development setup

### P3-4: Community
- Contributing guidelines
- Issue templates
- Code of conduct

### P3-5: Benchmarking
- Continuous benchmarking
- Performance regression detection
- Load testing

### P3-6: Cleanup
- Remove deprecated code
- Archive old documentation
- Consolidate duplicate logic

---

## IMPLEMENTATION TIMELINE

### Week 1-2: Security Commands
- Implement V6-SEC-010 through V6-SEC-016
- Unit tests for each command
- Integration tests

### Week 3-4: HMAN Execution
- Send auditor outreach emails
- Legal review of contracts
- Schedule scoping calls

### Week 5-6: Test Coverage
- Identify uncovered functions
- Write tests for priority areas
- Verify 90% coverage

### Week 7-8: V8-SKILL-002
- Implement backward compatibility gates
- Migration lanes
- Deprecation policy

### Week 9-12: Polish
- Documentation
- Performance optimization
- Final verification

---

## VERIFICATION CHECKLIST

### Pre-A+ Verification

- [ ] All P0 items complete
- [ ] All P1 items complete
- [ ] Test coverage ≥90%
- [ ] Security commands 100% implemented
- [ ] HMAN actions complete
- [ ] Documentation complete
- [ ] Performance targets met
- [ ] Formal proofs expanded

### Final Grade Calculation

| Section | Current | Target | Weight |
|---------|---------|--------|--------|
| V6-MEMORY | 37% | 50% | 10% |
| V6-LLMN | 100% | 100% | 10% |
| V6-RESEARCH | 100% | 100% | 10% |
| V6-F100 | 100% | 100% | 10% |
| V7-META | 83% | 90% | 10% |
| V7-TOP1 | 80% | 90% | 10% |
| V7-ASM | 100% | 100% | 10% |
| V7-F100 | 100% | 100% | 10% |
| V7-CANYON | 100% | 100% | 5% |
| V8-SKILL | 25% | 75% | 5% |
| Security | 43% | 90% | 5% |
| Governance | 80% | 100% | 5% |
| **Overall** | **91%** | **95%+** | **100%** |

---

## RISK MITIGATION

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Security command complexity | Medium | High | Phased implementation |
| HMAN timeline delays | High | Medium | Parallel execution |
| Test coverage gaps | Medium | Medium | Focused sprints |
| Resource constraints | Medium | Medium | Prioritize P0 |

---

## SUCCESS CRITERIA

**A+ Grade Achieved When:**
1. Overall SRS score ≥95%
2. All P0 items complete
3. No critical security gaps
4. All certifications obtained
5. Third-party verification complete

**Target Date:** June 2026

---

**Document Owner:** Protheus AI System  
**Review Cycle:** Weekly  
**Last Updated:** March 19, 2026
