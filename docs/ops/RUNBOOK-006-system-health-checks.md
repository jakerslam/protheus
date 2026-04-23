# Runbook 006: System Health Checks

**Owner:** Rohan Kapoor  
**Last Updated:** 2026-03-15  
**Review Cycle:** Monthly  
**Severity:** Operational Procedure

---

## Overview

This runbook defines standard health check procedures for the Infring platform. These checks should be performed regularly during on-call shifts and before any deployment activity.

Wrapper-first command policy:

- Canonical wrappers: `infring`, `infringctl`, `infringd`
- Legacy aliases (`infring`, `infringctl`, `infringd`) are deprecated compatibility-only shims.

## First-Run Failure Decision Tree (Operator Quick Path)

Use this path before escalation when a new machine fails first launch:

1. Command resolution / PATH
   - `. "$HOME/.infring/env.sh" && hash -r 2>/dev/null || true`
   - `infring --help`
2. Setup completion
   - `infring setup --yes --defaults`
   - `infring setup status --json`
3. Gateway and dashboard health
   - `infring gateway status`
   - `infring gateway restart`
   - `curl -fsS http://127.0.0.1:4173/healthz`
4. Stale workspace root/path drift
   - `infringctl doctor --json`
   - Validate `INFRING_WORKSPACE_ROOT` / `INFRING_WORKSPACE_ROOT`
5. Missing full-surface dependencies
   - `curl -fsSL https://raw.githubusercontent.com/protheuslabs/InfRing/main/install.sh | sh -s -- --full --install-node`

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
infringctl health check --all

# Check specific service with verbose output
infringctl health check --service core --verbose

# Check service dependencies
infringctl health check --service core --with-deps
```

**Expected Output:**
- All services: `STATUS: healthy`
- Response time: < 500ms for health endpoint
- Dependency chain: All green

### Database Health

```bash
# Connection pool status
infringctl db status --pool

# Query performance check (last 5 minutes)
infringctl db metrics --slow-queries --since "5m"

# Replication lag (if applicable)
infringctl db replication --lag
```

**Alert Thresholds:**
- Connection pool > 80%: Warning
- Connection pool > 95%: Critical
- Slow queries > 10/min: Investigation required
- Replication lag > 5 seconds: Escalate

### Message Queue Health

```bash
# Queue depth check
infringctl queue status --all

# Consumer lag check
infringctl queue consumers --lag

# Dead letter queue inspection
infringctl queue dlq --count
```

**Normal Operating Ranges:**
- Main queue depth: < 1000 messages
- Consumer lag: < 30 seconds
- DLQ: < 10 messages (investigate if higher)

### Disk and Resource Health

```bash
# Disk usage across nodes
infringctl nodes disk --threshold 80

# Memory utilization
infringctl nodes memory --top 10

# CPU utilization (5 min average)
infringctl nodes cpu --avg 5m
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

## Weekly Clean-Home Recovery Drill (Mandatory)

Run once per week to prevent stale launcher/runtime drift from accumulating unnoticed.

Drill sequence:

1. Capture baseline:
- `infringctl doctor --json > local/state/ops/weekly_drill/doctor_pre.json`
- `infring gateway status > local/state/ops/weekly_drill/gateway_pre.txt`
2. Exercise restart path:
- `infring gateway restart`
3. Verify runtime and health:
- `infring gateway status > local/state/ops/weekly_drill/gateway_post.txt`
- `curl -fsS http://127.0.0.1:4173/healthz > local/state/ops/weekly_drill/healthz_post.txt`
- `infringctl doctor --json > local/state/ops/weekly_drill/doctor_post.json`
4. Validate wrapper persistence/path rewriting:
- `infring --help > local/state/ops/weekly_drill/infring_help.txt`
- `infringctl --help > local/state/ops/weekly_drill/infringctl_help.txt`

Required artifacts:

- `doctor_pre.json`
- `gateway_pre.txt`
- `gateway_post.txt`
- `healthz_post.txt`
- `doctor_post.json`
- `infring_help.txt`
- `infringctl_help.txt`

## Manual Health Check Script

For comprehensive manual verification:

```bash
#!/bin/bash
# save as: health-check-manual.sh

echo "=== Infring Health Check ==="
echo "Timestamp: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo ""

echo "[1/5] Service Health..."
infringctl health check --all --format json | jq '.status'

echo "[2/5] Database Pool..."
infringctl db status --pool | grep -E "(active|idle|max)"

echo "[3/5] Queue Status..."
infringctl queue status --all | grep -E "(queue|depth)"

echo "[4/5] Disk Usage..."
df -h | grep infring

echo "[5/5] Recent Errors..."
infringctl logs --level error --since "5m" | wc -l
echo "   (error count in last 5 min)"

echo ""
echo "=== Health Check Complete ==="
```

## Health Check Failures

### Deterministic First-Run Failure Codes

| failure_code | primary_command | expected_output | immediate_fix |
| --- | --- | --- | --- |
| `command_not_found` | `infring --help` | Help output from canonical wrapper | Reload env and verify direct wrapper path |
| `setup_incomplete` | `infring setup status --json` | `onboarding_receipt.status` is `incomplete` or `completed` | Run `infring setup --yes --defaults` then re-check |
| `gateway_unhealthy` | `infring gateway status` + `/healthz` | Deterministic gateway status and health endpoint output | Run `infring gateway restart`, then re-verify |
| `stale_workspace_root` | `infringctl doctor --json` | Explicit stale root/path findings | Align workspace root env vars and retry |
| `full_surface_dependency_missing` | Full install with `--install-node` | Full command surface installed | Re-run first-launch sequence |

### Service Unhealthy

1. Check service logs: `infringctl logs --service [name] --tail 100`
2. Verify dependencies are healthy
3. Check for recent deployments
4. If isolated to one node, consider draining
5. Escalate to service owner if not resolved in 15 min

### Database Connection Pool Exhausted

1. Check for connection leaks: `infringctl db connections --list`
2. Identify long-running queries: `infringctl db queries --long-running`
3. Consider temporary pool size increase
4. If persistent, may indicate application issue

### Queue Backlog

1. Check consumer status: `infringctl queue consumers --status`
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
