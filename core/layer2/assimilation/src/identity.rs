// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer2/assimilation (authoritative).

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AliasKind {
    FunctionClone,
    InlinedFragment,
    ThunkChain,
    CallingConventionNormalization,
    OverlaidMemoryRegion,
    ReusedBuffer,
    DataCodeAlias,
    SymbolRebase,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IdentityAlias {
    pub alias_id: String,
    pub canonical_id: String,
    pub kind: AliasKind,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct IdentityResolver {
    pub canonical_by_alias: BTreeMap<String, String>,
    pub aliases: Vec<IdentityAlias>,
}

impl IdentityResolver {
    pub fn register_alias(&mut self, alias: IdentityAlias) -> Result<(), String> {
        if let Some(existing) = self.canonical_by_alias.get(&alias.alias_id) {
            if existing != &alias.canonical_id {
                return Err(format!(
                    "identity_alias_conflict:{}:{}:{}",
                    alias.alias_id, existing, alias.canonical_id
                ));
            }
            return Ok(());
        }
        self.canonical_by_alias
            .insert(alias.alias_id.clone(), alias.canonical_id.clone());
        self.aliases.push(alias);
        Ok(())
    }

    pub fn canonical_for<'a>(&'a self, id: &'a str) -> &'a str {
        self.canonical_by_alias
            .get(id)
            .map(String::as_str)
            .unwrap_or(id)
    }

    pub fn collapse_duplicates(&self, ids: &[String]) -> Vec<String> {
        let mut out = Vec::new();
        for id in ids {
            let canonical = self.canonical_for(id).to_string();
            if !out.contains(&canonical) {
                out.push(canonical);
            }
        }
        out.sort();
        out
    }
}
