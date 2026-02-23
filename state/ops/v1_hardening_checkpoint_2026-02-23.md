# V1 Hardening Checkpoint (2026-02-23)

Generated: 2026-02-23T10:35:56.533Z
Window days: 14

## Score

- Weighted score: **1**
- Verdict: **PASS**
- Failed criteria: none

## Criteria

| Criterion | Pass | Weight | Detail |
|---|---:|---:|---|
| security_integrity | yes | 3 | integrity_kernel + architecture_guard |
| startup_attestation | yes | 2 | startup_attestation_verified |
| routing_health | yes | 2 | local_routing_healthy_soft_latency_escalation |
| sensory_continuity | yes | 2 | dark_eyes + queue_backlog + proposal_starvation |
| drift_control | yes | 2 | spc_in_control |
| verification_pass_rate | yes | 2 | verification_pass_rate_healthy |
| slo_runbook_coverage | yes | 2 | missing_checks=0 missing_sections=0 |
| budget_governor | yes | 2 | budget_guard_clear |
| execute_readiness | yes | 2 | ready_for_execute |
| queue_hygiene | yes | 1 | open=21 stale_open=0 |
| outcome_throughput | yes | 2 | executed=13 shipped_rate=0.462 |

## Outcome Window

```json
{
  "window_days": 14,
  "attempted": 209,
  "executed": 13,
  "shipped": 6,
  "no_change": 7,
  "reverted": 0,
  "shipped_rate": 0.462
}
```

## Notes

- This checkpoint is for unattended-6-month V1 hardening readiness.
- `budget_governor` must stay clear in unattended mode.
- Re-run after major routing/security/autonomy policy changes.

## Next Steps

- Hold current policies and continue periodic checkpoint audits.

