// Split from github_repo_collector_kernel.combined.rs into focused include parts for maintainability.
include!("github_repo_collector_kernel.combined_parts/010-prelude-and-shared.rs");
include!("github_repo_collector_kernel.combined_parts/020-usage-to-build-repo-activity-fetch-plan.rs");
include!("github_repo_collector_kernel.combined_parts/030-finalize-repo-activity-to-build-pr-review.rs");
include!("github_repo_collector_kernel.combined_parts/040-collect-repo-activity-to-collect-pr-review.rs");
include!("github_repo_collector_kernel.combined_parts/050-run-to-dispatch.rs");
include!("github_repo_collector_kernel.combined_parts/060-run-to-mod-tests.rs");
