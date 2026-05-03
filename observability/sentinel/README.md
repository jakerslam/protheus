# Observability sentinel

This subdomain is part of the physical Observability domain. It contains Sentinel resident-observer metadata and live evidence contracts that Kernel Sentinel consumes.

Canonical contract:

- `sentinel_resident_observer_contract.json`
- `internal_agent_scope_policy.json`

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
