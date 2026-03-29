# Runbook 012: Monitoring and Alerting Setup

**Owner:** Rohan Kapoor  
**Last Updated:** 2026-03-29  
**Review Cycle:** Quarterly  
**Severity:** Operational Procedure

---

## Overview

This runbook provides guidance on configuring and maintaining the monitoring and alerting infrastructure for the Protheus platform. Proper alerting ensures the team is notified of issues before they impact users.

## Alerting Philosophy

Our alerting strategy follows these principles:

1. **Actionable alerts only** - Every alert should require human intervention
2. **Severity-appropriate routing** - Critical alerts page on-call; warnings go to Slack
3. **Context-rich notifications** - Alerts include links to relevant dashboards and runbooks
4. **Regular review** - Alert thresholds reviewed quarterly to reduce noise

## Alert Severity Levels

| Level | Response Time | Routing | Example |
|-------|--------------|---------|---------|
| P0 - Critical | 5 minutes | PagerDuty + Phone | Service down, data loss |
| P1 - High | 15 minutes | PagerDuty | Performance degraded, capacity critical |
| P2 - Medium | 1 hour | Slack #alerts | Elevated error rate, approaching limits |
| P3 - Low | Next business day | Slack #ops | Non-critical issues, optimization opportunities |

## Standard Alert Configuration

### Service Availability Alerts

```yaml
# Example alert configuration
alert: ServiceDown
expr: up{job=~"protheus-.*"} == 0
for: 2m
labels:
  severity: critical
annotations:
  summary: "Service {{ $labels.job }} is down"
  description: "Service has been down for more than 2 minutes"
  runbook_url: "https://wiki.protheus/ops/runbooks/service-down"
```

### Resource Utilization Alerts

```yaml
# Database connection pool
alert: DBPoolHigh
expr: protheus_db_pool_usage_percent > 80
for: 5m
labels:
  severity: warning
annotations:
  summary: "Database pool usage high on {{ $labels.instance }}"
  
# Disk space
alert: DiskSpaceLow
expr: (node_filesystem_avail_bytes / node_filesystem_size_bytes) < 0.1
for: 5m
labels:
  severity: high
annotations:
  summary: "Disk space below 10% on {{ $labels.instance }}"
```

### Performance Degradation Alerts

```yaml
# High latency
alert: HighLatency
expr: protheus_request_duration_seconds{quantile="0.95"} > 2
for: 5m
labels:
  severity: warning
annotations:
  summary: "95th percentile latency > 2s on {{ $labels.route }}"
```

## Alert Management Procedures

### Adding New Alerts

1. **Define the alert condition** - Must be actionable and specific
2. **Set appropriate severity** - Consider impact and urgency
3. **Add runbook link** - Every alert should point to remediation steps
4. **Test the alert** - Use `protheusctl alert test` to verify firing
5. **Document in CHANGELOG** - Include in weekly ops review

### Silencing Alerts

Sometimes alerts need temporary suppression:

```bash
# Silence for planned maintenance
protheusctl alert silence \
  --alert ServiceDown \
  --duration 30m \
  --reason "Planned database migration - see ENG-1234"
```

**Rules for silencing:**
- Always include a reason with ticket reference
- Set expiration (never permanent silences)
- Notify #ops channel when silencing high-severity alerts
- Review active silences weekly

### Alert Fatigue Reduction

If an alert fires frequently without action:

1. Check if threshold is appropriate
2. Verify if underlying issue is being addressed
3. Consider upgrading to auto-remediation if pattern is predictable
4. If truly noisy, adjust threshold or remove

## Common Issues and Remediation

### Alert Storm During Incident

**Symptom:** Multiple related alerts fire simultaneously

**Response:**
1. Acknowledge all related alerts
2. Post in #incident channel with summary
3. Focus on root cause, not individual alerts
4. Silence related noise alerts temporarily
5. Document lessons learned post-incident

### False Positive Alerts

**Symptom:** Alert fires but no actual issue exists

**Response:**
1. Check recent configuration changes
2. Verify metrics are being scraped correctly
3. Review alert expression for edge cases
4. Adjust query or add additional conditions
5. Document fix to prevent recurrence

## Dashboard Quick Links

Maintain bookmarks for incident response:

- **Overview Dashboard:** https://grafana.protheus/d/overview
- **Service Health:** https://grafana.protheus/d/service-health
- **Database Metrics:** https://grafana.protheus/d/db-metrics
- **Request Latency:** https://grafana.protheus/d/latency

## Document History

| Date | Version | Author | Changes |
|------|---------|--------|---------|
| 2026-03-29 | 1.0 | Rohan Kapoor | Initial draft |

---

*TODO(rohan): Add section on custom metrics for new services*
*TODO(rohan): Document alert testing procedures once protheusctl alert test is GA*

*This document is living documentation. All team members are encouraged to suggest improvements via PR.*
