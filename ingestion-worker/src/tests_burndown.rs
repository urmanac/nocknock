//! v0.8.1 burn-down (#130-#137): one section per issue. Nothing here touches the
//! network beyond 127.0.0.1 or needs a database or a config.yaml.
use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};

// ---------------- #134: one ingestion cycle at a time ----------------

/// A stand-in cycle: counts how many run at once and how many ran, and takes long
/// enough that an unserialised second cycle would overlap it.
async fn fake_cycle(running: Arc<AtomicUsize>, peak: Arc<AtomicUsize>, ran: Arc<AtomicUsize>) -> Result<()> {
    let now = running.fetch_add(1, Ordering::SeqCst) + 1;
    peak.fetch_max(now, Ordering::SeqCst);
    tokio::time::sleep(std::time::Duration::from_millis(40)).await;
    running.fetch_sub(1, Ordering::SeqCst);
    ran.fetch_add(1, Ordering::SeqCst);
    Ok(())
}

#[tokio::test]
async fn test_scheduled_and_requested_cycles_never_interleave() {
    let gate = Arc::new(CycleGate::default());
    let (running, peak, ran) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
    let mut tasks = Vec::new();
    for i in 0..6 {
        let (g, r, p, n) = (gate.clone(), running.clone(), peak.clone(), ran.clone());
        tasks.push(tokio::spawn(async move {
            if i % 2 == 0 {
                g.run(|| fake_cycle(r, p, n)).await
            } else {
                g.run_or_join(|| fake_cycle(r, p, n)).await
            }
        }));
    }
    for t in tasks {
        t.await.unwrap().unwrap();
    }
    assert_eq!(peak.load(Ordering::SeqCst), 1, "two cycles ran at the same time");
    assert!(ran.load(Ordering::SeqCst) >= 3, "every scheduled cycle runs");
}

#[tokio::test]
async fn test_requests_behind_a_running_cycle_share_one_new_cycle() {
    let gate = Arc::new(CycleGate::default());
    let (running, peak, ran) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
    // A scheduled cycle is in flight.
    let first = {
        let (g, r, p, n) = (gate.clone(), running.clone(), peak.clone(), ran.clone());
        tokio::spawn(async move { g.run(|| fake_cycle(r, p, n)).await })
    };
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    // Five requests arrive while it runs: none may reuse it (it read the plan before
    // they arrived), and they need no more than one new cycle between them.
    let mut requests = Vec::new();
    for _ in 0..5 {
        let (g, r, p, n) = (gate.clone(), running.clone(), peak.clone(), ran.clone());
        requests.push(tokio::spawn(async move { g.run_or_join(|| fake_cycle(r, p, n)).await }));
    }
    first.await.unwrap().unwrap();
    for r in requests {
        r.await.unwrap().unwrap();
    }
    assert_eq!(ran.load(Ordering::SeqCst), 2, "the cycle in flight plus one for every waiting request");
    assert_eq!(peak.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn test_a_joined_request_gets_the_shared_cycles_error() {
    let gate = Arc::new(CycleGate::default());
    let slow_fail = || async {
        tokio::time::sleep(std::time::Duration::from_millis(30)).await;
        Err::<(), _>(anyhow::anyhow!("RPC read failed"))
    };
    let g = gate.clone();
    let inflight = tokio::spawn(async move { g.run(|| async { tokio::time::sleep(std::time::Duration::from_millis(30)).await; Ok(()) }).await });
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    let (g1, g2) = (gate.clone(), gate.clone());
    let a = tokio::spawn(async move { g1.run_or_join(slow_fail).await });
    let b = tokio::spawn(async move { g2.run_or_join(slow_fail).await });
    inflight.await.unwrap().unwrap();
    let (a, b) = (a.await.unwrap(), b.await.unwrap());
    assert_eq!(a.unwrap_err().to_string(), "RPC read failed");
    assert_eq!(b.unwrap_err().to_string(), "RPC read failed");
}

#[tokio::test]
async fn test_a_dropped_request_does_not_cancel_its_cycle() {
    // The handler spawns the cycle and awaits its JoinHandle; dropping the handler's
    // future (a client gone, nginx's 60 s timeout) must leave the cycle running.
    let gate = Arc::new(CycleGate::default());
    let finished = Arc::new(AtomicUsize::new(0));
    let (g, f) = (gate.clone(), finished.clone());
    let request = async move {
        let cycle = tokio::spawn(async move {
            g.run_or_join(|| async move {
                tokio::time::sleep(std::time::Duration::from_millis(40)).await;
                f.fetch_add(1, Ordering::SeqCst);
                Ok(())
            })
            .await
        });
        cycle.await
    };
    // The request is abandoned after 5 ms.
    assert!(tokio::time::timeout(std::time::Duration::from_millis(5), request).await.is_err());
    tokio::time::sleep(std::time::Duration::from_millis(80)).await;
    assert_eq!(finished.load(Ordering::SeqCst), 1, "the cycle ran to the end");
}

// ---------------- #133: LAN API hardening ----------------

fn burndown_plan_cfg() -> AccumulationPlanConfig {
    AccumulationPlanConfig {
        id: "2026-11-reentry".to_string(),
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

fn envelope(plan: Option<AccumulationPlanStatus>) -> SnapshotEnvelope {
    SnapshotEnvelope {
        meta: SnapshotMeta {
            generated_at: Utc::now(),
            state_hash: "test".to_string(),
            version: SNAPSHOT_SPEC_VERSION.to_string(),
            snapshot_id: Uuid::new_v4(),
        },
        data: SnapshotData {
            closed_positions: vec![],
            wallet_balances: vec![],
            positions: vec![],
            pool_mechanics: vec![],
            concentrated_liquidity: vec![],
            contract_fees: vec![],
            derived_metrics: DerivedMetrics { accumulation_plan: plan, ..Default::default() },
        },
    }
}

/// A worker with a plan, no RPC key and a database that is never reached.
fn offline_state(snapshot: Option<SnapshotEnvelope>) -> Arc<AppState> {
    let mut config: AppConfig = serde_yaml::from_str("alchemy_api_keys: [\"YOUR_ALCHEMY_API_KEY\"]\n").unwrap();
    config.accumulation_plan = Some(burndown_plan_cfg());
    Arc::new(AppState {
        db: PgPool::connect_lazy("postgres://nobody@127.0.0.1:1/none").unwrap(),
        http_client: Client::new(),
        config: Arc::new(config),
        latest_snapshot: Mutex::new(snapshot),
        last_sync_duration_ms: std::sync::atomic::AtomicU64::new(0),
        last_ingest_error: std::sync::Mutex::new(None),
        cycle_gate: CycleGate::default(),
        quote_refresh_at: std::sync::Mutex::new(None),
    })
}

fn due_plan() -> AccumulationPlanStatus {
    build_plan_status(&burndown_plan_cfg(), &PlanFills::default(), &PlanDirective::TrancheDue { usdc: 20.0, limited_by: None })
}

#[test]
fn test_cross_site_writes_are_refused_and_same_host_ones_pass() {
    use axum::http::Method;
    let post = Method::POST;
    let refused = |sfs, origin, host| cross_site_refusal(&post, sfs, origin, host).is_some();
    // curl, scripts and MCP clients send neither header.
    assert!(!refused(None, None, Some("chudmax:8081")));
    assert!(!refused(None, None, None));
    // The dashboard through nginx on :3000: nginx forwards Host without the port.
    assert!(!refused(Some("same-origin"), Some("http://chudmax:3000"), Some("chudmax")));
    // Behind the TLS proxy (Caddy keeps Host, nginx drops the port again).
    assert!(!refused(Some("same-origin"), Some("https://chudmax:3443"), Some("chudmax")));
    // The Vite dev server passes the browser's Host through.
    assert!(!refused(Some("same-origin"), Some("http://127.0.0.1:3000"), Some("127.0.0.1:3000")));
    assert!(!refused(Some("same-origin"), Some("http://[::1]:3000"), Some("[::1]:3000")));
    // A page on another port of the operator's own host.
    assert!(!refused(Some("same-site"), Some("http://chudmax:3000"), Some("chudmax:8081")));
    // Case does not matter in a host name.
    assert!(!refused(None, Some("http://ChudMax:3000"), Some("chudmax")));

    // Any page elsewhere.
    assert!(refused(Some("cross-site"), Some("https://evil.example"), Some("chudmax:8081")));
    assert!(refused(Some("cross-site"), None, Some("chudmax:8081")));
    assert!(refused(None, Some("https://evil.example"), Some("chudmax:8081")));
    // A sandboxed page or a file sends "null".
    assert!(refused(None, Some("null"), Some("chudmax:8081")));
    // DNS rebinding: the page's name points at the host, but its Origin is its own name.
    assert!(refused(Some("same-origin"), Some("http://rebind.example:8081"), Some("chudmax:8081")));
    // An Origin with no Host to compare against.
    assert!(refused(None, Some("http://chudmax:3000"), None));
    // Reads are never refused.
    assert!(cross_site_refusal(&Method::GET, Some("cross-site"), Some("https://evil.example"), Some("chudmax")).is_none());
}

#[tokio::test]
async fn test_the_router_refuses_a_cross_site_post_before_any_handler_runs() {
    let state = offline_state(None);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, api_router(state)).await.unwrap() });
    let base = format!("http://{addr}");
    let client = Client::new();

    // A page elsewhere asks for an ingest cycle: 403, and no cycle runs (the database
    // here is unreachable, so a cycle would answer 500).
    let r = client
        .post(format!("{base}/v1/ingest"))
        .header("Origin", "https://evil.example")
        .header("Sec-Fetch-Site", "cross-site")
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), reqwest::StatusCode::FORBIDDEN);
    let body: serde_json::Value = r.json().await.unwrap();
    assert_eq!(body["error"].as_str(), Some("CROSS_SITE_REFUSED"));
    assert!(!body.to_string().contains("alchemy"), "no RPC URL in an error");

    for path in ["/v1/accumulation/quotes/refresh", "/v1/notifications/test"] {
        let r = client.post(format!("{base}{path}")).header("Origin", "https://evil.example").send().await.unwrap();
        assert_eq!(r.status(), reqwest::StatusCode::FORBIDDEN, "{path}");
    }

    // The same request without Origin (curl) reaches the handler: no snapshot yet.
    let r = client.post(format!("{base}/v1/accumulation/quotes/refresh")).send().await.unwrap();
    assert_eq!(r.status(), reqwest::StatusCode::SERVICE_UNAVAILABLE);
    // And so does the dashboard's own POST, through nginx (Host without the port).
    let r = client
        .post(format!("{base}/v1/accumulation/quotes/refresh"))
        .header("Origin", "http://127.0.0.1:3000")
        .header("Sec-Fetch-Site", "same-origin")
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), reqwest::StatusCode::SERVICE_UNAVAILABLE);
    // Reads are untouched.
    let r = client.get(format!("{base}/health")).header("Origin", "https://evil.example").send().await.unwrap();
    assert_eq!(r.status(), reqwest::StatusCode::OK);
}

#[test]
fn test_the_refresh_slot_is_held_for_the_cooldown() {
    let slot = std::sync::Mutex::new(None);
    let t0 = std::time::Instant::now();
    assert_eq!(claim_quote_refresh(&slot, t0), None, "the first refresh reads");
    let wait = claim_quote_refresh(&slot, t0 + std::time::Duration::from_secs(3)).expect("inside the window");
    assert_eq!(wait, std::time::Duration::from_secs(QUOTE_REFRESH_COOLDOWN_SECONDS - 3));
    // A refused call does not extend the window.
    assert_eq!(claim_quote_refresh(&slot, t0 + std::time::Duration::from_secs(QUOTE_REFRESH_COOLDOWN_SECONDS)), None);
    assert!(claim_quote_refresh(&slot, t0 + std::time::Duration::from_secs(QUOTE_REFRESH_COOLDOWN_SECONDS + 1)).is_some());
}

#[tokio::test]
async fn test_a_refresh_inside_the_cooldown_answers_with_the_quotes_held() {
    async fn body(resp: Response) -> (StatusCode, HeaderMap, serde_json::Value) {
        let (status, headers) = (resp.status(), resp.headers().clone());
        let bytes = axum::body::to_bytes(resp.into_body(), 1 << 20).await.unwrap();
        (status, headers, serde_json::from_slice(&bytes).unwrap())
    }
    let state = offline_state(Some(envelope(Some(due_plan()))));
    // The first refresh reads (no key here: "unavailable", which is still an answer)
    // and attaches its result.
    let (status, _, first) = body(refresh_route_quotes_handler(Extension(state.clone())).await).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(first["route_quotes"]["status"].as_str(), Some("unavailable"));
    // A second one moments later is answered from memory, with the same quotes.
    let (status, _, second) = body(refresh_route_quotes_handler(Extension(state.clone())).await).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(second, first);

    // Inside the window with nothing held: 429 with Retry-After, never an empty quote.
    let bare = offline_state(Some(envelope(Some(due_plan()))));
    *bare.quote_refresh_at.lock().unwrap() = Some(std::time::Instant::now());
    let (status, headers, err) = body(refresh_route_quotes_handler(Extension(bare)).await).await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(err["error"].as_str(), Some("REFRESH_COOLDOWN"));
    let secs: u64 = headers[axum::http::header::RETRY_AFTER].to_str().unwrap().parse().unwrap();
    assert!((1..=QUOTE_REFRESH_COOLDOWN_SECONDS).contains(&secs));
}

#[test]
fn test_stored_quotes_are_pruned_only_where_no_fill_can_use_them() {
    let now = DateTime::parse_from_rfc3339("2026-11-17T12:00:00Z").unwrap().with_timezone(&Utc);
    let at = |mins_ago: i64| now - chrono::Duration::minutes(mins_ago);
    // One row every 5 minutes for 10 hours, oldest first: (id, block, quoted_at).
    let rows: Vec<(i64, i64, DateTime<Utc>)> =
        (0..120).map(|i| (i as i64, 1000 + i as i64 * 150, at(600 - i as i64 * 5))).collect();
    // No recorded fill: nothing is pruned, whatever its age.
    assert!(stored_quotes_to_prune(&rows, None, &[], now).is_empty());

    // A recorded fill at block 10000 (row 60 sits at 10000). Rows before it, read more
    // than an hour ago, go; the rest stay.
    let gone = stored_quotes_to_prune(&rows, Some(10_000), &[], now);
    assert_eq!(gone, (0..60).collect::<Vec<i64>>());

    // A swap still on offer at block 5100 keeps the latest row read before it (row 27,
    // block 5050): confirming it later still finds its quote.
    let gone = stored_quotes_to_prune(&rows, Some(10_000), &[5_100], now);
    assert!(!gone.contains(&27));
    assert_eq!(gone.len(), 59);

    // Rows read within the last hour stay even below the fill.
    let recent: Vec<(i64, i64, DateTime<Utc>)> = vec![(1, 100, at(30)), (2, 200, at(90))];
    assert_eq!(stored_quotes_to_prune(&recent, Some(300), &[], now), vec![2]);

    // What is left still answers every fill the same way: for a fill above the latest
    // recorded one, the latest row before it is unchanged.
    let kept: Vec<_> = rows.iter().filter(|r| !stored_quotes_to_prune(&rows, Some(10_000), &[5_100], now).contains(&r.0)).collect();
    let latest_before = |rs: &[&(i64, i64, DateTime<Utc>)], b: i64| rs.iter().filter(|r| r.1 < b).max_by_key(|r| r.1).map(|r| r.0);
    let all: Vec<_> = rows.iter().collect();
    for fill_block in [5_100, 10_001, 12_000, 18_000] {
        assert_eq!(latest_before(&kept, fill_block), latest_before(&all, fill_block), "fill at {fill_block}");
    }
}

// ---------------- #130: a cycle that could not read the plan ledger ----------------

fn fills_at(count: i64, spent: f64) -> PlanFills {
    PlanFills {
        count,
        spent_usdc: spent,
        acquired_nock: spent / 0.011,
        last_at: Some(DateTime::parse_from_rfc3339("2026-11-16T10:00:00Z").unwrap().with_timezone(&Utc)),
        first_price: Some(0.011),
    }
}

fn unreadable_plan() -> AccumulationPlanStatus {
    build_plan_status(
        &burndown_plan_cfg(),
        &PlanFills::default(),
        &PlanDirective::Blocked { reason: PLAN_LEDGER_UNREADABLE_REASON.to_string() },
    )
}

#[test]
fn test_an_unread_ledger_reports_unknown_progress_not_zero() {
    let ps = unreadable_plan();
    assert!(ps.progress_unknown);
    assert_eq!((ps.spent_usdc.as_str(), ps.remaining_usdc.as_str(), ps.acquired_nock.as_str()), ("unknown", "unknown", "unknown"));
    let json = serde_json::to_value(&ps).unwrap();
    assert_eq!(json["progress_unknown"], serde_json::json!(true));

    let (action, details) = plan_advice(&ps, 0.0113);
    assert_eq!(action, "DCA BLOCKED (ledger unreadable)");
    assert!(details.contains("progress is unknown"), "{details}");
    for wrong in ["$0.00", "0 verified", "1 of 5", "(1 of"] {
        assert!(!details.contains(wrong) && !action.contains(wrong), "{wrong} in {action} / {details}");
    }

    // A cycle that read the ledger is unchanged, and the field is absent from it.
    let good = build_plan_status(&burndown_plan_cfg(), &fills_at(3, 60.0), &PlanDirective::Waiting { next_due_at: Utc::now() });
    assert!(!good.progress_unknown);
    assert_eq!(good.spent_usdc, "60.00");
    assert!(serde_json::to_value(&good).unwrap().get("progress_unknown").is_none());
    assert_eq!(plan_advice(&good, 0.0113).0, "DCA ON SCHEDULE (4 of 5 next)");
    // A snapshot from before 2.8.1 still reads.
    let old: AccumulationPlanStatus = serde_json::from_value(serde_json::to_value(&good).unwrap()).unwrap();
    assert!(!old.progress_unknown);
}

#[test]
fn test_mcp_text_says_unknown_for_an_unread_ledger() {
    let data = serde_json::json!({ "derived_metrics": { "accumulation_plan": unreadable_plan() } });
    let text = format_accumulation_plan_text(&data);
    assert!(text.contains("Progress: unknown of $100.00"), "{text}");
    assert!(!text.contains("$0.00") && !text.contains("1 of 5") && !text.contains("verified buy"), "{text}");
}

#[test]
fn test_an_unread_ledger_cycle_is_not_a_directive_change() {
    let waiting = build_plan_status(&burndown_plan_cfg(), &fills_at(3, 60.0), &PlanDirective::Waiting { next_due_at: Utc::now() });
    let good_action = "DCA ON SCHEDULE (4 of 5 next)";
    let bad = unreadable_plan();
    let bad_action = plan_advice(&bad, 0.0113).0;
    let notify = |prev: NotifyBaseline, last: Option<NotifyBaseline>, action: &str, plan: &AccumulationPlanStatus| {
        let (pa, pp) = notification_baseline(prev, last, action, Some(plan));
        cycle_notification_with_closed(pa.as_deref(), action, "details", pp.as_ref(), Some(plan), None, &[], None)
    };
    let good_base: NotifyBaseline = (Some(good_action.to_string()), Some(waiting.clone()));

    // good -> unreadable: silent.
    assert_eq!(notify(good_base.clone(), Some(good_base.clone()), &bad_action, &bad), None);
    // unreadable -> good, same directive as the last good read: silent.
    let bad_base: NotifyBaseline = (Some(bad_action.clone()), Some(bad.clone()));
    assert_eq!(notify(bad_base.clone(), Some(good_base.clone()), good_action, &waiting), None);
    // unreadable -> good with a real change since the last good read: pushed once.
    let due = build_plan_status(&burndown_plan_cfg(), &fills_at(3, 60.0), &PlanDirective::TrancheDue { usdc: 20.0, limited_by: None });
    let pushed = notify(bad_base.clone(), Some(good_base.clone()), "DCA TRANCHE DUE (4 of 5)", &due).expect("a change");
    assert!(pushed.contains("DCA TRANCHE DUE (4 of 5)"));
    // The last buy was recorded just before the unreadable cycle: the completion is
    // announced on the next good read, against the last cycle that read the ledger.
    let complete = build_plan_status(&burndown_plan_cfg(), &fills_at(5, 100.0), &PlanDirective::Complete);
    let text = notify(bad_base.clone(), Some(good_base.clone()), "HOLD & ACCRETE", &complete).expect("completion pushed");
    assert!(text.contains("is complete"), "{text}");
    // After a restart nothing is remembered: nothing is compared, as on a first boot.
    assert_eq!(notify(bad_base, None, good_action, &waiting), None);
}
