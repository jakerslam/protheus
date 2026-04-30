// Layer ownership: surface/orchestration (non-canonical orchestration coordination only).
use super::{SubdomainBoundary, SubdomainContract};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecurityPostureState {
    Active,
    Disabled,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecurityPostureInput {
    pub feature_key: String,
    pub backend_value: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuditVerificationStatus {
    Valid,
    Invalid,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditVerificationReceipt {
    pub status: AuditVerificationStatus,
    pub entry_count: Option<usize>,
    pub error_present: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerPollingRequest {
    pub active_tab: String,
    pub backend_poll_interval_secs: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MigrationIntent {
    AutoDetect,
    Scan,
    Run,
    DryRun,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationRequest {
    pub intent: MigrationIntent,
    pub source_path_present: bool,
    pub target_path_present: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecurityNetworkMigrationAction {
    ProjectSecurityPosture {
        feature_key: String,
        state: SecurityPostureState,
    },
    ProjectAuditVerification {
        status: AuditVerificationStatus,
    },
    RequestPeerSnapshot {
        poll_interval_secs: u64,
    },
    StopPeerPolling {
        reason: String,
    },
    RequestMigrationJob {
        dry_run: bool,
    },
    RequestMigrationScan,
    Clarify {
        prompt: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecurityNetworkMigrationPlan {
    pub action: SecurityNetworkMigrationAction,
    pub telemetry_note: String,
}

pub struct SecurityNetworkMigrationContract;

impl SubdomainContract for SecurityNetworkMigrationContract {
    fn boundary() -> SubdomainBoundary {
        boundary()
    }
}

pub fn boundary() -> SubdomainBoundary {
    SubdomainBoundary {
        id: "security_network_migration",
        legacy_module_bindings: &[
            "settings_security_network_helpers",
            "settings",
            "runtime",
            "overview",
        ],
        allowed_kernel_inputs: &[
            "typed_request_snapshot",
            "execution_observation_snapshot",
            "policy_scope_snapshot",
        ],
        allowed_kernel_outputs: &[
            "security_posture_projection",
            "audit_verification_projection",
            "peer_snapshot_projection",
            "migration_workflow_projection",
        ],
        message_boundaries: &[
            "security_to_shell_projection_boundary",
            "network_to_shell_projection_boundary",
            "migration_to_runtime_admission_boundary",
        ],
    }
}

pub fn project_security_posture(input: &SecurityPostureInput) -> SecurityNetworkMigrationPlan {
    let state = match input.backend_value {
        Some(true) => SecurityPostureState::Active,
        Some(false) => SecurityPostureState::Disabled,
        None => SecurityPostureState::Unknown,
    };
    SecurityNetworkMigrationPlan {
        action: SecurityNetworkMigrationAction::ProjectSecurityPosture {
            feature_key: input.feature_key.trim().to_string(),
            state,
        },
        telemetry_note: "security posture projection must not infer active from missing data"
            .to_string(),
    }
}

pub fn project_audit_verification(
    receipt: &AuditVerificationReceipt,
) -> SecurityNetworkMigrationPlan {
    SecurityNetworkMigrationPlan {
        action: SecurityNetworkMigrationAction::ProjectAuditVerification {
            status: receipt.status.clone(),
        },
        telemetry_note: "audit verification renders from backend receipt status".to_string(),
    }
}

pub fn coordinate_peer_polling(request: &PeerPollingRequest) -> SecurityNetworkMigrationPlan {
    if request.active_tab.trim() != "network" {
        return SecurityNetworkMigrationPlan {
            action: SecurityNetworkMigrationAction::StopPeerPolling {
                reason: "network tab is not active".to_string(),
            },
            telemetry_note: "peer polling disabled outside network surface".to_string(),
        };
    }
    SecurityNetworkMigrationPlan {
        action: SecurityNetworkMigrationAction::RequestPeerSnapshot {
            poll_interval_secs: request.backend_poll_interval_secs.unwrap_or(15),
        },
        telemetry_note: "request backend peer snapshot on declared cadence".to_string(),
    }
}

pub fn coordinate_migration(request: &MigrationRequest) -> SecurityNetworkMigrationPlan {
    match request.intent {
        MigrationIntent::AutoDetect => SecurityNetworkMigrationPlan {
            action: SecurityNetworkMigrationAction::RequestMigrationScan,
            telemetry_note: "request backend migration auto-detect scan".to_string(),
        },
        MigrationIntent::Scan => {
            if !request.source_path_present {
                return clarify(
                    "migration scan requires a source path",
                    "missing source path for migration scan",
                );
            }
            SecurityNetworkMigrationPlan {
                action: SecurityNetworkMigrationAction::RequestMigrationScan,
                telemetry_note: "request backend migration scan receipt".to_string(),
            }
        }
        MigrationIntent::DryRun => migration_job(true, request),
        MigrationIntent::Run => migration_job(false, request),
    }
}

fn migration_job(dry_run: bool, request: &MigrationRequest) -> SecurityNetworkMigrationPlan {
    if !request.source_path_present {
        return clarify(
            "migration run requires a source path",
            "missing source path for migration run",
        );
    }
    SecurityNetworkMigrationPlan {
        action: SecurityNetworkMigrationAction::RequestMigrationJob { dry_run },
        telemetry_note: "request backend migration job with progress receipts".to_string(),
    }
}

fn clarify(prompt: &str, note: &str) -> SecurityNetworkMigrationPlan {
    SecurityNetworkMigrationPlan {
        action: SecurityNetworkMigrationAction::Clarify {
            prompt: prompt.to_string(),
        },
        telemetry_note: note.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_security_data_projects_unknown_not_active() {
        let plan = project_security_posture(&SecurityPostureInput {
            feature_key: "ssrf_protection".to_string(),
            backend_value: None,
        });

        assert_eq!(
            plan.action,
            SecurityNetworkMigrationAction::ProjectSecurityPosture {
                feature_key: "ssrf_protection".to_string(),
                state: SecurityPostureState::Unknown
            }
        );
    }

    #[test]
    fn audit_verification_projects_receipt_status() {
        let plan = project_audit_verification(&AuditVerificationReceipt {
            status: AuditVerificationStatus::Invalid,
            entry_count: Some(42),
            error_present: true,
        });

        assert_eq!(
            plan.action,
            SecurityNetworkMigrationAction::ProjectAuditVerification {
                status: AuditVerificationStatus::Invalid
            }
        );
    }

    #[test]
    fn peer_polling_uses_backend_cadence_only_on_network_tab() {
        let plan = coordinate_peer_polling(&PeerPollingRequest {
            active_tab: "network".to_string(),
            backend_poll_interval_secs: Some(30),
        });

        assert_eq!(
            plan.action,
            SecurityNetworkMigrationAction::RequestPeerSnapshot {
                poll_interval_secs: 30
            }
        );
    }

    #[test]
    fn peer_polling_stops_outside_network_tab() {
        let plan = coordinate_peer_polling(&PeerPollingRequest {
            active_tab: "providers".to_string(),
            backend_poll_interval_secs: Some(30),
        });

        assert_eq!(
            plan.action,
            SecurityNetworkMigrationAction::StopPeerPolling {
                reason: "network tab is not active".to_string()
            }
        );
    }

    #[test]
    fn migration_scan_requires_source_path() {
        let plan = coordinate_migration(&MigrationRequest {
            intent: MigrationIntent::Scan,
            source_path_present: false,
            target_path_present: false,
        });

        assert_eq!(
            plan.action,
            SecurityNetworkMigrationAction::Clarify {
                prompt: "migration scan requires a source path".to_string()
            }
        );
    }

    #[test]
    fn dry_run_requests_migration_job_receipt() {
        let plan = coordinate_migration(&MigrationRequest {
            intent: MigrationIntent::DryRun,
            source_path_present: true,
            target_path_present: false,
        });

        assert_eq!(
            plan.action,
            SecurityNetworkMigrationAction::RequestMigrationJob { dry_run: true }
        );
    }
}
