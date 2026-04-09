# GitHub Issues to Create

Generated from system health audit on 2026-03-10

---

## Issue #1: [CRITICAL] external_eyes Signal SLO failing - conversation_eye paused

**Labels:** bug, critical-systems, sensory

### Problem
The external_eyes subsystem has failed Signal SLO for 6 consecutive runs, triggering pain proposal PAIN-3493c9df1d4df3a6.

### Symptoms
- `unknown_code` errors from external_eyes subsystem
- conversation_eye is paused
- failed_checks=real_external_items
- Last escalation: 2026-03-08T03:39:59.533Z

### Impact
- Reduced signal fidelity from external sources
- System running with impaired sensory capacity
- Reliance on manual probes (left_only dual_brain mode)

### Suggested Fix
1. Investigate conversation_eye pause root cause
2. Restore real_external_items check functionality
3. Implement circuit breaker pattern to prevent consecutive failures

### System State
- proposal_id: PAIN-3493c9df1d4df3a6
- cooldown_until: 2026-03-08T15:39:59.531Z
- severity: medium → high (after escalation)

---

## Issue #2: [HIGH] Workflow executor degraded - SLO failures in dry-run mode

**Labels:** bug, workflows, reliability

### Problem
Workflow executor repeatedly degraded on 2026-03-08 with multiple SLO failures during dry-run execution.

### Symptoms
- workflow_executor_degraded events: 2026-03-08T23:17:02, 23:21:15, 23:26:40
- execution_success_rate: 0%
- slo_pass: false, slo_window_pass: false
- All workflows: selected=0, deferred=0, executed=0, succeeded=0

### Impact
- No workflow execution happening
- Token usage: 0 (complete stagnation)
- Queue drain rate: 0
- Operations at a standstill

### Suggested Fix
1. Investigate why workflows aren't being selected for execution
2. Check if dry_run=true is preventing actual work
3. Review workflow scheduling logic
4. Ensure handoff receipts are being processed

### Events
- Run IDs: wfexec_mmidgm19_4wvnaf, wfexec_mmidm164_l0ort3, wfexec_mmidszrv_mi3g35

---

## Issue #3: [MEDIUM] Dopamine system flatlined - 22 days without activity

**Labels:** enhancement, dopamine, motivation

### Problem
The dopamine/reward system has been inactive for 22 days, causing the system to run on "maintenance mode" rather than "growth mode."

### Symptoms
- Last recorded date: 2026-02-16
- Current streak: 1 day (was 0)
- Last score: 0
- Highest score: 0
- dopamine_pain_state.active: false
- verified_objective_entries: 0
- revenue_actions: 0

### Impact
- No reward feedback loops
- System lacks motivational signal
- Autonomy candidates not being harvested (scanned: 0, candidates: 0)
- "Unlinked high-leverage minutes" ratio at 0%

### Suggested Fix
1. Investigate why dopamine_state.json hasn't updated since Feb 16
2. Check if the dopamine-ambient cron job is running
3. Restore linkage between work activity and reward signals
4. Consider manual kickstart of dopamine tracking

### Files Affected
- client/runtime/local/state/dopamine_state.json
- client/runtime/local/state/dopamine_pain_state.json

---

## Issue #4: [MEDIUM] Proposal queue congestion - 5 autophagy candidates pending approval

**Labels:** enhancement, autophagy, governance

### Problem
Five autophagy policy candidates have been stuck in PENDING status for 48+ hours awaiting human approval.

### Candidates
| Action ID | Type | Confidence | Impact | Status |
|-----------|------|------------|--------|--------|
| act_autophagy_nyc-5518c97e817c_20260308 | stop_init_gate_readiness_blocked | 0.870 | 34.96 | PENDING |
| act_autophagy_nyc-c63fb56cb5a5_20260308 | score_only_fallback_low_execution_confidence | 0.826 | 23.45 | PENDING |
| act_autophagy_nyc-11e6395f82e5_20260308 | burn_rate_exceeded | 0.824 | 23.05 | PENDING |
| act_autophagy_nyc-767aef715a67 | stop_init_gate_budget_autopause | 0.742 | 20.65 | PENDING |
| act_autophagy_nyc-fd75a71e4835_20260308 | auto:execute_confidence_fallback | 0.700 | 19.25 | PENDING |

### Impact
- Self-optimization is gated on human review
- System cannot auto-adapt to detected inefficiencies
- Deferred autonomy = slower convergence on optimal configuration

### Suggested Fix
1. Review and approve/reject pending proposals
2. Consider lowering confidence threshold for auto-approval
3. Implement time-based auto-approval for high-confidence proposals (>0.8)
4. Create batch approval workflow for related proposals

### File Location
- client/runtime/state/state/approvals_queue.yaml

---

## Issue #5: [MEDIUM] Dual-brain consistently running left_only mode

**Labels:** enhancement, dual-brain, cognition

### Problem
The dual_brain system has been running exclusively in left_only mode for all recent decisions (March 8 manual probes), indicating the right hemisphere is not being engaged.

### Symptoms
- All decisions: mode=left_only, right_permitted=false
- Context: manual_probe
- Dates affected: 2026-03-08 (multiple times)

### Impact
- Limited creative capacity
- Reduced parallel processing
- Right-brain capabilities (synthesis, creativity, lateral thinking) unavailable
- System may be overly cautious/conservative

### Suggested Fix
1. Investigate why right_permitted=false on all probes
2. Check if right hemisphere services are healthy
3. Review task classification logic - are tasks being downgraded to "general"?
4. Consider gradual re-enabling of right hemisphere for appropriate task classes

### File Location
- client/runtime/state/state/dual_brain/history.jsonl

---

*Generated by Infring agent at 2026-03-10T11:39:00-06:00*
