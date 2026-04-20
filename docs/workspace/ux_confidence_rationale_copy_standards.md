# Confidence + Rationale Copy Standards (V11-TODO-007)

Version: `v11_confidence_rationale_v1`

Purpose: keep confidence/rationale wording concise, consistent, and non-jargony across chat/status/receipt surfaces.

## Confidence Labels

| Range | Label | User-facing meaning |
| --- | --- | --- |
| `>= 0.85` | `high_confidence` | Signals are aligned and checks are healthy. |
| `0.60 - 0.84` | `medium_confidence` | Core checks pass, but follow-up is still warranted. |
| `< 0.60` | `low_confidence` | Evidence is limited or inconsistent; avoid strong claims. |

## Rationale Blurb Rules

1. Single sentence.
2. Use plain language (no internal acronyms unless unavoidable).
3. Lead with actionability when contracts fail.
4. Never claim certainty when confidence is medium/low.

## Canonical Blurbs

- Contract failed: `Output contract failed; hold response and repair before surfacing.`
- Final response incomplete: `Final response contract is incomplete; retry with full contract coverage.`
- Health degraded: `Runtime health is degraded; route through troubleshooting before user-facing claims.`
- High confidence: `Signals are aligned across execution, provider resolution, and completion checks.`
- Medium confidence: `Core checks passed, but some lanes need follow-up monitoring.`
- Low confidence: `Confidence is limited; collect more evidence before asserting strong conclusions.`

## Application Contract

Surfaces that expose confidence must include:

- numeric confidence value
- confidence label from the table above
- one canonical rationale blurb
- copy standard version tag
