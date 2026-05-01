# Kernel Sentinel Boundedness Repair Lane

Owner: Observability with Validation proof support.

This lane turns Sentinel boundedness feedback into concrete repair work instead of leaving it as generic advisory noise.

## Release policy

- Human review is required before filing or promoting work.
- Auto-apply is forbidden.
- Raw evidence streams must stay outside the compact report.
- Findings must pass the Sentinel quality filter before promotion.

## Primary repair lane

`workspace_tooling_boundedness_release_lane`

Problem: workspace/file tooling can look functionally healthy while still missing boundedness proof for memory, queue depth, stale surfaces, recovery time, and compact Sentinel reporting.

Next action: promote workspace tooling boundedness into a release-gated proof lane with budgeted reports, replay evidence, and Sentinel issue-candidate quality filtering.

## Required boundedness dimensions

- `max_rss`
- `queue_depth_p95`
- `queue_depth_max`
- `stale_surface_count`
- `recovery_time_ms`
- `report_size_bytes`

## Acceptance criteria

- Workspace tooling proof artifacts include RSS, queue-depth p95/max, stale-surface count, recovery time, and serialized report size.
- Sentinel final reports summarize boundedness findings through top findings, root-cause clusters, and artifact refs without embedding raw evidence streams.
- Repeated boundedness failures collapse into one structural issue candidate with owner guess, root-cause hypothesis, concrete next action, and freshness tier.
- Release evidence refuses promotion when freshness is `stale_reference_only` or when boundedness metrics are missing.

## Validation commands

```bash
npm run -s ops:workspace-tooling:context-soak
npm run -s ops:workspace-tooling:release-proof
cargo test --manifest-path core/layer0/ops/Cargo.toml --lib kernel_sentinel::report_budget -- --nocapture
npm run -s ops:ksent:boundedness-repair:guard
```
