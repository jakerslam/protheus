# Current Truth Freshness Contract

Owner: Observability.

Runtime decisions may rely only on `current_live_truth`. Everything else is context until refreshed or reviewed.

## Freshness tiers

- `current_live_truth`: decision-authoritative and promotion-eligible.
- `recent_but_not_current`: triage context; conditional human review before promotion.
- `historical_trend`: trend context only.
- `stale_reference_only`: historical reference only; never release-blocker or issue-promotion authority.

## Required consumers

- Kernel Sentinel report budget freshness classifier.
- Kernel Sentinel final output guide.
- Sentinel boundedness repair lane.
- Sentinel release bridge repair lane.

## Enforcement

`npm run -s ops:observability:current-truth:guard`
