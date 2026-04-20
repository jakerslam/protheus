// Split from dashboard_terminal_broker.rs into focused include parts for maintainability.
include!("dashboard_terminal_broker_parts/010-prelude-and-shared.rs");
include!("dashboard_terminal_broker_parts/020-terminal-state-rel-to-tracking-enabled.rs");
include!("dashboard_terminal_broker_parts/030-summary-enabled-to-maybe-track-command.rs");
include!("dashboard_terminal_broker_parts/040-build-tool-summary-to-resolve-operator-command.rs");
include!("dashboard_terminal_broker_parts/050-sessions-payload-to-close-session.rs");
include!("dashboard_terminal_broker_parts/060-exec-command.rs");
include!("dashboard_terminal_broker_parts/070-http-to-mod-tests.rs");
