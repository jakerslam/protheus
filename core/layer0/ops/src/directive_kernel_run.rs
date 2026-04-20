// Split from directive_kernel_run.rs into focused include parts for maintainability.
include!("directive_kernel_run_parts/010-prelude-and-shared.rs");
include!("directive_kernel_run_parts/020-decode-base64-text-to-decode-payload-or-emit.rs");
include!("directive_kernel_run_parts/030-run.rs");
