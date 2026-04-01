// AUTO-SPLIT: this file is composed from smaller parts to enforce <=1000 line policy.
include!("runtime_system_contracts_parts/010-runtime-system-contract-profile.rs");
include!("runtime_system_contracts_parts/020-contract-id-catalog.rs");
include!("runtime_system_contracts_parts/030-inferred-family-for.rs");

// Strategic registry audit reads this root file directly for contract IDs.
// Keep queued/in-progress IDs mirrored here even when primary catalogs are split into parts.
#[allow(dead_code)]
const STRATEGIC_CONTRACT_AUDIT_COVERAGE_IDS: &[&str] = &[
    "V10-PERF-001.1",
    "V10-PERF-001.2",
    "V10-PERF-001.3",
    "V10-PERF-001.4",
    "V10-PERF-001.5",
    "V10-PERF-001.6",
    "V6-DASHBOARD-007.1",
    "V6-DASHBOARD-007.2",
    "V6-DASHBOARD-007.3",
    "V6-DASHBOARD-007.4",
    "V6-DASHBOARD-007.5",
    "V6-DASHBOARD-007.6",
    "V6-DASHBOARD-007.7",
    "V6-DASHBOARD-007.8",
    "V6-DASHBOARD-008.1",
    "V6-DASHBOARD-008.2",
    "V6-DASHBOARD-008.3",
    "V6-DASHBOARD-008.4",
    "V6-DASHBOARD-009.1",
    "V6-DASHBOARD-009.2",
    "V6-INFRING-GAP-001.1",
    "V6-INFRING-GAP-001.2",
    "V6-INFRING-GAP-001.3",
    "V6-INFRING-GAP-001.4",
    "V6-INFRING-GAP-001.5",
];
