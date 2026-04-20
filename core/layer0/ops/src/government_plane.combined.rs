// Split from government_plane.combined.rs into focused include parts for maintainability.
include!("government_plane.combined_parts/010-prelude-and-shared.rs");
include!("government_plane.combined_parts/020-lane-id-to-attestation-command.rs");
include!("government_plane.combined_parts/030-classification-command.rs");
include!("government_plane.combined_parts/040-nonrepudiation-command-to-soc-command.rs");
include!("government_plane.combined_parts/050-coop-command-to-proofs-command.rs");
include!("government_plane.combined_parts/060-interoperability-command-to-run.rs");
