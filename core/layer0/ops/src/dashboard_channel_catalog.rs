// Layer ownership: core/layer0/ops (authoritative)
// SPDX-License-Identifier: Apache-2.0
use serde_json::{json, Value};

fn entry(name: &str, display_name: &str, category: &str, setup_type: &str) -> Value {
    if let Some(row) = crate::reference_parity_catalog::channel_catalog_entry(
        name,
        display_name,
        category,
        setup_type,
    ) {
        return row;
    }
    let (icon, difficulty, setup_time) = match category {
        "enterprise" => ("🏢", "Medium", "3-8 min"),
        "email" => ("✉️", "Easy", "2-4 min"),
        "social" => ("📣", "Easy", "2-5 min"),
        "streaming" => ("📺", "Medium", "4-10 min"),
        "community" => ("💬", "Medium", "3-8 min"),
        "developer" => ("🛠️", "Medium", "3-8 min"),
        "notifications" => ("🔔", "Easy", "2-5 min"),
        "messaging" => ("💬", "Easy", "2-5 min"),
        _ => ("🔗", "Easy", "2-5 min"),
    };
    let mut runtime_adapter = "generic_http";
    let mut runtime_mode = "generic_template";
    let mut channel_tier = "template";
    let mut requires_token = true;
    let mut supports_send = true;
    let mut probe_method = "get";

    let (description, quick_setup, fields, setup_steps, config_template) = if name == "gohighlevel"
    {
        runtime_adapter = "gohighlevel_api";
        runtime_mode = "native";
        channel_tier = "native";
        (
            "Connect GoHighLevel (HighLevel API 2.0) for CRM contacts, messaging, and workflow automation.".to_string(),
            "Create a Private Integration Token (PIT), add the location ID, then validate with a live API check.".to_string(),
            json!([
                {"key": "private_integration_token", "label": "Private Integration Token (PIT)", "type": "secret", "advanced": false, "placeholder": "pit-..."},
                {"key": "location_id", "label": "Location ID", "type": "text", "advanced": false, "placeholder": "ve9EPM428h8vShlRW1KT"},
                {"key": "api_version", "label": "API Version", "type": "text", "advanced": true, "placeholder": "2021-07-28"},
                {"key": "endpoint", "label": "API Endpoint", "type": "text", "advanced": true, "placeholder": "https://services.leadconnectorhq.com"}
            ]),
            json!([
                "In GoHighLevel, create a Private Integration with required scopes.",
                "Paste PIT + location ID in channel settings.",
                "Run test to verify /locations/{locationId} access."
            ]),
            "PRIVATE_INTEGRATION_TOKEN=pit-...\\nLOCATION_ID=ve9EPM428h8vShlRW1KT\\nAPI_VERSION=2021-07-28\\nENDPOINT=https://services.leadconnectorhq.com".to_string(),
        )
    } else if name == "whatsapp" {
        runtime_adapter = "whatsapp_qr";
        runtime_mode = "native";
        channel_tier = "native";
        requires_token = false;
        supports_send = false;
        probe_method = "internal";
        (
            "Connect WhatsApp using QR pairing with automatic session checks.".to_string(),
            "Start QR pairing, scan from WhatsApp mobile, then verify connected status.".to_string(),
            json!([
                {"key": "token", "label": "Business API Token (optional)", "type": "secret", "advanced": true, "placeholder": "••••••"},
                {"key": "endpoint", "label": "Business API Endpoint (optional)", "type": "text", "advanced": true, "placeholder": "https://graph.facebook.com"}
            ]),
            json!([
                "Click Start QR in channel setup.",
                "Scan from WhatsApp mobile linked devices.",
                "Wait for connected state in dashboard."
            ]),
            "QR_PAIRING=true\\nBUSINESS_API_TOKEN=... (optional)\\nBUSINESS_API_ENDPOINT=https://graph.facebook.com (optional)".to_string(),
        )
    } else if name == "telegram" {
        runtime_adapter = "telegram_bot";
        runtime_mode = "native";
        channel_tier = "native";
        (
            "Connect Telegram bot API with live getMe verification and send support.".to_string(),
            "Add bot token, optional chat_id, then run live test.".to_string(),
            json!([
                {"key": "token", "label": "Bot Token", "type": "secret", "advanced": false, "placeholder": "123456:ABCDEF..."},
                {"key": "chat_id", "label": "Default Chat ID", "type": "text", "advanced": true, "placeholder": "-1001234567890"}
            ]),
            json!([
                "Create Telegram bot via @BotFather.",
                "Paste bot token (and optional default chat_id).",
                "Run live test to verify token via getMe."
            ]),
            "TOKEN=123456:ABCDEF...\\nCHAT_ID=-1001234567890".to_string(),
        )
    } else if name == "slack" {
        runtime_adapter = "slack_bot";
        runtime_mode = "native";
        channel_tier = "native";
        (
            "Connect Slack bot token with live auth.test verification.".to_string(),
            "Paste bot token and optional default channel, then run live test.".to_string(),
            json!([
                {"key": "token", "label": "Bot/User Token", "type": "secret", "advanced": false, "placeholder": "xoxb-..."},
                {"key": "channel", "label": "Default Channel", "type": "text", "advanced": true, "placeholder": "#ops"},
                {"key": "endpoint", "label": "API Endpoint", "type": "text", "advanced": true, "placeholder": "https://slack.com/api"}
            ]),
            json!([
                "Create/install Slack app and grant chat scopes.",
                "Paste token and optional default channel.",
                "Run live test to verify auth.test."
            ]),
            "TOKEN=xoxb-...\\nCHANNEL=#ops\\nENDPOINT=https://slack.com/api".to_string(),
        )
    } else if name == "discord" {
        runtime_adapter = "discord_bot";
        runtime_mode = "native";
        channel_tier = "native";
        (
            "Connect Discord bot token with live /users/@me verification.".to_string(),
            "Paste bot token and optional channel_id, then run live test.".to_string(),
            json!([
                {"key": "bot_token", "label": "Bot Token", "type": "secret", "advanced": false, "placeholder": "••••••"},
                {"key": "channel_id", "label": "Default Channel ID", "type": "text", "advanced": true, "placeholder": "123456789012345678"},
                {"key": "endpoint", "label": "API Endpoint", "type": "text", "advanced": true, "placeholder": "https://discord.com/api/v10"}
            ]),
            json!([
                "Create Discord app and bot token.",
                "Paste token and optional channel_id.",
                "Run live test to verify /users/@me."
            ]),
            "BOT_TOKEN=...\\nCHANNEL_ID=123456789012345678\\nENDPOINT=https://discord.com/api/v10"
                .to_string(),
        )
    } else if matches!(
        name,
        "discord_webhook"
            | "slack_webhook"
            | "ntfy"
            | "gotify"
            | "ifttt"
            | "statuspage"
            | "pagerduty_events"
            | "opsgenie_alerts"
    ) {
        runtime_adapter = "webhook_http";
        runtime_mode = "native";
        channel_tier = "native";
        requires_token = false;
        probe_method = "post";
        (
            format!(
                "Connect {} using an inbound webhook endpoint.",
                display_name
            ),
            "Paste webhook URL and run live test ping.".to_string(),
            json!([
                {"key": "webhook_url", "label": "Webhook URL", "type": "text", "advanced": false, "placeholder": "https://..."},
                {"key": "token", "label": "Optional Token/Secret", "type": "secret", "advanced": true, "placeholder": "••••••"}
            ]),
            json!([
                "Generate webhook URL in target platform.",
                "Paste webhook URL (and optional token/secret).",
                "Run live test ping."
            ]),
            "WEBHOOK_URL=https://...\\nTOKEN=... (optional)".to_string(),
        )
    } else if name == "webchat" {
        runtime_adapter = "webchat_internal";
        runtime_mode = "native";
        channel_tier = "native";
        requires_token = false;
        probe_method = "internal";
        (
            "Built-in dashboard chat channel (always local and available).".to_string(),
            "No credentials required. This channel is always available.".to_string(),
            json!([]),
            json!(["No setup needed."]),
            "No config required.".to_string(),
        )
    } else {
        (
            format!(
                "Connect {} channel adapter for agent messaging with live HTTP probe.",
                display_name
            ),
            format!(
                "Configure credentials for {} and validate connectivity with a live endpoint probe.",
                display_name
            ),
            json!([
                {"key": "token", "label": "Token", "type": "secret", "advanced": false, "placeholder": "••••••"},
                {"key": "endpoint", "label": "Endpoint/Room/Channel", "type": "text", "advanced": true, "placeholder": "https://api.example.com"}
            ]),
            json!([
                "Create app/bot credentials",
                "Paste credentials + endpoint",
                "Run live connection test"
            ]),
            "TOKEN=...\\nENDPOINT=https://api.example.com".to_string(),
        )
    };
    json!({
        "name": name,
        "icon": icon,
        "display_name": display_name,
        "description": description,
        "quick_setup": quick_setup,
        "category": category,
        "difficulty": difficulty,
        "setup_time": setup_time,
        "setup_type": setup_type,
        "runtime_adapter": runtime_adapter,
        "runtime_mode": runtime_mode,
        "channel_tier": channel_tier,
        "real_channel": channel_tier == "native",
        "runtime_supported": true,
        "requires_token": requires_token,
        "supports_send": supports_send,
        "probe_method": probe_method,
        "live_probe_required_for_ready": true,
        "has_token": false,
        "configured": false,
        "fields": fields,
        "setup_steps": setup_steps,
        "config_template": config_template
    })
}

pub fn catalog() -> Vec<Value> {
    let defs = [
        // Core messaging + collaboration
        ("whatsapp", "WhatsApp", "messaging", "qr"),
        ("signal", "Signal", "messaging", "form"),
        ("telegram", "Telegram", "messaging", "form"),
        ("discord", "Discord", "messaging", "form"),
        ("slack", "Slack", "enterprise", "form"),
        ("teams", "Microsoft Teams", "enterprise", "form"),
        ("google_chat", "Google Chat", "enterprise", "oauth"),
        ("zoom_team_chat", "Zoom Team Chat", "enterprise", "oauth"),
        ("webex", "Webex", "enterprise", "oauth"),
        ("matrix", "Matrix", "community", "form"),
        ("xmpp", "XMPP", "community", "form"),
        ("irc", "IRC", "community", "form"),
        ("discourse", "Discourse", "community", "form"),
        ("guilded", "Guilded", "community", "oauth"),
        ("gitter", "Gitter", "community", "oauth"),
        ("keybase", "Keybase", "community", "form"),
        ("nextcloud", "Nextcloud", "community", "oauth"),
        ("nostr", "Nostr", "community", "form"),
        ("pumble", "Pumble", "community", "oauth"),
        ("revolt", "Revolt", "community", "oauth"),
        ("threema", "Threema", "community", "form"),
        ("twist", "Twist", "community", "oauth"),
        ("wecom", "WeCom", "community", "oauth"),
        ("skype", "Skype", "messaging", "form"),
        ("imessage", "iMessage", "messaging", "form"),
        (
            "facebook_messenger",
            "Facebook Messenger",
            "messaging",
            "oauth",
        ),
        ("messenger", "Messenger", "messaging", "oauth"),
        ("flock", "Flock", "enterprise", "oauth"),
        ("kakao", "KakaoTalk", "messaging", "form"),
        ("qq", "QQ", "messaging", "form"),
        ("feishu", "Feishu", "enterprise", "oauth"),
        ("dingtalk", "DingTalk", "enterprise", "oauth"),
        ("dingtalk_stream", "DingTalk Stream", "enterprise", "oauth"),
        ("wechat", "WeChat", "messaging", "form"),
        ("line", "LINE", "messaging", "form"),
        ("viber", "Viber", "messaging", "form"),
        ("kik", "Kik", "messaging", "form"),
        ("webchat", "Web Chat", "messaging", "form"),
        ("sms_twilio", "SMS (Twilio)", "messaging", "form"),
        ("sms_telnyx", "SMS (Telnyx)", "messaging", "form"),
        ("twilio_voice", "Twilio Voice", "messaging", "form"),
        ("vonage_sms", "Vonage SMS", "messaging", "form"),
        // Email
        ("email", "Email SMTP/IMAP", "email", "form"),
        ("gmail", "Gmail", "email", "oauth"),
        ("outlook", "Outlook", "email", "oauth"),
        ("protonmail", "Proton Mail", "email", "form"),
        ("mailgun", "Mailgun", "email", "form"),
        ("sendgrid", "SendGrid", "email", "form"),
        ("postmark", "Postmark", "email", "form"),
        // Social
        ("mastodon", "Mastodon", "social", "form"),
        ("bluesky", "Bluesky", "social", "form"),
        ("reddit", "Reddit", "social", "oauth"),
        ("hackernews", "Hacker News", "social", "oauth"),
        ("linkedin", "LinkedIn", "social", "oauth"),
        ("x", "X / Twitter", "social", "oauth"),
        ("facebook", "Facebook", "social", "oauth"),
        ("instagram", "Instagram", "social", "oauth"),
        ("threads", "Threads", "social", "oauth"),
        // Streaming + media
        ("youtube", "YouTube", "streaming", "oauth"),
        ("twitch", "Twitch", "streaming", "oauth"),
        ("tiktok", "TikTok", "streaming", "oauth"),
        ("kick", "Kick", "streaming", "oauth"),
        ("rumble", "Rumble", "streaming", "oauth"),
        ("spotify", "Spotify", "streaming", "oauth"),
        ("apple_music", "Apple Music", "streaming", "oauth"),
        ("mumble", "Mumble", "streaming", "form"),
        ("mqtt", "MQTT", "streaming", "form"),
        ("snapchat", "Snapchat", "social", "oauth"),
        // Enterprise PM + docs + repos
        ("mattermost", "Mattermost", "enterprise", "form"),
        ("zulip", "Zulip", "enterprise", "form"),
        ("rocketchat", "Rocket.Chat", "enterprise", "form"),
        ("rocket_chat", "Rocket.Chat", "enterprise", "form"),
        ("linear", "Linear", "enterprise", "oauth"),
        ("asana", "Asana", "enterprise", "oauth"),
        ("clickup", "ClickUp", "enterprise", "oauth"),
        ("monday", "Monday.com", "enterprise", "oauth"),
        ("trello", "Trello", "enterprise", "oauth"),
        ("airtable", "Airtable", "enterprise", "oauth"),
        ("servicenow", "ServiceNow", "enterprise", "oauth"),
        ("gohighlevel", "GoHighLevel", "enterprise", "form"),
        ("pagerduty", "PagerDuty", "enterprise", "form"),
        ("opsgenie", "Opsgenie", "enterprise", "form"),
        ("jira", "Jira", "enterprise", "oauth"),
        ("confluence", "Confluence", "enterprise", "oauth"),
        ("notion", "Notion", "enterprise", "oauth"),
        ("github", "GitHub", "enterprise", "oauth"),
        ("gitlab", "GitLab", "enterprise", "oauth"),
        ("bitbucket", "Bitbucket", "enterprise", "oauth"),
        // Developer + observability
        ("sentry", "Sentry", "developer", "form"),
        ("datadog", "Datadog", "developer", "form"),
        ("new_relic", "New Relic", "developer", "form"),
        ("grafana", "Grafana", "developer", "form"),
        ("splunk", "Splunk", "developer", "form"),
        ("sumologic", "Sumo Logic", "developer", "form"),
        ("cloudwatch", "AWS CloudWatch", "developer", "oauth"),
        ("zapier", "Zapier", "developer", "oauth"),
        ("make", "Make.com", "developer", "oauth"),
        // Notifications + alert fanout
        ("aws_sns", "AWS SNS", "notifications", "oauth"),
        (
            "azure_event_grid",
            "Azure Event Grid",
            "notifications",
            "oauth",
        ),
        ("gcp_pubsub", "GCP Pub/Sub", "notifications", "oauth"),
        ("pushover", "Pushover", "notifications", "form"),
        ("ntfy", "ntfy", "notifications", "form"),
        ("gotify", "Gotify", "notifications", "form"),
        ("ifttt", "IFTTT", "notifications", "oauth"),
        (
            "statuspage",
            "Atlassian Statuspage",
            "notifications",
            "oauth",
        ),
        (
            "pagerduty_events",
            "PagerDuty Events API",
            "notifications",
            "form",
        ),
        (
            "opsgenie_alerts",
            "Opsgenie Alerts",
            "notifications",
            "form",
        ),
        ("webhook", "Generic Webhook", "notifications", "form"),
        (
            "discord_webhook",
            "Discord Webhook",
            "notifications",
            "form",
        ),
        ("slack_webhook", "Slack Webhook", "notifications", "form"),
    ];
    defs.into_iter()
        .map(|(name, display, cat, setup)| entry(name, display, cat, setup))
        .collect::<Vec<_>>()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_has_broad_channel_coverage() {
        let rows = catalog();
        assert!(rows.len() >= 80);
    }

    #[test]
    fn catalog_marks_native_vs_template_channels() {
        let rows = catalog();
        let native = rows
            .iter()
            .filter(|row| {
                row.get("real_channel")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
            })
            .count();
        let template = rows
            .iter()
            .filter(|row| {
                row.get("channel_tier")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    == "template"
            })
            .count();
        assert!(
            native >= 10,
            "expected native channels to be explicitly marked"
        );
        assert!(
            template >= 10,
            "expected template channels to remain available"
        );
    }
}
