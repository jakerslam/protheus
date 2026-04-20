// Split from 020-provider-http-interop.combined.rs into focused include parts for maintainability.
include!("020-provider-http-interop.combined_parts/010-curl-json-to-models-from-probe-response.rs");
include!("020-provider-http-interop.combined_parts/020-parse-ollama-list-models-to-probe-ollama-runtime-models.rs");
include!("020-provider-http-interop.combined_parts/030-provider-rows-to-providers-payload.rs");
include!("020-provider-http-interop.combined_parts/040-save-provider-key-to-set-provider-url.rs");
include!("020-provider-http-interop.combined_parts/050-provider.rs");
include!("020-provider-http-interop.combined_parts/060-mod-provider-http-interop-tests.rs");
