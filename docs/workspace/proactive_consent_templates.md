# Proactive Consent Messaging Templates (V11-TODO-010)

Source of truth: [`client/runtime/config/proactive_consent_templates.json`](/Users/jay/.openclaw/workspace/client/runtime/config/proactive_consent_templates.json)

## Required Coverage per Source

Each proactive source ships:

1. `consent_template`
2. `renewal_template`
3. required placeholders:
   - `{{scope}}`
   - `{{cadence}}`
   - `{{quiet_hours}}`
   - `{{opt_out_path}}`

## Sources in v1

- `email_digest`
- `calendar_alert`
- `system_health_alert`
- `release_watch`

## Policy Notes

- Consent copy must state scope, cadence, quiet-hours behavior, and opt-out.
- Renewal copy must restate opt-out and current cadence.
- Templates are content-only; delivery transport stays governed by runtime policy.
