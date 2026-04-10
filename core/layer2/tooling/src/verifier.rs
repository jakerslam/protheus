use crate::deterministic_hash;
use crate::schemas::{Claim, ClaimBundle, ClaimStatus, ConfidenceVector, EvidenceCard};
use std::collections::{HashMap, HashSet};

#[derive(Default)]
pub struct StructuredVerifier;

fn has_negative_cue(text: &str) -> bool {
    const NEGATIVE_CUES: &[&str] = &["not", "no", "failed", "fails", "denied", "missing"];
    let tokens = text
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|row| !row.is_empty())
        .map(|row| row.to_ascii_lowercase())
        .collect::<Vec<_>>();
    tokens
        .iter()
        .any(|token| NEGATIVE_CUES.iter().any(|cue| token == cue))
}

impl StructuredVerifier {
    pub fn derive_claim_bundle(
        &self,
        task_id: &str,
        evidence_cards: &[EvidenceCard],
    ) -> ClaimBundle {
        let mut claims = Vec::<Claim>::new();
        let mut conflicts = Vec::<String>::new();
        let mut unresolved_questions = Vec::<String>::new();
        for (claim_index, card) in evidence_cards.iter().enumerate() {
            let status = support_status(&card.confidence_vector, &card.summary);
            if status == ClaimStatus::Unsupported {
                unresolved_questions.push(format!(
                    "Need stronger evidence for source {}",
                    card.source_ref
                ));
            }
            let claim_content_id = deterministic_hash(&serde_json::json!({
                "kind":"claim_content",
                "task_id": task_id,
                "evidence_content_id": card.evidence_content_id,
                "text": card.summary,
            }));
            let claim_event_id = deterministic_hash(&serde_json::json!({
                "kind":"claim_event",
                "task_id": task_id,
                "claim_content_id": claim_content_id,
                "evidence_event_id": card.evidence_event_id,
                "claim_index": claim_index,
            }));
            let claim = Claim {
                claim_id: claim_content_id.clone(),
                claim_content_id,
                claim_event_id,
                text: card.summary.clone(),
                evidence_ids: vec![card.evidence_id.clone()],
                status,
                confidence_vector: card.confidence_vector.clone(),
                conflict_refs: Vec::new(),
            };
            claims.push(claim);
        }
        let mut text_index = HashMap::<String, Vec<usize>>::new();
        for (idx, claim) in claims.iter().enumerate() {
            let key = claim.text.to_ascii_lowercase();
            text_index.entry(key).or_default().push(idx);
        }
        for indexes in text_index.values() {
            if indexes.len() < 2 {
                continue;
            }
            let mut has_negative = false;
            let mut has_positive = false;
            for idx in indexes {
                if has_negative_cue(&claims[*idx].text) {
                    has_negative = true;
                } else {
                    has_positive = true;
                }
            }
            if has_negative && has_positive {
                for idx in indexes {
                    claims[*idx].status = ClaimStatus::Conflicting;
                    claims[*idx].conflict_refs = indexes
                        .iter()
                        .filter(|row| **row != *idx)
                        .map(|row| claims[*row].claim_id.clone())
                        .collect::<Vec<_>>();
                    conflicts.push(claims[*idx].claim_id.clone());
                }
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
        let claim_ids = claims
            .iter()
            .map(|claim| claim.claim_id.clone())
            .collect::<Vec<_>>();
        let claim_bundle_content_id = deterministic_hash(&serde_json::json!({
            "kind":"claim_bundle_content",
            "task_id": task_id,
            "claim_ids": claim_ids
        }));
        let claim_bundle_event_id = deterministic_hash(&serde_json::json!({
            "kind":"claim_bundle_event",
            "task_id": task_id,
            "claim_bundle_content_id": claim_bundle_content_id,
            "evidence_count": evidence_cards.len(),
        }));
        ClaimBundle {
            claim_bundle_id: claim_bundle_content_id.clone(),
            claim_bundle_content_id,
            claim_bundle_event_id,
            task_id: task_id.to_string(),
            claims,
            unresolved_questions,
            conflicts,
            coverage_score,
        }
    }

    pub fn supported_claims_for_synthesis<'a>(&self, bundle: &'a ClaimBundle) -> Vec<&'a Claim> {
        bundle
            .claims
            .iter()
            .filter(|claim| matches!(claim.status, ClaimStatus::Supported | ClaimStatus::Partial))
            .collect::<Vec<_>>()
    }

    pub fn validate_claim_evidence_refs(
        &self,
        bundle: &ClaimBundle,
        evidence_cards: &[EvidenceCard],
    ) -> Result<(), String> {
        let evidence_ids = evidence_cards
            .iter()
            .map(|row| row.evidence_id.as_str())
            .collect::<HashSet<_>>();
        for claim in &bundle.claims {
            if claim.evidence_ids.is_empty() {
                return Err(format!("claim_without_evidence:{}", claim.claim_id));
            }
            for evidence_id in &claim.evidence_ids {
                if !evidence_ids.contains(evidence_id.as_str()) {
                    return Err(format!(
                        "claim_references_unknown_evidence:{}:{}",
                        claim.claim_id, evidence_id
                    ));
                }
            }
        }
        Ok(())
    }
}

fn support_status(confidence: &ConfidenceVector, summary: &str) -> ClaimStatus {
    let avg = (confidence.relevance + confidence.reliability + confidence.freshness) / 3.0;
    if summary.trim().is_empty() {
        return ClaimStatus::Unsupported;
    }
    if avg >= 0.74 {
        ClaimStatus::Supported
    } else if avg >= 0.45 {
        ClaimStatus::Partial
    } else {
        ClaimStatus::Unsupported
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card(id: &str, text: &str, reliability: f64) -> EvidenceCard {
        EvidenceCard {
            evidence_id: id.to_string(),
            evidence_content_id: format!("content-{id}"),
            evidence_event_id: format!("event-{id}"),
            trace_id: "trace-1".to_string(),
            task_id: "task-1".to_string(),
            derived_from_result_id: "r1".to_string(),
            source_ref: "https://example.com".to_string(),
            source_location: "payload".to_string(),
            excerpt: text.to_string(),
            summary: text.to_string(),
            confidence_vector: ConfidenceVector {
                relevance: reliability,
                reliability,
                freshness: reliability,
            },
            dedupe_hash: format!("d-{id}"),
            lineage: vec!["l1".to_string()],
            timestamp: 1,
        }
    }

    #[test]
    fn verifier_labels_supported_and_unsupported_claims() {
        let verifier = StructuredVerifier;
        let bundle = verifier.derive_claim_bundle(
            "task-1",
            &[card("e1", "Result is stable", 0.9), card("e2", "Weak", 0.2)],
        );
        assert_eq!(bundle.claims.len(), 2);
        assert!(bundle
            .claims
            .iter()
            .any(|claim| claim.status == ClaimStatus::Supported));
        assert!(bundle
            .claims
            .iter()
            .any(|claim| claim.status == ClaimStatus::Unsupported));
        verifier
            .validate_claim_evidence_refs(
                &bundle,
                &[card("e1", "Result is stable", 0.9), card("e2", "Weak", 0.2)],
            )
            .expect("claims should always map to evidence");
        let synth = verifier.supported_claims_for_synthesis(&bundle);
        assert!(!synth.is_empty());
    }
}
