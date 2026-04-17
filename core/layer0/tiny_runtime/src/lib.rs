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

pub fn bounded_channel_retry_attempts(requested_attempts: u16) -> u16 {
    requested_attempts.clamp(1, 8)
}

pub fn should_retry_channel_status(status_code: u16) -> bool {
    matches!(status_code, 408 | 409 | 425 | 429 | 500 | 502 | 503 | 504)
}

pub fn normalized_proxy_mode(enabled: bool, trusted_env_proxy: bool) -> u8 {
    if !enabled {
        0
    } else if trusted_env_proxy {
        2
    } else {
        1
    }
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

    #[test]
    fn retry_attempts_are_bounded() {
        assert_eq!(bounded_channel_retry_attempts(0), 1);
        assert_eq!(bounded_channel_retry_attempts(3), 3);
        assert_eq!(bounded_channel_retry_attempts(99), 8);
    }

    #[test]
    fn retry_status_policy_matches_transient_failures() {
        assert!(should_retry_channel_status(429));
        assert!(should_retry_channel_status(503));
        assert!(!should_retry_channel_status(401));
    }

    #[test]
    fn proxy_mode_encodes_strict_and_trusted() {
        assert_eq!(normalized_proxy_mode(false, false), 0);
        assert_eq!(normalized_proxy_mode(true, false), 1);
        assert_eq!(normalized_proxy_mode(true, true), 2);
    }
}
