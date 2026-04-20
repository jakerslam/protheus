// Split from agency_plane.rs into focused include parts for maintainability.
include!("agency_plane_parts/010-prelude-and-shared.rs");
include!("agency_plane_parts/020-state-env-to-validate-contract.rs");
include!("agency_plane_parts/030-create-shadow.rs");
include!("agency_plane_parts/040-topology.rs");
include!("agency_plane_parts/050-orchestrate.rs");
include!("agency_plane_parts/060-workflow-bind.rs");
include!("agency_plane_parts/070-run-to-mod-tests.rs");
