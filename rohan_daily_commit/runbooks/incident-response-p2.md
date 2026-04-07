# P2 Incident Response Runbook

> **Classification:** Internal - Operations Team  
> **Last Updated:** 2026-03-30  
> **Owner:** Platform Operations  
> **Review Cycle:** Quarterly
> **Escalation Path:** P1 Incident Response

## Overview

This runbook defines standardized procedures for P2 (High Severity) incidents—issues that significantly degrade service functionality but have acceptable workarounds or limited scope.

## Trigger Criteria

A P2 incident is declared when ANY of the following conditions apply:

| Condition | Impact Threshold |
|-----------|------------------|
| Degraded service performance | >25% of users experiencing latency >2x normal |
| Partial feature unavailability | Core functionality impacted, workaround exists |
| Backup/recovery failures | Non-critical data protection concerns |
| Elevated error rates | >10% error rate on non-critical endpoints |
| Infrastructure component failure | Redundancy failover activated successfully |

## Escalation Timeline

```
T+0 min   : Incident detected via monitoring or manual report
T+15 min  : On-call engineer acknowledges and begins assessment  
T+30 min  : Severity confirmed, P2 declared if criteria met
T+45 min  : Relevant stakeholders notified via Slack
T+1 hour  : Response team mobilized (virtual, async if after-hours)
T+2 hours : If unresolved: Consider escalation to P1
T+4 hours : Executive notification if customer-facing
T+8 hours : Mandatory escalation review if still unresolved
```

## Response Workflow

### Phase 1: Detection & Triage (T+0 to T+30)

1. **Acknowledge the alert in PagerDuty**
   - Set yourself as responder
   - Link to incident tracking ticket
   - Initial Slack post in `#incidents`

2. **Initial Assessment Questions:**
   - Is this affecting production customers?
   - Is there a viable workaround?
   - Can this wait for business hours response?
   - Who else needs to be involved?

3. **Classify and tag:**
   - Severity: P2
   - Category: Performance / Availability / Security / Data
   - Affected services

### Phase 2: Mobilization (T+30 to T+1 hour)

| Action | Owner | Method |
|--------|-------|--------|
| Create incident channel | On-call | Slack: `incident-{date}-{service}` |
| Notify stakeholders | On-call | Slack mentions + email digest |
| Document timeline | On-call | Incident tracking ticket |
| Engage specialists | Service owner | Slack DM or scheduled call |

### Phase 3: Response & Mitigation

**Immediate Response (first hour):**
- Attempt rollback or known mitigation
- Document all actions in incident channel
- Update incident ticket status every 15 minutes
- Assess customer impact continuously

**Sustained Response:**
- Async updates every hour in `#incidents`
- Daily standup if multi-day resolution required
- Document workarounds in status page or customer channels

## Communication Guidelines

### Internal (Slack - `#incidents`)
```
⚠️ P2 INCIDENT - {Service Name}

Status: {Monitoring|Investigating|Mitigating|Resolved}
Impact: {Brief description}
Started: {timestamp}
Updated: {timestamp}
ETA: {estimated resolution}

Next update in: 1 hour
```

### Customer Communication (via status page)
- Post brief status update within 1 hour of declaration
- Update every 4 hours until resolved
- Mark resolved with brief summary
- Link to post-mortem when available

## Diagnostic Commands

```bash
# Check service-specific health
ops healthcheck --service={service_name} --region=all --format=detailed

# Review recent changes
deployment-log --service={service_name} --since="6 hours ago"

# Analyze performance metrics
telemetry query 'histogram_quantile(0.99, rate(response_time_bucket[5m]))' \
  --service={service_name} --threshold=1000ms

# Check for upstream dependencies
ops dependency-check --service={service_name} --depth=2
```

## Escalation to P1

Escalate P2 → P1 if ANY of the following occur:
- Workaround proves ineffective
- Customer impact expands beyond initial scope
- Revenue impact exceeds $50K/hour
- Multiple P2 incidents occur simultaneously (cascading failure)
- Security/Data integrity concerns emerge

## Post-Incident Requirements

Within **72 hours** of resolution:
- [ ] Post-mortem drafted
- [ ] Root cause categorized
- [ ] Contributing factors documented
- [ ] Action items tracked in backlog
- [ ] Prevention measures proposed
- [ ] Follow-up review scheduled (if needed)

## Related Documentation

- [P1 Incident Response](./incident-response-p1-escalation.md) - Critical incidents
- [Post-Mortem Template](./post-mortem-template.md) - Learnings documentation
- [Severity Definitions](./severity-matrix.md) - Classification guidelines
- [On-Call Rotation](./oncall-handbook.md) - Contact details and procedures

---

**Document Revision History**

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0 | 2026-03-30 | Rohan Kapoor | Initial P2 incident runbook creation based on P1 template |