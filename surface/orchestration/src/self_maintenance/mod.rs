//! Supervisory coordination only.
//! Self-maintenance may read via contracts, use ephemeral memory, and propose work through
//! Task Fabric, but it must not create durable truth or bypass core-owned mutation authority.

pub mod analyzer;
pub mod contracts;
pub mod escalation;
pub mod executor;
pub mod observer;
pub mod task_generator;

pub use contracts::{
    Claim, ClaimBundle, ConfidenceVector, EscalationRequest, EvidenceCard, ObservationInputs,
    SupervisorMode, SupervisorReceipt, SupervisorReceiptStage, SupervisorRunResult, WorkerOutput,
};
pub use executor::GovernedSelfMaintenanceSupervisor;
