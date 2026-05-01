# Kernel Sentinel Final Output

Owner: Assurance / Observability
Status: canonical operator report guide

Kernel Sentinel final output is intentionally compact. Operators should not need to inspect huge raw state artifacts to understand the current Sentinel verdict, top findings, evidence refs, and next actions.

## Canonical Artifacts

- `local/state/kernel_sentinel/kernel_sentinel_final_report_current.json`
- `local/state/kernel_sentinel/kernel_sentinel_report_current.json`
- `local/state/kernel_sentinel/kernel_sentinel_verdict.json`
- `local/state/kernel_sentinel/kernel_sentinel_health_current.json`
- `local/state/kernel_sentinel/issues.jsonl`
- `local/state/kernel_sentinel/suggestions.jsonl`
- `local/state/kernel_sentinel/automation_candidates.jsonl`

## Final Report Shape

The final report must include:

- `verdict`: strict mode, release verdict, critical count, malformed count, and release blockers.
- `summary`: compact status, severity, category, source coverage, stale evidence, and data starvation counts.
- `top_findings`: only release-quality findings with evidence refs, owner guess, root-cause hypothesis, freshness support, and concrete next action.
- `triage_findings`: weak, stale, unevidenced, ownerless, or actionless findings that need review before promotion.
- `root_cause_clusters`: duplicate symptom families collapsed into structural issue clusters.
- `promotion_lane`: draft-only TODO/GitHub promotion candidates that require human review.
- `artifact_refs`: pointers to full internal report, verdict, health, findings, issues, suggestions, and automation candidates.
- `report_budget`: serialized byte budget, retained/dropped counts, and proof that raw evidence and the full internal report are not embedded.

## Operator Rules

- Treat `release_blockers` as the first attention surface.
- Treat `top_findings` as review-ready, not automatically mutable.
- Treat `triage_findings` as context until they pass the quality filter.
- Treat `stale_reference_only` findings as historical reference only; refresh evidence before turning them into work.
- Use `artifact_refs` to inspect full detail only after the compact report identifies a reason.
- Do not paste raw evidence streams into the report; raw evidence stays in append-only streams.

## Validation

The regression suite for this report is:

```bash
cargo test --manifest-path core/layer0/ops/Cargo.toml --lib kernel_sentinel::report_budget -- --nocapture
```

That suite proves the final report stays budgeted, excludes raw evidence payloads, releases only quality-filtered findings, blocks stale findings from promotion, clusters repeated symptoms, and collapses top findings when the byte budget is tiny.
