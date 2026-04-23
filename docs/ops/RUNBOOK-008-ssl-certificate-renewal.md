# Runbook 008: SSL Certificate Renewal

**Author:** Rohan Kapoor (VP Platform & Operations)  
**Last Updated:** 2026-03-17  
**Version:** 1.0.0  
**Review Cycle:** Quarterly  
**Priority:** P2 (High - prevents service disruption)

---

## Purpose

This runbook documents the standardized procedure for SSL/TLS certificate renewal across all Infring infrastructure. Timely renewal prevents certificate expiration incidents that could result in service unavailability and security warnings for end users.

## Scope

Applies to all SSL certificates managed by the Platform Operations team:
- Production load balancer certificates (AWS ACM)
- Internal service mesh certificates (mTLS)
- Development and staging environment certificates
- Third-party integration endpoints

## Prerequisites

- Access to AWS Certificate Manager (ACM) console or CLI
- Access to certificate monitoring dashboards
- Notification channels configured (Slack #ssl-alerts)
- Backup certificates prepared for emergency rotation

## Monitoring & Alerting

Certificate expiration is monitored via:
- **Datadog SSL monitor:** Checks daily for certificates expiring within 30 days
- **PagerDuty alert:** Triggered at 14 days, 7 days, and 1 day before expiration
- **Slack notifications:** Posted to #platform-operations for all renewals

## Renewal Procedure

### Step 1: Pre-Renewal Verification (T-7 days)

```bash
# Check current certificate status
aws acm describe-certificate \
  --certificate-arn arn:aws:acm:region:account-id:certificate/cert-id \
  --query 'Certificate.{DomainName:DomainName,Status:Status,NotAfter:NotAfter}'

# Verify domain validation status
aws acm describe-certificate \
  --certificate-arn arn:aws:acm:region:account-id:certificate/cert-id \
  --query 'Certificate.DomainValidationOptions[].{Domain:DomainName,Status:ValidationStatus}'
```

**Expected Result:** Status should be `ISSUED`, expiration date confirmed.

### Step 2: Request New Certificate (T-5 days)

```bash
# Request new certificate with same SANs
aws acm request-certificate \
  --domain-name api.infring.io \
  --subject-alternative-names api.infring.io *.infring.io \
  --validation-method DNS \
  --idempotency-token renewal-$(date +%Y%m%d)
```

**Note:** Save the new certificate ARN for later steps.

### Step 3: Validate Domain Ownership (T-4 days)

1. Retrieve validation records from ACM console
2. Add DNS validation records to Route 53:
   - Type: CNAME
   - Name: _validation.infring.io
   - Value: _validation.acm-validations.aws
3. Wait for validation status to change to `SUCCESS` (typically 5-30 minutes)

### Step 4: Update Load Balancer (T-2 days)

```bash
# Get current listener configuration
aws elbv2 describe-listeners \
  --load-balancer-arn arn:aws:elasticloadbalancing:region:account-id:loadbalancer/app/prod-alb/name \
  --query 'Listeners[?Port==`443`].{ListenerArn:ListenerArn,Certificates:Certificates}'

# Update certificate on HTTPS listener
aws elbv2 modify-listener \
  --listener-arn arn:aws:elasticloadbalancing:region:account-id:listener/app/prod-alb/name/uuid \
  --certificates CertificateArn=arn:aws:acm:region:account-id:certificate/new-cert-id
```

### Step 5: Post-Renewal Verification (T-0, after deployment)

```bash
# Verify new certificate is served
echo | openssl s_client -connect api.infring.io:443 -servername api.infring.io 2>/dev/null | \
  openssl x509 -noout -dates -subject

# Check certificate chain completeness
echo | openssl s_client -connect api.infring.io:443 -servername api.infring.io 2>/dev/null | \
  openssl x509 -noout -chain

# Verify no SSL errors in application logs
kubectl logs -l app=ingress-controller --tail=100 | grep -i "ssl\|certificate\|tls"
```

**Expected Result:**
- `notAfter` date should be ~395 days in the future
- Certificate chain should be complete
- No SSL-related errors in logs

### Step 6: Cleanup (T+1 day)

```bash
# Remove old certificate from ACM (after 24-hour grace period)
aws acm delete-certificate \
  --certificate-arn arn:aws:acm:region:account-id:certificate/old-cert-id

# Update documentation with new expiration date
```

## Rollback Procedure

If issues are detected post-renewal:

```bash
# Revert to previous certificate
aws elbv2 modify-listener \
  --listener-arn arn:aws:elasticloadbalancing:region:account-id:listener/app/prod-alb/name/uuid \
  --certificates CertificateArn=arn:aws:acm:region:account-id:certificate/old-cert-id

# Verify rollback
curl -v https://api.infring.io/health 2>&1 | grep -i "expire date"
```

## Troubleshooting

### Issue: Domain validation fails

**Symptoms:** Certificate status stuck in `PENDING_VALIDATION`

**Resolution:**
1. Verify DNS records are correctly added to Route 53
2. Check for conflicting CNAME records
3. Ensure validation records haven't expired (72-hour TTL)
4. Re-request certificate with new idempotency token if needed

### Issue: Load balancer health checks fail after renewal

**Symptoms:** 502/503 errors, targets marked unhealthy

**Resolution:**
1. Verify certificate chain is complete (intermediate certificates included)
2. Check security group allows traffic on port 443
3. Review target group health check configuration
4. Consider temporary rollback if service impact is severe

## Automation Opportunities

- **Certificate auto-renewal:** AWS ACM with DNS validation enables automatic renewal
- **Infrastructure as Code:** Terraform manages certificate ARNs in listener configs
- **Monitoring integration:** Datadog synthetic tests verify SSL endpoints daily

## References

- [AWS ACM Documentation](https://docs.aws.amazon.com/acm/)
- [Route 53 DNS Validation](https://docs.aws.amazon.com/acm/latest/userguide/dns-validation.html)
- [Internal Wiki: SSL Management](https://wiki.internal/infring/ssl-management)

## Change Log

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0.0 | 2026-03-17 | Rohan Kapoor | Initial version - consolidated tribal knowledge into standardized procedure |

---

*For questions or updates to this runbook, contact the Platform Operations team via Slack #platform-operations.*
