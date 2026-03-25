# Runbook: API Rate Limiting Troubleshooting

**Last Updated:** 2026-03-25  
**Author:** Rohan Kapoor  
**Scope:** External API integration failures due to rate limiting

## Overview

This runbook addresses incidents where external API providers return 429 (Too Many Requests) or 503 (Service Unavailable) responses due to rate limit exhaustion. This commonly affects market data providers, exchange APIs, and notification services.

## Symptoms

- Elevated 429/503 responses in gateway logs
- Delayed or missing market data updates
- Queue backlog in the spine router
- Alert: `external_api_rate_limit_approaching` firing

## Initial Assessment

1. Check current rate limit status:
   ```bash
   ./scripts/utils/rate-limit-status.sh
   ```

2. Review recent API call patterns:
   ```bash
   grep "429\|429 Too Many Requests" /app/logs/gateway-*.log | tail -50
   ```

3. Identify affected endpoints:
   ```bash
   ./scripts/utils/api-health-check.sh --detailed
   ```

## Resolution Steps

### Immediate Actions (within 5 minutes)

1. **Enable request throttling** if not already active:
   ```bash
   curl -X POST localhost:8080/admin/throttle/enable \
     -H "Authorization: Bearer $ADMIN_TOKEN"
   ```

2. **Reduce polling frequency** for non-critical feeds:
   ```bash
   # Temporarily increase poll intervals
   ./scripts/utils/adjust-polling-rate.sh --factor 2.0
   ```

3. **Notify on-call team** via PagerDuty if resolution time > 15 minutes expected

### Short-term Mitigation (5-30 minutes)

1. **Review and optimize request batching**:
   - Check for redundant subscription requests
   - Consolidate historical data fetches where possible
   - See docs/api/batching-optimization.md

2. **Activate circuit breakers** for severely limited endpoints:
   ```bash
   ./scripts/utils/circuit-breaker.sh --enable --endpoint $PROVIDER_NAME
   ```

### Long-term Resolution (post-incident)

1. **Analyze request patterns** using:
   ```bash
   ./scripts/utils/analyze-api-usage.sh --time-range 24h
   ```

2. **Update rate limit configurations** in `config/rate-limits.yaml`

3. **Consider implementing**:
   - Token bucket rate limiting (see ENG-891)
   - Request prioritization queues
   - Multi-key rotation strategies

## Prevention

- Monitor `external_api_calls_per_minute` metric
- Set up proactive alerts at 80% of rate limit
- Review API usage monthly with engineering team

## Related Runbooks

- RUNBOOK-002-deployment-procedures.md
- RUNBOOK-006-system-health-checks.md
- docs/api/best-practices.md

## Changelog

- 2026-03-25: Initial documentation (rohan)
- 2026-03-25: Added circuit breaker reference and prevention section
