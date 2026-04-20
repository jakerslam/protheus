// Split from dashboard_tool_turn_loop.rs into focused include parts for maintainability.
include!("dashboard_tool_turn_loop_parts/010-prelude-and-shared.rs");
include!("dashboard_tool_turn_loop_parts/020-terminal-permission-policy-rel-to-ensure-sub-nexus-registered.rs");
include!("dashboard_tool_turn_loop_parts/030-authorize-client-ingress-route-with-nexus-inner-to-input-confirmed.rs");
include!("dashboard_tool_turn_loop_parts/040-command-signature-to-rewrite-object-key.rs");
include!("dashboard_tool_turn_loop_parts/050-output-tokens-estimate.rs");
include!("dashboard_tool_turn_loop_parts/060-mod-tests.rs");
