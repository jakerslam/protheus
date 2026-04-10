use crate::now_ms;
use crate::policy::{TrustClass, VerityClass};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ModuleKind {
    Stomach,
    ContextStacks,
    ClientIngress,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum ModuleLifecycleState {
    Active,
    Draining { drain_deadline_ms: u64 },
    Quiesced,
    Detached,
    Maintenance,
}

impl ModuleLifecycleState {
    pub fn accepts_new_leases(&self) -> bool {
        matches!(self, Self::Active)
    }

    pub fn accepts_payload_delivery(&self, now_ms: u64) -> bool {
        match self {
            Self::Active => true,
            Self::Draining { drain_deadline_ms } => now_ms <= *drain_deadline_ms,
            Self::Quiesced | Self::Detached | Self::Maintenance => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubNexusRegistration {
    pub sub_nexus_id: String,
    pub module_kind: ModuleKind,
    pub trust_class: TrustClass,
    pub verity_class: VerityClass,
    pub lifecycle: ModuleLifecycleState,
    pub registered_at_ms: u64,
    pub last_updated_ms: u64,
}

impl SubNexusRegistration {
    pub fn new(
        sub_nexus_id: impl Into<String>,
        module_kind: ModuleKind,
        trust_class: TrustClass,
        verity_class: VerityClass,
    ) -> Self {
        let ts = now_ms();
        Self {
            sub_nexus_id: sub_nexus_id.into(),
            module_kind,
            trust_class,
            verity_class,
            lifecycle: ModuleLifecycleState::Active,
            registered_at_ms: ts,
            last_updated_ms: ts,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NexusRegistry {
    registrations: BTreeMap<String, SubNexusRegistration>,
}

impl NexusRegistry {
    fn fail(reason: &str) -> Result<(), String> {
        Err(reason.to_string())
    }

    fn registration_mut(
        &mut self,
        sub_nexus_id: &str,
    ) -> Result<&mut SubNexusRegistration, String> {
        self.registrations
            .get_mut(sub_nexus_id)
            .ok_or_else(|| "registration_missing".to_string())
    }

    pub fn register(&mut self, registration: SubNexusRegistration) -> Result<(), String> {
        if registration.sub_nexus_id.trim().is_empty() {
            return Self::fail("registration_id_missing");
        }
        if self.registrations.contains_key(&registration.sub_nexus_id) {
            return Self::fail("registration_already_exists");
        }
        self.registrations
            .insert(registration.sub_nexus_id.clone(), registration);
        Ok(())
    }

    pub fn unregister(&mut self, sub_nexus_id: &str) -> Option<SubNexusRegistration> {
        self.registrations.remove(sub_nexus_id)
    }

    pub fn set_lifecycle(
        &mut self,
        sub_nexus_id: &str,
        lifecycle: ModuleLifecycleState,
    ) -> Result<(), String> {
        let entry = self.registration_mut(sub_nexus_id)?;
        entry.lifecycle = lifecycle;
        entry.last_updated_ms = now_ms();
        Ok(())
    }

    pub fn get(&self, sub_nexus_id: &str) -> Option<&SubNexusRegistration> {
        self.registrations.get(sub_nexus_id)
    }

    pub fn get_mut(&mut self, sub_nexus_id: &str) -> Option<&mut SubNexusRegistration> {
        self.registrations.get_mut(sub_nexus_id)
    }

    pub fn list(&self) -> Vec<SubNexusRegistration> {
        self.registrations.values().cloned().collect()
    }

    pub fn contains(&self, sub_nexus_id: &str) -> bool {
        self.registrations.contains_key(sub_nexus_id)
    }
}
