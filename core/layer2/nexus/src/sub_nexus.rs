use crate::registry::{ModuleKind, SubNexusRegistration};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubNexus {
    pub sub_nexus_id: String,
    pub module_kind: ModuleKind,
    pub local_delivery_count: u64,
    pub cross_module_delivery_count: u64,
}

impl SubNexus {
    pub fn from_registration(registration: &SubNexusRegistration) -> Self {
        Self {
            sub_nexus_id: registration.sub_nexus_id.clone(),
            module_kind: registration.module_kind.clone(),
            local_delivery_count: 0,
            cross_module_delivery_count: 0,
        }
    }

    pub fn record_local_delivery(&mut self) {
        self.local_delivery_count = self.local_delivery_count.saturating_add(1);
    }

    pub fn record_cross_module_delivery(&mut self) {
        self.cross_module_delivery_count = self.cross_module_delivery_count.saturating_add(1);
    }

    pub fn local_resolution_ratio(&self) -> f64 {
        let total = self
            .local_delivery_count
            .saturating_add(self.cross_module_delivery_count);
        if total == 0 {
            return 1.0;
        }
        self.local_delivery_count as f64 / total as f64
    }
}
