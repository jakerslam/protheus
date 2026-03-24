// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops::runtime_system_contracts (authoritative)
use std::collections::BTreeMap;
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeSystemContractProfile {
    pub id: &'static str,
    pub family: &'static str,
    pub objective: &'static str,
    pub strict_conduit_only: bool,
    pub strict_fail_closed: bool,
}

#[derive(Debug, Clone, Copy)]
struct RuntimeSystemContractFamily {
    ids: &'static [&'static str],
    family: &'static str,
    objective: &'static str,
}

const NEW_ACTIONABLE_IDS: &[&str] = &[
    "V10-ULTIMATE-001.1",
    "V10-ULTIMATE-001.2",
    "V10-ULTIMATE-001.3",
    "V10-ULTIMATE-001.4",
    "V10-ULTIMATE-001.5",
    "V10-ULTIMATE-001.6",
    "V8-AUTOMATION-016.1",
    "V8-AUTOMATION-016.2",
    "V8-AUTOMATION-016.3",
    "V8-AUTOMATION-016.4",
    "V8-AUTOMATION-016.5",
    "V8-AUTONOMY-012.1",
    "V8-AUTONOMY-012.2",
    "V8-AUTONOMY-012.3",
    "V8-AUTONOMY-012.4",
    "V8-CLI-001.1",
    "V8-CLI-001.2",
    "V8-CLI-001.3",
    "V8-CLI-001.4",
    "V8-CLI-001.5",
    "V8-CLIENT-010.3",
    "V8-CLIENT-010.4",
    "V8-COMPETE-001.1",
    "V8-COMPETE-001.10",
    "V8-COMPETE-001.2",
    "V8-COMPETE-001.3",
    "V8-COMPETE-001.4",
    "V8-COMPETE-001.5",
    "V8-COMPETE-001.6",
    "V8-COMPETE-001.7",
    "V8-COMPETE-001.8",
    "V8-COMPETE-001.9",
    "V8-EYES-009.1",
    "V8-EYES-009.2",
    "V8-EYES-009.3",
    "V8-EYES-009.4",
    "V8-EYES-010.1",
    "V8-EYES-010.2",
    "V8-EYES-010.3",
    "V8-EYES-010.4",
    "V8-EYES-010.5",
    "V8-EYES-011.1",
    "V8-EYES-011.2",
    "V8-EYES-011.3",
    "V8-EYES-011.4",
    "V8-LEARNING-004.1",
    "V8-LEARNING-004.2",
    "V8-LEARNING-004.3",
    "V8-LEARNING-004.4",
    "V8-LEARNING-005.1",
    "V8-LEARNING-005.2",
    "V8-LEARNING-005.3",
    "V8-LEARNING-005.4",
    "V8-LEARNING-006.1",
    "V8-LEARNING-006.2",
    "V8-LEARNING-006.3",
    "V8-LEARNING-008.1",
    "V8-LEARNING-008.2",
    "V8-LEARNING-008.3",
    "V8-LEARNING-008.4",
    "V8-MEMORY-017.1",
    "V8-MEMORY-017.2",
    "V8-MEMORY-017.3",
    "V8-MEMORY-017.4",
    "V8-MEMORY-018.1",
    "V8-MEMORY-018.2",
    "V8-MEMORY-018.3",
    "V8-MEMORY-018.4",
    "V8-MEMORY-019.1",
    "V8-MEMORY-019.2",
    "V8-MEMORY-019.3",
    "V8-MEMORY-019.4",
    "V8-MEMORY-022.4",
    "V8-MEMORY-022.5",
    "V8-ORGANISM-015.1",
    "V8-ORGANISM-015.2",
    "V8-ORGANISM-015.3",
    "V8-ORGANISM-015.4",
    "V8-ORGANISM-022.1",
    "V8-ORGANISM-022.3",
    "V8-ORGANISM-023.1",
    "V8-ORGANISM-023.2",
    "V8-ORGANISM-023.3",
    "V8-ORGANISM-023.4",
    "V8-PERSONA-015.1",
    "V8-PERSONA-015.2",
    "V8-PERSONA-015.3",
    "V8-PERSONA-015.4",
    "V8-SAFETY-022.2",
    "V8-SECURITY-020.1",
    "V8-SECURITY-020.2",
    "V8-SECURITY-020.3",
    "V8-SECURITY-024.1",
    "V8-SECURITY-024.2",
    "V8-SECURITY-024.3",
    "V8-SKILLS-011.1",
    "V8-SKILLS-011.2",
    "V8-SKILLS-011.3",
    "V8-SKILLS-011.4",
    "V8-SKILLS-012.1",
    "V8-SKILLS-012.2",
    "V8-SKILLS-012.3",
    "V8-SKILLS-012.4",
    "V8-SKILLS-013.1",
    "V8-SKILLS-013.2",
    "V8-SKILLS-013.3",
    "V8-SKILLS-013.4",
    "V8-SKILLS-013.5",
    "V8-SKILLS-014.1",
    "V8-SKILLS-014.2",
    "V8-SKILLS-014.3",
    "V8-SKILLS-014.4",
    "V8-SWARM-009.1",
    "V8-SWARM-009.2",
    "V8-SWARM-009.3",
    "V8-SWARM-009.4",
    "V8-SWARM-010.1",
    "V8-SWARM-010.2",
    "V8-SWARM-010.3",
    "V8-SWARM-010.4",
    "V8-SWARM-011.1",
    "V8-SWARM-011.2",
    "V8-SWARM-011.3",
    "V8-SWARM-011.4",
    "V8-SWARM-012.1",
    "V8-SWARM-012.10",
    "V8-SWARM-012.2",
    "V8-SWARM-012.3",
    "V8-SWARM-012.4",
    "V8-SWARM-012.5",
    "V8-SWARM-012.6",
    "V8-SWARM-012.7",
    "V8-SWARM-012.8",
    "V8-SWARM-012.9",
    "V9-CLIENT-020.1",
    "V9-CLIENT-020.2",
    "V9-CLIENT-020.3",
    "V9-CLIENT-020.4",
    "V9-ORGANISM-025.1",
    "V9-ORGANISM-025.2",
    "V9-ORGANISM-025.3",
    "V9-ORGANISM-025.4",
    "V9-TINYMAX-021.1",
    "V9-TINYMAX-021.2",
    "V10-CORE-001.1",
    "V10-CORE-001.2",
    "V10-CORE-001.3",
    "V10-CORE-001.4",
    "V10-CORE-001.5",
    "V10-SWARM-INF-001.1",
    "V10-SWARM-INF-001.2",
    "V10-SWARM-INF-001.3",
    "V10-SWARM-INF-001.4",
    "V10-SWARM-INF-001.5",
    "V10-SWARM-INF-001.6",
    "V10-SWARM-INF-001.7",
    "V10-SWARM-INF-001.8",
    "V10-PERF-001.1",
    "V10-PERF-001.2",
    "V10-PERF-001.3",
    "V10-PERF-001.4",
    "V10-PERF-001.5",
    "V10-PERF-001.6",
    "V6-ADAPTER-001.1",
    "V6-ADAPTER-001.2",
    "V6-ADAPTER-001.3",
    "V6-ADAPTER-001.4",
    "V6-ADAPTER-001.5",
    "V6-ADAPTER-002.1",
    "V6-ADAPTER-002.2",
    "V6-ADAPTER-002.3",
    "V6-ADAPTER-002.4",
    "V6-ADAPTER-002.5",
    "V6-BEAT-OPENFANG-001",
    "V6-BEAT-OPENFANG-002",
    "V6-BEAT-OPENFANG-003",
    "V6-BEAT-OPENFANG-004",
    "V6-BEAT-OPENFANG-005",
    "V6-BEAT-OPENFANG-006",
    "V6-BEAT-OPENFANG-007",
    "V6-BEAT-OPENFANG-008",
    "V6-BEAT-OPENFANG-009",
    "V6-BEAT-OPENFANG-010",
    "V6-BLINDSPOT-001.1",
    "V6-BLINDSPOT-001.10",
    "V6-BLINDSPOT-001.2",
    "V6-BLINDSPOT-001.3",
    "V6-BLINDSPOT-001.4",
    "V6-BLINDSPOT-001.5",
    "V6-BLINDSPOT-001.6",
    "V6-BLINDSPOT-001.7",
    "V6-BLINDSPOT-001.8",
    "V6-BLINDSPOT-001.9",
    "V6-DASHBOARD-001.1",
    "V6-DASHBOARD-001.10",
    "V6-DASHBOARD-001.2",
    "V6-DASHBOARD-001.3",
    "V6-DASHBOARD-001.4",
    "V6-DASHBOARD-001.5",
    "V6-DASHBOARD-001.6",
    "V6-DASHBOARD-001.7",
    "V6-DASHBOARD-001.8",
    "V6-DASHBOARD-001.9",
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
    "V6-COLLAB-002.1",
    "V6-COLLAB-002.2",
    "V6-COLLAB-002.3",
    "V6-OPENCLAW-DETACH-001.1",
    "V6-OPENCLAW-DETACH-001.2",
    "V6-ECONOMY-003.1",
    "V6-ECONOMY-003.2",
    "V6-ECONOMY-003.3",
    "V6-ECONOMY-003.4",
    "V6-ECONOMY-003.5",
    "V6-EXECUTION-001.1",
    "V6-EXECUTION-001.2",
    "V6-EXECUTION-001.3",
    "V6-EXECUTION-001.4",
    "V6-INFERENCE-003.1",
    "V6-INFERENCE-003.2",
    "V6-INFERENCE-003.3",
    "V6-INFERENCE-003.4",
    "V6-INFERENCE-003.5",
    "V6-INFERENCE-004.1",
    "V6-INFERENCE-004.2",
    "V6-INFERENCE-004.3",
    "V6-INFERENCE-004.4",
    "V6-INFERENCE-004.5",
    "V6-LEARNING-009.1",
    "V6-LEARNING-009.2",
    "V6-LEARNING-009.3",
    "V6-LEARNING-009.4",
    "V6-LEARNING-009.5",
    "V6-LEARNING-009.6",
    "V6-LEARNING-010.1",
    "V6-LEARNING-010.2",
    "V6-LEARNING-010.3",
    "V6-LEARNING-010.4",
    "V6-LEARNING-010.5",
    "V6-MEMORY-020.1",
    "V6-MEMORY-020.2",
    "V6-MEMORY-020.3",
    "V6-MEMORY-020.4",
    "V6-MEMORY-020.5",
    "V6-MEMORY-020.6",
    "V6-MEMORY-023.1",
    "V6-MEMORY-023.2",
    "V6-MEMORY-023.3",
    "V6-MEMORY-023.4",
    "V6-MEMORY-023.5",
    "V6-MEMORY-023.6",
    "V6-MEMORY-024.1",
    "V6-MEMORY-024.2",
    "V6-MEMORY-024.3",
    "V6-MEMORY-024.4",
    "V6-MEMORY-024.5",
    "V6-MEMORY-025.1",
    "V6-MEMORY-025.2",
    "V6-MEMORY-025.3",
    "V6-MEMORY-025.4",
    "V6-MEMORY-026.1",
    "V6-MEMORY-026.2",
    "V6-MEMORY-026.3",
    "V6-MEMORY-026.4",
    "V6-MEMORY-026.5",
    "V6-MEMORY-COMPACT-002",
    "V6-MEMORY-DECAY-001.1",
    "V6-MEMORY-DECAY-001.2",
    "V6-MEMORY-DECAY-001.3",
    "V6-MEMORY-DECAY-001.4",
    "V6-MEMORY-DECAY-001.5",
    "V6-MEMORY-DECAY-001.6",
    "V6-SCHEDULER-002.1",
    "V6-SCHEDULER-002.2",
    "V6-SCHEDULER-002.3",
    "V6-SCHEDULER-002.4",
    "V6-SECURITY-021.1",
    "V6-SECURITY-021.2",
    "V6-SECURITY-021.3",
    "V6-SECURITY-021.4",
    "V6-SECURITY-021.5",
    "V6-SECURITY-021.6",
    "V6-SECURITY-021.7",
    "V6-SECURITY-022.1",
    "V6-SECURITY-022.2",
    "V6-SECURITY-022.3",
    "V6-SECURITY-022.4",
    "V6-SECURITY-022.5",
    "V6-SKILLS-002.1",
    "V6-SKILLS-002.2",
    "V6-SKILLS-002.3",
    "V6-SKILLS-002.4",
    "V6-SKILLS-002.5",
    "V6-SKILLS-003.1",
    "V6-SKILLS-003.2",
    "V6-SKILLS-003.3",
    "V6-SKILLS-003.4",
    "V6-SKILLS-003.5",
    "V6-SKILLS-004.1",
    "V6-SKILLS-004.2",
    "V6-SKILLS-004.3",
    "V6-SKILLS-004.4",
    "V6-SKILLS-004.5",
    "V6-WORKFLOW-016.1",
    "V6-WORKFLOW-016.2",
    "V6-WORKFLOW-016.3",
    "V6-WORKFLOW-016.4",
    "V6-WORKFLOW-016.5",
    "V6-WORKFLOW-016.6",
    "V6-WORKFLOW-016.7",
    "V6-WORKFLOW-016.8",
    "V6-WORKFLOW-018.1",
    "V6-WORKFLOW-018.2",
    "V6-WORKFLOW-018.3",
    "V6-WORKFLOW-018.4",
    "V6-WORKFLOW-018.5",
    "V6-WORKFLOW-019.1",
    "V6-WORKFLOW-019.2",
    "V6-WORKFLOW-019.3",
    "V6-WORKFLOW-019.4",
    "V6-WORKFLOW-019.5",
    "V6-WORKFLOW-019.6",
    "V6-WORKFLOW-019.7",
    "V6-WORKFLOW-020.1",
    "V6-WORKFLOW-020.2",
    "V6-WORKFLOW-020.3",
    "V6-WORKFLOW-020.4",
    "V6-WORKFLOW-020.5",
    "V6-WORKFLOW-020.6",
    "V6-WORKFLOW-020.7",
    "V6-WORKFLOW-021.1",
    "V6-WORKFLOW-021.2",
    "V6-WORKFLOW-021.3",
    "V6-WORKFLOW-021.4",
    "V6-WORKFLOW-021.5",
    "V6-WORKFLOW-022.1",
    "V6-WORKFLOW-022.2",
    "V6-WORKFLOW-022.3",
    "V6-WORKFLOW-022.4",
    "V6-WORKFLOW-022.5",
    "V6-WORKFLOW-023.1",
    "V6-WORKFLOW-023.2",
    "V6-WORKFLOW-023.3",
    "V6-WORKFLOW-023.4",
    "V6-WORKFLOW-023.5",
    "V6-WORKFLOW-024.1",
    "V6-WORKFLOW-024.2",
    "V6-WORKFLOW-024.3",
    "V6-WORKFLOW-024.4",
    "V6-WORKFLOW-024.5",
    "V6-WORKFLOW-025.1",
    "V6-WORKFLOW-025.2",
    "V6-WORKFLOW-025.3",
    "V6-WORKFLOW-025.4",
    "V6-WORKFLOW-025.5",
    "V6-WORKFLOW-026.1",
    "V6-WORKFLOW-026.2",
    "V6-WORKFLOW-026.3",
    "V6-WORKFLOW-026.4",
    "V6-WORKFLOW-026.5",
    "V10-PHONE-001.1",
    "V10-PHONE-001.2",
    "V10-PHONE-001.3",
    "V10-PHONE-001.4",
    "V10-PHONE-001.5",
    "V10-ULTIMATE-002.1",
    "V10-ULTIMATE-002.2",
    "V10-ULTIMATE-002.3",
    "V6-APP-023.10",
    "V6-APP-023.11",
    "V6-APP-023.7",
    "V6-APP-023.8",
    "V6-APP-023.9",
    "V6-WORKFLOW-001.1",
    "V6-WORKFLOW-001.10",
    "V6-WORKFLOW-001.11",
    "V6-WORKFLOW-001.12",
    "V6-WORKFLOW-001.2",
    "V6-WORKFLOW-001.3",
    "V6-WORKFLOW-001.4",
    "V6-WORKFLOW-001.5",
    "V6-WORKFLOW-001.6",
    "V6-WORKFLOW-001.7",
    "V6-WORKFLOW-001.8",
    "V6-WORKFLOW-001.9",
    "V6-WORKFLOW-002.1",
    "V6-WORKFLOW-002.2",
    "V6-WORKFLOW-002.3",
    "V6-WORKFLOW-002.4",
    "V6-WORKFLOW-002.5",
    "V6-WORKFLOW-002.6",
    "V6-WORKFLOW-004.1",
    "V6-WORKFLOW-004.10",
    "V6-WORKFLOW-004.2",
    "V6-WORKFLOW-004.3",
    "V6-WORKFLOW-004.4",
    "V6-WORKFLOW-004.5",
    "V6-WORKFLOW-004.6",
    "V6-WORKFLOW-004.7",
    "V6-WORKFLOW-004.8",
    "V6-WORKFLOW-004.9",
    "V6-WORKFLOW-005.1",
    "V6-WORKFLOW-005.2",
    "V6-WORKFLOW-005.3",
    "V6-WORKFLOW-005.4",
    "V6-WORKFLOW-005.5",
    "V6-WORKFLOW-005.6",
    "V6-WORKFLOW-005.7",
    "V6-WORKFLOW-006.1",
    "V6-WORKFLOW-006.2",
    "V6-WORKFLOW-006.3",
    "V6-WORKFLOW-006.4",
    "V6-WORKFLOW-006.5",
    "V6-WORKFLOW-006.6",
    "V6-WORKFLOW-006.7",
    "V6-WORKFLOW-006.8",
];

const ACT_IDS: &[&str] = &[
    "V8-ACT-001.1",
    "V8-ACT-001.2",
    "V8-ACT-001.3",
    "V8-ACT-001.4",
    "V8-ACT-001.5",
];

const COMPANY_IDS: &[&str] = &[
    "V6-COMPANY-002.1",
    "V6-COMPANY-002.2",
    "V6-COMPANY-002.3",
    "V6-COMPANY-002.4",
    "V6-COMPANY-002.5",
    "V6-COMPANY-003.1",
    "V6-COMPANY-003.2",
    "V6-COMPANY-003.3",
    "V6-COMPANY-003.4",
    "V6-COMPANY-003.5",
];

const COMPETITOR_IDS: &[&str] = &[
    "V10-COMPETITOR-001.1",
    "V10-COMPETITOR-001.2",
    "V10-COMPETITOR-001.3",
    "V10-COMPETITOR-001.4",
    "V10-COMPETITOR-001.5",
];

const CRUSH_IDS: &[&str] = &[
    "V10-CRUSH-001.1",
    "V10-CRUSH-001.2",
    "V10-CRUSH-001.3",
    "V10-CRUSH-001.4",
    "V10-CRUSH-001.5",
    "V10-CRUSH-001.6",
    "V10-CRUSH-001.7",
    "V10-CRUSH-001.8",
];

const MOLE_IDS: &[&str] = &[
    "V11-MOLE-001.1",
    "V11-MOLE-001.2",
    "V11-MOLE-001.3",
    "V11-MOLE-001.4",
];

const POWER_IDS: &[&str] = &[
    "V10-POWER-001.1",
    "V10-POWER-001.2",
    "V10-POWER-001.3",
    "V10-POWER-001.4",
    "V10-POWER-001.5",
    "V10-POWER-001.6",
];

const SWARM_IDS: &[&str] = &[
    "V8-SWARM-002.1",
    "V8-SWARM-002.2",
    "V8-SWARM-002.3",
    "V8-SWARM-002.4",
    "V8-SWARM-002.5",
];

const ECOSYSTEM_V11_IDS: &[&str] = &[
    "V11-ECOSYSTEM-001.1",
    "V11-ECOSYSTEM-001.2",
    "V11-ECOSYSTEM-001.3",
    "V11-ECOSYSTEM-001.4",
    "V11-ECOSYSTEM-001.5",
    "V11-ECOSYSTEM-001.6",
    "V11-ECOSYSTEM-001.7",
];

const ECOSYSTEM_V8_IDS: &[&str] = &[
    "V8-ECOSYSTEM-001.1",
    "V8-ECOSYSTEM-001.2",
    "V8-ECOSYSTEM-001.3",
    "V8-ECOSYSTEM-001.4",
    "V8-ECOSYSTEM-001.5",
    "V8-ECOSYSTEM-001.6",
    "V8-ECOSYSTEM-001.7",
    "V8-ECOSYSTEM-001.8",
];

const MEMORY_BANK_IDS: &[&str] = &[
    "V8-MEMORY-BANK-002.1",
    "V8-MEMORY-BANK-002.2",
    "V8-MEMORY-BANK-002.3",
    "V8-MEMORY-BANK-002.4",
    "V8-MEMORY-BANK-002.5",
    "V8-MEMORY-BANK-002.6",
    "V8-MEMORY-BANK-002.7",
    "V8-MEMORY-BANK-002.8",
];

const F100_IDS: &[&str] = &[
    "V7-F100-002.3",
    "V7-F100-002.7",
    "V7-F100-005",
    "V7-F100-006",
    "V7-F100-007",
    "V7-F100-008",
];

const V5_HOLD_IDS: &[&str] = &[
    "V5-HOLD-001",
    "V5-HOLD-002",
    "V5-HOLD-003",
    "V5-HOLD-004",
    "V5-HOLD-005",
];

const V5_RUST_HYB_IDS: &[&str] = &[
    "V5-RUST-HYB-001",
    "V5-RUST-HYB-002",
    "V5-RUST-HYB-003",
    "V5-RUST-HYB-004",
    "V5-RUST-HYB-005",
    "V5-RUST-HYB-006",
    "V5-RUST-HYB-007",
    "V5-RUST-HYB-008",
    "V5-RUST-HYB-009",
    "V5-RUST-HYB-010",
];

const V5_RUST_PROD_IDS: &[&str] = &[
    "V5-RUST-PROD-001",
    "V5-RUST-PROD-002",
    "V5-RUST-PROD-003",
    "V5-RUST-PROD-004",
    "V5-RUST-PROD-005",
    "V5-RUST-PROD-006",
    "V5-RUST-PROD-007",
    "V5-RUST-PROD-008",
    "V5-RUST-PROD-009",
    "V5-RUST-PROD-010",
    "V5-RUST-PROD-011",
    "V5-RUST-PROD-012",
];

const V6_EXECUTION_002_IDS: &[&str] = &[
    "V6-EXECUTION-002.1",
    "V6-EXECUTION-002.2",
    "V6-EXECUTION-002.3",
    "V6-EXECUTION-002.4",
];

const V6_EXECUTION_003_IDS: &[&str] = &[
    "V6-EXECUTION-003.1",
    "V6-EXECUTION-003.2",
    "V6-EXECUTION-003.3",
    "V6-EXECUTION-003.4",
];

const V6_ASSIMILATE_FAST_001_IDS: &[&str] = &[
    "V6-ASSIMILATE-FAST-001.1",
    "V6-ASSIMILATE-FAST-001.2",
    "V6-ASSIMILATE-FAST-001.3",
    "V6-ASSIMILATE-FAST-001.4",
    "V6-ASSIMILATE-FAST-001.5",
    "V6-ASSIMILATE-FAST-001.6",
];

const V6_WORKFLOW_028_IDS: &[&str] = &[
    "V6-WORKFLOW-028.1",
    "V6-WORKFLOW-028.2",
    "V6-WORKFLOW-028.3",
    "V6-WORKFLOW-028.4",
    "V6-WORKFLOW-028.5",
];

const V6_MEMORY_CONTEXT_001_IDS: &[&str] = &[
    "V6-MEMORY-CONTEXT-001.1",
    "V6-MEMORY-CONTEXT-001.2",
    "V6-MEMORY-CONTEXT-001.3",
    "V6-MEMORY-CONTEXT-001.4",
    "V6-MEMORY-CONTEXT-001.5",
];

const V6_INTEGRATION_001_IDS: &[&str] = &[
    "V6-INTEGRATION-001.1",
    "V6-INTEGRATION-001.2",
    "V6-INTEGRATION-001.3",
    "V6-INTEGRATION-001.4",
    "V6-INTEGRATION-001.5",
    "V6-INTEGRATION-001.6",
];

const V6_INFERENCE_005_IDS: &[&str] = &[
    "V6-INFERENCE-005.1",
    "V6-INFERENCE-005.2",
    "V6-INFERENCE-005.3",
    "V6-INFERENCE-005.4",
];

const V6_RUNTIME_CLEANUP_001_IDS: &[&str] = &[
    "V6-RUNTIME-CLEANUP-001.1",
    "V6-RUNTIME-CLEANUP-001.2",
    "V6-RUNTIME-CLEANUP-001.3",
    "V6-RUNTIME-CLEANUP-001.4",
    "V6-RUNTIME-CLEANUP-001.5",
    "V6-RUNTIME-CLEANUP-001.6",
    "V6-RUNTIME-CLEANUP-001.7",
];

const V6_ERP_AGENTIC_001_IDS: &[&str] = &[
    "V6-ERP-AGENTIC-001.1",
    "V6-ERP-AGENTIC-001.2",
    "V6-ERP-AGENTIC-001.3",
];

const V6_TOOLING_001_IDS: &[&str] = &[
    "V6-TOOLING-001.1",
    "V6-TOOLING-001.2",
    "V6-TOOLING-001.3",
    "V6-TOOLING-001.4",
    "V6-TOOLING-001.5",
];

const V6_WORKFLOW_029_IDS: &[&str] = &[
    "V6-WORKFLOW-029.1",
    "V6-WORKFLOW-029.2",
    "V6-WORKFLOW-029.3",
    "V6-WORKFLOW-029.4",
    "V6-WORKFLOW-029.5",
];

const V6_OPENCLAW_DETACH_001_IDS: &[&str] =
    &["V6-OPENCLAW-DETACH-001.1", "V6-OPENCLAW-DETACH-001.2"];

const CONTRACT_FAMILIES: &[RuntimeSystemContractFamily] = &[
    RuntimeSystemContractFamily {
        ids: ACT_IDS,
        family: "act_critical_judgment",
        objective: "pairwise_critical_judgment_and_self_modification_gate",
    },
    RuntimeSystemContractFamily {
        ids: COMPANY_IDS,
        family: "company_revenue_automation",
        objective: "crm_and_growth_automation_with_conduit_only_boundaries",
    },
    RuntimeSystemContractFamily {
        ids: COMPETITOR_IDS,
        family: "competitor_surface_expansion",
        objective: "provider_adapter_and_domain_hand_expansion_with_production_controls",
    },
    RuntimeSystemContractFamily {
        ids: CRUSH_IDS,
        family: "go_to_market_crush",
        objective: "enterprise_grade_distribution_migration_and_governance_flywheel",
    },
    RuntimeSystemContractFamily {
        ids: MOLE_IDS,
        family: "compatibility_mole",
        objective: "silent_protocol_compatibility_and_import_safety_absorption",
    },
    RuntimeSystemContractFamily {
        ids: POWER_IDS,
        family: "power_execution",
        objective: "release_speed_predictive_router_endurance_and_blocker_closure",
    },
    RuntimeSystemContractFamily {
        ids: SWARM_IDS,
        family: "swarm_orchestration",
        objective: "parallel_swarm_planning_and_shared_memory_under_conduit_enforcement",
    },
    RuntimeSystemContractFamily {
        ids: ECOSYSTEM_V11_IDS,
        family: "ecosystem_scale_v11",
        objective: "adoption_hub_marketplace_sdk_and_governance_economy",
    },
    RuntimeSystemContractFamily {
        ids: ECOSYSTEM_V8_IDS,
        family: "ecosystem_scale_v8",
        objective: "always_on_runtime_skills_import_and_realtime_companion_capabilities",
    },
    RuntimeSystemContractFamily {
        ids: MEMORY_BANK_IDS,
        family: "memory_bank_v2",
        objective: "multi_tier_memory_bank_with_decay_cross_reference_and_session_continuation",
    },
    RuntimeSystemContractFamily {
        ids: F100_IDS,
        family: "f100_assurance",
        objective: "zero_trust_enterprise_profile_and_super_gate_assurance_enforcement",
    },
    RuntimeSystemContractFamily {
        ids: V5_HOLD_IDS,
        family: "v5_hold_remediation",
        objective: "hold_category_reduction_with_fail_closed_routeability_and_budget_controls",
    },
    RuntimeSystemContractFamily {
        ids: V5_RUST_HYB_IDS,
        family: "v5_rust_hybrid",
        objective: "bounded_hybrid_rust_migration_with_hotpath_cutovers_and_guardrails",
    },
    RuntimeSystemContractFamily {
        ids: V5_RUST_PROD_IDS,
        family: "v5_rust_productivity",
        objective: "enterprise_rust_productivity_lane_with_perf_canary_and_unit_economics_controls",
    },
    RuntimeSystemContractFamily {
        ids: V6_EXECUTION_002_IDS,
        family: "execution_streaming_stack",
        objective: "ssd_streaming_quantization_cache_first_and_kernel_path_for_moe_execution",
    },
    RuntimeSystemContractFamily {
        ids: V6_EXECUTION_003_IDS,
        family: "execution_worktree_stack",
        objective: "per_agent_worktree_isolation_safe_merge_orchestration_and_cleanup_boundedness",
    },
    RuntimeSystemContractFamily {
        ids: V6_ASSIMILATE_FAST_001_IDS,
        family: "assimilate_fast_stack",
        objective: "fast_assimilation_mode_with_cache_progress_parallelism_warmup_and_safety_disclosure",
    },
    RuntimeSystemContractFamily {
        ids: V6_WORKFLOW_028_IDS,
        family: "workflow_open_swe_stack",
        objective: "open_swe_loop_registry_git_bridge_hitl_eval_harness_and_memory_continuity",
    },
    RuntimeSystemContractFamily {
        ids: V6_MEMORY_CONTEXT_001_IDS,
        family: "memory_context_maintenance",
        objective: "staleness_tracking_pre_generation_pruning_emergency_compaction_and_context_health",
    },
    RuntimeSystemContractFamily {
        ids: V6_INTEGRATION_001_IDS,
        family: "integration_lakehouse_stack",
        objective: "databricks_mlflow_vector_automl_dbrx_and_drift_monitoring_bridges",
    },
    RuntimeSystemContractFamily {
        ids: V6_INFERENCE_005_IDS,
        family: "inference_adaptive_routing",
        objective: "live_provider_scoring_policy_routing_ordered_failover_and_provider_health_observability",
    },
    RuntimeSystemContractFamily {
        ids: V6_RUNTIME_CLEANUP_001_IDS,
        family: "runtime_cleanup_autonomous",
        objective: "multi_trigger_self_cleaning_with_tiered_reclaim_device_profiles_and_boundedness_gates",
    },
    RuntimeSystemContractFamily {
        ids: V6_ERP_AGENTIC_001_IDS,
        family: "erp_agentic_stack",
        objective: "erp_team_templates_closed_loop_decisioning_and_lineage_enforced_governed_actions",
    },
    RuntimeSystemContractFamily {
        ids: V6_TOOLING_001_IDS,
        family: "tooling_uv_ruff_stack",
        objective: "uv_and_ruff_bridges_with_isolated_envs_autowire_pipeline_and_tooling_validation_gate",
    },
    RuntimeSystemContractFamily {
        ids: V6_WORKFLOW_029_IDS,
        family: "workflow_visual_bridge_stack",
        objective: "graph_canvas_prompt_routing_rag_tool_eval_and_enterprise_deployment_observability",
    },
    RuntimeSystemContractFamily {
        ids: V6_OPENCLAW_DETACH_001_IDS,
        family: "openclaw_detachment_stack",
        objective: "assimilate_openclaw_nursery_and_operator_state_into_infring_owned_runtime_surfaces_with_no_external_dependency",
    },
];

fn inferred_family_for(id: &str) -> Option<(&'static str, &'static str)> {
    if id.starts_with("V10-ULTIMATE-001.") {
        return Some((
            "ultimate_evolution",
            "viral_replication_metacognition_exotic_hardware_tokenomics_and_universal_adapters",
        ));
    }
    if id.starts_with("V10-CORE-001.") {
        return Some((
            "ultimate_evolution",
            "core_metakernel_evolution_bootstrap_and_replication_controls",
        ));
    }
    if id.starts_with("V10-ULTIMATE-002.") {
        return Some((
            "ultimate_evolution",
            "ultimate_expansion_lane_for_operator_and_runtime_evolution_controls",
        ));
    }
    if id.starts_with("V10-PHONE-001.") {
        return Some((
            "ecosystem_scale_v11",
            "phone_surface_runtime_integration_and_operator_control_plane",
        ));
    }
    if id.starts_with("V10-SWARM-INF-001.") {
        return Some((
            "swarm_runtime_scaling",
            "swarm_infrastructure_scaling_consensus_and_resilience_controls",
        ));
    }
    if id.starts_with("V10-PERF-001.") {
        return Some((
            "competitive_execution_moat",
            "receipt_batching_simd_lockfree_coordination_pgo_slab_allocation_and_throughput_regression_guards",
        ));
    }
    if id.starts_with("V6-WORKFLOW-") || id.starts_with("V6-EXECUTION-001.") {
        return Some((
            "swarm_runtime_scaling",
            "workflow_orchestration_parallel_execution_and_checkpoint_recovery",
        ));
    }
    if id.starts_with("V6-CODE-REVIEW-") {
        return Some((
            "swarm_runtime_scaling",
            "code_review_automation_orchestration_and_recovery_controls",
        ));
    }
    if id.starts_with("V6-SCHEDULER-002.") || id.starts_with("V6-DASHBOARD-001.") {
        return Some((
            "automation_mission_stack",
            "scheduler_hardening_handoff_memory_security_and_dashboard_control_plane",
        ));
    }
    if id.starts_with("V6-DASHBOARD-007.") {
        return Some((
            "automation_mission_stack",
            "dashboard_queue_conduit_cockpit_autoremediation_and_attention_compaction_under_runtime_pressure",
        ));
    }
    if id.starts_with("V6-DASHBOARD-008.") {
        return Some((
            "automation_mission_stack",
            "dashboard_auto_router_selection_preflight_and_receipted_model_routing",
        ));
    }
    if id.starts_with("V6-APP-023.") {
        return Some((
            "automation_mission_stack",
            "app_plane_operator_runtime_controls_and_dashboard_governance",
        ));
    }
    if id.starts_with("V6-MEMORY-") {
        return Some((
            "memory_depth_stack",
            "memory_depth_decay_compaction_and_provenance_preserving_retrieval",
        ));
    }
    if id.starts_with("V6-SKILLS-") {
        return Some((
            "skills_runtime_pack",
            "skills_runtime_expansion_focus_templates_and_deployment_pack",
        ));
    }
    if id.starts_with("V6-SECURITY-") {
        return Some((
            "security_sandbox_redteam",
            "security_gate_expansion_sandboxing_and_adversarial_resilience",
        ));
    }
    if id.starts_with("V6-LEARNING-") || id.starts_with("V6-INFERENCE-") {
        return Some((
            "learning_rsi_pipeline",
            "learning_and_inference_feedback_loops_distillation_and_policy_retraining",
        ));
    }
    if id.starts_with("V6-ADAPTER-") {
        return Some((
            "competitor_surface_expansion",
            "adapter_surface_expansion_with_provider_router_and_domain_controls",
        ));
    }
    if id.starts_with("V6-BEAT-OPENFANG-") {
        return Some((
            "competitive_execution_moat",
            "openfang_surpass_execution_moat_with_receipted_performance_controls",
        ));
    }
    if id.starts_with("V6-BLINDSPOT-") {
        return Some((
            "autonomy_opportunity_engine",
            "blindspot_detection_and_autonomous_opportunity_prioritization",
        ));
    }
    if id.starts_with("V6-OPENCLAW-DETACH-001.") {
        return Some((
            "openclaw_detachment_stack",
            "assimilate_openclaw_home_assets_into_infring_runtime_state_and_determine_local_independence_surfaces",
        ));
    }
    if id.starts_with("V6-ECONOMY-003.") {
        return Some((
            "ecosystem_scale_v11",
            "economy_loop_growth_governance_and_marketplace_alignment",
        ));
    }
    if id.starts_with("V8-AUTOMATION-016.") {
        return Some((
            "automation_mission_stack",
            "cron_handoff_memory_security_and_dashboard_hardening",
        ));
    }
    if id.starts_with("V8-AUTONOMY-012.") {
        return Some((
            "autonomy_opportunity_engine",
            "opportunity_scanning_inefficiency_detection_and_monetization_prioritization",
        ));
    }
    if id.starts_with("V8-CLI-001.") {
        return Some((
            "cli_surface_hardening",
            "single_rust_binary_state_machine_and_node_optional_wrapper_hardening",
        ));
    }
    if id.starts_with("V8-CLIENT-010.") {
        return Some((
            "client_model_access",
            "vibe_proxy_and_model_access_store_with_policy_controls",
        ));
    }
    if id.starts_with("V8-COMPETE-001.") {
        return Some((
            "competitive_execution_moat",
            "aot_performance_signed_receipts_non_divergence_and_resilience_flywheel",
        ));
    }
    if id.starts_with("V8-EYES-009.") {
        return Some((
            "eyes_media_assimilation",
            "video_transcription_course_assimilation_podcast_generation_and_swarm_integration",
        ));
    }
    if id.starts_with("V8-EYES-010.") {
        return Some((
            "eyes_computer_use",
            "browser_computer_use_navigation_reliability_voice_and_safety_gate",
        ));
    }
    if id.starts_with("V8-EYES-011.") {
        return Some((
            "eyes_lightpanda_router",
            "lightpanda_speed_profile_and_multi_backend_router_with_session_archival",
        ));
    }
    if id.starts_with("V8-LEARNING-") {
        return Some((
            "learning_rsi_pipeline",
            "signal_extraction_distillation_distributed_training_and_policy_retraining",
        ));
    }
    if id.starts_with("V8-MEMORY-") {
        return Some((
            "memory_depth_stack",
            "hierarchical_retrieval_lossless_sync_ast_indexing_and_provenance_memory",
        ));
    }
    if id.starts_with("V8-ORGANISM-") {
        return Some((
            "organism_parallel_intelligence",
            "side_sessions_hub_spoke_coordination_model_generation_and_evolution_archive",
        ));
    }
    if id.starts_with("V8-PERSONA-015.") {
        return Some((
            "persona_enterprise_pack",
            "ai_ceo_departmental_pack_cross_agent_memory_sync_and_role_extension",
        ));
    }
    if id.starts_with("V8-SAFETY-022.") {
        return Some((
            "safety_error_taxonomy",
            "structured_error_taxonomy_and_fail_closed_safety_receipts",
        ));
    }
    if id.starts_with("V8-SECURITY-") {
        return Some((
            "security_sandbox_redteam",
            "wasm_sandbox_credential_injection_privacy_plane_and_attack_chain_simulation",
        ));
    }
    if id.starts_with("V8-SKILLS-") {
        return Some((
            "skills_runtime_pack",
            "hf_cli_focus_templates_prompt_chaining_scaffolding_and_deployment_pack",
        ));
    }
    if id.starts_with("V8-SWARM-") {
        return Some((
            "swarm_runtime_scaling",
            "sentiment_swarm_role_routing_work_stealing_watchdog_and_real_time_dashboard",
        ));
    }
    if id.starts_with("V9-AUDIT-026.") {
        return Some((
            "audit_self_healing_stack",
            "self_healing_audit_stack_with_cross_agent_verification_and_human_gate",
        ));
    }
    if id.starts_with("V9-CLIENT-020.") {
        return Some((
            "client_wasm_bridge",
            "rust_wasm_bridge_structured_concurrency_demo_generation_and_artifact_archival",
        ));
    }
    if id.starts_with("V9-ORGANISM-025.") {
        return Some((
            "organism_adlc",
            "adlc_goals_replanning_parallel_subagents_and_live_feedback_testing",
        ));
    }
    if id.starts_with("V9-TINYMAX-021.") {
        return Some((
            "tinymax_extreme_profile",
            "trait_swappable_tinymax_core_and_sub5mb_idle_memory_mode",
        ));
    }
    None
}

fn build_profiles() -> Vec<RuntimeSystemContractProfile> {
    let mut out = BTreeMap::new();
    for group in CONTRACT_FAMILIES {
        for id in group.ids {
            out.insert(
                *id,
                RuntimeSystemContractProfile {
                    id: *id,
                    family: group.family,
                    objective: group.objective,
                    strict_conduit_only: true,
                    strict_fail_closed: true,
                },
            );
        }
    }
    for id in NEW_ACTIONABLE_IDS {
        let (family, objective) =
            inferred_family_for(id).unwrap_or(("unknown_contract_family", "unknown_objective"));
        out.insert(
            *id,
            RuntimeSystemContractProfile {
                id: *id,
                family,
                objective,
                strict_conduit_only: true,
                strict_fail_closed: true,
            },
        );
    }
    out.into_values().collect()
}

fn profiles_registry() -> &'static [RuntimeSystemContractProfile] {
    static REGISTRY: OnceLock<Vec<RuntimeSystemContractProfile>> = OnceLock::new();
    REGISTRY.get_or_init(build_profiles).as_slice()
}

fn profile_index() -> &'static BTreeMap<&'static str, RuntimeSystemContractProfile> {
    static INDEX: OnceLock<BTreeMap<&'static str, RuntimeSystemContractProfile>> = OnceLock::new();
    INDEX.get_or_init(|| {
        profiles_registry()
            .iter()
            .copied()
            .map(|profile| (profile.id, profile))
            .collect()
    })
}

pub fn actionable_profiles() -> &'static [RuntimeSystemContractProfile] {
    profiles_registry()
}

pub fn actionable_ids() -> &'static [&'static str] {
    static IDS: OnceLock<Vec<&'static str>> = OnceLock::new();
    IDS.get_or_init(|| profiles_registry().iter().map(|row| row.id).collect())
        .as_slice()
}

pub fn profile_for(system_id: &str) -> Option<RuntimeSystemContractProfile> {
    let wanted = system_id.trim();
    profile_index().get(wanted).copied()
}

pub fn looks_like_contract_id(system_id: &str) -> bool {
    let id = system_id.trim();
    id.starts_with('V') && id.contains('-')
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn actionable_registry_has_expected_cardinality_and_no_duplicates() {
        let profiles = actionable_profiles();
        let mut expected = BTreeSet::new();
        for group in CONTRACT_FAMILIES {
            for id in group.ids {
                expected.insert((*id).to_string());
            }
        }
        for id in NEW_ACTIONABLE_IDS {
            expected.insert((*id).to_string());
        }
        assert_eq!(
            profiles.len(),
            expected.len(),
            "runtime contract profile count should match static registry inputs"
        );
        let mut seen = BTreeSet::new();
        for profile in profiles {
            assert!(
                seen.insert(profile.id.to_string()),
                "duplicate contract id in runtime registry: {}",
                profile.id
            );
            assert!(profile.strict_conduit_only);
            assert!(profile.strict_fail_closed);
        }
    }

    #[test]
    fn profile_lookup_resolves_known_and_rejects_unknown_ids() {
        assert!(profile_for("V8-ACT-001.1").is_some());
        assert!(profile_for("V11-ECOSYSTEM-001.7").is_some());
        assert!(profile_for("V6-COMPANY-003.5").is_some());
        assert!(profile_for("V6-EXECUTION-002.4").is_some());
        assert!(profile_for("V6-RUNTIME-CLEANUP-001.7").is_some());
        assert!(profile_for("V5-HOLD-001").is_some());
        assert!(profile_for("V5-RUST-HYB-010").is_some());
        assert!(profile_for("V5-RUST-PROD-012").is_some());
        assert!(profile_for("V10-ULTIMATE-001.6").is_some());
        assert!(profile_for("V10-PERF-001.6").is_some());
        assert!(profile_for("V6-WORKFLOW-026.5").is_some());
        assert!(profile_for("V6-DASHBOARD-007.8").is_some());
        assert!(profile_for("V6-DASHBOARD-008.4").is_some());
        assert!(profile_for("V6-OPENCLAW-DETACH-001.2").is_some());
        assert!(profile_for("V8-SWARM-012.10").is_some());
        assert!(profile_for("V9-TINYMAX-021.2").is_some());
        assert!(profile_for("X-UNKNOWN-404.1").is_none());
    }

    #[test]
    fn inferred_family_covers_every_new_actionable_id() {
        for id in NEW_ACTIONABLE_IDS {
            assert!(
                inferred_family_for(id).is_some(),
                "new actionable id missing inferred family: {id}"
            );
        }
    }
}
