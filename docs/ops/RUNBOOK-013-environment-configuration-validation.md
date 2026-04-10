# Runbook 013: Environment Configuration Validation

**Owner:** Rohan Kapoor  
**Last Updated:** 2026-04-10  
**Review Cycle:** Quarterly  
**Applies To:** All deployment environments (dev, staging, production)

---

## Overview

This runbook defines procedures for validating environment configuration before and after deployments. Proper configuration validation prevents deployment failures, service misconfigurations, and security policy violations.

## Pre-Deployment Configuration Checklist

### 1. Environment File Verification

Before any deployment, verify that environment files match expected schemas:

```bash
# Validate .env file syntax
protheusctl config validate --env-file=.env.production --schema=config/schemas/env-schema.json

# Check for required variables
protheusctl config check-required --env-file=.env.production --required-list=config/required-vars.txt
```

### 2. Secret Reference Validation

Ensure all secret references point to valid vault paths:

```bash
# List all secret references
protheusctl config scan-secrets --env-file=.env.production

# Verify vault connectivity and secret existence
protheusctl vault verify --paths-from=secret-refs.txt
```

**Required Secret Patterns:**
- Database credentials: `vault://database/[env]/credentials`
- API keys: `vault://services/[service-name]/api-keys`
- TLS certificates: `vault://tls/[domain]/certificate`

### 3. Cross-Environment Variable Consistency

Certain variables must maintain consistency across environments:

| Variable | Consistency Rule | Rationale |
|----------|-----------------|-----------|
| `API_VERSION` | Same across dev/staging/prod | Prevents client compatibility issues |
| `LOG_FORMAT` | Same across all envs | Ensures log aggregation works uniformly |
| `CIRCUIT_BREAKER_THRESHOLD` | Prod ≤ Staging | Production should be more conservative |

### 4. Feature Flag Verification

Before production deployment, verify feature flags are in expected states:

```bash
# Export current feature flag configuration
protheusctl feature-flags export --output=feature-flags-current.json

# Compare against expected baseline
diff feature-flags-current.json config/baselines/feature-flags-production.json
```

## Validation Automation

### CI/CD Integration

Configuration validation runs automatically in the deployment pipeline:

```yaml
# .github/workflows/config-validation.yml (excerpt)
validate-config:
  runs-on: ubuntu-latest
  steps:
    - name: Checkout code
      uses: actions/checkout@v4
    
    - name: Validate environment configuration
      run: |
        protheusctl config validate --strict
        protheusctl config check-secrets --fail-on-missing
    
    - name: Verify no hardcoded secrets
      run: |
        protheusctl security scan --detect-hardcoded-secrets --fail-on-detection
```

### Pre-Commit Hooks

Developers should enable local validation:

```bash
# Install pre-commit hook for config validation
cat > .git/hooks/pre-commit << 'EOF'
#!/bin/bash
# Validate environment files before commit
if git diff --cached --name-only | grep -q '\.env'; then
    echo "Environment files changed - running validation..."
    protheusctl config validate --env-files-changed
fi
EOF
chmod +x .git/hooks/pre-commit
```

## Common Validation Failures

### Issue: Missing Required Variables

**Symptom:** Deployment fails with `RequiredConfigError`

**Resolution:**
1. Identify missing variable from error message
2. Check if variable exists in vault or secrets manager
3. Add to environment file or deployment secret injection
4. Re-validate before retrying deployment

### Issue: Secret Path Not Found

**Symptom:** `VaultSecretNotFound` during startup

**Resolution:**
1. Verify secret exists in vault: `vault kv get <path>`
2. Check path spelling and environment (dev vs prod paths differ)
3. Ensure service account has read permissions to secret path
4. Contact security team if secret needs to be created

### Issue: Invalid Configuration Values

**Symptom:** Service starts but behaves unexpectedly

**Resolution:**
1. Check for type mismatches (string vs integer)
2. Verify URL formats include proper schemes (`https://` not `http://` for production)
3. Ensure numeric values fall within valid ranges
4. Review recent changes in version control for typos

## Post-Deployment Configuration Verification

After deployment completes, verify configuration was applied correctly:

```bash
# Check running configuration
protheusctl config export --running > running-config.json

# Compare against expected
diff running-config.json expected-config.json

# Verify critical settings
protheusctl config verify-critical \
  --check CLEARANCE=3 \
  --check CIRCUIT_BREAKER_THRESHOLD=0.5 \
  --check WS_DEBUG=0
```

## Environment-Specific Considerations

### Development Environment

- Validation is advisory (warnings logged but don't block)
- Mock secrets acceptable for external service testing
- Debug flags may be enabled (`WS_DEBUG=1`)

### Staging Environment

- Validation is enforced (errors block deployment)
- Must use real secrets (no mock values)
- Should mirror production configuration closely

### Production Environment

- Strictest validation rules apply
- All secrets must exist in production vault
- Circuit breaker thresholds more conservative than staging
- Debug logging disabled by default

## Documentation References

- Environment variable reference: [../../.env.example](../../.env.example)
- Security policy: [../../SECURITY.md](../../SECURITY.md)
- Deployment procedures: [RUNBOOK-002-deployment-procedures.md](./RUNBOOK-002-deployment-procedures.md)

## Document History

| Date | Version | Author | Changes |
|------|---------|--------|---------|
| 2026-04-10 | 1.0 | Rohan Kapoor | Initial draft covering pre-deployment validation procedures |

---

*This runbook is living documentation. Suggested improvements should be submitted as PRs with ops-team review.*

<!-- NOTE(rohan): Created this runbook based on common issues seen during Q1 deployments.
     Future enhancements could include:
     - Automated drift detection for running configs
     - Integration with config management DB (if we migrate from env files)
     - Validation performance benchmarks for large configs
     See ENG-512 for tracking these enhancements.
-->
