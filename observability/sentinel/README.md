# Observability sentinel

This subdomain is part of the physical Observability domain. It contains Sentinel resident-observer metadata and live evidence contracts that Kernel Sentinel consumes.

Canonical contract:

- `sentinel_resident_observer_contract.json`
- `internal_agent_scope_policy.json`
- `usability_reliability_simplicity_enforcement_policy.json`
- `sentinel_finding_promotion_policy.md`

## Repo-wide doctrine enforcement

Kernel Sentinel enforces the repo-wide [Three Operating Laws](/Users/jay/.openclaw/workspace/docs/workspace/REAL_WORK_FIRST.md) as live system-health pressure. Informally, these are the system's "three commandments": usability, reliability, and simplicity.

Sentinel should flag recurring evidence that the system is:

- not becoming more useful for concrete user or agent work
- unreliable on install, gateway, request, release, TODO, or Sentinel feedback paths
- becoming harder to reason about through duplicate truth, vague ownership, compatibility tails, or subsystem sprawl

Sentinel may draft findings and TODO/issue candidates from those violations, but it must not auto-apply patches.

## Internal-agent scope split

Kernel Sentinel is the resident observer for live system evidence. It looks for
runtime, Kernel-truth, security, correctness, boundedness, and architectural
coherence failures.

Eval owns response-level judgment: hallucinations, wrong-tool behavior,
helpfulness, answer quality, and "the user did not get a good response" when no
deterministic runtime failure is attached.

The boundary for response failures is intentionally narrow:

- Sentinel may promote missing response findings when runtime observations,
  traces, receipts, or finalization phases show that the system failed to route,
  persist, or emit an assistant-visible response.
- Sentinel must hand off ordinary bad-answer, hallucination, and response-quality
  judgments to Eval unless those findings are corroborated by deterministic
  runtime evidence.

For now this split is captured in `internal_agent_scope_policy.json`. Later,
internal agents should graduate to explicit internal workflow templates that use
this policy as the workflow contract and handoff criteria.
