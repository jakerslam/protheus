use crate::handoff_contract::{HandoffContract, HandoffToken};
use crate::typed_channels::SwarmMessage;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;

pub const A2A_V1_PROTOCOL: &str = "a2a.v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum A2AIntent {
    Delegate,
    Reflect,
    Escalate,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct A2AEnvelope<T: SwarmMessage> {
    pub protocol_version: String,
    pub intent: A2AIntent,
    pub contract_id: String,
    pub trace_id: String,
    pub from_agent: String,
    pub to_agent: String,
    pub payload: T,
    pub handoff_digest: String,
    pub issued_unix_ms: u64,
    pub metadata: BTreeMap<String, String>,
}

impl<T: SwarmMessage> A2AEnvelope<T> {
    pub fn validate(&self) -> Result<(), String> {
        if self.protocol_version != A2A_V1_PROTOCOL {
            return Err("a2a_protocol_version_invalid".to_string());
        }
        if self.contract_id.trim().is_empty() {
            return Err("a2a_contract_id_required".to_string());
        }
        if self.trace_id.trim().is_empty() {
            return Err("a2a_trace_id_required".to_string());
        }
        if self.from_agent.trim().is_empty() || self.to_agent.trim().is_empty() {
            return Err("a2a_agent_id_required".to_string());
        }
        if self.handoff_digest.trim().is_empty() {
            return Err("a2a_handoff_digest_required".to_string());
        }
        Ok(())
    }
}

impl<T: SwarmMessage> A2AEnvelope<T> {
    pub fn from_handoff_token<C: HandoffContract<Message = T>>(
        intent: A2AIntent,
        trace_id: &str,
        token: &HandoffToken<C>,
    ) -> Result<Self, String> {
        let trace_id = trace_id.trim();
        if trace_id.is_empty() {
            return Err("a2a_trace_id_required".to_string());
        }
        let envelope = Self {
            protocol_version: A2A_V1_PROTOCOL.to_string(),
            intent,
            contract_id: C::CONTRACT_ID.to_string(),
            trace_id: trace_id.to_string(),
            from_agent: token.from_agent.clone(),
            to_agent: token.to_agent.clone(),
            payload: token.payload.clone(),
            handoff_digest: token.handoff_digest.clone(),
            issued_unix_ms: token.issued_unix_ms,
            metadata: BTreeMap::new(),
        };
        envelope.validate()?;
        Ok(envelope)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct A2AReceipt {
    pub protocol_version: String,
    pub status: String,
    pub error: Option<String>,
    pub trace_id: String,
    pub contract_id: String,
    pub from_agent: String,
    pub to_agent: String,
    pub handoff_digest: String,
}

pub fn a2a_receipt<T: SwarmMessage>(
    envelope: &A2AEnvelope<T>,
    status: &str,
    error: Option<&str>,
) -> A2AReceipt {
    A2AReceipt {
        protocol_version: A2A_V1_PROTOCOL.to_string(),
        status: status.trim().to_ascii_lowercase(),
        error: error.map(|value| value.trim().to_string()).filter(|value| !value.is_empty()),
        trace_id: envelope.trace_id.clone(),
        contract_id: envelope.contract_id.clone(),
        from_agent: envelope.from_agent.clone(),
        to_agent: envelope.to_agent.clone(),
        handoff_digest: envelope.handoff_digest.clone(),
    }
}

pub fn a2a_receipt_json<T: SwarmMessage>(
    envelope: &A2AEnvelope<T>,
    status: &str,
    error: Option<&str>,
) -> Value {
    let receipt = a2a_receipt(envelope, status, error);
    json!({
        "type": "a2a_receipt",
        "protocol": receipt.protocol_version,
        "status": receipt.status,
        "error": receipt.error,
        "trace_id": receipt.trace_id,
        "contract_id": receipt.contract_id,
        "from_agent": receipt.from_agent,
        "to_agent": receipt.to_agent,
        "handoff_digest": receipt.handoff_digest,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handoff_contract::{HandoffContract, HandoffError, HandoffToken};

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct TaskMsg {
        task_id: String,
    }

    struct TaskContract;

    impl HandoffContract for TaskContract {
        type Message = TaskMsg;
        const CONTRACT_ID: &'static str = "contract.task.v1";

        fn validate_handoff(
            _from_agent: &str,
            _to_agent: &str,
            payload: &Self::Message,
        ) -> Result<(), HandoffError> {
            if payload.task_id.trim().is_empty() {
                return Err(HandoffError::new("task_id_required", "task_id missing"));
            }
            Ok(())
        }
    }

    #[test]
    fn a2a_envelope_and_receipt_contract_is_stable() {
        let token = HandoffToken::<TaskContract>::new(
            "planner",
            "worker-a",
            TaskMsg {
                task_id: "T-42".to_string(),
            },
        )
        .expect("token");
        let envelope =
            A2AEnvelope::<TaskMsg>::from_handoff_token(A2AIntent::Delegate, "trace-1", &token)
                .expect("envelope");
        assert_eq!(envelope.protocol_version, A2A_V1_PROTOCOL);
        let receipt = a2a_receipt_json(&envelope, "ok", None);
        assert_eq!(receipt.get("type").and_then(Value::as_str), Some("a2a_receipt"));
        assert_eq!(
            receipt.get("contract_id").and_then(Value::as_str),
            Some("contract.task.v1")
        );
    }
}
