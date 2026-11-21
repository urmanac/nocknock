-- Positions found on-chain and tracked without a config.yaml entry (v0.9.0; #145 for
-- Uniswap v4). Three tables, and only what detection and database-tracked positions use.
-- The in-app deploy flow's attempt table is a later migration of its own.
--
-- Like the rest of this database, all three are disposable: the wallet and the chain are
-- the record. After a loss the next cycles read the wallet's position NFTs from the chain
-- again. What is lost with the tables is the operator's taps: a position tracked or
-- dismissed with one tap is offered again.
--
-- Vocabulary columns (role, rung, source, verdict, choice) are checked in code, not by a
-- CHECK constraint, so a later release can add a value without a migration.

-- A position counted in every cycle, appended after config.yaml's entries. A config entry
-- for the same position wins (position_key is adapters::position_key).
CREATE TABLE IF NOT EXISTS tracked_positions (
    wallet TEXT NOT NULL,
    position_key TEXT NOT NULL,
    protocol TEXT NOT NULL,
    network TEXT NOT NULL,
    nft_id BIGINT NOT NULL CHECK (nft_id >= 0),
    -- Uniswap v4: the bytes32 pool id (from the mint's ModifyLiquidity event, checked
    -- against the 25 bytes the position manager stores), the pool key's currencies in
    -- order, and the NFT's own ticks.
    pool_id TEXT NOT NULL,
    currency0 TEXT NOT NULL,
    currency1 TEXT NOT NULL,
    tick_lower INTEGER NOT NULL,
    tick_upper INTEGER NOT NULL,
    tick_spacing INTEGER,
    -- 'ladder' or NULL (classified by its pair, never a rebuild target).
    role TEXT,
    -- A ladder's rung, or NULL when none was named (stated as unset, never assumed).
    rung TEXT,
    -- The mint transaction detection found: the ladder's baseline (#167). NULL when the
    -- NFT reached the wallet by transfer.
    deposit_tx TEXT,
    mint_block BIGINT,
    -- 'detected' (matched a suggested rung exactly) or 'confirmed' (the operator's tap).
    source TEXT NOT NULL,
    -- What a ladder rung stands for after it is withdrawn from, burned or sent away
    -- (third review of the ladder bottle): the liquidity its verified mint receipt added,
    -- and the most liquidity a cycle has read it holding. Decimal strings of a uint128.
    -- NULL until known. deposit_liquidity comes back after a database loss (the mint
    -- receipt is read again); max_liquidity_seen does not.
    deposit_liquidity TEXT,
    max_liquidity_seen TEXT,
    tracked_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (wallet, position_key)
);

-- Every position NFT seen arriving in the wallet, with what the chain said about it when
-- it was last read. Never a tracked position and never in a total by itself.
--   verdict  eligible         whitelisted pool, no hooks: may be offered or tracked
--            not_whitelisted  a token outside the whitelist: never offered (dust)
--            hooks            a hooked pool: never offered
--            unsupported      a whitelisted token NockNock cannot read in a position yet
--            gone             the NFT does not exist
--   choice   NULL, or 'dismissed' (the operator's "not now", or an untrack): kept, so it
--            is not tracked again by itself, and restorable.
CREATE TABLE IF NOT EXISTS position_candidates (
    wallet TEXT NOT NULL,
    protocol TEXT NOT NULL,
    network TEXT NOT NULL,
    nft_id BIGINT NOT NULL CHECK (nft_id >= 0),
    verdict TEXT NOT NULL,
    choice TEXT,
    owned BOOLEAN NOT NULL,
    has_liquidity BOOLEAN,
    pool_id TEXT,
    pool_id_prefix TEXT,
    currency0 TEXT,
    currency1 TEXT,
    fee_raw BIGINT,
    tick_spacing INTEGER,
    hooks TEXT,
    tick_lower INTEGER,
    tick_upper INTEGER,
    -- The transfer that last brought it to the wallet: a hint, checked on-chain.
    received_tx TEXT,
    received_block BIGINT,
    received_from TEXT,
    -- From that transaction's receipt, when it mints this NFT to the wallet.
    mint_tx TEXT,
    mint_block BIGINT,
    minted_by TEXT,
    -- parse_ladder_deposit's verdict on the mint receipt (#167): 'ok', or its refusal.
    -- NULL when the receipt was not read or the NFT was not minted to the wallet.
    deposit_check TEXT,
    first_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (wallet, protocol, network, nft_id)
);

-- How far the wallet's position-NFT transfers were read, so a scan resumes where it
-- stopped and a backfill fits an RPC's log-range limit. logs_from_block..logs_to_block
-- is one range read without a gap. history_complete_at is when the indexed transfer
-- history was last read to its end.
CREATE TABLE IF NOT EXISTS position_detection_state (
    wallet TEXT NOT NULL,
    protocol TEXT NOT NULL,
    network TEXT NOT NULL,
    logs_from_block BIGINT,
    logs_to_block BIGINT,
    history_complete_at TIMESTAMPTZ,
    last_checked_at TIMESTAMPTZ,
    -- Where the next cycle's re-reads of untracked NFTs start: it moves by the number read,
    -- so a cycle cut short by its time budget resumes where it stopped.
    refresh_cursor BIGINT NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (wallet, protocol, network)
);
