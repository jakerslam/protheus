# Deployment Windows & Release Governance

> **Document Type:** Operational Guidelines  
> **Last Updated:** 2026-03-30  
> **Owner:** Platform Operations Team  
> **Review Cycle:** Quarterly or after major incidents

## Overview

This document defines the standardized deployment windows and release governance procedures for the Protheus platform. Following these guidelines ensures predictable release cycles while minimizing risk to production systems.

## Deployment Schedule

### Standard Windows

| Environment | Window (UTC) | Window (Pacific) | Purpose |
|-------------|-------------|------------------|---------|
| Staging | Mon-Fri 08:00-18:00 | Mon-Fri 00:00-10:00 | Continuous deployment |
| Production | Tue-Thu 14:00-17:00 | Tue-Thu 06:00-09:00 | Controlled releases |
| Hotfix | On-demand | On-demand | Critical fixes only |

### Blackout Periods

The following periods are designated as **deployment blackout windows**:

- **Month-end (EOM)**: Last business day of each month (financial close)
- **Quarter-end (EOQ)**: Last 3 business days of fiscal quarter
- **Holiday freezes**: December 23 - January 2
- **Black Friday / Cyber Monday**: Week of Thanksgiving (USA)
- **Tax season**: April 1-15 (USA)

> **Exception Process:** Emergency deployments during blackout periods require VP Engineering approval and CTO notification.

## Release Types

### 1. Standard Release

**Frequency:** Weekly (Thursday production window)

**Requirements:**
- All tests passing in staging
- Code review completed (minimum 2 approvers)
- Security scan completed with no critical findings
- QA sign-off documented
- Rollback plan documented

**Process:**
1. Tuesday: Deploy to staging, run smoke tests
2. Wednesday: Final QA validation, release notes published
3. Thursday: Production deployment during window
4. Friday: Production health monitoring report

### 2. Hotfix Release

**Frequency:** As needed

**Criteria:**
- Critical bug affecting >10% of active users
- Security vulnerability (CVSS score ≥ 7.0)
- Data integrity issue
- Compliance requirement

**Process:**
1. Branch from production tag
2. Apply minimal fix
3. Expedited review (1 approver for emergencies)
4. Deploy to staging for smoke tests
5. Deploy to production with continuous monitoring

### 3. Scheduled Maintenance Release

**Frequency:** Monthly

**Purpose:** Infrastructure updates, dependency upgrades, non-urgent security patches

**Process:**
1. Maintenance window announced 7 days in advance
2. Deploy to staging for extended testing (minimum 48 hours)
3. Execute during scheduled maintenance window
4. Full regression testing post-deployment

## Pre-Deployment Checklist

### Code Requirements

- [ ] All CI/CD checks passing
- [ ] Feature flags configured (if applicable)
- [ ] Database migrations tested on production-like data
- [ ] API schema changes documented
- [ ] Breaking changes announced 2 weeks in advance

### Operational Requirements

- [ ] Rollback procedure documented and tested
- [ ] Monitoring dashboards reviewed
- [ ] Alert thresholds verified
- [ ] On-call schedule confirmed
- [ ] Communication plan prepared

### Security Requirements

- [ ] Security scan passed (Snyk, SonarQube)
- [ ] Secrets rotated (if required)
- [ ] Access controls reviewed
- [ ] Audit logging enabled for new features

## Post-Deployment Verification

### Immediate (0-30 minutes)

1. Verify health check endpoints responding
2. Check error rates are within normal parameters
3. Validate critical user journeys
4. Confirm database connections healthy
5. Verify monitoring alerts are functional

### Short-term (2-4 hours)

1. Review error logs for anomalies
2. Monitor key performance metrics
3. Verify customer-facing features operational
4. Check integration partner health
5. Confirm scheduled jobs running

### Long-term (24-72 hours)

1. Analyze daily active user trends
2. Review performance regression tests
3. Check cost impact of new features
4. Validate data pipeline completeness
5. Monitor customer support ticket volume

## Rollback Procedures

### Rollback Triggers

Automatic rollback initiated when:
- Error rate exceeds 5% for 5 consecutive minutes
- Response time P95 increases 50% for 10 minutes
- Critical user journey fails for >2% of users
- Security anomaly detected

### Manual Rollback Steps

```bash
# 1. Alert the team
slack-notify "@channel Initiating emergency rollback for release $VERSION"

# 2. Pause deployments
ops deployment pause --service=$SERVICE

# 3. Execute rollback
ops rollback --to=$LAST_KNOWN_GOOD --service=$SERVICE

# 4. Verify stability
ops healthcheck --service=$SERVICE --duration=10m

# 5. Resume pipeline
ops deployment resume --service=$SERVICE
```

### Post-Rollback Actions

1. Preserve logs from failed deployment
2. Document rollback reason and timeline
3. Schedule post-mortem within 24 hours
4. Update runbooks with lessons learned

## Communication Plan

### Release Notifications

| Stakeholder | When | Channel | Content |
|-------------|------|---------|---------|
| Engineering | Deploy start | Slack #deployments | Deploy initiated |
| Product | Deploy complete | Slack #product-updates | Feature availability |
| Customer Success | 24h post-deploy | Email | Customer-facing changes |
| Leadership | Weekly | Email digest | Release summary |

### Status Page Updates

- **Scheduled maintenance:** Update 7 days before, 1 day before, 1 hour before
- **In-progress deployment:** Update at start and completion
- **Incidents:** Real-time updates every 15 minutes

## Metrics & KPIs

### Deployment Health

- Deployment frequency (target: ≥1/week)
- Lead time for changes (target: <1 week)
- Change failure rate (target: <5%)
- Mean time to recovery (target: <30 minutes)

### Release Quality

- Production bugs per release
- Rollback rate
- Customer-reported issues
- Performance regression incidents

## Appendix

### A. Emergency Contacts

| Role | Primary | Secondary |
|------|---------|-----------|
| Platform Ops | platform-ops@company.com | PagerDuty escalation |
| VP Engineering | rohan.kapoor@company.com | sarah.chen@company.com |
| Security | security-team@company.com | security-hotline@company.com |

### B. Related Documents

- [Incident Response Runbook](./incident-response-p1-escalation.md)
- [Environment Health Dashboard](https://grafana.company.com/d/production-health)
- [Release Automation Documentation](https://wiki.company.com/release-automation)
- [Security Incident Response](./security-incident-response.md)

---

**Document History**

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.2 | 2026-03-30 | Rohan Kapoor | Added emergency contact matrix, clarified blackout exception process |
| 1.1 | 2026-02-15 | Alex Rivera | Updated deployment windows, added PST/PDT conversion table |
| 1.0 | 2026-01-10 | Rohan Kapoor | Initial document creation |
