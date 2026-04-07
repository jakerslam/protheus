// SPDX-License-Identifier: Apache-2.0
// Layer ownership: client/pure-workspace (thin Rust client surface)

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PureWorkspaceProfile {
    pub mode: &'static str,
    pub rust_only: bool,
    pub conduit_required: bool,
    pub cold_start_target_ms: f64,
    pub idle_memory_target_mb: f64,
    pub install_size_target_mb: f64,
}

pub const PURE_WORKSPACE_PROFILE: PureWorkspaceProfile = PureWorkspaceProfile {
    mode: "pure-workspace",
    rust_only: true,
    conduit_required: true,
    cold_start_target_ms: 2.0,
    idle_memory_target_mb: 6.0,
    install_size_target_mb: 6.0,
};

pub const PURE_WORKSPACE_TINY_MAX_PROFILE: PureWorkspaceProfile = PureWorkspaceProfile {
    mode: "pure-workspace-tiny-max",
    rust_only: true,
    conduit_required: true,
    cold_start_target_ms: 4.5,
    idle_memory_target_mb: 1.4,
    install_size_target_mb: 1.5,
};

pub fn profile() -> PureWorkspaceProfile {
    PURE_WORKSPACE_PROFILE
}

pub fn tiny_max_profile() -> PureWorkspaceProfile {
    PURE_WORKSPACE_TINY_MAX_PROFILE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_targets_are_tiny() {
        let p = profile();
        assert!(p.rust_only);
        assert!(p.conduit_required);
        assert!(p.cold_start_target_ms <= 2.0);
        assert!(p.idle_memory_target_mb <= 6.0);
        assert!(p.install_size_target_mb <= 6.0);
    }

    #[test]
    fn tiny_max_profile_is_stricter_than_default_pure_gates() {
        let default = profile();
        let tiny_max = tiny_max_profile();
        assert!(tiny_max.rust_only);
        assert!(tiny_max.conduit_required);
        assert!(tiny_max.cold_start_target_ms <= 5.0);
        assert!(tiny_max.idle_memory_target_mb <= 2.0);
        assert!(tiny_max.install_size_target_mb <= default.install_size_target_mb);
    }
}
