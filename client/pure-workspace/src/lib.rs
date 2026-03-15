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

pub fn profile() -> PureWorkspaceProfile {
    PURE_WORKSPACE_PROFILE
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
}
