#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const MAX_ID_LEN: usize = 96;
const MAX_CLASS_LEN: usize = 96;
const MAX_PAYLOAD_REF_LEN: usize = 192;
const MAX_TS_MS: i64 = 9_999_999_999_999;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExoticDomain {
    Ternary,
    Quantum,
    Neural,
    Analog,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExoticEnvelope {
    pub domain: ExoticDomain,
    pub adapter_id: String,
    pub signal_type: String,
    pub payload_ref: String,
    pub ts_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Layer0Envelope {
    pub source_layer: String,
    pub adapter_id: String,
    pub capability_class: String,
    pub deterministic_digest: String,
    pub ts_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DegradationContract {
    pub primary: String,
    pub fallback: String,
    pub reason: String,
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

fn sanitize_token(raw: &str, max_len: usize, fallback: &str, lowercase: bool) -> String {
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
    if value.is_empty() {
        fallback.to_string()
    } else {
        value
    }
}

fn sanitize_identifier(raw: &str, max_len: usize, fallback: &str, lowercase: bool) -> String {
    let token = sanitize_token(raw, max_len, fallback, lowercase);
    let filtered: String = token
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.'))
        .collect();
    if filtered.is_empty() {
        fallback.to_string()
    } else {
        filtered
    }
}

fn normalize_timestamp(ts_ms: i64) -> i64 {
    ts_ms.clamp(0, MAX_TS_MS)
}

fn domain_tag(domain: &ExoticDomain) -> &'static str {
    match domain {
        ExoticDomain::Ternary => "ternary",
        ExoticDomain::Quantum => "quantum",
        ExoticDomain::Neural => "neural",
        ExoticDomain::Analog => "analog",
        ExoticDomain::Unknown => "unknown",
    }
}

pub fn wrap_exotic_signal(env: &ExoticEnvelope, capability_class: &str) -> Layer0Envelope {
    let adapter_id = sanitize_identifier(&env.adapter_id, MAX_ID_LEN, "unknown_adapter", false);
    let signal_type = sanitize_identifier(&env.signal_type, MAX_ID_LEN, "unknown_signal", true);
    let payload_ref = {
        let normalized = sanitize_token(
            &env.payload_ref,
            MAX_PAYLOAD_REF_LEN,
            "blob://unknown",
            false,
        );
        if normalized.starts_with("blob://") {
            normalized
        } else {
            "blob://unknown".to_string()
        }
    };
    let capability_class =
        sanitize_identifier(capability_class, MAX_CLASS_LEN, "exotic.unknown", true);
    let ts_ms = normalize_timestamp(env.ts_ms);

    let mut hasher = Sha256::new();
    hasher.update(format!(
        "{}|{}|{}|{}|{}|{}",
        domain_tag(&env.domain),
        adapter_id,
        signal_type,
        payload_ref,
        ts_ms,
        capability_class
    ));
    let digest = format!("{:x}", hasher.finalize());
    Layer0Envelope {
        source_layer: "layer_minus_one".to_string(),
        adapter_id,
        capability_class,
        deterministic_digest: digest,
        ts_ms,
    }
}

pub fn default_degradation(domain: &ExoticDomain) -> DegradationContract {
    match domain {
        ExoticDomain::Quantum => DegradationContract {
            primary: "quantum_domain".to_string(),
            fallback: "classical_approximation".to_string(),
            reason: "qpu_unavailable_or_fidelity_below_gate".to_string(),
        },
        ExoticDomain::Neural => DegradationContract {
            primary: "neural_io".to_string(),
            fallback: "standard_ui_io".to_string(),
            reason: "consent_kernel_unavailable".to_string(),
        },
        ExoticDomain::Ternary => DegradationContract {
            primary: "ternary_domain".to_string(),
            fallback: "binary_encoding".to_string(),
            reason: "no_ternary_backend".to_string(),
        },
        _ => DegradationContract {
            primary: "exotic_domain".to_string(),
            fallback: "binary_safe_mode".to_string(),
            reason: "unsupported_or_unknown_domain".to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrapper_is_deterministic() {
        let env = ExoticEnvelope {
            domain: ExoticDomain::Quantum,
            adapter_id: "qpu.ibm.sim".to_string(),
            signal_type: "measurement_batch".to_string(),
            payload_ref: "blob://abc".to_string(),
            ts_ms: 1_762_000_000_000,
        };
        let a = wrap_exotic_signal(&env, "measure.quantum");
        let b = wrap_exotic_signal(&env, "measure.quantum");
        assert_eq!(a, b);
    }
}
