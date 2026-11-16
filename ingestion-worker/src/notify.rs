use crate::*;

/// Each provider gets this long to answer. Without it a hung connection would hold the
/// ingestion cycle open: the worker's HTTP client has no timeout of its own.
pub(crate) const NOTIFY_TIMEOUT_SECONDS: u64 = 10;

/// Minimum gap between two test sends. The endpoint has no authentication beyond being
/// on the LAN, so this bounds how fast anything can make the phone buzz.
pub(crate) const TEST_SEND_COOLDOWN_SECONDS: u64 = 30;

/// A config value that is present and not blank.
fn configured(v: &Option<String>) -> Option<&str> {
    v.as_deref().map(str::trim).filter(|s| !s.is_empty())
}

/// Which channels the config enables. Pure. Always reports all three, in a fixed order,
/// so "nothing is configured" is stated rather than shown as an empty list. A channel
/// with only half of its pair set is `incomplete` and names the missing key: before #89
/// it was skipped without a word.
pub(crate) fn channel_readiness(cfg: &NotificationConfig) -> Vec<ChannelReport> {
    let report = |channel: &str, status: &str, detail: String| ChannelReport {
        channel: channel.to_string(),
        status: status.to_string(),
        detail,
    };
    let pair = |channel: &str, a: (&str, bool), b: (&str, bool)| match (a.1, b.1) {
        (true, true) => report(channel, "configured", format!("notifications.{} and notifications.{} are set", a.0, b.0)),
        (false, false) => report(channel, "not_configured", format!("notifications.{} and notifications.{} are not set", a.0, b.0)),
        (true, false) => report(channel, "incomplete", format!("notifications.{} is set but notifications.{} is not, so nothing is sent", a.0, b.0)),
        (false, true) => report(channel, "incomplete", format!("notifications.{} is set but notifications.{} is not, so nothing is sent", b.0, a.0)),
    };
    vec![
        pair(
            "telegram",
            ("telegram_bot_token", configured(&cfg.telegram_bot_token).is_some()),
            ("telegram_chat_id", configured(&cfg.telegram_chat_id).is_some()),
        ),
        pair(
            "pushover",
            ("pushover_token", configured(&cfg.pushover_token).is_some()),
            ("pushover_user", configured(&cfg.pushover_user).is_some()),
        ),
        if configured(&cfg.webhook_url).is_some() {
            report("webhook", "configured", "notifications.webhook_url is set".to_string())
        } else {
            report("webhook", "not_configured", "notifications.webhook_url is not set".to_string())
        },
    ]
}

/// Every configured value that must never reach a log line or an API response.
pub(crate) fn notification_secrets(cfg: &NotificationConfig) -> Vec<String> {
    [&cfg.telegram_bot_token, &cfg.pushover_token, &cfg.pushover_user, &cfg.webhook_url]
        .into_iter()
        .filter_map(configured)
        .map(str::to_string)
        .collect()
}

/// Replace every secret in `text`. Pure. Provider answers and transport errors are
/// passed through this before they are logged or returned.
pub(crate) fn redact_secrets(text: &str, secrets: &[String]) -> String {
    let mut out = text.to_string();
    for s in secrets {
        if !s.is_empty() {
            out = out.replace(s.as_str(), "[redacted]");
        }
    }
    out
}

/// What a provider said when it refused a message. Pure. Telegram answers
/// `{"ok":false,"error_code":401,"description":"Unauthorized"}` and Pushover
/// `{"status":0,"errors":["application token is invalid"]}`; a webhook's body is
/// arbitrary, so only its status is reported.
pub(crate) fn provider_error_detail(channel: &str, http_status: u16, body: &str) -> String {
    let parsed: Option<serde_json::Value> = serde_json::from_str(body).ok();
    let said: Option<String> = match channel {
        "telegram" => parsed
            .as_ref()
            .and_then(|v| v.get("description"))
            .and_then(|d| d.as_str())
            .map(str::to_string),
        "pushover" => parsed
            .as_ref()
            .and_then(|v| v.get("errors"))
            .and_then(|e| e.as_array())
            .map(|errs| errs.iter().filter_map(|e| e.as_str()).collect::<Vec<_>>().join("; "))
            .filter(|s| !s.is_empty()),
        _ => None,
    };
    match said {
        Some(s) => {
            let s: String = s.chars().take(200).collect();
            format!("HTTP {}: {}", http_status, s)
        }
        None => format!("HTTP {}", http_status),
    }
}

/// A transport failure in words, without the request URL (Telegram's carries the token).
fn transport_error_detail(e: reqwest::Error) -> String {
    let kind = if e.is_timeout() {
        format!("no answer within {}s", NOTIFY_TIMEOUT_SECONDS)
    } else if e.is_connect() {
        "could not connect".to_string()
    } else {
        "request failed".to_string()
    };
    let e = e.without_url();
    let mut causes: Vec<String> = Vec::new();
    let mut source = std::error::Error::source(&e);
    while let Some(s) = source {
        causes.push(s.to_string());
        source = s.source();
    }
    match causes.last() {
        Some(root) => format!("{}: {}", kind, root.chars().take(200).collect::<String>()),
        None => kind,
    }
}

fn provider_name(channel: &str) -> &'static str {
    match channel {
        "telegram" => "Telegram",
        "pushover" => "Pushover",
        _ => "the webhook",
    }
}

/// Send `text` through every configured channel and report each one. A channel that is
/// not (or only half) configured is reported as such and nothing is sent to it.
/// `delivered` means the provider accepted the message, not that a phone showed it.
pub(crate) async fn deliver_notifications(
    http: &Client,
    cfg: &NotificationConfig,
    endpoints: &NotifyEndpoints,
    text: &str,
) -> Vec<ChannelReport> {
    let secrets = notification_secrets(cfg);
    let mut reports = channel_readiness(cfg);
    for report in reports.iter_mut().filter(|r| r.status == "configured") {
        let request = match report.channel.as_str() {
            "telegram" => {
                let token = configured(&cfg.telegram_bot_token).unwrap_or_default();
                let chat = configured(&cfg.telegram_chat_id).unwrap_or_default();
                http.post(format!("{}/bot{}/sendMessage", endpoints.telegram_base, token))
                    .json(&json!({ "chat_id": chat, "text": text }))
            }
            "pushover" => {
                let token = configured(&cfg.pushover_token).unwrap_or_default();
                let user = configured(&cfg.pushover_user).unwrap_or_default();
                http.post(&endpoints.pushover_url)
                    .json(&json!({ "token": token, "user": user, "message": text }))
            }
            _ => http
                .post(configured(&cfg.webhook_url).unwrap_or_default())
                .json(&json!({ "text": text })),
        };
        let outcome = request
            .timeout(std::time::Duration::from_secs(NOTIFY_TIMEOUT_SECONDS))
            .send()
            .await;
        let (status, detail) = match outcome {
            Ok(resp) => {
                let code = resp.status();
                if code.is_success() {
                    ("delivered", format!("accepted by {} (HTTP {})", provider_name(&report.channel), code.as_u16()))
                } else {
                    let body = resp.text().await.unwrap_or_default();
                    ("failed", provider_error_detail(&report.channel, code.as_u16(), &body))
                }
            }
            Err(e) => ("failed", transport_error_detail(e)),
        };
        report.status = status.to_string();
        report.detail = redact_secrets(&detail, &secrets);
    }
    reports
}

/// Send a cycle's notification. Delivery failures are logged, never fatal to the cycle.
/// The log lines carry the redacted per-channel detail only.
pub(crate) async fn dispatch_notifications(state: &Arc<AppState>, text: &str) {
    let reports = deliver_notifications(
        &state.http_client,
        &state.config.notifications,
        &NotifyEndpoints::default(),
        text,
    )
    .await;
    for r in &reports {
        match r.status.as_str() {
            "delivered" => tracing::info!("{} notification delivered", provider_name(&r.channel)),
            "failed" => tracing::warn!("{} notification failed: {}", provider_name(&r.channel), r.detail),
            "incomplete" => tracing::warn!("{} notification skipped: {}", provider_name(&r.channel), r.detail),
            _ => {}
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

/// The sentence to push when a plan that was running last cycle is complete this cycle
/// (#89). Pure; every figure is the plan's own. None on first sight of a plan, for a
/// different plan id, or when it was already complete.
pub(crate) fn plan_completion_note(
    prev: Option<&AccumulationPlanStatus>,
    next: Option<&AccumulationPlanStatus>,
) -> Option<String> {
    let (prev, next) = (prev?, next?);
    if prev.plan_id != next.plan_id || prev.status == "complete" || next.status != "complete" {
        return None;
    }
    Some(format!(
        "Accumulation plan \"{}\" is complete: ${} of ${} deployed over {} verified buy(s){}. NockNock paces no further buys until you extend its budget or configure a new plan.",
        next.plan_id,
        next.spent_usdc,
        next.budget_usdc,
        next.fills,
        next.avg_price_usd
            .as_ref()
            .map(|p| format!(", average cost ${}", p))
            .unwrap_or_default()
    ))
}

/// Everything one cycle pushes (#89). Pure. A directive change notifies as before; a
/// plan completing adds its own sentence, and notifies on its own when the directive did
/// not change. That happens when the last buy is recorded while LP is under target: the
/// action reads REBUILD LP TRANCHES before and after, and the completion used to pass
/// in silence.
pub(crate) fn cycle_notification(
    prev_action: Option<&str>,
    next_action: &str,
    next_details: &str,
    prev_plan: Option<&AccumulationPlanStatus>,
    next_plan: Option<&AccumulationPlanStatus>,
    cockpit_url: Option<&str>,
) -> Option<String> {
    let directive = directive_transition_notification(prev_action, next_action, next_details, None);
    let completion = plan_completion_note(prev_plan, next_plan);
    let mut text = match (directive, completion) {
        (Some(d), Some(c)) => format!("{}\n{}", d, c),
        (Some(d), None) => d,
        (None, Some(c)) => format!("🛡️ NockNock: {}\nDirective unchanged: {}", c, next_action),
        (None, None) => return None,
    };
    if let Some(url) = cockpit_url.map(str::trim).filter(|u| !u.is_empty()) {
        text.push_str(&format!("\n→ {}", url));
    }
    Some(text)
}

/// The test message (#89). Pure. It says it is a test, and that nothing changed, in the
/// first line: a test must never read as a directive.
pub(crate) fn test_notification_text(now: DateTime<Utc>, cockpit_url: Option<&str>) -> String {
    let mut text = format!(
        "🧪 NockNock TEST notification, sent on request at {} UTC.\nThis is not a directive. Nothing changed in the wallet or the plan: it only checks that this channel works.",
        now.format("%Y-%m-%d %H:%M:%S")
    );
    if let Some(url) = cockpit_url.map(str::trim).filter(|u| !u.is_empty()) {
        text.push_str(&format!("\n→ {}", url));
    }
    text
}

/// A known mistake in `cockpit_url`, in words. Pure. The example config suggested a
/// `/lite` path until v0.5.3; the Lite view is `/?view=lite`, and nginx serves the full
/// dashboard for any other path.
pub(crate) fn cockpit_url_note(url: Option<&str>) -> Option<String> {
    let url = url.map(str::trim).filter(|u| !u.is_empty())?;
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Some("cockpit_url should start with http:// or https:// so the notification's link can be tapped.".to_string());
    }
    let path = url.split(['?', '#']).next().unwrap_or(url);
    if path.trim_end_matches('/').ends_with("/lite") {
        return Some("cockpit_url ends in /lite, which opens the full dashboard. The Lite cockpit is /?view=lite.".to_string());
    }
    None
}

/// `GET /v1/notifications/status` body. Pure. States which channels are configured and
/// when the worker notifies; carries no secret.
pub(crate) fn notification_status_body(cfg: &NotificationConfig) -> serde_json::Value {
    let channels = channel_readiness(cfg);
    let cockpit_url = configured(&cfg.cockpit_url);
    json!({
        "any_configured": channels.iter().any(|c| c.status == "configured"),
        "channels": channels,
        "cockpit_url": cockpit_url,
        "cockpit_url_note": cockpit_url_note(cockpit_url),
        "fires_on": "A change of the directive between two cycles, and an accumulation plan completing. Never on a repeat, and never between HOLD variants.",
        "test_cooldown_seconds": TEST_SEND_COOLDOWN_SECONDS,
    })
}

/// Seconds a test send must still wait, or None when it may go. Pure.
pub(crate) fn test_send_retry_after(since_last: Option<std::time::Duration>, cooldown_seconds: u64) -> Option<u64> {
    let elapsed = since_last?.as_secs();
    (elapsed < cooldown_seconds).then(|| cooldown_seconds - elapsed)
}

/// Send one TEST notification through the configured channels and report each channel
/// (#89). With no channel configured it says so and sends nothing.
pub(crate) async fn run_notification_test(
    http: &Client,
    cfg: &NotificationConfig,
    endpoints: &NotifyEndpoints,
    now: DateTime<Utc>,
) -> serde_json::Value {
    let readiness = channel_readiness(cfg);
    if !readiness.iter().any(|c| c.status == "configured") {
        return json!({
            "sent": false,
            "any_configured": false,
            "message": "No notification channel is configured, so nothing was sent. Add a notifications block to config.yaml (Telegram, Pushover or a webhook) and redeploy.",
            "channels": readiness,
        });
    }
    let text = test_notification_text(now, cfg.cockpit_url.as_deref());
    let channels = deliver_notifications(http, cfg, endpoints, &text).await;
    let attempted = channels.iter().filter(|c| c.status == "delivered" || c.status == "failed").count();
    let delivered = channels.iter().filter(|c| c.status == "delivered").count();
    json!({
        "sent": true,
        "any_configured": true,
        "sent_at": now.to_rfc3339(),
        "message": format!(
            "{} of {} configured channel(s) accepted the test. Accepted means the provider took the message; check that it reached your phone.",
            delivered, attempted
        ),
        "text": text,
        "channels": channels,
    })
}

static LAST_TEST_SEND: std::sync::Mutex<Option<std::time::Instant>> = std::sync::Mutex::new(None);

// ============================================================
// Handlers
// ============================================================

pub(crate) async fn notification_status_handler(Extension(state): Extension<Arc<AppState>>) -> impl IntoResponse {
    Json(notification_status_body(&state.config.notifications))
}

pub(crate) async fn notification_test_handler(Extension(state): Extension<Arc<AppState>>) -> Response {
    let cfg = &state.config.notifications;
    let any_configured = channel_readiness(cfg).iter().any(|c| c.status == "configured");
    if any_configured {
        // Claim the slot before sending, so two requests cannot both pass the check.
        let mut last = LAST_TEST_SEND.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(wait) = test_send_retry_after(last.map(|t| t.elapsed()), TEST_SEND_COOLDOWN_SECONDS) {
            return (
                StatusCode::TOO_MANY_REQUESTS,
                Json(json!({
                    "error": "RATE_LIMITED",
                    "message": format!("A test notification was sent moments ago. Try again in {}s.", wait),
                    "retry_after_seconds": wait,
                })),
            )
                .into_response();
        }
        *last = Some(std::time::Instant::now());
    }
    let body = run_notification_test(&state.http_client, cfg, &NotifyEndpoints::default(), Utc::now()).await;
    if let Some(channels) = body.get("channels").and_then(|c| c.as_array()) {
        for c in channels {
            tracing::info!(
                "Test notification: {} {} ({})",
                c["channel"].as_str().unwrap_or("?"),
                c["status"].as_str().unwrap_or("?"),
                c["detail"].as_str().unwrap_or("")
            );
        }
    }
    (StatusCode::OK, Json(body)).into_response()
}
