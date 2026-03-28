// Layer ownership: core/layer0/ops (authoritative)
// SPDX-License-Identifier: Apache-2.0
use serde_json::{json, Value};

fn entry(name: &str, display_name: &str, category: &str, setup_type: &str) -> Value {
    let (icon, difficulty, setup_time) = match category {
        "enterprise" => ("🏢", "Medium", "3-8 min"),
        "email" => ("✉️", "Easy", "2-4 min"),
        "social" => ("📣", "Easy", "2-5 min"),
        "streaming" => ("📺", "Medium", "4-10 min"),
        "community" => ("💬", "Medium", "3-8 min"),
        _ => ("🔗", "Easy", "2-5 min"),
    };
    json!({
        "name": name,
        "icon": icon,
        "display_name": display_name,
        "description": format!("Connect {} channel adapter for agent messaging.", display_name),
        "quick_setup": format!("Configure credentials for {} and validate connectivity.", display_name),
        "category": category,
        "difficulty": difficulty,
        "setup_time": setup_time,
        "setup_type": setup_type,
        "has_token": false,
        "configured": false,
        "fields": [
            {"key": "token", "label": "Token", "type": "secret", "advanced": false, "placeholder": "••••••"},
            {"key": "endpoint", "label": "Endpoint/Room/Channel", "type": "text", "advanced": true, "placeholder": "default"}
        ],
        "setup_steps": ["Create app/bot credentials", "Paste credentials", "Run connection test"],
        "config_template": "TOKEN=...\\nENDPOINT=..."
    })
}

pub fn catalog() -> Vec<Value> {
    let defs = [
        ("whatsapp", "WhatsApp", "messaging", "qr"),
        ("signal", "Signal", "messaging", "form"),
        ("telegram", "Telegram", "messaging", "form"),
        ("discord", "Discord", "messaging", "form"),
        ("slack", "Slack", "enterprise", "form"),
        ("teams", "Microsoft Teams", "enterprise", "form"),
        ("matrix", "Matrix", "community", "form"),
        ("xmpp", "XMPP", "community", "form"),
        ("irc", "IRC", "community", "form"),
        ("email", "Email SMTP/IMAP", "email", "form"),
        ("gmail", "Gmail", "email", "oauth"),
        ("outlook", "Outlook", "email", "oauth"),
        ("protonmail", "Proton Mail", "email", "form"),
        ("mastodon", "Mastodon", "social", "form"),
        ("bluesky", "Bluesky", "social", "form"),
        ("reddit", "Reddit", "social", "oauth"),
        ("linkedin", "LinkedIn", "social", "oauth"),
        ("x", "X / Twitter", "social", "oauth"),
        ("facebook", "Facebook", "social", "oauth"),
        ("instagram", "Instagram", "social", "oauth"),
        ("threads", "Threads", "social", "oauth"),
        ("youtube", "YouTube", "streaming", "oauth"),
        ("twitch", "Twitch", "streaming", "oauth"),
        ("tiktok", "TikTok", "streaming", "oauth"),
        ("snapchat", "Snapchat", "social", "oauth"),
        ("wechat", "WeChat", "messaging", "form"),
        ("line", "LINE", "messaging", "form"),
        ("viber", "Viber", "messaging", "form"),
        ("kik", "Kik", "messaging", "form"),
        ("mattermost", "Mattermost", "enterprise", "form"),
        ("zulip", "Zulip", "enterprise", "form"),
        ("rocket_chat", "Rocket.Chat", "enterprise", "form"),
        ("webchat", "Web Chat", "messaging", "form"),
        ("sms_twilio", "SMS (Twilio)", "messaging", "form"),
        ("sms_telnyx", "SMS (Telnyx)", "messaging", "form"),
        ("pagerduty", "PagerDuty", "enterprise", "form"),
        ("opsgenie", "Opsgenie", "enterprise", "form"),
        ("jira", "Jira", "enterprise", "oauth"),
        ("confluence", "Confluence", "enterprise", "oauth"),
        ("notion", "Notion", "enterprise", "oauth"),
        ("github", "GitHub", "enterprise", "oauth"),
        ("gitlab", "GitLab", "enterprise", "oauth"),
        ("bitbucket", "Bitbucket", "enterprise", "oauth"),
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
        assert!(rows.len() >= 40);
    }
}
