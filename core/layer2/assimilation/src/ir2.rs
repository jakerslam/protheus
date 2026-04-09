// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer2/assimilation (authoritative).

use crate::ir1::Ir1ExecutionStructure;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FactObservation {
    pub fact_id: String,
    pub description: String,
    pub source_block_id: String,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Hypothesis {
    pub hypothesis_id: String,
    pub statement: String,
    pub score: f64,
    pub supporting_fact_ids: Vec<String>,
    pub proof_type: String,
    pub admitted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CanonicalConcept {
    pub concept_id: String,
    pub label: String,
    pub supporting_hypothesis_ids: Vec<String>,
    pub confidence: f64,
    pub proof_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct FactLedger {
    pub facts: Vec<FactObservation>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct HypothesisLedger {
    pub hypotheses: Vec<Hypothesis>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct CanonicalOntology {
    pub concepts: Vec<CanonicalConcept>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Ir2SemanticLift {
    pub ir2_id: String,
    pub ir1_id: String,
    pub fact_ledger: FactLedger,
    pub hypothesis_ledger: HypothesisLedger,
    pub ontology: CanonicalOntology,
}

impl Ir2SemanticLift {
    pub fn from_ir1(ir1: &Ir1ExecutionStructure) -> Self {
        let mut facts = Vec::new();
        let mut hypotheses = Vec::new();
        for (idx, block) in ir1.blocks.iter().enumerate() {
            let fact_id = format!("fact-{idx:04}");
            let hypothesis_id = format!("hyp-{idx:04}");
            facts.push(FactObservation {
                fact_id: fact_id.clone(),
                description: format!(
                    "Block {} spans {} bytes in region {}",
                    block.block_id,
                    block.end_offset.saturating_sub(block.start_offset),
                    block.source_region_id
                ),
                source_block_id: block.block_id.clone(),
                confidence: 0.75,
            });
            hypotheses.push(Hypothesis {
                hypothesis_id,
                statement: format!("{} behaves as executable unit", block.block_id),
                score: 0.70,
                supporting_fact_ids: vec![fact_id],
                proof_type: "symbolic_frontier_hint".to_string(),
                admitted: false,
            });
        }
        Self {
            ir2_id: format!("ir2:{}", ir1.ir1_id),
            ir1_id: ir1.ir1_id.clone(),
            fact_ledger: FactLedger { facts },
            hypothesis_ledger: HypothesisLedger { hypotheses },
            ontology: CanonicalOntology::default(),
        }
    }

    pub fn admit_hypotheses(&mut self, threshold: f64) -> Result<usize, String> {
        if !(0.0..=1.0).contains(&threshold) {
            return Err("ir2_invalid_threshold".to_string());
        }
        let mut admitted_count = 0usize;
        let known_fact_ids: BTreeSet<String> = self
            .fact_ledger
            .facts
            .iter()
            .map(|fact| fact.fact_id.clone())
            .collect();
        for hypothesis in &mut self.hypothesis_ledger.hypotheses {
            let has_support = hypothesis
                .supporting_fact_ids
                .iter()
                .all(|fact_id| known_fact_ids.contains(fact_id));
            hypothesis.admitted = has_support && hypothesis.score >= threshold;
            if hypothesis.admitted {
                admitted_count = admitted_count.saturating_add(1);
                self.ontology.concepts.push(CanonicalConcept {
                    concept_id: format!("concept-{}", hypothesis.hypothesis_id),
                    label: hypothesis.statement.clone(),
                    supporting_hypothesis_ids: vec![hypothesis.hypothesis_id.clone()],
                    confidence: hypothesis.score,
                    proof_type: hypothesis.proof_type.clone(),
                });
            }
        }
        Ok(admitted_count)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.fact_ledger.facts.is_empty() {
            return Err("ir2_missing_fact_ledger".to_string());
        }
        if self.hypothesis_ledger.hypotheses.is_empty() {
            return Err("ir2_missing_hypothesis_ledger".to_string());
        }
        let admitted: BTreeSet<String> = self
            .hypothesis_ledger
            .hypotheses
            .iter()
            .filter(|hyp| hyp.admitted)
            .map(|hyp| hyp.hypothesis_id.clone())
            .collect();
        for concept in &self.ontology.concepts {
            if concept.supporting_hypothesis_ids.is_empty() {
                return Err(format!("ir2_ontology_empty_support:{}", concept.concept_id));
            }
            let all_admitted = concept
                .supporting_hypothesis_ids
                .iter()
                .all(|id| admitted.contains(id));
            if !all_admitted {
                return Err(format!(
                    "ir2_ontology_support_not_admitted:{}",
                    concept.concept_id
                ));
            }
        }
        Ok(())
    }
}
