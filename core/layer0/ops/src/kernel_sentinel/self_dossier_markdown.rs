// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use super::system_understanding_dossier::SystemUnderstandingDossier;

pub fn render_infring_self_dossier_markdown(dossier: &SystemUnderstandingDossier) -> String {
    let capabilities = dossier
        .capabilities
        .iter()
        .map(|row| {
            format!(
                "- `{}`: `{:?}` / `{:?}` -> `{:?}`\n  - Fit: {}\n  - Evidence: {}",
                row.id,
                row.kind,
                row.value,
                row.transfer_target,
                row.fit_rationale,
                row.evidence.join(", ")
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let rejected = dossier
        .rejected_capabilities
        .iter()
        .map(|row| format!("- `{}`: {}", row.id, row.fit_rationale))
        .collect::<Vec<_>>()
        .join("\n");
    let implementation = dossier
        .implementation_items
        .iter()
        .map(|item| {
            format!(
                "- `{}` ({})\n  - Summary: {}\n  - Invariant: {}\n  - Proof: {}\n  - Rollback: {}",
                item.id,
                item.owner_layer,
                item.summary,
                item.invariant,
                item.proof_requirement,
                item.rollback_plan
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "# InfRing System Understanding Dossier\n\n\
Metadata:\n\
- `dossier_id`: `{}`\n\
- `target_mode`: `{:?}`\n\
- `target_system`: `{}`\n\
- `target_version_or_revision`: `{}`\n\
- `status`: `{:?}`\n\
- `confidence_overall`: `{:.2}`\n\
- `updated_at`: `{}`\n\n\
## Soul / Philosophy\n\
- Confidence: `{:.2}`\n\
- Evidence: {}\n\n\
## Runtime Behavior\n\
- Confidence: `{:.2}`\n\
- Evidence: {}\n\
- Required next probes: {}\n\n\
## Ecology / Operating Environment\n\
- Confidence: `{:.2}`\n\
- Evidence: {}\n\n\
## Authority / Truth Model\n\
- Confidence: `{:.2}`\n\
- Evidence: {}\n\
- Risks: {}\n\n\
## Architecture / Boundaries\n\
- Confidence: `{:.2}`\n\
- Evidence: {}\n\
- Runtime mismatches: {}\n\n\
## Capability Map\n\
- Confidence: `{:.2}`\n\
{}\n\n\
Rejected capabilities:\n\
{}\n\n\
## Failure Model\n\
- Confidence: `{:.2}`\n\
- Known failure modes: {}\n\
- Violated invariants: {}\n\
- Stop-patching triggers: {}\n\n\
## Transfer / Improvement Plan\n\
- Confidence: `{:.2}`\n\
{}\n\n\
## Implementation Structure\n\
- Confidence: `{:.2}`\n\
- Files inspected: {}\n\n\
## Syntax / Detail\n\
- Confidence: `{:.2}`\n\
- Syntax evidence: {}\n\n\
## Evidence Index\n\
{}\n",
        dossier.dossier_id,
        dossier.target_mode,
        dossier.target_system,
        dossier.target_version_or_revision,
        dossier.status,
        dossier.confidence_overall,
        dossier.updated_at,
        dossier.soul_confidence,
        dossier.soul_evidence.join(", "),
        dossier.runtime_confidence,
        dossier.runtime_evidence.join(", "),
        if dossier.required_next_probes.is_empty() {
            "none".to_string()
        } else {
            dossier.required_next_probes.join(", ")
        },
        dossier.ecology_confidence,
        dossier.ecology_evidence.join(", "),
        dossier.authority_confidence,
        dossier.authority_evidence.join(", "),
        if dossier.authority_risks.is_empty() {
            "none".to_string()
        } else {
            dossier.authority_risks.join(", ")
        },
        dossier.architecture_confidence,
        dossier.architecture_evidence.join(", "),
        if dossier.runtime_architecture_mismatches.is_empty() {
            "none".to_string()
        } else {
            dossier.runtime_architecture_mismatches.join(", ")
        },
        dossier.capability_confidence,
        capabilities,
        if rejected.is_empty() {
            "- none".to_string()
        } else {
            rejected
        },
        dossier.failure_model_confidence,
        if dossier.known_failure_modes.is_empty() {
            "none".to_string()
        } else {
            dossier.known_failure_modes.join(", ")
        },
        if dossier.violated_invariants.is_empty() {
            "none".to_string()
        } else {
            dossier.violated_invariants.join(", ")
        },
        if dossier.stop_patching_triggers.is_empty() {
            "none".to_string()
        } else {
            dossier.stop_patching_triggers.join(", ")
        },
        dossier.transfer_confidence,
        implementation,
        dossier.implementation_confidence,
        dossier.files_inspected.join(", "),
        dossier.syntax_confidence,
        dossier.syntax_evidence.join(", "),
        dossier
            .evidence_index
            .iter()
            .map(|row| format!("- {}", row))
            .collect::<Vec<_>>()
            .join("\n")
    )
}
