// SPDX-License-Identifier: Apache-2.0
use sha2::{Digest, Sha256};

const MAX_CHANNEL_LEN: usize = 120;
const MAX_NONCE_LEN: usize = 160;
const MAX_ALLOWED_CHANNELS: usize = 256;
const MAX_POLICY_PAYLOAD_BYTES: usize = 8 * 1024 * 1024;
const MIN_TS_MILLIS: u64 = 1;
const MAX_TS_MILLIS: u64 = 9_999_999_999_999;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IpcEnvelope {
    pub channel: String,
    pub payload: Vec<u8>,
    pub nonce: String,
    pub ts_millis: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IpcPolicy {
    pub allowed_channels: Vec<String>,
    pub max_payload_bytes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IpcError {
    InvalidChannel,
    PayloadTooLarge,
    MissingNonce,
    InvalidTimestamp,
    InvalidPolicy,
}

fn strip_invisible_unicode(raw: &str) -> String {
    raw.chars()
        .filter(|ch| {
            !matches!(
                ch,
                '\u{200B}' | '\u{200C}' | '\u{200D}' | '\u{2060}' | '\u{FEFF}'
            )
        })
        .collect()
}

fn sanitize_token(raw: &str, max_len: usize, lowercase: bool) -> String {
    let mut value: String = strip_invisible_unicode(raw)
        .chars()
        .filter(|ch| !ch.is_control())
        .collect();
    value = value.trim().to_string();
    if lowercase {
        value = value.to_ascii_lowercase();
    }
    if value.chars().count() > max_len {
        value = value.chars().take(max_len).collect();
    }
    value
}

fn normalize_allowed_channels(raw_channels: &[String]) -> Vec<String> {
    let mut channels = raw_channels
        .iter()
        .map(|channel| sanitize_token(channel, MAX_CHANNEL_LEN, true))
        .filter(|channel| !channel.is_empty())
        .collect::<Vec<_>>();
    channels.sort();
    channels.dedup();
    channels
}

fn valid_nonce(raw: &str) -> bool {
    !raw.is_empty()
        && raw
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.'))
}

impl IpcPolicy {
    pub fn validate(&self, envelope: &IpcEnvelope) -> Result<(), IpcError> {
        if self.max_payload_bytes == 0 || self.max_payload_bytes > MAX_POLICY_PAYLOAD_BYTES {
            return Err(IpcError::InvalidPolicy);
        }
        let allowed_channels = normalize_allowed_channels(&self.allowed_channels);
        if allowed_channels.is_empty() || allowed_channels.len() > MAX_ALLOWED_CHANNELS {
            return Err(IpcError::InvalidPolicy);
        }
        if !(MIN_TS_MILLIS..=MAX_TS_MILLIS).contains(&envelope.ts_millis) {
            return Err(IpcError::InvalidTimestamp);
        }
        let nonce = sanitize_token(&envelope.nonce, MAX_NONCE_LEN, false);
        if !valid_nonce(&nonce) {
            return Err(IpcError::MissingNonce);
        }
        if envelope.payload.len() > self.max_payload_bytes {
            return Err(IpcError::PayloadTooLarge);
        }
        let channel = sanitize_token(&envelope.channel, MAX_CHANNEL_LEN, true);
        if channel.is_empty() {
            return Err(IpcError::InvalidChannel);
        }
        let channel_allowed = allowed_channels
            .iter()
            .any(|ch| ch.as_str() == channel.as_str());
        if !channel_allowed {
            return Err(IpcError::InvalidChannel);
        }
        Ok(())
    }
}

pub fn deterministic_envelope_hash(envelope: &IpcEnvelope) -> String {
    let channel = sanitize_token(&envelope.channel, MAX_CHANNEL_LEN, true);
    let nonce = sanitize_token(&envelope.nonce, MAX_NONCE_LEN, false);
    let mut hasher = Sha256::new();
    hasher.update(channel.as_bytes());
    hasher.update(b"|");
    hasher.update(nonce.as_bytes());
    hasher.update(b"|");
    hasher.update(envelope.ts_millis.to_string().as_bytes());
    hasher.update(b"|");
    hasher.update(envelope.payload.len().to_string().as_bytes());
    hasher.update(b"|");
    hasher.update(&envelope.payload);
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::{deterministic_envelope_hash, IpcEnvelope, IpcError, IpcPolicy};

    fn policy() -> IpcPolicy {
        IpcPolicy {
            allowed_channels: vec!["kernel.conduit".to_string(), "kernel.status".to_string()],
            max_payload_bytes: 1024,
        }
    }

    #[test]
    fn policy_allows_known_channel_and_bounded_payload() {
        let envelope = IpcEnvelope {
            channel: "kernel.conduit".to_string(),
            payload: b"{\"ok\":true}".to_vec(),
            nonce: "nonce-1".to_string(),
            ts_millis: 1_762_000_000_000,
        };
        assert!(policy().validate(&envelope).is_ok());
    }

    #[test]
    fn policy_rejects_unknown_channel() {
        let envelope = IpcEnvelope {
            channel: "external.unknown".to_string(),
            payload: b"{}".to_vec(),
            nonce: "nonce-2".to_string(),
            ts_millis: 1_762_000_000_001,
        };
        assert_eq!(policy().validate(&envelope), Err(IpcError::InvalidChannel));
    }

    #[test]
    fn policy_rejects_missing_nonce_and_oversized_payload() {
        let mut envelope = IpcEnvelope {
            channel: "kernel.conduit".to_string(),
            payload: vec![1u8; 2048],
            nonce: String::new(),
            ts_millis: 1_762_000_000_002,
        };
        assert_eq!(policy().validate(&envelope), Err(IpcError::MissingNonce));
        envelope.nonce = "nonce-3".to_string();
        assert_eq!(policy().validate(&envelope), Err(IpcError::PayloadTooLarge));
    }

    #[test]
    fn deterministic_hash_is_stable_for_same_envelope() {
        let envelope = IpcEnvelope {
            channel: "kernel.status".to_string(),
            payload: b"{\"mode\":\"run\"}".to_vec(),
            nonce: "nonce-4".to_string(),
            ts_millis: 1_762_000_000_003,
        };
        let h1 = deterministic_envelope_hash(&envelope);
        let h2 = deterministic_envelope_hash(&envelope);
        assert_eq!(h1, h2);
    }
}
