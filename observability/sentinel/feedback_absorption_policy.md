# Kernel Sentinel Feedback Absorption Policy

The Kernel Sentinel is an anti-entropy observer, not a noise generator. Its feedback must converge into structural repair work instead of repeatedly restating local symptoms.

## Promotion lane

Sentinel findings move through this lane:

1. `raw_observation`: live evidence, logs, receipts, freshness, or runtime facts.
2. `candidate_finding`: normalized problem statement with evidence references.
3. `clustered_issue`: symptoms grouped by root-cause hypothesis and owner.
4. `repair_backlog_item`: human/Codex-reviewed TODO or issue with concrete next action.
5. `closed_by_evidence`: resolved only after fresh evidence shows the signature stopped recurring.

## Required release shape

A Sentinel finding is releasable only when it has:

- Evidence refs, not just prose.
- Freshness or recurrence metadata.
- Owner guess by domain.
- Root-cause hypothesis.
- Concrete next action.
- One of the Three Operating Laws affected: usability, reliability, simplicity.

## Deduplication key

Cluster repeated symptoms with:

```text
law + owner_domain + evidence_family + root_cause_hypothesis + failure_signature
```

If five symptoms have the same structural cause, Sentinel should emit one issue with symptom refs, not five separate TODOs.

## Noise controls

- Raw evidence remains in observability streams.
- Final reports contain only top findings, summaries, and refs.
- Oversized reports are invalid output, not useful output.
- Drafts are kept proposal-only until human/Codex review promotes them.
