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

impl ResourceBudget {
    pub fn allows(&self, usage: ResourceUsage) -> bool {
        usage.cpu_used_millis <= self.cpu_quota_millis
            && usage.memory_used_bytes <= self.memory_quota_bytes
            && usage.io_used_bytes <= self.io_quota_bytes
    }
}

#[cfg(test)]
mod tests {
    use super::{ResourceBudget, ResourceUsage};

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
}
