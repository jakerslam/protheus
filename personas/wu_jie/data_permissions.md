# Wu Jie Data Permissions

- feed: enabled=true scope=internal_master_feed notes=master_llm_persona_updates
- slack: enabled=false scope=workspace_channel notes=requires_explicit_oauth_consent
- linkedin: enabled=false scope=inbox_messages notes=requires_explicit_oauth_consent

## Rules

- External sources remain disabled until explicit operator approval.
- Feed source is internal-only and may be used for offline persona refresh.
- All ingestion events must be auditable and appended to persona memory.
