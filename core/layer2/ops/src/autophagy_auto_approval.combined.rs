// Split from autophagy_auto_approval.combined.rs into focused include parts for maintainability.
include!("autophagy_auto_approval.combined_parts/010-prelude-and-shared.rs");
include!("autophagy_auto_approval.combined_parts/020-default-policy-path-to-stable-proposal-id.rs");
include!("autophagy_auto_approval.combined_parts/030-load-policy-to-evaluate-proposal.rs");
include!("autophagy_auto_approval.combined_parts/040-remove-entry-to-evaluate-command.rs");
include!("autophagy_auto_approval.combined_parts/050-rollback-from-state-to-cli-error.rs");
include!("autophagy_auto_approval.combined_parts/060-run-to-mod-tests.rs");
