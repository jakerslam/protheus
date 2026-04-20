// Split from stock_market_collector_kernel.combined.rs into focused include parts for maintainability.
include!("stock_market_collector_kernel.combined_parts/010-prelude-and-shared.rs");
include!("stock_market_collector_kernel.combined_parts/020-usage-to-load-cache-items.rs");
include!("stock_market_collector_kernel.combined_parts/030-today-utc-to-format-signed-2.rs");
include!("stock_market_collector_kernel.combined_parts/040-map-quotes-to-command-build-fetch-plan.rs");
include!("stock_market_collector_kernel.combined_parts/050-finalize-success-to-command-finalize-run.rs");
include!("stock_market_collector_kernel.combined_parts/060-command-collect-to-mod-stock-market-collector-kernel-tests.rs");
