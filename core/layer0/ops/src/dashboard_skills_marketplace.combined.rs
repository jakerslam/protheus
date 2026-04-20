// Split from dashboard_skills_marketplace.combined.rs into focused include parts for maintainability.
include!("dashboard_skills_marketplace.combined_parts/010-prelude-and-shared.rs");
include!("dashboard_skills_marketplace.combined_parts/020-core-skills-registry-rel-to-default-tags.rs");
include!("dashboard_skills_marketplace.combined_parts/030-normalize-skill-row-to-list-skills-payload.rs");
include!("dashboard_skills_marketplace.combined_parts/040-mcp-servers-payload-to-detail-code-payload.rs");
include!("dashboard_skills_marketplace.combined_parts/050-install-payload-to-handle.rs");
include!("dashboard_skills_marketplace.combined_parts/060-mod-tests.rs");
