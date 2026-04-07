# Incident Response Runbook

## Overview

This document outlines the procedures for responding to infrastructure incidents within the Protheus platform.

## Severity Levels

### SEV-1: Critical
- Complete platform outage
- Data loss or corruption
- Security breach

**Response:** Page on-call engineer immediately. War room within 15 minutes.

### SEV-2: High
- Degraded performance (>50% impact)
- Major feature unavailable
- Single component failure with workarounds

**Response:** Alert on-call engineer. Response within 30 minutes.

### SEV-3: Medium
- Minor feature degradation
- Non-critical alerts only

**Response:** Create ticket, address during business hours.

## Escalation Path

1. **Primary:** On-call SRE (PagerDuty)
2. **Secondary:** Infrastructure Team Lead
3. **Executive:** VP Engineering (SEV-1 only)

## Communication Channels

- **War Room:** `#incidents` Slack channel
- **Public Updates:** Status page (status.protheus.io)
- **Stakeholder Comms:** Incident Commander discretion

## Post-Incident

All SEV-1 and SEV-2 incidents require a post-mortem within 48 hours.

---

_Last updated: 2026-04-02 by Rohan Kapoor_
