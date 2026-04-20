// Split from 080-tests.tail.rs into focused include parts for maintainability.
include!("080-tests.tail_parts/010-bing-rss-content-filter-tests.rs");
include!("080-tests.tail_parts/020-search-early-validation-tests.rs");
include!("080-tests.tail_parts/030-search-shape-and-source-tests.rs");
include!("080-tests.tail_parts/040-meta-query-override-tests.rs");
include!("080-tests.tail_parts/050-domain-query-shape-tests.rs");
include!("080-tests.tail_parts/060-fetch-request-construction-tests.rs");
include!("080-tests.tail_parts/070-meta-query-response-contract-tests.rs");
include!("080-tests.tail_parts/080-time-filter-conflict-tests.rs");
include!("080-tests.tail_parts/090-provider-fail-closed-retry-tests.rs");
include!("080-tests.tail_parts/100-retry-envelope-helper-tests.rs");
include!("080-tests.tail_parts/110-fetch-retry-runtime-helper-tests.rs");
include!("080-tests.tail_parts/120-loop-detection-query-shape-tests.rs");
