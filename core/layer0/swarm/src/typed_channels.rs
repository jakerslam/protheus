use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::any::type_name;
use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

pub trait SwarmMessage: Serialize + DeserializeOwned + Clone + Send + Sync + 'static {
    fn schema_id() -> &'static str {
        type_name::<Self>()
    }
}

impl<T> SwarmMessage for T where T: Serialize + DeserializeOwned + Clone + Send + Sync + 'static {}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct TypedHandoffEnvelope<T: SwarmMessage> {
    pub channel_id: String,
    pub contract_id: String,
    pub from_agent: String,
    pub to_agent: String,
    pub payload: T,
    pub payload_schema: String,
    pub created_unix_ms: u64,
    pub digest: String,
}

impl<T: SwarmMessage> TypedHandoffEnvelope<T> {
    pub fn new(
        channel_id: &str,
        contract_id: &str,
        from_agent: &str,
        to_agent: &str,
        payload: T,
    ) -> Result<Self, String> {
        let channel_id = normalized_token(channel_id, "channel_id")?;
        let contract_id = normalized_token(contract_id, "contract_id")?;
        let from_agent = normalized_token(from_agent, "from_agent")?;
        let to_agent = normalized_token(to_agent, "to_agent")?;
        let created_unix_ms = now_unix_ms();
        let payload_schema = T::schema_id().to_string();
        let digest = envelope_digest(
            &channel_id,
            &contract_id,
            &from_agent,
            &to_agent,
            &payload,
            created_unix_ms,
        )?;
        Ok(Self {
            channel_id,
            contract_id,
            from_agent,
            to_agent,
            payload,
            payload_schema,
            created_unix_ms,
            digest,
        })
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct TypedChannel<T: SwarmMessage> {
    pub channel_id: String,
    pub message_schema: String,
    pub queue: Vec<TypedHandoffEnvelope<T>>,
    pub delivered: u64,
}

impl<T: SwarmMessage> TypedChannel<T> {
    pub fn new(channel_id: &str) -> Result<Self, String> {
        let channel_id = normalized_token(channel_id, "channel_id")?;
        Ok(Self {
            channel_id,
            message_schema: T::schema_id().to_string(),
            queue: Vec::new(),
            delivered: 0,
        })
    }

    pub fn publish(&mut self, envelope: TypedHandoffEnvelope<T>) -> Result<(), String> {
        if envelope.channel_id != self.channel_id {
            return Err("channel_id_mismatch".to_string());
        }
        if envelope.payload_schema != self.message_schema {
            return Err("payload_schema_mismatch".to_string());
        }
        self.queue.push(envelope);
        Ok(())
    }

    pub fn drain_for(&mut self, agent_id: &str) -> Vec<TypedHandoffEnvelope<T>> {
        let target = agent_id.trim();
        if target.is_empty() {
            return Vec::new();
        }
        let mut keep = Vec::<TypedHandoffEnvelope<T>>::new();
        let mut out = Vec::<TypedHandoffEnvelope<T>>::new();
        for envelope in self.queue.drain(..) {
            if envelope.to_agent == target {
                self.delivered = self.delivered.saturating_add(1);
                out.push(envelope);
            } else {
                keep.push(envelope);
            }
        }
        self.queue = keep;
        out
    }

    pub fn pending(&self) -> usize {
        self.queue.len()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct TypedChannelCatalog {
    pub channels: BTreeMap<String, String>,
}

impl TypedChannelCatalog {
    pub fn register<T: SwarmMessage>(&mut self, channel: &TypedChannel<T>) {
        self.channels
            .insert(channel.channel_id.clone(), channel.message_schema.clone());
    }

    pub fn schema_for(&self, channel_id: &str) -> Option<&String> {
        self.channels.get(channel_id)
    }
}

fn envelope_digest<T: SwarmMessage>(
    channel_id: &str,
    contract_id: &str,
    from_agent: &str,
    to_agent: &str,
    payload: &T,
    created_unix_ms: u64,
) -> Result<String, String> {
    let payload_json = serde_json::to_string(payload).map_err(|e| format!("payload_encode_failed:{e}"))?;
    let mut hasher = Sha256::new();
    hasher.update(channel_id.as_bytes());
    hasher.update(contract_id.as_bytes());
    hasher.update(from_agent.as_bytes());
    hasher.update(to_agent.as_bytes());
    hasher.update(payload_json.as_bytes());
    hasher.update(created_unix_ms.to_le_bytes());
    Ok(hex::encode(hasher.finalize()))
}

fn normalized_token(raw: &str, field: &str) -> Result<String, String> {
    let cleaned = raw.trim();
    if cleaned.is_empty() {
        return Err(format!("{field}_required"));
    }
    if cleaned.len() > 160 {
        return Err(format!("{field}_too_long"));
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
    struct DemoMsg {
        task_id: String,
        attempt: u32,
    }

    #[test]
    fn typed_channel_publish_and_drain_contract() {
        let mut channel = TypedChannel::<DemoMsg>::new("swarm.research").expect("channel");
        let envelope = TypedHandoffEnvelope::new(
            "swarm.research",
            "contract.demo",
            "planner",
            "worker-a",
            DemoMsg {
                task_id: "T-1".to_string(),
                attempt: 1,
            },
        )
        .expect("envelope");
        channel.publish(envelope).expect("publish");
        assert_eq!(channel.pending(), 1);
        let drained = channel.drain_for("worker-a");
        assert_eq!(drained.len(), 1);
        assert_eq!(channel.pending(), 0);
        assert_eq!(channel.delivered, 1);
    }
}
