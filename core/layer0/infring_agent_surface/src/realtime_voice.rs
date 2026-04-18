use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VoiceSessionRequest {
    pub provider: Option<String>,
    pub model: Option<String>,
    pub transport: String,
    pub vad_enabled: bool,
    pub room: Option<String>,
}

pub fn normalize_voice_session_request(raw: Option<&Value>) -> Option<VoiceSessionRequest> {
    let value = raw?;
    let transport = value
        .get("transport")
        .and_then(Value::as_str)
        .map(|item| item.trim().to_ascii_lowercase())
        .filter(|item| !item.is_empty())
        .unwrap_or_else(|| "webrtc".to_string());
    let valid_transport = matches!(transport.as_str(), "webrtc" | "livekit" | "ws");
    if !valid_transport {
        return None;
    }
    let provider = value
        .get("provider")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(str::to_string);
    let model = value
        .get("model")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(str::to_string);
    let room = value
        .get("room")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(str::to_string);
    let vad_enabled = value
        .get("vad_enabled")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    Some(VoiceSessionRequest {
        provider,
        model,
        transport,
        vad_enabled,
        room,
    })
}

pub fn voice_session_contract(
    request: &VoiceSessionRequest,
    voice_permission_allowed: bool,
) -> Value {
    if !voice_permission_allowed {
        return json!({
            "enabled": false,
            "error": "voice_permission_denied",
            "transport": request.transport,
        });
    }
    json!({
        "enabled": true,
        "provider": request.provider,
        "model": request.model,
        "transport": request.transport,
        "vad_enabled": request.vad_enabled,
        "room": request.room,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn voice_session_normalizes_transport() {
        let parsed = normalize_voice_session_request(Some(&json!({
            "transport": "LiveKit",
            "provider": "realtime",
            "vad_enabled": false
        })))
        .expect("voice request");
        assert_eq!(parsed.transport, "livekit");
        assert!(!parsed.vad_enabled);
    }

    #[test]
    fn voice_session_rejects_unknown_transport() {
        let parsed = normalize_voice_session_request(Some(&json!({
            "transport": "unsupported"
        })));
        assert!(parsed.is_none());
    }
}
