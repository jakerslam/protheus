// Split from dashboard_compat_api_channels.combined.rs into focused include parts for maintainability.
include!("dashboard_compat_api_channels.combined_parts/010-prelude-and-shared.rs");
include!("dashboard_compat_api_channels.combined_parts/020-channel-registry-rel-to-ok-response.rs");
include!("dashboard_compat_api_channels.combined_parts/030-curl-json-request-to-channel-rows.rs");
include!("dashboard_compat_api_channels.combined_parts/040-channel-name-from-path-to-live-probe-whatsapp.rs");
include!("dashboard_compat_api_channels.combined_parts/050-live-probe-gohighlevel-to-live-probe-generic.rs");
include!("dashboard_compat_api_channels.combined_parts/060-live-probe-to-handle.rs");
