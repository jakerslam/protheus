// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer2/assimilation (authoritative).

use crate::ir2::CanonicalConcept;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DomainProjection {
    pub domain: String,
    pub artifact_hash: String,
    pub concept_count: usize,
    pub receipt_count: usize,
    pub notes: Vec<String>,
}

pub trait AssimilationCoreView {
    fn concepts_for_artifact(&self, artifact_hash: &str) -> Vec<CanonicalConcept>;
    fn receipt_lineage_for_artifact(&self, artifact_hash: &str) -> Vec<String>;
}

pub trait DomainWrapper {
    fn domain_name(&self) -> &'static str;
    fn project(
        &self,
        view: &dyn AssimilationCoreView,
        artifact_hash: &str,
    ) -> Result<DomainProjection, String>;
}

#[derive(Debug, Clone, Default)]
pub struct GameRemasterWrapper;

impl DomainWrapper for GameRemasterWrapper {
    fn domain_name(&self) -> &'static str {
        "game_remaster"
    }

    fn project(
        &self,
        view: &dyn AssimilationCoreView,
        artifact_hash: &str,
    ) -> Result<DomainProjection, String> {
        if artifact_hash.trim().is_empty() {
            return Err("domain_wrapper_missing_artifact_hash".to_string());
        }
        let concepts = view.concepts_for_artifact(artifact_hash);
        let receipts = view.receipt_lineage_for_artifact(artifact_hash);
        Ok(DomainProjection {
            domain: self.domain_name().to_string(),
            artifact_hash: artifact_hash.to_string(),
            concept_count: concepts.len(),
            receipt_count: receipts.len(),
            notes: vec![
                "Domain wrapper consumed canonical assimilation view only.".to_string(),
                "Code behavior lift remains in assimilation core.".to_string(),
            ],
        })
    }
}
