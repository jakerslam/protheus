# Enterprise Support Envelope (Template)

Status: Draft template for legal/executive completion.

This document defines the support surface that can be published once legal and executive approvals are complete (`HMAN-028`).

## Scope

- Product: Infring core + client runtime
- Environment: self-hosted Kubernetes and managed container deployments
- Support channel owner: enterprise support desk

## Proposed SLA Tiers (Draft)

| Tier | Initial Response Target | Coverage Window | Escalation Path |
|---|---|---|---|
| Essential | 8 business hours | 5x8 | ticket + email |
| Business | 4 hours | 24x5 | ticket + email + on-call handoff |
| Critical | 1 hour | 24x7 | pager + incident bridge |

## Severity Model

- `SEV-1`: production outage or security incident affecting critical workload
- `SEV-2`: major degradation with partial workaround
- `SEV-3`: non-critical defects or operational questions
- `SEV-4`: feature requests and advisory guidance

## Support Deliverables

- Incident triage and workaround guidance
- Deterministic evidence bundle support (receipts, logs, provenance)
- Upgrade and rollback assistance for supported versions
- Security response coordination under the disclosure policy

## Legal Completion Checklist (Human-Only)

- Approve indemnification boundaries
- Approve liability caps and exclusions
- Approve governed support contact details
- Publish signed customer-facing terms

Until this checklist is complete, this file is informational only and is not a contractual promise.
