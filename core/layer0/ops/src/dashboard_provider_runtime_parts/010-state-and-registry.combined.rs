// Split from 010-state-and-registry.combined.rs into focused include parts for maintainability.
include!("010-state-and-registry.combined_parts/010-prelude-and-shared.rs");
include!("010-state-and-registry.combined_parts/020-provider-registry-rel-to-command-exists.rs");
include!("010-state-and-registry.combined_parts/030-local-provider-reachable-to-model-profiles-for-provider.rs");
include!("010-state-and-registry.combined_parts/040-parse-billion-hint-to-profile-tags-are-general-only.rs");
include!("010-state-and-registry.combined_parts/050-enrich-single-model-profile-to-masked-prefix.rs");
include!("010-state-and-registry.combined_parts/060-masked-last4-to-content-from-message-rows.rs");
