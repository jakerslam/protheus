fn dashboard_troubleshooting_recent_tooling_contract(rows: &[Value]) -> Value {
    include!("020-troubleshooting_recent_tooling_contract_parts/010-counter-initialization-and-row-scan.rs");
    include!("020-troubleshooting_recent_tooling_contract_parts/020-provider-and-readiness-scoring.rs");
    include!("020-troubleshooting_recent_tooling_contract_parts/030-response-gate-blockers-and-escalation.rs");
    include!("020-troubleshooting_recent_tooling_contract_parts/040-retry-class-window-mode-contracts.rs");
    include!("020-troubleshooting_recent_tooling_contract_parts/050-retry-budget-pressure-and-vectors.rs");
    include!("020-troubleshooting_recent_tooling_contract_parts/060-expected-consistency-guard-cluster-a.rs");
    include!("020-troubleshooting_recent_tooling_contract_parts/070-expected-consistency-guard-cluster-b.rs");
    include!("020-troubleshooting_recent_tooling_contract_parts/080-contract-summary-and-json-payload.rs");
}
