#![no_std]
// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/tiny_runtime (authoritative)

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct TinyRuntimeProfile {
    pub profile: &'static str,
    pub no_std: bool,
    pub max_heap_kib: u32,
    pub max_concurrent_hands: u16,
    pub supports_hibernation: bool,
    pub supports_receipt_batching: bool,
}

pub const TINY_PROFILE: TinyRuntimeProfile = TinyRuntimeProfile {
    profile: "tiny-embedded",
    no_std: true,
    max_heap_kib: 4096,
    max_concurrent_hands: 64,
    supports_hibernation: true,
    supports_receipt_batching: true,
};

pub fn tiny_profile() -> TinyRuntimeProfile {
    TINY_PROFILE
}

pub fn normalized_capacity_score(mem_kib: u32, hands: u16) -> u16 {
    let mem_cap = if mem_kib >= TINY_PROFILE.max_heap_kib {
        100u32
    } else {
        (mem_kib.saturating_mul(100)) / TINY_PROFILE.max_heap_kib.max(1)
    };
    let hand_cap = if hands >= TINY_PROFILE.max_concurrent_hands {
        100u32
    } else {
        (u32::from(hands).saturating_mul(100)) / u32::from(TINY_PROFILE.max_concurrent_hands.max(1))
    };
    ((mem_cap + hand_cap) / 2) as u16
}

#[cfg(test)]
extern crate std;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tiny_profile_is_no_std_and_bounded() {
        let profile = tiny_profile();
        assert!(profile.no_std);
        assert_eq!(profile.profile, "tiny-embedded");
        assert!(profile.max_heap_kib > 0);
    }

    #[test]
    fn capacity_score_saturates_at_100() {
        assert_eq!(normalized_capacity_score(10_000, 999), 100);
        assert!(normalized_capacity_score(1024, 16) < 100);
    }
}

