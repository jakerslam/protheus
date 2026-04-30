use crate::self_maintenance::contracts::{Claim, ClaimBundle, EscalationRequest, RemediationClass};
use sha2::{Digest, Sha256};

pub fn requires_high_risk_escalation(claim: &Claim) -> bool {
    !matches!(
        claim.remediation_class,
        RemediationClass::DocsDriftFix
            | RemediationClass::PathCorrection
            | RemediationClass::CleanupTask
            | RemediationClass::BacklogHygiene
    )
}

pub fn build_escalation_request(claim_bundle: &ClaimBundle, claim: &Claim) -> EscalationRequest {
    let reason_codes = vec![
        "high_risk_mutation_requires_verity".to_string(),
        format!(
            "remediation_class:{}",
            format!("{:?}", claim.remediation_class).to_ascii_lowercase()
        ),
    ];
    EscalationRequest {
        escalation_id: stable_id(&format!(
            "{}::{}::{}",
            claim_bundle.claim_bundle_id, claim_bundle.task_id, claim.claim_id
        )),
        claim_bundle: claim_bundle.clone(),
        proposed_diff: format!("deferred_auto_apply_for_claim={}", claim.claim_id),
        impact_analysis:
            "Potential structural or policy-level mutation is outside safe auto-apply scope."
                .to_string(),
        rollback_plan:
            "No mutation applied. Re-run in propose mode or execute with explicit Verity approval."
                .to_string(),
        reason_codes,
    }
}

fn stable_id(raw: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(raw.as_bytes());
    format!("escalation-{:x}", hasher.finalize())
}
