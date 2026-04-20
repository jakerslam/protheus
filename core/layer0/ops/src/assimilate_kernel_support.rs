// Split from assimilate_kernel_support.rs into focused include parts for maintainability.
include!("assimilate_kernel_support_parts/010-prelude-and-shared.rs");
include!("assimilate_kernel_support_parts/020-default-showcase-duration-ms-to-normalize-target.rs");
include!("assimilate_kernel_support_parts/030-parse-args-to-should-skip-scan-path.rs");
include!("assimilate_kernel_support_parts/040-repo-root-from-env-or-cwd-to-parse-go-mod-dependency-hints.rs");
include!("assimilate_kernel_support_parts/050-parse-pom-dependency-hints-to-normalize-dependency-token.rs");
include!("assimilate_kernel_support_parts/060-parse-manifest-inventory-to-parse-api-surface.rs");
include!("assimilate_kernel_support_parts/070-parse-structure-surface-to-framework-targets-from-hints.rs");
include!("assimilate_kernel_support_parts/080-framework-targets-from-surfaces-to-recon-index-for-target.rs");
include!("assimilate_kernel_support_parts/090-canonical-assimilation-plan.rs");
include!("assimilate_kernel_support_parts/100-parse-last-json-object-to-bar.rs");
include!("assimilate_kernel_support_parts/110-mod-tests.rs");
