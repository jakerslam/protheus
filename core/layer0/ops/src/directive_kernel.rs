include!("directive_kernel_parts/010-state-root.rs");
include!("directive_kernel_parts/020-payload-contains-authorization-bearer.rs");
include!("directive_kernel_parts/025-web-tooling-vault-helpers.rs");
include!("directive_kernel_parts/030-directive-vault-hash.rs");
include!("directive_kernel_parts/040-run.rs");
include!("directive_kernel_parts/050-env-guard.rs");
include!("directive_kernel_parts/060-placeholder.rs");

pub fn web_tooling_policy_status(root: &std::path::Path) -> serde_json::Value {
    web_tooling_policy_snapshot(root)
}

pub fn web_tooling_policy_missing(root: &std::path::Path) -> Vec<String> {
    web_tooling_policy_missing_codes(root)
}

pub fn web_tooling_policy_is_ready(root: &std::path::Path) -> bool {
    web_tooling_policy_ready(root)
}
