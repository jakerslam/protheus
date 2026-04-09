use crate::self_maintenance::contracts::{
    confidence_average, Claim, ClaimBundle, ClaimStatus, ClaimType, EvidenceCard, RemediationClass,
};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub fn evidence_to_claim_bundle(task_id: &str, evidence: &[EvidenceCard]) -> ClaimBundle {
    let mut claims = Vec::<Claim>::new();
    let mut unresolved_questions = Vec::<String>::new();
    let mut conflicts = Vec::<String>::new();

    for card in evidence {
        let avg = confidence_average(&card.confidence_vector);
        let status = if avg >= 0.78 {
            ClaimStatus::Supported
        } else if avg >= 0.52 {
            ClaimStatus::Partial
        } else {
            ClaimStatus::Unsupported
        };
        if status != ClaimStatus::Supported {
            unresolved_questions.push(format!(
                "review_required:{}:{}",
                card.source_ref, card.evidence_id
            ));
        }
        claims.push(Claim {
            claim_id: stable_id("claim", &card.evidence_id),
            claim_type: infer_claim_type(card),
            text: card.summary.clone(),
            evidence_ids: vec![card.evidence_id.clone()],
            status,
            confidence_vector: card.confidence_vector.clone(),
            conflict_refs: Vec::new(),
            remediation_class: infer_remediation_class(card),
        });
    }

    let mut normalized_index = BTreeMap::<String, Vec<usize>>::new();
    for (idx, claim) in claims.iter().enumerate() {
        let key = normalize_claim_key(claim.text.as_str());
        normalized_index.entry(key).or_default().push(idx);
    }

    for idxs in normalized_index.values() {
        if idxs.len() < 2 {
            continue;
        }
        let mut has_positive = false;
        let mut has_negative = false;
        for idx in idxs {
            if is_negative_claim(claims[*idx].text.as_str()) {
                has_negative = true;
            } else {
                has_positive = true;
            }
        }
        if !(has_positive && has_negative) {
            continue;
        }
        for idx in idxs {
            claims[*idx].status = ClaimStatus::Conflicting;
            claims[*idx].conflict_refs = idxs
                .iter()
                .copied()
                .filter(|other| *other != *idx)
                .map(|other| claims[other].claim_id.clone())
                .collect::<Vec<_>>();
            conflicts.push(claims[*idx].claim_id.clone());
        }
    }

    let supported_or_partial = claims
        .iter()
        .filter(|claim| matches!(claim.status, ClaimStatus::Supported | ClaimStatus::Partial))
        .count();
    let coverage_score = if claims.is_empty() {
        0.0
    } else {
        supported_or_partial as f64 / claims.len() as f64
    };

    dedupe_in_place(&mut conflicts);
    dedupe_in_place(&mut unresolved_questions);

    ClaimBundle {
        claim_bundle_id: stable_id(
            "claim_bundle",
            &format!(
                "{}::{}",
                task_id,
                claims
                    .iter()
                    .map(|row| row.claim_id.as_str())
                    .collect::<Vec<_>>()
                    .join(",")
            ),
        ),
        task_id: task_id.to_string(),
        claims,
        unresolved_questions,
        conflicts,
        coverage_score,
    }
}

fn infer_claim_type(card: &EvidenceCard) -> ClaimType {
    if has_tag(card, "violation") || has_tag(card, "blocked") {
        return ClaimType::Violation;
    }
    if has_tag(card, "degraded") || has_tag(card, "health") {
        return ClaimType::Health;
    }
    if has_tag(card, "orphaned") || has_tag(card, "cleanup") {
        return ClaimType::DeadCode;
    }
    if has_tag(card, "stale") || has_tag(card, "drift") {
        return ClaimType::Drift;
    }
    if has_tag(card, "pressure") {
        return ClaimType::Inefficiency;
    }
    ClaimType::Unknown
}

fn infer_remediation_class(card: &EvidenceCard) -> RemediationClass {
    if has_tag(card, "orphaned") || has_tag(card, "cleanup") {
        return RemediationClass::CleanupTask;
    }
    if has_tag(card, "architecture") || has_tag(card, "dependency") {
        return RemediationClass::PathCorrection;
    }
    if has_tag(card, "drift") {
        return RemediationClass::DocsDriftFix;
    }
    if has_tag(card, "backlog") {
        return RemediationClass::BacklogHygiene;
    }
    RemediationClass::Unsafe
}

fn has_tag(card: &EvidenceCard, expected: &str) -> bool {
    card.tags
        .iter()
        .any(|tag| tag.eq_ignore_ascii_case(expected))
}

fn normalize_claim_key(text: &str) -> String {
    let stop = BTreeSet::from([
        "a", "an", "the", "to", "for", "of", "in", "on", "and", "or", "is", "are", "was", "were",
        "with", "by",
    ]);
    text.to_ascii_lowercase()
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|token| !token.is_empty() && !stop.contains(token))
        .take(16)
        .collect::<Vec<_>>()
        .join(" ")
}

fn is_negative_claim(text: &str) -> bool {
    let normalized = format!(" {} ", text.to_ascii_lowercase());
    [
        " fail ",
        " failed ",
        " not ",
        " cannot ",
        " blocked ",
        " denied ",
    ]
    .iter()
    .any(|token| normalized.contains(token))
}

fn dedupe_in_place(values: &mut Vec<String>) {
    let mut seen = BTreeSet::<String>::new();
    values.retain(|value| seen.insert(value.clone()));
}

fn stable_id(prefix: &str, raw: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(raw.as_bytes());
    format!("{prefix}-{:x}", hasher.finalize())
}
