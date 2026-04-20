// Split from healthcare_plane.combined.rs into focused include parts for maintainability.
include!("healthcare_plane.combined_parts/010-prelude-and-shared.rs");
include!("healthcare_plane.combined_parts/020-lane-id-to-patient-command.rs");
include!("healthcare_plane.combined_parts/030-phi-audit-command-to-cds-command.rs");
include!("healthcare_plane.combined_parts/040-devices-command-to-alerts-command.rs");
include!("healthcare_plane.combined_parts/050-coordination-command-to-imaging-command.rs");
include!("healthcare_plane.combined_parts/060-emergency-command-to-run.rs");
