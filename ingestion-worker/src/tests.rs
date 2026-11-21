    use super::*;

    // Canonical Uniswap v3 NonfungiblePositionManager on Base — used by fixtures to
    // mirror the npm_address configured for the live v3 position (ONCHAIN_INTEGRATION.md §5).
    const UNISWAP_V3_NPM_BASE: &str = "0x03a520b32C04BF3bEEf7BEb72E919cf822Ed34f1";

    #[test]
    fn test_claim_fees_hex_parsing() {
        // Simulated return from 0xd294f093 (claimFees)
        // c0 = 256220 (0.25622 USDC with 6 decimals) -> 0x3e8dc
        // c1 = 86924344947911169 (8.69243 NOCK with 16 decimals) -> 0x134d13b2ef42a01
        let hex_str = "0x000000000000000000000000000000000000000000000000000000000003e8dc0000000000000000000000000000000000000000000000000134d13b2ef42a01";
        let clean = hex_str.trim_start_matches("0x");
        assert!(clean.len() >= 128);

        let c0 = u128::from_str_radix(&clean[..64], 16).unwrap();
        let c1 = u128::from_str_radix(&clean[64..128], 16).unwrap();

        let usdc = c0 as f64 / 1e6;
        let nock = c1 as f64 / 1e16;

        assert!((usdc - 0.25622).abs() < 1e-4);
        assert!((nock - 8.69243).abs() < 1e-4);
    }

    #[test]
    fn test_derived_metrics_defensive_lock_under_100k() {
        let mut raw = four_position_fixture();
        // 80,000 wallet NOCK + ~15,296 LP NOCK < 100,000 floor
        raw.wallet_balances[0].amount = "80000.0".to_string();
        raw.wallet_balances[0].amount_usd = "1664.0".to_string();
        // The floor is the operator's own number (v0.5.1: there is no default floor).
        let profile = operator_profile_2026_11();
        let metrics = compute_derived_metrics(&raw, 2500.0, &profile, StructuralChannels { channels: vec![] }, &[], &LedgerAggregate::default(), &MarketRegime::default());
        assert_eq!(metrics.global_status.shot_advice_action, "DEFENSIVE LOCK (Floor Recovery)");
        let d = &metrics.global_status.shot_advice_details;
        assert!(d.contains("is below the 100000 floor by"), "{d}");
        assert!(d.contains("accumulation_plan"));
        for banned in ["CRITICAL", "locked", "sovereign foundation", "Deploy $", "Buy $"] {
            assert!(!d.contains(banned), "{banned} in: {d}");
        }
        // With the default profile there is no floor, so no lock.
        let none = compute_derived_metrics(&raw, 2500.0, &OperatorProfile::default(), StructuralChannels { channels: vec![] }, &[], &LedgerAggregate::default(), &MarketRegime::default());
        assert!(!none.global_status.shot_advice_action.contains("DEFENSIVE LOCK"));
        assert_eq!(none.global_status.nock_floor_target, "0");
    }

    /// An operator's own profile, every number set explicitly (floor 100,000, tax ratio
    /// 0.40, weights 8000/1500/500, the values these tests were written against). The
    /// code defaults choose nothing: no floor, no tax ratio, no target weights.
    pub(crate) fn operator_profile_2026_11() -> OperatorProfile {
        OperatorProfile {
            treasury_floor_nock: 100_000.0,
            lp_floor_tolerance_bps: 0,
            tax_reserve_ratio: 0.40,
            target_weights_bps: TargetWeightsBps { treasury: 8000, uniswap_lp: 1500, cash: 500 },
            cash_reserve_usdc: None,
        }
    }

    /// The same explicit numbers without a floor.
    fn operator_profile_no_floor() -> OperatorProfile {
        OperatorProfile { treasury_floor_nock: 0.0, ..operator_profile_2026_11() }
    }

    #[test]
    fn test_derived_metrics_batch_harvest_above_5_dollars() {
        let mut raw = four_position_fixture();
        // Uncollected USDC fees on the NOCK/USDC position push the aggregate over $5.
        raw.positions[1].fee0 = 6.0;
        raw.live_gas_cost_usd = 0.005;

        let profile = OperatorProfile::default();
        let metrics = compute_derived_metrics(&raw, 2500.0, &profile, StructuralChannels { channels: vec![] }, &[], &LedgerAggregate::default(), &MarketRegime::default());
        assert!(metrics.global_status.shot_advice_action.contains("BATCH HARVEST SHOT"));
    }

    #[test]
    fn test_derived_metrics_hold_and_accrete() {
        let raw = four_position_fixture();
        let profile = OperatorProfile::default();
        let metrics = compute_derived_metrics(&raw, 2500.0, &profile, StructuralChannels { channels: vec![] }, &[], &LedgerAggregate::default(), &MarketRegime::default());
        assert!(metrics.global_status.shot_advice_action.contains("HOLD & ACCRETE"));
        // Advice text is Uniswap-only now: no Aerodrome wording survives.
        assert!(!metrics.global_status.shot_advice_details.contains("Aerodrome"));
        // #95: no P/L claim, no yield claim, no "Uniswap position(s)".
        let d = &metrics.global_status.shot_advice_details;
        for banned in ["realized", "0%", "Uniswap position", "generating yield"] {
            assert!(!d.contains(banned), "{banned} in: {d}");
        }
        // Pools were not read in this fixture: an unread pool does not disprove activity,
        // the same rule the "N in range" count uses, so the label matches the details.
        assert_eq!(metrics.global_status.shot_advice_action, "HOLD & ACCRETE (Dual Growth Engine Active)");
        assert!(d.contains("4 ranged (4 in range)"), "{d}");
    }

    #[test]
    fn test_hold_advice_states_no_pnl_and_names_lp_positions() {
        // Issue #95: HOLD is built from the positions. Four ranged positions on live
        // pools plus an Aero full-range pool; one pool then reads zero liquidity.
        let mut raw = four_position_fixture();
        for p in &raw.positions {
            raw.pool_liquidity.insert(p.position_id.clone(), "80173935548453779".to_string());
        }
        let aero = aero_position(false);
        raw.pool_liquidity.insert(aero.position_id.clone(), "342542824101829799".to_string());
        raw.positions.push(aero);
        let m = compute_derived_metrics(&raw, 0.0, &OperatorProfile::default(), StructuralChannels { channels: vec![] }, &[], &LedgerAggregate::default(), &MarketRegime::default());
        assert_eq!(m.global_status.shot_advice_action, "HOLD & ACCRETE (Dual Growth Engine Active)");
        let d = &m.global_status.shot_advice_details;
        assert!(d.starts_with("Total NOCK held is "), "{d}");
        assert!(d.contains("; no floor configured."), "{d}");
        assert!(d.contains("5 LP positions tracked: 4 ranged (4 in range), 1 full range."), "{d}");
        assert!(d.contains("of LP fees uncollected."), "{d}");
        assert!(d.contains("Liquid dry powder: "), "{d}");
        for banned in ["realized", "0%", "Uniswap position", "generating yield", "Maintain posture", "hard floor"] {
            assert!(!d.contains(banned), "{banned} in: {d}");
        }

        // A stale pool is called stale, and the label stops claiming growth once no
        // ranged position is confirmed in range on a live pool.
        raw.pool_liquidity.insert("uniswap-v4:base:3163431".to_string(), "0".to_string());
        let m = compute_derived_metrics(&raw, 0.0, &OperatorProfile::default(), StructuralChannels { channels: vec![] }, &[], &LedgerAggregate::default(), &MarketRegime::default());
        assert!(m.global_status.shot_advice_details.contains("4 ranged (3 in range, 1 on a stale pool), 1 full range"));
        for p in raw.positions.iter().filter(|p| !is_full_range(p.tick_lower, p.tick_upper)).map(|p| p.position_id.clone()).collect::<Vec<_>>() {
            raw.pool_liquidity.insert(p, "0".to_string());
        }
        let m = compute_derived_metrics(&raw, 0.0, &OperatorProfile::default(), StructuralChannels { channels: vec![] }, &[], &LedgerAggregate::default(), &MarketRegime::default());
        assert_eq!(m.global_status.shot_advice_action, "HOLD & ACCRETE (No Ranged NOCK LP Confirmed In Range)");
    }

    #[test]
    fn test_hold_advice_replays_the_2026_11_14_empty_book() {
        // 05:36Z on 2026-11-14: four tracked positions, all emptied, and HOLD said
        // "4 Uniswap position(s) generating yield ... 0% realized loss". With the plan
        // complete (no dip arm), HOLD now states the empty book.
        let raw = liquid_wallet_fixture();
        let fills = PlanFills { count: 5, spent_usdc: 500.0, acquired_nock: 40500.0, last_at: Some(t("2026-11-15T12:00:00Z")), first_price: Some(0.0125) };
        let ctx = PlanContext { cfg: Some(plan_cfg()), fills, ledger_unavailable: false, now: t("2026-11-20T12:00:00Z") };
        let m = metrics_with_plan(&raw, &ctx, &MarketRegime::default());
        assert_eq!(m.global_status.shot_advice_action, "HOLD & ACCRETE (NOCK LP Empty)");
        let d = &m.global_status.shot_advice_details;
        assert!(d.starts_with("Total NOCK held is 115748 (+15748 above the 100000 floor). 4 LP positions tracked: 4 ranged (0 in range, 4 empty). $0.00 of LP fees uncollected. Liquid dry powder: $929.22 (wallet USDC $1053.00 less the larger of the $123.78 cash target and the $0.00 virtual tax reserve)."), "{d}");
        for banned in ["realized", "generating yield", "Uniswap position", "Deploy $", "120,000", "CRITICAL"] {
            assert!(!d.contains(banned), "{banned} in: {d}");
        }
    }

    #[test]
    fn test_derived_metrics_four_position_fixture_matches_screenshot() {
        // Full aggregation over the discovery fixture: totals, brackets, series and
        // the deviation buckets must all reflect N positions, not the old tranches.
        let raw = four_position_fixture();
        let profile = OperatorProfile::default();
        let metrics = compute_derived_metrics(&raw, 2500.0, &profile, StructuralChannels { channels: vec![] }, &[], &LedgerAggregate::default(), &MarketRegime::default());

        // NOCK spot from the price-source pool matches the screenshot-era value.
        assert!((raw.spot_price_nock_usd - 0.0208).abs() < 0.0002);

        // Position values (USD): B ≈ 71, C ≈ 18, A ≈ 156 (post-compounding),
        // D ≈ 207 (wide raft, largest holding after the operator's post-mint adds).
        let total_lp_usd = metrics.asset_aggregation_series.active_protocol_series[0].parse::<f64>().unwrap();
        assert!(total_lp_usd > 430.0 && total_lp_usd < 470.0, "lp total {}", total_lp_usd);

        // One bracket per position, all in range.
        assert_eq!(metrics.tick_bracket_hypervisor.brackets.len(), 4);
        assert!(metrics.tick_bracket_hypervisor.brackets.iter().all(|b| b.in_range));

        // Per-position NOCK series entries carry the position ids.
        assert_eq!(metrics.asset_aggregation_series.positions_nock.len(), 4);
        assert!(metrics.asset_aggregation_series.positions_nock.iter().any(|s| s.position_id == "uniswap-v3:base:6336662"));
        assert!(metrics.asset_aggregation_series.positions_nock.iter().any(|s| s.position_id == "uniswap-v4:base:3163450"));

        // Deviation matrix: Treasury / Uniswap LP / Cash only (no Aerodrome bucket).
        let names: Vec<&str> = metrics.portfolio_deviation_matrix.allocations.iter().map(|a| a.bucket_name.as_str()).collect();
        assert!(names.iter().any(|n| n.contains("NOCK LP (NOCK-pair positions)")));
        assert!(!names.iter().any(|n| n.contains("Aerodrome")));

        // v3 unclaimed fees surface in contract_fees with the NPM address.
        let v3_fee = metrics.velocity_gas_sandbox.contracts.iter().find(|c| c.contract_name.contains("Uniswap V3"));
        assert!(v3_fee.is_some());
    }

    #[test]
    fn test_operator_profile_custom_treasury_floor_and_weights() {
        // Legacy config keys (v4_baseline_*, aerodrome_lp, uniswap_v4_lp) still parse
        // but are ignored; the LP target now comes from `uniswap_lp`.
        let yaml = r#"
database_url: "postgresql://localhost/test"
operator_profile:
  treasury_floor_nock: 150000.0
  v4_baseline_liquidity: 65719361234409.0
  v4_baseline_nock: 4004.31
  v4_baseline_usdc: 23.63
  v4_fee_baseline_usdc: 0.05
  v4_fee_baseline_nock: 2.5
  tax_reserve_ratio: 0.33
  target_weights_bps:
    treasury: 7000
    aerodrome_lp: 1500
    uniswap_v4_lp: 1000
    cash: 500
"#;
        let cfg: AppConfig = serde_yaml::from_str(yaml).expect("Failed to deserialize yaml");
        let profile = cfg.profile();
        assert_eq!(profile.treasury_floor_nock, 150000.0);
        assert_eq!(profile.tax_reserve_ratio, 0.33);
        assert_eq!(profile.target_weights_bps.treasury, 7000);
        // aerodrome_lp/uniswap_v4_lp are ignored; an unset uniswap_lp is no target (0).
        assert_eq!(profile.target_weights_bps.uniswap_lp, 0);
        assert_eq!(profile.target_weights_bps.cash, 500);

        let mut raw = RawOnChainData::default();
        raw.wallet_balances = vec![
            WalletBalance {
                wallet_address: "0x123".to_string(),
                token_symbol: "NOCK".to_string(),
                token_address: "0xNOCK".to_string(),
                amount: "120000.0".to_string(),
                amount_usd: "3120.0".to_string(),
                updated_at: "now".to_string(),
            },
        ];
        // 120,000 is above default 100k floor, BUT below custom 150k floor!
        let metrics = compute_derived_metrics(&raw, 3000.0, &profile, StructuralChannels { channels: vec![] }, &[], &LedgerAggregate::default(), &MarketRegime::default());
        assert_eq!(metrics.global_status.nock_floor_target, "150000");
        assert_eq!(metrics.global_status.nock_floor_delta, "-30000");
        assert!(metrics.global_status.shot_advice_action.contains("DEFENSIVE LOCK"));
        assert_eq!(metrics.portfolio_deviation_matrix.allocations[0].target_weight_bps, 7000);
        assert_eq!(metrics.portfolio_deviation_matrix.allocations[1].target_weight_bps, 0);
        assert_eq!(metrics.portfolio_deviation_matrix.allocations[1].status, "no_target");
        assert_eq!(metrics.portfolio_deviation_matrix.allocations[2].target_weight_bps, 500);
    }

    #[test]
    fn test_operator_profile_default_tax_reserve_ratio() {
        // R11: configs without the fields still parse, and the code chooses nothing:
        // no tax ratio, no target weights, no floor, no wallet, no plan.
        let yaml = r#"
database_url: "postgresql://localhost/test"
"#;
        let cfg: AppConfig = serde_yaml::from_str(yaml).expect("Failed to deserialize yaml");
        let p = cfg.profile();
        assert_eq!(p.tax_reserve_ratio, 0.0);
        assert_eq!(p.treasury_floor_nock, 0.0);
        assert_eq!((p.target_weights_bps.treasury, p.target_weights_bps.uniswap_lp, p.target_weights_bps.cash), (0, 0, 0));
        assert!(cfg.tracked_wallets.is_empty());
        assert!(cfg.accumulation_plan.is_none());
        let d = OperatorProfile::default();
        assert_eq!((d.tax_reserve_ratio, d.treasury_floor_nock, d.target_weights_bps.cash), (0.0, 0.0, 0));
    }

    #[test]
    fn test_sovereign_ledger_60_40_split() {
        let raw = RawOnChainData {
            wallet_balances: vec![WalletBalance {
                wallet_address: "0x123".to_string(),
                token_symbol: "NOCK".to_string(),
                token_address: "0xNOCK".to_string(),
                amount: "110000.0".to_string(),
                amount_usd: "2860.0".to_string(),
                updated_at: "now".to_string(),
            }],
            ..Default::default()
        };
        let mut profile = OperatorProfile::default();
        profile.tax_reserve_ratio = 0.40;
        let ledger = LedgerAggregate {
            total_usd: 100.0,
            harvest_count: 2,
            last_at: Some(Utc::now()),
            unpriced_count: 0,
        };
        let metrics = compute_derived_metrics(
            &raw, 0.0, &profile,
            StructuralChannels { channels: vec![] },
            &[], &ledger, &MarketRegime::default(),
        );
        assert_eq!(metrics.sovereign_ledger.cumulative_harvested_usd, "100.00");
        assert_eq!(metrics.sovereign_ledger.tax_reserve_usd, "40.00");
        assert_eq!(metrics.sovereign_ledger.growth_dry_powder_usd, "60.00");
        assert_eq!(metrics.sovereign_ledger.harvest_count, 2);
        assert!(metrics.sovereign_ledger.last_harvest_at.is_some());

        // Custom ratio honored
        profile.tax_reserve_ratio = 0.25;
        let metrics = compute_derived_metrics(
            &raw, 0.0, &profile,
            StructuralChannels { channels: vec![] },
            &[], &ledger, &MarketRegime::default(),
        );
        assert_eq!(metrics.sovereign_ledger.tax_reserve_usd, "25.00");
        assert_eq!(metrics.sovereign_ledger.growth_dry_powder_usd, "75.00");
    }

    #[test]
    fn test_koinly_csv_format() {
        let rows = vec![
            HarvestExportRow {
                platform: "aerodrome".to_string(),
                tx_hash: "0xabc123".to_string(),
                token_symbol: "USDC".to_string(),
                amount_human: "0.119983".to_string(),
                usd_value: Some("0.119983".to_string()),
                confirmed_at: "2026-10-12T18:31:21+00:00".to_string(),
            },
            HarvestExportRow {
                platform: "uniswap-v4".to_string(),
                tx_hash: "0xdef456".to_string(),
                token_symbol: "NOCK".to_string(),
                amount_human: "5.4468217823004916".to_string(),
                usd_value: Some("0.2612".to_string()),
                confirmed_at: "2026-10-12T18:35:00+00:00".to_string(),
            },
        ];
        let csv = format_koinly_csv(&rows);
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines.len(), 3);
        assert!(lines[0].starts_with("Date, Sent Amount, Sent Currency, Received Amount"));
        assert!(lines[0].ends_with("Label, Description, TxHash"));
        assert!(lines[1].contains(", , , 0.119983, USDC, , , 0.119983, USD, income,"));
        assert!(lines[1].contains("\"NockNock fee harvest (aerodrome)\""));
        assert!(lines[1].ends_with("0xabc123"));
        assert!(lines[2].contains("5.4468217823004916, NOCK"));
        assert!(lines[2].contains("income"));
        assert!(lines[2].ends_with("0xdef456"));
    }

    #[test]
    fn test_tax_tie_out_split_equals_export_total() {
        // Koinly tie-out: the CSV rows must carry exactly the ledger amounts that the
        // 60/40 sub-ledger split was computed from — every harvest dollar accounted
        // for, nothing invented.
        let ledger_rows = vec![
            ("0xabc123", "USDC", "0.119983", 119983u128, 6i32),
            ("0xabc123", "NOCK", "5.4468217823004916", 54468217823004916, 16),
        ];
        let total_usd: f64 = 0.119983 + 5.4468217823004916 * 0.048; // NOCK spot example
        let profile_tax_ratio = 0.40f64;
        let tax_reserve = total_usd * profile_tax_ratio;
        let dry_powder = total_usd * (1.0 - profile_tax_ratio);

        // Ledger aggregate must equal the sum of the exported receipt-verified rows
        let exported_total: f64 = ledger_rows
            .iter()
            .map(|(_, sym, human, raw, dec)| {
                let units = format_units(*raw, *dec);
                assert_eq!(&units, human);
                match *sym {
                    "USDC" => units.parse::<f64>().unwrap(),
                    "NOCK" => units.parse::<f64>().unwrap() * 0.048,
                    _ => 0.0,
                }
            })
            .sum();
        assert!((exported_total - total_usd).abs() < 1e-9);
        assert!((tax_reserve + dry_powder - total_usd).abs() < 1e-9);
    }

    #[test]
    fn test_tax_guardrail_blocks_dip_deployment() {
        // Issue #63: only wallet USDC backs the virtual tax reserve. USDC inside LP
        // (B+D in this fixture) does not cover it and is not dry powder.
        let mut raw = four_position_fixture();
        raw.spot_price_nock_usd = 0.015; // dip zone (< 0.020)
        raw.wallet_balances[1].amount = "10.0".to_string();
        let profile = operator_profile_no_floor(); // ratio 0.40
        let ledger = LedgerAggregate { total_usd: 500.0, harvest_count: 3, last_at: None, unpriced_count: 0 };
        let metrics = compute_derived_metrics(
            &raw, 0.0, &profile, StructuralChannels { channels: vec![] }, &[], &ledger, &MarketRegime::default(),
        );
        assert_eq!(metrics.global_status.shot_advice_action, "TAX RESERVE SHORT (No Buy Advised)");
        let d = &metrics.global_status.shot_advice_details;
        assert!(d.contains("the virtual tax reserve ($200.00) exceeds liquid wallet USDC ($10.00)."), "{d}");
        assert!(d.contains("of USDC inside NOCK LP is working capital, not dry powder"), "{d}");
        assert!(d.contains("No buy is advised while wallet USDC is below the reserve."), "{d}");
        assert!(!d.contains("60/40"));
        assert!(!d.contains("SWEEP") && !d.contains("sweep"), "{d}");

        // A $16 reserve still exceeds the $10 wallet: LP USDC no longer covers it.
        let small = LedgerAggregate { total_usd: 40.0, harvest_count: 1, last_at: None, unpriced_count: 0 };
        let metrics = compute_derived_metrics(
            &raw, 0.0, &profile, StructuralChannels { channels: vec![] }, &[], &small, &MarketRegime::default(),
        );
        assert_eq!(metrics.global_status.shot_advice_action, "TAX RESERVE SHORT (No Buy Advised)");
        assert!(metrics.global_status.shot_advice_details.contains("exceeds liquid wallet USDC ($10.00)"));

        // $50 covers the reserve but not the cash target: nothing is dry powder, so the
        // engine holds and says so.
        raw.wallet_balances[1].amount = "50.0".to_string();
        let metrics = compute_derived_metrics(
            &raw, 0.0, &profile, StructuralChannels { channels: vec![] }, &[], &small, &MarketRegime::default(),
        );
        assert!(metrics.global_status.shot_advice_action.starts_with("HOLD & ACCRETE"));
        let total: f64 = metrics.global_status.total_portfolio_usd.parse().unwrap();
        assert!(
            metrics.global_status.shot_advice_details.contains(&format!(
                "Liquid dry powder: none (wallet USDC $50.00 less the larger of the ${:.2} cash target and the $16.00 virtual tax reserve).",
                total * 0.05
            )),
            "{}", metrics.global_status.shot_advice_details
        );

        // Liquid USDC well above the cash target: DIP ZONE states the dry powder and
        // sizes nothing (no lump-sum "Deploy $" without a plan).
        raw.wallet_balances[1].amount = "5000.0".to_string();
        let metrics = compute_derived_metrics(
            &raw, 0.0, &profile, StructuralChannels { channels: vec![] }, &[], &small, &MarketRegime::default(),
        );
        assert_eq!(metrics.global_status.shot_advice_action, "DIP ZONE (No Buy Plan Configured)");
        let total: f64 = metrics.global_status.total_portfolio_usd.parse().unwrap();
        let d = &metrics.global_status.shot_advice_details;
        assert!(
            d.contains(&format!(
                "Liquid dry powder: ${:.2} (wallet USDC $5000.00 less the larger of the ${:.2} cash target and the $16.00 virtual tax reserve).",
                5000.0 - total * 0.05,
                total * 0.05
            )),
            "{d}"
        );
        for banned in ["Deploy $", "120,000", "at discount", "CRITICAL"] {
            assert!(!d.contains(banned), "{banned} in: {d}");
        }
    }

    // ---------------- Accumulation plan (issue #79) ----------------

    fn plan_cfg() -> AccumulationPlanConfig {
        AccumulationPlanConfig {
            id: "test-plan".to_string(),
            budget_usdc: 500.0,
            tranches: 5,
            interval_hours: 24.0,
            starts_at: None,
            venue: Some("Aero".to_string()),
            max_price_usd: None,
            pause_when_overextended: true,
            respect_cash_target: true,
            cash_reserve_usdc: None,
        }
    }

    #[test]
    fn test_plan_never_draws_below_the_protected_cash_reserve() {
        // The operator's 2026-11-14 setup: $1,053 liquid, $950 protected, $100 over 5 buys.
        let mut cfg = plan_cfg();
        cfg.budget_usdc = 100.0;
        cfg.cash_reserve_usdc = Some(950.0);
        let now = t("2026-11-15T12:00:00Z");
        // Nominal $20 fits above the reserve ($103 of headroom): full tranche, no limit named.
        assert_eq!(
            plan_directive(&cfg, &PlanFills::default(), &roomy_guards(), false, now),
            PlanDirective::TrancheDue { usdc: 20.0, limited_by: None }
        );
        // Headroom shrinks to $8 (e.g. USDC moved into LP): the tranche shrinks and says why.
        let mut guards = roomy_guards();
        guards.wallet_usdc = 958.0;
        match plan_directive(&cfg, &PlanFills::default(), &guards, false, now) {
            PlanDirective::TrancheDue { usdc, limited_by } => {
                assert!((usdc - 8.0).abs() < 1e-9);
                assert!(limited_by.unwrap().contains("protected cash reserve"));
            }
            other => panic!("expected a shrunk tranche, got {other:?}"),
        }
        // At or below the reserve nothing is spendable, however cheap NOCK is.
        guards.wallet_usdc = 950.0;
        guards.spot_price = 0.0001;
        let d = plan_directive(&cfg, &PlanFills::default(), &guards, false, now);
        assert!(matches!(d, PlanDirective::Blocked { ref reason } if reason.contains("protected cash reserve")));
        // The reserve binds even when the operator opts out of the 5% cash target.
        cfg.respect_cash_target = false;
        assert!(matches!(
            plan_directive(&cfg, &PlanFills::default(), &guards, false, now),
            PlanDirective::Blocked { .. }
        ));
        // A reserve protects against an oversized budget too: the example plan
        // ($500 / 5) can only ever take the $103 above a $950 reserve.
        let mut big = plan_cfg();
        big.cash_reserve_usdc = Some(950.0);
        match plan_directive(&big, &PlanFills::default(), &roomy_guards(), false, now) {
            PlanDirective::TrancheDue { usdc, limited_by } => {
                assert!((usdc - 100.0).abs() < 1e-9 && limited_by.is_none());
            }
            other => panic!("{other:?}"),
        }
        let mut after_one = roomy_guards();
        after_one.wallet_usdc = 953.0; // one $100 buy later
        match plan_directive(&big, &PlanFills { count: 1, spent_usdc: 100.0, acquired_nock: 8000.0, last_at: Some(t("2026-11-14T00:00:00Z")), first_price: Some(0.0125) }, &after_one, false, now) {
            PlanDirective::TrancheDue { usdc, .. } => assert!((usdc - 3.0).abs() < 1e-9),
            other => panic!("{other:?}"),
        }
        // Negative reserve is a config error, stated explicitly.
        cfg.cash_reserve_usdc = Some(-1.0);
        assert!(matches!(plan_directive(&cfg, &PlanFills::default(), &roomy_guards(), false, now), PlanDirective::Invalid { .. }));
    }

    #[test]
    fn test_advice_states_the_protected_reserve() {
        let raw = liquid_wallet_fixture();
        let mut cfg = plan_cfg();
        cfg.budget_usdc = 100.0;
        cfg.cash_reserve_usdc = Some(950.0);
        let ctx = PlanContext { cfg: Some(cfg), fills: PlanFills::default(), ledger_unavailable: false, now: t("2026-11-15T12:00:00Z") };
        let m = metrics_with_plan(&raw, &ctx, &MarketRegime::default());
        assert_eq!(m.global_status.shot_advice_action, "DCA TRANCHE DUE (1 of 5)");
        assert!(m.global_status.shot_advice_details.contains("Buy $20.00 of NOCK"));
        assert!(m.global_status.shot_advice_details.contains("never spends the stable balance (wallet + NOCK LP USDC) below $950.00"));
        assert_eq!(m.accumulation_plan.unwrap().cash_reserve_usdc.as_deref(), Some("950.00"));
    }

    fn roomy_guards() -> PlanGuards {
        PlanGuards { overextended: false, spot_price: 0.0123, wallet_usdc: 1053.0, lp_usdc: 0.0, tax_reserve_usd: 0.0, cash_target_usd: 124.0, standing_reserve_usdc: None }
    }

    fn t(s: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
    }

    #[test]
    fn test_plan_first_tranche_due_immediately_at_nominal_size() {
        let d = plan_directive(&plan_cfg(), &PlanFills::default(), &roomy_guards(), false, t("2026-11-15T12:00:00Z"));
        assert_eq!(d, PlanDirective::TrancheDue { usdc: 100.0, limited_by: None });
    }

    #[test]
    fn test_plan_pending_before_start() {
        let mut cfg = plan_cfg();
        cfg.starts_at = Some(t("2026-11-20T00:00:00Z"));
        let d = plan_directive(&cfg, &PlanFills::default(), &roomy_guards(), false, t("2026-11-15T12:00:00Z"));
        assert_eq!(d, PlanDirective::Pending { starts_at: t("2026-11-20T00:00:00Z") });
    }

    #[test]
    fn test_plan_waits_out_the_interval_then_is_due() {
        let fills = PlanFills { count: 1, spent_usdc: 100.0, acquired_nock: 8000.0, last_at: Some(t("2026-11-15T12:00:00Z")), first_price: Some(0.0125) };
        let early = plan_directive(&plan_cfg(), &fills, &roomy_guards(), false, t("2026-11-16T11:59:00Z"));
        assert_eq!(early, PlanDirective::Waiting { next_due_at: t("2026-11-16T12:00:00Z") });
        let on_time = plan_directive(&plan_cfg(), &fills, &roomy_guards(), false, t("2026-11-16T12:00:00Z"));
        assert_eq!(on_time, PlanDirective::TrancheDue { usdc: 100.0, limited_by: None });
    }

    #[test]
    fn test_plan_missed_windows_do_not_stack() {
        // Operator away for ten intervals: exactly one nominal tranche is due, never ten.
        let fills = PlanFills { count: 1, spent_usdc: 100.0, acquired_nock: 8000.0, last_at: Some(t("2026-11-15T12:00:00Z")), first_price: Some(0.0125) };
        let d = plan_directive(&plan_cfg(), &fills, &roomy_guards(), false, t("2026-11-25T12:00:00Z"));
        assert_eq!(d, PlanDirective::TrancheDue { usdc: 100.0, limited_by: None });
    }

    #[test]
    fn test_plan_pauses_when_overextended_unless_opted_out() {
        let mut guards = roomy_guards();
        guards.overextended = true;
        let d = plan_directive(&plan_cfg(), &PlanFills::default(), &guards, false, t("2026-11-15T12:00:00Z"));
        assert!(matches!(d, PlanDirective::Paused { ref reason } if reason.contains("RSI")));
        let mut cfg = plan_cfg();
        cfg.pause_when_overextended = false;
        let d = plan_directive(&cfg, &PlanFills::default(), &guards, false, t("2026-11-15T12:00:00Z"));
        assert!(matches!(d, PlanDirective::TrancheDue { .. }));
    }

    #[test]
    fn test_plan_pauses_above_price_ceiling() {
        let mut cfg = plan_cfg();
        cfg.max_price_usd = Some(0.0100);
        let d = plan_directive(&cfg, &PlanFills::default(), &roomy_guards(), false, t("2026-11-15T12:00:00Z"));
        assert!(matches!(d, PlanDirective::Paused { ref reason } if reason.contains("ceiling")));
    }

    #[test]
    fn test_plan_tranche_shrinks_to_cash_target_and_names_it() {
        // $160 liquid, $124 cash target -> only $36 spendable, below the $100 nominal.
        let mut guards = roomy_guards();
        guards.wallet_usdc = 160.0;
        match plan_directive(&plan_cfg(), &PlanFills::default(), &guards, false, t("2026-11-15T12:00:00Z")) {
            PlanDirective::TrancheDue { usdc, limited_by } => {
                assert!((usdc - 36.0).abs() < 1e-9);
                assert!(limited_by.unwrap().contains("cash target"));
            }
            other => panic!("expected a shrunk tranche, got {other:?}"),
        }
        // Opting out of the cash target restores the nominal size.
        let mut cfg = plan_cfg();
        cfg.respect_cash_target = false;
        assert_eq!(
            plan_directive(&cfg, &PlanFills::default(), &guards, false, t("2026-11-15T12:00:00Z")),
            PlanDirective::TrancheDue { usdc: 100.0, limited_by: None }
        );
    }

    #[test]
    fn test_plan_blocked_when_nothing_is_spendable() {
        let mut guards = roomy_guards();
        guards.wallet_usdc = 100.0; // below the $124 cash target
        let d = plan_directive(&plan_cfg(), &PlanFills::default(), &guards, false, t("2026-11-15T12:00:00Z"));
        assert!(matches!(d, PlanDirective::Blocked { ref reason } if reason.contains("cash target")));
        // Tax reserve larger than the wallet blocks too, and is named.
        let mut guards = roomy_guards();
        guards.tax_reserve_usd = 2000.0;
        let d = plan_directive(&plan_cfg(), &PlanFills::default(), &guards, false, t("2026-11-15T12:00:00Z"));
        assert!(matches!(d, PlanDirective::Blocked { ref reason } if reason.contains("tax reserve")));
    }

    #[test]
    fn test_plan_last_tranche_is_capped_to_remaining_budget_then_completes() {
        let fills = PlanFills { count: 5, spent_usdc: 470.0, acquired_nock: 38000.0, last_at: Some(t("2026-11-10T00:00:00Z")), first_price: Some(0.0125) };
        match plan_directive(&plan_cfg(), &fills, &roomy_guards(), false, t("2026-11-15T12:00:00Z")) {
            PlanDirective::TrancheDue { usdc, limited_by } => {
                assert!((usdc - 30.0).abs() < 1e-9);
                assert!(limited_by.unwrap().contains("plan budget remains"));
            }
            other => panic!("expected the remainder tranche, got {other:?}"),
        }
        let done = PlanFills { spent_usdc: 499.60, ..fills };
        assert_eq!(plan_directive(&plan_cfg(), &done, &roomy_guards(), false, t("2026-11-15T12:00:00Z")), PlanDirective::Complete);
    }

    #[test]
    fn test_plan_invalid_config_and_unreadable_ledger_are_explicit() {
        let mut cfg = plan_cfg();
        cfg.tranches = 0;
        assert!(matches!(
            plan_directive(&cfg, &PlanFills::default(), &roomy_guards(), false, t("2026-11-15T12:00:00Z")),
            PlanDirective::Invalid { .. }
        ));
        // Zero-mock: an unreadable ledger must never look like "no buys yet".
        let d = plan_directive(&plan_cfg(), &PlanFills::default(), &roomy_guards(), true, t("2026-11-15T12:00:00Z"));
        assert!(matches!(d, PlanDirective::Blocked { ref reason } if reason.contains("ledger")));
    }

    #[test]
    fn test_plan_status_reports_average_cost_and_lump_sum_counterfactual() {
        let fills = PlanFills { count: 2, spent_usdc: 200.0, acquired_nock: 17000.0, last_at: Some(t("2026-11-16T12:00:00Z")), first_price: Some(0.0125) };
        let d = plan_directive(&plan_cfg(), &fills, &roomy_guards(), false, t("2026-11-16T13:00:00Z"));
        let ps = build_plan_status(&plan_cfg(), &fills, &d);
        assert_eq!(ps.status, "waiting");
        assert_eq!(ps.tranche_index, 3);
        assert_eq!(ps.tranches_total, 5);
        assert_eq!(ps.spent_usdc, "200.00");
        assert_eq!(ps.remaining_usdc, "300.00");
        assert_eq!(ps.avg_price_usd.as_deref(), Some("0.011765")); // 200 / 17000
        assert_eq!(ps.lump_sum_nock.as_deref(), Some("40000.0000")); // 500 / 0.0125
        assert_eq!(ps.next_due_at.as_deref(), Some("2026-11-17T12:00:00+00:00"));
        // Before any fill there is no average and no counterfactual (never fabricated).
        let empty = build_plan_status(&plan_cfg(), &PlanFills::default(), &PlanDirective::TrancheDue { usdc: 100.0, limited_by: None });
        assert_eq!(empty.avg_price_usd, None);
        assert_eq!(empty.lump_sum_nock, None);
        assert_eq!(empty.tranche_usdc, "100.00");
    }

    #[test]
    fn test_accumulation_plan_config_parses_with_defaults() {
        let cfg: AppConfig = serde_yaml::from_str(
            "accumulation_plan:\n  id: reentry\n  budget_usdc: 500\n  tranches: 5\n  interval_hours: 24\n",
        )
        .unwrap();
        let plan = cfg.accumulation_plan.expect("plan parsed");
        // No venue is chosen for the operator (issue #132).
        assert_eq!(plan.venue, None);
        assert!(plan.pause_when_overextended && plan.respect_cash_target);
        assert!(plan.starts_at.is_none() && plan.max_price_usd.is_none());
        // Absent section keeps the feature off.
        let none: AppConfig = serde_yaml::from_str("port: 8080\n").unwrap();
        assert!(none.accumulation_plan.is_none());
    }

    fn liquid_wallet_fixture() -> RawOnChainData {
        // The 2026-11-14 wallet: fully liquid, every tracked position emptied.
        let mut raw = four_position_fixture();
        raw.spot_price_nock_usd = 0.01229;
        for p in raw.positions.iter_mut() {
            p.liquidity = 0;
            p.amount0 = 0.0;
            p.amount1 = 0.0;
            p.fee0 = 0.0;
            p.fee1 = 0.0;
        }
        raw.wallet_balances[0].amount = "115747.8001".to_string();
        raw.wallet_balances[1].amount = "1053.0".to_string();
        raw
    }

    fn metrics_with_plan(raw: &RawOnChainData, ctx: &PlanContext, regime: &MarketRegime) -> DerivedMetrics {
        compute_derived_metrics_with_plan(
            raw, 0.0, &operator_profile_2026_11(), StructuralChannels { channels: vec![] }, &[],
            &LedgerAggregate::default(), regime, ctx,
        )
    }

    #[test]
    fn test_advice_plan_replaces_lump_sum_dip_shot() {
        let raw = liquid_wallet_fixture();
        assert_eq!(raw.wallet_balances[0].token_symbol, "NOCK");
        assert_eq!(raw.wallet_balances[1].token_symbol, "USDC");
        // No plan: the dip zone is stated as a fact and nothing is sized (v0.5.1).
        let legacy = metrics_with_plan(&raw, &PlanContext::none(), &MarketRegime::default());
        assert_eq!(legacy.global_status.shot_advice_action, "DIP ZONE (No Buy Plan Configured)");
        assert!(!legacy.global_status.shot_advice_details.contains("Deploy $"));
        assert!(legacy.accumulation_plan.is_none());

        let ctx = PlanContext { cfg: Some(plan_cfg()), fills: PlanFills::default(), ledger_unavailable: false, now: t("2026-11-15T12:00:00Z") };
        let m = metrics_with_plan(&raw, &ctx, &MarketRegime::default());
        assert_eq!(m.global_status.shot_advice_action, "DCA TRANCHE DUE (1 of 5)");
        assert!(m.global_status.shot_advice_details.contains("Buy $100.00 of NOCK (spot"));
        assert!(!m.global_status.shot_advice_details.contains("Deploy $"));
        let ps = m.accumulation_plan.expect("plan status emitted");
        assert_eq!(ps.status, "tranche_due");
        assert_eq!(ps.tranche_usdc, "100.00");
    }

    #[test]
    fn test_advice_lp_rebuild_bias_leads_when_idle_and_rides_along_when_due() {
        // LP is 0% against a 15% target, and pools are readable, so deposits can be suggested.
        // (Without a fundable suggestion the rebuild note never leads; see
        // test_rebuild_does_not_lead_or_nag_when_nothing_is_fundable.)
        let raw = rebuild_fixture();
        // A due tranche stays the primary directive; the rebuild bias is appended.
        let due = PlanContext { cfg: Some(plan_cfg()), fills: PlanFills::default(), ledger_unavailable: false, now: t("2026-11-15T12:00:00Z") };
        let m = metrics_with_plan(&raw, &due, &MarketRegime::default());
        assert!(m.global_status.shot_advice_action.starts_with("DCA TRANCHE DUE"));
        assert!(m.global_status.shot_advice_details.contains("Standing bias: NOCK LP holds $0.00"));
        assert_eq!(m.lp_rebuild.status, "under_target");

        // Between tranches there is nothing to buy, so the rebuild leads.
        let fills = PlanFills { count: 1, spent_usdc: 100.0, acquired_nock: 8000.0, last_at: Some(t("2026-11-15T12:00:00Z")), first_price: Some(0.0125) };
        let waiting = PlanContext { cfg: Some(plan_cfg()), fills, ledger_unavailable: false, now: t("2026-11-15T18:00:00Z") };
        let m = metrics_with_plan(&raw, &waiting, &MarketRegime::default());
        assert_eq!(m.global_status.shot_advice_action, "REBUILD LP TRANCHES (No Net Cash Change)");
        assert!(m.global_status.shot_advice_details.contains("reposition, not a purchase"));
        assert!(m.global_status.shot_advice_details.contains("DCA ON SCHEDULE (2 of 5 next)"));
        assert_eq!(m.accumulation_plan.unwrap().status, "waiting");

        // Safety directives are never displaced, only annotated.
        let hot = MarketRegime { rsi_5m: None, rsi_1d: Some(82.0), overextended: true };
        let m = metrics_with_plan(&raw, &due, &hot);
        assert_eq!(m.global_status.shot_advice_action, "OVEREXTENDED (Daily RSI ≥ 70, No Buy Advised)");
        assert!(m.global_status.shot_advice_details.contains("Standing bias"));
        assert_eq!(m.accumulation_plan.unwrap().status, "paused");
    }

    #[test]
    fn test_advice_completed_plan_does_not_fall_back_to_lump_sum() {
        let raw = liquid_wallet_fixture();
        let fills = PlanFills { count: 5, spent_usdc: 500.0, acquired_nock: 40500.0, last_at: Some(t("2026-11-15T12:00:00Z")), first_price: Some(0.0125) };
        let ctx = PlanContext { cfg: Some(plan_cfg()), fills, ledger_unavailable: false, now: t("2026-11-20T12:00:00Z") };
        let m = metrics_with_plan(&raw, &ctx, &MarketRegime::default());
        assert!(!m.global_status.shot_advice_action.contains("DIP REINVESTMENT SHOT"));
        assert!(!m.global_status.shot_advice_action.starts_with("DCA"));
        let ps = m.accumulation_plan.unwrap();
        assert_eq!(ps.status, "complete");
        assert_eq!(ps.tranche_index, 5);
    }

    #[test]
    fn test_lp_rebuild_status_uses_the_deviation_matrix_tolerance() {
        // 15% target on $1,000: tolerance is 20% of target = 300 bps -> under below 12%.
        assert_eq!(lp_rebuild_status(0.0, 1000.0, 1500).status, "under_target");
        assert_eq!(lp_rebuild_status(119.0, 1000.0, 1500).status, "under_target");
        assert_eq!(lp_rebuild_status(130.0, 1000.0, 1500).status, "on_target");
        assert_eq!(lp_rebuild_status(200.0, 1000.0, 1500).status, "over_target");
        let lp = lp_rebuild_status(0.0, 2475.27, 1500);
        assert_eq!(lp.target_usd, "371.29");
        assert_eq!(lp.gap_usd, "371.29");
        // On target: the bias leaves advice untouched.
        let on = lp_rebuild_status(150.0, 1000.0, 1500);
        let (a, d) = apply_lp_rebuild_bias("HOLD & ACCRETE (x)".to_string(), "details".to_string(), &on);
        assert_eq!((a.as_str(), d.as_str()), ("HOLD & ACCRETE (x)", "details"));
    }

    #[test]
    fn test_plan_anchor_prefers_operator_start_over_first_sight() {
        let registered = t("2026-11-14T05:00:00Z");
        // No starts_at: buys count from the moment the worker first saw the plan, so a
        // week-old swap (the 2026-11-06 hash used in the playtest) is before the anchor.
        assert_eq!(plan_anchor(None, registered), registered);
        assert!(t("2026-11-06T03:23:19Z") < plan_anchor(None, registered));
        // An explicit starts_at wins, including one that reaches back on purpose.
        assert_eq!(plan_anchor(Some(t("2026-11-01T00:00:00Z")), registered), t("2026-11-01T00:00:00Z"));
        assert!(t("2026-11-06T03:23:19Z") >= plan_anchor(Some(t("2026-11-01T00:00:00Z")), registered));
    }

    // ---------------- Pre-0.5.0 fixes (issues #85, #86, #87) ----------------

    #[test]
    fn test_wallet_nock_series_is_token_units_not_dollars() {
        let raw = liquid_wallet_fixture(); // 115,747.8001 NOCK at $0.01229
        let m = metrics_with_plan(&raw, &PlanContext::none(), &MarketRegime::default());
        let ag = &m.asset_aggregation_series;
        assert_eq!(ag.wallet_nock_series, vec!["115747.8001".to_string()]);
        // The USD series is unchanged and is NOT the token count.
        let usd: f64 = ag.safe_haven_series[0].parse().unwrap();
        assert!((usd - 115_747.8001 * 0.01229).abs() < 0.01);
        assert!(usd < 2_000.0);
    }

    #[test]
    fn test_example_config_is_the_base_layer() {
        let dir = std::env::temp_dir().join(format!("nocknock-cfg-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let example = dir.join("config.yaml.example");
        let main = dir.join("config.yaml");
        std::fs::write(
            &example,
            "alchemy_api_keys: [\"YOUR_ALCHEMY_API_KEY\"]\nport: 8080\nsync_window_seconds: 300\npositions:\n  - id: \"p1\"\n    protocol: \"uniswap-v4\"\n    nft_id: 3163450\n    is_price_source: true\noperator_profile:\n  treasury_floor_nock: 100000.0\n",
        )
        .unwrap();
        // A partial operator config: key and port only, exactly the shape that failed
        // ingestion on 2026-11-14 with "No positions configured".
        std::fs::write(&main, "alchemy_api_keys: [\"real-key\"]\nport: 8081\noperator_profile:\n  lp_floor_tolerance_bps: 500\n").unwrap();
        let cfg = load_app_config(example.to_str().unwrap(), Some(main.to_str().unwrap()), false).unwrap();
        assert_eq!(cfg.positions.len(), 1, "positions must come from the example base layer");
        assert_eq!(cfg.positions[0].nft_id, 3163450);
        assert_eq!(cfg.alchemy_api_keys, vec!["real-key".to_string()]);
        assert_eq!(cfg.port, 8081);
        // Nested sections merge key by key.
        assert_eq!(cfg.operator_profile.treasury_floor_nock, 100000.0);
        assert_eq!(cfg.operator_profile.lp_floor_tolerance_bps, 500);
        // With no config.yaml at all the example alone still parses.
        let only = load_app_config(example.to_str().unwrap(), Some(dir.join("absent.yaml").to_str().unwrap()), false).unwrap();
        assert_eq!(only.positions.len(), 1);
        assert_eq!(only.port, 8080);
        let _ = std::fs::remove_file(&example);
        let _ = std::fs::remove_file(&main);
        let _ = std::fs::remove_dir(&dir);
    }

    #[test]
    fn test_health_status_is_not_ok_when_ingestion_is_failing_or_stale() {
        // Healthy: fresh snapshot, last cycle fine.
        assert_eq!(health_status(true, 10, 300, false), "ok");
        assert_eq!(health_status(true, 390, 300, false), "ok"); // inside window + 90s grace
        // The 2026-11-14 incident: old snapshot, every cycle failing, lag 491s.
        assert_eq!(health_status(true, 491, 300, true), "degraded");
        // Failing counts even while the last snapshot is still recent.
        assert_eq!(health_status(true, 5, 300, true), "degraded");
        // Stale without a recorded error (scheduler stuck) is degraded too.
        assert_eq!(health_status(true, 391, 300, false), "degraded");
        // Boot: nothing yet and nothing failed is "starting"; a failed first cycle is not.
        assert_eq!(health_status(false, 0, 300, false), "starting");
        assert_eq!(health_status(false, 0, 300, true), "degraded");
    }

    #[test]
    fn test_nothing_to_collect_only_for_truly_empty_positions() {
        assert!(nothing_to_collect("0", "0.000000", "0.000000"));
        assert!(!nothing_to_collect("0", "0.000001", "0.000000")); // fees left behind
        assert!(!nothing_to_collect("48295502692597", "0.000000", "0.000000")); // funded
        assert!(!nothing_to_collect("not-a-number", "0", "0")); // unknown is not "empty"
    }

    // ---------------- Aerodrome full-range tranche (issue #88) ----------------

    fn aero_cfg(price_source: bool) -> PositionConfig {
        serde_yaml::from_str(&format!(
            "id: aerodrome-vamm-nock-usdc\nprotocol: aerodrome\npool_address: \"0x85f1aa3a70fedd1c52705c15baed143e675cd626\"\nis_price_source: {price_source}\n"
        ))
        .unwrap()
    }

    /// The legacy vAMM as read on 2026-11-14 (block 57409285): 369,321.72 USDC and
    /// 31,839,446 NOCK in reserve, LP supply 342542824101829799; wallet given 1%.
    fn aero_position(price_source: bool) -> RawPosition {
        aerodrome_position_from_reads(
            &aero_cfg(price_source),
            "0xeE114cC97eC5adeB34bA5645e315B497D8487a45",
            "0x85f1aa3a70fedd1c52705c15baed143e675cd626",
            token_info(USDC_BASE).unwrap(),
            token_info(NOCK_BASE).unwrap(),
            369_321_720_000,
            318_394_460_000_000_000_000_000,
            342_542_824_101_829_799,
            3_425_428_241_018_297, // 1% of supply
            440_190,               // 0.44019 USDC of fees
            250_000_000_000_000_000, // 25 NOCK of fees
        )
        .unwrap()
    }

    #[test]
    fn test_aerodrome_position_is_a_share_of_reserves_and_full_range() {
        let p = aero_position(false);
        assert_eq!(p.protocol, "aerodrome");
        assert_eq!(p.position_id, "aerodrome:base:0x85f1aa3a70fedd1c52705c15baed143e675cd626");
        assert_eq!((p.token0.symbol.as_str(), p.token1.symbol.as_str()), ("USDC", "NOCK"));
        // 1% of each reserve.
        assert!((p.amount0 - 3693.2172).abs() < 0.01, "USDC {}", p.amount0);
        assert!((p.amount1 - 318_394.46).abs() < 1.0, "NOCK {}", p.amount1);
        assert!((p.fee0 - 0.44019).abs() < 1e-9 && (p.fee1 - 25.0).abs() < 1e-9);
        assert_eq!(p.fee_tier_bps, 30);
        // Full range: always in range, never "sells out".
        assert!(p.in_range && is_full_range(p.tick_lower, p.tick_upper));
        assert_eq!(nock_sell_out_multiple(&p), None);
        assert_eq!(position_label(&p), "Aero USDC/NOCK 0.30% (full range)");
        // An x*y=k position is 50/50 by value at the pool's own price.
        let spot = 369_321.72 / 31_839_446.0;
        let (usdc_per_usd, nock_per_usd) = amounts_per_usd(&p, spot, 1830.0).unwrap();
        assert!((usdc_per_usd - 0.5).abs() < 1e-6, "USDC leg {usdc_per_usd}");
        assert!((nock_per_usd * spot - 0.5).abs() < 1e-6);
        // Snapshot view flags it so clients do not draw a bracket or print a range.
        let snap = build_position_snapshots(&[p.clone()], spot, 1830.0);
        assert!(snap[0].full_range);
        assert_eq!((snap[0].lower_price.as_str(), snap[0].upper_price.as_str()), ("0", "0"));
        assert!((snap[0].amount_usd.parse::<f64>().unwrap() - 2.0 * 3693.2172).abs() < 0.05);
    }

    #[test]
    fn test_aerodrome_pool_can_be_the_price_source() {
        // The deepest NOCK/USDC market gives the spot: reserve ratio, to within float noise.
        let p = aero_position(true);
        let spot = nock_spot_from_positions(&[p]).expect("spot from reserves");
        assert!((spot - 369_321.72 / 31_839_446.0).abs() < 1e-9, "spot {spot}");
        assert!((spot - 0.0116).abs() < 0.0001);
    }

    #[test]
    fn test_aerodrome_rejects_an_empty_pool_and_handles_no_lp() {
        let empty = aerodrome_position_from_reads(
            &aero_cfg(false), "0xw", "0xpool", token_info(USDC_BASE).unwrap(), token_info(NOCK_BASE).unwrap(),
            0, 0, 0, 0, 0, 0,
        );
        assert!(empty.is_err(), "an empty pool has no price and must not be read as zero");
        // Wallet holds no LP yet: a valid, empty position that still carries the pool price.
        let none = aerodrome_position_from_reads(
            &aero_cfg(true), "0xw", "0xpool", token_info(USDC_BASE).unwrap(), token_info(NOCK_BASE).unwrap(),
            369_321_720_000, 318_394_460_000_000_000_000_000, 342_542_824_101_829_799, 0, 0, 0,
        )
        .unwrap();
        assert_eq!((none.amount0, none.amount1, none.liquidity), (0.0, 0.0, 0));
        assert!(nock_spot_from_positions(&[none]).is_some());
    }

    // ---------------- v0.5.2 surfaces: Aero fees that cannot be claimed ----------------

    #[test]
    fn test_aero_claim_revert_is_not_an_rpc_outage() {
        let (sim, detail) = aero_claim_simulation(&Ok(None));
        assert_eq!(sim, "not_claimable");
        let d = detail.unwrap();
        // The reason only. Since #93 the AERO earned instead is read from the gauge and
        // stated after it on the fee row; the claim payload builds no AERO claim.
        assert_eq!(d, "not LP-claimable (claimFees() reverts)");
        assert!(!d.contains("RPC unavailable"));
        assert_eq!(aero_claim_simulation(&Ok(Some("00".to_string()))), ("success", None));
        let (sim, detail) = aero_claim_simulation(&Err(anyhow::anyhow!("network error")));
        assert_eq!(sim, "unavailable");
        assert!(detail.unwrap().contains("RPC unavailable"));
    }

    fn claim_action(platform: &str, simulation: &str, gas_usd: Option<&str>) -> ClaimFeeAction {
        ClaimFeeAction {
            platform: platform.to_string(),
            label: platform.to_string(),
            to: "0x0".to_string(),
            data: "0x".to_string(),
            value: "0x0".to_string(),
            expected_amount_0: None,
            expected_amount_1: None,
            expected_usd: None,
            gas_estimate: None,
            gas_usd: gas_usd.map(|g| g.to_string()),
            simulation: simulation.to_string(),
            simulation_detail: None,
        }
    }

    #[test]
    fn test_claim_payload_totals_ignore_non_claimable_pools() {
        // The 2026-11-14 payload: v4 $0.0014, v3 $0.0027, and a reverting Aero claim
        // that used to null both totals.
        let actions = vec![
            claim_action("uniswap-v4", "success", Some("0.0014")),
            claim_action("uniswap-v3", "success", Some("0.0027")),
            claim_action("aerodrome", "not_claimable", None),
        ];
        let (gas, net) = claim_payload_totals(&actions, "0.30", true);
        assert_eq!(gas.as_deref(), Some("0.0041"));
        assert_eq!(net.as_deref(), Some("0.2959"));
        // An executable action whose estimate failed still makes the total unknown.
        let failed = vec![
            claim_action("uniswap-v4", "success", Some("0.0014")),
            claim_action("uniswap-v3", "success", None),
        ];
        assert_eq!(claim_payload_totals(&failed, "0.30", true), (None, None));
        // No live gas price or ETH price: unknown, never zero.
        assert_eq!(claim_payload_totals(&actions, "0.30", false), (None, None));
        // Nothing executable: no total to report.
        let only_aero = vec![claim_action("aerodrome", "not_claimable", None)];
        assert_eq!(claim_payload_totals(&only_aero, "0.00", true), (None, None));
    }

    #[test]
    fn test_reverting_aero_pool_is_not_shown_as_zero_earned() {
        let mut raw = four_position_fixture();
        raw.live_gas_cost_usd = 0.0037;
        let mut aero = aero_position(false);
        aero.fee0 = 0.0;
        aero.fee1 = 0.0;
        raw.fees_not_lp_claimable = vec![aero.position_id.clone()];
        raw.positions.push(aero.clone());
        let m = compute_derived_metrics(&raw, 0.0, &OperatorProfile::default(), StructuralChannels { channels: vec![] }, &[], &LedgerAggregate::default(), &MarketRegime::default());
        let contracts = &m.velocity_gas_sandbox.contracts;
        let row = contracts.iter().find(|c| c.contract_name.starts_with("Aero")).expect("aero row");
        assert_eq!(row.recommended_action, "not_claimable");
        // The Uniswap rows keep the gas economics.
        assert!(contracts.iter().filter(|c| !c.contract_name.starts_with("Aero")).all(|c| c.recommended_action != "not_claimable"));

        // Contract fees (the MCP claim simulation) say so too, with no gas counted.
        let snaps = build_position_snapshots(&[aero], 0.0116, 1830.0);
        let fees = build_contract_fees(&snaps, 0.0037, &raw.fees_not_lp_claimable);
        assert_eq!(fees[0].amount, AERO_NOT_LP_CLAIMABLE);
        assert!(!fees[0].claimable);
        let text = format_claim_simulation_text(&json!({ "contract_fees": fees }));
        assert!(text.contains("USDC+NOCK (Aerodrome): not LP-claimable (claimFees() reverts)\n"), "{text}");
        // Nothing is claimable, so there is no claim to cost: said as such since v0.6.0
        // (#91), no longer as "live gas estimate unavailable" (gas was read: 0.0037).
        assert!(text.contains("Estimated Gas: n/a (nothing LP-claimable)"), "{text}");
        // Without the flag the same position reads as ordinary fees.
        let plain = build_contract_fees(&snaps, 0.0037, &[]);
        assert_ne!(plain[0].amount, AERO_NOT_LP_CLAIMABLE);
    }

    // ---------------- v0.5.2 surfaces: snapshot by id, first-run errors, MCP ----------------

    #[test]
    fn test_envelope_from_stored_round_trips_the_stored_payload() {
        // store_snapshot writes only the SnapshotData payload into `data`.
        let data = sample_snapshot_data();
        let stored = serde_json::to_value(&data).unwrap();
        let id = Uuid::new_v4();
        let at = DateTime::parse_from_rfc3339("2026-11-14T06:00:00Z").unwrap().with_timezone(&Utc);
        let env = envelope_from_stored(id, at, "ab".repeat(32), "2.2.0".to_string(), stored).expect("stored rows parse");
        assert_eq!(env.meta.snapshot_id, id);
        assert_eq!(env.meta.generated_at, at);
        assert_eq!(env.meta.version, "2.2.0");
        assert_eq!(
            serde_json::to_value(&env.data).unwrap(),
            serde_json::to_value(&data).unwrap()
        );
        // The old handler's assumption (a whole envelope in `data`) is not what is stored.
        let wrong = json!({ "meta": {}, "data": serde_json::to_value(&data).unwrap() });
        assert!(envelope_from_stored(id, at, String::new(), String::new(), wrong).is_err());
    }

    #[test]
    fn test_no_snapshot_body_carries_the_last_ingest_error() {
        let cause = "No NOCK/USD spot: exactly one configured position must have is_price_source: true";
        let body = no_snapshot_body(Some(cause));
        assert_eq!(body["error"], "NO_SNAPSHOT");
        assert_eq!(body["detail"], cause);
        assert!(no_snapshot_body(None).get("detail").is_none());
        assert!(no_snapshot_body(Some("  ")).get("detail").is_none());

        let mcp = no_snapshot_mcp_text(Some(cause));
        assert!(mcp.contains("No snapshot available yet") && mcp.contains(&format!("Last ingestion error: {cause}")), "{mcp}");
        assert!(!no_snapshot_mcp_text(None).contains("error"));
    }

    #[test]
    fn test_mcp_text_shows_snapshot_time_and_health_not_live_api() {
        let at = DateTime::parse_from_rfc3339("2026-11-14T06:00:00Z").unwrap().with_timezone(&Utc);
        let now = at + chrono::Duration::minutes(12);
        let line = snapshot_source_line(at, now, "degraded");
        assert_eq!(line, "Snapshot generated 2026-11-14T06:00:00Z (12 min ago) · worker health: degraded");
        let data = serde_json::to_value(sample_snapshot_data()).unwrap();
        for text in [format_portfolio_state_text(&data), format_shot_advice_text(&data)] {
            assert!(!text.contains("Live API") && !text.contains("Tier 1 Connected"), "{text}");
        }
        // The harvest rule names the precedence the engine applies.
        let advice = format_shot_advice_text(&data);
        assert!(advice.contains("only when no DCA tranche is due"), "{advice}");
        assert!(advice.contains("fixed $5.00 threshold"));
    }

    #[test]
    fn test_aerodrome_joins_the_rebuild_split_and_counts_toward_lp() {
        let mut raw = rebuild_fixture();
        let mut aero = aero_position(false);
        aero.amount0 = 0.0;
        aero.amount1 = 0.0;
        aero.liquidity = 0;
        raw.pool_liquidity.insert(aero.position_id.clone(), "342542824101829799".to_string());
        raw.positions.push(aero);
        let (s, x) = lp_rebuild_suggestions(&raw, 400.0, roomy_funds());
        // Two eligible positions: the dead 0.75% pool stays out, and the two ranges with
        // an ETH/WETH leg are gas-excluded (issue #94). Half each.
        assert_eq!(s.len(), 2);
        assert_eq!(x.iter().filter(|e| e.reason.contains("gas")).count(), 2);
        let a = s.iter().find(|x| x.protocol == "aerodrome").expect("aerodrome suggested");
        assert_eq!(a.target_usd, "200.00");
        assert_eq!(a.nock_sold_out_at_multiple, None);
        // The link is the pool's deposit page in the Aero app: never aerodrome.finance
        // (retired) and never a Uniswap link.
        assert_eq!(a.url, "https://app.aero.xyz/deposit/8453-0x85f1aa3a70fedd1c52705c15baed143e675cd626");
        assert!(!a.url.contains("aerodrome.finance") && !a.url.contains("uniswap"));
        // A config override wins.
        raw.rebuild_rules.insert(
            "aerodrome:base:0x85f1aa3a70fedd1c52705c15baed143e675cd626".to_string(),
            RebuildRule { weight_bps: None, when_out_of_range: false, deposit_url: Some("https://app.aero.xyz/portfolio".to_string()) },
        );
        let (o, _) = lp_rebuild_suggestions(&raw, 400.0, roomy_funds());
        assert_eq!(o.iter().find(|x| x.protocol == "aerodrome").unwrap().url, "https://app.aero.xyz/portfolio");
        raw.rebuild_rules.clear();
        // About half the deposit is USDC and half NOCK. The split is exactly 50/50 at
        // the pool's own price; this fixture's engine spot ($0.01229) sits 6% above the
        // pool's ($0.0116), so the NOCK leg is valued slightly higher here.
        let usdc_leg: f64 = a.amount0.parse().unwrap();
        assert!((90.0..=110.0).contains(&usdc_leg), "USDC leg {usdc_leg}");
        // A config weight makes it the core holding.
        raw.rebuild_rules.insert(
            "aerodrome:base:0x85f1aa3a70fedd1c52705c15baed143e675cd626".to_string(),
            RebuildRule { weight_bps: Some(7000), when_out_of_range: false, deposit_url: None },
        );
        let (s, _) = lp_rebuild_suggestions(&raw, 400.0, roomy_funds());
        assert_eq!(s.iter().find(|x| x.protocol == "aerodrome").unwrap().target_usd, "280.00");
        assert_eq!(sg(&s, 3163450).target_usd, "120.00");

        // Funded: its value and NOCK count in LP totals; its bracket is flagged full range.
        let mut funded = rebuild_fixture();
        funded.positions.push(aero_position(false));
        let m = metrics_with_plan(&funded, &PlanContext::none(), &MarketRegime::default());
        let lp_usd: f64 = m.asset_aggregation_series.active_protocol_series[0].parse().unwrap();
        assert!(lp_usd > 3693.0, "LP total must include the Aerodrome USDC leg, got {lp_usd}");
        let b = m.tick_bracket_hypervisor.brackets.last().unwrap();
        assert!(b.full_range && b.distance_bps.is_none());
        assert_eq!(m.asset_aggregation_series.positions_nock.last().unwrap().position_id,
            "aerodrome:base:0x85f1aa3a70fedd1c52705c15baed143e675cd626");
    }

    #[test]
    fn test_rebuild_fills_only_the_gap_when_other_positions_are_above_their_share() {
        // LP holds $358 ($250 in the USDC/NOCK range, $108 in the WETH/NOCK range),
        // Aerodrome is empty, and the LP target is $486. The WETH/NOCK range is
        // gas-excluded (issue #94), so the two eligible positions share $243 each: the
        // USDC/NOCK range is over its share and is left alone, and the Aerodrome
        // suggestion is the remaining gap ($128), not its full $243 share.
        let mut raw = rebuild_fixture();
        let spot = raw.spot_price_nock_usd;
        for p in raw.positions.iter_mut() {
            match p.nft_id {
                3163450 => { p.amount0 = 71.0; p.amount1 = (250.0 - 71.0) / spot; }
                6336662 => { p.amount0 = 0.0; p.amount1 = 108.0 / spot; }
                _ => {}
            }
        }
        let mut aero = aero_position(false);
        aero.amount0 = 0.0;
        aero.amount1 = 0.0;
        aero.liquidity = 0;
        raw.pool_liquidity.insert(aero.position_id.clone(), "342542824101829799".to_string());
        raw.positions.push(aero);
        let (s, _) = lp_rebuild_suggestions(&raw, 486.0, roomy_funds());
        assert_eq!(s.len(), 1, "only positions below their share are suggested: {s:?}");
        let total: f64 = s.iter().map(|x| x.add_usd.parse::<f64>().unwrap()).sum();
        assert!((total - 128.0).abs() < 0.05, "suggestions must sum to the $128 gap, got {total}");
        // Neither funded Uniswap range is asked to change.
        assert!(s.iter().all(|x| x.nft_id != 3163450 && x.nft_id != 6336662));
        let a = s.iter().find(|x| x.protocol == "aerodrome").unwrap();
        assert!(a.add_usd.parse::<f64>().unwrap() > 60.0);
        // The stated target is what is being filled: nothing is reported as held back.
        assert_eq!(a.target_usd, a.add_usd);
        assert_eq!(a.limited_by, None);
    }

    #[test]
    fn test_eth_call_revert_is_an_answer_not_a_failure() {
        // Returned data.
        assert_eq!(
            classify_eth_call_response(&json!({ "jsonrpc": "2.0", "id": 1, "result": "0x00ff" })).unwrap(),
            Some("00ff".to_string())
        );
        // The new Aero pool's claimFees(): Alchemy answers code 3, "execution reverted".
        // This failed the whole ingestion cycle on 2026-11-14 once LP was held there.
        let reverted = json!({ "jsonrpc": "2.0", "id": 1, "error": { "code": 3, "message": "execution reverted" } });
        assert_eq!(classify_eth_call_response(&reverted).unwrap(), None);
        // A node-side problem is NOT a revert and must still fail the read.
        let limited = json!({ "jsonrpc": "2.0", "id": 1, "error": { "code": 429, "message": "rate limit exceeded" } });
        assert!(classify_eth_call_response(&limited).is_err());
        assert!(classify_eth_call_response(&json!({ "jsonrpc": "2.0", "id": 1 })).is_err());
    }

    #[test]
    fn test_rebuild_does_not_lead_or_nag_when_nothing_is_fundable() {
        // LP is under target but only dust could be added (the 2026-11-14 state after
        // the operator's deposits used up the floor tolerance: 89 NOCK of headroom).
        let raw = rebuild_fixture();
        let dust = RebuildFunds { nock: 89.0, floor_tolerance_nock: 10_000.0, floor_set: true, usdc: 10_000.0 };
        let (s, x) = lp_rebuild_suggestions(&raw, 300.0, dust);
        assert!(s.is_empty(), "a ~$2 top-up is not worth suggesting: {s:?}");
        assert!(x.iter().any(|e| e.reason.contains("cannot be funded")));
        // With no step to take, the rebuild note annotates the directive instead of replacing it.
        let lp = LpRebuild { status: "under_target".to_string(), target_usd: "468.00".to_string(), actual_usd: "423.00".to_string(), gap_usd: "45.00".to_string(), target_weight_bps: 2000, ..Default::default() };
        let (action, details) = apply_lp_rebuild_bias("DCA ON SCHEDULE (2 of 5 next)".to_string(), "Nothing to buy yet.".to_string(), &lp);
        assert_eq!(action, "DCA ON SCHEDULE (2 of 5 next)");
        assert!(details.contains("No tracked position is eligible"));
    }

    // ---------------- LP rebuild suggestions (issue #83) ----------------

    /// Liquid wallet with every tracked pool live except the v4 0.75% pool (#3163431),
    /// which is dead exactly as on 2026-11-14. ETH balance read; no WETH.
    pub(crate) fn rebuild_fixture() -> RawOnChainData {
        let mut raw = liquid_wallet_fixture();
        for p in &raw.positions {
            let live = if p.nft_id == 3163431 { "0" } else { "80173935548453779" };
            raw.pool_liquidity.insert(p.position_id.clone(), live.to_string());
        }
        raw.wallet_eth = Some(0.114226);
        raw.wallet_weth = Some(0.0);
        raw
    }

    fn roomy_funds() -> RebuildFunds {
        RebuildFunds { nock: 1_000_000.0, floor_tolerance_nock: 0.0, floor_set: true, usdc: 10_000.0 }
    }

    #[test]
    fn test_floor_tolerance_lets_lp_hold_a_bounded_overage() {
        // Operator decision: a little more NOCK than sits above the floor may go into
        // LP, because ranges sell gradually. The overage is bounded and reported.
        let raw = rebuild_fixture(); // wallet 115,747.8 NOCK; floor 100,000
        let strict = metrics_with_plan(&raw, &PlanContext::none(), &MarketRegime::default());
        let nock_in = |m: &DerivedMetrics| -> f64 {
            m.lp_rebuild.suggestions.iter().map(|sg| {
                (if sg.token0_symbol == "NOCK" { sg.amount0.parse::<f64>().unwrap() } else { 0.0 })
                    + (if sg.token1_symbol == "NOCK" { sg.amount1.parse::<f64>().unwrap() } else { 0.0 })
            }).sum()
        };
        assert!(nock_in(&strict) <= 15_747.8 + 3.0);
        assert_eq!(strict.lp_rebuild.floor_tolerance_nock.as_deref(), Some("0"));

        let mut profile = operator_profile_2026_11();
        profile.lp_floor_tolerance_bps = 500; // 5% of the floor = 5,000 NOCK
        let loose = compute_derived_metrics_with_plan(
            &raw, 0.0, &profile, StructuralChannels { channels: vec![] }, &[],
            &LedgerAggregate::default(), &MarketRegime::default(), &PlanContext::none(),
        );
        assert_eq!(loose.lp_rebuild.floor_tolerance_nock.as_deref(), Some("5000"));
        // More NOCK may go in than strictly sits above the floor, but never beyond the tolerance.
        assert!(nock_in(&loose) >= nock_in(&strict));
        assert!(nock_in(&loose) <= 15_747.8 + 5_000.0 + 3.0);
        // The worst case (every range sells out) is stated and stays within the tolerance.
        let worst: f64 = loose.lp_rebuild.worst_case_nock.as_deref().unwrap().parse().unwrap();
        assert!((worst - (115_747.8001 - nock_in(&loose))).abs() < 2.0);
        assert!(worst >= 100_000.0 - 5_000.0 - 3.0, "worst case {worst} must stay within 5% of the floor");
        assert!(loose.global_status.shot_advice_details.contains("If every range sold out"));
    }

    #[test]
    fn test_nock_sell_out_multiple_from_ticks() {
        let raw = rebuild_fixture();
        // The 2026-11-14 live ticks: the wide USDC/NOCK range needs a ~3.6x rise to sell
        // all its NOCK; the narrow WETH/NOCK ladder only ~1.44x.
        let mut wide = raw.positions.iter().find(|p| p.nft_id == 3163450).unwrap().clone();
        wide.tick = 274970;
        assert!((nock_sell_out_multiple(&wide).unwrap() - 3.586).abs() < 0.01);
        let mut ladder = raw.positions.iter().find(|p| p.nft_id == 6336662).unwrap().clone();
        ladder.tick = 74039;
        assert!((nock_sell_out_multiple(&ladder).unwrap() - 1.439).abs() < 0.01);
        // Past the edge the range holds no NOCK, so there is nothing left to sell.
        ladder.tick = ladder.tick_lower;
        assert_eq!(nock_sell_out_multiple(&ladder), None);
    }

    fn sg<'a>(v: &'a [LpSuggestion], nft: u64) -> &'a LpSuggestion {
        v.iter().find(|s| s.nft_id == nft).unwrap_or_else(|| panic!("no suggestion for #{nft}"))
    }

    #[test]
    fn test_amounts_per_usd_is_worth_one_dollar() {
        let raw = rebuild_fixture();
        for p in &raw.positions {
            let (a0, a1) = amounts_per_usd(p, raw.spot_price_nock_usd, raw.live_eth_price_usd).expect("priced");
            let value = a0 * leg_usd_price(&p.token0, raw.spot_price_nock_usd, raw.live_eth_price_usd)
                + a1 * leg_usd_price(&p.token1, raw.spot_price_nock_usd, raw.live_eth_price_usd);
            assert!((value - 1.0).abs() < 1e-9, "{}: $1 of range must be worth $1, got {value}", p.position_id);
            assert!(a0 >= 0.0 && a1 >= 0.0);
        }
    }

    #[test]
    fn test_rebuild_skips_dead_pools_and_splits_equally() {
        let raw = rebuild_fixture();
        let (s, x) = lp_rebuild_suggestions(&raw, 300.0, roomy_funds());
        // Dead pool is excluded even though its frozen tick still reads "in range".
        let dead = x.iter().find(|e| e.nft_id == 3163431).expect("dead pool excluded");
        assert!(dead.reason.contains("no in-range liquidity"));
        assert!(raw.positions.iter().find(|p| p.nft_id == 3163431).unwrap().in_range);
        // Of the three live, in-range positions two need ETH/WETH, which is gas (issue
        // #94): the USDC/NOCK range takes the whole target.
        assert_eq!(s.len(), 1);
        let one = sg(&s, 3163450);
        assert_eq!(one.target_usd, "300.00");
        assert_eq!(one.add_usd, "300.00");
        assert_eq!(one.limited_by, None);
        for nft in [3163432u64, 6336662] {
            assert!(x.iter().any(|e| e.nft_id == nft && e.reason.contains("gas, not investable capital")));
        }
        assert_eq!(sg(&s, 3163450).url, "https://app.uniswap.org/positions/v4/base/3163450");
        // The USDC/NOCK range spans spot, so its deposit takes both tokens.
        let wide = sg(&s, 3163450);
        assert_eq!((wide.token0_symbol.as_str(), wide.token1_symbol.as_str()), ("USDC", "NOCK"));
        assert!(wide.amount0.parse::<f64>().unwrap() > 0.0 && wide.amount1.parse::<f64>().unwrap() > 0.0);
    }

    #[test]
    fn test_rebuild_does_not_fill_out_of_range_positions_by_default() {
        // Operator rule: "we should not generally aim to fill positions that are out of range".
        let mut raw = rebuild_fixture();
        raw.positions.iter_mut().find(|p| p.nft_id == 3163432).unwrap().in_range = false;
        let (s, x) = lp_rebuild_suggestions(&raw, 300.0, roomy_funds());
        assert!(s.iter().all(|one| one.nft_id != 3163432));
        assert!(x.iter().any(|e| e.nft_id == 3163432 && e.reason.contains("out of range")));
        // The USDC/NOCK range takes the whole target (the WETH/NOCK range is gas-excluded).
        assert_eq!(s.len(), 1);
        assert_eq!(sg(&s, 3163450).target_usd, "300.00");
        // "Generally": a per-position exception brings an out-of-range USDC/NOCK range back.
        let mut raw = rebuild_fixture();
        raw.positions.iter_mut().find(|p| p.nft_id == 3163450).unwrap().in_range = false;
        let (s, x) = lp_rebuild_suggestions(&raw, 300.0, roomy_funds());
        assert!(s.is_empty());
        assert!(x.iter().any(|e| e.nft_id == 3163450 && e.reason.contains("out of range")));
        raw.rebuild_rules.insert(
            "uniswap-v4:base:3163450".to_string(),
            RebuildRule { weight_bps: None, when_out_of_range: true, deposit_url: None },
        );
        let (s, _) = lp_rebuild_suggestions(&raw, 300.0, roomy_funds());
        assert_eq!(s.len(), 1);
        // The exception never overrides the gas rule: an ETH-leg range stays out.
        let mut raw = rebuild_fixture();
        raw.positions.iter_mut().find(|p| p.nft_id == 3163432).unwrap().in_range = false;
        raw.rebuild_rules.insert(
            "uniswap-v4:base:3163432".to_string(),
            RebuildRule { weight_bps: None, when_out_of_range: true, deposit_url: None },
        );
        let (s, x) = lp_rebuild_suggestions(&raw, 300.0, roomy_funds());
        assert!(s.iter().all(|one| one.nft_id != 3163432));
        assert!(x.iter().any(|e| e.nft_id == 3163432 && e.reason.contains("gas")));
    }

    #[test]
    fn test_rebuild_weights_and_config_exclusion() {
        let mut raw = rebuild_fixture();
        raw.rebuild_rules.insert("uniswap-v4:base:3163450".to_string(), RebuildRule { weight_bps: Some(7000), when_out_of_range: false, deposit_url: None });
        raw.rebuild_rules.insert("uniswap-v4:base:3163432".to_string(), RebuildRule { weight_bps: Some(0), when_out_of_range: false, deposit_url: None });
        let (s, x) = lp_rebuild_suggestions(&raw, 1000.0, roomy_funds());
        // The only unweighted position (#6336662, WETH/NOCK) is gas-excluded, so its share
        // is redistributed: the one eligible position's 7000 bps scales to the whole.
        assert_eq!(sg(&s, 3163450).target_usd, "1000.00");
        assert!(s.iter().all(|one| one.nft_id != 6336662));
        assert!(x.iter().any(|e| e.nft_id == 6336662 && e.reason.contains("gas")));
        assert!(x.iter().any(|e| e.nft_id == 3163432 && e.reason.contains("rebuild_weight_bps: 0")));
        // With an unweighted eligible position, explicit weights are kept as set and the
        // unweighted one takes what is left.
        let mut aero = aero_position(false);
        aero.amount0 = 0.0;
        aero.amount1 = 0.0;
        aero.liquidity = 0;
        raw.pool_liquidity.insert(aero.position_id.clone(), "342542824101829799".to_string());
        raw.positions.push(aero);
        let (s, _) = lp_rebuild_suggestions(&raw, 1000.0, roomy_funds());
        assert_eq!(sg(&s, 3163450).target_usd, "700.00");
        assert_eq!(s.iter().find(|x| x.protocol == "aerodrome").unwrap().target_usd, "300.00");
    }

    #[test]
    fn test_rebuild_never_places_floor_nock_in_lp_and_scales_fairly() {
        // Two eligible NOCK-bearing positions: the USDC/NOCK range and an empty Aero pool
        // (the ETH-leg ranges are gas-excluded).
        let with_aero = || {
            let mut raw = rebuild_fixture();
            let mut aero = aero_position(false);
            aero.amount0 = 0.0;
            aero.amount1 = 0.0;
            aero.liquidity = 0;
            raw.pool_liquidity.insert(aero.position_id.clone(), "342542824101829799".to_string());
            raw.positions.push(aero);
            raw
        };
        let raw = with_aero();
        // Unconstrained NOCK demand for the two deposits.
        let (free, _) = lp_rebuild_suggestions(&raw, 900.0, roomy_funds());
        let nock_of = |v: &[LpSuggestion]| -> f64 {
            v.iter()
                .map(|one| {
                    (if one.token0_symbol == "NOCK" { one.amount0.parse::<f64>().unwrap() } else { 0.0 })
                        + (if one.token1_symbol == "NOCK" { one.amount1.parse::<f64>().unwrap() } else { 0.0 })
                })
                .sum()
        };
        let demand = nock_of(&free);
        assert!(demand > 1000.0);
        // Only half of that sits above the floor.
        let tight = RebuildFunds { nock: demand / 2.0, floor_tolerance_nock: 0.0, floor_set: true, usdc: 10_000.0 };
        let (s, _) = lp_rebuild_suggestions(&raw, 900.0, tight);
        assert!(nock_of(&s) <= demand / 2.0 + 3.0, "LP NOCK {} must stay within the {} above the floor", nock_of(&s), demand / 2.0);
        // Every position that needs NOCK is scaled by the same factor and says why.
        assert_eq!(s.len(), 2);
        for one in &s {
            assert_eq!(one.add_usd, "225.00", "{} should be halved from $450", one.label);
            assert!(one.limited_by.as_ref().unwrap().contains("hard floor"));
        }
        // Order independence: reversing the config order changes nothing.
        let mut reversed = with_aero();
        reversed.positions.reverse();
        let (r, _) = lp_rebuild_suggestions(&reversed, 900.0, tight);
        for one in &s {
            let twin = r.iter().find(|x| x.position_id == one.position_id).unwrap();
            assert_eq!(twin.add_usd, one.add_usd);
        }
        // Nothing above the floor at all: no NOCK-bearing deposit is suggested.
        let none = RebuildFunds { nock: 0.0, floor_tolerance_nock: 0.0, floor_set: true, usdc: 10_000.0 };
        let (s, x) = lp_rebuild_suggestions(&raw, 900.0, none);
        assert!(s.is_empty());
        assert!(x.iter().any(|e| e.reason.contains("cannot be funded") && e.reason.contains("hard floor")));
    }

    #[test]
    fn test_rebuild_unread_balances_are_never_assumed() {
        let mut raw = rebuild_fixture();
        // Pool read failed for one position: unknown, so not suggested.
        raw.pool_liquidity.remove("uniswap-v4:base:3163450");
        let (s, x) = lp_rebuild_suggestions(&raw, 300.0, roomy_funds());
        assert!(s.iter().all(|one| one.nft_id != 3163450));
        assert!(x.iter().any(|e| e.nft_id == 3163450 && e.reason.contains("could not be read")));
        // Wallet gas is never a funding source, read or not: the ETH-leg ranges are
        // excluded either way, with the gas reason, and the USDC/NOCK range stands.
        for (eth, weth) in [(None, None), (Some(0.114226), Some(0.0)), (Some(10.0), Some(10.0))] {
            let mut raw = rebuild_fixture();
            raw.wallet_eth = eth;
            raw.wallet_weth = weth;
            let (s, x) = lp_rebuild_suggestions(&raw, 300.0, roomy_funds());
            assert!(s.iter().all(|one| one.nft_id != 6336662 && one.nft_id != 3163432));
            assert!(x.iter().any(|e| e.nft_id == 6336662 && e.reason.contains("gas")));
            assert!(s.iter().any(|one| one.nft_id == 3163450));
        }
    }

    #[test]
    fn test_reserve_counts_usdc_inside_lp() {
        // $157 of USDC moved into a range: wallet 876, LP 157, stable total still 1,033.
        let mut cfg = plan_cfg();
        cfg.budget_usdc = 100.0;
        cfg.cash_reserve_usdc = Some(950.0);
        let mut guards = roomy_guards();
        guards.wallet_usdc = 876.0;
        guards.lp_usdc = 157.0;
        let now = t("2026-11-15T12:00:00Z");
        // Rebuilding LP did not consume reserve headroom: the $20 tranche still stands.
        assert_eq!(
            plan_directive(&cfg, &PlanFills::default(), &guards, false, now),
            PlanDirective::TrancheDue { usdc: 20.0, limited_by: None }
        );
        // If the range converts that USDC to NOCK on a dip, the stable total falls and
        // the plan stops spending: the reserve is honest about what is left.
        guards.lp_usdc = 0.0;
        let d = plan_directive(&cfg, &PlanFills::default(), &guards, false, now);
        assert!(matches!(d, PlanDirective::Blocked { ref reason } if reason.contains("protected cash reserve")));
    }

    #[test]
    fn test_advice_carries_concrete_rebuild_steps_and_pool_activity() {
        let raw = rebuild_fixture();
        let m = metrics_with_plan(&raw, &PlanContext::none(), &MarketRegime::default());
        assert_eq!(m.lp_rebuild.status, "under_target");
        assert!(!m.lp_rebuild.suggestions.is_empty());
        assert!(m.global_status.shot_advice_details.contains("Suggested: add "));
        assert!(m.global_status.shot_advice_details.contains("#3163450"));
        assert!(!m.global_status.shot_advice_details.contains("#3163431"), "the dead pool must not be suggested");
        // NOCK placed in LP never exceeds what sits above the 100,000 floor.
        let lp_nock: f64 = m.lp_rebuild.suggestions.iter().map(|one| {
            (if one.token0_symbol == "NOCK" { one.amount0.parse::<f64>().unwrap() } else { 0.0 })
                + (if one.token1_symbol == "NOCK" { one.amount1.parse::<f64>().unwrap() } else { 0.0 })
        }).sum();
        assert!(lp_nock <= 115_747.8001 - 100_000.0 + 3.0, "LP NOCK {lp_nock} exceeds the NOCK above the floor");
        // Snapshot positions are stamped live/dead/unknown.
        let mut positions = build_position_snapshots(&raw.positions, raw.spot_price_nock_usd, raw.live_eth_price_usd);
        assert!(positions.iter().all(|p| p.pool_active.is_none()));
        apply_pool_activity(&mut positions, &raw.pool_liquidity);
        assert_eq!(positions.iter().find(|p| p.nft_id == 3163431).unwrap().pool_active, Some(false));
        assert_eq!(positions.iter().find(|p| p.nft_id == 3163450).unwrap().pool_active, Some(true));
        // With no pool reads at all, nothing is suggested and the advice says so.
        let blind = liquid_wallet_fixture();
        let m = metrics_with_plan(&blind, &PlanContext::none(), &MarketRegime::default());
        assert!(m.lp_rebuild.suggestions.is_empty());
        assert!(m.global_status.shot_advice_details.contains("No tracked position is eligible"));
    }

    fn transfer_log(token: &str, from: &str, to: &str, amount: u128) -> serde_json::Value {
        let pad = |a: &str| format!("0x000000000000000000000000{}", a.trim_start_matches("0x").to_lowercase());
        json!({
            "address": token,
            "topics": [TRANSFER_EVENT_TOPIC, pad(from), pad(to)],
            "data": format!("0x{:064x}", amount)
        })
    }

    #[test]
    fn test_parse_accumulation_swap_nets_wallet_flows() {
        let wallet = "0xeE114cC97eC5adeB34bA5645e315B497D8487a45";
        let pool = "0x85f1aa3a70fedd1c52705c15baed143e675cd626";
        let other = "0x1111111111111111111111111111111111111111";
        // $100 USDC out, 8,100 NOCK in (16 decimals); an unrelated transfer is ignored.
        let receipt = json!({ "logs": [
            transfer_log(USDC_BASE, wallet, pool, 100_000_000),
            transfer_log(NOCK_BASE, pool, wallet, 81_000_000_000_000_000_000),
            transfer_log(NOCK_BASE, pool, other, 5_000_000_000_000_000_000),
        ]});
        assert_eq!(parse_accumulation_swap(&receipt, wallet), Some((100_000_000, 81_000_000_000_000_000_000)));
        assert_eq!(format_units(81_000_000_000_000_000_000, 16), "8100");

        // A partial USDC refund nets out.
        let refund = json!({ "logs": [
            transfer_log(USDC_BASE, wallet, pool, 100_000_000),
            transfer_log(USDC_BASE, pool, wallet, 1_000_000),
            transfer_log(NOCK_BASE, pool, wallet, 80_000_000_000_000_000_000),
        ]});
        assert_eq!(parse_accumulation_swap(&refund, wallet), Some((99_000_000, 80_000_000_000_000_000_000)));

        // A SELL (NOCK out, USDC in), a fee claim (both in) and someone else's swap are rejected.
        let sell = json!({ "logs": [
            transfer_log(NOCK_BASE, wallet, pool, 80_000_000_000_000_000_000),
            transfer_log(USDC_BASE, pool, wallet, 99_000_000),
        ]});
        assert_eq!(parse_accumulation_swap(&sell, wallet), None);
        let claim = json!({ "logs": [
            transfer_log(USDC_BASE, pool, wallet, 1_000_000),
            transfer_log(NOCK_BASE, pool, wallet, 10_000_000_000_000_000),
        ]});
        assert_eq!(parse_accumulation_swap(&claim, wallet), None);
        let foreign = json!({ "logs": [
            transfer_log(USDC_BASE, other, pool, 100_000_000),
            transfer_log(NOCK_BASE, pool, other, 81_000_000_000_000_000_000),
        ]});
        assert_eq!(parse_accumulation_swap(&foreign, wallet), None);
        assert_eq!(parse_accumulation_swap(&json!({}), wallet), None);
    }

    #[test]
    fn test_accumulation_plan_mcp_text() {
        let raw = liquid_wallet_fixture();
        let ctx = PlanContext { cfg: Some(plan_cfg()), fills: PlanFills::default(), ledger_unavailable: false, now: t("2026-11-15T12:00:00Z") };
        let m = metrics_with_plan(&raw, &ctx, &MarketRegime::default());
        let data = json!({ "derived_metrics": serde_json::to_value(&m).unwrap() });
        let text = format_accumulation_plan_text(&data);
        assert!(text.contains("Status: tranche_due"));
        assert!(text.contains("1 of 5"));
        assert!(text.contains("$100.00 due now"));
        // A finished plan never reports a tranche as due.
        let fills = PlanFills { count: 5, spent_usdc: 500.0, acquired_nock: 40500.0, last_at: Some(t("2026-11-15T12:00:00Z")), first_price: Some(0.0125) };
        let done_ctx = PlanContext { cfg: Some(plan_cfg()), fills, ledger_unavailable: false, now: t("2026-11-20T12:00:00Z") };
        let done = metrics_with_plan(&raw, &done_ctx, &MarketRegime::default());
        let done_text = format_accumulation_plan_text(&json!({ "derived_metrics": serde_json::to_value(&done).unwrap() }));
        assert!(done_text.contains("Next tranche: none (plan complete)"));
        // Between tranches the text states the size and never "due now".
        let wfills = PlanFills { count: 1, spent_usdc: 100.0, acquired_nock: 8000.0, last_at: Some(t("2026-11-15T12:00:00Z")), first_price: Some(0.0125) };
        let wctx = PlanContext { cfg: Some(plan_cfg()), fills: wfills, ledger_unavailable: false, now: t("2026-11-15T18:00:00Z") };
        let waiting = metrics_with_plan(&raw, &wctx, &MarketRegime::default());
        let wtext = format_accumulation_plan_text(&json!({ "derived_metrics": serde_json::to_value(&waiting).unwrap() }));
        assert!(wtext.contains("Next tranche: 2 of 5 — $100.00 when due"));
        assert!(!wtext.contains("due now"));
        assert!(!done_text.contains("due now"));
        assert!(text.contains("LP rebuild bias: under_target"));
        let rb = rebuild_fixture();
        let rm = metrics_with_plan(&rb, &PlanContext::none(), &MarketRegime::default());
        let rtext = format_accumulation_plan_text(&json!({ "derived_metrics": serde_json::to_value(&rm).unwrap() }));
        assert!(rtext.contains("→ add ") && rtext.contains("https://app.uniswap.org/positions/v4/base/3163450"));
        assert!(rtext.contains("✗ v4 USDC/NOCK 0.75% #3163431: pool has no in-range liquidity"));
        // Before the first fill the average is "n/a", never a made-up number.
        assert!(text.contains("average $n/a"));
        let none = format_accumulation_plan_text(&json!({ "derived_metrics": { "lp_rebuild": {} } }));
        assert!(none.contains("No accumulation_plan is configured"));
    }

    #[test]
    fn test_rsi_series_known_values() {
        // Monotonic gains -> RSI pinned near 100 (overbought)
        let closes: Vec<f64> = (1..=20).map(|i| 100.0 + i as f64).collect();
        let rsi = compute_rsi_series(&closes, 14);
        assert!(rsi[..14].iter().all(|x| x.is_none()));
        assert!((rsi[14]).is_some());
        let last = rsi[19].unwrap();
        assert!(last > 95.0, "all-gain series should push RSI near 100, got {last}");
        // Monotonic losses -> RSI near 0 (oversold)
        let closes: Vec<f64> = (0..20).map(|i| 100.0 - i as f64).collect();
        let rsi = compute_rsi_series(&closes, 14);
        let last = rsi[19].unwrap();
        assert!(last < 5.0, "all-loss series should push RSI near 0, got {last}");
        // Too few samples -> all None
        assert!(compute_rsi_series(&closes[..5], 14).iter().all(|x| x.is_none()));
    }

    #[test]
    fn test_directive_transition_notification() {
        // First boot (no previous directive): never notify
        assert!(directive_transition_notification(None, "A", "d", None).is_none());
        // Unchanged directive: never notify
        assert!(directive_transition_notification(Some("A"), "A", "d", None).is_none());
        // Changed: notify with action + details
        let text = directive_transition_notification(
            Some("HOLD & ACCRETE"),
            "BATCH HARVEST SHOT",
            "Accumulated yield ($5.20) exceeds threshold",
            Some("https://cockpit.example/lite"),
        )
        .unwrap();
        assert!(text.contains("directive change: BATCH HARVEST SHOT"));
        assert!(text.contains("Accumulated yield"));
        assert!(text.contains("https://cockpit.example/lite"));
        // Changed without cockpit URL: no trailing link
        let text = directive_transition_notification(Some("A"), "B", "d", None).unwrap();
        assert!(!text.contains("→"));
    }

    #[test]
    fn test_state_hash_determinism() {
        let data = sample_snapshot_data();

        let hash1 = compute_state_hash(&data);
        let hash2 = compute_state_hash(&data);
        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64);
    }

    #[test]
    fn test_state_hash_ignores_volatile_timestamps() {
        // Two snapshots whose ONLY differences are wall-clock fields (produced fresh each
        // cycle) must hash identically — otherwise snapshot deduplication never fires.
        let mut a = sample_snapshot_data();
        let b = sample_snapshot_data();

        a.wallet_balances[0].updated_at = "2027-01-01T00:00:00Z".to_string();
        a.derived_metrics.global_status.high_water_mark_date = "2027-01-01T00:00:00Z".to_string();
        a.derived_metrics.asset_aggregation_series.time_points = vec!["2027-01-01T00:00:00Z".to_string()];
        a.derived_metrics.total_value_vs_baseline.time_points = vec!["2027-01-01T00:00:00Z".to_string()];
        a.derived_metrics.strategy_counterfactual.time_points = vec!["2027-01-01T00:00:00Z".to_string(); 4];
        a.derived_metrics.structural_channels.channels[0].start_time = "2027-01-01T00:00:00Z".to_string();
        a.derived_metrics.structural_channels.channels[0].break_point = Some("2027-01-01T00:00:00Z".to_string());
        for p in a.derived_metrics.structural_channels.channels[0].center_line.iter_mut() {
            p.time = "2027-01-01T00:00:00Z".to_string();
        }

        assert_eq!(compute_state_hash(&a), compute_state_hash(&b));
    }

    #[test]
    fn test_state_hash_changes_with_state() {
        // Real state changes must change the hash so dedup keeps genuinely new snapshots.
        let mut a = sample_snapshot_data();
        let b = sample_snapshot_data();
        a.wallet_balances[0].amount = "96004.52".to_string();
        a.derived_metrics.global_status.total_nock_amount = "106141".to_string();

        assert_ne!(compute_state_hash(&a), compute_state_hash(&b));
    }

    #[test]
    fn test_counterfactual_series_use_real_timestamps_and_stay_aligned() {
        let history = vec![
            (Utc::now() - chrono::Duration::minutes(30), 2900.0f64, 2600.0f64),
            (Utc::now() - chrono::Duration::minutes(25), 2910.0, 2602.0),
            (Utc::now() - chrono::Duration::minutes(20), 2925.0, 2603.5),
        ];
        let now_str = Utc::now().to_rfc3339();
        let cf = compute_counterfactual_sync(&history, 2950.0, 2605.0, &now_str);

        assert_eq!(cf.time_points.len(), 4);
        assert_eq!(cf.active_portfolio_series.len(), 4);
        assert_eq!(cf.passive_baseline_series.len(), 4);

        // Real snapshot timestamps, in chronological order — no fabricated spacing
        assert_eq!(cf.time_points[0], history[0].0.to_rfc3339());
        assert_eq!(cf.time_points[2], history[2].0.to_rfc3339());
        assert_ne!(cf.time_points[0], cf.time_points[1]);

        // Paired series values align with the same history rows
        assert_eq!(cf.active_portfolio_series[1], "2910.00");
        assert_eq!(cf.passive_baseline_series[1], "2602.00");

        // Current point appended last
        assert_eq!(cf.time_points[3], now_str);
        assert_eq!(cf.active_portfolio_series[3], "2950.00");
        assert_eq!(cf.passive_baseline_series[3], "2605.00");
    }

    #[test]
    fn test_structural_channels_final_window_includes_newest_sample() {
        // 21 samples: the final 20-sample window must include the newest price.
        let base_ts = Utc::now() - chrono::Duration::minutes(60);
        let mut history: Vec<(DateTime<Utc>, f64)> = (0..20)
            .map(|i| (base_ts + chrono::Duration::minutes(i as i64), 100.0))
            .collect();
        let newest_ts = base_ts + chrono::Duration::minutes(20);
        history.push((newest_ts, 200.0)); // outlier newest sample

        let channels = compute_structural_channels_sync(&history);
        let ch = &channels.channels[0];
        let last_center: f64 = ch.center_line.last().unwrap().value.parse().unwrap();

        // Window of the last 20 samples = nineteen 100s + one 200 → mean 105.00
        assert!((last_center - 105.0).abs() < 0.01);
        // Point is labeled with the timestamp of the newest sample in the window
        assert_eq!(ch.center_line.last().unwrap().time, newest_ts.to_rfc3339());
    }

    #[test]
    fn test_tick_to_display_price_matches_live_values() {
        // Derived bracket prices must round-trip against the previously hardcoded (and
        // live-verified) display values for the NOCK/USDC V4 ticks (USDC per NOCK).
        let lower_display = tick_to_display_price_dec(274200, 6, 16);
        let upper_display = tick_to_display_price_dec(223200, 6, 16);

        assert!((lower_display - 0.01237).abs() / 0.01237 < 0.001);
        assert!((upper_display - 2.02783).abs() / 2.02783 < 0.001);
        assert!(lower_display < upper_display);

        // The negated-exponent orientation: larger on-chain tick → lower displayed price
        assert!(tick_to_display_price_dec(274200, 6, 16) < tick_to_display_price_dec(223200, 6, 16));

        // Discovery cross-check: the ETH(18)/NOCK(16) pool's screenshot range
        // (0.08091 → 0.1474 in the operator's UI scale) corresponds to ticks
        // [65200, 71200] — spot 70370 sits between the decoded bounds.
        let lower_eth = tick_to_display_price_dec(71200, 18, 16);
        let upper_eth = tick_to_display_price_dec(65200, 18, 16);
        assert!(lower_eth < upper_eth);
        assert!(upper_eth / lower_eth > 1.5);
    }

    #[test]
    fn test_bracket_distance_bps() {
        // Spot inside the range: distance is to the nearest edge, sign says which side
        let lower = tick_to_display_price_dec(274200, 6, 16); // ~0.012366
        let upper = tick_to_display_price_dec(223200, 6, 16); // ~2.027880
        let d = bracket_distance_bps(1.0, lower, upper);
        // Nearest edge is the lower (~0.0124, 9876 bps away vs 10279 to the upper):
        // (0.012366 - 1.0)/1.0 * 10000 ≈ -9876
        assert!((d as f64 + 9876.0).abs() < 5.0);

        // Spot above range → distance to upper edge is negative
        assert!(bracket_distance_bps(2.5, lower, upper) < 0);
        // Degenerate inputs stay zero instead of panicking on division
        assert_eq!(bracket_distance_bps(0.0, lower, upper), 0);
    }

    #[test]
    fn test_recommended_claim_action_thresholds() {
        // Claimable well above 2x gas → claim now
        assert_eq!(recommended_claim_action(0.12, 0.005), "claim_now");
        // Claimable positive but under the 2x gas margin → moderate recommendation
        assert_eq!(recommended_claim_action(0.008, 0.005), "claim_recommended");
        // Nothing claimable → accumulate
        assert_eq!(recommended_claim_action(0.0, 0.005), "accumulate");
        // Gas unknown (0) is NOT free gas — never claim_now
        assert_eq!(recommended_claim_action(5.0, 0.0), "claim_recommended");
        assert_eq!(recommended_claim_action(0.0, 0.0), "accumulate");
    }

    #[test]
    fn test_recommended_claim_action_below_gas_accumulates() {
        // #153: fees worth less than the gas the claim costs never say claim.
        // The WETH/USDC range #6607225: claimable $0.000674 against $0.0036 gas.
        assert_eq!(recommended_claim_action(0.0007, 0.0036), "accumulate");
        assert_eq!(recommended_claim_action(0.000674, 0.0036), "accumulate");
        // Exactly the gas cost breaks even: moderate recommendation.
        assert_eq!(recommended_claim_action(0.0036, 0.0036), "claim_recommended");
        assert_eq!(recommended_claim_action(0.0071, 0.0036), "claim_recommended");
        // Twice the gas cost or more: claim now.
        assert_eq!(recommended_claim_action(0.0072, 0.0036), "claim_now");
        // Gas unknown keeps the moderate recommendation for any positive amount.
        assert_eq!(recommended_claim_action(0.0007, 0.0), "claim_recommended");
        assert_eq!(recommended_claim_action(0.0007, -1.0), "claim_recommended");
    }

    fn sample_snapshot_data() -> SnapshotData {
        SnapshotData {
            closed_positions: vec![],
            wallet_balances: vec![WalletBalance {
                wallet_address: "0x123".to_string(),
                token_symbol: "NOCK".to_string(),
                token_address: "0xNOCK".to_string(),
                amount: "96004.52".to_string(),
                amount_usd: "2496.12".to_string(),
                updated_at: "2026-10-11T00:00:00Z".to_string(),
            }],
            pool_mechanics: vec![],
            positions: vec![],
            concentrated_liquidity: vec![],
            contract_fees: vec![],
            derived_metrics: DerivedMetrics {
                global_status: GlobalStatus {
                    total_portfolio_usd: "100.00".to_string(),
                    total_native_usd: "50.00".to_string(),
                    total_stable_usd: "50.00".to_string(),
                    total_fees_accrued_usd: "1.00".to_string(),
                    total_fees_claimable_usd: "1.00".to_string(),
                    gas_to_fee_delta_usd: "0.99".to_string(),
                    high_water_mark_usd: "100.00".to_string(),
                    high_water_mark_date: "2026-10-11T00:00:00Z".to_string(),
                    total_nock_amount: "105000".to_string(),
                    nock_floor_delta: "5000".to_string(),
                    nock_floor_target: "100000".to_string(),
                    total_usdc_amount: "50.00".to_string(),
                    shot_advice_action: "HOLD".to_string(),
                    shot_advice_details: "Steady".to_string(),
                    regime: "normal".to_string(),
                    rsi_14: None,
                    rsi_14_1d: None,
                    ..Default::default()
                },
                sovereign_ledger: Default::default(),
                accumulation_plan: None,
                lp_rebuild: LpRebuild::default(),
                cost_basis: None,
                floor_breach: None,
                asset_aggregation_series: AssetAggregationSeries {
                    time_points: vec!["2026-10-11T00:00:00Z".to_string()],
                    safe_haven_series: vec!["48.00".to_string()],
                    wallet_nock_series: vec![],
                    active_protocol_series: vec!["2.00".to_string()],
                    positions_nock: vec![],
                },
                tick_bracket_hypervisor: TickBracketHypervisor {
                    brackets: vec![],
                    current_price: "0.026".to_string(),
                },
                velocity_gas_sandbox: VelocityGasSandbox {
                    contracts: vec![],
                },
                portfolio_deviation_matrix: PortfolioDeviationMatrix {
                    allocations: vec![],
                },
                strategy_counterfactual: StrategyCounterfactual {
                    time_points: vec![
                        "2026-10-11T00:00:00Z".to_string(),
                        "2026-10-11T00:05:00Z".to_string(),
                        "2026-10-11T00:10:00Z".to_string(),
                    ],
                    active_portfolio_series: vec!["99.00".to_string(), "99.50".to_string(), "100.00".to_string()],
                    passive_baseline_series: vec!["49.00".to_string(), "49.50".to_string(), "50.00".to_string()],
                },
                structural_channels: StructuralChannels {
                    channels: vec![RegressionChannel {
                        start_time: "2026-10-11T00:00:00Z".to_string(),
                        end_time: None,
                        center_line: vec![
                            LinePoint { time: "2026-10-11T00:00:00Z".to_string(), value: "99.00".to_string() },
                            LinePoint { time: "2026-10-11T00:05:00Z".to_string(), value: "100.00".to_string() },
                        ],
                        upper_1sd: vec![],
                        lower_1sd: vec![],
                        upper_2sd: vec![],
                        lower_2sd: vec![],
                        is_active: true,
                        break_point: None,
                    }],
                },
                total_value_vs_baseline: TotalValueVsBaseline {
                    time_points: vec!["2026-10-11T00:00:00Z".to_string()],
                    portfolio_value_series: vec!["100.00".to_string()],
                    high_water_mark_series: vec!["100.00".to_string()],
                    token_price_series: vec!["0.02600".to_string()],
                },
            },
        }
    }

    #[test]
    fn test_eth_usd_spot_parsing() {
        // Sample return from Aerodrome vAMM-WETH/USDC getReserves (0x0902f1ac)
        // r0 = 0x997534da7f7a6b1a4 = 176929380488719577508 (176.929 WETH with 18 decimals)
        // r1 = 0x689640625e = 449198383710 (449,198.38 USDC with 6 decimals)
        let hex_str = "0x00000000000000000000000000000000000000000000000997534da7f7a6b1a4000000000000000000000000000000000000000000000000000000689640625e";
        let clean = hex_str.trim_start_matches("0x");
        assert!(clean.len() >= 128);

        let r0 = u128::from_str_radix(&clean[..64], 16).unwrap();
        let r1 = u128::from_str_radix(&clean[64..128], 16).unwrap();

        let weth = r0 as f64 / 1e18;
        let usdc = r1 as f64 / 1e6;
        assert!(weth > 0.0);
        let eth_price = usdc / weth;

        // Verify ETH price falls within expected ~$2538 range
        assert!((eth_price - 2538.86).abs() < 1.0);
    }

    #[test]
    fn test_u256_wrapping_sub_and_mul_div() {
        // Live on-chain feeGrowthInside values for Uniswap V4 NOCK/USDC pool #3126000
        let fg0_curr = U256::from_hex_be("00000000000000000000002af933ab46390e82c57ffd6903").unwrap();
        let fg0_last = U256::from_hex_be("000000000000000000000024a1cdc1fa3d9c5ed6c43492cf").unwrap();
        let fg1_curr = U256::from_hex_be("000000000000087947b99b01f17bd63c97cbe99129a02a0e").unwrap();
        let fg1_last = U256::from_hex_be("00000000000005db4b66cee311145e59a7723d5ce16bb18f").unwrap();
        let liquidity = 81297590130616u128;

        let delta0 = fg0_curr.wrapping_sub(&fg0_last);
        let delta1 = fg1_curr.wrapping_sub(&fg1_last);

        let fee0_raw = delta0.mul_div_q128(liquidity);
        let fee1_raw = delta1.mul_div_q128(liquidity);

        // Verify exact fee token amounts match on-chain Uniswap mobile UI
        assert_eq!(fee0_raw, 120033); // 0.120033 USDC (6 decimals)
        assert_eq!(fee1_raw, 54468217823004916); // 5.446822 NOCK (16 decimals)

        let usdc = fee0_raw as f64 / 1e6;
        let nock = fee1_raw as f64 / 1e16;
        assert!((usdc - 0.120033).abs() < 1e-6);
        assert!((nock - 5.446822).abs() < 1e-6);
    }

    #[test]
    fn test_v4_collect_unlock_data_golden_vector() {
        // Golden vector: byte-for-byte identical to @uniswap/v4-sdk@2.4.1
        // V4PositionPlanner.finalize() for addDecrease(3126000, 0, 0, 0, no hookData)
        // + addTakePair(USDC, NOCK, sovereign wallet). Accepted by live Base mainnet
        // eth_call simulation against the deployed PositionManager on 2026-10-12
        // (audit trail: helix-specs task 000661 verification/v4collect_sim.py).
        let unlock = build_v4_collect_unlock_data(
            3126000,
            "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
            "0x9B5E262cF9bb04869ab40b19AF91D2dc85761722",
            "0xeE114cC97eC5adeB34bA5645e315B497D8487a45",
        );
        assert_eq!(unlock, "0x000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000800000000000000000000000000000000000000000000000000000000000000002011100000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000040000000000000000000000000000000000000000000000000000000000000012000000000000000000000000000000000000000000000000000000000000000c000000000000000000000000000000000000000000000000000000000002fb2f000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000a000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000060000000000000000000000000833589fcd6edb6e08f4c7c32d4f71b54bda029130000000000000000000000009b5e262cf9bb04869ab40b19af91d2dc85761722000000000000000000000000ee114cc97ec5adeb34ba5645e315b497d8487a45");
    }

    #[test]
    fn test_v4_collect_calldata_golden_vector() {
        // Full outer calldata accepted by the live eth_call simulation (selector
        // modifyLiquidities(bytes,uint256) = 0xdd46508f; offset 0x40; deadline
        // 0xFFFFFFFFFFFF; bytes length word; unlockData). Reference bytes:
        // helix-specs task 000661 verification/verified_calldata.txt.
        let data = build_v4_collect_calldata(
            3126000,
            "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
            "0x9B5E262cF9bb04869ab40b19AF91D2dc85761722",
            "0xeE114cC97eC5adeB34bA5645e315B497D8487a45",
            0xFFFFFFFFFFFF,
        );
        assert_eq!(data, "0xdd46508f00000000000000000000000000000000000000000000000000000000000000400000000000000000000000000000000000000000000000000000ffffffffffff0000000000000000000000000000000000000000000000000000000000000240000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000800000000000000000000000000000000000000000000000000000000000000002011100000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000040000000000000000000000000000000000000000000000000000000000000012000000000000000000000000000000000000000000000000000000000000000c000000000000000000000000000000000000000000000000000000000002fb2f000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000a000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000060000000000000000000000000833589fcd6edb6e08f4c7c32d4f71b54bda029130000000000000000000000009b5e262cf9bb04869ab40b19af91d2dc85761722000000000000000000000000ee114cc97ec5adeb34ba5645e315b497d8487a45");
    }

    #[test]
    fn test_v3_collect_calldata_golden_vector() {
        // collect((uint256,address,uint128,uint128)) on the canonical Base NPM
        // 0x03a520b3…Ed34f1 for tokenId 6336662 (WETH/NOCK 1%). Live-verified via
        // eth_call 2026-10-24: returns (0, 1654339369283004) = 0.165434 NOCK —
        // exactly the snapshot's tokens_owed_1 for uniswap-v3:base:6336662.
        let data = build_v3_collect_calldata(
            6336662,
            "0xeE114cC97eC5adeB34bA5645e315B497D8487a45",
        );
        assert_eq!(data, "0xfc6f7865000000000000000000000000000000000000000000000000000000000060b096000000000000000000000000ee114cc97ec5adeb34ba5645e315b497d8487a4500000000000000000000000000000000ffffffffffffffffffffffffffffffff00000000000000000000000000000000ffffffffffffffffffffffffffffffff");
    }

    #[test]
    fn test_v4_collect_unlock_data_decrease_only_golden_vector() {
        // Single-action variant (DECREASE only, actions byte 0x01), also live-verified.
        let unlock = build_v4_collect_unlock_data_decrease_only(3126000);
        assert_eq!(unlock, "0x00000000000000000000000000000000000000000000000000000000000000400000000000000000000000000000000000000000000000000000000000000080000000000000000000000000000000000000000000000000000000000000000101000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000c000000000000000000000000000000000000000000000000000000000002fb2f000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000a00000000000000000000000000000000000000000000000000000000000000000");
    }

    #[test]
    fn test_format_units_exact_decimal() {
        assert_eq!(format_units(120033, 6), "0.120033");
        assert_eq!(format_units(54468217823004916, 16), "5.4468217823004916");
        assert_eq!(format_units(1, 6), "0.000001");
        assert_eq!(format_units(1000000, 6), "1");
        assert_eq!(format_units(0, 16), "0");
        assert_eq!(format_units(4000000, 6), "4");
    }

    #[test]
    fn test_is_valid_tx_hash() {
        assert!(is_valid_tx_hash(
            "0xdd46508fdd46508fdd46508fdd46508fdd46508fdd46508fdd46508fdd46508f"
        ));
        assert!(is_valid_tx_hash(
            "dd46508fdd46508fdd46508fdd46508fdd46508fdd46508fdd46508fdd46508f"
        ));
        assert!(!is_valid_tx_hash("0x1234"));
        assert!(!is_valid_tx_hash(""));
        assert!(!is_valid_tx_hash(
            "zz46508fdd46508fdd46508fdd46508fdd46508fdd46508fdd46508fdd46508f"
        ));
    }

    #[test]
    fn test_parse_harvest_transfers_receipt_parsing() {
        let wallet = "0xeE114cC97eC5adeB34bA5645e315B497D8487a45";
        let from = "000000000000000000000000360e68faccca8ca495c1b759fd9eee466db9fb32";
        let wallet_word = format!("000000000000000000000000{}", wallet.trim_start_matches("0x").to_lowercase());
        let other_word = "0000000000000000000000001111111111111111111111111111111111111111";
        let receipt = json!({
            "status": "0x1",
            "blockNumber": "0x1234",
            "logs": [
                // USDC Transfer -> wallet (keep)
                { "address": USDC_BASE, "topics": [
                    TRANSFER_EVENT_TOPIC, from, wallet_word ], "data": "0x000000000000000000000000000000000000000000000000000000000001d4af" },
                // NOCK Transfer -> wallet (keep)
                { "address": NOCK_BASE, "topics": [
                    TRANSFER_EVENT_TOPIC, from, wallet_word ], "data": "0x0000000000000000000000000000000000000000000000000c1828e36e388f4" },
                // USDC Transfer -> someone else (skip)
                { "address": USDC_BASE, "topics": [
                    TRANSFER_EVENT_TOPIC, from, other_word ], "data": "0x1" },
                // Untracked token -> wallet (skip)
                { "address": "0xdead000000000000000000000000000000000001", "topics": [
                    TRANSFER_EVENT_TOPIC, from, wallet_word ], "data": "0x2" },
                // wrong topic (skip)
                { "address": USDC_BASE, "topics": [
                    "0x8c5be1e5ebec7d5bd14f71427d1e84f3dd0314c0f7b2291e5b200ac8c7c3b925", from, wallet_word ], "data": "0x3" }
            ]
        });
        let transfers = parse_harvest_transfers(&receipt, wallet);
        assert_eq!(transfers.len(), 2);
        // 0x1d4af = 119983 raw USDC
        assert_eq!(transfers[0].token_symbol, "USDC");
        assert_eq!(transfers[0].amount_raw, "119983");
        assert_eq!(transfers[0].amount_human, "0.119983");
        assert_eq!(transfers[0].decimals, 6);
        // 0xc1e3fd41c20874 = 54468217823004916 raw NOCK
        assert_eq!(transfers[1].token_symbol, "NOCK");
        assert_eq!(transfers[1].amount_raw, "54468217823004916");
        assert_eq!(transfers[1].amount_human, "5.4468217823004916");
        assert_eq!(transfers[1].decimals, 16);
    }

    fn word(addr: &str) -> String {
        format!("0x000000000000000000000000{}", addr.trim_start_matches("0x").to_lowercase())
    }

    #[test]
    fn test_parse_harvest_transfers_books_weth_fees() {
        // #111: a v3 WETH/NOCK collect straight to the wallet pays WETH by Transfer.
        let wallet = "0xeE114cC97eC5adeB34bA5645e315B497D8487a45";
        let pool = "0x1111111111111111111111111111111111111111";
        let receipt = json!({
            "status": "0x1", "from": wallet,
            "logs": [
                { "address": WETH_BASE, "topics": [TRANSFER_EVENT_TOPIC, word(pool), word(wallet)],
                  "data": "0x00000000000000000000000000000000000000000000000000038d7ea4c68000" },
                { "address": NOCK_BASE, "topics": [TRANSFER_EVENT_TOPIC, word(pool), word(wallet)],
                  "data": "0x0000000000000000000000000000000000000000000000000de0b6b3a7640000" }
            ]
        });
        let t = parse_harvest_transfers(&receipt, wallet);
        assert_eq!(t.len(), 2);
        assert_eq!((t[0].token_symbol.as_str(), t[0].decimals, t[0].amount_human.as_str()), ("WETH", 18, "0.001"));
        assert_eq!(t[1].token_symbol, "NOCK");
    }

    #[test]
    fn test_parse_harvest_transfers_books_eth_unwrapped_by_a_position_manager() {
        // #111: the Uniswap app collects to the NPM and unwraps; the wallet gets native ETH,
        // which leaves only a WETH Withdrawal by the NPM.
        let wallet = "0xeE114cC97eC5adeB34bA5645e315B497D8487a45";
        let pool = "0x1111111111111111111111111111111111111111";
        let withdrawal = |src: &str| json!({ "address": WETH_BASE, "topics": [WETH_WITHDRAWAL_TOPIC, word(src)],
            "data": "0x00000000000000000000000000000000000000000000000000071afd498d0000" });
        let weth_to_npm = json!({ "address": WETH_BASE, "topics": [TRANSFER_EVENT_TOPIC, word(pool), word(UNISWAP_V3_NPM)],
            "data": "0x00000000000000000000000000000000000000000000000000071afd498d0000" });
        let receipt = json!({ "status": "0x1", "from": wallet, "logs": [weth_to_npm.clone(), withdrawal(UNISWAP_V3_NPM)] });
        let t = parse_harvest_transfers(&receipt, wallet);
        assert_eq!(t.len(), 1, "the WETH Transfer to the NPM is not the wallet's; only the unwrap is booked");
        assert_eq!(t[0].token_symbol, "ETH");
        assert_eq!(t[0].token_address, NATIVE_ETH_LEDGER_ADDRESS);
        assert_eq!(t[0].amount_human, "0.002");
        assert_eq!(t[0].from_address, UNISWAP_V3_NPM.to_lowercase());
        // The v4 PositionManager counts too.
        let r4 = json!({ "status": "0x1", "from": wallet, "logs": [withdrawal(UNISWAP_V4_POSM_BASE)] });
        assert_eq!(parse_harvest_transfers(&r4, wallet).len(), 1);
        // A router or anyone else unwrapping is not a fee payout.
        let other = json!({ "status": "0x1", "from": wallet, "logs": [withdrawal(pool)] });
        assert!(parse_harvest_transfers(&other, wallet).is_empty());
        // A transaction someone else sent never books an unwrap to this wallet.
        let not_ours = json!({ "status": "0x1", "from": pool, "logs": [withdrawal(UNISWAP_V3_NPM)] });
        assert!(parse_harvest_transfers(&not_ours, wallet).is_empty());
    }

    #[test]
    fn test_sovereign_ledger_reports_its_ratio_and_unpriced_rows() {
        // #102: the panel's labels come from the data, so the ledger carries the ratio.
        let raw = four_position_fixture();
        let agg = LedgerAggregate { total_usd: 10.0, harvest_count: 2, last_at: None, unpriced_count: 1 };
        let m = compute_derived_metrics(&raw, 0.0, &OperatorProfile::default(), StructuralChannels { channels: vec![] }, &[], &agg, &MarketRegime::default());
        let l = &m.sovereign_ledger;
        assert_eq!((l.tax_reserve_ratio.as_str(), l.tax_reserve_usd.as_str(), l.growth_dry_powder_usd.as_str()), ("0.00", "0.00", "10.00"));
        assert_eq!(l.unpriced_count, 1);
        let mut profile = OperatorProfile::default();
        profile.tax_reserve_ratio = 0.25;
        let m = compute_derived_metrics(&raw, 0.0, &profile, StructuralChannels { channels: vec![] }, &[], &agg, &MarketRegime::default());
        let l = &m.sovereign_ledger;
        assert_eq!((l.tax_reserve_ratio.as_str(), l.tax_reserve_usd.as_str(), l.growth_dry_powder_usd.as_str()), ("0.25", "2.50", "7.50"));
    }

    #[test]
    fn test_harvest_valuation_uses_the_block_prices_or_nothing() {
        let nock = (0.0125, "market_price_history aerodrome_rpc at T".to_string());
        let eth = (1790.0, "eth_price_pool slot0 at block 1".to_string());
        assert_eq!(harvest_unit_price("USDC", None, None), Some((1.0, "USDC at $1")));
        assert_eq!(harvest_unit_price("NOCK", Some(&nock), Some(&eth)).map(|p| p.0), Some(0.0125));
        assert_eq!(harvest_unit_price("WETH", Some(&nock), Some(&eth)).map(|p| p.0), Some(1790.0));
        assert_eq!(harvest_unit_price("ETH", None, Some(&eth)).map(|p| p.1), Some("eth_price_pool slot0 at block 1"));
        // No price found: no value, never a substitute.
        assert_eq!(harvest_unit_price("NOCK", None, Some(&eth)), None);
        assert_eq!(harvest_unit_price("AERO", Some(&nock), Some(&eth)), None);
        assert_eq!(harvest_usd_value("0.002", Some(1790.0)).as_deref(), Some("3.5800"));
        assert_eq!(harvest_usd_value("1000", Some(0.0125)).as_deref(), Some("12.5000"));
        assert_eq!(harvest_usd_value("1000", None), None);
        assert_eq!(harvest_usd_value("1000", Some(0.0)), None);
        assert_eq!(harvest_usd_value("abc", Some(1.0)), None);
    }

    #[test]
    fn test_concentrated_amounts_match_discovery_magnitudes() {
        // Golden anchors from Phase-1 discovery (live chain reads on 2026-10-24).
        // Position B (v4 NOCK/USDC 0.75%): screenshot-era value ≈ $71.45 with the
        // split 62.82% USDC / 37.18% NOCK.
        let b = fixture_position_v4_usdc();
        let (a0, a1) = concentrated_amounts_raw(b.liquidity, b.sqrt_price_x96, b.tick_lower, b.tick_upper);
        let usdc = a0 / 1e6;
        let nock = a1 / 1e16;
        let nock_usd = 0.020783;
        let total = usdc + nock * nock_usd;
        assert!((total - 71.45).abs() / 71.45 < 0.05, "B total {}", total);
        let usdc_pct = usdc / total * 100.0;
        assert!((usdc_pct - 62.82).abs() < 2.0, "USDC share {}%", usdc_pct);
        assert!(b.tick_lower <= b.tick && b.tick <= b.tick_upper);

        // Position A (v4 ETH/NOCK 2%): compounded to ~$156 since the $89.48 screenshot,
        // distribution preserved (~15% ETH / ~85% NOCK).
        let a = fixture_position_v4_nock_eth();
        let (a0, a1) = concentrated_amounts_raw(a.liquidity, a.sqrt_price_x96, a.tick_lower, a.tick_upper);
        let eth = a0 / 1e18;
        let nock = a1 / 1e16;
        let eth_usd = 2373.14;
        let total = eth * eth_usd + nock * nock_usd;
        assert!(total > 130.0 && total < 180.0, "A total {}", total);
        let eth_pct = eth * eth_usd / total * 100.0;
        assert!((eth_pct - 15.13).abs() < 3.0, "ETH share {}%", eth_pct);

        // Position C (v3 WETH/NOCK 1%): ≈ $18 with 99.17% WETH.
        let c = fixture_position_v3_weth_nock();
        let (a0, a1) = concentrated_amounts_raw(c.liquidity, c.sqrt_price_x96, c.tick_lower, c.tick_upper);
        let weth = a0 / 1e18;
        let nock = a1 / 1e16;
        let total = weth * eth_usd + nock * nock_usd;
        assert!((total - 18.13).abs() / 18.13 < 0.05, "C total {}", total);
        let weth_pct = weth * eth_usd / total * 100.0;
        assert!(weth_pct > 97.0, "WETH share {}%", weth_pct);
    }

    #[test]
    fn test_decode_v3_positions_return_golden() {
        // REAL 12-word return from NPM positions(6336662) on Base (Phase-1 discovery).
        let hex = "0x"
            .to_string()
            + "0000000000000000000000000000000000000000000000000000000000000000" // nonce
            + "0000000000000000000000000000000000000000000000000000000000000000" // operator
            + "0000000000000000000000004200000000000000000000000000000000000006" // token0 WETH
            + "0000000000000000000000009b5e262cf9bb04869ab40b19af91d2dc85761722" // token1 NOCK
            + "0000000000000000000000000000000000000000000000000000000000002710" // fee 10000 = 1%
            + "0000000000000000000000000000000000000000000000000000000000011300" // tickLower 70400
            + "0000000000000000000000000000000000000000000000000000000000012430" // tickUpper 74800
            + "000000000000000000000000000000000000000000000000121d7ef51ca5e1ae" // liquidity
            + "0000000000000000000000000000000000000000000000000000000000000000" // fgInside0Last
            + "0000000000000000000000000000000000000000000000000000000000000000" // fgInside1Last
            + "0000000000000000000000000000000000000000000000000000000000000000" // tokensOwed0
            + "0000000000000000000000000000000000000000000000000000000000000000"; // tokensOwed1
        let d = decode_v3_positions_return(&hex).expect("decode");
        assert_eq!(d.token0, "0x4200000000000000000000000000000000000006");
        assert_eq!(d.token1, "0x9b5e262cf9bb04869ab40b19af91d2dc85761722");
        assert_eq!(d.fee, 10000);
        assert_eq!(d.tick_lower, 70400);
        assert_eq!(d.tick_upper, 74800);
        assert_eq!(d.liquidity, 1305339058220032430);
        assert_eq!(d.tokens_owed0, 0);
        assert_eq!(d.tokens_owed1, 0);
        // Fee tier in bps as the pipeline derives it: fee/100 = 100 bps = 1%.
        assert_eq!((d.fee / 100) as u32, 100);
    }

    #[test]
    fn test_decode_v3_positions_return_rejects_short() {
        assert!(decode_v3_positions_return("0x1234").is_err());
    }

    #[test]
    fn test_price_source_liquidity_guard() {
        // Live pool: any non-zero active liquidity passes.
        assert!(check_price_source_liquidity("uniswap-v4:base:3163450", "0x064a", 80173935548453779).is_ok());
        // Dead pool (the v4 0.75% pool after the 2026-11 liquidation): frozen price is rejected.
        let err = check_price_source_liquidity("uniswap-v4:base:3163431", "0x91a9", 0).unwrap_err();
        assert!(err.to_string().contains("zero in-range liquidity"));
        assert!(err.to_string().contains("0x91a9"));
    }

    #[test]
    fn test_nock_spot_from_positions() {
        // The price-source position (NOCK/USDC v4) supplies the NOCK/USD spot.
        let a = fixture_position_v4_nock_eth();
        let b = fixture_position_v4_usdc();
        let spot = nock_spot_from_positions(&[a.clone(), b.clone()]).expect("spot");
        assert!((spot - 0.020783).abs() < 0.0002, "spot {}", spot);

        // No price source configured -> None (caller fails the cycle explicitly).
        let mut b2 = b.clone();
        b2.is_price_source = false;
        assert!(nock_spot_from_positions(&[a, b2]).is_none());
    }

    #[test]
    fn test_app_config_positions() {
        // Real discovered identifiers (2026-10-24) as they appear in config.yaml.
        let yaml = r#"
database_url: "postgresql://localhost/test"
alchemy_api_keys: ["key1"]
tracked_wallets: ["0xee114cc97ec5adeb34ba5645e315b497d8487a45"]
sync_window_seconds: 60
positions:
  - id: "uniswap-v4-nock-usdc"
    protocol: "uniswap-v4"
    nft_id: 3163431
    pool_id: "0x91a9ef0a8dd231b99fa53a2a145a9e8bca7897e0d85852107abb997916f9f99c"
    currency0: "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913"
    currency1: "0x9B5E262cF9bb04869ab40b19AF91D2dc85761722"
    tick_lower: 267825
    tick_upper: 271050
    is_price_source: true
  - id: "uniswap-v4-nock-eth"
    protocol: "uniswap-v4"
    nft_id: 3163432
    pool_id: "0xd316152008e12105594c32686b9d0401fd2c60ca4805c84f280069d218b771a8"
    currency0: "0x0000000000000000000000000000000000000000"
    currency1: "0x9B5E262cF9bb04869ab40b19AF91D2dc85761722"
    tick_lower: 65200
    tick_upper: 71200
  - id: "uniswap-v4-nock-usdc-wide"
    protocol: "uniswap-v4"
    nft_id: 3163450
    pool_id: "0x064a5df2ecfcb7c05779b147aa38e4b0e91c966267fbdbc8cf16addd83636b13"
    currency0: "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913"
    currency1: "0x9B5E262cF9bb04869ab40b19AF91D2dc85761722"
    tick_lower: 262200
    tick_upper: 282300
  - id: "uniswap-v3-weth-nock"
    protocol: "uniswap-v3"
    nft_id: 6336662
    npm_address: "0x03a520b32C04BF3bEEf7BEb72E919cf822Ed34f1"
    pool_address: "0x71a4a6f6ea5c26ce486b7ed0d8723a897ef5ad9a"
eth_price_pool: "0xd0b53d9277642d899df5c87a3966a349a798f224"
"#;
        let cfg: AppConfig = serde_yaml::from_str(yaml).expect("Failed to deserialize yaml");
        assert_eq!(cfg.positions.len(), 4);
        assert_eq!(cfg.positions[0].nft_id, 3163431);
        assert_eq!(cfg.positions[0].tick_lower, Some(267825));
        assert!(cfg.positions[0].is_price_source);
        assert_eq!(cfg.positions[1].protocol, "uniswap-v4");
        assert!(!cfg.positions[1].is_price_source);
        assert_eq!(cfg.positions[2].nft_id, 3163450);
        assert_eq!(cfg.positions[2].tick_lower, Some(262200));
        assert_eq!(cfg.positions[2].tick_upper, Some(282300));
        assert!(!cfg.positions[2].is_price_source);
        assert_eq!(cfg.positions[3].protocol, "uniswap-v3");
        assert_eq!(cfg.positions[3].npm_address.as_deref(), Some("0x03a520b32C04BF3bEEf7BEb72E919cf822Ed34f1"));
        assert_eq!(cfg.positions[3].pool_address.as_deref(), Some("0x71a4a6f6ea5c26ce486b7ed0d8723a897ef5ad9a"));
        assert_eq!(cfg.eth_price_pool.as_deref(), Some("0xd0b53d9277642d899df5c87a3966a349a798f224"));
        assert_eq!(cfg.mcp_bearer_token, None);

        // Defaults: positions optional (worker fails the cycle with a clear error),
        // ETH price pool falls back to the verified canonical address.
        let minimal: AppConfig = serde_yaml::from_str("database_url: \"x\"").unwrap();
        assert!(minimal.positions.is_empty());
        assert_eq!(minimal.eth_price_pool.as_deref(), Some(DEFAULT_ETH_PRICE_POOL));
    }

    #[test]
    fn test_mcp_text_formatters() {
        let sample_data = json!({
            "contract_fees": [
                {
                    "contract_address": "0x7c5f5a4bbd8fd63184577525326123b519429bdc",
                    "fee_token_address": "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
                    "fee_token_symbol": "USDC+NOCK (Uniswap V4)",
                    "amount": "0.030 USDC + 1.83 NOCK",
                    "amount_usd": "0.0753",
                    "claimable": true,
                    "gas_break_even_usd": "0.0043"
                },
                {
                    "contract_address": "0x85f1aa3a70fedd1c52705c15baed143e675cd626",
                    "fee_token_address": "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
                    "fee_token_symbol": "USDC+NOCK (Aerodrome)",
                    "amount": "0.03772 USDC + 2.14164 NOCK",
                    "amount_usd": "0.0902",
                    "claimable": true,
                    "gas_break_even_usd": "0.0043"
                }
            ],
            "derived_metrics": {
                "global_status": {
                    "total_portfolio_usd": "2952.37",
                    "total_nock_amount": "106140",
                    "nock_floor_delta": "6140",
                    "nock_floor_target": "100000",
                    "total_usdc_amount": "349.68",
                    "high_water_mark_usd": "3200.25",
                    "shot_advice_action": "HOLD & ACCRETE",
                    "shot_advice_details": "Accreting yield steadily",
                    "total_fees_claimable_usd": "0.17",
                    "gas_to_fee_delta_usd": "0.16"
                }
            }
        });

        let state_text = format_portfolio_state_text(&sample_data);
        assert!(state_text.contains("106140 NOCK"));
        assert!(state_text.contains("+6140 above the 100000 NOCK floor"), "{state_text}");
        assert!(!state_text.contains("100k"));
        assert!(state_text.contains("$2952.37 USD"));

        let advice_text = format_shot_advice_text(&sample_data);
        assert!(advice_text.contains("HOLD & ACCRETE"));
        assert!(advice_text.contains("$0.17 USD"));

        let claim_text = format_claim_simulation_text(&sample_data);
        assert!(claim_text.contains("Uniswap V4"));
        assert!(claim_text.contains("Aerodrome"));
        assert!(claim_text.contains("0.0753"));
    }

    #[test]
    fn test_claim_simulation_text_uses_live_gas_never_fabricates() {
        // Live per-contract gas break-even is surfaced verbatim; the historical
        // "~$0.005 USD" fabrication must never appear.
        let data = json!({
            "contract_fees": [
                { "fee_token_symbol": "USDC+NOCK (Uniswap V4)", "amount": "0.02 USDC",
                  "amount_usd": "0.0279", "gas_break_even_usd": "0.0041" },
                { "fee_token_symbol": "USDC+NOCK (Aerodrome)", "amount": "0.01 USDC",
                  "amount_usd": "0.0167", "gas_break_even_usd": "0.0042" }
            ]
        });
        let text = format_claim_simulation_text(&data);
        assert!(text.contains("0.0042"), "representative live gas (max) should be shown: {text}");
        assert!(!text.contains("~$0.005"), "fabricated constant gas must not appear: {text}");
        assert!(text.contains("Net Accretion if Harvested: $0.0404"), "net = gross - live gas: {text}");

        // Missing gas data degrades honestly instead of substituting a constant.
        let no_gas = json!({
            "contract_fees": [
                { "fee_token_symbol": "X", "amount": "1", "amount_usd": "0.50" }
            ]
        });
        let text2 = format_claim_simulation_text(&no_gas);
        assert!(text2.contains("unavailable"), "gas absence must say so: {text2}");
        assert!(!text2.contains("0.005"), "no fabricated gas constant: {text2}");
    }

    #[test]
    fn test_no_synthetic_bracket_emitted() {
        // The Tick Bracket payload contains ONLY brackets derived from live per-position
        // ticks. Empty positions -> zero brackets (the old fabricated Aerodrome
        // 0.00000 -> 999.00000 placeholder stays gone).
        let raw = RawOnChainData::default();
        let metrics = compute_derived_metrics(
            &raw,
            0.0,
            &OperatorProfile::default(),
            StructuralChannels { channels: vec![] },
            &[],
            &LedgerAggregate::default(),
            &MarketRegime::default(),
        );
        assert_eq!(metrics.tick_bracket_hypervisor.brackets.len(), 0);

        // With the four-position fixture, exactly four live brackets.
        let metrics = compute_derived_metrics(
            &four_position_fixture(),
            0.0,
            &OperatorProfile::default(),
            StructuralChannels { channels: vec![] },
            &[],
            &LedgerAggregate::default(),
            &MarketRegime::default(),
        );
        assert_eq!(metrics.tick_bracket_hypervisor.brackets.len(), 4);
        let bracket = &metrics.tick_bracket_hypervisor.brackets[0];
        assert_ne!(bracket.lower_price, "0.00000");
        assert_ne!(bracket.upper_price, "999.00000");
    }

    // ============================================================
    // Market price backfill & multi-timeframe regime (v0.3.0)
    // ============================================================

    // ============================================================
    // v2.0.0 per-position fixtures — REAL on-chain values from Phase-1 discovery
    // (2026-10-24, see design.md / ONCHAIN_INTEGRATION.md §5). The chain-read
    // fields (liquidity, sqrtPrice, ticks, fees) are golden anchors; amounts are
    // derived through the same math the pipeline uses so tests double as
    // regression checks on the screenshot magnitudes.
    // ============================================================

    fn fixture_position_v4_nock_eth() -> RawPosition {
        RawPosition {
            position_id: "uniswap-v4:base:3163432".to_string(),
            network: "base".to_string(),
            protocol: "uniswap-v4".to_string(),
            wallet: "0xee114cc97ec5adeb34ba5645e315b497d8487a45".to_string(),
            nft_id: 3163432,
            pool_id: "0xd316152008e12105594c32686b9d0401fd2c60ca4805c84f280069d218b771a8".to_string(),
            npm_address: None,
            fee_tier_bps: 200,
            token0: PositionTokenInfo { address: NATIVE_ETH.to_string(), symbol: "ETH".to_string(), decimals: 18 },
            token1: PositionTokenInfo { address: NOCK_BASE.to_string(), symbol: "NOCK".to_string(), decimals: 16 },
            liquidity: 8293339594552439540,
            sqrt_price_x96: U256::from_hex_be("0x21baa0a039c91a6726d2879fc9").unwrap(),
            tick: 70370,
            tick_lower: 65200,
            tick_upper: 71200,
            amount0: 0.0,
            amount1: 0.0,
            fee0: 0.0,
            fee1: 0.0,
            in_range: true,
            is_price_source: false,
        }
    }

    fn fixture_position_v4_usdc() -> RawPosition {
        RawPosition {
            position_id: "uniswap-v4:base:3163431".to_string(),
            network: "base".to_string(),
            protocol: "uniswap-v4".to_string(),
            wallet: "0xee114cc97ec5adeb34ba5645e315b497d8487a45".to_string(),
            nft_id: 3163431,
            pool_id: "0x91a9ef0a8dd231b99fa53a2a145a9e8bca7897e0d85852107abb997916f9f99c".to_string(),
            npm_address: None,
            fee_tier_bps: 75,
            token0: PositionTokenInfo { address: USDC_BASE.to_string(), symbol: "USDC".to_string(), decimals: 6 },
            token1: PositionTokenInfo { address: NOCK_BASE.to_string(), symbol: "NOCK".to_string(), decimals: 16 },
            liquidity: 320235280612027,
            sqrt_price_x96: U256::from_hex_be("0x0a9598f7fe4938c0312958557374bf").unwrap(),
            tick: 269008,
            tick_lower: 267825,
            tick_upper: 271050,
            amount0: 0.0,
            amount1: 0.0,
            fee0: 0.0,
            fee1: 0.0,
            in_range: true,
            is_price_source: true,
        }
    }

    fn fixture_position_v3_weth_nock() -> RawPosition {
        RawPosition {
            position_id: "uniswap-v3:base:6336662".to_string(),
            network: "base".to_string(),
            protocol: "uniswap-v3".to_string(),
            wallet: "0xee114cc97ec5adeb34ba5645e315b497d8487a45".to_string(),
            nft_id: 6336662,
            pool_id: "0x71a4a6f6ea5c26ce486b7ed0d8723a897ef5ad9a".to_string(),
            npm_address: Some(UNISWAP_V3_NPM_BASE.to_string()),
            fee_tier_bps: 100,
            token0: PositionTokenInfo { address: WETH_BASE.to_string(), symbol: "WETH".to_string(), decimals: 18 },
            token1: PositionTokenInfo { address: NOCK_BASE.to_string(), symbol: "NOCK".to_string(), decimals: 16 },
            liquidity: 1305339058220032430,
            sqrt_price_x96: U256::from_hex_be("0x21d595ed92e90d8945a5ef7766").unwrap(),
            tick: 70433,
            tick_lower: 70400,
            tick_upper: 74800,
            amount0: 0.0,
            amount1: 0.0,
            fee0: 0.0,
            fee1: 0.0079878029, // v3 uncollected NOCK fees at discovery ($0.000166)
            in_range: true,
            is_price_source: false,
        }
    }

    /// Position D — USDC/NOCK, Uniswap v4, 3.00% fee, wide raft (change request:
    /// largest holding ≈ $207). Chain reads 2026-10-24 02:37 UTC: liquidity =
    /// PositionLiquidity(3163450) — the operator compounded post-mint (mint tx
    /// 0x6cdabada… +48.3e12, add +143.7e12, +1.2e12, partial remove −7.7e12);
    /// sqrtPrice/tick via StateView getSlot0; feeGrowthInside (0,0) — fees zero.
    fn fixture_position_v4_usdc_wide() -> RawPosition {
        RawPosition {
            position_id: "uniswap-v4:base:3163450".to_string(),
            network: "base".to_string(),
            protocol: "uniswap-v4".to_string(),
            wallet: "0xee114cc97ec5adeb34ba5645e315b497d8487a45".to_string(),
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
            is_price_source: false,
        }
    }

    /// The four-position portfolio fixture with amounts derived exactly like the
    /// live pipeline. Magnitude anchors from the operator screenshots (± drift):
    /// B ≈ $71, C ≈ $18–47, A ≈ $156 (compounded +75% since the $89.48 screenshot),
    /// D ≈ $208 (largest holding; minted 2026-10-24 morning).
    pub(crate) fn four_position_fixture() -> RawOnChainData {
        let mut a = fixture_position_v4_nock_eth();
        let mut b = fixture_position_v4_usdc();
        let mut c = fixture_position_v3_weth_nock();
        let mut d = fixture_position_v4_usdc_wide();
        for p in [&mut a, &mut b, &mut c, &mut d] {
            let (a0, a1) = concentrated_amounts_raw(p.liquidity, p.sqrt_price_x96, p.tick_lower, p.tick_upper);
            p.amount0 = a0 / 10f64.powi(p.token0.decimals);
            p.amount1 = a1 / 10f64.powi(p.token1.decimals);
        }
        let spot = nock_spot_from_positions(&[a.clone(), b.clone(), c.clone(), d.clone()]).expect("fixture price source");
        let mut raw = RawOnChainData::default();
        raw.positions = vec![a, b, c, d];
        raw.spot_price_nock_usd = spot;
        raw.live_eth_price_usd = 2373.14; // canonical v3 WETH/USDC pool at discovery
        raw.live_gas_cost_usd = 0.005;
        raw.wallet_balances = vec![
            WalletBalance {
                wallet_address: "0xee114cc97ec5adeb34ba5645e315b497d8487a45".to_string(),
                token_symbol: "NOCK".to_string(),
                token_address: NOCK_BASE.to_string(),
                amount: "110648.0751".to_string(),
                amount_usd: "2300.0".to_string(),
                updated_at: "now".to_string(),
            },
            WalletBalance {
                wallet_address: "0xee114cc97ec5adeb34ba5645e315b497d8487a45".to_string(),
                token_symbol: "USDC".to_string(),
                token_address: USDC_BASE.to_string(),
                amount: "134.4148".to_string(),
                amount_usd: "134.41".to_string(),
                updated_at: "now".to_string(),
            },
        ];
        raw
    }

    /// Raw fixture above the default 100k floor so the Overextension branch is
    /// reachable in advice tests (total NOCK ≈ 125,944, spot forced into the dip zone).
    fn raw_above_floor_fixture() -> RawOnChainData {
        let mut raw = four_position_fixture();
        raw.spot_price_nock_usd = 0.015; // dip zone (< $0.020 accumulation boundary)
        raw
    }

    #[test]
    fn test_parse_geckoterminal_ohlcv_normalizes_newest_first() {
        // Recorded fixture from the live endpoint (2026-10-12): ohlcv_list entries
        // are [open_time_unix_secs, open, high, low, close, volume], newest first.
        let body = json!({
            "data": {
                "id": "x",
                "type": "ohlcv_request_response",
                "attributes": {
                    "ohlcv_list": [
                        [1791763200, 0.0246178045182968, 0.0269877692445729, 0.022041369483387, 0.0239357572998183, 278995.4950266538],
                        [1791676800, 0.0268425121191988, 0.0271935570968404, 0.023807436927751, 0.0246178045182968, 263160.2698464311],
                        [1791590400, 0.0234294762339549, 0.0283541273531182, 0.0225412081009429, 0.0268425121191988],
                        ["not-a-number", 1, 2, 3, 4],
                        [1791504000, 1, 2, 3]
                    ]
                }
            }
        });
        let candles = parse_geckoterminal_ohlcv(&body);
        assert_eq!(candles.len(), 3, "malformed entries must be skipped");
        // Newest-first input normalized to ascending close order
        assert!(candles[0].timestamp < candles[1].timestamp);
        assert!(candles[1].timestamp < candles[2].timestamp);
        assert_eq!(
            candles[2].timestamp.format("%Y-%m-%d").to_string(),
            "2026-10-12"
        );
        assert!((candles[2].close - 0.0239357572998183).abs() < 1e-12);
        assert_eq!(candles[0].volume, None, "5-element entry has no volume");
        assert!(candles[2].volume.is_some());
    }

    #[test]
    fn test_parse_geckoterminal_ohlcv_rejects_garbage() {
        assert!(parse_geckoterminal_ohlcv(&json!({})).is_empty());
        assert!(parse_geckoterminal_ohlcv(&json!({ "data": {} })).is_empty());
        assert!(parse_geckoterminal_ohlcv(&json!({ "data": { "attributes": { "ohlcv_list": [] } } })).is_empty());
    }

    #[test]
    fn test_parse_dexscreener_price() {
        let body = json!({
            "schemaVersion": "1.0.0",
            "pairs": [
                { "chainId": "base", "dexId": "aerodrome", "priceUsd": "0.02396" }
            ]
        });
        let price = parse_dexscreener_price(&body).unwrap();
        assert!((price - 0.02396).abs() < 1e-12);
        // Missing/malformed shapes degrade to None, never fabricate a price
        assert!(parse_dexscreener_price(&json!({})).is_none());
        assert!(parse_dexscreener_price(&json!({ "pairs": [] })).is_none());
        assert!(parse_dexscreener_price(&json!({ "pairs": [{ "priceUsd": "abc" }] })).is_none());
        assert!(parse_dexscreener_price(&json!({ "pairs": [{ "priceUsd": "0" }] })).is_none());
    }

    #[test]
    fn test_plan_backfill_cases() {
        use chrono::TimeZone;
        let at = |d: i64, m: i64| Utc.with_ymd_and_hms(2026, 10, 12, 0, 0, 0).unwrap()
            + chrono::Duration::days(d)
            + chrono::Duration::minutes(m);
        let now = at(0, 0);

        // Empty table (first boot): full backfill + fallback seeding allowed
        let plan = plan_backfill(None, None, now, 15, 14);
        assert!(plan.backfill_hourly && plan.backfill_daily && plan.seed_fallback_point);

        // Fresh deep history, last point 5 minutes old: nothing to do
        let plan = plan_backfill(Some(at(0, -5)), Some(at(-30, 0)), now, 15, 14);
        assert_eq!(plan, BackfillPlan::default());

        // Laptop slept for 3 hours: hourly gap fill (daily depth still sufficient)
        let plan = plan_backfill(Some(at(0, -180)), Some(at(-30, 0)), now, 15, 14);
        assert!(plan.backfill_hourly && plan.seed_fallback_point && !plan.backfill_daily);

        // Fresh but shallow history: extend daily depth, no gap fill
        let plan = plan_backfill(Some(at(0, -5)), Some(at(-7, 0)), now, 15, 14);
        assert!(!plan.backfill_hourly && !plan.seed_fallback_point && plan.backfill_daily);
    }

    #[test]
    fn test_daily_closes_from_points() {
        use chrono::TimeZone;
        let at = |d: u32, h: u32| Utc.with_ymd_and_hms(2026, 10, d, h, 0, 0).unwrap();
        let points = vec![
            (at(1, 0), Some(0.10), 0.10), // day-1 daily candle close
            (at(1, 5), None, 0.105),      // day-1 spot: ignored, candle is authoritative
            (at(2, 5), None, 0.20),       // day-2 spots only
            (at(2, 10), None, 0.21),      // ...last spot of the day wins
            (at(3, 0), Some(0.30), 0.30), // day-3 candle only
        ];
        let closes = daily_closes_from_points(&points);
        assert_eq!(closes.len(), 3);
        assert!((closes[0] - 0.10).abs() < 1e-12);
        assert!((closes[1] - 0.21).abs() < 1e-12);
        assert!((closes[2] - 0.30).abs() < 1e-12);
        assert!(daily_closes_from_points(&[]).is_empty());
    }

    #[test]
    fn test_daily_rsi_invariant_to_provenance_mix_and_gaps() {
        use chrono::TimeZone;
        let day0 = Utc.with_ymd_and_hms(2026, 9, 1, 0, 0, 0).unwrap();
        // 16 daily closes, monotonic gains; day 5 carries spots instead of a candle
        // (backfilled vs contemporaneous mix) and day 7 is missing entirely (gap).
        let mut points = Vec::new();
        let mut expected = Vec::new();
        for i in 0..16u32 {
            let close = 100.0 + i as f64;
            if i == 5 {
                points.push((day0 + chrono::Duration::days(i as i64) + chrono::Duration::hours(5), None, close));
                points.push((day0 + chrono::Duration::days(i as i64) + chrono::Duration::hours(10), None, close + 0.001));
                expected.push(close + 0.001); // last spot of the day
            } else if i == 7 {
                continue; // provider gap: no data at all for this day
            } else {
                points.push((day0 + chrono::Duration::days(i as i64), Some(close), close));
                expected.push(close);
            }
        }
        let closes = daily_closes_from_points(&points);
        assert_eq!(closes, expected);
        let rsi_actual = compute_rsi_series(&closes, RSI_PERIOD).into_iter().last().flatten().unwrap();
        let rsi_expected = compute_rsi_series(&expected, RSI_PERIOD).into_iter().last().flatten().unwrap();
        assert!((rsi_actual - rsi_expected).abs() < 1e-9);
        assert!(rsi_actual > 99.0, "monotonic gains must pin daily RSI high");
    }

    #[test]
    fn test_overextension_cooldown_ignores_micro_pulse() {
        // The v0.2.x bug this feature fixes: a steep 4-hour intraday climb pins
        // the 5-minute RSI(14) near 100 while the daily trend stays healthy.
        // Deployment must NOT be suppressed — the cooldown keys on the daily RSI.
        let micro_closes: Vec<f64> = (1..=30).map(|i| 100.0 + i as f64).collect();
        let rsi_5m = compute_rsi_series(&micro_closes, RSI_PERIOD).into_iter().last().flatten().unwrap();
        assert!(rsi_5m >= RSI_OVERBOUGHT, "fixture must reproduce the micro pulse: {rsi_5m}");
        let macro_closes: Vec<f64> = vec![
            100.0, 99.5, 101.0, 100.5, 99.0, 100.0, 101.5, 100.0,
            99.5, 100.5, 101.0, 100.0, 99.0, 100.5, 101.0,
        ];
        let rsi_1d = compute_rsi_series(&macro_closes, RSI_PERIOD).into_iter().last().flatten().unwrap();
        assert!(rsi_1d < RSI_OVERBOUGHT, "healthy daily trend: {rsi_1d}");

        let regime = MarketRegime {
            rsi_5m: Some(rsi_5m),
            rsi_1d: Some(rsi_1d),
            overextended: rsi_1d >= RSI_OVERBOUGHT,
        };
        assert!(!regime.overextended);
        let metrics = compute_derived_metrics(
            &raw_above_floor_fixture(),
            0.0,
            &OperatorProfile::default(),
            StructuralChannels { channels: vec![] },
            &[],
            &LedgerAggregate::default(),
            &regime,
        );
        assert!(
            !metrics.global_status.shot_advice_action.starts_with("OVEREXTENDED"),
            "micro pulse must not trigger the cooldown: {}",
            metrics.global_status.shot_advice_action
        );
        assert_eq!(metrics.global_status.regime, "normal");
        assert!(metrics.global_status.rsi_14.is_some());
        assert!(metrics.global_status.rsi_14_1d.is_some());
    }

    #[test]
    fn test_overextension_cooldown_fires_on_daily_rsi() {
        // Monotonic multi-day gains pin the daily RSI(14) near 100: the cooldown
        // fires and cites the daily series explicitly.
        let macro_closes: Vec<f64> = (1..=20).map(|i| 100.0 + i as f64).collect();
        let rsi_1d = compute_rsi_series(&macro_closes, RSI_PERIOD).into_iter().last().flatten().unwrap();
        assert!(rsi_1d >= RSI_OVERBOUGHT);
        let regime = MarketRegime {
            rsi_5m: Some(40.0), // micro is calm; only the daily series decides
            rsi_1d: Some(rsi_1d),
            overextended: rsi_1d >= RSI_OVERBOUGHT,
        };
        let metrics = compute_derived_metrics(
            &raw_above_floor_fixture(),
            0.0,
            &OperatorProfile::default(),
            StructuralChannels { channels: vec![] },
            &[],
            &LedgerAggregate::default(),
            &regime,
        );
        assert_eq!(metrics.global_status.shot_advice_action, "OVEREXTENDED (Daily RSI ≥ 70, No Buy Advised)");
        assert!(metrics.global_status.shot_advice_details.contains("daily RSI(14)"));
        assert_eq!(metrics.global_status.regime, "overextended");
        assert_eq!(metrics.global_status.rsi_14.as_deref(), Some("40.0"));
    }

    // ============================================================
    // From zero (v0.5.1): a newcomer's own numbers, and nothing else
    // ============================================================

    /// Text NockNock must never produce for a newcomer (or anyone): sized lump sums,
    /// someone else's goal, alarm words, and uncomputed P/L or yield claims.
    const NEVER_IN_ADVICE: [&str; 5] = ["Deploy $", "120,000", "CRITICAL", "realized loss", "generating yield"];

    fn assert_no_invented_text(details: &str) {
        for banned in NEVER_IN_ADVICE {
            assert!(!details.contains(banned), "{banned:?} in: {details}");
        }
    }

    /// A newcomer's wallet: no LP, gas read as zero, every amount a test input.
    pub(crate) fn from_zero_raw(usdc: f64, nock: f64) -> RawOnChainData {
        let mut raw = RawOnChainData::default();
        raw.spot_price_nock_usd = 0.01153;
        raw.live_eth_price_usd = 3000.0;
        raw.live_gas_cost_usd = 0.004;
        raw.wallet_eth = Some(0.0);
        raw.wallet_weth = Some(0.0);
        let row = |sym: &str, addr: &str, amount: f64| WalletBalance {
            wallet_address: "0x00000000000000000000000000000000000000aa".to_string(),
            token_symbol: sym.to_string(),
            token_address: addr.to_string(),
            amount: format!("{:.4}", amount),
            amount_usd: "0".to_string(),
            updated_at: "now".to_string(),
        };
        raw.wallet_balances = vec![row("NOCK", NOCK_BASE, nock), row("USDC", USDC_BASE, usdc)];
        raw
    }

    /// The newcomer's own plan: $100 over 5 tranches, 24 h apart.
    fn own_plan() -> AccumulationPlanConfig {
        AccumulationPlanConfig {
            id: "my-first-plan".to_string(),
            budget_usdc: 100.0,
            tranches: 5,
            interval_hours: 24.0,
            starts_at: None,
            venue: Some("Aero".to_string()),
            max_price_usd: None,
            pause_when_overextended: true,
            respect_cash_target: true,
            cash_reserve_usdc: None,
        }
    }

    fn zero_metrics(raw: &RawOnChainData, profile: &OperatorProfile, ctx: &PlanContext) -> DerivedMetrics {
        compute_derived_metrics_with_plan(
            raw, 0.0, profile, StructuralChannels { channels: vec![] }, &[],
            &LedgerAggregate::default(), &MarketRegime::default(), ctx,
        )
    }

    fn own_plan_due() -> PlanContext {
        PlanContext { cfg: Some(own_plan()), fills: PlanFills::default(), ledger_unavailable: false, now: t("2026-11-15T12:00:00Z") }
    }

    fn own_plan_after_first_fill() -> PlanContext {
        PlanContext {
            cfg: Some(own_plan()),
            fills: PlanFills { count: 1, spent_usdc: 20.0, acquired_nock: 1734.6054, last_at: Some(t("2026-11-15T12:00:00Z")), first_price: Some(0.01153) },
            ledger_unavailable: false,
            now: t("2026-11-15T18:00:00Z"),
        }
    }

    #[test]
    fn test_from_zero_usdc_only_without_plan_never_sizes_a_buy() {
        // R1
        let m = zero_metrics(&from_zero_raw(500.0, 0.0), &OperatorProfile::default(), &PlanContext::none());
        let g = &m.global_status;
        assert_eq!(g.total_portfolio_usd, "500.00");
        assert_eq!(g.total_native_usd, "0.00");
        assert_eq!(g.total_stable_usd, "500.00");
        assert_eq!(g.total_nock_amount, "0");
        assert_eq!(g.nock_floor_target, "0");
        assert_eq!(g.high_water_mark_usd, "500.00");
        assert_eq!(g.shot_advice_action, "BUILD FROM ZERO (No Buy Plan Configured)");
        let d = &g.shot_advice_details;
        assert!(d.contains("$500.00") && d.contains("accumulation_plan") && d.contains("no ETH"), "{d}");
        assert_no_invented_text(d);
        for banned in ["Buy $", "DEFENSIVE LOCK", "Standing bias", "Re-establishing"] {
            assert!(!d.contains(banned), "{banned} in: {d}");
        }
        assert!(m.accumulation_plan.is_none());
        assert_eq!(m.lp_rebuild.status, "no_positions");
        assert!(m.lp_rebuild.suggestions.is_empty() && m.lp_rebuild.excluded.is_empty());
        let w: Vec<i32> = m.portfolio_deviation_matrix.allocations.iter().map(|a| a.actual_weight_bps).collect();
        // The fourth row is gas and the fifth AERO (#93): measured shares with no target.
        assert_eq!(w, vec![0, 0, 10000, 0, 0]);
        assert_eq!(m.portfolio_deviation_matrix.allocations[3].status, "no_target");
        assert_eq!(m.portfolio_deviation_matrix.allocations[4].status, "no_target");
        assert!(m.tick_bracket_hypervisor.brackets.is_empty());
        assert!(m.asset_aggregation_series.positions_nock.is_empty());
        assert!(m.velocity_gas_sandbox.contracts.is_empty());
    }

    #[test]
    fn test_from_zero_empty_wallet_is_an_explicit_empty_state() {
        // R2
        let m = zero_metrics(&from_zero_raw(0.0, 0.0), &OperatorProfile::default(), &PlanContext::none());
        assert_eq!(m.global_status.shot_advice_action, "NOTHING TO TRACK YET (Empty Wallet)");
        let d = &m.global_status.shot_advice_details;
        assert!(d.contains("0x00000000000000000000000000000000000000aa"), "{d}");
        assert!(d.contains("holds no NOCK, no USDC and no LP"));
        assert!(d.contains("accumulation_plan") && d.contains("no ETH"));
        assert!(!d.contains("Maintain posture"));
        assert_no_invented_text(d);
        assert_eq!(m.global_status.total_portfolio_usd, "0.00");
        assert_eq!(m.global_status.high_water_mark_usd, "0.00");
        let w: Vec<i32> = m.portfolio_deviation_matrix.allocations.iter().map(|a| a.actual_weight_bps).collect();
        assert_eq!(w, vec![0, 0, 0, 0, 0]);
        assert_eq!(m.lp_rebuild.status, "no_positions");
        // Gas held: the gas fact is not stated.
        let mut gas = from_zero_raw(0.0, 0.0);
        gas.wallet_eth = Some(0.01);
        let m = zero_metrics(&gas, &OperatorProfile::default(), &PlanContext::none());
        assert!(!m.global_status.shot_advice_details.contains("no ETH"));
        // A gas-only wallet is still empty: the gas counts in the total (0.01 x $3000)
        // but is never something to deploy.
        assert_eq!(m.global_status.shot_advice_action, "NOTHING TO TRACK YET (Empty Wallet)");
        assert_eq!(m.global_status.total_portfolio_usd, "30.00");
        assert_eq!(m.global_status.wallet_gas_usd.as_deref(), Some("30.00"));
        assert!(m.global_status.shot_advice_details.contains("never deployed"));
        let w: Vec<i32> = m.portfolio_deviation_matrix.allocations.iter().map(|a| a.actual_weight_bps).collect();
        assert_eq!(w, vec![0, 0, 0, 10000, 0]);
    }

    #[test]
    fn test_from_zero_users_own_plan_leads_and_uses_only_their_numbers() {
        // R3
        let m = zero_metrics(&from_zero_raw(500.0, 0.0), &OperatorProfile::default(), &own_plan_due());
        assert_eq!(m.global_status.shot_advice_action, "DCA TRANCHE DUE (1 of 5)");
        let d = &m.global_status.shot_advice_details;
        assert!(d.starts_with("Buy $20.00 of NOCK (spot $0.01153; your plan's venue label: Aero)."), "{d}");
        assert!(d.contains("Plan \"my-first-plan\": $0.00 of $100.00 deployed over 0 verified buy(s)."), "{d}");
        assert!(!d.contains("DEFENSIVE LOCK") && !d.contains("Standing bias"));
        assert_no_invented_text(d);
        let ps = m.accumulation_plan.unwrap();
        assert_eq!(ps.status, "tranche_due");
        assert_eq!(ps.tranche_usdc, "20.00");
        assert_eq!(ps.nominal_tranche_usdc, "20.00");
        assert_eq!(ps.budget_usdc, "100.00");
        assert_eq!(ps.binding_constraint, None);
    }

    #[test]
    fn test_from_zero_after_first_fill_waits_on_the_users_interval() {
        // R4
        let m = zero_metrics(&from_zero_raw(480.0, 1734.6054), &OperatorProfile::default(), &own_plan_after_first_fill());
        assert_eq!(m.global_status.shot_advice_action, "DCA ON SCHEDULE (2 of 5 next)");
        assert_eq!(
            m.global_status.shot_advice_details,
            "Nothing to buy yet: the next $20.00 tranche is due at 2026-11-16T12:00:00+00:00. Plan \"my-first-plan\": $20.00 of $100.00 deployed over 1 verified buy(s), average cost $0.011530."
        );
        assert_eq!(m.global_status.total_nock_amount, "1735");
        assert_eq!(m.global_status.total_portfolio_usd, "500.00");
        let ps = m.accumulation_plan.unwrap();
        assert_eq!(ps.tranche_index, 2);
        assert_eq!(ps.fills, 1);
        assert_eq!(ps.avg_price_usd.as_deref(), Some("0.011530"));
        assert_eq!(ps.lump_sum_nock.as_deref(), Some("8673.0269"));
        assert_eq!(ps.remaining_usdc, "80.00");
        assert_eq!(ps.next_due_at.as_deref(), Some("2026-11-16T12:00:00+00:00"));
    }

    #[test]
    fn test_from_zero_users_own_floor_keeps_the_plan_visible_and_notifying() {
        // R5: the user's own 100,000 floor, with their own plan.
        let profile = operator_profile_2026_11();
        let due = zero_metrics(&from_zero_raw(500.0, 0.0), &profile, &own_plan_due());
        assert_eq!(due.global_status.shot_advice_action, "DEFENSIVE LOCK (Floor Recovery) · DCA TRANCHE DUE (1 of 5)");
        let d = &due.global_status.shot_advice_details;
        assert!(d.contains("is below the 100000 floor by 100000 NOCK"), "{d}");
        assert!(d.contains("Your accumulation plan is the recovery path: Buy $20.00 of NOCK (spot $0.01153; your plan's venue label: Aero)"), "{d}");
        assert!(!d.contains("stable reserves are locked"));
        assert_no_invented_text(d);
        assert_eq!(due.global_status.nock_floor_target, "100000");
        assert_eq!(due.global_status.nock_floor_delta, "-100000");
        assert_eq!(due.accumulation_plan.as_ref().unwrap().status, "tranche_due");

        let waiting = zero_metrics(&from_zero_raw(480.0, 1734.6054), &profile, &own_plan_after_first_fill());
        assert_eq!(waiting.global_status.shot_advice_action, "DEFENSIVE LOCK (Floor Recovery) · DCA ON SCHEDULE (2 of 5 next)");
        assert!(waiting.global_status.shot_advice_details.contains("below the 100000 floor by 98265 NOCK"));
        // The tranche transition changes the action, so it notifies.
        assert!(directive_transition_notification(
            Some(&due.global_status.shot_advice_action),
            &waiting.global_status.shot_advice_action,
            &waiting.global_status.shot_advice_details,
            None,
        )
        .is_some());
    }

    #[test]
    fn test_from_zero_users_own_floor_without_plan_points_to_a_plan() {
        // R6
        let m = zero_metrics(&from_zero_raw(500.0, 0.0), &operator_profile_2026_11(), &PlanContext::none());
        assert_eq!(m.global_status.shot_advice_action, "DEFENSIVE LOCK (Floor Recovery)");
        let d = &m.global_status.shot_advice_details;
        assert!(d.contains("below the 100000 floor by 100000 NOCK") && d.contains("accumulation_plan"), "{d}");
        assert!(!d.contains("Buy $"));
        assert_no_invented_text(d);
    }

    #[test]
    fn test_planless_dip_zone_states_facts_only() {
        // R7: the operator's 2026-11-14 wallet with no plan section.
        let raw = liquid_wallet_fixture();
        let m = compute_derived_metrics_with_plan(
            &raw, 0.0, &operator_profile_2026_11(), StructuralChannels { channels: vec![] }, &[],
            &LedgerAggregate::default(), &MarketRegime::default(), &PlanContext::none(),
        );
        assert_eq!(m.global_status.shot_advice_action, "DIP ZONE (No Buy Plan Configured)");
        let d = &m.global_status.shot_advice_details;
        assert!(d.contains("$0.01229") && d.contains("$1053.00") && d.contains("accumulation_plan"), "{d}");
        assert!(d.contains("Liquid dry powder: $929.22 (wallet USDC $1053.00 less the larger of the $123.78 cash target and the $0.00 virtual tax reserve)."), "{d}");
        // The whole text, including the appended LP-bias sentence, invents nothing.
        assert!(d.contains("Standing bias: "), "{d}");
        assert_no_invented_text(d);
    }

    #[test]
    fn test_due_tranche_leads_but_waiting_plan_yields_to_harvest_and_range_exit() {
        // N11: a plan with nothing due no longer hides a due harvest or a range exit.
        let mut raw = four_position_fixture();
        raw.wallet_balances[1].amount = "5000.0".to_string(); // the tranche is fundable
        raw.positions[1].fee0 = 6.0; // fees over the fixed $5.00 threshold
        let profile = OperatorProfile::default();
        let run = |raw: &RawOnChainData, ctx: &PlanContext| {
            compute_derived_metrics_with_plan(raw, 0.0, &profile, StructuralChannels { channels: vec![] }, &[], &LedgerAggregate::default(), &MarketRegime::default(), ctx)
        };
        let due = PlanContext { cfg: Some(plan_cfg()), fills: PlanFills::default(), ledger_unavailable: false, now: t("2026-11-15T12:00:00Z") };
        let m = run(&raw, &due);
        assert_eq!(m.global_status.shot_advice_action, "DCA TRANCHE DUE (1 of 5)");

        let fills = PlanFills { count: 1, spent_usdc: 100.0, acquired_nock: 8000.0, last_at: Some(t("2026-11-15T12:00:00Z")), first_price: Some(0.0125) };
        let waiting = PlanContext { cfg: Some(plan_cfg()), fills, ledger_unavailable: false, now: t("2026-11-15T18:00:00Z") };
        let m = run(&raw, &waiting);
        assert_eq!(m.global_status.shot_advice_action, "BATCH HARVEST SHOT (Execute Claim)");
        assert_eq!(m.accumulation_plan.as_ref().unwrap().status, "waiting");

        // Spot above the price-source range's top: RANGE EXIT leads and reports that
        // range's own value, not the whole LP book, and no invented $5.00 wall.
        let src = raw.positions.iter().find(|p| p.is_price_source).unwrap().clone();
        let top = tick_to_display_price_dec(src.tick_lower, src.token0.decimals, src.token1.decimals);
        raw.spot_price_nock_usd = top * 1.01;
        let m = run(&raw, &waiting);
        assert_eq!(m.global_status.shot_advice_action, "RANGE EXIT FLASHPOINT (Above Range)");
        let d = &m.global_status.shot_advice_details;
        let spot = raw.spot_price_nock_usd;
        let pos_usd = src.amount0 * leg_usd_price(&src.token0, spot, raw.live_eth_price_usd)
            + src.amount1 * leg_usd_price(&src.token1, spot, raw.live_eth_price_usd);
        assert!(d.contains(&format!("That range ({}) holds ${:.2}", position_label(&src), pos_usd)), "{d}");
        let lp_total: f64 = m.asset_aggregation_series.active_protocol_series[0].parse().unwrap();
        let exit_text = d.split(" Standing bias:").next().unwrap();
        assert!(!exit_text.contains(&format!("${:.2}", lp_total)), "the LP book total must not be the range's value: {d}");
        assert!(!d.contains("$5.00") && !d.contains("High Wall"), "{d}");
        // Blocked and paused plans yield too; a due tranche still leads.
        let m = run(&raw, &due);
        assert_eq!(m.global_status.shot_advice_action, "DCA TRANCHE DUE (1 of 5)");
    }

    #[test]
    fn test_sweep_to_stable_states_facts_and_instructs_no_sale() {
        // N13: the overextension arm states the daily RSI and the plan's pause.
        let raw = raw_above_floor_fixture();
        let hot = MarketRegime { rsi_5m: None, rsi_1d: Some(82.0), overextended: true };
        let ctx = PlanContext { cfg: Some(plan_cfg()), fills: PlanFills::default(), ledger_unavailable: false, now: t("2026-11-15T12:00:00Z") };
        let m = compute_derived_metrics_with_plan(&raw, 0.0, &OperatorProfile::default(), StructuralChannels { channels: vec![] }, &[], &LedgerAggregate::default(), &hot, &ctx);
        // The label is a measurement, not an instruction to sell (N13, G11).
        assert_eq!(m.global_status.shot_advice_action, "OVEREXTENDED (Daily RSI ≥ 70, No Buy Advised)");
        assert!(!m.global_status.shot_advice_action.contains("SWEEP"));
        let d = &m.global_status.shot_advice_details;
        assert!(d.starts_with("NOCK spot ($0.01500) is overextended: daily RSI(14) 82 is at or above 70. Your accumulation plan pauses buys until it cools."), "{d}");
        for banned in ["sweep to the stable stack", "suppress deployment", "Buying tops", "sell"] {
            assert!(!d.contains(banned), "{banned} in: {d}");
        }
        assert_eq!(m.accumulation_plan.unwrap().status, "paused");
        // Without a plan there is no pause to mention.
        let m = compute_derived_metrics(&raw, 0.0, &OperatorProfile::default(), StructuralChannels { channels: vec![] }, &[], &LedgerAggregate::default(), &hot);
        assert!(!m.global_status.shot_advice_details.contains("accumulation plan"));
    }

    #[test]
    fn test_batch_harvest_text_follows_the_configured_tax_ratio() {
        let mut raw = four_position_fixture();
        raw.positions[1].fee0 = 6.0;
        // No ratio configured: nothing is booked to a reserve, and no percentage is named.
        let m = compute_derived_metrics(&raw, 0.0, &OperatorProfile::default(), StructuralChannels { channels: vec![] }, &[], &LedgerAggregate::default(), &MarketRegime::default());
        let d = &m.global_status.shot_advice_details;
        assert!(d.contains("no tax_reserve_ratio is configured, so none of it is booked to a virtual tax reserve"), "{d}");
        assert!(!d.contains('%'), "{d}");
        let mut profile = operator_profile_no_floor();
        let m = compute_derived_metrics(&raw, 0.0, &profile, StructuralChannels { channels: vec![] }, &[], &LedgerAggregate::default(), &MarketRegime::default());
        let d = &m.global_status.shot_advice_details;
        assert!(d.contains("books 40% of the USDC, NOCK and WETH/ETH received to the virtual tax reserve and 60% to growth"), "{d}");
        assert!(d.contains("valued at the claim's block") && !d.contains("not booked"), "{d}");
        profile.tax_reserve_ratio = 0.25;
        let m = compute_derived_metrics(&raw, 0.0, &profile, StructuralChannels { channels: vec![] }, &[], &LedgerAggregate::default(), &MarketRegime::default());
        let d = &m.global_status.shot_advice_details;
        assert!(d.contains("books 25% of the USDC, NOCK and WETH/ETH received to the virtual tax reserve and 75% to growth"), "{d}");
        assert!(!d.contains("60/40") && !d.contains("40%") && !d.contains("Tax Escrow"), "{d}");
    }

    #[test]
    fn test_mcp_formatters_from_zero() {
        // R14 (worker side): the floor comes from the snapshot, missing fields say so.
        let data = json!({"derived_metrics":{"global_status":{"total_portfolio_usd":"500.00","total_nock_amount":"0","nock_floor_delta":"0","nock_floor_target":"0","total_usdc_amount":"500.00","high_water_mark_usd":"500.00","shot_advice_action":"BUILD FROM ZERO (No Buy Plan Configured)","total_fees_claimable_usd":"0.00","gas_to_fee_delta_usd":"0.00"},"accumulation_plan":null,"lp_rebuild":{"status":"no_positions","actual_usd":"0.00","target_usd":"0.00","gap_usd":"0.00","suggestions":[],"excluded":[]}},"contract_fees":[]});
        let state = format_portfolio_state_text(&data);
        assert!(state.contains("Sovereign NOCK Stack: 0 NOCK (no floor configured)"), "{state}");
        assert!(!state.contains("100k"));
        let mut below = data.clone();
        below["derived_metrics"]["global_status"]["nock_floor_target"] = json!("100000");
        below["derived_metrics"]["global_status"]["nock_floor_delta"] = json!("-100000");
        let text = format_portfolio_state_text(&below);
        assert!(text.contains("100000 below the 100000 NOCK floor") && !text.contains("above"), "{text}");
        let plan = format_accumulation_plan_text(&data);
        assert!(plan.contains("No accumulation_plan is configured") && plan.contains("No NOCK LP positions are tracked"), "{plan}");
        assert!(!plan.contains("Rebuilding ranges"));
        let empty = format_shot_advice_text(&json!({}));
        assert!(empty.contains("Recommended Action: unavailable"), "{empty}");
        assert!(!empty.contains("HOLD") && !empty.contains("Accreting") && !empty.contains("0.00"));
        let empty_state = format_portfolio_state_text(&json!({}));
        assert!(empty_state.contains("floor unavailable") && empty_state.contains("Tactical Directive: unavailable"));
        assert!(!empty_state.contains("HOLD") && !empty_state.contains("0.00"));
    }

    // --- DB-gated integration tests (gracefully skipped without DATABASE_URL) ---

    async fn integration_pool() -> Option<PgPool> {
        let Ok(url) = std::env::var("DATABASE_URL") else {
            eprintln!("DATABASE_URL not set; skipping DB integration test");
            return None;
        };
        match PgPool::connect(&url).await {
            Ok(pool) => Some(pool),
            Err(e) => {
                eprintln!("Postgres unreachable ({e}); skipping DB integration test");
                None
            }
        }
    }

    async fn ensure_market_price_history_table(pool: &PgPool) {
        let ddl = "CREATE TABLE IF NOT EXISTS market_price_history (
                timestamp      TIMESTAMPTZ NOT NULL PRIMARY KEY,
                price_usd      FLOAT8 NOT NULL,
                open_usd       FLOAT8,
                high_usd       FLOAT8,
                low_usd        FLOAT8,
                close_usd      FLOAT8,
                volume_usd     FLOAT8,
                source         VARCHAR NOT NULL,
                is_backfilled  BOOLEAN NOT NULL DEFAULT false,
                backfilled_at  TIMESTAMPTZ
            )";
        if let Err(first_err) = sqlx::query(ddl).execute(pool).await {
            // Parallel test tasks racing CREATE TABLE IF NOT EXISTS can still lose
            // the pg_type rowtype race inside Postgres; a single retry settles it.
            if sqlx::query(ddl).execute(pool).await.is_err() {
                panic!("create market_price_history: {first_err}");
            }
        }
    }

    #[tokio::test]
    async fn test_candle_upsert_is_idempotent() {
        let Some(pool) = integration_pool().await else { return };
        ensure_market_price_history_table(&pool).await;
        let open = Utc::now();
        let candles = vec![PriceCandle {
            timestamp: open,
            open: 1.0,
            high: 2.0,
            low: 0.5,
            close: 1.5,
            volume: Some(9.0),
        }];
        let now = open + chrono::Duration::hours(2); // candle fully completed
        let first = upsert_price_candles(&pool, &candles, PRICE_SOURCE_GECKOTERMINAL, chrono::Duration::hours(1), now).await.unwrap();
        let rerun = upsert_price_candles(&pool, &candles, PRICE_SOURCE_GECKOTERMINAL, chrono::Duration::hours(1), now).await.unwrap();
        assert_eq!(first, 1, "first insert adds one row");
        assert_eq!(rerun, 0, "rerun must be a no-op");
        let count: i64 = sqlx::query("SELECT COUNT(*) FROM market_price_history WHERE timestamp = $1")
            .bind(open)
            .fetch_one(&pool)
            .await
            .unwrap()
            .try_get(0)
            .unwrap();
        assert_eq!(count, 1);
        // In-progress candle (open < one period ago) must be skipped, not frozen
        let partial = upsert_price_candles(
            &pool,
            &[PriceCandle { timestamp: now, open: 1.0, high: 1.0, low: 1.0, close: 1.0, volume: None }],
            PRICE_SOURCE_GECKOTERMINAL,
            chrono::Duration::hours(1),
            now,
        )
        .await
        .unwrap();
        assert_eq!(partial, 0, "in-progress candle must not be stored");
        sqlx::query("DELETE FROM market_price_history WHERE timestamp = $1")
            .bind(open)
            .execute(&pool)
            .await
            .unwrap();
    }

    // ---- Review findings on the v0.5.1 engine commit ----

    #[test]
    fn test_regime_unknown_without_daily_rsi() {
        // R16: no daily RSI is no regime reading, never "normal".
        let raw = raw_above_floor_fixture();
        let run = |r: &MarketRegime| compute_derived_metrics(&raw, 0.0, &OperatorProfile::default(), StructuralChannels { channels: vec![] }, &[], &LedgerAggregate::default(), r);
        assert_eq!(run(&MarketRegime::default()).global_status.regime, "unknown");
        assert_eq!(run(&MarketRegime { rsi_5m: None, rsi_1d: Some(34.2), overextended: false }).global_status.regime, "normal");
        assert_eq!(run(&MarketRegime { rsi_5m: None, rsi_1d: Some(82.0), overextended: true }).global_status.regime, "overextended");
    }

    #[test]
    fn test_degenerate_math_edges() {
        // R17: a flat series has no RSI (not 100), and a zero sqrtPrice is no spot.
        assert!(compute_rsi_series(&[0.0115; 20], 14).iter().all(|x| x.is_none()));
        let mut src = four_position_fixture().positions.into_iter().find(|p| p.is_price_source).unwrap();
        assert!(nock_spot_from_positions(std::slice::from_ref(&src)).is_some());
        src.sqrt_price_x96 = U256(0, 0);
        assert_eq!(nock_spot_from_positions(std::slice::from_ref(&src)), None);
        // Without a daily RSI the engine never reports an overextension.
        let m = zero_metrics(&from_zero_raw(500.0, 1000.0), &OperatorProfile::default(), &PlanContext::none());
        assert!(!m.global_status.shot_advice_action.starts_with("OVEREXTENDED"));
    }

    #[test]
    fn test_unset_weights_and_ratio_size_nothing() {
        // A user with NOCK, USDC and a tracked range but no weights or tax ratio: no cash
        // target or reserve is named, the LP target is no_target, and no bucket is
        // labelled over- or under-funded against numbers they never set.
        let mut raw = liquid_wallet_fixture();
        raw.spot_price_nock_usd = 0.025; // above the dip line: HOLD
        let m = compute_derived_metrics(&raw, 0.0, &OperatorProfile::default(), StructuralChannels { channels: vec![] }, &[], &LedgerAggregate::default(), &MarketRegime::default());
        let d = &m.global_status.shot_advice_details;
        assert!(m.global_status.shot_advice_action.starts_with("HOLD & ACCRETE"), "{}", m.global_status.shot_advice_action);
        assert!(d.contains("Liquid dry powder: $1053.00 (wallet USDC; no cash target or tax reserve ratio is configured)."), "{d}");
        for banned in ["less the", "cash target)", "virtual tax reserve", "Standing bias", "Re-establishing"] {
            assert!(!d.contains(banned), "{banned} in: {d}");
        }
        assert_eq!(m.lp_rebuild.status, "no_target");
        assert!(m.lp_rebuild.suggestions.is_empty());
        assert!(m.portfolio_deviation_matrix.allocations.iter().all(|a| a.status == "no_target" && a.target_weight_bps == 0));
        assert_eq!(m.sovereign_ledger.tax_reserve_usd, "0.00");
        // Set only a cash weight: only the cash target is named.
        let mut cash_only = OperatorProfile::default();
        cash_only.target_weights_bps.cash = 500;
        let m = compute_derived_metrics(&raw, 0.0, &cash_only, StructuralChannels { channels: vec![] }, &[], &LedgerAggregate::default(), &MarketRegime::default());
        let d = &m.global_status.shot_advice_details;
        assert!(d.contains("cash target).") && !d.contains("virtual tax reserve"), "{d}");
        assert_eq!(m.portfolio_deviation_matrix.allocations[2].target_weight_bps, 500);
        assert_ne!(m.portfolio_deviation_matrix.allocations[2].status, "no_target");
    }

    #[test]
    fn test_rebuild_limit_names_no_floor_when_none_is_set() {
        let raw = rebuild_fixture();
        let demand: f64 = lp_rebuild_suggestions(&raw, 300.0, roomy_funds()).0.iter().map(|sg| {
            (if sg.token0_symbol == "NOCK" { sg.amount0.parse::<f64>().unwrap() } else { 0.0 })
                + (if sg.token1_symbol == "NOCK" { sg.amount1.parse::<f64>().unwrap() } else { 0.0 })
        }).sum();
        let tight = RebuildFunds { nock: demand / 2.0, floor_tolerance_nock: 0.0, floor_set: false, usdc: 10_000.0 };
        let (s, _) = lp_rebuild_suggestions(&raw, 300.0, tight);
        let limited: Vec<&String> = s.iter().filter_map(|sg| sg.limited_by.as_ref()).collect();
        assert!(!limited.is_empty());
        for l in limited {
            assert!(l.contains("NOCK is in the wallet") && !l.contains("floor"), "{l}");
        }
    }

    #[test]
    fn test_defensive_lock_with_a_completed_plan_names_it() {
        // Floor set, plan complete, NOCK below the floor: the plan exists and is done.
        let mut raw = liquid_wallet_fixture();
        raw.wallet_balances[0].amount = "90000.0".to_string();
        let fills = PlanFills { count: 5, spent_usdc: 500.0, acquired_nock: 40500.0, last_at: Some(t("2026-11-15T12:00:00Z")), first_price: Some(0.0125) };
        let ctx = PlanContext { cfg: Some(plan_cfg()), fills, ledger_unavailable: false, now: t("2026-11-20T12:00:00Z") };
        let m = metrics_with_plan(&raw, &ctx, &MarketRegime::default());
        assert!(m.global_status.shot_advice_action.starts_with("DEFENSIVE LOCK (Floor Recovery)"));
        let d = &m.global_status.shot_advice_details;
        assert!(d.contains("Your accumulation plan \"test-plan\" is complete."), "{d}");
        assert!(!d.contains("add an accumulation_plan"), "{d}");
    }

    #[test]
    fn test_harvest_and_range_exit_outrank_planless_no_buy_arms() {
        // No plan, dip zone, $5000 liquid: DIP ZONE would apply, but a due harvest leads.
        let mut raw = four_position_fixture();
        raw.spot_price_nock_usd = 0.015;
        raw.wallet_balances[1].amount = "5000.0".to_string();
        raw.positions[1].fee0 = 6.0;
        let run = |raw: &RawOnChainData, r: &MarketRegime| compute_derived_metrics(raw, 0.0, &operator_profile_no_floor(), StructuralChannels { channels: vec![] }, &[], &LedgerAggregate::default(), r);
        assert_eq!(run(&raw, &MarketRegime::default()).global_status.shot_advice_action, "BATCH HARVEST SHOT (Execute Claim)");
        // BUILD FROM ZERO with fees still due on an emptied range: the harvest leads.
        let mut zero = from_zero_raw(500.0, 0.0);
        let mut p = four_position_fixture().positions[1].clone();
        p.liquidity = 0;
        p.amount0 = 0.0;
        p.amount1 = 0.0;
        p.fee0 = 6.0;
        zero.positions.push(p);
        let m = zero_metrics(&zero, &OperatorProfile::default(), &PlanContext::none());
        assert_eq!(m.global_status.shot_advice_action, "BATCH HARVEST SHOT (Execute Claim)");
        // An overextension has nothing to buy either: the harvest leads and the
        // cooldown fact rides along in the details.
        let hot = MarketRegime { rsi_5m: None, rsi_1d: Some(82.0), overextended: true };
        let m = run(&raw, &hot);
        assert_eq!(m.global_status.shot_advice_action, "BATCH HARVEST SHOT (Execute Claim)");
        assert!(m.global_status.shot_advice_details.contains("daily RSI(14) 82 is at or above 70"), "{}", m.global_status.shot_advice_details);
    }

    #[test]
    fn test_emptied_price_source_range_is_never_a_range_exit() {
        // The 2026-11-14 book: every range emptied, plan waiting, spot above the
        // price-source top. Nothing to exit, so the rebuild keeps the lead.
        let mut raw = rebuild_fixture();
        let src = raw.positions.iter().find(|p| p.is_price_source).unwrap().clone();
        let top = tick_to_display_price_dec(src.tick_lower, src.token0.decimals, src.token1.decimals);
        raw.spot_price_nock_usd = top * 1.01;
        let fills = PlanFills { count: 1, spent_usdc: 100.0, acquired_nock: 8000.0, last_at: Some(t("2026-11-15T12:00:00Z")), first_price: Some(0.0125) };
        let waiting = PlanContext { cfg: Some(plan_cfg()), fills, ledger_unavailable: false, now: t("2026-11-15T18:00:00Z") };
        let m = metrics_with_plan(&raw, &waiting, &MarketRegime::default());
        assert!(!m.global_status.shot_advice_action.contains("RANGE EXIT"), "{}", m.global_status.shot_advice_action);
        assert!(!m.global_status.shot_advice_details.contains("RANGE EXIT"));
    }

    #[test]
    fn test_dip_zone_crossing_still_notifies_under_the_rebuild_bias() {
        // No plan, LP under target with fundable suggestions: above $0.020 the rebuild
        // replaces HOLD; below it, it carries DIP ZONE, so the crossing changes the action.
        let mut raw = rebuild_fixture();
        let none = PlanContext::none();
        let profile = operator_profile_no_floor();
        let run = |raw: &RawOnChainData| compute_derived_metrics_with_plan(raw, 0.0, &profile, StructuralChannels { channels: vec![] }, &[], &LedgerAggregate::default(), &MarketRegime::default(), &none);
        raw.spot_price_nock_usd = 0.025;
        let above = run(&raw);
        raw.spot_price_nock_usd = 0.01229;
        let below = run(&raw);
        assert_eq!(above.global_status.shot_advice_action, "REBUILD LP TRANCHES (No Net Cash Change)");
        assert_eq!(below.global_status.shot_advice_action, "REBUILD LP TRANCHES (No Net Cash Change) · DIP ZONE (No Buy Plan Configured)");
        assert!(directive_transition_notification(
            Some(&above.global_status.shot_advice_action),
            &below.global_status.shot_advice_action,
            &below.global_status.shot_advice_details,
            None
        )
        .is_some());
    }

    #[test]
    fn test_hold_label_survives_a_failed_pool_read_and_never_pushes_between_variants() {
        let mut raw = four_position_fixture();
        for p in &raw.positions {
            raw.pool_liquidity.insert(p.position_id.clone(), "80173935548453779".to_string());
        }
        let run = |raw: &RawOnChainData| compute_derived_metrics(raw, 0.0, &OperatorProfile::default(), StructuralChannels { channels: vec![] }, &[], &LedgerAggregate::default(), &MarketRegime::default());
        let a = run(&raw).global_status.shot_advice_action;
        // One cycle's pool reads fail: the keys are absent, which disproves nothing.
        raw.pool_liquidity.clear();
        let b = run(&raw).global_status.shot_advice_action;
        assert_eq!(a, "HOLD & ACCRETE (Dual Growth Engine Active)");
        assert_eq!(a, b);
        // A real change between HOLD variants (a range drifting out) does not push.
        assert!(directive_transition_notification(Some("HOLD & ACCRETE (Dual Growth Engine Active)"), "HOLD & ACCRETE (No Ranged LP Confirmed In Range)", "d", None).is_none());
        assert!(directive_transition_notification(Some("HOLD & ACCRETE (LP Empty)"), "DCA TRANCHE DUE (1 of 5)", "d", None).is_some());
    }

    #[test]
    fn test_missing_floor_field_is_unavailable_not_100000() {
        let g: GlobalStatus = serde_json::from_value(json!({
            "total_portfolio_usd": "1", "total_native_usd": "1", "total_stable_usd": "0",
            "total_fees_accrued_usd": "0", "total_fees_claimable_usd": "0", "gas_to_fee_delta_usd": "0",
            "high_water_mark_usd": "1", "high_water_mark_date": "x"
        }))
        .unwrap();
        assert_eq!(g.nock_floor_target, "");
        let v = serde_json::to_value(&g).unwrap();
        assert_eq!(floor_clause(Some(&v)), "floor unavailable");
    }

    #[test]
    fn test_shot_advice_mcp_states_the_real_harvest_rule_and_a_signed_net() {
        let data = json!({"derived_metrics":{"global_status":{"shot_advice_action":"HOLD & ACCRETE (LP Empty)","shot_advice_details":"d","total_fees_claimable_usd":"0.50","gas_to_fee_delta_usd":"-0.01"}}});
        let t = format_shot_advice_text(&data);
        assert!(t.contains("fixed $5.00 threshold"), "{t}");
        assert!(!t.contains("2× live gas"), "{t}");
        assert!(t.contains("-$0.01 USD") && !t.contains("+$-"), "{t}");
        // The engine no longer clamps a negative net to 0.
        let mut raw = four_position_fixture();
        raw.live_gas_cost_usd = 10.0;
        let m = compute_derived_metrics(&raw, 0.0, &OperatorProfile::default(), StructuralChannels { channels: vec![] }, &[], &LedgerAggregate::default(), &MarketRegime::default());
        assert!(m.global_status.gas_to_fee_delta_usd.starts_with('-'), "{}", m.global_status.gas_to_fee_delta_usd);
    }

    #[tokio::test]
    async fn test_backfill_never_overwrites_contemporaneous_row() {
        let Some(pool) = integration_pool().await else { return };
        ensure_market_price_history_table(&pool).await;
        let ts = Utc::now();
        insert_price_point(&pool, ts, 1.23, PRICE_SOURCE_RPC, false).await.unwrap();
        insert_price_point(&pool, ts, 9.99, PRICE_SOURCE_GECKOTERMINAL, true).await.unwrap();
        let row = sqlx::query(
            "SELECT price_usd, source, is_backfilled FROM market_price_history WHERE timestamp = $1",
        )
        .bind(ts)
        .fetch_one(&pool)
        .await
        .unwrap();
        let price: f64 = row.try_get("price_usd").unwrap();
        let source: String = row.try_get("source").unwrap();
        let backfilled: bool = row.try_get("is_backfilled").unwrap();
        assert!((price - 1.23).abs() < 1e-12, "contemporaneous observation must win");
        assert_eq!(source, PRICE_SOURCE_RPC);
        assert!(!backfilled);
        sqlx::query("DELETE FROM market_price_history WHERE timestamp = $1")
            .bind(ts)
            .execute(&pool)
            .await
            .unwrap();
    }

    // ---------------- Gas pillar (issue #94) ----------------

    fn gas_metrics(raw: &RawOnChainData) -> DerivedMetrics {
        compute_derived_metrics_with_plan(
            raw, 0.0, &operator_profile_2026_11(), StructuralChannels { channels: vec![] }, &[],
            &LedgerAggregate::default(), &MarketRegime::default(), &PlanContext::none(),
        )
    }

    fn usd_of(v: &str) -> f64 {
        v.parse::<f64>().unwrap()
    }

    #[test]
    fn test_gas_counted_in_total_never_cash_or_deployable() {
        let mut no_gas = four_position_fixture();
        no_gas.wallet_eth = Some(0.0);
        no_gas.wallet_weth = Some(0.0);
        let mut gas = four_position_fixture();
        gas.wallet_eth = Some(0.0125);
        gas.wallet_weth = Some(0.0);
        let (a, b) = (gas_metrics(&no_gas), gas_metrics(&gas));
        let px = 2373.14;
        let (ga, gb) = (&a.global_status, &b.global_status);
        // The total rises by exactly the wallet gas.
        assert!((usd_of(&gb.total_portfolio_usd) - usd_of(&ga.total_portfolio_usd) - 0.0125 * px).abs() <= 0.011);
        assert_eq!(gb.wallet_gas_eth.as_deref(), Some("0.012500"));
        assert_eq!(gb.wallet_gas_usd.as_deref(), Some(format!("{:.2}", 0.0125 * px).as_str()));
        assert_eq!(gb.eth_price_usd.as_deref(), Some("2373.14"));
        // Gas is never cash: the stable pillar and the USDC amount are unchanged.
        assert_eq!(ga.total_stable_usd, gb.total_stable_usd);
        assert_eq!(ga.total_usdc_amount, gb.total_usdc_amount);
        // ...and never dry powder: the liquid dry powder sentence names wallet USDC only.
        for g in [ga, gb] {
            assert!(!g.shot_advice_details.contains("ETH is deployable"));
            if let Some(i) = g.shot_advice_details.find("Liquid dry powder") {
                let rest = &g.shot_advice_details[i..];
                let sentence = &rest[..rest.find(").").map(|j| j + 2).unwrap_or(rest.len())];
                assert!(!sentence.contains("ETH"), "dry powder must be USDC only: {sentence}");
            }
        }
        // The liquid USDC bucket measures wallet USDC only; gas has its own row, no target.
        let row = |m: &DerivedMetrics, name: &str| m.portfolio_deviation_matrix.allocations.iter().find(|r| r.bucket_name.starts_with(name)).cloned().unwrap();
        let gas_row = row(&b, "Gas");
        assert_eq!((gas_row.target_weight_bps, gas_row.status.as_str()), (0, "no_target"));
        assert!(gas_row.actual_weight_bps > 0);
        let usdc_b = row(&b, "Liquid USDC");
        let expect = ((134.4148 / usd_of(&gb.total_portfolio_usd)) * 10000.0) as i32;
        assert_eq!(usdc_b.actual_weight_bps, expect);
        // Weights are shares of the one full total: the four rows cover it.
        let sum: i32 = b.portfolio_deviation_matrix.allocations.iter().map(|r| r.actual_weight_bps).sum();
        assert!((9996..=10000).contains(&sum), "weights sum to {sum}");
    }

    #[test]
    fn test_pillars_partition_total() {
        let mut raw = four_position_fixture();
        raw.wallet_eth = Some(0.0125);
        raw.wallet_weth = Some(0.003);
        let m = gas_metrics(&raw);
        let g = &m.global_status;
        let native = usd_of(&g.total_native_usd);
        let stable = usd_of(&g.total_stable_usd);
        let eth = usd_of(g.total_eth_usd.as_deref().unwrap());
        let total = usd_of(&g.total_portfolio_usd);
        assert!((native + stable + eth - total).abs() <= 0.02, "{native} + {stable} + {eth} != {total}");
        // The ETH pillar is wallet gas plus every ETH/WETH leg in LP.
        let lp_eth: f64 = raw.positions.iter().map(|p| {
            (if is_eth_token(&p.token0) { p.amount0 } else { 0.0 }) + (if is_eth_token(&p.token1) { p.amount1 } else { 0.0 })
        }).sum();
        assert!(lp_eth > 0.0, "fixture must hold ETH in LP");
        let amount = usd_of(g.total_eth_amount.as_deref().unwrap());
        assert!((amount - (0.0155 + lp_eth)).abs() < 1e-6, "{amount}");
        assert!(eth >= usd_of(g.wallet_gas_usd.as_deref().unwrap()));
    }

    #[test]
    fn test_moving_eth_wallet_to_lp_is_total_neutral() {
        let mut before = four_position_fixture();
        before.wallet_eth = Some(0.05);
        before.wallet_weth = Some(0.0);
        let mut after = four_position_fixture();
        after.wallet_eth = Some(0.04);
        after.wallet_weth = Some(0.0);
        after.positions.iter_mut().find(|p| p.nft_id == 6336662).unwrap().amount0 += 0.01;
        let (a, b) = (gas_metrics(&before), gas_metrics(&after));
        assert!((usd_of(&a.global_status.total_portfolio_usd) - usd_of(&b.global_status.total_portfolio_usd)).abs() <= 0.01);
        assert!((usd_of(a.global_status.total_eth_usd.as_deref().unwrap()) - usd_of(b.global_status.total_eth_usd.as_deref().unwrap())).abs() <= 0.01);
        // The high-water mark sees no gain from the move.
        assert!((usd_of(&a.global_status.high_water_mark_usd) - usd_of(&b.global_status.high_water_mark_usd)).abs() <= 0.01);
    }

    #[test]
    fn test_unread_or_unpriced_gas_is_null_never_zero() {
        let mut raw = four_position_fixture();
        raw.wallet_eth = None;
        raw.wallet_weth = Some(0.0);
        let m = gas_metrics(&raw);
        let g = &m.global_status;
        assert!(g.wallet_gas_eth.is_none() && g.wallet_gas_usd.is_none());
        assert!(g.total_eth_amount.is_none() && g.total_eth_usd.is_none());
        let v = serde_json::to_value(g).unwrap();
        for k in ["wallet_gas_eth", "wallet_gas_usd", "total_eth_amount", "total_eth_usd"] {
            assert!(v[k].is_null(), "{k} must be null, got {}", v[k]);
        }
        // Unpriced ETH: the amount is known, its USD value is not.
        let mut raw = four_position_fixture();
        raw.wallet_eth = Some(0.01);
        raw.wallet_weth = Some(0.0);
        raw.live_eth_price_usd = 0.0;
        let g = gas_metrics(&raw).global_status;
        assert_eq!(g.wallet_gas_eth.as_deref(), Some("0.010000"));
        assert!(g.wallet_gas_usd.is_none() && g.total_eth_usd.is_none() && g.eth_price_usd.is_none());
        // Older snapshots without the 2.3.0 fields still parse.
        let old: GlobalStatus = serde_json::from_value(json!({
            "total_portfolio_usd": "1", "total_native_usd": "1", "total_stable_usd": "0",
            "total_fees_accrued_usd": "0", "total_fees_claimable_usd": "0", "gas_to_fee_delta_usd": "0",
            "high_water_mark_usd": "1", "high_water_mark_date": "x"
        })).unwrap();
        assert!(old.total_eth_usd.is_none() && old.eth_price_usd.is_none());
    }

    #[test]
    fn test_gas_pillars_helper() {
        let p = gas_pillars(Some(0.01), Some(0.002), 0.02, 2000.0);
        assert!((p.wallet_gas_eth.unwrap() - 0.012).abs() < 1e-12);
        assert!((p.wallet_gas_usd.unwrap() - 24.0).abs() < 1e-9);
        assert!((p.total_eth_amount.unwrap() - 0.032).abs() < 1e-12);
        assert!((p.total_eth_usd.unwrap() - 64.0).abs() < 1e-9);
        assert_eq!(gas_pillars(Some(0.01), None, 0.02, 2000.0), GasPillars::default());
        assert_eq!(gas_pillars(Some(0.01), Some(0.0), 0.0, f64::NAN).wallet_gas_usd, None);
    }

    #[test]
    fn test_gas_balances_required() {
        assert_eq!(require_gas_balances(Some(0.0125), Some(0.0)).unwrap(), (0.0125, 0.0));
        let e = require_gas_balances(None, Some(0.0)).unwrap_err().to_string();
        assert!(e.contains("ETH") && e.contains("gas"), "{e}");
        let w = require_gas_balances(Some(0.1), None).unwrap_err().to_string();
        assert!(w.contains("WETH"), "{w}");
        assert!(require_gas_balances(None, None).unwrap_err().to_string().contains("ETH and WETH"));
    }

    #[test]
    fn test_every_wallet_token_balance_must_be_present() {
        let ok = json!({"result": {"tokenBalances": [
            {"contractAddress": "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913", "tokenBalance": "0x3b9aca00"},
            {"contractAddress": "0x9B5E262cF9bb04869ab40b19AF91D2dc85761722", "tokenBalance": "0x2386f26fc10000"}
        ]}});
        let (nock, usdc) = parse_wallet_token_balances(&ok).unwrap();
        assert!((usdc - 1000.0).abs() < 1e-9 && (nock - 1.0).abs() < 1e-9, "{nock} {usdc}");
        // A null balance is a failed read, never 0.
        let null_usdc = json!({"result": {"tokenBalances": [
            {"contractAddress": "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913", "tokenBalance": null},
            {"contractAddress": "0x9B5E262cF9bb04869ab40b19AF91D2dc85761722", "tokenBalance": "0x1"}
        ]}});
        assert!(parse_wallet_token_balances(&null_usdc).unwrap_err().to_string().contains("USDC"));
        // A missing token fails too (one present token used to be enough).
        let only_usdc = json!({"result": {"tokenBalances": [
            {"contractAddress": "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913", "tokenBalance": "0x1"}
        ]}});
        assert!(parse_wallet_token_balances(&only_usdc).unwrap_err().to_string().contains("NOCK"));
        let errored = json!({"result": {"tokenBalances": [
            {"contractAddress": "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913", "tokenBalance": "0x1"},
            {"contractAddress": "0x9B5E262cF9bb04869ab40b19AF91D2dc85761722", "tokenBalance": null, "error": "rate limited"}
        ]}});
        assert!(parse_wallet_token_balances(&errored).is_err());
        assert!(parse_wallet_token_balances(&json!({"error": {"code": 429}})).is_err());
        // "0x" is an explicit zero balance, not a failed read.
        let zero = json!({"result": {"tokenBalances": [
            {"contractAddress": "0x833589fcd6edb6e08f4c7c32d4f71b54bda02913", "tokenBalance": "0x"},
            {"contractAddress": "0x9b5e262cf9bb04869ab40b19af91d2dc85761722", "tokenBalance": "0x0"}
        ]}});
        assert_eq!(parse_wallet_token_balances(&zero).unwrap(), (0.0, 0.0));
    }

    #[test]
    fn test_eth_price_pool_needs_in_range_liquidity() {
        assert!(check_eth_price_pool_liquidity("0xd0b5", 1).is_ok());
        let e = check_eth_price_pool_liquidity("0xd0b5", 0).unwrap_err().to_string();
        assert!(e.contains("Stale ETH/USD") && e.contains("0xd0b5"), "{e}");
    }

    #[test]
    fn test_hwm_date_is_the_peak_rows_time_not_now() {
        let peak_at = t("2026-11-14T04:38:29Z");
        let now = "2026-11-14T18:11:04+00:00";
        // Below the stored peak: the date is when the peak was set.
        assert_eq!(resolve_high_water_mark_date(Some((2475.27, peak_at)), 2390.58, now), peak_at.to_rfc3339());
        // A tie keeps the earlier mark (the query orders ties by generated_at ASC).
        assert_eq!(resolve_high_water_mark_date(Some((2475.27, peak_at)), 2475.27, now), peak_at.to_rfc3339());
        // A new mark is set now.
        assert_eq!(resolve_high_water_mark_date(Some((2475.27, peak_at)), 2500.0, now), now);
        // Empty table (a fresh start): the current value is the first mark.
        assert_eq!(resolve_high_water_mark_date(None, 10.0, now), now);
    }

    #[test]
    fn test_snapshot_version_matches_the_spec() {
        let spec = include_str!("../../specs/core/snapshot.yaml");
        assert!(
            spec.contains(&format!("version: {}", SNAPSHOT_SPEC_VERSION))
                || spec.contains(&format!("version: \"{}\"", SNAPSHOT_SPEC_VERSION))
                || spec.contains(&format!("version: '{}'", SNAPSHOT_SPEC_VERSION)),
            "specs/core/snapshot.yaml info.version must be {SNAPSHOT_SPEC_VERSION}"
        );
    }

    #[test]
    fn test_mcp_state_text_reports_gas_and_pillars() {
        let data = json!({"derived_metrics": {"global_status": {
            "total_portfolio_usd": "2413.57", "total_native_usd": "1312.88", "total_stable_usd": "1033.71",
            "total_eth_usd": "66.98", "wallet_gas_eth": "0.012500", "wallet_gas_usd": "22.99",
            "total_nock_amount": "115000", "nock_floor_target": "0", "nock_floor_delta": "115000",
            "total_usdc_amount": "1033.71", "high_water_mark_usd": "2475.27", "shot_advice_action": "HOLD"
        }}});
        let text = format_portfolio_state_text(&data);
        assert!(text.contains("Pillars: NOCK $1312.88 · USDC $1033.71 · ETH $66.98"), "{text}");
        assert!(text.contains("Gas held (wallet ETH+WETH): 0.012500 ETH ($22.99), counted in the total; not cash, not deployable"), "{text}");
        // A pre-2.3.0 snapshot says so, never $0.00.
        let mut old = data.clone();
        for k in ["total_eth_usd", "wallet_gas_eth", "wallet_gas_usd"] {
            old["derived_metrics"]["global_status"].as_object_mut().unwrap().remove(k);
        }
        let text = format_portfolio_state_text(&old);
        assert!(text.contains("ETH not reported (snapshot predates 2.3.0)"), "{text}");
        assert!(text.contains("Gas held (wallet ETH+WETH): not reported (snapshot predates 2.3.0)"), "{text}");
        assert!(!text.contains("$0.00"));
    }

    // ---------------- Golden snapshot (issue #97) ----------------
    // Pins the whole pure path from one cycle's reads to the stored payload, the state
    // hash and the MCP text, byte for byte. Added before the module split; a move-only
    // refactor must leave ingestion-worker/testdata/golden_snapshot.json untouched.
    // Regenerate only for an intended behaviour change:
    //   NOCKNOCK_BLESS_GOLDEN=1 cargo test test_golden_snapshot

    /// The steps fetch_raw_data (its tail) and run_ingestion_cycle apply after the reads.
    #[allow(clippy::too_many_arguments)]
    fn golden_snapshot(
        mut raw: RawOnChainData,
        prev_hwm: f64,
        profile: &OperatorProfile,
        history: &[(DateTime<Utc>, f64)],
        counterfactual: &[(DateTime<Utc>, f64, f64)],
        ledger: &LedgerAggregate,
        regime: &MarketRegime,
        plan: &PlanContext,
    ) -> SnapshotData {
        let (spot, eth) = (raw.spot_price_nock_usd, raw.live_eth_price_usd);
        let mut positions = build_position_snapshots(&raw.positions, spot, eth);
        apply_pool_activity(&mut positions, &raw.pool_liquidity);
        raw.pool_mechanics = build_pool_mechanics(&raw.positions, spot, eth);
        raw.contract_fees = build_contract_fees(&positions, raw.live_gas_cost_usd, &raw.fees_not_lp_claimable);
        let derived_metrics = compute_derived_metrics_with_plan(
            &raw, prev_hwm, profile, compute_structural_channels_sync(history), counterfactual, ledger, regime, plan,
        );
        SnapshotData {
            closed_positions: vec![],
            wallet_balances: raw.wallet_balances,
            positions,
            pool_mechanics: raw.pool_mechanics,
            concentrated_liquidity: raw.concentrated_liquidity,
            contract_fees: raw.contract_fees,
            derived_metrics,
        }
    }

    fn golden_case(data: &SnapshotData) -> serde_json::Value {
        let full = serde_json::to_value(data).unwrap();
        json!({
            "state_hash": compute_state_hash(data),
            "snapshot": canonical_state_json(data),
            "mcp_text": {
                "get_portfolio_state": format_portfolio_state_text(&full),
                "get_shot_advice": format_shot_advice_text(&full),
                "simulate_claim_fees": format_claim_simulation_text(&full),
                "get_accumulation_plan": format_accumulation_plan_text(&full),
            },
        })
    }

    #[test]
    fn test_golden_snapshot_is_byte_identical() {
        let no_plan = PlanContext { cfg: None, fills: PlanFills::default(), ledger_unavailable: false, now: t("2026-11-15T12:00:00Z") };

        // 1. The fixed fixture named by #97, exactly as the screenshot test uses it.
        let four = golden_snapshot(
            four_position_fixture(), 2500.0, &OperatorProfile::default(), &[], &[],
            &LedgerAggregate::default(), &MarketRegime::default(), &no_plan,
        );

        // 2. Every adapter and engine input at once: four Uniswap positions, the Aero
        //    volatile pool with a reverting claimFees(), gas read, pool activity, a
        //    harvest ledger, both RSI reads, history, and a plan with a tranche due.
        let mut raw = rebuild_fixture();
        let mut aero = aero_position(false);
        // 0.01% of the pool instead of the fixture's 1%, so LP stays under its target.
        aero.liquidity /= 100;
        aero.amount0 /= 100.0;
        aero.amount1 /= 100.0;
        raw.pool_liquidity.insert(aero.position_id.clone(), "342542824101829799".to_string());
        raw.fees_not_lp_claimable = vec![aero.position_id.clone()];
        raw.positions.push(aero);
        let history: Vec<(DateTime<Utc>, f64)> = (0..22)
            .map(|i| (t("2026-11-14T00:00:00Z") + chrono::Duration::minutes(5 * i), 2400.0 + (i as f64) * 1.25))
            .collect();
        let counterfactual = [
            (t("2026-11-14T00:00:00Z"), 2400.0, 1400.0),
            (t("2026-11-14T00:05:00Z"), 2401.25, 1400.5),
        ];
        let ledger = LedgerAggregate { total_usd: 12.5, harvest_count: 2, last_at: Some(t("2026-11-13T09:30:00Z")), unpriced_count: 1 };
        let regime = MarketRegime { rsi_5m: Some(48.2), rsi_1d: Some(55.4), overextended: false };
        let plan = PlanContext { cfg: Some(plan_cfg()), fills: PlanFills::default(), ledger_unavailable: false, now: t("2026-11-15T12:00:00Z") };
        let whole = golden_snapshot(raw, 2600.0, &operator_profile_2026_11(), &history, &counterfactual, &ledger, &regime, &plan);

        // 3. A wallet starting from zero: USDC only, no positions, no plan, no rules.
        let zero = golden_snapshot(
            from_zero_raw(500.0, 0.0), 0.0, &OperatorProfile::default(), &[], &[],
            &LedgerAggregate::default(), &MarketRegime::default(), &no_plan,
        );

        let doc = json!({
            "four_position_fixture": golden_case(&four),
            "whole_book_plan_due": golden_case(&whole),
            "from_zero": golden_case(&zero),
        });
        let actual = format!("{}\n", serde_json::to_string_pretty(&doc).unwrap());
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/testdata/golden_snapshot.json");
        if std::env::var("NOCKNOCK_BLESS_GOLDEN").is_ok() {
            std::fs::write(path, &actual).unwrap();
        }
        let expected = std::fs::read_to_string(path).expect("testdata/golden_snapshot.json is checked in");
        assert!(
            actual == expected,
            "snapshot bytes changed; compare {path} with a fresh NOCKNOCK_BLESS_GOLDEN=1 run and `git diff` it"
        );
    }
