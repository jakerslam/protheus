# Runbook 001: Incident Response Procedures

**Owner:** Rohan Kapoor  
**Last Updated:** 2026-03-08  
**Review Cycle:** Quarterly  
**Severity Levels:** P1 (Critical), P2 (High), P3 (Medium), P4 (Low)

---

## Overview

This runbook defines standard operating procedures for handling production incidents within the Infring platform. All responders should familiarize themselves with these procedures before an incident occurs.

## Incident Classification

| Level | Impact | Response Time | Owner Notification |
|-------|--------|---------------|-------------------|
| P1 | Service unavailable, data loss imminent | 15 min | Immediate page |
| P2 | Degraded performance, partial outage | 30 min | Slack alert + email |
| P3 | Minor feature unavailable, workarounds exist | 2 hours | Email only |
| P4 | Cosmetic issues, documentation fixes | 24 hours | Ticket queue |

## Response Playbook

### Phase 1: Detection & Triage (0-15 min)

1. **Acknowledge** the alert within monitoring system
2. **Classify** severity based on impact matrix above
3. **Create** incident channel in Slack: `#incident-YYYY-MM-DD-brief`
4. **Notify** on-call engineer if P1/P2

### Phase 2: Investigation (15-45 min)

1. Review recent deployments: `git log --since="4 hours ago" --oneline`
2. Check system health dashboards
3. Collect relevant logs and metrics
4. Update incident timeline in tracking doc

### Phase 3: Mitigation (varies)

1. Prioritize service restoration over root cause analysis
2. Document all actions taken in real-time
3. If rollback needed, follow emergency rollback procedure
4. Verify restoration with smoke tests

### Phase 4: Post-Incident (within 24 hours)

1. Schedule post-mortem within 24 hours for P1/P2
2. Document timeline, root cause, and remediation
3. Create follow-up action items with owners
4. Update this runbook if procedures evolved

## Communication Templates

### Initial Acknowledgment
```
Incident #1234 acknowledged at HH:MM UTC.
Severity: [P1/P2/P3/P4] | Responder: [Name]
Initial status: Investigating
```

### Status Updates (every 30 min for P1/P2)
```
[HH:MM] Update: [Brief status statement]
ETA next update: [HH:MM]
```

## Escalation Path

1. Primary on-call → 15 min
2. Engineering Manager → 30 min
3. Director of Engineering → 1 hour
4. CTO → 2 hours (P1 only)

## Weekend/Holiday Escalation Procedures

During weekends and company holidays, the escalation path remains the same
but response time expectations are adjusted:

- P1 incidents: Same response time (15 min), all levels reachable
- P2 incidents: Extended to 45 min initial response
- P3/P4: Deferred to next business day unless customer-impacting

Contact method hierarchy:
1. PagerDuty alert (primary)
2. SMS to personal phone (if PagerDuty unacknowledged after 10 min)
3. Phone call from on-call manager (if SMS unacknowledged after 5 min)

## On-Call Rotation Handoff

For incidents spanning multiple shifts, the handoff procedure includes:
1. Explicit artifact transfer (incident doc, Slack channel, active logs)
2. Voice handoff required (no async handoff for P1/P2)
3. Incoming responder must acknowledge understanding before outgoing can stand down
4. Transfer recorded in incident timeline

See MEETING-NOTES-2026-03-20 for recent procedure updates.

## Additional Context
# NOTE(rohan): The handoff procedure should include explicit artifact transfer for
# incidents in progress. This prevents context loss during shift changes, which
# historically has caused 15-20 min delays in resolution. See MEETING-NOTES-2026-03-11.

## Useful Commands

```bash
# Quick system status check
infringctl status --all

# View recent logs
infringctl logs --tail 100 --system core

# Check active alerts
infringctl alerts list --active

# Emergency restart (requires authorization)
infringctl emergency restart --system [name]
```

## InfRing precheck to action mapping (runtime/gateway)

Use this deterministic map before ad-hoc debugging:

| Symptom (precheck) | Immediate action | Escalation if still failing |
|---|---|---|
| Dashboard health endpoint down (`/healthz` unreachable) | `infring recover --dashboard-host=127.0.0.1 --dashboard-port=4173` | `infring gateway status --dashboard-host=127.0.0.1 --dashboard-port=4173` then collect daemon logs |
| Required runtime assets missing (`infringctl verify-install --json`) | `infring update --repair --full` | rerun `infringctl verify-install --json` and open incident with output artifact |
| Stale workspace-root env reference detected | `unset INFRING_WORKSPACE_ROOT INFRING_WORKSPACE_ROOT` then `infring recover` | if mismatch persists, pin the correct root and re-run verify-install |
| Gateway route drift (`gateway status` route mismatch) | `infring update --repair --full` | run `infringctl doctor --json` and attach route mismatch rows |

Recovery evidence to capture for incident artifacts:

- `infringctl verify-install --json` output
- `infring gateway status --dashboard-host=127.0.0.1 --dashboard-port=4173`
- Dashboard health probe result: `http://127.0.0.1:4173/healthz`

## Document History

| Date | Version | Author | Changes |
|------|---------|--------|---------|
| 2026-03-08 | 1.0 | Rohan Kapoor | Initial draft |

---

*This document is living documentation. All team members are encouraged to suggest improvements via PR.*
