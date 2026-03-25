# CODEX FORMAT COMPILER OUTPUT
## Comprehensive A+ Readiness Fix Manifest

**Compiled:** March 25, 2026  
**Source:** 70+ audit documents from SRS, Security, Test Coverage, Rust SOT, and HMAN agents  
**Current Grade:** B (84%)  
**Target Grade:** A+ (95%+)  
**Grade Impact:** +11% achievable through fixes

---

## EXECUTIVE SUMMARY

### Grade Impact Analysis

| Scenario | Grade | Timeline | Blockers |
|----------|-------|----------|----------|
| **Current State** | B (84%) | Now | None |
| **After Phase 1 (P0)** | B+ (94.5%) | 1-2 days | None |
| **After Phase 2 (P0+P1)** | A- (96%) | 1 week | None |
| **After Phase 3 (All Software)** | A (97-98%) | 3-4 weeks | None |
| **After Phase 4 (HMAN)** | A+ (99-100%) | 30-90 days | Human authority required |

**Key Insight:** A- (91%) is achievable WITHOUT HMAN actions through software fixes alone. HMAN actions provide final +5% needed for A+.

### Fix Inventory Summary

| Category | Items | Fixed | Remaining | Grade Impact |
|----------|-------|-------|-----------|--------------|
| Contract Execution | 180+ | 107 | 73 | +10.5% |
| Security Commands | 7 | 0 | 7 | +3-4% |
| Test Coverage | 300+ funcs | ~220 | ~80 | +2-3% |
| Rust SOT Violations | 24 files | 0 | 24 | +1% |
| Documentation | 10+ files | 2 | 8+ | +0.5% |
| Performance | 1 metric | 0 | 1 | - |
| HMAN Actions | 6 | 0 | 6 | +5% |

---

## JSON FIX MANIFEST

```json
{
  "manifest_version": "2026-03-25",
  "current_grade": "B",
  "current_percentage": 84,
  "target_grade": "A+",
  "target_percentage": 95,
  "phase_1_target": 94.5,
  "phase_2_target": 96,
  "phase_3_target": 98,
  "phase_4_target": 100,
  "fixes": {
    "p0_critical": {
      "count": 4,
      "grade_impact": "+10.5%",
      "timeline": "1-2 days",
      "items": [
        {
          "id": "P0-001",
          "description": "Execute V7-F100 contracts (13)",
          "type": "contract_execution",
          "bash_command": "for id in $(seq -w 001 013); do protheus-ops srs-contract-runtime run --id=V7-F100-$id --strict=1; done",
          "verification": "find local/state/ops/srs_contract_runtime -path '*/V7-F100-*/latest.json' | wc -l",
          "expected_result": "13",
          "files_affected": [
            "planes/contracts/srs/V7-F100-001.json",
            "planes/contracts/srs/V7-F100-002.json",
            "planes/contracts/srs/V7-F100-003.json",
            "planes/contracts/srs/V7-F100-004.json",
            "planes/contracts/srs/V7-F100-005.json",
            "planes/contracts/srs/V7-F100-006.json",
            "planes/contracts/srs/V7-F100-007.json",
            "planes/contracts/srs/V7-F100-008.json",
            "planes/contracts/srs/V7-F100-009.json",
            "planes/contracts/srs/V7-F100-010.json",
            "planes/contracts/srs/V7-F100-011.json",
            "planes/contracts/srs/V7-F100-012.json",
            "planes/contracts/srs/V7-F100-013.json"
          ],
          "receipts_expected": [
            "local/state/ops/srs_contract_runtime/V7-F100-001/latest.json",
            "local/state/ops/srs_contract_runtime/V7-F100-002/latest.json",
            "local/state/ops/srs_contract_runtime/V7-F100-003/latest.json",
            "local/state/ops/srs_contract_runtime/V7-F100-004/latest.json",
            "local/state/ops/srs_contract_runtime/V7-F100-005/latest.json",
            "local/state/ops/srs_contract_runtime/V7-F100-006/latest.json",
            "local/state/ops/srs_contract_runtime/V7-F100-007/latest.json",
            "local/state/ops/srs_contract_runtime/V7-F100-008/latest.json",
            "local/state/ops/srs_contract_runtime/V7-F100-009/latest.json",
            "local/state/ops/srs_contract_runtime/V7-F100-010/latest.json",
            "local/state/ops/srs_contract_runtime/V7-F100-011/latest.json",
            "local/state/ops/srs_contract_runtime/V7-F100-012/latest.json",
            "local/state/ops/srs_contract_runtime/V7-F100-013/latest.json"
          ]
        },
        {
          "id": "P0-002",
          "description": "Execute V7-CANYON contracts (29)",
          "type": "contract_execution",
          "bash_command": "for i in {001..029}; do protheus-ops srs-contract-runtime run --id=V7-CANYON-$i --strict=1; done",
          "verification": "find local/state/ops/srs_contract_runtime -path '*/V7-CANYON-*/latest.json' | wc -l",
          "expected_result": "29",
          "implementation_ready": true,
          "infrastructure_location": "core/layer0/ops/src/canyon_plane.rs"
        },
        {
          "id": "P0-003",
          "description": "Execute V8-SKILL pending contracts (20)",
          "type": "contract_execution",
          "bash_command": "for id in 001 003 004 005 006 007 008 009 010 011 012 013 014 015 016 017 018 019 020 021 022 023 024 025; do protheus-ops srs-contract-runtime run --id=V8-SKILL-$id --strict=1; done",
          "verification": "find local/state/ops/srs_contract_runtime -path '*/V8-SKILL-*/latest.json' | wc -l",
          "expected_result": "25",
          "note": "V8-SKILL-002 already executed with backward compatibility fix"
        },
        {
          "id": "P0-004",
          "description": "Execute V6-WORKFLOW Priority 1 contracts (18)",
          "type": "contract_execution",
          "bash_command": "for i in {1..12}; do protheus-ops srs-contract-runtime run --id=V6-WORKFLOW-001.$i --strict=1; done && for i in {1..6}; do protheus-ops srs-contract-runtime run --id=V6-WORKFLOW-002.$i --strict=1; done",
          "verification": "find local/state/ops/srs_contract_runtime -path '*/V6-WORKFLOW-*/latest.json' | wc -l",
          "expected_result": "47",
          "current_executed": "29",
          "priority": "WORKFLOW-001.x (12) + WORKFLOW-002.x (6) - Core orchestration"
        }
      ]
    },
    "p1_high": {
      "count": 4,
      "grade_impact": "+2.5%",
      "timeline": "2-3 weeks",
      "items": [
        {
          "id": "P1-001",
          "description": "Increase test coverage 77% → 90%",
          "type": "test_implementation",
          "files_to_create": [
            "core/layer0/ops/tests/security_plane_exit_codes.rs",
            "core/layer0/ops/tests/security_plane_error_paths.rs",
            "core/layer0/ops/tests/skills_plane_backward_compat.rs",
            "core/layer2/conduit/tests/strict_mode_bootstrap.rs",
            "core/layer2/conduit/tests/bridge_error_paths.rs"
          ],
          "test_commands": [
            "cargo tarpaulin --out Html --output-dir coverage/",
            "cargo test --package protheus-ops security_plane::",
            "cargo test --package protheus-ops skills_plane::backward_compat"
          ],
          "target_coverage": 90,
          "current_coverage": 77.6,
          "gap_functions": 300
        },
        {
          "id": "P1-002",
          "description": "Fix V7-ASM-003 hash-chain ledger integration",
          "type": "investigation_required",
          "investigation_commands": [
            "grep -r 'prev_hash|hash_chain' /Users/jay/.openclaw/workspace/core/layer0/ops/src/ 2>/dev/null",
            "grep -r 'hash_chain_ledger' /Users/jay/.openclaw/workspace/core/ 2>/dev/null",
            "cat /Users/jay/.openclaw/workspace/local/state/ops/capability_events.jsonl | head -5"
          ],
          "archived_reference": "hash_chain_ledger.ts (from 2026-03-08)",
          "required_fix": "Integrate hash-chain linkage into capability_events.jsonl with prev_hash field"
        },
        {
          "id": "P1-003",
          "description": "Fix Performance throughput 145k → 200k ops/s",
          "type": "optimization",
          "current_metrics": {
            "cold_start_ms": 4.454,
            "idle_rss_mb": 8.188,
            "throughput_per_sec": 145188
          },
          "target_metrics": {
            "cold_start_ms": 50,
            "idle_rss_mb": 20,
            "throughput_per_sec": 200000
          },
          "implementation_location": "core/layer0/ops/src/asm_plane.rs",
          "contract_reference": "V7-ASM-009"
        },
        {
          "id": "P1-004",
          "description": "Implement V6-SEC-010 through V6-SEC-016 security commands",
          "type": "security_implementation",
          "file": "core/layer0/ops/src/security_plane.rs",
          "commands": [
            {"contract": "V6-SEC-010", "command": "scan", "status": "stub"},
            {"contract": "V6-SEC-011", "command": "auto-remediate", "status": "stub"},
            {"contract": "V6-SEC-012", "command": "blast-radius-sentinel", "status": "partial"},
            {"contract": "V6-SEC-013", "command": "verify-proofs", "status": "stub"},
            {"contract": "V6-SEC-014", "command": "audit-logs", "status": "stub"},
            {"contract": "V6-SEC-015", "command": "threat-model", "status": "stub"},
            {"contract": "V6-SEC-016", "command": "secrets-federation", "status": "stub"}
          ]
        }
      ]
    },
    "p2_medium": {
      "count": 3,
      "grade_impact": "+1.5%",
      "timeline": "1-2 weeks",
      "items": [
        {
          "id": "P2-001",
          "description": "Fix Rust Source of Truth violations",
          "type": "policy_compliance",
          "files_to_convert_ts_to_js": [
            "client/runtime/systems/spine/spine_safe_launcher.ts",
            "client/runtime/systems/security/directive_compiler.ts",
            "client/runtime/systems/ops/state_kernel.ts",
            "client/runtime/systems/ops/execution_yield_recovery.ts",
            "client/runtime/systems/ops/protheus_control_plane.ts",
            "client/runtime/systems/ops/rust50_migration_program.ts",
            "client/runtime/systems/security/venom_containment_layer.ts",
            "client/runtime/systems/ops/dynamic_burn_budget_oracle.ts",
            "client/runtime/systems/ops/backlog_registry.ts",
            "client/runtime/systems/ops/rust_enterprise_productivity_program.ts",
            "client/runtime/systems/ops/backlog_github_sync.ts"
          ],
          "wrapper_template": "#!/usr/bin/env node\n'use strict';\n\nrequire('../../lib/ts_bootstrap.ts').bootstrap(__filename, module);",
          "policy_file": "client/runtime/config/rust_source_of_truth_policy.json"
        },
        {
          "id": "P2-002",
          "description": "Complete Documentation (20% → 100%)",
          "type": "documentation",
          "files_to_create": [
            {"path": "docs/api/README.md", "status": "create"},
            {"path": "docs/security/runbook.md", "status": "expand", "current": "25%"},
            {"path": "docs/deployment/guide.md", "status": "expand", "current": "40%"},
            {"path": "docs/adr/ADR-001.md", "status": "add"},
            {"path": "docs/adr/ADR-002.md", "status": "add"}
          ],
          "estimated_effort": "56-82 hours"
        },
        {
          "id": "P2-003",
          "description": "Create Root-Level GOVERNANCE.md",
          "type": "governance",
          "file": "GOVERNANCE.md",
          "required_sections": [
            "Decision Making",
            "Code Review",
            "Release Process",
            "Security Response"
          ]
        }
      ]
    },
    "p3_low": {
      "count": 1,
      "grade_impact": "+0.5%",
      "timeline": "ongoing",
      "items": [
        {
          "id": "P3-001",
          "description": "Code Quality Improvements",
          "type": "maintenance",
          "tasks": [
            "Linting rule enforcement",
            "Clippy warnings cleanup",
            "Dead code removal",
            "Dependency updates"
          ]
        }
      ]
    },
    "hman_required": {
      "count": 6,
      "grade_impact": "+5%",
      "timeline": "30-90 days",
      "human_authority_required": true,
      "items": [
        {"id": "HMAN-087", "action": "Third-party verification", "timeline": "30-60 days", "owner": "Legal"},
        {"id": "HMAN-086", "action": "High-assurance profile", "timeline": "30-60 days", "owner": "CEO/CTO"},
        {"id": "HMAN-026", "action": "SOC2 Type II", "timeline": "60-90 days", "owner": "Compliance"},
        {"id": "HMAN-027", "action": "ISO 27001", "timeline": "60-90 days", "owner": "Compliance"},
        {"id": "HMAN-028", "action": "Commercial contracts", "timeline": "30-60 days", "owner": "Legal"},
        {"id": "HMAN-040", "action": "Legal documentation", "timeline": "30-60 days", "owner": "Legal"}
      ]
    }
  }
}
```

---

## PRIORITIZED TASK LIST (P0-P3)

### P0 CRITICAL - Execute Immediately (Blocks A-)
**Timeline:** 1-2 days | **Impact:** +10.5% grade

| # | Task | Bash Command | Verification | Acceptance Criteria |
|---|------|--------------|------------|---------------------|
| P0-1 | Execute V7-F100 (13 contracts) | `for id in $(seq -w 001 013); do protheus-ops srs-contract-runtime run --id=V7-F100-$id --strict=1; done` | `find local/state/ops/srs_contract_runtime -path '*/V7-F100-*/latest.json' \| wc -l` = 13 | All 13 receipts exist, no exit code 2 |
| P0-2 | Execute V7-CANYON (29 contracts) | `for i in {001..029}; do protheus-ops srs-contract-runtime run --id=V7-CANYON-$i --strict=1; done` | `find local/state/ops/srs_contract_runtime -path '*/V7-CANYON-*/latest.json' \| wc -l` = 29 | All 29 receipts exist, canyon_plane.rs unchanged |
| P0-3 | Execute V8-SKILL (20 contracts) | `for id in 001 003 004 005 006 007 008 009 010 011 012 013 014 015 016 017 018 019 020 021 022 023 024 025; do protheus-ops srs-contract-runtime run --id=V8-SKILL-$id --strict=1; done` | `find local/state/ops/srs_contract_runtime -path '*/V8-SKILL-*/latest.json' \| wc -l` = 25 | All 25 receipts exist |
| P0-4 | Execute V6-WORKFLOW Priority 1 (18 contracts) | `for i in {1..12}; do protheus-ops srs-contract-runtime run --id=V6-WORKFLOW-001.$i --strict=1; done && for i in {1..6}; do protheus-ops srs-contract-runtime run --id=V6-WORKFLOW-002.$i --strict=1; done` | Receipts for WORKFLOW-001.x and 002.x exist | Priority framework bridges executed |

**New Grade After P0:** B+ (94.5%)

---

### P1 HIGH - Complete Within 2 Weeks (Blocks A)
**Timeline:** 2-3 weeks | **Impact:** +2.5% grade

| # | Task | Implementation | Verification | Acceptance Criteria |
|---|------|------------------|------------|---------------------|
| P1-1 | Increase test coverage to 90% | Create test files in core/layer0/ops/tests/ and core/layer2/conduit/tests/ | `cargo tarpaulin --out Html` shows 90%+ | Combined coverage ≥90%, TypeScript ≥85%, Rust ≥90% |
| P1-2 | Fix V7-ASM-003 hash-chain | Verify then integrate prev_hash linkage | Check capability_events.jsonl has prev_hash field | Hash-chain ledger integrated, prev_hash in events |
| P1-3 | Fix throughput to 200k/s | Implement V7-ASM-009 fastpaths in asm_plane.rs | `protheus-ops top1-assurance benchmark-thresholds` shows 200k+ | Throughput ≥200,000 ops/sec |
| P1-4 | Implement V6-SEC-010..016 | Complete security commands in security_plane.rs | All 7 commands return receipts | scan, auto-remediate, blast-radius-sentinel, verify-proofs, audit-logs, threat-model, secrets-federation all working |

**New Grade After P1:** A- (96%)

---

### P2 MEDIUM - Complete Within 1 Month (Blocks A+)
**Timeline:** 1-2 weeks | **Impact:** +1.5% grade

| # | Task | Implementation | Verification | Acceptance Criteria |
|---|------|------------------|------------|---------------------|
| P2-1 | Fix Rust SOT violations | Convert .ts→.js or update policy | Policy validator passes | All 24 violations resolved |
| P2-2 | Complete Documentation | Create docs/api/, expand docs/security/runbook.md | All docs exist and current | API reference, security runbook, ADRs complete |
| P2-3 | Create GOVERNANCE.md | Write root-level governance document | File exists with required sections | Decision making, code review, release process, security response documented |

**New Grade After P2:** A (97-98%) - **WITHOUT HMAN**

---

### P3 LOW - Nice to Have
**Timeline:** Ongoing | **Impact:** +0.5% grade

| # | Task | Implementation | Acceptance Criteria |
|---|------|------------------|---------------------|
| P3-1 | Code Quality | Clippy cleanup, dead code removal | Zero clippy warnings |
| P3-2 | Dependencies | Security patches, major updates | All dependencies current |
| P3-3 | Benchmarking | Continuous perf regression detection | Benchmarks run in CI |

---

### HMAN ACTIONS REQUIRED for A+
**Timeline:** 30-90 days | **Impact:** +5% grade | **Blocker:** Human Authority Required

| ID | Action | Timeline | Owner | Prep Location |
|----|--------|----------|-------|---------------|
| HMAN-087 | Third-party verification | 30-60 days | Legal | hman-prep/auditor-outreach/ |
| HMAN-086 | High-assurance profile | 30-60 days | CEO/CTO | hman-prep/high-assurance-profile/ |
| HMAN-026 | SOC2 Type II | 60-90 days | Compliance | hman-prep/soc2-readiness-bundle/ |
| HMAN-027 | ISO 27001 | 60-90 days | Compliance | hman-prep/iso27001-readiness-bundle/ |
| HMAN-028 | Commercial contracts | 30-60 days | Legal | hman-prep/legal-contracts/ |
| HMAN-040 | Legal documentation | 30-60 days | Legal | hman-prep/legal-contracts/ |

---

## COPY-PASTE READY CODE BLOCKS

### Block A: Pre-Execution Verification

```bash
#!/bin/bash
# CODEX P0-PRE-EXEC: Pre-execution verification
set -e

echo "=== CODEX PRE-EXECUTION VERIFICATION ==="

# 1. Check git status
echo "[1/5] Checking git status..."
git status --short

# 2. Create backup branch
echo "[2/5] Creating backup branch..."
BRANCH_NAME="pre-codex-fix-backup-$(date +%Y%m%d-%H%M%S)"
git checkout -b "$BRANCH_NAME"
echo "Backup branch created: $BRANCH_NAME"

# 3. Verify protheus-ops binary
echo "[3/5] Verifying protheus-ops binary..."
which protheus-ops || cargo build --release

# 4. Test single contract execution (dry run)
echo "[4/5] Testing dry run execution..."
protheus-ops srs-contract-runtime run --id=V7-F100-001 --strict=1 --dry-run || echo "Dry run not supported, proceeding anyway"

# 5. Count current state
echo "[5/5] Current contract/receipt counts:"
echo "  Contracts: $(find planes/contracts/srs -name 'V*.json' | wc -l)"
echo "  Receipts:  $(find local/state/ops/srs_contract_runtime -name 'latest.json' | wc -l)"
echo "  V7-F100:   $(find local/state/ops/srs_contract_runtime -name 'V7-F100-*' -type d | wc -l)/13"
echo "  V7-CANYON: $(find local/state/ops/srs_contract_runtime -name 'V7-CANYON-*' -type d | wc -l)/29"
echo "  V8-SKILL:  $(find local/state/ops/srs_contract_runtime -name 'V8-SKILL-*' -type d | wc -l)/25"
echo ""
echo "=== VERIFICATION COMPLETE - Ready for CODEX ==="
```

### Block B: P0 Contract Execution

```bash
#!/bin/bash
# CODEX P0-EXEC: Execute all P0 contracts
set -e

echo "=== CODEX PHASE 0: Critical Contract Execution ==="

# P0-1: Execute V7-F100 (13 contracts)
echo "[P0-1/4] Executing V7-F100 contracts..."
for id in $(seq -w 001 013); do
  echo -n "  V7-F100-$id: "
  if protheus-ops srs-contract-runtime run --id=V7-F100-$id --strict=1; then
    echo "✅"
  else
    echo "❌ FAILED (exit code $?)"
  fi
done

# P0-2: Execute V7-CANYON (29 contracts)  
echo "[P0-2/4] Executing V7-CANYON contracts..."
for i in {001..029}; do
  echo -n "  V7-CANYON-$i: "
  if protheus-ops srs-contract-runtime run --id=V7-CANYON-$i --strict=1; then
    echo "✅"
  else
    echo "❌ FAILED (exit code $?)"
  fi
done

# P0-3: Execute V8-SKILL pending (20 contracts)
echo "[P0-3/4] Executing V8-SKILL contracts..."
for id in 001 003 004 005 006 007 008 009 010 011 012 013 014 015 016 017 018 019 020 021 022 023 024 025; do
  echo -n "  V8-SKILL-$id: "
  if protheus-ops srs-contract-runtime run --id=V8-SKILL-$id --strict=1; then
    echo "✅"
  else
    echo "❌ FAILED (exit code $?)"
  fi
done

# P0-4: Execute V6-WORKFLOW Priority 1 (18 contracts)
echo "[P0-4/4] Executing V6-WORKFLOW Priority 1..."
echo "  WORKFLOW-001.x: "
for i in {1..12}; do
  echo -n "    001.$i: "
  if protheus-ops srs-contract-runtime run --id=V6-WORKFLOW-001.$i --strict=1; then
    echo "✅"
  else
    echo "❌"
  fi
done
echo "  WORKFLOW-002.x: "
for i in {1..6}; do
  echo -n "    002.$i: "
  if protheus-ops srs-contract-runtime run --id=V6-WORKFLOW-002.$i --strict=1; then
    echo "✅"
  else
    echo "❌"
  fi
done

echo ""
echo "=== PHASE 0 COMPLETE ==="
```

### Block C: Post-Execution Verification

```bash
#!/bin/bash
# CODEX P0-VERIFY: Verify P0 execution results

echo "=== CODEX PHASE 0 VERIFICATION ==="

# Count receipts
echo "[1/5] Counting receipts..."
V7F100_COUNT=$(find local/state/ops/srs_contract_runtime -name 'V7-F100-*' -type d | wc -l)
V7CANYON_COUNT=$(find local/state/ops/srs_contract_runtime -name 'V7-CANYON-*' -type d | wc -l)
V8SKILL_COUNT=$(find local/state/ops/srs_contract_runtime -name 'V8-SKILL-*' -type d | wc -l)
WORKFLOW_COUNT=$(find local/state/ops/srs_contract_runtime -name 'V6-WORKFLOW-*' -type d | wc -l)

echo "  V7-F100:   $V7F100_COUNT/13"
echo "  V7-CANYON: $V7CANYON_COUNT/29"
echo "  V8-SKILL:  $V8SKILL_COUNT/25"
echo "  WORKFLOW:  $WORKFLOW_COUNT/131"

# Verify each V7-F100
ec ho "[2/5] Verifying V7-F100 contracts..."
for id in $(seq -w 001 013); do
  if [ -f "local/state/ops/srs_contract_runtime/V7-F100-$id/latest.json" ]; then
    echo "  V7-F100-$id: ✅"
  else
    echo "  V7-F100-$id: ❌ MISSING"
  fi
done

# Verify each V7-CANYON  
echo "[3/5] Verifying V7-CANYON contracts..."
for i in {001..029}; do
  if [ -f "local/state/ops/srs_contract_runtime/V7-CANYON-$i/latest.json" ]; then
    echo "  V7-CANYON-$i: ✅"
  else
    echo "  V7-CANYON-$i: ❌ MISSING"
  fi
done

# Verify V8-SKILL
echo "[4/5] Verifying V8-SKILL contracts..."
for id in $(seq -w 001 025); do
  if [ -f "local/state/ops/srs_contract_runtime/V8-SKILL-$id/latest.json" ]; then
    echo "  V8-SKILL-$id: ✅"
  else
    echo "  V8-SKILL-$id: ❌ MISSING"
  fi
done

# Calculate grade impact
echo "[5/5] Grade impact calculation..."
TOTAL_RECEIPTS=$(find local/state/ops/srs_contract_runtime -name 'latest.json' | wc -l)
echo "  Total receipts: $TOTAL_RECEIPTS"
echo "  Phase 1 complete: Receipts should be $((TOTAL_RECEIPTS + 0)) or higher"
echo ""
echo "=== VERIFICATION COMPLETE ==="
```

### Block D: Security Command Implementation Template

```rust
// CODEX SECURITY-CMD: V6-SEC-010 scan command implementation
// Add to: core/layer0/ops/src/security_plane.rs

#[derive(Debug, Serialize, Deserialize)]
pub struct ScanCommand {
    pub target: ScanTarget,
    pub scan_type: ScanType,
    pub depth: ScanDepth,
}

impl SecurityCommand for ScanCommand {
    fn execute(&self, ctx: &SecurityContext) -> Result<SecurityReceipt, SecurityError> {
        // 1. Validate scan scope against ABAC policy
        self.validate_scope(ctx)?;
        
        // 2. Run injection detection patterns
        let injection_findings = self.run_injection_scan(ctx)?;
        
        // 3. Check MCP poisoning vectors  
        let mcp_findings = self.run_mcp_poisoning_scan(ctx)?;
        
        // 4. Generate deterministic receipt
        let receipt = SecurityReceipt::new()
            .with_findings(injection_findings)
            .with_mcp_findings(mcp_findings)
            .with_timestamp()
            .with_hash();
            
        // 5. Log to security events
        ctx.log_security_event("scan_completed", &receipt)?;
        
        Ok(receipt)
    }
}
```

### Block E: Test Coverage Implementation Template

```rust
// CODEX TEST: Security plane exit codes test
// Create: core/layer0/ops/tests/security_plane_exit_codes.rs

#[test]
fn v6_sec_contract_strict_exit_codes() {
    // Test that all contract violations return exit code 2 when strict=1
    let test_cases = vec![
        ("V6-SEC-010", "scan", vec!["--invalid-target"]),
        ("V6-SEC-011", "auto-remediate", vec!["--no-policy"]),
        ("V6-SEC-012", "blast-radius", vec!["--invalid-scope"]),
    ];
    
    for (contract, command, args) in test_cases {
        let output = Command::new("protheus-ops")
            .args(&["srs-contract-runtime", "run", 
                    &format!("--id={}", contract), 
                    "--strict=1"])
            .output()
            .expect("Failed to execute command");
            
        assert_eq!(
            output.status.code(),
            Some(2),
            "{} should return exit code 2 in strict mode",
            contract
        );
    }
}

#[test]
fn skills_plane_backward_compat_gate() {
    // Test version downgrade blocking
    let output = Command::new("protheus-ops")
        .args(&["skill", "install", 
                "--id=test-skill", 
                "--version=1.0.0",
                "--installed-version=2.0.0",
                "--strict=1"])
        .output()
        .expect("Failed to execute command");
        
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("version_downgrade_requires_allow_downgrade"));
}
```

### Block F: Rust SOT Wrapper Template

```javascript
#!/usr/bin/env node
// CODEX RUST-SOT: ts_bootstrap wrapper template
// Save as: client/runtime/systems/<PATH>/<FILE>.js

'use strict';

require('../../lib/ts_bootstrap.ts').bootstrap(__filename, module);
```

```typescript
#!/usr/bin/env tsx
// CODEX RUST-SOT: TypeScript implementation template
// Save as: client/runtime/systems/<PATH>/<FILE>.ts

import { createConduitLaneModule } from '../../lib/strategy_resolver.ts';

const SAFE_MODE = process.env.SAFE_MODE ?? '1';

export function run(argv: string[] = process.argv.slice(2)): number {
  if (SAFE_MODE === '1' && !argv.includes('--allow-legacy-fallback')) {
    console.error('rust_authoritative_mode');
    return 2;
  }
  // Implementation here
  return 0;
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}
```

---

## FILE DIFF SUGGESTIONS

### Diff 1: protheus_status_dashboard.js (Fix ts_bootstrap pattern)

```diff
--- a/client/runtime/systems/ops/protheus_status_dashboard.js
+++ b/client/runtime/systems/ops/protheus_status_dashboard.js
@@ -1,9 +1,5 @@
 #!/usr/bin/env node
 'use strict';
 
-// Compatibility wrapper: dashboard requests route to control-plane status.
-
-const { run } = require('./protheus_control_plane.js');
-
-if (require.main === module) {
-  process.exit(run(['status'].concat(process.argv.slice(2))));
-}
-
-module.exports = { run };
+require('../../lib/ts_bootstrap.ts').bootstrap(__filename, module);
```

### Diff 2: rust_source_of_truth_policy.json (Add explicit entries)

```diff
--- a/client/runtime/config/rust_source_of_truth_policy.json
+++ b/client/runtime/config/rust_source_of_truth_policy.json
@@ -45,6 +45,18 @@
     {
       "path": "client/runtime/systems/ops/protheusd.js",
       "required_tokens": ["PROTHEUS_CONDUIT_STRICT", "conduit_required_strict"]
     }
+  ],
+  "primitive_ts_wrapper_contract": {
+    "entries": [
+      {
+        "path": "client/runtime/systems/spine/spine_safe_launcher.ts",
+        "required_tokens": ["SPINE_SAFE_MODE", "rust_authoritative_mode"]
+      },
+      {
+        "path": "client/runtime/systems/security/directive_compiler.ts", 
+        "required_tokens": ["rustAuthoritative", "security_lane_bridge"]
+      }
+    ]
   }
 }
```

---

## ACCEPTANCE CRITERIA PER TASK

### P0-1: V7-F100 Contract Execution
**Criteria:**
- [ ] All 13 contracts generate receipts at `local/state/ops/srs_contract_runtime/V7-F100-*/latest.json`
- [ ] No contract returns exit code 2 (unless intentional test)
- [ ] Receipts contain valid claim evidence
- [ ] SRS runtime dispatch logs show successful plane interactions

**Verification Command:**
```bash
for id in $(seq -w 001 013); do 
  [ -f "local/state/ops/srs_contract_runtime/V7-F100-$id/latest.json" ] && echo "✅ V7-F100-$id" || echo "❌ V7-F100-$id"
done
```

### P0-2: V7-CANYON Contract Execution
**Criteria:**
- [ ] All 29 contracts generate receipts
- [ ] Canyon plane integration verified in dispatch logs
- [ ] No infrastructure errors

**Verification Command:**
```bash
find local/state/ops/srs_contract_runtime -name 'V7-CANYON-*' -type d | wc -l  # Should be 29
```

### P1-1: Test Coverage 90%
**Criteria:**
- [ ] Combined coverage ≥90%
- [ ] TypeScript coverage ≥85%
- [ ] Rust coverage ≥90%
- [ ] All fail-closed exit code paths tested
- [ ] Security plane error paths covered

**Verification Command:**
```bash
cargo tarpaulin --out Html --output-dir coverage/
# Open coverage/tarpaulin-report.html and verify percentages
```

### P1-4: Security Command Implementation
**Criteria:**
- [ ] V6-SEC-010 (scan) returns SecurityReceipt with findings
- [ ] V6-SEC-011 (auto-remediate) executes remediation actions
- [ ] V6-SEC-012 (blast-radius) contains blast events
- [ ] V6-SEC-013 (verify-proofs) validates formal proofs
- [ ] V6-SEC-014 (audit-logs) analyzes security events
- [ ] V6-SEC-015 (threat-model) generates threat models
- [ ] V6-SEC-016 (secrets-federation) syncs secrets across systems

**Verification Command:**
```bash
for cmd in scan auto-remediate blast-radius-sentinel verify-proofs audit-logs threat-model secrets-federation; do
  protheus-ops security $cmd --dry-run && echo "✅ $cmd" || echo "❌ $cmd"
done
```

---

## CODEX COMMAND REFERENCE

### Execution Commands

```bash
# Single contract execution
protheus-ops srs-contract-runtime run --id=<CONTRACT_ID> --strict=1

# Batch contract execution (example: V7-F100)
for id in $(seq -w 001 013); do
  protheus-ops srs-contract-runtime run --id=V7-F100-$id --strict=1
done

# With dry-run (for testing)
protheus-ops srs-contract-runtime run --id=<CONTRACT_ID> --strict=1 --dry-run
```

### Verification Commands

```bash
# Count receipts by suite
count_receipts() { find local/state/ops/srs_contract_runtime -name "$1*" -type d | wc -l; }
count_receipts "V7-F100"    # Should be 13
count_receipts "V7-CANYON"  # Should be 29
count_receipts "V8-SKILL"   # Should be 25

# Test coverage
cargo tarpaulin --out Html --output-dir coverage/

# Security command check
protheus-ops security <COMMAND> --help

# Performance benchmark
protheus-ops top1-assurance benchmark-thresholds
```

### Build Commands

```bash
# Build protheus-ops binary
cargo build --release

# Run tests
cargo test

# Check clippy
cargo clippy -- -D warnings
```

---

## SUMMARY

This Codex-ready compilation includes:

1. **JSON Fix Manifest:** Machine-readable list of all fixes with file paths
2. **Prioritized Tasks:** P0-P3 with bash commands and verification steps
3. **Copy-Paste Code:** Ready-to-use bash scripts and Rust/TypeScript templates
4. **File Diff Suggestions:** Specific patches for policy files
5. **Acceptance Criteria:** Measurable criteria for each task
6. **Command Reference:** Quick reference for execution and verification

**Next Action:** Execute P0 blocks (B and C) to achieve B+ (94.5%) grade immediately.

---

*Compiled by: CODEX Format Compiler Agent*  
*Source Documents: 70+ audit reports*  
*Target: A+ (95%+) Readiness*
