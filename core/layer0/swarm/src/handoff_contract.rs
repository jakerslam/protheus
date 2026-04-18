use crate::typed_channels::{SwarmMessage, TypedHandoffEnvelope};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::marker::PhantomData;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HandoffError {
    pub code: String,
    pub detail: String,
}

impl HandoffError {
    pub fn new(code: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            detail: detail.into(),
        }
    }
}

pub trait HandoffContract {
    type Message: SwarmMessage;
    const CONTRACT_ID: &'static str;

    fn validate_handoff(_from_agent: &str, _to_agent: &str, _payload: &Self::Message) -> Result<(), HandoffError> {
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HandoffToken<C: HandoffContract> {
    pub from_agent: String,
    pub to_agent: String,
    pub payload: C::Message,
    pub issued_unix_ms: u64,
    pub handoff_digest: String,
    #[serde(skip)]
    _contract: PhantomData<C>,
}

impl<C: HandoffContract> HandoffToken<C> {
    pub fn new(from_agent: &str, to_agent: &str, payload: C::Message) -> Result<Self, HandoffError> {
        let from_agent = normalized_agent(from_agent)?;
        let to_agent = normalized_agent(to_agent)?;
        C::validate_handoff(&from_agent, &to_agent, &payload)?;
        let issued_unix_ms = now_unix_ms();
        let handoff_digest = digest_token::<C>(&from_agent, &to_agent, &payload, issued_unix_ms)?;
        Ok(Self {
            from_agent,
            to_agent,
            payload,
            issued_unix_ms,
            handoff_digest,
            _contract: PhantomData,
        })
    }

    pub fn contract_id(&self) -> &'static str {
        C::CONTRACT_ID
    }

    pub fn into_envelope(self, channel_id: &str) -> Result<TypedHandoffEnvelope<C::Message>, HandoffError> {
        TypedHandoffEnvelope::new(
            channel_id,
            C::CONTRACT_ID,
            &self.from_agent,
            &self.to_agent,
            self.payload,
        )
        .map_err(|error| HandoffError::new("typed_envelope_invalid", error))
    }
}

fn digest_token<C: HandoffContract>(
    from_agent: &str,
    to_agent: &str,
    payload: &C::Message,
    issued_unix_ms: u64,
) -> Result<String, HandoffError> {
    let payload_json = serde_json::to_string(payload)
        .map_err(|error| HandoffError::new("payload_encode_failed", error.to_string()))?;
    let mut hasher = Sha256::new();
    hasher.update(C::CONTRACT_ID.as_bytes());
    hasher.update(from_agent.as_bytes());
    hasher.update(to_agent.as_bytes());
    hasher.update(payload_json.as_bytes());
    hasher.update(issued_unix_ms.to_le_bytes());
    Ok(hex::encode(hasher.finalize()))
}

fn normalized_agent(raw: &str) -> Result<String, HandoffError> {
    let cleaned = raw.trim();
    if cleaned.is_empty() {
        return Err(HandoffError::new("agent_id_required", "agent identifier is required"));
    }
    if cleaned.len() > 120 {
        return Err(HandoffError::new(
            "agent_id_too_long",
            "agent identifier must be <=120 chars",
        ));
    }
    Ok(cleaned.to_string())
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct ResearchTask {
        query: String,
        strict: bool,
    }

    struct ResearchHandoff;

    impl HandoffContract for ResearchHandoff {
        type Message = ResearchTask;
        const CONTRACT_ID: &'static str = "contract.research.v1";

        fn validate_handoff(
            _from_agent: &str,
            _to_agent: &str,
            payload: &Self::Message,
        ) -> Result<(), HandoffError> {
            if payload.query.trim().is_empty() {
                return Err(HandoffError::new("query_required", "query must be non-empty"));
            }
            Ok(())
        }
    }

    #[test]
    fn compile_time_handoff_contract_emits_typed_envelope() {
        let token = HandoffToken::<ResearchHandoff>::new(
            "planner",
            "worker-1",
            ResearchTask {
                query: "top ai safety papers".to_string(),
                strict: true,
            },
        )
        .expect("token");
        assert_eq!(token.contract_id(), "contract.research.v1");
        let envelope = token.into_envelope("swarm.research").expect("envelope");
        assert_eq!(envelope.contract_id, "contract.research.v1");
        assert_eq!(envelope.to_agent, "worker-1");
    }
}

