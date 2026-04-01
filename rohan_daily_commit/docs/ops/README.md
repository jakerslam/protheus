# Protheus Operations Documentation

> **Classification:** Internal  
> **Last Updated:** 2026-04-01  
> **Owner:** Platform Operations  

## Overview

This directory contains operational documentation for the Protheus platform, including runbooks, deployment procedures, and incident response protocols.

## Document Index

### Incident Response

| Document | Purpose | Last Updated |
|----------|---------|--------------|
| [P1 Escalation Runbook](./runbooks/incident-response-p1-escalation.md) | Critical incident procedures | 2026-03-28 |
| [P2 Incident Response](./runbooks/incident-response-p2.md) | High-priority incident procedures | 2026-03-30 |

### Deployment & Release

| Document | Purpose | Last Updated |
|----------|---------|--------------|
| [Deployment Windows](./deployment-windows.md) | Release governance & schedules | 2026-03-30 |

### Configuration Examples

| Document | Purpose |
|----------|---------|
| [Database Config](../config/examples/database.yaml.example) | Connection pooling & SSL setup |

### Operational Scripts

| Script | Purpose |
|--------|---------|
| Service Health Check](../../scripts/utils/service-health-check.sh) | Non-intrusive platform health checks |

## Contribution Guidelines

When adding new operational documentation:

1. Use the established Markdown template
2. Include last-updated date and owner
3. Tag documents with appropriate classification
4. Update this README with new entries

## Contact

For questions about operational procedures, contact:
- **Platform Ops**: platform-ops@company.com
- **On-call Escalation**: PagerDuty rotation

---

**TODO:** Add runbooks for P2/P3 incident response (lower priority)  
**FIXME:** Incident contact list needs quarterly verification