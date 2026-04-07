# Ops Toolkit

Infrastructure automation, monitoring, CI/CD tooling, and release governance for the Protheus platform.

## Structure

- `docs/` - Documentation and runbooks
- `scripts/` - Operational scripts and utilities
- `config/` - Configuration templates and examples

## Getting Started

See `docs/ops/getting-started.md` for initial setup instructions.

## Documentation Quick Reference

| Document | Purpose |
|----------|---------|
| `docs/ops/incident-response-runbook.md` | Incident severity levels and escalation |
| `docs/ops/log-rotation-procedures.md` | Log management and disk space guidelines |
| `config/monitoring.conf.example` | Monitoring stack configuration template |
| `scripts/utils/health-check.sh` | Basic health verification script |
| `scripts/utils/log-rotate.sh` | Automated log rotation utility |
| `scripts/utils/ssl-expiry-check.sh` | SSL certificate expiration monitoring |

## Contributing

All changes should be peer-reviewed before merging to main.

### Pull Request Guidelines

- **Docs changes:** One reviewer from the team
- **Script changes:** Two reviewers + testing evidence
- **Config changes:** SRE approval required

### Documentation Standards

- Use clear, concise language
- Include examples where applicable
- Update the last modified date

See `docs/ops/contributing.md` for detailed guidelines.
