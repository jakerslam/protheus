# Incident Response Automation Module

Production-grade Terraform module for automated incident detection, classification, escalation, and remediation on AWS.

**Author:** Rohan Kapoor (VP Platform & Operations)  
**Version:** 1.0.0  
**Last Updated:** 2026-04-01

---

## Overview

This module establishes a comprehensive incident response pipeline that:

1. **Detects** incidents via CloudWatch alarms and EventBridge rules
2. **Classifies** severity automatically (P1-P4) based on alarm metadata
3. **Escalates** through appropriate channels (Slack, PagerDuty, SMS)
4. **Tracks** all incidents in DynamoDB with audit trails
5. **Automates** remediation for known incident patterns
6. **Monitors** the monitoring system to prevent blind spots

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         AWS CloudWatch                          │
│                    (Critical Metric Alarms)                       │
└────────────────────┬────────────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Amazon EventBridge                         │
│                    (Route P1/P2 Incidents)                      │
└────────────────────┬────────────────────────────────────────────┘
                     │
         ┌───────────┴────────────┐
         ▼                      ▼
┌─────────────────┐    ┌──────────────────┐
│   Orchestrator  │    │ Auto-Remediation │
│   Lambda        │    │ Lambda           │
└────────┬────────┘    └────────┬─────────┘
         │                      │
         ▼                      ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Response Actions                           │
│  ┌──────────┬────────────┬──────────────┬──────────────────┐   │
│  │ Slack    │ PagerDuty  │ DynamoDB     │ Runbook Link     │   │
│  │ Notify   │ Escalation │ Tracking     │ Association      │   │
│  └──────────┴────────────┴──────────────┴──────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

## Features

### 🚨 Intelligent Incident Classification
- **P1 Critical:** Customer-impacting, revenue-impacting, security events
- **P2 High:** Service degradation, performance issues
- **P3/P4 Standard:** Monitoring alerts, non-urgent thresholds

### ⚡ Automated Response Workflows
| Severity | Actions |
|----------|---------|
| P1 | Slack + PagerDuty + SMS + Runbook |
| P2 | Slack + PagerDuty + Runbook |
| P3 | Slack + Runbook |
| P4 | Logged only |

### 🔧 Auto-Remediation (Optional)
Eligible incidents (P2/P3) matching known patterns:
- Disk full → Auto-cleanup old logs
- Memory pressure → Restart non-critical services
- Stuck queues → Dead letter queue reprocessing
- Stale connections → Connection pool refresh

### 📊 Compliance & Audit
- All incidents stored in DynamoDB with 90-day TTL
- Full event payloads archived to S3 with encryption
- SOC2/ISO27001 compliant audit trails
- Point-in-time recovery enabled on tracking tables

## Usage

```hcl
module "incident_response" {
  source = "./terraform/modules/incident-response-automation"

  environment = "prod"
  region      = "us-west-2"

  # PagerDuty integration
  pagerduty_service_key = var.pagerduty_key

  # Slack notifications
  slack_webhook_url = var.slack_webhook

  # Escalation contacts
  p1_escalation_contacts = ["+1-555-0100", "rohan@company.com"]
  p2_escalation_contacts = ["oncall@company.com"]
  p3_escalation_contacts = ["team@company.com"]

  # Compliance
  compliance_mode = true
  runbook_s3_bucket = "company-runbooks"

  common_tags = {
    Environment = "production"
    Team        = "platform-operations"
    CostCenter  = "eng-infra"
  }
}
```

## Inputs

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `environment` | string | required | dev/staging/prod |
| `region` | string | `us-west-2` | AWS region |
| `critical_metrics` | list | `["ErrorRate", "Latency"...]` | Metrics triggering P1 |
| `auto_remediation_enabled` | bool | `true` | Enable automated remediation |
| `pagerduty_service_key` | string | required | PD integration key |
| `slack_webhook_url` | string | required | Slack webhook URL |
| `p1_escalation_contacts` | list | required | P1 contacts |
| `p2_escalation_contacts` | list | required | P2 contacts |
| `p3_escalation_contacts` | list | required | P3 contacts |
| `retention_days` | number | `90` | Data retention period |
| `compliance_mode` | bool | `true` | Enable audit logging |

## Outputs

| Name | Description |
|------|-------------|
| `incident_table_arn` | DynamoDB table ARN |
| `orchestrator_lambda_arn` | Orchestrator Lambda ARN |
| `auto_remediation_lambda_arn` | Remediation Lambda ARN |
| `eventbridge_rule_arn` | EventBridge rule ARN |

## Security Considerations

- All secrets stored in AWS Secrets Manager with 7-day recovery window
- DynamoDB tables encrypted at rest with AWS managed keys
- Lambda functions use least-privilege IAM roles
- Audit logs stored in S3 with SSE-S3 encryption
- PII redacted from Slack notifications

## Monitoring

The module includes self-monitoring:
- CloudWatch alarms for Lambda errors
- Throttle detection on DynamoDB tables
- Dead letter queue monitoring

## Cost Estimates

| Component | Monthly Cost |
|-----------|--------------|
| Lambda (1M invocations) | ~$0.20 |
| DynamoDB (on-demand) | ~$5-50 |
| EventBridge | ~$1 |
| Secrets Manager | ~$0.40 |
| **Total** | **~$10-60/month** |

## Roadmap

- [x] Initial release with core incident pipeline
- [ ] ML-based severity prediction
- [ ] Integration with ServiceNow
- [ ] Multi-region failover support
- [ ] Runbook execution automation

## License

MIT - See LICENSE file

---

**Questions?** Contact: rohan.kapoor@company.com  
**Runbook:** `runbooks/incident-response/system-overview.md`