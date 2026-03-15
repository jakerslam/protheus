# Runbook 006: System Health Checks

**Owner:** Rohan Kapoor  
**Last Updated:** 2026-03-15  
**Review Cycle:** Monthly  
**Severity:** Operational Procedure

---

## Overview

This runbook defines standard health check procedures for the Protheus platform. These checks should be performed regularly during on-call shifts and before any deployment activity.

## Quick Health Check (5 minutes)

Run this checklist at the start of each on-call shift:

- [ ] All core services responding to health probes
- [ ] Database connection pool utilization < 80%
- [ ] Message queue depth within normal parameters
- [ ] No critical alerts firing in monitoring
- [ ] Recent backup completion verified

## Detailed Health Checks

### Service Health Verification

```bash
# Check all registered services
protheusctl health check --all

# Check specific service with verbose output
protheusctl health check --service core --verbose

# Check service dependencies
protheusctl health check --service core --with-deps
```

**Expected Output:**
- All services: `STATUS: healthy`
- Response time: < 500ms for health endpoint
- Dependency chain: All green

### Database Health

```bash
# Connection pool status
protheusctl db status --pool

# Query performance check (last 5 minutes)
protheusctl db metrics --slow-queries --since "5m"

# Replication lag (if applicable)
protheusctl db replication --lag
```

**Alert Thresholds:**
- Connection pool > 80%: Warning
- Connection pool > 95%: Critical
- Slow queries > 10/min: Investigation required
- Replication lag > 5 seconds: Escalate

### Message Queue Health

```bash
# Queue depth check
protheusctl queue status --all

# Consumer lag check
protheusctl queue consumers --lag

# Dead letter queue inspection
protheusctl queue dlq --count
```

**Normal Operating Ranges:**
- Main queue depth: < 1000 messages
- Consumer lag: < 30 seconds
- DLQ: < 10 messages (investigate if higher)

### Disk and Resource Health

```bash
# Disk usage across nodes
protheusctl nodes disk --threshold 80

# Memory utilization
protheusctl nodes memory --top 10

# CPU utilization (5 min average)
protheusctl nodes cpu --avg 5m
```

**Resource Thresholds:**

| Resource | Warning | Critical |
|----------|---------|----------|
| Disk Usage | 75% | 90% |
| Memory | 80% | 95% |
| CPU (avg) | 70% | 90% |

## Automated Health Checks

The following health checks run automatically via cron:

| Check | Frequency | Alert Channel |
|-------|-----------|---------------|
| Service heartbeat | Every 60s | PagerDuty |
| DB connection pool | Every 5min | Slack #alerts |
| Queue depth | Every 2min | Slack #alerts |
| Disk usage | Every 15min | Email |

## Manual Health Check Script

For comprehensive manual verification:

```bash
#!/bin/bash
# save as: health-check-manual.sh

echo "=== Protheus Health Check ==="
echo "Timestamp: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo ""

echo "[1/5] Service Health..."
protheusctl health check --all --format json | jq '.status'

echo "[2/5] Database Pool..."
protheusctl db status --pool | grep -E "(active|idle|max)"

echo "[3/5] Queue Status..."
protheusctl queue status --all | grep -E "(queue|depth)"

echo "[4/5] Disk Usage..."
df -h | grep protheus

echo "[5/5] Recent Errors..."
protheusctl logs --level error --since "5m" | wc -l
echo "   (error count in last 5 min)"

echo ""
echo "=== Health Check Complete ==="
```

## Health Check Failures

### Service Unhealthy

1. Check service logs: `protheusctl logs --service [name] --tail 100`
2. Verify dependencies are healthy
3. Check for recent deployments
4. If isolated to one node, consider draining
5. Escalate to service owner if not resolved in 15 min

### Database Connection Pool Exhausted

1. Check for connection leaks: `protheusctl db connections --list`
2. Identify long-running queries: `protheusctl db queries --long-running`
3. Consider temporary pool size increase
4. If persistent, may indicate application issue

### Queue Backlog

1. Check consumer status: `protheusctl queue consumers --status`
2. Verify consumers are processing (not stalled)
3. Check for message processing errors
4. Consider scaling consumers if sustained load

## Health Check History

Maintain a log of health check results:

```
# Template for shift handoff
Date: YYYY-MM-DD
On-call: [Name]
Health Check Status: [PASS/WARNING/CRITICAL]
Notes: [Any observations or issues]
```

## Document History

| Date | Version | Author | Changes |
|------|---------|--------|---------|
| 2026-03-15 | 1.0 | Rohan Kapoor | Initial draft |

---

*TODO(rohan): Add Grafana dashboard links once new health overview panel is deployed*
*TODO(rohan): Consider automating the manual health check script into a nightly report*

*This document is living documentation. All team members are encouraged to suggest improvements via PR.*
