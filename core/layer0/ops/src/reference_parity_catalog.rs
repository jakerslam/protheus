// Layer ownership: core/layer0/ops (authoritative)
// SPDX-License-Identifier: Apache-2.0
use serde_json::{json, Value};

fn clean_text(raw: &str, max_len: usize) -> String {
    raw.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .chars()
        .take(max_len)
        .collect::<String>()
}

fn category_style(category: &str) -> (&'static str, &'static str, &'static str) {
    match category {
        "enterprise" => ("🏢", "Medium", "3-8 min"),
        "email" => ("✉️", "Easy", "2-4 min"),
        "social" => ("📣", "Easy", "2-5 min"),
        "streaming" => ("📺", "Medium", "4-10 min"),
        "community" => ("💬", "Medium", "3-8 min"),
        "developer" => ("🛠️", "Medium", "3-8 min"),
        "notifications" => ("🔔", "Easy", "2-5 min"),
        "messaging" => ("💬", "Easy", "2-5 min"),
        _ => ("🔗", "Easy", "2-5 min"),
    }
}

fn reference_runtime_adapter(name: &str) -> Option<&'static str> {
    match name {
        "signal" => Some("reference_runtime_signal"),
        "teams" => Some("reference_runtime_teams"),
        "matrix" => Some("reference_runtime_matrix"),
        "irc" => Some("reference_runtime_irc"),
        "email" => Some("reference_runtime_email"),
        "google_chat" => Some("reference_runtime_google_chat"),
        "mattermost" => Some("reference_runtime_mattermost"),
        "zulip" => Some("reference_runtime_zulip"),
        "rocketchat" => Some("reference_runtime_rocketchat"),
        "rocket_chat" => Some("reference_runtime_rocketchat"),
        "xmpp" => Some("reference_runtime_xmpp"),
        "bluesky" => Some("reference_runtime_bluesky"),
        "feishu" => Some("reference_runtime_feishu"),
        "line" => Some("reference_runtime_line"),
        "mastodon" => Some("reference_runtime_mastodon"),
        "messenger" => Some("reference_runtime_messenger"),
        "facebook_messenger" => Some("reference_runtime_messenger"),
        "reddit" => Some("reference_runtime_reddit"),
        "guilded" => Some("reference_runtime_guilded"),
        "nextcloud" => Some("reference_runtime_nextcloud"),
        "nostr" => Some("reference_runtime_nostr"),
        "revolt" => Some("reference_runtime_revolt"),
        "viber" => Some("reference_runtime_viber"),
        "webex" => Some("reference_runtime_webex"),
        "dingtalk" => Some("reference_runtime_dingtalk"),
        "dingtalk_stream" => Some("reference_runtime_dingtalk_stream"),
        "discourse" => Some("reference_runtime_discourse"),
        "gitter" => Some("reference_runtime_gitter"),
        "keybase" => Some("reference_runtime_keybase"),
        "linkedin" => Some("reference_runtime_linkedin"),
        "flock" => Some("reference_runtime_flock"),
        "mqtt" => Some("reference_runtime_mqtt"),
        "mumble" => Some("reference_runtime_mumble"),
        "pumble" => Some("reference_runtime_pumble"),
        "threema" => Some("reference_runtime_threema"),
        "twist" => Some("reference_runtime_twist"),
        "twitch" => Some("reference_runtime_twitch"),
        "webhook" => Some("reference_runtime_webhook"),
        "wecom" => Some("reference_runtime_wecom"),
        _ => None,
    }
}

fn adapter_label(name: &str, display_name: &str) -> String {
    match name {
        "rocketchat" => "Rocket.Chat".to_string(),
        "rocket_chat" => "Rocket.Chat".to_string(),
        "messenger" => "Messenger".to_string(),
        "facebook_messenger" => "Messenger".to_string(),
        _ => clean_text(display_name, 120),
    }
}

pub fn reference_runtime_native_channel_names() -> &'static [&'static str] {
    &[
        "signal",
        "teams",
        "matrix",
        "irc",
        "email",
        "google_chat",
        "mattermost",
        "zulip",
        "rocketchat",
        "rocket_chat",
        "xmpp",
        "bluesky",
        "feishu",
        "line",
        "mastodon",
        "messenger",
        "facebook_messenger",
        "reddit",
        "guilded",
        "nextcloud",
        "nostr",
        "revolt",
        "viber",
        "webex",
        "dingtalk",
        "dingtalk_stream",
        "discourse",
        "gitter",
        "keybase",
        "linkedin",
        "flock",
        "mqtt",
        "mumble",
        "pumble",
        "threema",
        "twist",
        "twitch",
        "webhook",
        "wecom",
    ]
}

pub fn channel_catalog_entry(
    name: &str,
    display_name: &str,
    category: &str,
    setup_type: &str,
) -> Option<Value> {
    let adapter = reference_runtime_adapter(name)?;
    let (icon, difficulty, setup_time) = category_style(category);
    let label = adapter_label(name, display_name);
    let requires_token = !matches!(name, "mqtt");
    let quick = if requires_token {
        format!("Connect {label} credentials, then run live test.")
    } else {
        format!("Configure {label} endpoint and run live test.")
    };
    let token_label = if name == "email" {
        "App Password / OAuth Token"
    } else if name == "mqtt" {
        "Optional Password"
    } else {
        "Bot Token / API Token"
    };
    let endpoint_label = if name == "email" {
        "Server / Domain"
    } else if name == "mqtt" {
        "Broker URL"
    } else {
        "Endpoint / Room / Channel"
    };
    Some(json!({
        "name": name,
        "icon": icon,
        "display_name": display_name,
        "description": format!("Reference Runtime-native {} adapter with governed live probe and receipts.", label),
        "quick_setup": quick,
        "category": category,
        "difficulty": difficulty,
        "setup_time": setup_time,
        "setup_type": setup_type,
        "runtime_adapter": adapter,
        "runtime_mode": "native",
        "channel_tier": "native",
        "real_channel": true,
        "runtime_supported": true,
        "requires_token": requires_token,
        "supports_send": true,
        "probe_method": "get",
        "live_probe_required_for_ready": true,
        "has_token": false,
        "configured": false,
        "fields": [
            {"key":"token","label":token_label,"type":"secret","advanced":false,"placeholder":"••••••"},
            {"key":"endpoint","label":endpoint_label,"type":"text","advanced":true,"placeholder":"https://api.example.com"}
        ],
        "setup_steps": [
            format!("Create {} app/bot credentials.", label),
            "Paste credentials and endpoint settings.",
            "Run live test to verify connectivity."
        ],
        "config_template": "TOKEN=...\\nENDPOINT=https://api.example.com"
    }))
}

fn extra_hand(
    id: &str,
    name: &str,
    icon: &str,
    category: &str,
    description: &str,
    tools: &[&str],
    role: &str,
    system_prompt: &str,
    fallback_provider: &str,
    fallback_model: &str,
    dashboard_key: &str,
) -> Value {
    json!({
        "id": id,
        "name": name,
        "icon": icon,
        "category": category,
        "description": description,
        "tools": tools,
        "agent": {
            "provider": fallback_provider,
            "model": fallback_model,
            "role": role,
            "system_prompt": system_prompt
        },
        "settings": [],
        "requirements": [],
        "dashboard": [
            {"memory_key": format!("{}_jobs_total", dashboard_key), "label": "Jobs", "format": "number"},
            {"memory_key": format!("{}_last_status", dashboard_key), "label": "Last Status", "format": "text"},
            {"memory_key": format!("{}_uptime", dashboard_key), "label": "Uptime", "format": "duration"}
        ]
    })
}

pub fn extra_hands(fallback_provider: &str, fallback_model: &str) -> Vec<Value> {
    vec![
        extra_hand(
            "clip",
            "Clip Hand",
            "🎬",
            "content",
            "Short-form clip pipeline for extracting and packaging high-signal segments.",
            &["video", "editor", "publish"],
            "clip-operator",
            "You are Clip Hand. Build concise clip workflows, preserve source provenance, and emit receipts.",
            fallback_provider,
            fallback_model,
            "clip_hand",
        ),
        extra_hand(
            "lead",
            "Lead Hand",
            "🧲",
            "data",
            "Lead discovery, qualification scoring, and CRM-ready routing with receipts.",
            &["crm", "prospecting", "scoring"],
            "lead-operator",
            "You are Lead Hand. Prioritize qualified opportunities and produce auditable lead scoring rationale.",
            fallback_provider,
            fallback_model,
            "lead_hand",
        ),
        extra_hand(
            "collector",
            "Collector Hand",
            "🛰️",
            "data",
            "Autonomous multi-source collection with source confidence and dedupe discipline.",
            &["collect", "normalize", "dedupe"],
            "collector-operator",
            "You are Collector Hand. Gather signal-rich data, dedupe aggressively, and preserve provenance.",
            fallback_provider,
            fallback_model,
            "collector_hand",
        ),
        extra_hand(
            "predictor",
            "Predictor Hand",
            "🔮",
            "analysis",
            "Forecasting and scenario testing with explicit confidence ranges.",
            &["forecast", "scenario", "risk"],
            "predictor-operator",
            "You are Predictor Hand. Produce calibrated forecasts with transparent assumptions and uncertainty.",
            fallback_provider,
            fallback_model,
            "predictor_hand",
        ),
        extra_hand(
            "researcher",
            "Researcher Hand",
            "📚",
            "research",
            "Deep research synthesis with ranked evidence and contradiction checks.",
            &["research", "citation", "synthesis"],
            "research-operator",
            "You are Researcher Hand. Deliver concise evidence-ranked research and note conflicts explicitly.",
            fallback_provider,
            fallback_model,
            "researcher_hand",
        ),
        extra_hand(
            "twitter",
            "Twitter Hand",
            "🐦",
            "communication",
            "Autonomous social drafting, scheduling, and engagement signal triage.",
            &["social", "draft", "schedule"],
            "social-operator",
            "You are Twitter Hand. Draft concise posts, optimize timing, and preserve brand-safe voice.",
            fallback_provider,
            fallback_model,
            "twitter_hand",
        ),
        extra_hand(
            "infisical-sync",
            "Infisical Sync Hand",
            "🔐",
            "security",
            "Secrets synchronization and drift checks across governed environments.",
            &["secrets", "sync", "audit"],
            "secrets-operator",
            "You are Infisical Sync Hand. Keep secrets synchronized, auditable, and fail-closed.",
            fallback_provider,
            fallback_model,
            "infisical_sync_hand",
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dingtalk_stream_is_in_native_catalog() {
        assert!(reference_runtime_native_channel_names()
            .iter()
            .any(|name| *name == "dingtalk_stream"));
        let entry =
            channel_catalog_entry("dingtalk_stream", "DingTalk Stream", "enterprise", "oauth")
                .expect("channel entry");
        assert_eq!(
            entry
                .get("runtime_adapter")
                .and_then(Value::as_str)
                .unwrap_or(""),
            "reference_runtime_dingtalk_stream"
        );
    }
}
