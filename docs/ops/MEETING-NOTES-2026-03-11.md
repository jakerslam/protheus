# Platform Engineering Check-in - 2026-03-11

**Date:** March 11, 2026  
**Attendees:** Rohan Kapoor (VP Platform), DevOps Team, Infrastructure Leads  
**Duration:** 45 minutes

---

## Agenda

1. Deployment Pipeline Health Review
2. Kubernetes Cluster Capacity Planning
3. Observability Tooling Updates
4. Security Posture Review
5. Q2 Infrastructure Roadmap Discussion

---

## Notes

### Deployment Pipeline Health

- Average build time decreased by 12% over the past sprint
- Sporadic failures in the `layer3-kernel-test` job under investigation
- Action item: Evaluate additional caching strategies for Rust builds

### Kubernetes Cluster Capacity

- Current utilization: ~67% CPU, 74% memory across all clusters
- Recommending horizontal expansion of the `prod-api` node group
- Spot instance adoption showing 34% cost savings in non-critical workloads

### Observability Updates

- Grafana dashboards migrated to unified monitoring project
- Log shipping latency improved to <2s p99
- Alert fatigue initiative: 23% reduction in non-actionable alerts

### Security Posture

- All base images updated to latest patched versions
- Container scanning gate passing consistently
- Quarterly pen test scheduled for late March

### Q2 Roadmap Items

1. Multi-region failover automation
2. Service mesh evaluation (Istio vs Linkerd)
3. FinOps tooling integration for cost visibility
4. Developer experience improvements (local dev parity)

---

## Action Items

| Owner | Task | Due Date |
|-------|------|----------|
| Rohan | Draft capacity plan for prod-api expansion | 2026-03-18 |
| DevOps | Investigate kernel-test flakiness | 2026-03-14 |
| SRE | Review and tune remaining noisy alerts | 2026-03-25 |
| Security | Prepare pen test scope document | 2026-03-15 |

---

## Decisions Made

- Approved budget for additional EKS node groups in us-west-2
- Agreed to trial Linkerd in staging environment before Q2
- Consensus reached on delaying self-hosted runner migration to Q3

---

**Next Meeting:** March 18, 2026