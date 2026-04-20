// Split from strategy_campaign_scheduler_kernel.combined.rs into focused include parts for maintainability.
include!("strategy_campaign_scheduler_kernel.combined_parts/010-prelude-and-shared.rs");
include!("strategy_campaign_scheduler_kernel.combined_parts/020-usage-to-normalize-campaigns.rs");
include!("strategy_campaign_scheduler_kernel.combined_parts/030-campaigns-as-value-to-score-match.rs");
include!("strategy_campaign_scheduler_kernel.combined_parts/040-best-campaign-match-to-open-proposal-type-counts.rs");
include!("strategy_campaign_scheduler_kernel.combined_parts/050-build-campaign-decomposition-plans-to-command.rs");
include!("strategy_campaign_scheduler_kernel.combined_parts/060-run-to-mod-tests.rs");
