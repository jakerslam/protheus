// SPDX-License-Identifier: Apache-2.0
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceBudget {
    pub cpu_quota_millis: u64,
    pub memory_quota_bytes: u64,
    pub io_quota_bytes: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceUsage {
    pub cpu_used_millis: u64,
    pub memory_used_bytes: u64,
    pub io_used_bytes: u64,
}

pub const EDGE_MEMORY_THRESHOLD_BYTES: u64 = 512 * 1024 * 1024;
pub const EDGE_CPU_CORE_THRESHOLD: u16 = 1;
pub const MAX_RESOURCE_QUOTA_CPU_MILLIS: u64 = 86_400_000;
pub const MAX_RESOURCE_QUOTA_MEMORY_BYTES: u64 = 8 * 1024 * 1024 * 1024 * 1024;
pub const MAX_RESOURCE_QUOTA_IO_BYTES: u64 = 16 * 1024 * 1024 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HardwareProfile {
    pub total_memory_bytes: u64,
    pub cpu_cores: u16,
    pub has_mmu: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InferenceBackend {
    Primary,
    Edge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendOverride {
    ForcePrimary,
    ForceEdge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackendSelectionReceipt {
    pub backend: InferenceBackend,
    pub constrained_hardware: bool,
    pub reason: &'static str,
}

fn normalize_budget(budget: ResourceBudget) -> Option<ResourceBudget> {
    if budget.cpu_quota_millis == 0 || budget.memory_quota_bytes == 0 || budget.io_quota_bytes == 0
    {
        return None;
    }
    if budget.cpu_quota_millis > MAX_RESOURCE_QUOTA_CPU_MILLIS
        || budget.memory_quota_bytes > MAX_RESOURCE_QUOTA_MEMORY_BYTES
        || budget.io_quota_bytes > MAX_RESOURCE_QUOTA_IO_BYTES
    {
        return None;
    }
    Some(budget)
}

fn normalize_hardware_profile(profile: HardwareProfile) -> HardwareProfile {
    let normalized_memory = profile
        .total_memory_bytes
        .min(MAX_RESOURCE_QUOTA_MEMORY_BYTES);
    let normalized_cpu = profile.cpu_cores.max(1);
    HardwareProfile {
        total_memory_bytes: normalized_memory,
        cpu_cores: normalized_cpu,
        has_mmu: profile.has_mmu && normalized_memory > 0,
    }
}

impl ResourceBudget {
    pub fn allows(&self, usage: ResourceUsage) -> bool {
        let Some(normalized_budget) = normalize_budget(*self) else {
            return false;
        };

        usage.cpu_used_millis <= normalized_budget.cpu_quota_millis
            && usage.memory_used_bytes <= normalized_budget.memory_quota_bytes
            && usage.io_used_bytes <= normalized_budget.io_quota_bytes
            && usage.cpu_used_millis <= MAX_RESOURCE_QUOTA_CPU_MILLIS
            && usage.memory_used_bytes <= MAX_RESOURCE_QUOTA_MEMORY_BYTES
            && usage.io_used_bytes <= MAX_RESOURCE_QUOTA_IO_BYTES
    }
}

pub fn is_constrained_hardware(profile: HardwareProfile) -> bool {
    let profile = normalize_hardware_profile(profile);
    !profile.has_mmu
        || profile.total_memory_bytes < EDGE_MEMORY_THRESHOLD_BYTES
        || profile.cpu_cores <= EDGE_CPU_CORE_THRESHOLD
}

pub fn select_inference_backend(
    profile: HardwareProfile,
    backend_override: Option<BackendOverride>,
) -> BackendSelectionReceipt {
    let invalid_profile_input = profile.total_memory_bytes == 0 || profile.cpu_cores == 0;
    if invalid_profile_input {
        return BackendSelectionReceipt {
            backend: InferenceBackend::Edge,
            constrained_hardware: true,
            reason: "invalid_hardware_profile_fail_closed",
        };
    }
    let normalized_profile = normalize_hardware_profile(profile);
    let constrained = is_constrained_hardware(normalized_profile);
    if let Some(requested) = backend_override {
        return match requested {
            BackendOverride::ForcePrimary => BackendSelectionReceipt {
                backend: InferenceBackend::Primary,
                constrained_hardware: constrained,
                reason: "override_force_primary",
            },
            BackendOverride::ForceEdge => BackendSelectionReceipt {
                backend: InferenceBackend::Edge,
                constrained_hardware: constrained,
                reason: "override_force_edge",
            },
        };
    }

    if constrained {
        BackendSelectionReceipt {
            backend: InferenceBackend::Edge,
            constrained_hardware: true,
            reason: "tier_d_constrained_hardware",
        }
    } else {
        BackendSelectionReceipt {
            backend: InferenceBackend::Primary,
            constrained_hardware: false,
            reason: "default_primary_backend",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        select_inference_backend, BackendOverride, HardwareProfile, InferenceBackend,
        ResourceBudget, ResourceUsage,
    };

    #[test]
    fn budget_enforces_cpu_memory_and_io_quotas() {
        let budget = ResourceBudget {
            cpu_quota_millis: 500,
            memory_quota_bytes: 8 * 1024,
            io_quota_bytes: 4 * 1024,
        };

        let under_quota = ResourceUsage {
            cpu_used_millis: 350,
            memory_used_bytes: 6 * 1024,
            io_used_bytes: 1 * 1024,
        };
        assert!(budget.allows(under_quota));

        let over_cpu = ResourceUsage {
            cpu_used_millis: 501,
            memory_used_bytes: 6 * 1024,
            io_used_bytes: 1 * 1024,
        };
        assert!(!budget.allows(over_cpu));
    }

    #[test]
    fn constrained_hardware_defaults_to_edge_backend() {
        let profile = HardwareProfile {
            total_memory_bytes: 256 * 1024 * 1024,
            cpu_cores: 1,
            has_mmu: true,
        };
        let receipt = select_inference_backend(profile, None);
        assert_eq!(receipt.backend, InferenceBackend::Edge);
        assert!(receipt.constrained_hardware);
        assert_eq!(receipt.reason, "tier_d_constrained_hardware");
    }

    #[test]
    fn unconstrained_hardware_defaults_to_primary_backend() {
        let profile = HardwareProfile {
            total_memory_bytes: 8 * 1024 * 1024 * 1024,
            cpu_cores: 8,
            has_mmu: true,
        };
        let receipt = select_inference_backend(profile, None);
        assert_eq!(receipt.backend, InferenceBackend::Primary);
        assert!(!receipt.constrained_hardware);
        assert_eq!(receipt.reason, "default_primary_backend");
    }

    #[test]
    fn no_mmu_forces_edge_backend_when_no_override() {
        let profile = HardwareProfile {
            total_memory_bytes: 2 * 1024 * 1024 * 1024,
            cpu_cores: 4,
            has_mmu: false,
        };
        let receipt = select_inference_backend(profile, None);
        assert_eq!(receipt.backend, InferenceBackend::Edge);
        assert!(receipt.constrained_hardware);
    }

    #[test]
    fn explicit_override_force_primary_wins_for_testing() {
        let profile = HardwareProfile {
            total_memory_bytes: 128 * 1024 * 1024,
            cpu_cores: 1,
            has_mmu: false,
        };
        let receipt = select_inference_backend(profile, Some(BackendOverride::ForcePrimary));
        assert_eq!(receipt.backend, InferenceBackend::Primary);
        assert!(receipt.constrained_hardware);
        assert_eq!(receipt.reason, "override_force_primary");
    }

    #[test]
    fn explicit_override_force_edge_supported_on_unconstrained_hosts() {
        let profile = HardwareProfile {
            total_memory_bytes: 16 * 1024 * 1024 * 1024,
            cpu_cores: 12,
            has_mmu: true,
        };
        let receipt = select_inference_backend(profile, Some(BackendOverride::ForceEdge));
        assert_eq!(receipt.backend, InferenceBackend::Edge);
        assert!(!receipt.constrained_hardware);
        assert_eq!(receipt.reason, "override_force_edge");
    }

    #[test]
    fn invalid_profile_input_fails_closed_to_edge() {
        let profile = HardwareProfile {
            total_memory_bytes: 0,
            cpu_cores: 0,
            has_mmu: true,
        };
        let receipt = select_inference_backend(profile, Some(BackendOverride::ForcePrimary));
        assert_eq!(receipt.backend, InferenceBackend::Edge);
        assert!(receipt.constrained_hardware);
        assert_eq!(receipt.reason, "invalid_hardware_profile_fail_closed");
    }
}
