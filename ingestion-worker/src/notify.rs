use crate::*;

pub(crate) async fn dispatch_notifications(state: &Arc<AppState>, text: &str) {
    let cfg = &state.config.notifications;
    if let (Some(token), Some(chat)) = (&cfg.telegram_bot_token, &cfg.telegram_chat_id) {
        if !token.is_empty() && !chat.is_empty() {
            let url = format!("https://api.telegram.org/bot{}/sendMessage", token);
            let payload = json!({ "chat_id": chat, "text": text });
            match state.http_client.post(&url).json(&payload).send().await {
                Ok(r) if r.status().is_success() => {
                    tracing::info!("Telegram notification delivered")
                }
                Ok(r) => tracing::warn!("Telegram notification failed: HTTP {}", r.status()),
                Err(e) => tracing::error!("Telegram notification error: {e}"),
            }
        }
    }
    if let (Some(token), Some(user)) = (&cfg.pushover_token, &cfg.pushover_user) {
        if !token.is_empty() && !user.is_empty() {
            let payload = json!({ "token": token, "user": user, "message": text });
            match state
                .http_client
                .post("https://api.pushover.net/1/messages.json")
                .json(&payload)
                .send()
                .await
            {
                Ok(r) if r.status().is_success() => {
                    tracing::info!("Pushover notification delivered")
                }
                Ok(r) => tracing::warn!("Pushover notification failed: HTTP {}", r.status()),
                Err(e) => tracing::error!("Pushover notification error: {e}"),
            }
        }
    }
    if let Some(url) = &cfg.webhook_url {
        if !url.is_empty() {
            let payload = json!({ "text": text });
            match state.http_client.post(url).json(&payload).send().await {
                Ok(r) if r.status().is_success() => {
                    tracing::info!("Webhook notification delivered")
                }
                Ok(r) => tracing::warn!("Webhook notification failed: HTTP {}", r.status()),
                Err(e) => tracing::error!("Webhook notification error: {e}"),
            }
        }
    }
}

/// Pure transition decision: notify only when a directive CHANGES between cycles
/// (never on first boot, never on repeats). Returns the message text.
pub(crate) fn directive_transition_notification<'a>(
    prev: Option<&'a str>,
    next: &'a str,
    details: &str,
    cockpit_url: Option<&str>,
) -> Option<String> {
    let prev = prev?;
    // HOLD's parenthetical follows each cycle's position reads (in range, pool read); a
    // change between HOLD variants is not a directive change and must not push.
    let stem = |a: &'a str| -> &'a str { if a.starts_with("HOLD & ACCRETE") { "HOLD & ACCRETE" } else { a } };
    if stem(prev) == stem(next) {
        return None;
    }
    let mut text = format!("🛡️ NockNock directive change: {}\n{}", next, details);
    if let Some(url) = cockpit_url {
        if !url.is_empty() {
            text.push_str(&format!("\n→ {}", url));
        }
    }
    Some(text)
}
