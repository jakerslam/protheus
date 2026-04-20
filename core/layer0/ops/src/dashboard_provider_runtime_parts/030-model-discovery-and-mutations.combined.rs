// Split from 030-model-discovery-and-mutations.combined.rs into focused include parts for maintainability.
include!("030-model-discovery-and-mutations.combined_parts/010-prelude-and-shared.rs");
include!("030-model-discovery-and-mutations.combined_parts/020-split-model-ref-to-add-custom-model.rs");
include!("030-model-discovery-and-mutations.combined_parts/030-delete-custom-model-to-invoke-chat-live.rs");
include!("030-model-discovery-and-mutations.combined_parts/040-infer-auto-route-request-to-invoke-chat-impl.rs");
include!("030-model-discovery-and-mutations.combined_parts/050-invoke-chat.rs");
include!("030-model-discovery-and-mutations.combined_parts/060-mod-tests.rs");
