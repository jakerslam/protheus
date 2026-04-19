# Pre-Deployment Checklist

**Version:** 1.0  
**Last Updated:** April 15, 2026  
**Owner:** Infrastructure Team  
**Applies To:** All Protheus infrastructure deployments

## Overview

This checklist ensures consistent and safe deployment practices across the Protheus trading infrastructure. Use this document before any deployment to production or staging environments.

## Deployment Preparation

### 1. Code Review Verification

- [ ] All changes have been reviewed by at least one team member
- [ ] Security-sensitive changes have security team approval
- [ ] Configuration changes are documented in the deployment ticket
- [ ] Database migrations (if any) have been tested in staging

### 2. Testing Requirements

- [ ] Unit tests pass locally (`make test`)
- [ ] Integration tests pass in CI/CD pipeline
- [ ] Smoke tests pass in staging environment
- [ ] Backward compatibility verified (if applicable)

### 3. Communication

- [ ] Deployment scheduled in shared calendar
- [ ] Trading desk notified of planned changes
- [ ] Rollback plan documented and reviewed
- [ ] Incident response team on standby (for major deployments)

## Pre-Deployment Checks

### Environment Verification

```bash
# Verify target environment
./scripts/utils/health-check.sh --component=all --verbose

# Check current system status
curl -s http://localhost:8080/health | jq .

# Review recent logs for anomalies
./scripts/utils/log-analyzer.sh --mode=errors -s "1 hour ago" /var/log/protheus/app.log
```

### Checklist Items

- [ ] Target environment passes all health checks
- [ ] Sufficient disk space available (>20% free)
- [ ] Database connections within normal limits
- [ ] No active incidents or degraded services
- [ ] Backup completion verified (within last 24 hours)
- [ ] Feature flags configured correctly for gradual rollout

## Deployment Execution

### During Deployment

- [ ] Monitor deployment logs in real-time
- [ ] Verify each service starts successfully
- [ ] Check for error spikes in application logs
- [ ] Validate health check endpoints respond correctly
- [ ] Confirm database migrations apply without errors

### Immediate Post-Deployment

- [ ] Run smoke tests against deployed services
- [ ] Verify critical user workflows
- [ ] Check monitoring dashboards for anomalies
- [ ] Confirm log aggregation is functioning
- [ ] Validate metrics are being reported correctly

## Post-Deployment Validation

### Functional Verification

- [ ] Core trading functionality tested
- [ ] Risk management systems responding correctly
- [ ] Order entry and execution paths verified
- [ ] Reporting and reconciliation jobs running

### Monitoring & Alerting

- [ ] No new ERROR level log entries
- [ ] Memory usage within expected range
- [ ] CPU utilization normal
- [ ] API response times acceptable
- [ ] No increase in error rates

## Rollback Criteria

**Immediately initiate rollback if ANY of the following occur:**

1. Critical functionality broken (trading, risk, execution)
2. Data integrity concerns detected
3. Performance degradation >50% from baseline
4. Security vulnerabilities introduced
5. Unable to resolve within 30 minutes

## Deployment Sign-Off

| Role | Name | Signature | Date |
|------|------|-----------|------|
| Deployer | | | |
| Reviewer | | | |
| Trading Desk Representative | | | |

## Post-Deployment Review

*(To be completed within 48 hours for significant deployments)*

- [ ] Deployment retrospective scheduled
- [ ] Lessons learned documented
- [ ] Process improvements identified
- [ ] Checklist updated based on experience

## Related Documentation

- [Incident Response Runbook](./incident-response-runbook.md)
- [Deployment Procedures](./deployment-procedures.md)
- [Health Check Script](../scripts/utils/health-check.sh)
- [Log Analyzer Tool](../scripts/utils/log-analyzer.sh)

---

*Remember: When in doubt, rollback. It's always safer to revert and investigate than to push through issues in production.*

*Last reviewed: April 15, 2026*
