//! Notification tests (#89): which transitions push, what the config enables, and
//! delivery against a local stand-in for each provider. Nothing here touches the
//! network beyond 127.0.0.1, and nothing needs a config.yaml.
use super::*;

fn t(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
}

/// One operator's explicit numbers; `floor` is the only one these tests vary.
fn profile(floor: f64) -> OperatorProfile {
    OperatorProfile {
        treasury_floor_nock: floor,
        lp_floor_tolerance_bps: 0,
        tax_reserve_ratio: 0.0,
        target_weights_bps: TargetWeightsBps { treasury: 8000, uniswap_lp: 1500, cash: 500 },
    }
}

fn balance(sym: &str, addr: &str, amount: f64) -> WalletBalance {
    WalletBalance {
        wallet_address: "0x00000000000000000000000000000000000000aa".to_string(),
        token_symbol: sym.to_string(),
        token_address: addr.to_string(),
        amount: format!("{:.4}", amount),
        amount_usd: "0".to_string(),
        updated_at: "now".to_string(),
    }
}

/// A wallet with no LP: every amount is a test input.
fn wallet_only(usdc: f64, nock: f64) -> RawOnChainData {
    let mut raw = RawOnChainData::default();
    raw.spot_price_nock_usd = 0.01153;
    raw.live_eth_price_usd = 3000.0;
    raw.live_gas_cost_usd = 0.004;
    raw.wallet_eth = Some(0.0);
    raw.wallet_weth = Some(0.0);
    raw.wallet_balances = vec![balance("NOCK", NOCK_BASE, nock), balance("USDC", USDC_BASE, usdc)];
    raw
}

/// The wide USDC/NOCK v4 range from ONCHAIN_INTEGRATION.md §5 (#3163450), as the price
/// source, with its amounts derived the way the live pipeline derives them.
fn usdc_nock_range() -> RawPosition {
    let mut p = RawPosition {
        position_id: "uniswap-v4:base:3163450".to_string(),
        network: "base".to_string(),
        protocol: "uniswap-v4".to_string(),
        wallet: "0x00000000000000000000000000000000000000aa".to_string(),
        nft_id: 3163450,
        pool_id: "0x064a5df2ecfcb7c05779b147aa38e4b0e91c966267fbdbc8cf16addd83636b13".to_string(),
        npm_address: None,
        fee_tier_bps: 300,
        token0: PositionTokenInfo { address: USDC_BASE.to_string(), symbol: "USDC".to_string(), decimals: 6 },
        token1: PositionTokenInfo { address: NOCK_BASE.to_string(), symbol: "NOCK".to_string(), decimals: 16 },
        liquidity: 185454734478733,
        sqrt_price_x96: U256::from_hex_be("0xa9d5d9b7b19cc983c088d1f6cd97b").unwrap(),
        tick: 269065,
        tick_lower: 262200,
        tick_upper: 282300,
        amount0: 0.0,
        amount1: 0.0,
        fee0: 0.0,
        fee1: 0.0,
        in_range: true,
        is_price_source: true,
    };
    let (a0, a1) = concentrated_amounts_raw(p.liquidity, p.sqrt_price_x96, p.tick_lower, p.tick_upper);
    p.amount0 = a0 / 10f64.powi(p.token0.decimals);
    p.amount1 = a1 / 10f64.powi(p.token1.decimals);
    p
}

/// A wallet holding that range, funded and in range.
fn wallet_with_range(usdc: f64, nock: f64) -> RawOnChainData {
    let p = usdc_nock_range();
    let mut raw = wallet_only(usdc, nock);
    raw.spot_price_nock_usd = nock_spot_from_positions(&[p.clone()]).expect("fixture price source");
    raw.pool_liquidity.insert(p.position_id.clone(), "80173935548453779".to_string());
    raw.positions = vec![p];
    raw
}

/// The same wallet after the range was emptied: LP is under target, the pool is live,
/// and the wallet can fund a deposit, so the LP rebuild bias leads every idle state.
fn wallet_with_emptied_range(usdc: f64, nock: f64) -> RawOnChainData {
    let mut raw = wallet_with_range(usdc, nock);
    let p = &mut raw.positions[0];
    p.liquidity = 0;
    p.amount0 = 0.0;
    p.amount1 = 0.0;
    raw
}

/// The user's own plan: $100 over 5 tranches, 24 h apart.
fn plan() -> AccumulationPlanConfig {
    AccumulationPlanConfig {
        id: "my-plan".to_string(),
        budget_usdc: 100.0,
        tranches: 5,
        interval_hours: 24.0,
        starts_at: None,
        venue: "Aero".to_string(),
        max_price_usd: None,
        pause_when_overextended: true,
        respect_cash_target: true,
        cash_reserve_usdc: None,
    }
}

/// `fills` verified $20 buys, the last one at 2026-11-15 12:00 UTC, seen at `now`.
fn plan_after(fills: i64, now: &str) -> PlanContext {
    PlanContext {
        cfg: Some(plan()),
        fills: PlanFills {
            count: fills,
            spent_usdc: 20.0 * fills as f64,
            acquired_nock: 1734.6054 * fills as f64,
            last_at: (fills > 0).then(|| t("2026-11-15T12:00:00Z")),
            first_price: (fills > 0).then_some(0.01153),
        },
        ledger_unavailable: false,
        now: t(now),
    }
}

const BEFORE_DUE: &str = "2026-11-16T11:55:00Z";
const AT_DUE: &str = "2026-11-16T12:00:00Z";

fn metrics(raw: &RawOnChainData, profile: &OperatorProfile, ctx: &PlanContext) -> DerivedMetrics {
    compute_derived_metrics_with_plan(
        raw, 0.0, profile, StructuralChannels { channels: vec![] }, &[],
        &LedgerAggregate::default(), &MarketRegime::default(), ctx,
    )
}

/// What the cycle would push going from `prev` to `next`, exactly as `run_ingestion_cycle`
/// decides it.
fn pushed(prev: &DerivedMetrics, next: &DerivedMetrics) -> Option<String> {
    cycle_notification(
        Some(&prev.global_status.shot_advice_action),
        &next.global_status.shot_advice_action,
        &next.global_status.shot_advice_details,
        prev.accumulation_plan.as_ref(),
        next.accumulation_plan.as_ref(),
        Some("http://host:3000/?view=lite"),
    )
}

// ---------------- Transitions the operator cares about ----------------

#[test]
fn test_tranche_becoming_due_changes_the_action_and_notifies() {
    let raw = wallet_only(480.0, 1734.6054);
    let p = profile(0.0);
    let waiting = metrics(&raw, &p, &plan_after(1, BEFORE_DUE));
    let due = metrics(&raw, &p, &plan_after(1, AT_DUE));
    assert_eq!(waiting.global_status.shot_advice_action, "DCA ON SCHEDULE (2 of 5 next)");
    assert_eq!(due.global_status.shot_advice_action, "DCA TRANCHE DUE (2 of 5)");
    let text = pushed(&waiting, &due).expect("a tranche becoming due must notify");
    assert!(text.starts_with("🛡️ NockNock directive change: DCA TRANCHE DUE (2 of 5)\n"), "{text}");
    assert!(text.contains("Buy $20.00 of NOCK on Aero"), "{text}");
    assert!(text.ends_with("\n→ http://host:3000/?view=lite"), "{text}");
    // The cycles before it, and a second cycle while it is still due, push nothing.
    assert!(pushed(&waiting, &waiting).is_none());
    assert!(pushed(&due, &due).is_none());
    // Recording the buy moves the plan on, and that notifies too.
    let recorded = metrics(&wallet_only(460.0, 3469.2108), &p, &plan_after(2, "2026-11-15T12:05:00Z"));
    assert_eq!(recorded.global_status.shot_advice_action, "DCA ON SCHEDULE (3 of 5 next)");
    assert!(pushed(&due, &recorded).is_some());
}

#[test]
fn test_tranche_becoming_due_notifies_under_the_lp_rebuild_bias() {
    // LP under target with a fundable deposit: every idle plan state reads REBUILD LP
    // TRANCHES, and a due tranche must still take the lead.
    let raw = wallet_with_emptied_range(480.0, 60_000.0);
    let p = profile(0.0);
    let waiting = metrics(&raw, &p, &plan_after(1, BEFORE_DUE));
    let due = metrics(&raw, &p, &plan_after(1, AT_DUE));
    assert_eq!(waiting.global_status.shot_advice_action, "REBUILD LP TRANCHES (No Net Cash Change)");
    assert_eq!(due.global_status.shot_advice_action, "DCA TRANCHE DUE (2 of 5)");
    let text = pushed(&waiting, &due).expect("a due tranche must notify under the rebuild bias");
    assert!(text.contains("DCA TRANCHE DUE (2 of 5)"), "{text}");
}

#[test]
fn test_tranche_becoming_due_notifies_under_a_floor_lock() {
    let raw = wallet_only(480.0, 1734.6054);
    let p = profile(100_000.0);
    let waiting = metrics(&raw, &p, &plan_after(1, BEFORE_DUE));
    let due = metrics(&raw, &p, &plan_after(1, AT_DUE));
    assert_eq!(waiting.global_status.shot_advice_action, "DEFENSIVE LOCK (Floor Recovery) · DCA ON SCHEDULE (2 of 5 next)");
    assert_eq!(due.global_status.shot_advice_action, "DEFENSIVE LOCK (Floor Recovery) · DCA TRANCHE DUE (2 of 5)");
    assert!(pushed(&waiting, &due).is_some());
}

#[test]
fn test_plan_completing_notifies_and_says_so() {
    let p = profile(0.0);
    let last_due = metrics(&wallet_only(420.0, 6938.4216), &p, &plan_after(4, AT_DUE));
    assert_eq!(last_due.global_status.shot_advice_action, "DCA TRANCHE DUE (5 of 5)");
    let done = metrics(&wallet_only(400.0, 8673.027), &p, &plan_after(5, "2026-11-15T12:05:00Z"));
    assert_eq!(done.accumulation_plan.as_ref().unwrap().status, "complete");
    assert!(done.global_status.shot_advice_action.starts_with("HOLD & ACCRETE"), "{}", done.global_status.shot_advice_action);
    let text = pushed(&last_due, &done).expect("a plan completing must notify");
    assert!(text.contains("directive change: HOLD & ACCRETE"), "{text}");
    assert!(
        text.contains("Accumulation plan \"my-plan\" is complete: $100.00 of $100.00 deployed over 5 verified buy(s), average cost $0.011530."),
        "{text}"
    );
    assert!(text.ends_with("\n→ http://host:3000/?view=lite"), "{text}");
    // Complete stays complete: no second push.
    assert!(pushed(&done, &done).is_none());
}

#[test]
fn test_plan_completing_under_the_rebuild_bias_is_not_silent() {
    // The last buy is recorded early, while the plan was waiting and LP was under
    // target. The action reads REBUILD LP TRANCHES before and after, so the directive
    // alone would push nothing.
    let p = profile(0.0);
    let waiting = metrics(&wallet_with_emptied_range(420.0, 60_000.0), &p, &plan_after(4, BEFORE_DUE));
    let done = metrics(&wallet_with_emptied_range(400.0, 61_734.0), &p, &plan_after(5, "2026-11-16T11:58:00Z"));
    assert_eq!(waiting.global_status.shot_advice_action, "REBUILD LP TRANCHES (No Net Cash Change)");
    assert_eq!(done.global_status.shot_advice_action, "REBUILD LP TRANCHES (No Net Cash Change)");
    assert!(directive_transition_notification(
        Some(&waiting.global_status.shot_advice_action),
        &done.global_status.shot_advice_action,
        &done.global_status.shot_advice_details,
        None,
    )
    .is_none());
    let text = pushed(&waiting, &done).expect("a plan completing must notify even when the directive is unchanged");
    assert!(text.starts_with("🛡️ NockNock: Accumulation plan \"my-plan\" is complete: $100.00 of $100.00 deployed over 5 verified buy(s)"), "{text}");
    assert!(text.contains("\nDirective unchanged: REBUILD LP TRANCHES (No Net Cash Change)"), "{text}");
    assert!(!text.contains("directive change"), "{text}");
}

#[test]
fn test_plan_completion_note_only_on_the_cycle_it_completes() {
    let p = profile(0.0);
    let running = metrics(&wallet_only(420.0, 6938.0), &p, &plan_after(4, AT_DUE)).accumulation_plan.unwrap();
    let done = metrics(&wallet_only(400.0, 8673.0), &p, &plan_after(5, AT_DUE)).accumulation_plan.unwrap();
    assert!(plan_completion_note(Some(&running), Some(&done)).is_some());
    // First sight of a plan that is already complete (a restart, a re-recorded ledger).
    assert!(plan_completion_note(None, Some(&done)).is_none());
    assert!(plan_completion_note(Some(&done), Some(&done)).is_none());
    assert!(plan_completion_note(Some(&running), Some(&running)).is_none());
    // The plan was removed from config: not a completion.
    assert!(plan_completion_note(Some(&running), None).is_none());
    // A different plan id that happens to be complete is not this plan completing.
    let mut other = done.clone();
    other.plan_id = "another-plan".to_string();
    assert!(plan_completion_note(Some(&running), Some(&other)).is_none());
}

#[test]
fn test_floor_lock_starting_and_ending_notifies() {
    let p = profile(100_000.0);
    let none = PlanContext::none();
    // Spot above the $0.020 line, so the only thing that changes is the floor.
    let at = |nock: f64| {
        let mut raw = wallet_only(500.0, nock);
        raw.spot_price_nock_usd = 0.025;
        raw
    };
    let above = metrics(&at(100_500.0), &p, &none);
    let below = metrics(&at(99_000.0), &p, &none);
    assert!(above.global_status.shot_advice_action.starts_with("HOLD & ACCRETE"), "{}", above.global_status.shot_advice_action);
    assert_eq!(below.global_status.shot_advice_action, "DEFENSIVE LOCK (Floor Recovery)");
    let text = pushed(&above, &below).expect("a floor lock starting must notify");
    assert!(text.starts_with("🛡️ NockNock directive change: DEFENSIVE LOCK (Floor Recovery)\n"), "{text}");
    assert!(text.contains("Total NOCK (99000) is below the 100000 floor by 1000 NOCK."), "{text}");
    // While it lasts: silence. When the stack is back above the floor: one push.
    assert!(pushed(&below, &below).is_none());
    assert!(pushed(&below, &above).is_some());
    // With no floor configured there is no lock to start.
    let no_floor = metrics(&at(99_000.0), &profile(0.0), &none);
    assert!(!no_floor.global_status.shot_advice_action.contains("DEFENSIVE LOCK"));
}

#[test]
fn test_range_exit_notifies() {
    let p = profile(0.0);
    let none = PlanContext::none();
    let inside = wallet_with_range(50.0, 1000.0);
    let before = metrics(&inside, &p, &none);
    assert!(!before.global_status.shot_advice_action.contains("RANGE EXIT"), "{}", before.global_status.shot_advice_action);

    // Spot moves above the top of the price-source range. USDC is token0, so the NOCK
    // price is highest at the lower tick.
    let mut outside = wallet_with_range(50.0, 1000.0);
    let src = outside.positions[0].clone();
    let top = tick_to_display_price_dec(src.tick_lower, src.token0.decimals, src.token1.decimals);
    outside.spot_price_nock_usd = top * 1.01;
    outside.positions[0].in_range = false;
    let after = metrics(&outside, &p, &none);
    assert_eq!(after.global_status.shot_advice_action, "RANGE EXIT FLASHPOINT (Above Range)");
    let text = pushed(&before, &after).expect("a range exit must notify");
    assert!(text.starts_with("🛡️ NockNock directive change: RANGE EXIT FLASHPOINT (Above Range)\n"), "{text}");
    assert!(text.contains("is at or above the top of the price-source range"), "{text}");
    assert!(pushed(&after, &after).is_none());
    // Falling back into the range ends the flashpoint, and says so once.
    assert!(pushed(&after, &before).is_some());
}

#[test]
fn test_no_previous_snapshot_and_hold_variants_stay_silent() {
    let p = profile(0.0);
    let due = metrics(&wallet_only(480.0, 1734.6054), &p, &plan_after(1, AT_DUE));
    // A first cycle has nothing to compare against.
    assert!(cycle_notification(
        None,
        &due.global_status.shot_advice_action,
        &due.global_status.shot_advice_details,
        None,
        due.accumulation_plan.as_ref(),
        Some("http://host:3000/?view=lite"),
    )
    .is_none());
    assert!(cycle_notification(
        Some("HOLD & ACCRETE (Dual Growth Engine Active)"),
        "HOLD & ACCRETE (No Ranged LP Confirmed In Range)",
        "d",
        None,
        None,
        None,
    )
    .is_none());
    // A blank cockpit_url adds no dangling arrow.
    let text = cycle_notification(Some("A"), "B", "d", None, None, Some("  ")).unwrap();
    assert_eq!(text, "🛡️ NockNock directive change: B\nd");
}

// ---------------- Config ----------------

fn cfg(telegram: Option<(&str, &str)>, pushover: Option<(&str, &str)>, webhook: Option<&str>) -> NotificationConfig {
    NotificationConfig {
        telegram_bot_token: telegram.map(|x| x.0.to_string()),
        telegram_chat_id: telegram.map(|x| x.1.to_string()),
        pushover_token: pushover.map(|x| x.0.to_string()),
        pushover_user: pushover.map(|x| x.1.to_string()),
        webhook_url: webhook.map(str::to_string),
        cockpit_url: None,
    }
}

fn status_of<'a>(reports: &'a [ChannelReport], channel: &str) -> &'a ChannelReport {
    reports.iter().find(|r| r.channel == channel).unwrap_or_else(|| panic!("no {channel} report"))
}

#[test]
fn test_channel_readiness_names_what_is_missing() {
    let none = channel_readiness(&NotificationConfig::default());
    assert_eq!(none.iter().map(|r| r.channel.as_str()).collect::<Vec<_>>(), vec!["telegram", "pushover", "webhook"]);
    assert!(none.iter().all(|r| r.status == "not_configured"));

    let all = channel_readiness(&cfg(Some(("tok", "42")), Some(("app", "user")), Some("http://h/hook")));
    assert!(all.iter().all(|r| r.status == "configured"));

    // Half a pair is not a channel, and the missing key is named.
    let mut half = NotificationConfig::default();
    half.telegram_bot_token = Some("tok".to_string());
    half.pushover_user = Some("user".to_string());
    let r = channel_readiness(&half);
    assert_eq!(status_of(&r, "telegram").status, "incomplete");
    assert!(status_of(&r, "telegram").detail.contains("notifications.telegram_chat_id is not"), "{}", status_of(&r, "telegram").detail);
    assert_eq!(status_of(&r, "pushover").status, "incomplete");
    assert!(status_of(&r, "pushover").detail.contains("notifications.pushover_token is not"), "{}", status_of(&r, "pushover").detail);

    // Blank strings are unset, not credentials.
    let blank = channel_readiness(&cfg(Some(("", " ")), Some(("", "")), Some("  ")));
    assert!(blank.iter().all(|r| r.status == "not_configured"));
}

fn write_temp_config(name: &str, body: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("nocknock-notify-{}", Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);
    std::fs::write(&path, body).unwrap();
    path
}

const MINIMAL_CONFIG: &str = "alchemy_api_keys: [\"k\"]\nport: 8080\nsync_window_seconds: 300\n";

#[test]
fn test_pushover_user_key_spelling_is_accepted() {
    // The example documented `pushover_user_key` until v0.5.3; the worker read only
    // `pushover_user`, so a copied example left Pushover off without a word.
    for key in ["pushover_user", "pushover_user_key"] {
        let path = write_temp_config(
            "config.yaml",
            &format!("{MINIMAL_CONFIG}notifications:\n  pushover_token: \"app\"\n  {key}: \"user\"\n"),
        );
        let absent = path.with_file_name("absent.yaml.example");
        let loaded = load_app_config(absent.to_str().unwrap(), Some(path.to_str().unwrap()), false).unwrap();
        assert_eq!(loaded.notifications.pushover_user.as_deref(), Some("user"), "{key}");
        assert_eq!(status_of(&channel_readiness(&loaded.notifications), "pushover").status, "configured", "{key}");
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }
}

fn example_config_text() -> String {
    std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/config.yaml.example")).unwrap()
}

#[test]
fn test_example_config_leaves_notifications_off() {
    // config.yaml.example is the base layer of every deployment's config, so a channel
    // left uncommented there would be live for everyone.
    let example = concat!(env!("CARGO_MANIFEST_DIR"), "/config.yaml.example");
    let loaded = load_app_config(example, Some("/nonexistent/nocknock-config.yaml"), false).unwrap();
    let body = notification_status_body(&loaded.notifications);
    assert_eq!(body["any_configured"], json!(false));
    assert!(body["cockpit_url"].is_null());
}

#[test]
fn test_example_notifications_block_uses_keys_the_worker_reads() {
    // Uncomment the documented block exactly as an operator would and load it.
    let text = example_config_text();
    let mut block = String::new();
    let mut inside = false;
    for line in text.lines() {
        if line == "# notifications:" {
            inside = true;
            block.push_str("notifications:\n");
        } else if inside {
            match line.strip_prefix("#   ") {
                Some(rest) => block.push_str(&format!("  {rest}\n")),
                None => break,
            }
        }
    }
    assert!(inside, "config.yaml.example must document a `# notifications:` block");
    let doc: serde_yaml::Value = serde_yaml::from_str(&block).unwrap();
    let keys: Vec<String> = doc["notifications"]
        .as_mapping()
        .unwrap()
        .keys()
        .map(|k| k.as_str().unwrap().to_string())
        .collect();
    assert_eq!(
        keys,
        vec!["telegram_bot_token", "telegram_chat_id", "pushover_token", "pushover_user", "webhook_url", "cockpit_url"]
    );
    let path = write_temp_config("config.yaml", &format!("{MINIMAL_CONFIG}{block}"));
    let absent = path.with_file_name("absent.yaml.example");
    let loaded = load_app_config(absent.to_str().unwrap(), Some(path.to_str().unwrap()), false).unwrap();
    let n = &loaded.notifications;
    assert!(channel_readiness(n).iter().all(|r| r.status == "configured"), "{:?}", channel_readiness(n));
    // The documented link opens the Lite cockpit, which is /?view=lite and not /lite.
    assert!(n.cockpit_url.as_deref().unwrap().ends_with("/?view=lite"), "{:?}", n.cockpit_url);
    assert_eq!(cockpit_url_note(n.cockpit_url.as_deref()), None);
    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}

#[test]
fn test_cockpit_url_note_catches_the_old_example() {
    assert_eq!(cockpit_url_note(None), None);
    assert_eq!(cockpit_url_note(Some("")), None);
    assert_eq!(cockpit_url_note(Some("http://chudmax:3000/?view=lite")), None);
    assert_eq!(cockpit_url_note(Some("https://chudmax:3443/?view=lite")), None);
    assert!(cockpit_url_note(Some("https://your-host:3000/lite")).unwrap().contains("/?view=lite"));
    assert!(cockpit_url_note(Some("http://chudmax:3000/lite/")).unwrap().contains("/?view=lite"));
    assert!(cockpit_url_note(Some("chudmax:3000/?view=lite")).unwrap().contains("http://"));
}

// ---------------- Text, redaction, rate limit ----------------

#[test]
fn test_test_notification_is_labelled_a_test_and_nothing_else() {
    let text = test_notification_text(t("2026-11-15T23:40:05Z"), Some("http://chudmax:3000/?view=lite"));
    assert!(text.starts_with("🧪 NockNock TEST notification, sent on request at 2026-11-15 23:40:05 UTC."), "{text}");
    assert!(text.contains("This is not a directive. Nothing changed in the wallet or the plan"), "{text}");
    assert!(text.ends_with("\n→ http://chudmax:3000/?view=lite"), "{text}");
    // It must never read as a real push: none of the directive vocabulary appears.
    for word in ["directive change", "DCA", "HOLD", "DEFENSIVE", "RANGE EXIT", "HARVEST", "REBUILD", "Buy", "buy", "Sell", "sell"] {
        assert!(!text.contains(word), "{word:?} in: {text}");
    }
    assert!(!test_notification_text(t("2026-11-15T23:40:05Z"), None).contains('→'));
}

#[test]
fn test_secrets_are_redacted_from_details() {
    let c = cfg(Some(("123456:SECRET-bot", "42")), Some(("app-SECRET", "user-SECRET")), Some("https://hooks.example/T000/SECRET"));
    let secrets = notification_secrets(&c);
    assert_eq!(secrets.len(), 4);
    let leaky = "error sending request for url (https://api.telegram.org/bot123456:SECRET-bot/sendMessage) app-SECRET user-SECRET https://hooks.example/T000/SECRET";
    let clean = redact_secrets(leaky, &secrets);
    assert!(!clean.contains("SECRET"), "{clean}");
    assert_eq!(clean.matches("[redacted]").count(), 4);
    // The status body names channels and keys, never values.
    let mut with_link = c.clone();
    with_link.cockpit_url = Some("http://chudmax:3000/?view=lite".to_string());
    let body = notification_status_body(&with_link).to_string();
    assert!(!body.contains("SECRET"), "{body}");
    assert!(body.contains("http://chudmax:3000/?view=lite"));
}

#[test]
fn test_provider_error_detail_reads_each_providers_answer() {
    // Shapes from the providers' own API documentation.
    assert_eq!(
        provider_error_detail("telegram", 401, r#"{"ok":false,"error_code":401,"description":"Unauthorized"}"#),
        "HTTP 401: Unauthorized"
    );
    assert_eq!(
        provider_error_detail("telegram", 400, r#"{"ok":false,"error_code":400,"description":"Bad Request: chat not found"}"#),
        "HTTP 400: Bad Request: chat not found"
    );
    assert_eq!(
        provider_error_detail("pushover", 400, r#"{"user":"invalid","errors":["user identifier is invalid"],"status":0,"request":"5042853c-402d-4a18-abcb-168734a801de"}"#),
        "HTTP 400: user identifier is invalid"
    );
    // A webhook's body is arbitrary, and never echoed.
    assert_eq!(provider_error_detail("webhook", 500, "<html>secret stack trace</html>"), "HTTP 500");
    assert_eq!(provider_error_detail("telegram", 502, "Bad Gateway"), "HTTP 502");
    assert!(provider_error_detail("telegram", 400, &format!(r#"{{"description":"{}"}}"#, "x".repeat(500))).len() < 220);
}

#[test]
fn test_long_messages_are_cut_to_the_providers_limit_and_keep_the_link() {
    // Short text is untouched.
    assert_eq!(fit_message("hello\n→ http://h/?view=lite", 1024), "hello\n→ http://h/?view=lite");
    // A directive whose details list LP rebuild steps can pass Pushover's 1024 characters.
    let details = "Suggested: add 156.01 USDC + 4532 NOCK ($211.70) to v4 USDC/NOCK 3.00% #3163450. ".repeat(20);
    let text = format!("🛡️ NockNock directive change: DCA TRANCHE DUE (2 of 5)\n{}\n→ http://chudmax:3000/?view=lite", details);
    assert!(text.chars().count() > 1024);
    let cut = fit_message(&text, PUSHOVER_MESSAGE_LIMIT);
    assert_eq!(cut.chars().count(), 1024);
    assert!(cut.starts_with("🛡️ NockNock directive change: DCA TRANCHE DUE (2 of 5)\n"), "{cut}");
    assert!(cut.ends_with("…\n→ http://chudmax:3000/?view=lite"), "{cut}");
    // Without a link the cut is still marked, and multi-byte characters are never split.
    let no_link = fit_message(&"é".repeat(2000), 1024);
    assert_eq!(no_link.chars().count(), 1024);
    assert!(no_link.ends_with('…'));
    // A last line that only looks like a link but is the whole budget is not kept.
    assert_eq!(fit_message(&format!("x\n→ {}", "y".repeat(50)), 10).chars().count(), 10);
}

#[tokio::test]
async fn test_a_long_directive_reaches_pushover_within_its_limit() {
    let (base, seen) = provider_stand_in().await;
    let details = "add 65.86 USDC + 5678 NOCK ($135.64) to Aero USDC/NOCK 0.30% (full range); ".repeat(30);
    let text = cycle_notification(Some("A"), "DCA TRANCHE DUE (2 of 5)", &details, None, None, Some("http://chudmax:3000/?view=lite")).unwrap();
    assert!(text.chars().count() > PUSHOVER_MESSAGE_LIMIT);
    let c = cfg(Some(("good", "42")), Some(("good", "user-key")), None);
    let reports = deliver_notifications(&Client::new(), &c, &endpoints(&base), &text).await;
    assert!(reports.iter().filter(|r| r.channel != "webhook").all(|r| r.status == "delivered"), "{reports:?}");
    let seen = seen.lock().unwrap();
    let got = |who: &str| seen.iter().find(|(w, _)| w == who).map(|(_, b)| b.clone()).unwrap();
    let pushover = got("pushover")["message"].as_str().unwrap().to_string();
    assert_eq!(pushover.chars().count(), PUSHOVER_MESSAGE_LIMIT);
    assert!(pushover.ends_with("…\n→ http://chudmax:3000/?view=lite"));
    // Telegram's limit is higher, so it carries the whole text.
    assert_eq!(got("telegram")["text"].as_str().unwrap(), text);
}

#[test]
fn test_test_send_cooldown() {
    use std::time::Duration;
    assert_eq!(test_send_retry_after(None, 30), None);
    assert_eq!(test_send_retry_after(Some(Duration::from_secs(0)), 30), Some(30));
    assert_eq!(test_send_retry_after(Some(Duration::from_secs(12)), 30), Some(18));
    assert_eq!(test_send_retry_after(Some(Duration::from_secs(30)), 30), None);
    assert_eq!(test_send_retry_after(Some(Duration::from_secs(900)), 30), None);
}

// ---------------- Delivery against a local stand-in ----------------

type Seen = Arc<std::sync::Mutex<Vec<(String, serde_json::Value)>>>;

/// A local server that answers the way each provider does: Telegram and Pushover accept
/// only the credential "good"; the webhook accepts anything on /hook and fails on /down.
async fn provider_stand_in() -> (String, Seen) {
    use axum::extract::Path;
    use axum::routing::post;
    let seen: Seen = Arc::new(std::sync::Mutex::new(Vec::new()));
    let (s1, s2, s3) = (seen.clone(), seen.clone(), seen.clone());
    let app = axum::Router::new()
        .route(
            "/:bot/sendMessage",
            post(move |Path(bot): Path<String>, Json(body): Json<serde_json::Value>| async move {
                s1.lock().unwrap().push(("telegram".to_string(), body));
                if bot == "botgood" {
                    (StatusCode::OK, Json(json!({ "ok": true, "result": { "message_id": 1 } })))
                } else {
                    (StatusCode::UNAUTHORIZED, Json(json!({ "ok": false, "error_code": 401, "description": "Unauthorized" })))
                }
            }),
        )
        .route(
            "/1/messages.json",
            post(move |Json(body): Json<serde_json::Value>| async move {
                let ok = body["token"] == "good";
                s2.lock().unwrap().push(("pushover".to_string(), body));
                if ok {
                    (StatusCode::OK, Json(json!({ "status": 1, "request": "647d2300-702c-4b38-8b2f-d56326ae460b" })))
                } else {
                    (StatusCode::BAD_REQUEST, Json(json!({ "token": "invalid", "errors": ["application token is invalid"], "status": 0 })))
                }
            }),
        )
        .route(
            "/hook",
            post(move |Json(body): Json<serde_json::Value>| async move {
                s3.lock().unwrap().push(("webhook".to_string(), body));
                StatusCode::NO_CONTENT
            }),
        )
        .route("/down", post(|| async { (StatusCode::INTERNAL_SERVER_ERROR, "boom: the hook's own secrets") }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    (base, seen)
}

fn endpoints(base: &str) -> NotifyEndpoints {
    NotifyEndpoints { telegram_base: base.to_string(), pushover_url: format!("{base}/1/messages.json") }
}

#[tokio::test]
async fn test_test_send_reaches_every_configured_channel_and_reports_each() {
    let (base, seen) = provider_stand_in().await;
    let hook = format!("{base}/hook");
    let mut c = cfg(Some(("good", "42")), Some(("good", "user-key")), Some(&hook));
    c.cockpit_url = Some("http://chudmax:3000/?view=lite".to_string());
    let body = run_notification_test(&Client::new(), &c, &endpoints(&base), t("2026-11-15T23:40:05Z")).await;

    assert_eq!(body["sent"], json!(true));
    assert_eq!(body["any_configured"], json!(true));
    assert_eq!(body["sent_at"], json!("2026-11-15T23:40:05+00:00"));
    assert!(body["message"].as_str().unwrap().starts_with("3 of 3 configured channel(s) accepted the test."));
    let channels: Vec<ChannelReport> = serde_json::from_value(body["channels"].clone()).unwrap();
    assert_eq!(status_of(&channels, "telegram").detail, "accepted by Telegram (HTTP 200)");
    assert_eq!(status_of(&channels, "pushover").detail, "accepted by Pushover (HTTP 200)");
    assert_eq!(status_of(&channels, "webhook").detail, "accepted by the webhook (HTTP 204)");
    assert!(channels.iter().all(|r| r.status == "delivered"));

    // Each provider received the labelled test text in the field it reads.
    let text = body["text"].as_str().unwrap();
    assert!(text.starts_with("🧪 NockNock TEST notification"));
    let seen = seen.lock().unwrap();
    assert_eq!(seen.len(), 3);
    let got = |who: &str| seen.iter().find(|(w, _)| w == who).map(|(_, b)| b.clone()).unwrap();
    assert_eq!(got("telegram"), json!({ "chat_id": "42", "text": text }));
    assert_eq!(got("pushover"), json!({ "token": "good", "user": "user-key", "message": text }));
    assert_eq!(got("webhook"), json!({ "text": text }));
    // The response names no credential.
    let raw = body.to_string();
    assert!(!raw.contains("user-key") && !raw.contains(&hook), "{raw}");
}

#[tokio::test]
async fn test_test_send_reports_a_bad_credential_per_channel_without_leaking_it() {
    let (base, _seen) = provider_stand_in().await;
    let down = format!("{base}/down");
    let c = cfg(Some(("111:WRONG-bot-token", "42")), Some(("WRONG-app-token", "WRONG-user-key")), Some(&down));
    let body = run_notification_test(&Client::new(), &c, &endpoints(&base), t("2026-11-15T23:40:05Z")).await;

    assert_eq!(body["sent"], json!(true));
    assert!(body["message"].as_str().unwrap().starts_with("0 of 3 configured channel(s) accepted the test."));
    let channels: Vec<ChannelReport> = serde_json::from_value(body["channels"].clone()).unwrap();
    assert!(channels.iter().all(|r| r.status == "failed"), "{channels:?}");
    assert_eq!(status_of(&channels, "telegram").detail, "HTTP 401: Unauthorized");
    assert_eq!(status_of(&channels, "pushover").detail, "HTTP 400: application token is invalid");
    assert_eq!(status_of(&channels, "webhook").detail, "HTTP 500");
    let raw = body.to_string();
    assert!(!raw.contains("WRONG") && !raw.contains("/down") && !raw.contains("boom"), "{raw}");
}

#[tokio::test]
async fn test_one_good_channel_and_one_bad_are_reported_separately() {
    let (base, seen) = provider_stand_in().await;
    let c = cfg(Some(("good", "42")), Some(("WRONG-app-token", "user-key")), None);
    let body = run_notification_test(&Client::new(), &c, &endpoints(&base), t("2026-11-15T23:40:05Z")).await;
    let channels: Vec<ChannelReport> = serde_json::from_value(body["channels"].clone()).unwrap();
    assert_eq!(status_of(&channels, "telegram").status, "delivered");
    assert_eq!(status_of(&channels, "pushover").status, "failed");
    assert_eq!(status_of(&channels, "webhook").status, "not_configured");
    assert!(body["message"].as_str().unwrap().starts_with("1 of 2 configured channel(s) accepted the test."));
    // Nothing was posted for the channel that is not configured.
    assert!(seen.lock().unwrap().iter().all(|(who, _)| who != "webhook"));
}

#[tokio::test]
async fn test_test_send_with_no_channel_says_so_and_sends_nothing() {
    let (base, seen) = provider_stand_in().await;
    let body = run_notification_test(&Client::new(), &NotificationConfig::default(), &endpoints(&base), t("2026-11-15T23:40:05Z")).await;
    assert_eq!(body["sent"], json!(false));
    assert_eq!(body["any_configured"], json!(false));
    assert!(body["message"].as_str().unwrap().starts_with("No notification channel is configured, so nothing was sent."));
    assert!(body.get("text").is_none() && body.get("sent_at").is_none());
    let channels: Vec<ChannelReport> = serde_json::from_value(body["channels"].clone()).unwrap();
    assert!(channels.iter().all(|r| r.status == "not_configured"));
    assert!(seen.lock().unwrap().is_empty());

    // Half a channel is reported as incomplete, and still nothing is sent.
    let mut half = NotificationConfig::default();
    half.telegram_bot_token = Some("good".to_string());
    let body = run_notification_test(&Client::new(), &half, &endpoints(&base), t("2026-11-15T23:40:05Z")).await;
    assert_eq!(body["sent"], json!(false));
    assert_eq!(body["channels"][0]["status"], json!("incomplete"));
    assert!(seen.lock().unwrap().is_empty());
}

#[tokio::test]
async fn test_an_unreachable_provider_is_a_failure_with_no_token_in_it() {
    // Nothing listens here: bind a port, then drop the listener.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    drop(listener);
    let c = cfg(Some(("999:UNREACHABLE-token", "42")), None, None);
    let reports = deliver_notifications(&Client::new(), &c, &endpoints(&base), "hello").await;
    let telegram = status_of(&reports, "telegram");
    assert_eq!(telegram.status, "failed");
    assert!(telegram.detail.starts_with("could not connect"), "{}", telegram.detail);
    assert!(!telegram.detail.contains("UNREACHABLE") && !telegram.detail.contains("sendMessage"), "{}", telegram.detail);
}

#[tokio::test]
async fn test_a_real_directive_message_goes_out_through_the_same_path() {
    // The cycle's own text, as decided from two computed snapshots, delivered end to end.
    let raw = wallet_only(480.0, 1734.6054);
    let p = profile(0.0);
    let waiting = metrics(&raw, &p, &plan_after(1, BEFORE_DUE));
    let due = metrics(&raw, &p, &plan_after(1, AT_DUE));
    let text = pushed(&waiting, &due).unwrap();
    let (base, seen) = provider_stand_in().await;
    let reports = deliver_notifications(&Client::new(), &cfg(Some(("good", "42")), None, None), &endpoints(&base), &text).await;
    assert_eq!(status_of(&reports, "telegram").status, "delivered");
    let seen = seen.lock().unwrap();
    assert_eq!(seen.len(), 1);
    assert!(seen[0].1["text"].as_str().unwrap().starts_with("🛡️ NockNock directive change: DCA TRANCHE DUE (2 of 5)"));
}

// ---------------- The routes themselves ----------------

/// Worker state built from the shipped example config. The pool is lazy and never
/// connects: these handlers read config only.
fn state_from_example_config() -> Arc<AppState> {
    let example = concat!(env!("CARGO_MANIFEST_DIR"), "/config.yaml.example");
    let config = load_app_config(example, Some("/nonexistent/nocknock-config.yaml"), false).unwrap();
    Arc::new(AppState {
        db: PgPool::connect_lazy("postgresql://nobody@127.0.0.1:1/none").unwrap(),
        http_client: Client::new(),
        config: Arc::new(config),
        latest_snapshot: Mutex::new(None),
        last_sync_duration_ms: std::sync::atomic::AtomicU64::new(0),
        last_ingest_error: std::sync::Mutex::new(None),
    })
}

async fn json_of(resp: Response) -> (StatusCode, serde_json::Value) {
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

#[tokio::test]
async fn test_routes_on_an_unconfigured_worker_say_nothing_is_configured() {
    let state = state_from_example_config();
    let (code, status) = json_of(notification_status_handler(Extension(state.clone())).await.into_response()).await;
    assert_eq!(code, StatusCode::OK);
    assert_eq!(status["any_configured"], json!(false));
    assert_eq!(status["channels"].as_array().unwrap().len(), 3);
    assert_eq!(status["test_cooldown_seconds"], json!(TEST_SEND_COOLDOWN_SECONDS));

    // The test route answers 200 with sent=false, twice in a row: with nothing to send
    // there is nothing to rate-limit, and nothing leaves the machine.
    for _ in 0..2 {
        let (code, body) = json_of(notification_test_handler(Extension(state.clone())).await).await;
        assert_eq!(code, StatusCode::OK);
        assert_eq!(body["sent"], json!(false));
        assert!(body["message"].as_str().unwrap().starts_with("No notification channel is configured"));
    }
}
