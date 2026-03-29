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
        "developer" => ("🛠️", "Medium", "3-8 min"),
        "notifications" => ("🔔", "Easy", "2-5 min"),
        "messaging" => ("💬", "Easy", "2-5 min"),
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
        ("skype", "Skype", "messaging", "form"),
        ("imessage", "iMessage", "messaging", "form"),
        ("facebook_messenger", "Facebook Messenger", "messaging", "oauth"),
        ("kakao", "KakaoTalk", "messaging", "form"),
        ("qq", "QQ", "messaging", "form"),
        ("feishu", "Feishu", "enterprise", "oauth"),
        ("dingtalk", "DingTalk", "enterprise", "oauth"),
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
        ("snapchat", "Snapchat", "social", "oauth"),

        // Enterprise PM + docs + repos
        ("mattermost", "Mattermost", "enterprise", "form"),
        ("zulip", "Zulip", "enterprise", "form"),
        ("rocket_chat", "Rocket.Chat", "enterprise", "form"),
        ("linear", "Linear", "enterprise", "oauth"),
        ("asana", "Asana", "enterprise", "oauth"),
        ("clickup", "ClickUp", "enterprise", "oauth"),
        ("monday", "Monday.com", "enterprise", "oauth"),
        ("trello", "Trello", "enterprise", "oauth"),
        ("airtable", "Airtable", "enterprise", "oauth"),
        ("servicenow", "ServiceNow", "enterprise", "oauth"),
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
        ("azure_event_grid", "Azure Event Grid", "notifications", "oauth"),
        ("gcp_pubsub", "GCP Pub/Sub", "notifications", "oauth"),
        ("pushover", "Pushover", "notifications", "form"),
        ("ntfy", "ntfy", "notifications", "form"),
        ("gotify", "Gotify", "notifications", "form"),
        ("ifttt", "IFTTT", "notifications", "oauth"),
        ("statuspage", "Atlassian Statuspage", "notifications", "oauth"),
        ("pagerduty_events", "PagerDuty Events API", "notifications", "form"),
        ("opsgenie_alerts", "Opsgenie Alerts", "notifications", "form"),
        ("discord_webhook", "Discord Webhook", "notifications", "form"),
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
}
