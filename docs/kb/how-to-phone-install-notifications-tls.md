# How to: NockNock on your phone (install, notifications, HTTPS)

*Issue [#89](https://github.com/urmanac/nocknock/issues/89). Written 2026-11-15 for the
rootless Podman deployment (`DEPLOYMENT.md`, Option D). Examples use the host name
`chudmax`; use your own.*

Three separate things, each off until you turn it on:

| Part | What you get | What turns it on |
|---|---|---|
| 1. Install | A home-screen icon that opens the Lite cockpit | Deploy this release, then "Add to Home Screen" on the phone |
| 2. Notifications | A push when the directive changes or a plan completes | A `notifications:` block in `config.yaml` |
| 3. HTTPS | `https://<host>:3443` with a certificate from a private CA | `NOCKNOCK_TLS=1` on the deploy command |

Nothing here was tried on a real phone or on the host. [What was checked, and what was
not](#what-was-checked-and-what-was-not) lists both precisely. Read it before relying
on any step.

---

## The short checklist

On the workstation:

1. Add a `notifications:` block to `ingestion-worker/config.yaml` ([part 2](#2-notifications)).
2. Deploy: `deploy/podman/deploy.sh chudmax`.
3. Check the worker sees the channel:
   `curl -s http://chudmax:8081/v1/notifications/status`

On the phone, on the same network:

4. Open `http://chudmax:3000/?view=lite`. If the name does not resolve on the phone,
   use the host's IP address.
5. The DIRECTIVE card at the top must end with `LIVE ON-CHAIN · read …s ago`.
6. Add it to the home screen. iPhone: Safari, Share, "Add to Home Screen". Android:
   Chrome, ⋮ menu, "Add to Home screen".
7. Open it from the home screen. It should open the Lite cockpit, not the full
   dashboard.
8. Scroll to NOTIFICATIONS. Your channel should read `configured`. Tap
   **SEND TEST NOTIFICATION**.
9. The phone should show a message starting `🧪 NockNock TEST notification`. Tap its
   link: it should open the Lite cockpit.

If step 8 says a channel `FAILED`, the reason from the provider is shown next to it
(for example `HTTP 401: Unauthorized` means the Telegram token is wrong).

HTTPS (part 3) is optional and has a cost. Read [part 3](#3-https-on-the-lan) first.

---

## 1. Install

The dashboard now ships a web app manifest, icons, and the meta tags iOS and Android
read for "Add to Home Screen". The start URL is `/?view=lite`.

**There is no service worker, on purpose.** A service worker could answer a `/v1`
request from a cache and show an old snapshot as if it were live. NockNock caches
nothing: an installed copy reads the worker every time, and when it cannot reach the
worker it says so. The cost is that the app does not open offline or away from the
LAN. It shows the browser's own "cannot connect" page instead.

An app that is already open when the phone leaves the LAN keeps the last snapshot on
the page. The DIRECTIVE card polls every 10 s, and after the first poll that fails it
reads `NOT LIVE · Worker unreachable: … · last read 50s ago`, however recent that last
read is. It reads the worker again as soon as the app comes back to the foreground.

The Lite cockpit gained what a phone needs first:

- **DIRECTIVE**: the current directive and its details, then one line saying whether
  the page is live. A snapshot that has stopped refreshing reads
  `NOT LIVE · last read 12m ago. Everything on this page is from that read.`
  A failed poll adds the cause, as above.
- **ACCUMULATION PLAN**, when a plan is configured: the same panel as the full
  dashboard, with RECORD BUY. A "DCA TRANCHE DUE" notification now lands on the
  tranche it is about.
- **ON YOUR PHONE**: the install steps for the device you are holding.
- **NOTIFICATIONS**: the channel status and the test button.

What to expect per platform (expected behaviour, not tested here):

- **iPhone and iPad.** "Add to Home Screen" works over plain HTTP. The icon opens
  the app without Safari's address bar.
- **Android.** Chrome installs a web app only from an HTTPS address. Over plain HTTP,
  "Add to Home screen" makes a bookmark that opens in a normal browser tab. For a
  real install, turn on part 3 and add it from the `https://` address.
- An installed app belongs to the address it was added from.
  `http://chudmax:3000` and `https://chudmax:3443` are two different apps to the
  phone. If you turn HTTPS on later, add it again from the new address.

**The icon** is a crop of the "N" roundel in `assets/image_6cc4c3c4.jpg`, scaled up.
The repo holds no logo, only that concept art. `frontend/scripts/make-icons.sh`
regenerates the icons (macOS, uses `sips`); to use a real logo, point its `SRC` at a
square image.

---

## 2. Notifications

### When the worker notifies

- When the directive **changes** between two sync cycles. Examples:
  `DCA ON SCHEDULE (2 of 5 next)` to `DCA TRANCHE DUE (2 of 5)`;
  `HOLD & ACCRETE …` to `DEFENSIVE LOCK (Floor Recovery)`;
  anything to `RANGE EXIT FLASHPOINT (Above Range)`.
- When an accumulation plan **completes** (new in this release). The message carries
  the plan's own figures: budget deployed, number of verified buys, average cost.
- When spot **reaches the top of the price-source range** (a range exit), whatever the
  directive reads (new in this release). A floor lock and a due tranche both outrank
  `RANGE EXIT FLASHPOINT` in the directive, so the action does not change then; the
  push says `Range exit: spot $… is at or above the top of the price-source range
  ($…)` and `Directive unchanged: …`. It states the two prices and leaves the choice
  to you.

A plan completing or a range exit that arrives together with a directive change is
one message, with its sentence placed before the directive's details so a provider's
length cut does not remove it.

It does not notify on a repeat, between `HOLD & ACCRETE` variants, or on the first
cycle of a worker that has no earlier snapshot.

Known gaps:

- A failed ingestion cycle sends nothing. `/health` shows it; your phone does not.
- While LP is under target and a deposit is fundable, every idle plan state reads
  `REBUILD LP TRANCHES`. A change between ON SCHEDULE, PAUSED and BLOCKED sends
  nothing then. A tranche becoming due always notifies.
- Spot falling **back into** the price-source range is pushed only when it changes the
  directive (it does when the directive was `RANGE EXIT FLASHPOINT`). Under a floor
  lock or a due tranche the exit is pushed and the return is not.
- The worker remembers the last range reading in memory only. A range exit that
  happens across a worker restart is pushed only if it changes the directive.
- In a cycle where the accumulation ledger cannot be read, the directive becomes
  `DCA BLOCKED` and that change is pushed. Its text gives the plan's progress as
  `$0.00 … 0 verified buy(s)`, which is wrong: the progress is unknown that cycle.
  This is older than this release and is not fixed here. The next good cycle pushes
  the directive changing back; it no longer announces an already completed plan as
  complete a second time.

### Telegram

1. In Telegram, open a chat with **BotFather** (the verified account with that
   username). Send `/newbot`. Choose a display name, then a username ending in `bot`.
2. BotFather replies with a token that looks like `123456789:AAExampleExample`.
   That is `telegram_bot_token`.
3. Open a chat with your new bot and send it any message. A bot cannot message you
   until you have.
4. Find your chat id. From the workstation:
   ```bash
   curl -s "https://api.telegram.org/bot<token>/getUpdates"
   ```
   In the answer, `result[0].message.chat.id` is `telegram_chat_id`. If `result` is
   empty, send the bot another message and run it again.

```yaml
notifications:
  telegram_bot_token: "123456789:AAExampleExample"
  telegram_chat_id: "123456789"
  cockpit_url: "http://chudmax:3000/?view=lite"
```

### Pushover

1. Create an account at pushover.net and install the Pushover app on the phone.
   Pushover is a paid app after a trial; check its current terms.
2. After logging in on the website, **Your User Key** on the front page is
   `pushover_user`.
3. Choose **Create an Application/API Token**, name it (for example "NockNock").
   The **API Token/Key** it shows is `pushover_token`.

```yaml
notifications:
  pushover_token: "your-application-api-token"
  pushover_user: "your-user-key"
  cockpit_url: "http://chudmax:3000/?view=lite"
```

`pushover_user_key` is accepted as another spelling of `pushover_user`. The example
config used that name until v0.5.3 while the worker read only `pushover_user`, so a
copied example left Pushover off without saying so. A config that carries both keys
loads; `pushover_user` is the one used, and the status route says the other is ignored.

### Webhook

`webhook_url` receives `POST {"text": "..."}` as JSON. Treat the URL as a secret.

### Notes on the block

- You can set more than one channel. Each is sent independently.
- A long message is cut to the provider's limit (Pushover: 1024 characters) with a
  "…", and the link is kept. The full text is on the page the link opens. The test
  message is short, so it passing says nothing about length; that is why every
  message is cut the same way before it is sent.
- `cockpit_url` is the link at the end of each message. The Lite cockpit is
  `/?view=lite`. A `/lite` path opens the full dashboard; the status route and the
  dashboard panel warn when they see one.
- Half a channel (a token without its chat id, say) sends nothing. It is reported as
  `incomplete`, with the missing key named.
- The block goes in `ingestion-worker/config.yaml` on the workstation. That file is
  gitignored and holds your Alchemy key already. `deploy.sh` stores it on the host as
  the Podman secret `nocknock-config`. Back it up before editing. Do not put
  credentials in `config.yaml.example`: that file is the base layer of every
  deployment's config.
- The worker needs to reach `api.telegram.org` or `api.pushover.net`. It already
  reaches the internet for the RPC provider.

### Test it

After deploying:

```bash
curl -s http://chudmax:8081/v1/notifications/status        # what is configured
curl -s -X POST http://chudmax:8081/v1/notifications/test  # send one TEST message
```

The dashboard's **SEND TEST NOTIFICATION** button (full dashboard, last panel; Lite
cockpit, bottom card) does the same. The answer lists each channel:

```json
{ "sent": true, "channels": [
  { "channel": "telegram", "status": "delivered", "detail": "accepted by Telegram (HTTP 200)" },
  { "channel": "pushover", "status": "failed", "detail": "HTTP 400: application token is invalid" },
  { "channel": "webhook", "status": "not_configured", "detail": "notifications.webhook_url is not set" } ] }
```

- `delivered` means the provider accepted the message. Whether the phone showed it is
  for you to see.
- With no channel configured the answer is `"sent": false` and a sentence saying so.
  Nothing is sent.
- The test message says TEST in its first line and that nothing changed. It cannot be
  mistaken for a directive.
- No token, user key or webhook URL appears in any response or log line.
- The route has no authentication beyond reaching the worker on the LAN, like every
  other route. Test sends are limited to one every 30 seconds.

---

## 3. HTTPS on the LAN

Plain HTTP on `:3000` keeps working whether or not you do this. HTTPS matters for two
things: Android will only install a web app from an HTTPS address, and browser
features that wallets rely on expect a secure origin.

A certificate that a phone trusts without any setup has to come from a public
certificate authority, and a public CA only certifies a name it can check. A LAN-only
host has no such name. There are three ways around that. One is built here because it
needs no account. The other two are described so you can choose.

### Option A (built, opt-in): Caddy with its own private CA

```bash
NOCKNOCK_TLS=1 NOCKNOCK_TLS_NAMES="chudmax 192.168.1.20" deploy/podman/deploy.sh chudmax
```

- `NOCKNOCK_TLS_NAMES`: every name or IPv4 address you will type on the phone,
  separated by spaces. The certificate covers these and nothing else. Default: the
  host argument.
- `NOCKNOCK_TLS_PORT`: the host port. Default `3443`. Ports below 1024 are refused
  (rootless Podman cannot publish them), as are 8080 (Strata), 3000 and 8081. Write
  it as a plain number: a leading zero (`08080`) is refused.

This adds one more rootless quadlet, `nocknock-tls`: a Caddy container on the
`nocknock` network that forwards to the dashboard container. It needs no `sudo` to
install or to keep running. The first start pulls `docker.io/library/caddy:2`.

Then, on each phone:

1. Open `https://chudmax:3443/nocknock-root-ca.crt`. The browser warns that the
   connection is not trusted, which is true at this point. Continue and download the
   file. (Or copy it over another way: `deploy/podman/tls/export-root-ca.sh chudmax`
   writes it to the workstation.)
2. Compare the certificate's SHA-256 fingerprint on the phone with the one
   `deploy.sh` printed. If they differ, stop.
3. Install and trust it.
   - **iPhone**: Settings, "Profile Downloaded", Install. Then Settings, General,
     About, Certificate Trust Settings, and switch on full trust for it.
   - **Android**: Settings, Security, Encryption & credentials, Install a
     certificate, CA certificate, and pick the file. Menu names vary by maker.
4. Open `https://chudmax:3443/?view=lite`. No warning should appear now.
5. If you use notifications, change `cockpit_url` to the `https://` address and
   redeploy.

**What this costs. Read this part.**

- **The root certificate you install can vouch for any website, not just NockNock.**
  Its private key sits in the `nocknock-tls-data` Podman volume on the host,
  readable by your user account there. Someone who copies that key and can get
  between your phone and the internet could impersonate any HTTPS site to that phone.
  Install it only on devices you own. Remove it from the phone when you stop using
  this.
- Every device needs the certificate installed by hand. A guest's phone, or a new
  phone, sees a warning until you do it.
- Android shows a standing notice that the network may be monitored once a user
  certificate is installed. Apps other than the browser generally ignore user
  certificates.
- The address carries a port: `https://chudmax:3443`.
- The phone has to resolve the name you chose. If it cannot, list the host's IP
  address in `NOCKNOCK_TLS_NAMES` and use that.
- Ports 3000 and 8081 still serve plain HTTP on the LAN. This adds an encrypted
  door; it does not close the others.
- No HSTS header is sent, deliberately. HSTS applies to a whole hostname regardless
  of port, so it would break `http://chudmax:3000`, `http://chudmax:8081` and
  anything else that host serves over plain HTTP.

**Operating it**

```bash
systemctl --user status nocknock-tls          # on the host
journalctl --user -u nocknock-tls -f
NOCKNOCK_TLS=remove deploy/podman/deploy.sh chudmax   # stop it and remove its quadlet
```

- A later deploy without `NOCKNOCK_TLS` leaves the proxy exactly as it is: not
  restarted, not removed.
- `remove` keeps the `nocknock-tls-data` volume, so turning HTTPS on again reuses the
  same root and phones need nothing new. To destroy the CA for good, on the host:
  `podman volume rm nocknock-tls-data`. Then remove the certificate from each phone.
- Caddy renews the site certificate itself. By Caddy's documented defaults the root
  lasts about ten years.

### Option B (not built): a tailnet

Put the host and the phone on a Tailscale network and let Tailscale issue a
certificate for the host's `…ts.net` name.

- For: a publicly trusted certificate, so nothing to install on the phone beyond the
  Tailscale app. It also reaches the dashboard away from home without exposing the
  host to the internet, which is the remaining item in #89.
- Against: needs a Tailscale account, the app on every phone, and Tailscale
  installed on the host (a `sudo` step). The machine name is published in public
  certificate-transparency logs. A third party is in the path of your access
  control.

### Option C (not built): a real domain with DNS-01

Own a domain, point a name at the host's LAN address, and have Caddy prove control of
the domain through the DNS provider's API to get a Let's Encrypt certificate.

- For: a publicly trusted certificate with no client setup at all, and no exposure
  of the host.
- Against: needs a domain and a DNS provider account, an API token stored on the
  host, and a Caddy image built with that provider's plugin. The name is published in
  certificate-transparency logs. Some routers refuse to resolve public names to
  private addresses.

Neither B nor C can be set up without an account that is yours to open, so neither is
built.

---

## What was checked, and what was not

Checked on the workstation:

- **Worker**: unit tests. A tranche becoming due changes the directive action and
  notifies, on its own, under the LP rebuild bias and under a floor lock. A plan
  completing notifies, including the case where the directive does not change, and
  its sentence survives Pushover's length cut. A floor lock starting and ending
  notifies. A range exit notifies when it becomes the directive, and also under a
  floor lock, under a floor lock with a waiting plan, and with a tranche due, where
  the directive does not change. A ledger read failure does not announce a completed
  plan again. A config with both Pushover spellings loads. Delivery was run
  end to end against a local stand-in for each provider (correct payload per
  provider, a bad credential reported per channel, an unreachable provider, no
  channel configured), with assertions that no credential appears in any response.
- **Dashboard**: unit tests for the manifest (valid JSON, start URL opens the Lite
  view, every icon exists at its declared size), for the tags in `index.html`, for
  the absence of any service worker, and for the display helpers. `npm run build`
  succeeds and the manifest and icons are in `dist/`. The Lite cockpit was opened
  in a desktop browser at phone width against a local stand-in API: live,
  stale (`NOT LIVE`), and worker-unreachable states, and the test button. The
  worker-unreachable state was checked both on a cold start and with a snapshot
  already on the page (the stand-in stopped answering `/v1` while the page was open).
  Refreshing when the app returns to the foreground is from reading the code only.
- **Deploy script**: a dry run with a stub in place of `ssh`. With `NOCKNOCK_TLS`
  unset, the remote commands and the output are identical to the script before this
  change. With it set, three more remote commands run. A bad flag, port or name
  stops before any remote command. `tls/render.sh` was run on its own with good and
  bad ports, including `08080`.
- **Quadlet**: written with the same keys the existing quadlets use.

Not checked:

- **No real phone.** "Add to Home Screen" on iOS and Android, the standalone
  window, the icon as each platform crops it, and installing the root certificate
  are all untested.
- **No real provider.** No message was sent to Telegram or Pushover. The request
  shapes follow each provider's API documentation.
- **The proxy never ran.** The Caddyfile has not been through `caddy validate`, the
  quadlet has not been through Podman's generator, and nothing was deployed to the
  host. Reaching the proxy by IP address depends on Caddy's `default_sni` option
  behaving as documented.
- **WalletConnect pairing from a phone**, over HTTP or HTTPS.

If the proxy fails to start, the rest of the stack is unaffected:
`journalctl --user -u nocknock-tls` shows why, and
`NOCKNOCK_TLS=remove deploy/podman/deploy.sh chudmax` takes it out.
