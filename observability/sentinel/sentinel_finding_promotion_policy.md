# Sentinel Finding Promotion Policy

Owner: Observability / Kernel Sentinel
Status: active policy

Sentinel findings may inform TODOs and issues only when they are useful repair material, not raw anxiety.

## Required Shape

A promoted Sentinel finding must include:

- `violated_law`: `usability`, `reliability`, or `simplicity`
- `evidence_refs`
- `freshness` or recurrence support
- `owner_guess`
- `root_cause_hypothesis`
- `concrete_next_action`
- `falsification_probe`
- `suggested_todo_or_issue`
- `human_review_required`

## Promotion States

- `raw_observation`: evidence exists but is not ready for operator work.
- `triage_to_todo`: likely useful, but still requires human/Codex review.
- `todo_ready`: shaped enough for TODO promotion, still not auto-mutated.
- `issue_ready`: shaped enough for GitHub issue drafting, still not auto-filed unless policy explicitly allows.
- `stale_reference_only`: historical evidence; must be refreshed before promotion.

## Hard Rules

- Stale evidence must not become a release blocker without fresh corroboration.
- Eval-quality symptoms must remain Eval-owned unless deterministic runtime evidence exists.
- Multiple symptoms should collapse into one structural repair lane when they share owner, law, root cause, and evidence family.
- Sentinel must not auto-apply patches.

## Doctrine Mapping

- Empty responses and failed user workflows map to `usability`.
- Hangs, stale evidence, release blockers, install/gateway failures, and receipt drift map to `reliability`.
- Duplicate findings, fragmented feedback, unclear boundaries, and repeated local symptom tickets map to `simplicity`.
