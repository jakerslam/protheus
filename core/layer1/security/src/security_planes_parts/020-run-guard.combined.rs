// Split from 020-run-guard.combined.rs into focused include parts for maintainability.
include!("020-run-guard.combined_parts/010-parse-last-json-line-to-guard-state-logs.rs");
include!("020-run-guard.combined_parts/020-guard.rs");
include!("020-run-guard.combined_parts/030-antisabotagepolicy-to-anti-sabotage-snapshot.rs");
include!("020-run-guard.combined_parts/040-anti-sabotage-verify-to-anti-sabotage-status.rs");
include!("020-run-guard.combined_parts/050-anti-sabotage-shield-to-proposal-status.rs");
include!("020-run-guard.combined_parts/060-constitution-guardian.rs");
include!("020-run-guard.combined_parts/070-remoteemergencyhaltpolicy-to-clean-expired-nonces.rs");
include!("020-run-guard.combined_parts/080-remote-emergency-halt.rs");
include!("020-run-guard.combined_parts/090-soultokenguardpolicy-to-read-jsonl-rows.rs");
include!("020-run-guard.combined_parts/100-soul-token-guard.rs");
