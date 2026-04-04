# Log Rotation Procedures

## Overview

This document outlines standard operating procedures for log rotation across the Protheus infrastructure.

## Purpose

Prevent disk space exhaustion on production nodes while maintaining compliance with data retention policies.

## Default Rotation Schedule

| Log Type | Retention | Rotation Frequency |
|----------|-----------|-------------------|
| Application | 30 days | Daily |
| Access | 90 days | Daily |
| Error | 60 days | Weekly |
| Audit | 1 year | Monthly |

## Manual Rotation

### When to manually rotate

- Disk usage exceeds 85% on `/var/log`
- Preparing for high-traffic events (market open, etc.)
- Debugging requires fresh log files

### Commands

```bash
# Check current disk usage
df -h /var/log

# Force rotation for specific service
sudo logrotate -f /etc/logrotate.d/protheus

# Verify rotation completed
ls -la /var/log/protheus/
```

## Compression

All rotated logs are compressed with gzip to optimize storage. Compression occurs immediately after rotation.

## Monitoring

Alert thresholds:
- **Warning:** 75% disk usage
- **Critical:** 90% disk usage

## Troubleshooting

### Rotation not occurring

Check cron job status:
```bash
sudo systemctl status cron
grep logrotate /var/log/syslog
```

### Permission denied errors

Ensure logrotate runs as root or has appropriate permissions:
```bash
sudo chown -R root:root /var/log/protheus/
sudo chmod 755 /var/log/protheus/
```

## Related Documents

- `../scripts/utils/log-rotate.sh` - Automated rotation script
- `incident-response-runbook.md` - Escalation procedures

---

_Last updated: 2026-04-04 by Rohan Kapoor_