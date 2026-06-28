# Milestone: VPS relay — stable two-user test

**Goal:** You and your brother can each run Inertia on your own machines, add each other via invite, and exchange **text posts and DMs** reliably — even when direct LAN/WAN paths fail — by routing libp2p through a **small VPS you control**.

**Not in scope for this milestone:** mobile app (Capacitor), public internet scale, account servers, cloud storage, algorithmic feed.

---

## Architecture

Each person keeps **local-first** data (SQLite + blobs on their device). The VPS is **connectivity only** — it never sees decrypted content.

```
┌─────────────────────┐         ┌─────────────────────┐
│  You                │         │  Brother            │
│  inertia-api (local)│         │  inertia-api (local)│
│  inertia-core+P2P   │         │  inertia-core+P2P   │
│  SvelteKit (browser)│         │  SvelteKit (browser)│
└──────────┬──────────┘         └──────────┬──────────┘
           │  E2E encrypted envelopes      │
           │  (Noise + ChaCha20)           │
           └────────────┬──────────────────┘
                        │ circuit relay (TCP)
                        ▼
              ┌─────────────────────┐
              │  VPS                │
              │  inertia-relay      │
              │  (libp2p relay only)│
              │  no SQLite, no keys │
              └─────────────────────┘
```

**Why VPS even on the same LAN:** exercises the same path you will need across real networks (NAT, no port forwarding, asymmetric reachability). LAN-only direct dial stays a dev shortcut, not the product path.

---

## Success criteria (definition of “stable”)

- [x] Both sides: create profile → P2P starts automatically on API boot (no manual Outbox retry in the happy path).
- [x] Inviter generates invite → link contains **relay-routable** multiaddr(s), not `0.0.0.0`.
- [x] Accepter opens invite → safety code → **Accept** succeeds with inviter online.
- [x] Either side posts text → other side sees it on Feed within one reconnect cycle (no manual Outbox retry in the happy path).
- [x] Either side sends DM → other side sees it in Messages.
- [x] API restart on one side → after both are back online, delivery resumes without re-inviting.
- [x] VPS runs one binary + one open TCP port; no user data persisted on the server.

**Stretch (same milestone if time allows):**

- [x] Photo posts deliver image bytes, not just hash placeholders.

---

## Phase 0 — Client reliability (required before VPS helps)

These gaps block *any* real-world test, relay or not.


| #   | Task                                                                          | Area                                | Notes                                          |
| --- | ----------------------------------------------------------------------------- | ----------------------------------- | ---------------------------------------------- |
| 0.1 | Auto-start P2P when identity exists (API boot + UI refresh fallback)          | `engine/mod.rs`, `identity.svelte.ts` | `Engine::open` calls `ensure_p2p_started`; UI falls back via `/p2p/start`. |
| 0.2 | Fixed P2P listen port (config / settings, default e.g. `4784`)                | core + settings UI                  | Random ports + firewall = pain.                |
| 0.3 | Persist `multiaddrs` per contact in SQLite                                    | `storage.rs`, `Contact`             | Needed to redial after restart.                |
| 0.4 | On P2P start: dial relay (if configured) then dial all contacts               | `engine.rs`, `p2p/node.rs`          |                                                |
| 0.5 | Auto-retry outbox when `PeerConnected` fires                                  | `p2p/node.rs` or `engine.rs`        | Implemented in `engine/outbox.rs`.             |
| 0.6 | Separate UI status: **API** vs **P2P** vs per-friend connection               | `OnlineStatus`, settings            | Header “online” today = HTTP only.             |
| 0.7 | Settings: connection panel (peer id, listen port, relay addr, copy multiaddr) | `settings/+page.svelte`             | Replaces raw env vars for test setup.          |


---

## Phase 1 — VPS relay binary


| #   | Task                                                                       | Area                   | Notes                                                   |
| --- | -------------------------------------------------------------------------- | ---------------------- | ------------------------------------------------------- |
| 1.1 | New crate `inertia-relay` — libp2p relay listener on `0.0.0.0:<port>`      | `crates/inertia-relay` | Add `relay` (+ `autonat` if useful) to libp2p features. |
| 1.2 | Relay identity persisted to disk (stable peer id across restarts)          | relay                  | Clients pin a known relay peer id in config.            |
| 1.3 | `docker compose` + `Dockerfile` for VPS deploy                             | `docker/`              | Single service, one published port.                     |
| 1.4 | VPS ops doc: firewall (`ufw allow <port>/tcp`), systemd or compose restart | `crates/inertia-relay/README.md` | Ubuntu 22.04+ assumed.                                  |
| 1.5 | Health/log: `tracing` lines for relay reservations, not payload content    | relay                  |                                                         |


**VPS does not run:** `inertia-api`, SQLite, invite signing, or the web UI.

---

## Phase 2 — Clients use the relay


| #   | Task                                                                                                   | Area            | Notes                                                        |
| --- | ------------------------------------------------------------------------------------------------------ | --------------- | ------------------------------------------------------------ |
| 2.1 | Config: `INERTIA_RELAY` / settings field — relay multiaddr `/ip4/<vps>/tcp/<port>/p2p/<relay_peer_id>` | core + UI       |                                                              |
| 2.2 | Enable libp2p **dcutr** (hole punching) + **relay client** behaviour on `P2pNode`                      | `p2p/node.rs`   | Try direct path when possible; fall back to circuit via VPS. |
| 2.3 | On P2P start: listen on relay; obtain circuit address for this peer                                    | `p2p/node.rs`   |                                                              |
| 2.4 | Embed circuit (or relay + local) addresses in invites via `p2p_invite_addresses`                       | `engine.rs`     | Replace useless `0.0.0.0` listeners.                         |
| 2.5 | Accept flow: dial invite multiaddrs (relay paths included) before redemption                           | already partial | Verify timeout UX when relay down.                           |
| 2.6 | Store inviter/accepter observed addresses on successful friend handshake                               | storage         | Feeds 0.3 redial list.                                       |


---

## Phase 3 — Brother test playbook


| #   | Task                                                                                                      | Area                | Notes                                                   |
| --- | --------------------------------------------------------------------------------------------------------- | ------------------- | ------------------------------------------------------- |
| 3.1 | Written test script: deploy relay → both install → both set relay URL → invite → post → DM → restart test | `crates/inertia-relay/README.md` |                                                         |
| 3.2 | Prebuilt web `adapter-static` option (brother may not run `npm run dev`)                                  | `apps/web`          | `npm run build` + serve `build/` or tiny static server. |
| 3.3 | Windows + Linux build notes for `inertia-api`                                                             | README or inertia-relay README | You on Windows; brother’s OS may differ.                |
| 3.4 | Invite link uses HTTPS origin you control **or** `inertia://` deep link + copy payload                    | invites             | For remote open without localhost in URL.               |


**Suggested test order**

1. Deploy `inertia-relay` on VPS; confirm port open (`nc -vz <vps> <port>`).
2. You: `INERTIA_RELAY=...`, start API + web, create profile, confirm P2P + relay in Settings.
3. Brother: same relay config, create profile.
4. You: Friends → Generate invite → send link (Signal/iMessage/etc.).
5. Brother: open `/invite`, verify safety code, Accept.
6. Brother: post “hello”; you: Feed updates automatically (polls while the tab is open).
7. You: DM; brother: Messages.
8. Kill your API, restart, repeat post — should recover without new invite.

---

## Phase 4 — Media (stretch)


| #   | Task                                                       | Area                    | Notes                                      |
| --- | ---------------------------------------------------------- | ----------------------- | ------------------------------------------ |
| 4.1 | P2P blob request/response for `media_ref` hashes           | `p2p/protocol.rs`, core | Phase 6 in VISION; needed for photo posts. |
| 4.2 | Sender pushes blob after envelope ACK (or inline chunking) | engine                  |                                            |
| 4.3 | Recipient writes `blobs/` and renders in Feed              | existing blob API       |                                            |


Photo posts sync over P2P after the text envelope is delivered (blob push on ack, pull on reconnect). Validated in the two-user relay test (stretch criteria).

---

## Phase 5 — Hardening before calling it “stable”


| #   | Task                                                                         | Area                 | Notes                                                  |
| --- | ---------------------------------------------------------------------------- | -------------------- | ------------------------------------------------------ |
| 5.1 | Relay: connection limits / reservation caps (basic abuse guard)              | `inertia-relay`      | `config.rs` + env caps; logs denied reservations/circuits |
| 5.2 | Integration test: two in-process peers via memory transport or local relay   | `inertia-core` tests | `tests/two_peer_local.rs` — loopback TCP dial          |
| 5.3 | Clear errors: “relay unreachable”, “inviter offline”, “P2P not started”      | API + UI             | `user_error.rs` codes + `ApiError.code` in clients   |
| 5.4 | Update `VISION.md` phased table: insert **Phase 4b — VPS relay** or renumber | docs                 | Done — 4b marked complete; Phase 6 scoped to GC/thumbs |


---

## Config sketch (after implementation)

**VPS** (`docker compose`):

```yaml
environment:
  INERTIA_RELAY_ADDR: "0.0.0.0:9000"
  INERTIA_RELAY_DATA_DIR: "/data"   # relay key only
ports:
  - "9000:9000"
```

**Each client** (env or Settings UI):

```bash
INERTIA_RELAY=/ip4/<VPS_PUBLIC_IP>/tcp/9000/p2p/<RELAY_PEER_ID>
INERTIA_P2P_LISTEN_PORT=4784
# INERTIA_API_ADDR stays 127.0.0.1:4783 — API remains local-only
```

---

## Explicit non-goals (this milestone)

- Central user database or login on VPS
- Hosting the Svelte app on VPS for production (static export per user is fine)
- Public relay federation or paid multi-tenant relay
- Replacing E2E encryption with server-side storage
- LAN mDNS discovery (defer)

---

## Suggested implementation order

Phases 0–4 and the brother test are **complete**. VPS relay hardening is **complete**. **Android Stage B** (Capacitor + bundled API) shipped in **v0.10.0** — see [CAPACITOR.md](./CAPACITOR.md) for build/resume checklist.

Next focus:

1. **Android polish** — P2pStatus tap panel, invite preview UX, Stage A regression, Play signing
2. **Phase 6** (VISION) — thumbnails, orphan blob GC
3. **Phase 7** — community relays (optional)
4. **iOS** Capacitor shell (not started)