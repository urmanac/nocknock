use crate::*;

// ============================================================
// Closed positions — issue #144
// ============================================================
// A configured position whose NFT no longer exists (the operator withdrew it and the
// wallet app burned the NFT) used to fail every ingestion cycle until its entry was
// removed from config.yaml. It is now reported as CLOSED and the cycle goes on.
//
// "Closed" is a claim about the chain, and a wrong one silently removes real money from
// the total, so it is made only on proof:
//   - the position manager is one whose answers for a nonexistent token id were read
//     on-chain (ONCHAIN_INTEGRATION.md §13) and are listed in adapters/mod.rs;
//   - TWO independent reads of the same cycle both gave that contract's own "no such
//     token" answer, decoded strictly from the raw revert data;
//   - everything else fails the cycle exactly as before: a transport failure, an HTTP
//     error, a timeout, a rate limit, a JSON-RPC error that is not an execution revert,
//     a revert with another reason, with empty data or with no data, and one read
//     saying "nonexistent" while the other does not.
//
// A never-minted id answers exactly like a burned one, so the text always says both:
// "withdrawn and burned, or a mistyped nft_id". An NFT that exists with zero liquidity
// is NOT closed: it is still a live position, holding nothing.
//
// A closed position is never in `positions`, `pool_mechanics`, `contract_fees`, the
// velocity rows or the tick brackets (all built from the live positions, so they stay
// index-aligned), and nothing of it enters a total, a pillar, an LP weight, a rebuild
// suggestion, a claim action or a reward figure. It is listed in `closed_positions`.

/// What one configured position's reads said this cycle.
#[derive(Debug)]
pub(crate) enum PositionRead<T> {
    Live(T),
    /// Proven closed: the position manager it was read on, and the reads that proved it.
    Closed(ClosedProof),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ClosedProof {
    pub(crate) position_manager: String,
    pub(crate) evidence: Vec<ClosedEvidence>,
}

/// The verdict of the two reads. `NotProven` carries the reason, for the cycle error.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ClosureVerdict {
    Closed(Vec<ClosedEvidence>),
    NotProven(String),
}

pub(crate) const ERC721_OWNER_OF_SIGNATURE: &str = "ownerOf(uint256)";
pub(crate) const NPM_POSITIONS_SIGNATURE: &str = "positions(uint256)";
pub(crate) const V4_POOL_AND_POSITION_INFO_SIGNATURE: &str = "getPoolAndPositionInfo(uint256)";
/// getPoolAndPositionInfo(uint256) on the Uniswap v4 PositionManager.
pub(crate) const V4_GET_POOL_AND_POSITION_INFO: &str = "0x7ba03aad";
/// Its return: a PoolKey (5 words) and the packed PositionInfo (1 word).
pub(crate) const V4_POOL_AND_POSITION_INFO_HEX_LEN: usize = 6 * 64;

/// An outcome in words, for an error message. Never contains a URL.
pub(crate) fn describe_outcome(outcome: &CallOutcome) -> String {
    match outcome {
        CallOutcome::Returned(hex) => format!("returned {} bytes of data", hex.len() / 2),
        CallOutcome::Reverted(data) => format!("reverted with {}", decode_revert_reason(data.as_deref()).describe()),
    }
}

fn evidence(call: &str, contract: &str, outcome: &CallOutcome) -> ClosedEvidence {
    let (kind, answer, raw) = match outcome {
        CallOutcome::Returned(hex) => (
            "returned",
            if hex.bytes().all(|b| b == b'0') {
                format!("{} zero bytes: no pool and no position info is stored for this id", hex.len() / 2)
            } else {
                format!("{} bytes of data", hex.len() / 2)
            },
            hex.clone(),
        ),
        CallOutcome::Reverted(data) => {
            ("reverted", decode_revert_reason(data.as_deref()).describe(), data.clone().unwrap_or_default())
        }
    };
    ClosedEvidence {
        call: call.to_string(),
        contract: contract.to_lowercase(),
        outcome: kind.to_string(),
        answer,
        raw: format!("0x{raw}"),
    }
}

/// True only for a revert whose data decodes, strictly, to `Error(expected)`.
fn reverted_with(outcome: &CallOutcome, expected: &str) -> bool {
    match outcome {
        CallOutcome::Reverted(data) => decode_revert_reason(data.as_deref()) == RevertReason::Message(expected.to_string()),
        CallOutcome::Returned(_) => false,
    }
}

/// Aero CL and Uniswap v3: is the NFT proven not to exist? Pure.
/// `positions` is the outcome of `positions(tokenId)` and `owner_of` of
/// `ownerOf(tokenId)`, both on `manager`, both read this cycle. Closed only when the
/// manager is on the allow-list (adapters::nft_nonexistence_rule) and BOTH reverted
/// with that manager's own reason.
pub(crate) fn nft_closure_verdict(
    protocol: &str,
    manager: &str,
    positions: &CallOutcome,
    owner_of: &CallOutcome,
) -> ClosureVerdict {
    let Some(rule) = nft_nonexistence_rule(protocol, manager) else {
        return ClosureVerdict::NotProven(format!(
            "positions() {}, and position manager {manager} is not one whose answers for a nonexistent token id have been verified, so this is not read as a closed position",
            describe_outcome(positions)
        ));
    };
    if !reverted_with(positions, rule.positions_reason) {
        return ClosureVerdict::NotProven(format!(
            "positions() {}, which is not this position manager's answer for a token id that does not exist (Error(\"{}\"))",
            describe_outcome(positions),
            rule.positions_reason
        ));
    }
    if !reverted_with(owner_of, rule.owner_of_reason) {
        return ClosureVerdict::NotProven(format!(
            "positions() {} but ownerOf() {}: the two reads do not agree that the token id does not exist (ownerOf must answer Error(\"{}\"))",
            describe_outcome(positions),
            describe_outcome(owner_of),
            rule.owner_of_reason
        ));
    }
    ClosureVerdict::Closed(vec![
        evidence(NPM_POSITIONS_SIGNATURE, manager, positions),
        evidence(ERC721_OWNER_OF_SIGNATURE, manager, owner_of),
    ])
}

/// Uniswap v4: is the NFT proven not to exist? Pure. The PositionManager does not
/// revert on its own reads for a burned id (getPositionLiquidity answers 0, as it does
/// for an NFT that exists and is empty), so the proof is `ownerOf` reverting with
/// Error("NOT_MINTED") AND `getPoolAndPositionInfo` returning exactly six zero words
/// (an NFT that exists, even empty, has its pool key and position info stored).
pub(crate) fn v4_closure_verdict(owner_of: &CallOutcome, pool_and_position_info: &CallOutcome) -> ClosureVerdict {
    if !reverted_with(owner_of, V4_OWNER_OF_NONEXISTENT) {
        return ClosureVerdict::NotProven(format!(
            "ownerOf() {}, which is not the PositionManager's answer for a token id that does not exist (Error(\"{V4_OWNER_OF_NONEXISTENT}\"))",
            describe_outcome(owner_of)
        ));
    }
    let info_is_empty = matches!(
        pool_and_position_info,
        CallOutcome::Returned(hex) if hex.len() == V4_POOL_AND_POSITION_INFO_HEX_LEN && hex.bytes().all(|b| b == b'0')
    );
    if !info_is_empty {
        return ClosureVerdict::NotProven(format!(
            "ownerOf() {} but getPoolAndPositionInfo() {}, not the six zero words of a token id that does not exist: the two reads do not agree",
            describe_outcome(owner_of),
            describe_outcome(pool_and_position_info)
        ));
    }
    ClosureVerdict::Closed(vec![
        evidence(ERC721_OWNER_OF_SIGNATURE, UNISWAP_V4_POSM_BASE, owner_of),
        evidence(V4_POOL_AND_POSITION_INFO_SIGNATURE, UNISWAP_V4_POSM_BASE, pool_and_position_info),
    ])
}

/// The fact a closed position states. A never-minted id answers like a burned one, so
/// both are named. Pure.
pub(crate) fn closed_reason(nft_id: u64, manager: &str) -> String {
    format!(
        "NFT #{nft_id} does not exist on position manager {}: withdrawn and burned, or a mistyped nft_id",
        manager.to_lowercase()
    )
}

/// The snapshot entry for a config entry proven closed. Pure; built from config and the
/// proof alone. `first_seen_closed_at` is stamped by the cycle (stamp_first_seen_closed).
pub(crate) fn closed_position_entry(cfg: &PositionConfig, proof: ClosedProof, supplies_spot_price: bool) -> ClosedPosition {
    ClosedPosition {
        config_id: cfg.id.clone(),
        position_id: position_key(cfg),
        protocol: cfg.protocol.clone(),
        network: cfg.network.clone(),
        nft_id: cfg.nft_id,
        pool_id: closed_position_pool(cfg),
        reason: closed_reason(cfg.nft_id, &proof.position_manager),
        position_manager: proof.position_manager.to_lowercase(),
        label: closed_position_label(cfg),
        status: "closed".to_string(),
        evidence: proof.evidence,
        first_seen_closed_at: None,
        supplies_spot_price,
    }
}

/// The cycle error when the position flagged `is_price_source` is closed and its pool
/// cannot go on supplying the spot (Uniswap v3, Aero CL: the pool's tokens are read
/// from the NFT, which is gone). No price is guessed; the message names the fix. Pure.
pub(crate) fn closed_price_source_error(cfg: &PositionConfig, manager: &str) -> anyhow::Error {
    anyhow::anyhow!(
        "Configuration error: position {} is the NOCK/USD price source (is_price_source: true) and is closed: {}. The pool of an entry with protocol {} is identified through its NFT, so no price is read from it and none is guessed. Flag another position as is_price_source: true in config.yaml",
        cfg.id,
        closed_reason(cfg.nft_id, manager),
        cfg.protocol
    )
}

/// The cycle error when no configured position is live. As with no positions configured
/// at all (issue #119), there is nothing to report positions from. Pure.
pub(crate) fn all_positions_closed_error(closed: &[ClosedPosition]) -> anyhow::Error {
    anyhow::anyhow!(
        "No live positions: every configured position is closed ({}). Remove the closed entries and add your current positions under `positions:` in config.yaml (see ONCHAIN_INTEGRATION.md §5)",
        closed.iter().map(|c| format!("{}: {}", c.config_id, c.reason)).collect::<Vec<_>>().join("; ")
    )
}

/// `global_status.closed_positions_note`: one statement every surface can print. None
/// when no position is closed. Pure.
pub(crate) fn closed_positions_note(closed: &[ClosedPosition]) -> Option<String> {
    if closed.is_empty() {
        return None;
    }
    let one = closed.len() == 1;
    let list = closed
        .iter()
        .map(|c| format!("{} (NFT #{} no longer exists: withdrawn and burned, or a mistyped nft_id)", c.label, c.nft_id))
        .collect::<Vec<_>>()
        .join("; ");
    let mut note = format!(
        "{} configured position{} closed and {} not counted in any total: {}. {} can be removed from config.yaml.",
        closed.len(),
        if one { " is" } else { "s are" },
        if one { "is" } else { "are" },
        list,
        if one { "Its entry" } else { "Their entries" },
    );
    for c in closed.iter().filter(|c| c.supplies_spot_price) {
        note.push_str(&format!(
            " The pool of {} still supplies the NOCK/USD spot: flag another position as is_price_source before removing that entry.",
            c.label
        ));
    }
    Some(note)
}

/// Stamp when this worker first recorded each closed position. Pure. An entry the
/// previous snapshot already listed keeps that snapshot's time (also a null one: a
/// time is never invented afterwards); a new one gets this cycle's time.
pub(crate) fn stamp_first_seen_closed(closed: &mut [ClosedPosition], prev: Option<&[ClosedPosition]>, now: &str) {
    for c in closed.iter_mut() {
        c.first_seen_closed_at = match prev.and_then(|p| p.iter().find(|x| x.position_id == c.position_id)) {
            Some(before) => before.first_seen_closed_at.clone(),
            None => Some(now.to_string()),
        };
    }
}

/// The sentences to push for positions that were live in the previous snapshot and are
/// closed in this one. Pure. Nothing on a first cycle (no previous snapshot), nothing
/// for an entry the previous snapshot already listed as closed (every later cycle, and
/// a restart, since the worker reloads its last snapshot), and nothing for an entry the
/// previous snapshot did not hold as a live position (a new entry with a mistyped id
/// was never a tracked position; the dashboard says it).
pub(crate) fn closed_position_notes(prev_live_position_ids: Option<&[String]>, closed: &[ClosedPosition]) -> Vec<String> {
    let Some(prev_live) = prev_live_position_ids else {
        return Vec::new();
    };
    closed
        .iter()
        .filter(|c| prev_live.iter().any(|id| id == &c.position_id))
        .map(|c| {
            let mut text = format!(
                "Position closed: {}. NFT #{} no longer exists on {} (withdrawn and burned, or a mistyped nft_id). It is no longer counted in any total. Its entry ({}) can be removed from config.yaml.",
                c.label, c.nft_id, c.position_manager, c.config_id
            );
            if c.supplies_spot_price {
                text.push_str(" Its pool still supplies the NOCK/USD spot: flag another position as is_price_source before removing the entry.");
            }
            text
        })
        .collect()
}

/// The config entries the latest snapshot does not list as closed, in config order.
/// The claim payload builds an action only for these. Pure.
pub(crate) fn live_position_configs<'a>(configs: &'a [PositionConfig], closed: &[ClosedPosition]) -> Vec<&'a PositionConfig> {
    configs
        .iter()
        .filter(|c| {
            let key = position_key(c);
            !closed.iter().any(|x| x.position_id == key)
        })
        .collect()
}
