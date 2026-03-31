# Database Failover Runbook

**Author:** Rohan Kapoor  
**Last Updated:** March 31, 2026  
**Review Cycle:** Quarterly  
**Severity:** P1 - Critical

---

## Purpose

This runbook provides step-by-step procedures for executing a manual database failover in the Protheus production environment. It covers both planned maintenance failovers and emergency unplanned failovers.

## Prerequisites

- [ ] Access to AWS Console (production read-only role)
- [ ] Slack notifications channel: `#ops-alerts`
- [ ] PagerDuty incident created (if unplanned)
- [ ] Secondary DBA on standby for verification

## Impact Assessment

| Component | Downtime | Recovery Time |
|-----------|----------|---------------|
| Trading API | 30-60s | Automatic |
| Reporting | 2-3 min | Manual cache clear |
| WebSocket feeds | 15-30s | Automatic |

## Procedure: Planned Failover

### Step 1: Pre-Failover Checklist (5 min)

```bash
# Verify replication lag
./scripts/ops/check_replication_lag.sh --env=prod --threshold=100ms

# Confirm backup completion
aws rds describe-db-snapshots --db-instance-identifier protheus-primary
```

### Step 2: Announce Maintenance Window

Post in `#trading-ops`:
```
🔧 Scheduled DB failover starting now. Expected brief API latency (30-60s). 
Incident: PD-2026-XXXX
```

### Step 3: Execute Failover

```bash
# Via AWS CLI
aws rds failover-db-cluster \
  --db-cluster-identifier protheus-prod-cluster \
  --target-db-instance-identifier protheus-replica-01
```

### Step 4: Verification (3 min)

```bash
# Check new primary status
./scripts/ops/verify_db_primary.sh

# Run connectivity test
./scripts/ops/db_health_check.sh --critical
```

### Step 5: Post-Failover

- [ ] Update DNS if needed (rare with RDS)
- [ ] Clear application connection pools: `POST /admin/reset-pools`
- [ ] Monitor error rates for 10 minutes
- [ ] Update status page if maintenance window

---

## Procedure: Emergency Unplanned Failover

### Immediate Actions

1. **Create PagerDuty incident** (if not auto-generated)
2. **Notify on-call DBA** via PagerDuty escalation
3. **Post in `#war-room`** with initial assessment

### Rapid Assessment

```bash
# Determine if failover is appropriate
./scripts/ops/assess_db_health.sh --quick
```

Execute failover only if:
- Primary is unresponsive for >60s
- Replication lag >10s and growing
- Disk I/O stalled (check CloudWatch)

### Post-Failover Recovery

1. **DO NOT** restart old primary immediately — assess logs first
2. Check for split-brain scenarios
3. Document root cause in incident timeline

---

## Rollback Procedure

If failover causes issues:

1. Halt all trading (emergency kill switch — see risk/kill_switches.py)
2. Assess data consistency between old/new primary
3. Coordinate with Engineering Lead before reverse failover
4. **Never reverse failover during market hours without explicit approval**

---

## Troubleshooting

### Issue: Applications not reconnecting

**Symptom:** Connection pool exhaustion, 5xx errors  
**Fix:** Force connection pool reset

```bash
curl -X POST https://api.protheus.trade/admin/reset-pools \
  -H "Authorization: Bearer $ADMIN_TOKEN"
```

### Issue: Replication lag post-failover

**Symptom:** Read replica lag >5s in CloudWatch  
**Action:** Monitor only — do not take action unless >30s sustained

### Issue: DNS not resolving to new primary

**Symptom:** Applications still hitting old IP  
**Fix:** Verify RDS endpoint hasn't changed; if using custom DNS, update Route53

---

## Related Resources

- [AWS RDS Failover Documentation](https://docs.aws.amazon.com/AmazonRDS/latest/AuroraUserGuide/Aurora.Managing.Failure.html)
- [Protheus Architecture Diagram: DB Layer](./architecture/db-layer-2026.png)
- [Connection Pool Configuration](../config/database.yml.example)
- [Post-Failover Verification Checklist](./verification-checklist.md)

---

## Revision History

| Date | Author | Changes |
|------|--------|---------|
| 2026-03-31 | Rohan Kapoor | Initial runbook created, consolidated from wiki |
| 2026-03-15 | Sarah Chen | Added rollback procedure clarification |

---

**Questions?** Contact #infrastructure or on-call DBA.
