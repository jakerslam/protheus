// Split from dashboard_model_catalog.combined.rs into focused include parts for maintainability.
include!("dashboard_model_catalog.combined_parts/010-prelude-and-shared.rs");
include!("dashboard_model_catalog.combined_parts/020-session-analytics-tuning-rel-to-scale-to-five.rs");
include!("dashboard_model_catalog.combined_parts/030-registry-rows.rs");
include!("dashboard_model_catalog.combined_parts/040-catalog-payload-to-resolve-model-selection.rs");
include!("dashboard_model_catalog.combined_parts/050-route-decision-payload.rs");
include!("dashboard_model_catalog.combined_parts/060-route-score.rs");
include!("dashboard_model_catalog.combined_parts/070-mod-tests.rs");
