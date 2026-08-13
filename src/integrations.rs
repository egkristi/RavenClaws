//! # Messaging & Connector Integrations
//!
//! Outbound notifications to common messaging platforms. Each integration is
//! **env-var gated** — when the required credentials are absent, the function
//! returns a `disabled` result instead of failing. This mirrors the
//! `RavenAssistant01` orchestrator's integration surface, re-expressed as a
//! clean library API (no HTTP framework coupling).
//!
//! Supported channels: Slack, Discord, Microsoft Teams, Signal (signald),
//! Matrix, Telegram, Email (Mailgun), and SMS (Twilio).
//!
//! ## Usage
//!
//! ```rust,no_run
//! use ravenclaws::integrations::send_slack;
//!
//! # async fn example() {
//! let result = send_slack("Hello from RavenClaws").await;
//! println!("{}", result.status);
//! # }
//! ```
//!
//! This module exposes a public API consumed by library users rather than by the
//! default binary, so dead-code analysis on the binary produces false positives.
#![allow(dead_code)]

use serde::{Deserialize, Serialize};

/// Outcome of an integration attempt.
///
/// # Stability
/// This struct is `#[non_exhaustive]` — new fields may be added in minor releases.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct IntegrationResult {
    /// One of: `sent`, `disabled`, `error`
    pub status: String,
    /// Human-readable detail (error message or configuration hint)
    pub detail: Option<String>,
    /// HTTP status code when an HTTP call was made
    pub http_code: Option<u16>,
}

impl IntegrationResult {
    fn sent(http_code: u16) -> Self {
        Self {
            status: "sent".to_string(),
            detail: None,
            http_code: Some(http_code),
        }
    }

    fn disabled(message: impl Into<String>) -> Self {
        Self {
            status: "disabled".to_string(),
            detail: Some(message.into()),
            http_code: None,
        }
    }

    fn error(detail: impl Into<String>) -> Self {
        Self {
            status: "error".to_string(),
            detail: Some(detail.into()),
            http_code: None,
        }
    }
}

/// Build a shared `reqwest` client for integrations.
fn client() -> Result<reqwest::Client, reqwest::Error> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .user_agent("RavenClaws/1.4.0")
        .build()
}

/// Post a JSON payload and map the response to an [`IntegrationResult`].
async fn post_json(
    client: &reqwest::Client,
    url: &str,
    payload: serde_json::Value,
) -> IntegrationResult {
    match client.post(url).json(&payload).send().await {
        Ok(r) => IntegrationResult::sent(r.status().as_u16()),
        Err(e) => IntegrationResult::error(e.to_string()),
    }
}

// ── Slack ──────────────────────────────────────────────────────────────────

/// Send a Slack message via incoming webhook.
///
/// Requires the `SLACK_WEBHOOK_URL` environment variable.
pub async fn send_slack(text: &str) -> IntegrationResult {
    let webhook_url = std::env::var("SLACK_WEBHOOK_URL").unwrap_or_default();
    if webhook_url.is_empty() {
        return IntegrationResult::disabled(
            "Set SLACK_WEBHOOK_URL env to enable Slack notifications",
        );
    }
    let client = match client() {
        Ok(c) => c,
        Err(e) => return IntegrationResult::error(e.to_string()),
    };
    let payload = serde_json::json!({ "text": text });
    post_json(&client, &webhook_url, payload).await
}

// ── Discord ────────────────────────────────────────────────────────────────

/// Send a Discord message via webhook.
///
/// Requires the `DISCORD_WEBHOOK_URL` environment variable.
pub async fn send_discord(content: &str) -> IntegrationResult {
    let webhook_url = std::env::var("DISCORD_WEBHOOK_URL").unwrap_or_default();
    if webhook_url.is_empty() {
        return IntegrationResult::disabled(
            "Set DISCORD_WEBHOOK_URL env to enable Discord notifications",
        );
    }
    let client = match client() {
        Ok(c) => c,
        Err(e) => return IntegrationResult::error(e.to_string()),
    };
    let payload = serde_json::json!({ "content": content });
    post_json(&client, &webhook_url, payload).await
}

// ── Microsoft Teams ────────────────────────────────────────────────────────

/// Send a Microsoft Teams message via incoming webhook.
///
/// Requires the `TEAMS_WEBHOOK_URL` environment variable.
pub async fn send_teams(text: &str, title: &str) -> IntegrationResult {
    let webhook_url = std::env::var("TEAMS_WEBHOOK_URL").unwrap_or_default();
    if webhook_url.is_empty() {
        return IntegrationResult::disabled(
            "Set TEAMS_WEBHOOK_URL env to enable Teams notifications",
        );
    }
    let client = match client() {
        Ok(c) => c,
        Err(e) => return IntegrationResult::error(e.to_string()),
    };
    let payload = serde_json::json!({
        "@type": "MessageCard",
        "@context": "http://schema.org/extensions",
        "title": title,
        "text": text,
    });
    post_json(&client, &webhook_url, payload).await
}

// ── Signal (signald) ───────────────────────────────────────────────────────

/// Send a Signal message via a signald REST endpoint.
///
/// Requires the `SIGNALD_REST_URL` environment variable.
pub async fn send_signal(recipient: &str, message: &str) -> IntegrationResult {
    let signald_url = std::env::var("SIGNALD_REST_URL").unwrap_or_default();
    if signald_url.is_empty() {
        return IntegrationResult::disabled(
            "Set SIGNALD_REST_URL env to enable Signal notifications",
        );
    }
    if recipient.is_empty() {
        return IntegrationResult::error("recipient field required (phone number)");
    }
    let client = match client() {
        Ok(c) => c,
        Err(e) => return IntegrationResult::error(e.to_string()),
    };
    let payload = serde_json::json!({ "number": recipient, "message": message });
    let url = format!("{}/v2/send", signald_url.trim_end_matches('/'));
    post_json(&client, &url, payload).await
}

// ── Matrix ─────────────────────────────────────────────────────────────────

/// Send a Matrix message.
///
/// Requires `MATRIX_HOMESERVER`, `MATRIX_ACCESS_TOKEN`, and `MATRIX_ROOM_ID`
/// environment variables.
pub async fn send_matrix(room_id: &str, message: &str) -> IntegrationResult {
    let homeserver = std::env::var("MATRIX_HOMESERVER").unwrap_or_default();
    let access_token = std::env::var("MATRIX_ACCESS_TOKEN").unwrap_or_default();
    let default_room = std::env::var("MATRIX_ROOM_ID").unwrap_or_default();

    if homeserver.is_empty() || access_token.is_empty() {
        return IntegrationResult::disabled(
            "Set MATRIX_HOMESERVER, MATRIX_ACCESS_TOKEN, MATRIX_ROOM_ID env vars",
        );
    }
    let room = if room_id.is_empty() {
        &default_room
    } else {
        room_id
    };
    let url = format!(
        "{}/_matrix/client/r0/rooms/{}/send/m.room.message",
        homeserver.trim_end_matches('/'),
        room
    );
    let payload = serde_json::json!({ "msgtype": "m.text", "body": message });

    let client = match client() {
        Ok(c) => c,
        Err(e) => return IntegrationResult::error(e.to_string()),
    };
    match client
        .post(&url)
        .bearer_auth(&access_token)
        .json(&payload)
        .send()
        .await
    {
        Ok(r) => IntegrationResult::sent(r.status().as_u16()),
        Err(e) => IntegrationResult::error(e.to_string()),
    }
}

// ── Telegram ───────────────────────────────────────────────────────────────

/// Send a Telegram message via the Bot API.
///
/// Requires `TELEGRAM_BOT_TOKEN` and `TELEGRAM_CHAT_ID` environment variables.
pub async fn send_telegram(text: &str) -> IntegrationResult {
    let token = std::env::var("TELEGRAM_BOT_TOKEN").unwrap_or_default();
    let chat_id = std::env::var("TELEGRAM_CHAT_ID").unwrap_or_default();
    if token.is_empty() || chat_id.is_empty() {
        return IntegrationResult::disabled(
            "Set TELEGRAM_BOT_TOKEN and TELEGRAM_CHAT_ID env vars to enable Telegram",
        );
    }
    let url = format!("https://api.telegram.org/bot{}/sendMessage", token);
    let payload = serde_json::json!({ "chat_id": chat_id, "text": text });

    let client = match client() {
        Ok(c) => c,
        Err(e) => return IntegrationResult::error(e.to_string()),
    };
    post_json(&client, &url, payload).await
}

// ── Email (Mailgun) ────────────────────────────────────────────────────────

/// Send an email via the Mailgun API.
///
/// Requires `MAILGUN_API_KEY` and `MAILGUN_DOMAIN` environment variables.
pub async fn send_email(to: &str, subject: &str, body: &str) -> IntegrationResult {
    let api_key = std::env::var("MAILGUN_API_KEY").unwrap_or_default();
    let domain = std::env::var("MAILGUN_DOMAIN").unwrap_or_default();
    if api_key.is_empty() || domain.is_empty() {
        return IntegrationResult::disabled(
            "Set MAILGUN_API_KEY and MAILGUN_DOMAIN env vars to enable email",
        );
    }
    let url = format!("https://api.mailgun.net/v3/{}/messages", domain);
    let client = match client() {
        Ok(c) => c,
        Err(e) => return IntegrationResult::error(e.to_string()),
    };
    match client
        .post(&url)
        .basic_auth("api", Some(&api_key))
        .form(&[
            ("from", format!("RavenClaws <noreply@{}>", domain)),
            ("to", to.to_string()),
            ("subject", subject.to_string()),
            ("text", body.to_string()),
        ])
        .send()
        .await
    {
        Ok(r) => {
            let status = r.status();
            if status.is_success() {
                IntegrationResult::sent(status.as_u16())
            } else {
                IntegrationResult {
                    status: "error".to_string(),
                    detail: Some(r.text().await.unwrap_or_default()),
                    http_code: Some(status.as_u16()),
                }
            }
        }
        Err(e) => IntegrationResult::error(e.to_string()),
    }
}

// ── SMS (Twilio) ───────────────────────────────────────────────────────────

/// Send an SMS via the Twilio API.
///
/// Requires `TWILIO_ACCOUNT_SID`, `TWILIO_AUTH_TOKEN`, and `TWILIO_PHONE_NUMBER`
/// environment variables.
pub async fn send_sms(to: &str, message: &str) -> IntegrationResult {
    let account_sid = std::env::var("TWILIO_ACCOUNT_SID").unwrap_or_default();
    let auth_token = std::env::var("TWILIO_AUTH_TOKEN").unwrap_or_default();
    let from_phone = std::env::var("TWILIO_PHONE_NUMBER").unwrap_or_default();

    if account_sid.is_empty() || auth_token.is_empty() || from_phone.is_empty() {
        return IntegrationResult::disabled(
            "Set TWILIO_ACCOUNT_SID, TWILIO_AUTH_TOKEN, TWILIO_PHONE_NUMBER env vars to enable SMS",
        );
    }
    if to.is_empty() {
        return IntegrationResult::error("to field required (phone number)");
    }
    let url = format!(
        "https://api.twilio.com/2010-04-01/Accounts/{}/Messages.json",
        account_sid
    );
    let client = match client() {
        Ok(c) => c,
        Err(e) => return IntegrationResult::error(e.to_string()),
    };
    match client
        .post(&url)
        .basic_auth(&account_sid, Some(&auth_token))
        .form(&[("From", from_phone.as_str()), ("To", to), ("Body", message)])
        .send()
        .await
    {
        Ok(r) => {
            let status = r.status();
            if status.is_success() {
                IntegrationResult::sent(status.as_u16())
            } else {
                IntegrationResult {
                    status: "error".to_string(),
                    detail: Some(r.text().await.unwrap_or_default()),
                    http_code: Some(status.as_u16()),
                }
            }
        }
        Err(e) => IntegrationResult::error(e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_disabled_when_env_missing() {
        // Ensure the env vars are not set for these tests.
        std::env::remove_var("SLACK_WEBHOOK_URL");
        std::env::remove_var("DISCORD_WEBHOOK_URL");
        std::env::remove_var("TELEGRAM_BOT_TOKEN");
        std::env::remove_var("TELEGRAM_CHAT_ID");
        std::env::remove_var("MAILGUN_API_KEY");
        std::env::remove_var("MAILGUN_DOMAIN");
        std::env::remove_var("TWILIO_ACCOUNT_SID");
        std::env::remove_var("TWILIO_AUTH_TOKEN");
        std::env::remove_var("TWILIO_PHONE_NUMBER");
        std::env::remove_var("TEAMS_WEBHOOK_URL");
        std::env::remove_var("SIGNALD_REST_URL");
        std::env::remove_var("MATRIX_HOMESERVER");
        std::env::remove_var("MATRIX_ACCESS_TOKEN");
        std::env::remove_var("MATRIX_ROOM_ID");
    }

    #[tokio::test]
    async fn test_send_slack_disabled_without_env() {
        std::env::remove_var("SLACK_WEBHOOK_URL");
        let result = send_slack("hello").await;
        assert_eq!(result.status, "disabled");
    }

    #[tokio::test]
    async fn test_send_discord_disabled_without_env() {
        std::env::remove_var("DISCORD_WEBHOOK_URL");
        let result = send_discord("hello").await;
        assert_eq!(result.status, "disabled");
    }

    #[tokio::test]
    async fn test_send_telegram_disabled_without_env() {
        std::env::remove_var("TELEGRAM_BOT_TOKEN");
        std::env::remove_var("TELEGRAM_CHAT_ID");
        let result = send_telegram("hello").await;
        assert_eq!(result.status, "disabled");
    }

    #[tokio::test]
    async fn test_send_teams_disabled_without_env() {
        std::env::remove_var("TEAMS_WEBHOOK_URL");
        let result = send_teams("hello", "title").await;
        assert_eq!(result.status, "disabled");
    }

    #[tokio::test]
    async fn test_send_email_disabled_without_env() {
        std::env::remove_var("MAILGUN_API_KEY");
        std::env::remove_var("MAILGUN_DOMAIN");
        let result = send_email("to@example.com", "subject", "body").await;
        assert_eq!(result.status, "disabled");
    }

    #[tokio::test]
    async fn test_send_sms_disabled_without_env() {
        std::env::remove_var("TWILIO_ACCOUNT_SID");
        std::env::remove_var("TWILIO_AUTH_TOKEN");
        std::env::remove_var("TWILIO_PHONE_NUMBER");
        let result = send_sms("+15551234567", "hello").await;
        assert_eq!(result.status, "disabled");
    }

    #[tokio::test]
    async fn test_send_sms_requires_recipient() {
        std::env::set_var("TWILIO_ACCOUNT_SID", "sid");
        std::env::set_var("TWILIO_AUTH_TOKEN", "token");
        std::env::set_var("TWILIO_PHONE_NUMBER", "+10000000000");
        let result = send_sms("", "hello").await;
        assert_eq!(result.status, "error");
    }

    #[tokio::test]
    async fn test_send_signal_disabled_without_env() {
        std::env::remove_var("SIGNALD_REST_URL");
        let result = send_signal("+15551234567", "hello").await;
        assert_eq!(result.status, "disabled");
    }

    #[tokio::test]
    async fn test_send_matrix_disabled_without_env() {
        std::env::remove_var("MATRIX_HOMESERVER");
        std::env::remove_var("MATRIX_ACCESS_TOKEN");
        std::env::remove_var("MATRIX_ROOM_ID");
        let result = send_matrix("", "hello").await;
        assert_eq!(result.status, "disabled");
    }
}
