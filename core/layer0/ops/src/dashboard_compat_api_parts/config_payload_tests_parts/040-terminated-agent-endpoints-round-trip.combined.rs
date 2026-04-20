// Split from 040-terminated-agent-endpoints-round-trip.combined.rs into focused include parts for maintainability.
include!("040-terminated-agent-endpoints-round-trip.combined_parts/010-endpoint-env-mutex-to-archive-all-agents-endpoint-rejects-actor-scop.rs");
include!("040-terminated-agent-endpoints-round-trip.combined_parts/020-roster-excludes-zombies-and-archived-profiles-surface-in-terminated.rs");
include!("040-terminated-agent-endpoints-round-trip.combined_parts/030-agent-terminal-routes-through-command-router-to-agent-terminal-block.rs");
include!("040-terminated-agent-endpoints-round-trip.combined_parts/040-agent-command-endpoint-routes-runtime-queries-in-core-to-active-coll.rs");
include!("040-terminated-agent-endpoints-round-trip.combined_parts/050-actor-agent-management-is-scoped-to-descendants-to-direct-slash-tool.rs");
include!("040-terminated-agent-endpoints-round-trip.combined_parts/060-direct-file-read-endpoint-emits-nexus-connection-metadata-to-direct.rs");
include!("040-terminated-agent-endpoints-round-trip.combined_parts/070-direct-web-search-endpoint-fails-closed-when-ingress-route-pair-bloc.rs");
