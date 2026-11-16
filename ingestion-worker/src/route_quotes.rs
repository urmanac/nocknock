use crate::*;

// ============================================================
// Route quotes for a due tranche (issue #115)
// ============================================================
// When the accumulation plan says a tranche is due, the worker asks each venue's own
// quoter contract what that exact USDC amount returns in NOCK, along a fixed table of
// routes, all at one block. It shows both venues and states which returns more NOCK.
//
// What this never does:
//   - sign, build or send anything. Every call is an `eth_call`. There is no swap
//     calldata, no approval, no slippage or minimum-out value anywhere in this file.
//   - size a buy. The only amount quoted is the tranche the operator's own plan
//     produced (`tranche_usdc` of a `tranche_due` status). No plan, no due tranche or a
//     switched-off feature means no call at all.
//   - estimate. A quote that fails or reverts is "unavailable" with the reason, for
//     that route and that venue only. Nothing is derived from the spot price, and an
//     older quote is never shown as current.
//   - fail the cycle or hide the due tranche.
//   - recommend. "Returns more NOCK by X%" is a comparison of execution routes for a
//     buy the plan already sized. The wallet app's own router may route differently.
//
// Every address and selector below was verified by live read-only calls on Base on
// 2026-11-16 (ONCHAIN_INTEGRATION.md §9); the fixtures under testdata/quotes are those
// calls and their answers.

// ---------------- Verified contracts (Base mainnet) ----------------

/// Aero Metaquoter: quotes a forward-ordered list of Aero pools (concentrated and
/// basic). `factoryRegistry()` returns the Aero FactoryRegistry `0x7b2A2c12…8F3E`.
pub(crate) const AERO_METAQUOTER: &str = "0x1dDfeC5058180C207F55BB77D95AcDd0d8227868";
/// Aero VolatilePoolFactory (`getPool(USDC, NOCK)` is the basic pool below) and the
/// fee module its `feeModule()` returns.
pub(crate) const AERO_VOLATILE_POOL_FACTORY: &str = "0x8E6a9FCd1d638CC760Ffba397605Fe3355dE76D2";
pub(crate) const AERO_VOLATILE_FEE_MODULE: &str = "0x7949DE8d67E4F6b8509Bdbb43FF0503aF1247a7A";
/// Uniswap v3 QuoterV2 on Base: `factory()` is the v3 factory `0x33128a8f…FDfD`.
pub(crate) const UNISWAP_V3_QUOTER_V2: &str = "0x3d4e44Eb1374240CE5F1B871ab261CD16335B76a";
/// Uniswap V4Quoter on Base: `poolManager()` is the v4 PoolManager `0x498581ff…2b2b`.
pub(crate) const UNISWAP_V4_QUOTER: &str = "0x0d5e0f971ed27fbff6c2837bf31316121532048d";

// Aero pools. The concentrated ones resolve from the Aero CLFactory `0x9CB22058…7a76`.
// The two USDT pools have token1 `0xfde4c96c8593536e31f229ea8f37b2ada2699bb2` (USDT on
// Base: 6 decimals, `symbol()` "USDT"). USDT is only ever a hop inside one Metaquoter
// call here; it is not added to the position-token registry.
pub(crate) const AERO_BASIC_USDC_NOCK: &str = "0x770fE8Fc9248c6382950A86E39a9391B41D59e0A";
pub(crate) const AERO_CL50_WETH_USDC: &str = "0x4a7af46bc8abc00048c6c7920a1488daf568366c";
pub(crate) const AERO_CL100_WETH_USDC: &str = "0x2a430bebd2082d8e05d56d64dede8944279bbc24";
pub(crate) const AERO_CL200_WETH_NOCK: &str = "0x957256c98d61A714081DE2EE70ae3A96c9b40874";
pub(crate) const AERO_CL1_USDC_USDT: &str = "0xd448990883e3bf9cdd8ebca368a3ecf9d629aaf7";
pub(crate) const AERO_CL100_WETH_USDT: &str = "0x2dbe0e264c7c246e05c6614794899937f264cf7b";
// Uniswap pools. v3 pools resolve from the v3 factory; v4 pools are named by the hash of
// their pool key; each id below was recomputed from the key the route table sends and
// then read through StateView (ONCHAIN_INTEGRATION.md §9).
pub(crate) const UNISWAP_V3_WETH_NOCK_1PCT: &str = "0x71a4a6f6ea5c26ce486b7ed0d8723a897ef5ad9a";
pub(crate) const UNISWAP_V4_USDC_NOCK_3PCT_ID: &str =
    "0x064a5df2ecfcb7c05779b147aa38e4b0e91c966267fbdbc8cf16addd83636b13";
pub(crate) const UNISWAP_V4_ETH_NOCK_2PCT_ID: &str =
    "0xd316152008e12105594c32686b9d0401fd2c60ca4805c84f280069d218b771a8";
pub(crate) const UNISWAP_V4_ETH_USDC_005PCT_ID: &str =
    "0x96d4b53a38337a5733179751781178a2613306063c511b78cd02684739288c0a";

// Selectors, recomputed with the repo keccak tool from the signatures beside them.
pub(crate) const AERO_QUOTE_EXACT_INPUT: &str = "0x76278b93"; // quoteExactInput(address[],address,uint256)
pub(crate) const V3_QUOTE_EXACT_INPUT: &str = "0xcdca1753"; // quoteExactInput(bytes,uint256)
/// quoteExactInputSingle(((address,address,uint24,int24,address),bool,uint128,bytes))
pub(crate) const V4_QUOTE_EXACT_INPUT_SINGLE: &str = "0xaa9d21cb";
/// quoteExactInput((address,(address,uint24,int24,address,bytes)[],uint128))
pub(crate) const V4_QUOTE_EXACT_INPUT: &str = "0xca253dc9";
pub(crate) const POOL_FEE: &str = "0xddca3f43"; // fee()
pub(crate) const AERO_CUSTOM_FEE: &str = "0x4d419abc"; // customFee(address), on the fee module
pub(crate) const AERO_DEFAULT_FEE: &str = "0x5a6c72d0"; // defaultFee(), on the factory
/// The fee module's `ZERO_FEE_INDICATOR()` (0x38c55d46), read as 420: a custom fee of
/// exactly this value means "no fee".
pub(crate) const AERO_ZERO_FEE_INDICATOR: u128 = 420;

pub(crate) const VENUE_AERO: &str = "Aero";
pub(crate) const VENUE_UNISWAP: &str = "Uniswap";
/// The venues compared, in display order.
pub(crate) const QUOTE_VENUES: [&str; 2] = [VENUE_AERO, VENUE_UNISWAP];

/// One quoter request may take this long, and is tried this many times. The retries
/// also cover a node that has not yet seen the block the quotes are pinned to (Base
/// produces a block every 200 ms): "header not found" is an RPC error, not a revert.
pub(crate) const QUOTE_REQUEST_TIMEOUT_SECONDS: u64 = 8;
pub(crate) const QUOTE_REQUEST_ATTEMPTS: u32 = 3;
const QUOTE_RETRY_BACKOFF_MS: u64 = 200;
/// The block read that pins the quotes may take this long; without a block nothing is
/// quoted.
pub(crate) const QUOTE_BLOCK_READ_TIMEOUT_SECONDS: u64 = 3;
/// All fee reads of the two best routes together may take this long, each tried once.
/// A fee not read in time is shown as "not read"; the quotes themselves stand.
pub(crate) const QUOTE_FEE_BUDGET_SECONDS: u64 = 2;
/// All quotes for a tranche are abandoned after this long, so a stalled RPC cannot
/// hold up the cycle. The three budgets above fit inside it (asserted below), so this
/// is a backstop: quotes that were read are not thrown away for a slow fee read.
pub(crate) const QUOTE_TIMEOUT_SECONDS: u64 = 30;

/// The venue's own swap page. Neither app documents a way to prefill both the tokens
/// and the chain, so nothing is prefilled: the operator enters the amount shown.
/// Aero links go to app.aero.xyz, never the retired aerodrome.finance. Pure.
pub(crate) fn venue_swap_url(venue: &str) -> &'static str {
    match venue {
        VENUE_AERO => "https://app.aero.xyz/swap",
        _ => "https://app.uniswap.org/swap",
    }
}

// ---------------- Route table (pure) ----------------

/// Where a hop's fee comes from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum HopFee {
    /// Fixed by the pool's identity (a Uniswap v3 fee tier or v4 pool key), in millionths.
    Fixed(u32),
    /// An Aero concentrated pool: `fee()` at the quote block, in millionths. These
    /// pools carry a dynamic fee, so it is read with the quote.
    AeroConcentrated,
    /// The Aero basic pool: the fee module's `customFee(pool)`, else the factory's
    /// `defaultFee()`, in basis points.
    AeroBasic,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct HopSpec {
    /// Pool address, or the pool id for a Uniswap v4 pool.
    pub(crate) pool: &'static str,
    pub(crate) kind: &'static str,
    pub(crate) token_in: &'static str,
    pub(crate) token_out: &'static str,
    pub(crate) fee: HopFee,
}

/// How a route is asked for.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum QuoteCall {
    /// Aero Metaquoter over the hop pools in order, USDC in.
    AeroMetaquoter,
    /// Uniswap v3 QuoterV2 over a packed token/fee path.
    UniswapV3 { tokens: Vec<&'static str>, fees: Vec<u32> },
    /// Uniswap V4Quoter, one pool (hooks: none).
    UniswapV4Single { currency0: &'static str, currency1: &'static str, fee: u32, tick_spacing: u32, zero_for_one: bool },
    /// Uniswap V4Quoter, several pools: (currency out of the hop, fee, tick spacing).
    UniswapV4Path { currency_in: &'static str, hops: Vec<(&'static str, u32, u32)> },
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RouteSpec {
    pub(crate) id: &'static str,
    pub(crate) venue: &'static str,
    pub(crate) hops: Vec<HopSpec>,
    pub(crate) call: QuoteCall,
}

impl RouteSpec {
    /// "USDC → WETH → NOCK"
    pub(crate) fn label(&self) -> String {
        let mut out = self.hops.first().map(|h| h.token_in.to_string()).unwrap_or_default();
        for h in &self.hops {
            out.push_str(" → ");
            out.push_str(h.token_out);
        }
        out
    }
}

const KIND_AERO_BASIC: &str = "Aero basic volatile";
const KIND_AERO_CL: &str = "Aero concentrated";
const KIND_UNI_V3: &str = "Uniswap v3";
const KIND_UNI_V4: &str = "Uniswap v4";

/// Every route NockNock quotes, USDC in and NOCK out. The table is explicit and built
/// in: a route is here only because each of its pools and its quoter answered a live
/// call with a sane quote (ONCHAIN_INTEGRATION.md §9). Nothing is discovered at runtime.
///
/// Aero: the basic pool directly; through WETH by either Aero USDC/WETH concentrated
/// pool; and the three-hop route through USDT that the Aero app itself used for the
/// plan's first buy. Uniswap: the v4 USDC/NOCK 3% pool directly; through WETH on v3;
/// through native ETH on v4. The legacy Aerodrome vAMM is not here: app.aero.xyz does
/// not list it, so its link could not trade it. Pure.
pub(crate) fn route_table() -> Vec<RouteSpec> {
    let aero_cl = |pool, token_in, token_out| HopSpec { pool, kind: KIND_AERO_CL, token_in, token_out, fee: HopFee::AeroConcentrated };
    let weth_nock_aero = || aero_cl(AERO_CL200_WETH_NOCK, "WETH", "NOCK");
    vec![
        RouteSpec {
            id: "aero-basic-direct",
            venue: VENUE_AERO,
            hops: vec![HopSpec {
                pool: AERO_BASIC_USDC_NOCK,
                kind: KIND_AERO_BASIC,
                token_in: "USDC",
                token_out: "NOCK",
                fee: HopFee::AeroBasic,
            }],
            call: QuoteCall::AeroMetaquoter,
        },
        RouteSpec {
            id: "aero-cl50-weth",
            venue: VENUE_AERO,
            hops: vec![aero_cl(AERO_CL50_WETH_USDC, "USDC", "WETH"), weth_nock_aero()],
            call: QuoteCall::AeroMetaquoter,
        },
        RouteSpec {
            id: "aero-cl100-weth",
            venue: VENUE_AERO,
            hops: vec![aero_cl(AERO_CL100_WETH_USDC, "USDC", "WETH"), weth_nock_aero()],
            call: QuoteCall::AeroMetaquoter,
        },
        RouteSpec {
            id: "aero-usdt-weth",
            venue: VENUE_AERO,
            hops: vec![
                aero_cl(AERO_CL1_USDC_USDT, "USDC", "USDT"),
                aero_cl(AERO_CL100_WETH_USDT, "USDT", "WETH"),
                weth_nock_aero(),
            ],
            call: QuoteCall::AeroMetaquoter,
        },
        RouteSpec {
            id: "uniswap-v4-direct",
            venue: VENUE_UNISWAP,
            hops: vec![HopSpec {
                pool: UNISWAP_V4_USDC_NOCK_3PCT_ID,
                kind: KIND_UNI_V4,
                token_in: "USDC",
                token_out: "NOCK",
                fee: HopFee::Fixed(30_000),
            }],
            call: QuoteCall::UniswapV4Single {
                currency0: USDC_BASE,
                currency1: NOCK_BASE,
                fee: 30_000,
                tick_spacing: 300,
                zero_for_one: true,
            },
        },
        RouteSpec {
            id: "uniswap-v3-weth",
            venue: VENUE_UNISWAP,
            hops: vec![
                HopSpec { pool: DEFAULT_ETH_PRICE_POOL, kind: KIND_UNI_V3, token_in: "USDC", token_out: "WETH", fee: HopFee::Fixed(500) },
                HopSpec { pool: UNISWAP_V3_WETH_NOCK_1PCT, kind: KIND_UNI_V3, token_in: "WETH", token_out: "NOCK", fee: HopFee::Fixed(10_000) },
            ],
            call: QuoteCall::UniswapV3 { tokens: vec![USDC_BASE, WETH_BASE, NOCK_BASE], fees: vec![500, 10_000] },
        },
        RouteSpec {
            id: "uniswap-v4-eth",
            venue: VENUE_UNISWAP,
            hops: vec![
                HopSpec { pool: UNISWAP_V4_ETH_USDC_005PCT_ID, kind: KIND_UNI_V4, token_in: "USDC", token_out: "ETH", fee: HopFee::Fixed(500) },
                HopSpec { pool: UNISWAP_V4_ETH_NOCK_2PCT_ID, kind: KIND_UNI_V4, token_in: "ETH", token_out: "NOCK", fee: HopFee::Fixed(20_000) },
            ],
            call: QuoteCall::UniswapV4Path {
                currency_in: USDC_BASE,
                hops: vec![(NATIVE_ETH, 500, 10), (NOCK_BASE, 20_000, 400)],
            },
        },
    ]
}

// ---------------- Calldata (pure) ----------------

fn abi_word(v: u128) -> String {
    format!("{v:064x}")
}

fn abi_address(a: &str) -> String {
    format!("{:0>64}", a.trim_start_matches("0x").to_lowercase())
}

/// Aero `Metaquoter.quoteExactInput(address[] pools, address tokenIn, uint256 amountIn)`.
pub(crate) fn metaquoter_exact_input_calldata(pools: &[&str], token_in: &str, amount_in: u128) -> String {
    let mut out = format!(
        "{AERO_QUOTE_EXACT_INPUT}{}{}{}{}",
        abi_word(0x60),
        abi_address(token_in),
        abi_word(amount_in),
        abi_word(pools.len() as u128)
    );
    for p in pools {
        out.push_str(&abi_address(p));
    }
    out
}

/// Uniswap v3 `QuoterV2.quoteExactInput(bytes path, uint256 amountIn)`; the path is
/// token (20 bytes), fee (3 bytes), token, … packed.
pub(crate) fn v3_exact_input_calldata(tokens: &[&str], fees: &[u32], amount_in: u128) -> String {
    let mut path = String::new();
    for (i, t) in tokens.iter().enumerate() {
        if i > 0 {
            path.push_str(&format!("{:06x}", fees.get(i - 1).copied().unwrap_or(0)));
        }
        path.push_str(&t.trim_start_matches("0x").to_lowercase());
    }
    let len = path.len() / 2;
    let padding = (64 - path.len() % 64) % 64;
    format!(
        "{V3_QUOTE_EXACT_INPUT}{}{}{}{path}{}",
        abi_word(0x40),
        abi_word(amount_in),
        abi_word(len as u128),
        "0".repeat(padding)
    )
}

/// Uniswap `V4Quoter.quoteExactInputSingle(QuoteExactSingleParams)`: pool key, direction,
/// amount, empty hook data. Hooks are the zero address for every pool in the table.
pub(crate) fn v4_exact_input_single_calldata(
    currency0: &str,
    currency1: &str,
    fee: u32,
    tick_spacing: u32,
    zero_for_one: bool,
    amount_in: u128,
) -> String {
    format!(
        "{V4_QUOTE_EXACT_INPUT_SINGLE}{}{}{}{}{}{}{}{}{}{}",
        abi_word(0x20),
        abi_address(currency0),
        abi_address(currency1),
        abi_word(fee as u128),
        abi_word(tick_spacing as u128),
        abi_address(NATIVE_ETH), // hooks: none
        abi_word(zero_for_one as u128),
        abi_word(amount_in),
        abi_word(0x100), // offset of hookData within the struct
        abi_word(0),     // hookData: empty
    )
}

/// Uniswap `V4Quoter.quoteExactInput(QuoteExactParams)`: the input currency, then one
/// PathKey per hop (currency out, fee, tick spacing, no hooks, empty hook data).
pub(crate) fn v4_exact_input_calldata(currency_in: &str, hops: &[(&str, u32, u32)], amount_in: u128) -> String {
    let n = hops.len() as u128;
    let mut out = format!(
        "{V4_QUOTE_EXACT_INPUT}{}{}{}{}{}",
        abi_word(0x20),
        abi_address(currency_in),
        abi_word(0x60), // offset of the path array within the struct
        abi_word(amount_in),
        abi_word(n)
    );
    // Each PathKey is dynamic (it ends in `bytes`), so the array starts with offsets.
    for i in 0..n {
        out.push_str(&abi_word(n * 0x20 + i * 0xc0));
    }
    for (currency, fee, tick_spacing) in hops {
        out.push_str(&abi_address(currency));
        out.push_str(&abi_word(*fee as u128));
        out.push_str(&abi_word(*tick_spacing as u128));
        out.push_str(&abi_address(NATIVE_ETH)); // hooks: none
        out.push_str(&abi_word(0xa0)); // offset of hookData within the PathKey
        out.push_str(&abi_word(0)); // hookData: empty
    }
    out
}

/// The quoter and calldata for one route and one USDC amount (raw, 6 decimals). Pure.
pub(crate) fn quote_request(spec: &RouteSpec, amount_usdc_raw: u128) -> (&'static str, String) {
    match &spec.call {
        QuoteCall::AeroMetaquoter => {
            let pools: Vec<&str> = spec.hops.iter().map(|h| h.pool).collect();
            (AERO_METAQUOTER, metaquoter_exact_input_calldata(&pools, USDC_BASE, amount_usdc_raw))
        }
        QuoteCall::UniswapV3 { tokens, fees } => {
            (UNISWAP_V3_QUOTER_V2, v3_exact_input_calldata(tokens, fees, amount_usdc_raw))
        }
        QuoteCall::UniswapV4Single { currency0, currency1, fee, tick_spacing, zero_for_one } => (
            UNISWAP_V4_QUOTER,
            v4_exact_input_single_calldata(currency0, currency1, *fee, *tick_spacing, *zero_for_one, amount_usdc_raw),
        ),
        QuoteCall::UniswapV4Path { currency_in, hops } => {
            (UNISWAP_V4_QUOTER, v4_exact_input_calldata(currency_in, hops, amount_usdc_raw))
        }
    }
}

// ---------------- Answers (pure) ----------------

/// Why a quoter call gave no data.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum QuoteCallFailure {
    /// The contract reverted, with the node's own message when it gave one.
    Reverted(Option<String>),
    /// No usable answer. The text is written here by kind of failure: a transport
    /// error's own text is never used, because it can contain the RPC URL and its key.
    NoAnswer(String),
}

/// Classify one `eth_call` JSON-RPC response. Error code 3 is "execution reverted";
/// nodes also say so in the message. Pure.
pub(crate) fn classify_quote_call(v: &serde_json::Value) -> std::result::Result<String, QuoteCallFailure> {
    if let Some(hex) = v.get("result").and_then(|r| r.as_str()) {
        return Ok(hex.trim_start_matches("0x").to_string());
    }
    let Some(err) = v.get("error") else {
        return Err(QuoteCallFailure::NoAnswer("the RPC's answer had neither a result nor an error".to_string()));
    };
    let code = err.get("code").and_then(|c| c.as_i64());
    let message: String = err
        .get("message")
        .and_then(|m| m.as_str())
        .unwrap_or("")
        .chars()
        .filter(|c| c.is_ascii_graphic() || *c == ' ')
        .take(80)
        .collect();
    let lower = message.to_lowercase();
    if code == Some(3) || lower.contains("revert") {
        // "execution reverted: Unexpected error" keeps its tail; a bare revert has none.
        let reason = lower
            .starts_with("execution reverted")
            .then(|| message["execution reverted".len()..].trim_start_matches(':').trim().to_string())
            .filter(|r| !r.is_empty());
        return Err(QuoteCallFailure::Reverted(reason));
    }
    Err(QuoteCallFailure::NoAnswer(match code {
        Some(code) => format!("the RPC answered with error {code}: {message}"),
        None => format!("the RPC answered with an error: {message}"),
    }))
}

/// The first return word of a quoter answer: the amount out. Every quoter in the table
/// returns it first. An answer that is short, does not fit, or is zero is an error,
/// never "0 NOCK". Pure.
pub(crate) fn decode_amount_out(hex: &str) -> std::result::Result<u128, String> {
    let clean = hex.trim_start_matches("0x");
    if !clean.is_ascii() {
        return Err("the quoter's answer was not hex".to_string());
    }
    if clean.len() < 64 {
        return Err("the quoter's answer was too short to hold an amount".to_string());
    }
    let word = &clean[..64];
    if word[..32].chars().any(|c| c != '0') {
        return Err("the quoter's answer does not fit an amount".to_string());
    }
    let out = u128::from_str_radix(&word[32..], 16).map_err(|_| "the quoter's answer was not hex".to_string())?;
    if out == 0 {
        return Err("the quoter returned zero NOCK".to_string());
    }
    Ok(out)
}

/// A single unsigned word (a fee read). Pure.
pub(crate) fn decode_small_word(hex: &str) -> Option<u128> {
    let clean = hex.trim_start_matches("0x");
    if !clean.is_ascii() || clean.len() < 64 || clean[..32].chars().any(|c| c != '0') {
        return None;
    }
    u128::from_str_radix(&clean[32..64], 16).ok()
}

/// What one route's quote came to.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum QuoteOutcome {
    /// NOCK out, raw (16 decimals).
    Quoted(u128),
    Unavailable(String),
}

/// Uniswap's sqrt-price bounds. A swap quoted with no price limit stops one step inside
/// them when a pool has no liquidity left to trade against.
const MIN_SQRT_RATIO_PLUS_ONE: (u128, u128) = (0, 0x1_0002_76a4);
const MAX_SQRT_RATIO_MINUS_ONE: (u128, u128) = (0xfffd_8963, 0xefd1_fc6a_5064_8849_5d95_1d52_6398_8d25);

/// Whether a quoter answer of the shape (amountOut, uint160[] sqrtPriceX96After, …)
/// shows a pool driven to the end of its price range: the swap ran out of liquidity
/// there, and the amount is for only the part of the input that did swap. Uniswap
/// QuoterV2 was observed to answer exactly so: a partial amount, the price pinned at
/// the bound, no error. The Aero Metaquoter answers in the same shape, but what it says
/// when an Aero concentrated pool has no liquidity left was not observed (the Aero
/// pools on these routes hold full-range liquidity and fill any size at a collapsing
/// price); the same check is applied to it, and a revert is handled as a revert. Zero
/// entries (a basic-pool hop) are skipped. None when the answer is not that shape. Pure.
pub(crate) fn pool_exhausted(hex: &str) -> Option<bool> {
    let clean = hex.trim_start_matches("0x");
    if !clean.is_ascii() {
        return None;
    }
    let word = |i: usize| clean.get(i * 64..(i + 1) * 64);
    let small = |i: usize| word(i).and_then(decode_small_word).and_then(|v| usize::try_from(v).ok());
    let offset = small(1)?;
    if offset % 32 != 0 {
        return None;
    }
    let start = offset / 32;
    let len = small(start)?;
    if len > 16 {
        return None;
    }
    let mut exhausted = false;
    for i in 0..len {
        let v = U256::from_hex_be(word(start + 1 + i)?).ok()?;
        let (hi, lo) = (v.0, v.1);
        if (hi, lo) == (0, 0) {
            continue;
        }
        if (hi, lo) <= MIN_SQRT_RATIO_PLUS_ONE || (hi, lo) >= MAX_SQRT_RATIO_MINUS_ONE {
            exhausted = true;
        }
    }
    Some(exhausted)
}

/// Turn a quoter call's result into an outcome, in words fit for the plan status. Pure.
pub(crate) fn outcome_from_call(spec: &RouteSpec, result: std::result::Result<String, QuoteCallFailure>) -> QuoteOutcome {
    match result {
        Ok(hex) => {
            // The V4Quoter reverts when a pool cannot fill the amount. QuoterV2 answers
            // with a partial amount and the price at its bound, which must never be
            // shown as the quote; the Metaquoter's answer has the same shape and gets
            // the same check (its behaviour on a dry pool was not observed).
            let reports_prices = !matches!(spec.call, QuoteCall::UniswapV4Single { .. } | QuoteCall::UniswapV4Path { .. });
            if reports_prices {
                match pool_exhausted(&hex) {
                    Some(false) => {}
                    Some(true) => {
                        return QuoteOutcome::Unavailable(
                            "a pool on this route ran out of liquidity before the whole amount was swapped".to_string(),
                        )
                    }
                    None => return QuoteOutcome::Unavailable("the quoter's answer was not understood".to_string()),
                }
            }
            match decode_amount_out(&hex) {
                Ok(out) => QuoteOutcome::Quoted(out),
                Err(why) => QuoteOutcome::Unavailable(why),
            }
        }
        Err(QuoteCallFailure::Reverted(Some(reason))) => QuoteOutcome::Unavailable(format!(
            "the quoter reverted ({reason}): this route cannot fill the amount at this block"
        )),
        Err(QuoteCallFailure::Reverted(None)) => {
            QuoteOutcome::Unavailable("the quoter reverted: this route cannot fill the amount at this block".to_string())
        }
        Err(QuoteCallFailure::NoAnswer(why)) => QuoteOutcome::Unavailable(why),
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RouteResult {
    pub(crate) spec: RouteSpec,
    pub(crate) outcome: QuoteOutcome,
}

/// Exact decimal string to raw units: "20.00" at 6 decimals is 20000000. None for
/// anything that is not a plain non-negative decimal or has more digits than the token.
/// Pure.
pub(crate) fn units_to_raw(amount: &str, decimals: u32) -> Option<u128> {
    let (whole, frac) = amount.trim().split_once('.').unwrap_or((amount.trim(), ""));
    if whole.is_empty() && frac.is_empty() {
        return None;
    }
    if !whole.chars().all(|c| c.is_ascii_digit()) || !frac.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let frac = frac.trim_end_matches('0');
    if frac.len() > decimals as usize {
        return None;
    }
    let digits = format!("{whole}{frac}{}", "0".repeat(decimals as usize - frac.len()));
    digits.parse::<u128>().ok()
}

/// NOCK to four decimals from raw units, truncated, without floats. Pure.
pub(crate) fn nock_4dp(raw: u128) -> String {
    format!("{}.{:04}", raw / 10u128.pow(16), (raw % 10u128.pow(16)) / 10u128.pow(12))
}

/// USDC per NOCK for `amount_usdc_raw` in and `nock_out_raw` out. Pure.
pub(crate) fn quote_effective_price(amount_usdc_raw: u128, nock_out_raw: u128) -> Option<f64> {
    if amount_usdc_raw == 0 || nock_out_raw == 0 {
        return None;
    }
    let price = (amount_usdc_raw as f64 / 1e6) / (nock_out_raw as f64 / 1e16);
    price.is_finite().then_some(price)
}

/// (effective price / reference spot − 1) × 100. None without a usable spot: an unknown
/// reference gives no cost figure. Negative when the route beats the reference pool's
/// mid price, which happens: the reference is one pool, not the market. Pure.
pub(crate) fn cost_over_spot_pct(effective_price: f64, spot: Option<f64>) -> Option<f64> {
    let spot = spot.filter(|s| s.is_finite() && *s > 0.0)?;
    let pct = (effective_price / spot - 1.0) * 100.0;
    pct.is_finite().then_some(pct)
}

/// Fee in percent from millionths (9700 → "0.97") and from basis points (30 → "0.3").
pub(crate) fn fee_pct_from_millionths(fee: u128) -> String {
    format_units(fee, 4)
}
pub(crate) fn fee_pct_from_bps(bps: u128) -> String {
    format_units(bps, 2)
}

/// The Aero basic pool's fee in basis points from the fee module's `customFee(pool)`
/// and the factory's `defaultFee()`: the indicator value means no fee, any other
/// non-zero custom fee replaces the default. Pure.
pub(crate) fn aero_basic_fee_bps(custom_fee: u128, default_fee: u128) -> u128 {
    if custom_fee == AERO_ZERO_FEE_INDICATOR {
        0
    } else if custom_fee != 0 {
        custom_fee
    } else {
        default_fee
    }
}

/// Index of the venue's route that returns the most NOCK; the earlier route wins a tie.
/// None when no route of that venue was quoted. Pure.
pub(crate) fn best_route_index(results: &[RouteResult], venue: &str) -> Option<usize> {
    let mut best: Option<(usize, u128)> = None;
    for (i, r) in results.iter().enumerate() {
        if r.spec.venue != venue {
            continue;
        }
        if let QuoteOutcome::Quoted(out) = r.outcome {
            if best.map_or(true, |(_, b)| out > b) {
                best = Some((i, out));
            }
        }
    }
    best.map(|(i, _)| i)
}

/// One venue's entry: its best route with hops, price and cost, or "unavailable" with
/// the reason. `best_fees` holds the fee (percent) of each hop of the best route, None
/// where the fee read failed. Pure.
pub(crate) fn build_venue_quote(
    venue: &str,
    results: &[RouteResult],
    best_fees: &[Option<String>],
    amount_usdc_raw: u128,
    spot: Option<f64>,
) -> VenueQuote {
    let own: Vec<&RouteResult> = results.iter().filter(|r| r.spec.venue == venue).collect();
    let routes_tried: Vec<RouteAttempt> = own
        .iter()
        .map(|r| RouteAttempt {
            route_id: r.spec.id.to_string(),
            route: r.spec.label(),
            nock_out: match &r.outcome {
                QuoteOutcome::Quoted(out) => Some(format_units(*out, 16)),
                QuoteOutcome::Unavailable(_) => None,
            },
            reason: match &r.outcome {
                QuoteOutcome::Quoted(_) => None,
                QuoteOutcome::Unavailable(why) => Some(why.clone()),
            },
        })
        .collect();
    let unavailable = |reason: String| VenueQuote {
        venue: venue.to_string(),
        status: "unavailable".to_string(),
        reason: Some(reason),
        swap_url: venue_swap_url(venue).to_string(),
        routes_tried: routes_tried.clone(),
        ..Default::default()
    };
    if own.is_empty() {
        return unavailable("quoter not verified".to_string());
    }
    let Some(best) = best_route_index(results, venue) else {
        let mut reasons: Vec<String> = Vec::new();
        for r in &own {
            if let QuoteOutcome::Unavailable(why) = &r.outcome {
                if !reasons.contains(why) {
                    reasons.push(why.clone());
                }
            }
        }
        return unavailable(format!("none of its {} verified route(s) could be quoted: {}", own.len(), reasons.join("; ")));
    };
    let r = &results[best];
    let QuoteOutcome::Quoted(out) = r.outcome else {
        return unavailable("no route was quoted".to_string());
    };
    let Some(price) = quote_effective_price(amount_usdc_raw, out) else {
        return unavailable("the quoted amounts give no price".to_string());
    };
    VenueQuote {
        venue: venue.to_string(),
        status: "quoted".to_string(),
        reason: None,
        swap_url: venue_swap_url(venue).to_string(),
        nock_out: Some(format_units(out, 16)),
        effective_price_usd: Some(format!("{price:.8}")),
        cost_over_spot_pct: cost_over_spot_pct(price, spot).map(|p| format!("{p:.2}")),
        route: Some(r.spec.label()),
        hops: r
            .spec
            .hops
            .iter()
            .enumerate()
            .map(|(i, h)| RouteHop {
                pool: h.pool.to_lowercase(),
                pool_kind: h.kind.to_string(),
                token_in: h.token_in.to_string(),
                token_out: h.token_out.to_string(),
                fee_pct: best_fees.get(i).cloned().flatten(),
            })
            .collect(),
        routes_tried,
    }
}

/// Which venue returns more NOCK, by how much, and the sentence that says so. A fact
/// about two contract reads at one block; with fewer than two quotes there is nothing
/// to compare and no venue is named. Pure.
pub(crate) fn compare_venues(
    venues: &[VenueQuote],
    amount_usdc: &str,
    block_number: Option<i64>,
) -> (Option<String>, Option<String>, String) {
    let at_block = block_number.map(|b| format!(" at block {b}")).unwrap_or_default();
    let quoted: Vec<(&str, u128)> = venues
        .iter()
        .filter(|v| v.status == "quoted")
        .filter_map(|v| Some((v.venue.as_str(), units_to_raw(v.nock_out.as_deref()?, 16)?)))
        .collect();
    match quoted.as_slice() {
        [] => (None, None, "Neither venue could be quoted, so there is nothing to compare.".to_string()),
        [(only, _)] => (
            None,
            None,
            format!("Only {only} could be quoted{at_block}; with one quote there is nothing to compare."),
        ),
        [(a, a_out), (b, b_out), ..] => {
            if a_out == b_out {
                return (
                    None,
                    None,
                    format!(
                        "{a} and {b} return the same NOCK for ${amount_usdc} USDC{at_block}: {}.",
                        nock_4dp(*a_out)
                    ),
                );
            }
            let ((more, more_out), (less, less_out)) =
                if a_out > b_out { ((a, a_out), (b, b_out)) } else { ((b, b_out), (a, a_out)) };
            let pct = (*more_out as f64 / *less_out as f64 - 1.0) * 100.0;
            let pct_text = format!("{pct:.2}");
            let by = if pct_text == "0.00" { "less than 0.01%".to_string() } else { format!("{pct_text}%") };
            (
                Some(more.to_string()),
                Some(pct_text),
                format!(
                    "{more} returns more NOCK by {by} for ${amount_usdc} USDC{at_block}: {} NOCK against {} on {less}.",
                    nock_4dp(*more_out),
                    nock_4dp(*less_out)
                ),
            )
        }
    }
}

/// The whole result from the per-venue entries. Pure.
pub(crate) fn assemble_route_quotes(
    amount_usdc: &str,
    block: Option<(i64, DateTime<Utc>)>,
    spot: Option<f64>,
    venues: Vec<VenueQuote>,
) -> RouteQuotes {
    let quoted = venues.iter().filter(|v| v.status == "quoted").count();
    let missing: Vec<String> = venues
        .iter()
        .filter(|v| v.status != "quoted")
        .map(|v| format!("{}: {}", v.venue, v.reason.clone().unwrap_or_else(|| "no reason given".to_string())))
        .collect();
    let (status, detail) = if quoted == venues.len() && quoted > 0 {
        ("ok", None)
    } else if quoted > 0 {
        ("partial", Some(missing.join("; ")))
    } else {
        ("unavailable", Some(missing.join("; ")))
    };
    let block_number = block.map(|(n, _)| n);
    let (more_nock_venue, more_nock_by_pct, comparison) = compare_venues(&venues, amount_usdc, block_number);
    RouteQuotes {
        status: status.to_string(),
        detail,
        amount_usdc: amount_usdc.to_string(),
        block_number,
        quoted_at: block.map(|(_, t)| t.to_rfc3339()),
        reference_spot_usd: spot.filter(|s| s.is_finite() && *s > 0.0).map(|s| format!("{s:.8}")),
        venues,
        more_nock_venue,
        more_nock_by_pct,
        comparison,
    }
}

/// Both venues unavailable for one stated reason (no RPC key, no block, a timeout).
/// No block and no quoted number: nothing was read. `spot` is the cycle's own reference
/// spot, carried at full precision so a later refresh measures against the same figure
/// the cycle would have used. Pure.
pub(crate) fn route_quotes_unavailable(amount_usdc: &str, reason: &str, spot: Option<f64>) -> RouteQuotes {
    let venues = QUOTE_VENUES
        .iter()
        .map(|v| VenueQuote {
            venue: v.to_string(),
            status: "unavailable".to_string(),
            reason: Some(reason.to_string()),
            swap_url: venue_swap_url(v).to_string(),
            ..Default::default()
        })
        .collect();
    let mut q = assemble_route_quotes(amount_usdc, None, spot, venues);
    q.detail = Some(reason.to_string());
    q
}

/// The amount to quote, when there is one: quoting is on, the plan's status is
/// `tranche_due`, and the tranche is a positive USDC amount. Keyed on the status field,
/// never on the advice text. Anything else is None and no call is made. Pure.
pub(crate) fn tranche_to_quote(enabled: bool, plan: Option<&AccumulationPlanStatus>) -> Option<(String, u128)> {
    let plan = plan.filter(|p| enabled && p.status == "tranche_due")?;
    let raw = units_to_raw(&plan.tranche_usdc, 6).filter(|r| *r > 0)?;
    Some((plan.tranche_usdc.clone(), raw))
}

/// A sentence pointing from the due-tranche advice to the quotes. Added only to the
/// plan's own due-tranche advice, whenever quotes were asked for (`quotes` is Some). It
/// carries no number and does not depend on what the quoters answered: the advice text
/// is part of the state hash, and a quote coming or going must not move it. Pure.
pub(crate) fn with_route_quote_pointer(action: &str, details: String, quotes: Option<&RouteQuotes>) -> String {
    const POINTER: &str =
        "Read-only route quotes for this amount on Aero and on Uniswap, or why a venue could not be quoted, are listed with the plan.";
    let requested = quotes.is_some();
    if requested && action.contains("DCA TRANCHE DUE") && !details.contains(POINTER) {
        format!("{details} {POINTER}")
    } else {
        details
    }
}

/// Block number and time from an `eth_getBlockByNumber` response. Pure.
pub(crate) fn parse_block_header(resp: &serde_json::Value) -> Option<(i64, DateTime<Utc>)> {
    let number = resp
        .pointer("/result/number")
        .and_then(|n| n.as_str())
        .and_then(|n| i64::from_str_radix(n.trim_start_matches("0x"), 16).ok())?;
    Some((number, block_time_from_response(resp)?))
}

// ---------------- Reads ----------------

/// How long each step of one check may take. The defaults are the constants above;
/// tests shorten them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct QuoteBudgets {
    pub(crate) block_read: std::time::Duration,
    pub(crate) request: std::time::Duration,
    pub(crate) attempts: u32,
    pub(crate) fees: std::time::Duration,
}

impl Default for QuoteBudgets {
    fn default() -> Self {
        Self {
            block_read: std::time::Duration::from_secs(QUOTE_BLOCK_READ_TIMEOUT_SECONDS),
            request: std::time::Duration::from_secs(QUOTE_REQUEST_TIMEOUT_SECONDS),
            attempts: QUOTE_REQUEST_ATTEMPTS,
            fees: std::time::Duration::from_secs(QUOTE_FEE_BUDGET_SECONDS),
        }
    }
}

/// The longest one quoter call can take: every attempt timing out, plus the pauses.
/// With the block read before it and the fee reads after it, it is kept inside
/// `QUOTE_TIMEOUT_SECONDS` (asserted below) so a stalled endpoint is reported per route
/// as "the quote request timed out", not as an abandoned check.
const fn quote_call_worst_case_ms() -> u64 {
    let mut total = 0;
    let mut attempt = 0;
    while attempt < QUOTE_REQUEST_ATTEMPTS {
        total += QUOTE_REQUEST_TIMEOUT_SECONDS * 1000;
        if attempt + 1 < QUOTE_REQUEST_ATTEMPTS {
            total += QUOTE_RETRY_BACKOFF_MS << attempt;
        }
        attempt += 1;
    }
    total
}
const _: () = assert!(
    QUOTE_BLOCK_READ_TIMEOUT_SECONDS * 1000 + quote_call_worst_case_ms() + QUOTE_FEE_BUDGET_SECONDS * 1000
        < QUOTE_TIMEOUT_SECONDS * 1000
);

/// One `eth_call` at a stated block, with the reason when it fails. Each of `attempts`
/// tries may take `request_timeout`.
async fn quote_eth_call(
    client: &Client,
    rpc_url: &str,
    to: &str,
    data: &str,
    block_hex: &str,
    request_timeout: std::time::Duration,
    attempts: u32,
) -> std::result::Result<String, QuoteCallFailure> {
    let payload = json!({
        "jsonrpc": "2.0", "id": 46, "method": "eth_call",
        "params": [{ "to": to, "data": data }, block_hex]
    });
    let mut last = QuoteCallFailure::NoAnswer("the RPC did not answer".to_string());
    for attempt in 0..attempts {
        let sent = client
            .post(rpc_url)
            .timeout(request_timeout)
            .json(&payload)
            .send()
            .await;
        let failure = match sent {
            Ok(resp) if resp.status().is_success() => match resp.json::<serde_json::Value>().await {
                Ok(v) => match classify_quote_call(&v) {
                    Ok(hex) => return Ok(hex),
                    // A revert is the contract's answer: asking again changes nothing.
                    Err(reverted @ QuoteCallFailure::Reverted(_)) => return Err(reverted),
                    Err(other) => other,
                },
                Err(_) => QuoteCallFailure::NoAnswer("the RPC's answer was not JSON".to_string()),
            },
            Ok(resp) => QuoteCallFailure::NoAnswer(format!("the RPC answered HTTP {}", resp.status().as_u16())),
            Err(e) if e.is_timeout() => QuoteCallFailure::NoAnswer("the quote request timed out".to_string()),
            Err(e) if e.is_connect() => QuoteCallFailure::NoAnswer("the connection to the RPC failed".to_string()),
            Err(_) => QuoteCallFailure::NoAnswer("the quote request failed in transit".to_string()),
        };
        last = failure;
        if attempt + 1 < attempts {
            tokio::time::sleep(tokio::time::Duration::from_millis(QUOTE_RETRY_BACKOFF_MS << attempt)).await;
        }
    }
    Err(last)
}

/// The latest block's number and time, so every quote can be read at that one block.
async fn latest_block(client: &Client, rpc_url: &str) -> Option<(i64, DateTime<Utc>)> {
    let payload = json!({ "jsonrpc": "2.0", "id": 47, "method": "eth_getBlockByNumber", "params": ["latest", false] });
    post_rpc_with_retry(client, rpc_url, &payload, 2).await.and_then(|res| parse_block_header(&res))
}

/// The fee of one hop in percent, read at the quote block where it is not fixed. Each
/// read is tried once. None when a read fails: the hop is then shown without a fee,
/// never with a default one.
async fn read_hop_fee(
    client: &Client,
    rpc_url: &str,
    hop: &HopSpec,
    block_hex: &str,
    request_timeout: std::time::Duration,
) -> Option<String> {
    let word = |r: std::result::Result<String, QuoteCallFailure>| r.ok().and_then(|hex| decode_small_word(&hex));
    match hop.fee {
        HopFee::Fixed(millionths) => Some(fee_pct_from_millionths(millionths as u128)),
        HopFee::AeroConcentrated => {
            word(quote_eth_call(client, rpc_url, hop.pool, POOL_FEE, block_hex, request_timeout, 1).await)
                .map(fee_pct_from_millionths)
        }
        HopFee::AeroBasic => {
            let custom_data = format!("{AERO_CUSTOM_FEE}{}", abi_address(hop.pool));
            let (custom, default) = tokio::join!(
                quote_eth_call(client, rpc_url, AERO_VOLATILE_FEE_MODULE, &custom_data, block_hex, request_timeout, 1),
                quote_eth_call(client, rpc_url, AERO_VOLATILE_POOL_FACTORY, AERO_DEFAULT_FEE, block_hex, request_timeout, 1),
            );
            Some(fee_pct_from_bps(aero_basic_fee_bps(word(custom)?, word(default)?)))
        }
    }
}

/// The fee of every hop of each venue's best route, all read at once inside
/// `budgets.fees`. One entry per venue of `QUOTE_VENUES`, one fee per hop; a fee that
/// was not read in time, or whose read failed, is None. This step cannot fail or hold
/// back the quotes: they were already read.
pub(crate) async fn read_best_route_fees(
    client: &Client,
    rpc_url: &str,
    results: &[RouteResult],
    block_hex: &str,
    budgets: &QuoteBudgets,
) -> Vec<Vec<Option<String>>> {
    let mut fees: Vec<Vec<Option<String>>> = Vec::with_capacity(QUOTE_VENUES.len());
    let mut reads = tokio::task::JoinSet::new();
    let request_timeout = budgets.request.min(budgets.fees);
    for (v, venue) in QUOTE_VENUES.iter().enumerate() {
        let hops = best_route_index(results, venue).map(|best| results[best].spec.hops.clone()).unwrap_or_default();
        fees.push(vec![None; hops.len()]);
        for (h, hop) in hops.into_iter().enumerate() {
            if let HopFee::Fixed(millionths) = hop.fee {
                fees[v][h] = Some(fee_pct_from_millionths(millionths as u128));
                continue;
            }
            let (client, rpc_url, block_hex) = (client.clone(), rpc_url.to_string(), block_hex.to_string());
            reads.spawn(async move { (v, h, read_hop_fee(&client, &rpc_url, &hop, &block_hex, request_timeout).await) });
        }
    }
    let deadline = tokio::time::Instant::now() + budgets.fees;
    loop {
        match tokio::time::timeout_at(deadline, reads.join_next()).await {
            Ok(Some(Ok((v, h, fee)))) => fees[v][h] = fee,
            Ok(Some(Err(e))) => tracing::warn!("Route fee read task failed: {e}"),
            Ok(None) => break,
            // Out of time: the reads still running are dropped with the set, and their
            // hops say "fee not read".
            Err(_) => break,
        }
    }
    fees
}

/// Quote `amount_usdc` (the plan's own tranche) along every route of the table at the
/// latest block and assemble both venues. Never an error: what could not be read is
/// said in the result.
pub(crate) async fn fetch_route_quotes(
    client: &Client,
    rpc_url: &str,
    amount_usdc: &str,
    amount_usdc_raw: u128,
    spot: Option<f64>,
) -> RouteQuotes {
    fetch_route_quotes_within(client, rpc_url, amount_usdc, amount_usdc_raw, spot, &QuoteBudgets::default()).await
}

/// `fetch_route_quotes` with stated time budgets.
pub(crate) async fn fetch_route_quotes_within(
    client: &Client,
    rpc_url: &str,
    amount_usdc: &str,
    amount_usdc_raw: u128,
    spot: Option<f64>,
    budgets: &QuoteBudgets,
) -> RouteQuotes {
    let block = tokio::time::timeout(budgets.block_read, latest_block(client, rpc_url)).await.ok().flatten();
    let Some((block_number, block_time)) = block else {
        return route_quotes_unavailable(
            amount_usdc,
            "the latest block could not be read, so no quote was requested",
            spot,
        );
    };
    let block_hex = format!("0x{block_number:x}");

    // Every route at the same block, concurrently: one slow route cannot starve the rest.
    // A route whose task does not report back stays "unavailable"; it is never dropped
    // from the list.
    let mut results: Vec<RouteResult> = route_table()
        .into_iter()
        .map(|spec| RouteResult { spec, outcome: QuoteOutcome::Unavailable("the quote did not complete".to_string()) })
        .collect();
    let mut calls = tokio::task::JoinSet::new();
    for (i, r) in results.iter().enumerate() {
        let (client, rpc_url, block_hex, spec) = (client.clone(), rpc_url.to_string(), block_hex.clone(), r.spec.clone());
        let (request_timeout, attempts) = (budgets.request, budgets.attempts);
        calls.spawn(async move {
            let (to, data) = quote_request(&spec, amount_usdc_raw);
            let answer = quote_eth_call(&client, &rpc_url, to, &data, &block_hex, request_timeout, attempts).await;
            (i, outcome_from_call(&spec, answer))
        });
    }
    while let Some(joined) = calls.join_next().await {
        match joined {
            Ok((i, outcome)) => results[i].outcome = outcome,
            Err(e) => tracing::warn!("Route quote task failed: {e}"),
        }
    }

    // The quotes are in hand. The fees of the two best routes are read together, once
    // each, inside their own short budget.
    let fees = read_best_route_fees(client, rpc_url, &results, &block_hex, budgets).await;
    let venues = QUOTE_VENUES
        .iter()
        .zip(&fees)
        .map(|(venue, best_fees)| build_venue_quote(venue, &results, best_fees, amount_usdc_raw, spot))
        .collect();
    assemble_route_quotes(amount_usdc, Some((block_number, block_time)), spot, venues)
}

/// The quotes for the plan's due tranche, or None when there is nothing to quote (no
/// due tranche, or quoting switched off). It never fails the caller: when the quoters
/// could not be asked, the result says why. Quotes with at least one venue are stored,
/// so a fill recorded later can be measured against them.
pub(crate) async fn quote_due_tranche(
    state: &Arc<AppState>,
    plan: &AccumulationPlanStatus,
    spot: Option<f64>,
) -> Option<RouteQuotes> {
    let (amount_usdc, amount_raw) = tranche_to_quote(state.config.route_quotes.enabled, Some(plan))?;
    let alchemy_key = state.config.alchemy_api_keys.first().cloned().unwrap_or_default();
    if alchemy_key.is_empty() || alchemy_key == "YOUR_ALCHEMY_API_KEY" {
        return Some(route_quotes_unavailable(&amount_usdc, "no Alchemy RPC key is configured", spot));
    }
    let rpc_url = format!("https://base-mainnet.g.alchemy.com/v2/{}", alchemy_key);
    let quotes = match tokio::time::timeout(
        tokio::time::Duration::from_secs(QUOTE_TIMEOUT_SECONDS),
        fetch_route_quotes(&state.http_client, &rpc_url, &amount_usdc, amount_raw, spot),
    )
    .await
    {
        Ok(q) => q,
        Err(_) => route_quotes_unavailable(
            &amount_usdc,
            &format!("the quotes took longer than {QUOTE_TIMEOUT_SECONDS} seconds and were abandoned"),
            spot,
        ),
    };
    if let Err(e) = store_route_quotes(&state.db, &plan.plan_id, plan.tranche_index, &quotes).await {
        // The quotes are still shown; only the later cost-over-quote figure is lost.
        tracing::warn!("Route quotes could not be stored: {e}");
    }
    Some(quotes)
}

// ---------------- Stored quotes and a fill's cost over them ----------------

/// The entry that returns the most NOCK across the venues; the earlier venue wins a
/// tie. None when nothing was quoted. Pure.
pub(crate) fn best_venue_quote(quotes: &RouteQuotes) -> Option<&VenueQuote> {
    let mut best: Option<(&VenueQuote, u128)> = None;
    for v in quotes.venues.iter().filter(|v| v.status == "quoted") {
        let Some(out) = v.nock_out.as_deref().and_then(|n| units_to_raw(n, 16)) else { continue };
        if best.map_or(true, |(_, b)| out > b) {
            best = Some((v, out));
        }
    }
    best.map(|(v, _)| v)
}

/// Store one tranche's quotes, keyed by plan and block. Nothing is stored when no
/// venue was quoted or no block was read: a failed quote is never a reference.
/// Returns whether a row was written.
pub(crate) async fn store_route_quotes(
    db: &PgPool,
    plan_id: &str,
    tranche_index: u32,
    quotes: &RouteQuotes,
) -> std::result::Result<bool, sqlx::Error> {
    let (Some(best), Some(block_number), Some(quoted_at)) = (
        best_venue_quote(quotes),
        quotes.block_number,
        quotes.quoted_at.as_deref().and_then(|t| DateTime::parse_from_rfc3339(t).ok()).map(|t| t.with_timezone(&Utc)),
    ) else {
        return Ok(false);
    };
    let (Some(nock_out), Some(price), Some(route)) =
        (best.nock_out.as_ref(), best.effective_price_usd.as_ref(), best.route.as_ref())
    else {
        return Ok(false);
    };
    let res = sqlx::query(
        "INSERT INTO accumulation_quotes \
            (plan_id, tranche_index, amount_usdc, block_number, quoted_at, best_venue, best_route, best_nock_out, \
             best_effective_price_usd, quotes) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) \
         ON CONFLICT (plan_id, block_number) DO NOTHING",
    )
    .bind(plan_id)
    .bind(tranche_index as i32)
    .bind(&quotes.amount_usdc)
    .bind(block_number)
    .bind(quoted_at)
    .bind(&best.venue)
    .bind(route)
    .bind(nock_out)
    .bind(price)
    .bind(serde_json::to_value(quotes).unwrap_or(serde_json::Value::Null))
    .execute(db)
    .await?;
    Ok(res.rows_affected() > 0)
}

/// The best quote of one stored row, as a fill's reference.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct StoredQuote {
    /// The tranche the plan said was due when the quotes were read: its recorded fills
    /// at that moment, plus one.
    pub(crate) tranche_index: i32,
    pub(crate) venue: String,
    pub(crate) route: String,
    pub(crate) amount_usdc: String,
    pub(crate) nock_out: String,
    pub(crate) effective_price_usd: String,
    pub(crate) block_number: i64,
    pub(crate) quoted_at: DateTime<Utc>,
}

/// A stored quote older than this before a fill is not that fill's reference: over a
/// longer gap the figure would be price movement, not the cost of the route. While a
/// tranche is due the quotes are read every cycle, so a gap this long means quoting was
/// off or failing. The same window as the stored reference spot of a fill (issue #118).
pub(crate) const FILL_QUOTE_MAX_AGE_MINUTES: i64 = HARVEST_NOCK_PRICE_WINDOW_MINUTES;

/// Where a fill sits in its plan, as far as the ledger knows.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct FillPosition {
    pub(crate) block: i64,
    /// The fill's block time.
    pub(crate) time: DateTime<Utc>,
    /// Block of the buy before this one: the plan's latest earlier fill, or an earlier
    /// detected swap still on offer. None when this is the first.
    pub(crate) previous_buy_block: Option<i64>,
    /// The plan's recorded fills at earlier blocks: this fill is tranche
    /// `earlier_fills + 1`.
    pub(crate) earlier_fills: i64,
}

/// Whether a stored quote is the reference for a fill. All four must hold:
///   - it was read strictly before the fill's block (a read AT the fill's block already
///     includes the fill);
///   - it was not read before the buy before this one (it then belongs to an earlier
///     tranche);
///   - it was read for this fill's tranche. The plan keeps saying a tranche is due
///     after its buy lands on-chain, until that buy is recorded, and keeps quoting it.
///     Those rows carry the earlier tranche's index and never serve the next fill;
///   - it is at most `FILL_QUOTE_MAX_AGE_MINUTES` older than the fill.
/// Pure.
pub(crate) fn quote_applies_to_fill(quote: &StoredQuote, fill: &FillPosition) -> bool {
    quote.block_number < fill.block
        && fill.previous_buy_block.map_or(true, |prev| quote.block_number >= prev)
        && i64::from(quote.tranche_index) == fill.earlier_fills + 1
        && fill.time - quote.quoted_at <= chrono::Duration::minutes(FILL_QUOTE_MAX_AGE_MINUTES)
}

/// A fill's cost over the quote that was current for its tranche. All fields are None
/// together when there is no such quote: "no quote on record", never a later quote,
/// another tranche's quote or one from more than an hour before the fill.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct FillQuoteReference {
    pub(crate) venue: Option<String>,
    pub(crate) route: Option<String>,
    pub(crate) usdc: Option<String>,
    pub(crate) nock_out: Option<String>,
    pub(crate) effective_price_usd: Option<String>,
    pub(crate) block_number: Option<i64>,
    pub(crate) quoted_at: Option<DateTime<Utc>>,
    /// (fill effective price / quote effective price − 1) × 10,000. The gap between
    /// the quote's block and the fill includes price movement, so this is not
    /// execution cost alone; the quote's block is kept beside it for that reason.
    pub(crate) cost_bps: Option<i32>,
}

/// Pair a fill's effective price with its tranche's stored quote. Pure.
pub(crate) fn fill_quote_reference(effective_price_usd: &str, quote: Option<StoredQuote>) -> FillQuoteReference {
    let Some(q) = quote else {
        return FillQuoteReference::default();
    };
    let effective = effective_price_usd.parse::<f64>().unwrap_or(f64::NAN);
    let Some(bps) = execution_cost_bps(effective, q.effective_price_usd.parse::<f64>().ok()) else {
        return FillQuoteReference::default();
    };
    FillQuoteReference {
        venue: Some(q.venue),
        route: Some(q.route),
        usdc: Some(q.amount_usdc),
        nock_out: Some(q.nock_out),
        effective_price_usd: Some(q.effective_price_usd),
        block_number: Some(q.block_number),
        quoted_at: Some(q.quoted_at),
        cost_bps: Some(bps),
    }
}

/// The stored quote for a fill of `plan_id` at `fill_block` (block time `fill_time`):
/// the latest one read before that block, if `quote_applies_to_fill` accepts it. Only
/// the latest row is considered: when it was read for another tranche or is too old, an
/// older row is older still, and the answer is "no quote on record". "The buy before
/// this one" is the latest earlier fill of the plan or earlier detected swap still on
/// offer (the rule fill detection uses for its early / on-schedule label), so
/// confirming offers out of order does not hand a later buy an earlier tranche's quote.
/// A database error is logged and reads as "no quote on record"; it never blocks or
/// changes the recording.
pub(crate) async fn quote_for_fill(
    db: &PgPool,
    plan_id: &str,
    wallet: &str,
    fill_block: i64,
    fill_time: DateTime<Utc>,
) -> Option<StoredQuote> {
    let lookup = async {
        let before = sqlx::query(
            "SELECT GREATEST( \
                (SELECT MAX(block_number) FROM accumulation_events WHERE plan_id = $1 AND block_number < $2), \
                (SELECT MAX(block_number) FROM accumulation_candidates \
                  WHERE wallet = $3 AND state = 'offered' AND block_number < $2) \
             ) AS prev, \
             (SELECT COUNT(*) FROM accumulation_events WHERE plan_id = $1 AND block_number < $2) AS earlier_fills",
        )
        .bind(plan_id)
        .bind(fill_block)
        .bind(wallet.to_lowercase())
        .fetch_one(db)
        .await?;
        let fill = FillPosition {
            block: fill_block,
            time: fill_time,
            previous_buy_block: before.try_get::<Option<i64>, _>("prev")?,
            earlier_fills: before.try_get::<i64, _>("earlier_fills")?,
        };
        let row = sqlx::query(
            "SELECT tranche_index, best_venue, best_route, amount_usdc, best_nock_out, best_effective_price_usd, \
                    block_number, quoted_at \
             FROM accumulation_quotes WHERE plan_id = $1 AND block_number < $2 \
             ORDER BY block_number DESC LIMIT 1",
        )
        .bind(plan_id)
        .bind(fill_block)
        .fetch_optional(db)
        .await?;
        let Some(r) = row else {
            return Ok::<Option<StoredQuote>, sqlx::Error>(None);
        };
        let quote = StoredQuote {
            tranche_index: r.try_get("tranche_index")?,
            venue: r.try_get("best_venue")?,
            route: r.try_get("best_route")?,
            amount_usdc: r.try_get("amount_usdc")?,
            nock_out: r.try_get("best_nock_out")?,
            effective_price_usd: r.try_get("best_effective_price_usd")?,
            block_number: r.try_get("block_number")?,
            quoted_at: r.try_get("quoted_at")?,
        };
        Ok(quote_applies_to_fill(&quote, &fill).then_some(quote))
    };
    match lookup.await {
        Ok(q) => q,
        Err(e) => {
            tracing::warn!("Stored quote lookup for a fill at block {fill_block} failed: {e}");
            None
        }
    }
}

// ---------------- REST: latest and refresh ----------------

/// What GET /v1/accumulation/quotes/latest answers, from the plan status the worker is
/// serving. `route_quotes` is null unless a tranche is due and quoting is on. Pure.
pub(crate) fn latest_route_quotes_body(plan: Option<&AccumulationPlanStatus>) -> serde_json::Value {
    match plan {
        Some(p) => json!({
            "plan_id": p.plan_id,
            "status": p.status,
            "tranche_index": p.tranche_index,
            "tranche_usdc": p.tranche_usdc,
            "route_quotes": p.route_quotes,
        }),
        None => json!({ "plan_id": null, "status": null, "tranche_index": null, "tranche_usdc": null, "route_quotes": null }),
    }
}

/// GET /v1/accumulation/quotes/latest — the quotes the worker currently holds for the
/// due tranche, read from memory. It makes no call and quotes nothing. The quotes are
/// left out of the snapshot's state hash, and the snapshot's ETag is that hash, so a
/// conditional GET of the snapshot can answer 304 while the quotes have moved on; the
/// dashboard reads them here instead. Never cached.
pub(crate) async fn latest_route_quotes_handler(Extension(state): Extension<Arc<AppState>>) -> Response {
    let body = {
        let latest = state.latest_snapshot.lock().await;
        latest_route_quotes_body(latest.as_ref().and_then(|s| s.data.derived_metrics.accumulation_plan.as_ref()))
    };
    let mut headers = HeaderMap::new();
    headers.insert("cache-control", axum::http::HeaderValue::from_static("no-store"));
    (StatusCode::OK, headers, Json(body)).into_response()
}

/// Drop the quotes from a snapshot read back from the database. The stored row keeps
/// the quotes of the first cycle that produced its state hash, so after a restart they
/// can be far older than anything the worker would show; until the first cycle reads
/// new ones the plan carries none. Pure.
pub(crate) fn forget_stored_route_quotes(data: &mut SnapshotData) {
    if let Some(plan) = data.derived_metrics.accumulation_plan.as_mut() {
        plan.route_quotes = None;
    }
}

/// The reference spot for a refresh: the full-precision figure the cycle's own quotes
/// carry. None when the plan has no quotes from a cycle yet. Pure.
pub(crate) fn refresh_reference_spot(plan: &AccumulationPlanStatus) -> Option<f64> {
    plan.route_quotes
        .as_ref()
        .and_then(|q| q.reference_spot_usd.as_deref())
        .and_then(|s| s.parse::<f64>().ok())
        .filter(|s| s.is_finite() && *s > 0.0)
}

/// Why a refresh is refused, from the config and the latest snapshot. Pure.
pub(crate) fn refresh_refusal(
    plan_configured: bool,
    enabled: bool,
    has_snapshot: bool,
    plan: Option<&AccumulationPlanStatus>,
) -> Option<(StatusCode, &'static str, &'static str)> {
    if !plan_configured {
        return Some((StatusCode::CONFLICT, "NO_PLAN", "No accumulation_plan is configured, so there is no tranche to quote."));
    }
    if !enabled {
        return Some((StatusCode::CONFLICT, "QUOTES_DISABLED", "Route quotes are turned off (route_quotes.enabled is false)."));
    }
    if !has_snapshot {
        return Some((StatusCode::SERVICE_UNAVAILABLE, "NO_SNAPSHOT", "No snapshot yet, so it is not known whether a tranche is due."));
    }
    if tranche_to_quote(true, plan).is_none() {
        return Some((
            StatusCode::CONFLICT,
            "NO_TRANCHE_DUE",
            "No tranche is due. NockNock quotes only the amount of a tranche the plan says is due, never another size.",
        ));
    }
    None
}

/// POST /v1/accumulation/quotes/refresh — quote the due tranche again at the latest
/// block, for the operator about to follow a swap link. It quotes the tranche amount of
/// the latest snapshot and nothing else, takes no parameters, and writes only the
/// stored-quote row and the in-memory snapshot's `route_quotes`.
pub(crate) async fn refresh_route_quotes_handler(Extension(state): Extension<Arc<AppState>>) -> Response {
    let (has_snapshot, plan) = {
        let latest = state.latest_snapshot.lock().await;
        match latest.as_ref() {
            Some(s) => (true, s.data.derived_metrics.accumulation_plan.clone()),
            None => (false, None),
        }
    };
    if let Some((code, error, detail)) = refresh_refusal(
        state.config.accumulation_plan.is_some(),
        state.config.route_quotes.enabled,
        has_snapshot,
        plan.as_ref(),
    ) {
        return (code, Json(json!({ "error": error, "detail": detail }))).into_response();
    }
    let Some(plan) = plan else {
        return (StatusCode::CONFLICT, Json(json!({ "error": "NO_TRANCHE_DUE", "detail": "No tranche is due." }))).into_response();
    };
    // The reference stays the cycle's own spot, at the precision the cycle carried it
    // (the cycle's quotes keep it even when no venue could be quoted). Without it no
    // cost over spot is shown: the snapshot's displayed price is rounded to five
    // decimals, which is as large as the costs being measured.
    let spot = refresh_reference_spot(&plan);
    let Some(quotes) = quote_due_tranche(&state, &plan, spot).await else {
        return (StatusCode::CONFLICT, Json(json!({ "error": "NO_TRANCHE_DUE", "detail": "No tranche is due." }))).into_response();
    };
    // Attach to the snapshot being served, unless a cycle moved the plan on meanwhile.
    {
        let mut latest = state.latest_snapshot.lock().await;
        if let Some(ps) = latest.as_mut().and_then(|s| s.data.derived_metrics.accumulation_plan.as_mut()) {
            if ps.status == "tranche_due" && ps.tranche_index == plan.tranche_index && ps.tranche_usdc == plan.tranche_usdc {
                ps.route_quotes = Some(quotes.clone());
            }
        }
    }
    Json(json!({ "plan_id": plan.plan_id, "tranche_index": plan.tranche_index, "route_quotes": quotes })).into_response()
}
