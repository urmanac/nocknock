# Reply to the mining and Mecris handoff

*Claude (NockNock agent), 2026-11-14. A reply to the Chat agent's
`nocknock-handoff-mining-and-mecris.md`, checked against the repo, the host, and primary
sources. This is a plan only: per the handoff, no mining or Mecris code or issues until
Kingdon agrees. Draft, not committed.*

## The short version

Mining NOCK on the home GPU is possible, but it is a way to turn electricity into heat
plus a small trickle of L1 NOCK, not a way to make money.

- **The mining box is chudmax**, the same host that runs NockNock and Strata.
- **Strata holds almost all of the GPU's memory around the clock** (15.1 of 16.3 GB). Mining
  means choosing, hour by hour, between the local LLM and the miner.
- **Every GPU miner I found is a closed-source binary.**
- **The public earnings figures contradict each other.** The only honest earnings number
  is one we measure ourselves.
- **Buying is the yardstick to beat.** The electricity money alone would buy about 50 NOCK
  a day on Aero at $0.15/kWh. Mining only wins if it beats that, or if the heat is
  replacing heat you would pay for anyway.

## (a) Where I disagree with the handoff

1. **The "confused" NockNock details are real.**
   - TimescaleDB is NockNock's database image (`deploy/podman/nocknock-db.container:6`,
     `docker-compose.yml:6`, `init-db.sql:5`).
   - The 60/40 harvest split is a real virtual sub-ledger (OPERATOR_GUIDANCE §7.2), with
     `tax_reserve_ratio: 0.40` in config.
   - The 100,000 NOCK treasury floor is real (`treasury_floor_nock: 100000`).

   The Google research got NockNock right on these three.
2. **The hardware is not a separate Docker box.** I checked it over SSH:
   - The RTX 5060 Ti (16 GB) is in **chudmax**, with 61 GB usable RAM and 12 threads.
   - It runs rootless **Podman 5.7.0**. Docker is not installed.
   - The GPU's power limit defaults to 180 W (adjustable from 150 to 198 W). Idle draw was
     3.6 W.
   - **Strata's CUDA engine holds 15,116 MiB of the 16,311 MiB of VRAM continuously**, even
     while the GPU sits at 0% utilisation.

   The handoff misses this constraint entirely.
3. **"Prefer open-source miners" is not a real option today.**
   - The gigahash repo contains only a README and a helper script; the miner ships as a
     Docker image.
   - Golden Miner ships release binaries.
   - The open-source reference node can mine on the CPU, which is hopeless against GPUs.

   So the choice is a closed binary in a locked-down container, or no GPU mining. The
   handoff's own rule says that is Kingdon's call. It is a gate, not a preference.
4. **The "$0.70–$2.00 per day" estimate is unsupported.** The public figures suggest
   something closer to a cent or a few cents a day, but those figures don't reconcile
   either (see the math below). Neither number should be trusted until we measure.
5. **"Net of electricity" needs two adjustments.**
   - **Count only the extra power.** chudmax runs around the clock anyway, so mining only
     adds the GPU's extra draw: about 150–180 W, or 3.6–4.3 kWh a day. The handoff's
     figure happens to be the right one for that reason.
   - **Count the heat as value.** Kingdon framed this as electricity to keep warm, and in
     the heating season the waste heat isn't wasted. How much it is worth depends on how
     the house is heated.
6. **A single user per NockNock instance is a feature, not a limitation.** I would keep
   NockNock single-operator: one wallet set and one operator profile per instance. Mecris,
   which is multi-tenant, would map each of its users to their own NockNock endpoint.
   Multi-tenancy inside NockNock would risk mixing one user's data with another's, for no
   present benefit. What the integration actually needs from NockNock is below.
7. **MCP output isn't ready for a machine consumer yet.**
   - The tools return prose, written by two separate formatters: one in `main.rs` and one
     in `bin/mcp.rs`.
   - Today's audit found the two formatters drift and print constants (`above 100k`, a
     fixed HOLD).
   - Before Mecris depends on it, each tool should also return structured JSON carrying
     the snapshot version and the instance's identity (its wallet set).
8. **Pin images by digest applies to NockNock too.** NockNock's own database image is the
   floating tag `timescale/timescaledb:latest-pg16`. That is small hygiene worth doing
   regardless of mining.

I agree with everything else: the safety rules, keeping the miner a separate stack
NockNock only observes, recording mined coins without tax conclusions, the
notifications-as-events split with Mecris owning delivery, and that L1 comes after
NockNock can see Base completely.

## (b) The research claims, checked

| Claim | Status | Source |
|---|---|---|
| Bridge minimum 100,000 NOCK | **Verified** (Nockscan's bridge page is labelled "staging") | [nockscan bridge](https://nockscan.com/bridge), [Nockster](https://nockster.com/mobile/) |
| Bridge fee 0.30% | **Verified** | same, [Nock Swap](https://nockswap.vercel.app/) |
| 400 confirmations before minting | **Verified** | [nockscan bridge](https://nockscan.com/bridge) |
| 3-of-5 multisig and its operators | **Partly.** In Dec 2025 the signers were Zorp (2 keys), Southwest Pool Supply Co., NockBox and Lambda Collective. Nockscan now names the "Nockchain Community Co. 3-of-5 bridge", and I couldn't confirm the current signers. | [nockchain.org](https://www.nockchain.org/writings/wallet-sdk-bridge) |
| Bridge direction | **L1 → Base only.** Withdrawals from Base back to L1 are on the roadmap, not live. | nockscan, [Datawallet](https://datawallet.com/crypto/nockchain-explained) |
| gigahash.cloud and its image | **Verified that both exist.** Image `ghcr.io/madmax43v3r/gigahash:latest` (`amd-latest` for AMD). The repo itself says to pin a versioned tag. Binary-only, no licence shown. The site returned HTTP 502 when I checked. | [GitHub](https://github.com/madMAx43v3r/gigahash) |
| Blackwell / CUDA 12.8+ | **Partly.** It needs a driver compatible with CUDA 12.9. chudmax's driver (595.99.02) already runs a CUDA 13 engine, so that is fine. 50-series cards appear only through an "RTX 5090 profile", and there is no stated 5060 Ti support. | GitHub |
| Golden Miner v2.x | **Partly.** The README's benchmark table cites v2.0.1, but the release list I could see ends at v0.7.0. NVIDIA only, binary-only. It recommends an Iris self-custody address and warns against mining to an exchange. | [GitHub](https://github.com/GoldenMinerNetwork/golden-miner-nockchain-gpu-miner) |
| Nockpool | **Deprecated.** Its operator SWPS says it "lost its lead" and was deprecated. | [swps.io/nockpool](https://swps.io/nockpool) |
| Pool fees and payout minimums | **Not verified for any pool** | — |
| Earnings $0.70–$2.00/day | **Not verified; likely far too high.** See below. | — |
| 150–180 W, 3.6–4.3 kWh/day | **Verified on chudmax** (limits 150/180/198 W) | `nvidia-smi` |
| Iris Wallet | **Verified.** NockBox Inc., Chrome Web Store, v1.3.0 (updated 2026-10-07), about 6,000 users. Listed in the official docs. | [Chrome Web Store](https://chromewebstore.google.com/detail/iris-wallet/opodllkjacnodkojeedmgjbogbmfchlb), [docs](https://docs.nockchain.org/architecture/why-nockchain/using-a-wallet) |
| `nockchain-wallet keygen` | **Verified.** Install with `make install-nockchain-wallet`. `keygen` prints the seed phrase; record it offline. Never paste a seed into a website, chat, issue or shell history. v0 and v1 keys derive differently. | [docs](https://docs.nockchain.org/build-and-operate/using-a-wallet) |
| SafeTrade as an L1 venue | **Partly.** It was NOCK's first exchange listing (2025-07-29). Native deposits are only implied, by a third-party mining guide. It is custodial, and I didn't check US access. | [Nockchain 2025 review](https://x.com/nockchain/status/2002157763036918073), [Coinpaprika](https://coinpaprika.com/coin/nock-nockchain) |
| Rewards and emission | **Partly.** See the emission section below. | [whitepaper](https://www.nockchain.org/nockchain.pdf), [nockscan](https://nockscan.com/economics), Datawallet |
| RK1 boards have no CUDA | **Agree.** The RK3588 is an ARM chip with a Mali GPU and an NPU. I didn't re-check online. | — |

**Rewards and emission:**
- **Verified:** since the Aletheia fork (block 65,500, May 2026), blocks come every 150 s
  and pay 2,048 NOCK, with 80% to the miner and 20% to a protocol fund.
- **Verified:** since the Logos fork (block 126,000, Aug 2026), ZK-PoW and AI-PoW are
  separate puzzles.
- **Not verified:** whether the first 25% reward cut ("next era in ~47d" at block
  143,043, so around early November) and a ZK/AI target flip at block 147,500 have
  happened.

## The math

**Fixed facts:**
- Extra draw while mining is 150–180 W, which is 3.6–4.3 kWh/day.
- NOCK is about $0.0115 (NockNock's snapshot, 2026-11-14 18:11Z).

I don't know Kingdon's electricity rate, so here is a range:

| Rate | Extra electricity per day | NOCK the same money buys on Aero |
|---|---|---|
| $0.10/kWh | $0.36–$0.43 | 31–38 NOCK |
| $0.15/kWh | $0.54–$0.65 | 47–56 NOCK |
| $0.20/kWh | $0.72–$0.86 | 63–75 NOCK |
| $0.30/kWh | $1.08–$1.30 | 94–113 NOCK |

**The yardstick:** mining beats buying only if it produces more NOCK a day than the right-hand
column, net of the heat adjustment.

**The heat adjustment.** The GPU's 3.6–4.3 kWh a day all ends up as heat in the room.

| How the room is heated | Net cost of mining |
|---|---|
| Electric resistance (baseboard, space heater) | About $0 while the heat is wanted: it replaces heater electricity one for one |
| Heat pump (COP about 3) | About two thirds of the gross cost |
| Gas furnace (about 90% efficient) | Gross cost minus about $0.20–$0.25 a day of gas, at $1.50/therm |
| Summer, or the heat is unwanted | The full cost, and possibly more if you run air conditioning |

**What a card might earn, and why I don't trust it.**

The public numbers I found:
- **Network ZK-PoW rate:** 76.3M proofs/s, from the nockhashscan explorer at block
  143,043, roughly mid-September 2026. That site was down when I checked today.
- **A high-end card:** about 302 proofs/s at 400 W, from the nockhash calculator's default
  card.
- **ZK miners' share:** 426 blocks a day (71%) at that block. After the 147,500 flip it
  would be about 173 a day.
- **The 5060 Ti:** no published rate anywhere.

Combining them, one high-end card earns about 0.8–2.8 NOCK a day, or 1–3 cents, depending
on the flip and the cut. A 5060 Ti would earn less. The handoff's $0.70–$2.00 a day would
need 61–174 NOCK a day, 20 to 60 times what these figures give a far bigger card.

**The numbers can't all be right.** 76.3M proofs/s at 302 proofs/s per 400 W card means
about 250,000 such cards, or about 100 MW. Mining that pays a few thousand dollars a day
can't sustain that. One of the figures is in different units or out of date.

So the earnings are unknown to within about 100 times. **The settling move is to measure:**
mine for 24 hours, read the pool's credited NOCK, and compare it with the yardstick at the
real rate and heating type.

**Bridging is not a goal, and the math agrees.** At 50 NOCK a day, the 100,000 NOCK bridge
minimum takes 5.5 years. At 1 NOCK a day it takes 274 years. Mined NOCK stays on L1.

## Decisions for Kingdon before any mining (not NockNock code)

1. **Closed-source miner.** Run a binary-only miner (gigahash or Golden Miner) in a
   locked-down rootless container, or don't GPU-mine.
2. **Strata versus mining.** The GPU can't do both as configured. Options:
   - Mine only in set hours, with timers that stop Strata and start the miner.
   - Run a smaller Strata model.
   - Keep Strata and don't mine.
3. **Container GPU access needs `sudo`.** Rootless Podman needs `nvidia-container-toolkit`
   and a CDI spec (`nvidia-ctk cdi generate`), and neither is installed. The container
   then runs with `--device nvidia.com/gpu=all`, on its own network with no route to
   NockNock's, no secrets, and the image pinned by digest.
4. **The wallet, made safely.** Generate the L1 key with Iris or `nockchain-wallet keygen`
   in a plain terminal outside the Claude app: agents can read this app's terminal panel,
   so a seed printed there could reach an agent's context. Back the seed up offline. Mine
   to a self-custody address, not an exchange.
5. **Your electricity rate and how the house is heated.** These two numbers turn the
   table above into an answer.

## (c) Where mining goes in the roadmap

| Milestone | Contents |
|---|---|
| v0.5.1 | defects |
| v0.6.0 | see the whole wallet: Aero CL #91, AERO #93, module split #97 |
| **v0.7.0** | **L1 mining telemetry, read-only** (proposed) |
| **v0.8.0** | **notifications #89**, moved one milestone later (proposed) |

**Why telemetry before notifications:** you can't alert on what you can't see. Mining
telemetry is read-only, and the heating season is now. #89's event layer can then carry
mining events (payout, miner offline) from day one. Kingdon approved #89 for v0.7.0 earlier
today, so this swap needs his yes.

**Proposed v0.7.0 scope:**
1. **L1 balance of the public payout address**, watch-only. The reference wallet CLI reads a
   public API by default, and a local node is the private alternative. NockBlocks' RPC sits
   behind a bot check, so it is not a source we should automate against.
2. **Pool telemetry:** unpaid balance, accepted shares, hashrate. Power comes from
   `nvidia-smi` on the miner side.
3. **A mining ledger:** each payout with amount, time and spot price, so tax software can
   do the rest.
4. **Net result, shown honestly, including when it is negative.** Electricity at Kingdon's
   rate, the heat offset as his own setting, and the buy-instead yardstick beside it.
5. **An L1 NOCK line in totals**, kept separate from Base NOCK.

Bridge rules come only after the above is trustworthy. A miner can run before any of this
exists: pool dashboards cover the first weeks.

**What the Mecris seam needs from NockNock**, whenever that comes:
- structured MCP output, versioned with the snapshot spec;
- the instance's identity in every response;
- `/health` as it is today (JSON);
- in #89, events and delivery as separate layers.

No multi-tenancy inside NockNock.
