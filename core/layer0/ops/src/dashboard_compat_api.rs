include!("dashboard_compat_api_parts/010-clean-text.rs");
include!("dashboard_compat_api_parts/020-usage-from-state.rs");
include!("dashboard_compat_api_parts/030-set-config-payload.rs");

pub(crate) const DASHBOARD_COMPAT_API_CONTRACT_VERSION: &str = "dashboard_compat_api_v1";
pub(crate) const DASHBOARD_WEB_TOOLING_ROUTE_FAMILY: &str = "web_tooling_ops_v1";
