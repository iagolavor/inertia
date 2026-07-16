# Vision: Ephemeral P2P Social Network

## Purpose

To create a peer-to-peer social media system that:

- Stores all data locally on users' devices.
- Prevents tech companies from harvesting user activity for AI models or advertising.
- Embraces ephemerality as a core design principle — posts vanish naturally, leaving no permanent archive.
- Offers Instagram-like sharing within your real social circle — without ads, algorithms, or doomscrolling.

---

## Core Principles

- **Local-first**: All posts, messages, and profiles live on the user's device. No central servers.
- **Ephemeral by design**: Content auto-expires after a set time. If delivery fails, the post simply disappears.
- **Direct connections**: Users connect by sharing invite links or QR codes. Each friendship is a secure peer-to-peer link with mutual consent.
- **Transparency**: Failed deliveries are visible to the sender, who can choose to retry or let the post expire.
- **Zero tracking**: No global user database, no analytics, no corporate intermediaries.

---

## Identity and Connections

- **Cryptographic keys = identity**: Ed25519/X25519 keypairs generated on-device at install. This is the only real identity.
- **Display name = local label**: A human-readable name shown to friends. Not globally unique.
- **No phone numbers, no global IDs**: Phone numbers are not used for discovery or verification. Users connect through channels they already trust (SMS, iMessage, in person).
- **Invite links and QR codes**: The primary way to add friends. An invite contains public keys, optional P2P reachability hints, and a signed expiry.
- **Mutual consent**: Opening an invite shows a preview with a safety code. The recipient must explicitly accept before keys are trusted.
- **Social circle only**: You only connect with people you already know. No searchable directory, no "find users by phone."

---

## Content Model

- **Personal profile**: A user's own profile and pictures are stored only on their device.
- **Friend profiles**: Friends' profiles and posts are fetched only when the user chooses to view them.
- **Posts**: Shared asynchronously, synced when devices connect. Delivery is not real-time.
- **Messages**: Delivered like Instagram DMs — not guaranteed instant, but reliable once both peers are online.
- **Ephemeral lifecycle**:
  - Posts and messages expire after **7 days**.
  - Failed-to-send messages remain on the sender's device for manual retry.

---

## Design Goals

- **Local Ownership**: Every user fully owns their profile, posts, and messages.
- **Ephemerality**: Posts and messages are temporary by default. The system forgets naturally.
- **Simplicity**: Delivery failures are handled on the sender's device.
- **Transparency**: Users see what was delivered and what failed.
- **Small-Scale Social Graphs**: Personal circles, not mass following.
- **Open Source**: Community-driven, auditable, no corporate lock-in. Licensed under [AGPL-3.0-or-later](../LICENSE).

---

## Non-Goals

- **No Global Discovery**: No hashtags, trending, global feeds, or **user** search. An optional **public relay list** (connectivity nodes only) is allowed — see §2b.
- **No Centralized Permanent Archives**: Feed posts and messages expire. There is no cloud library or global catalog. Author-hosted profile photos and optional shared folders may persist on the owner's device only.
- **No Corporate Servers**: No central infrastructure for accounts, content, or analytics.
- **No Phone-Based Directory**: No "type a number and find them" registry.
- **No Algorithmic Feeds**: Chronological friends-only content only.
- **No Ads or Engagement Optimization**: Not designed for doomscrolling or data harvesting.

---

## User Journey

### 1. Onboarding

- User installs the app.
- A cryptographic identity is generated locally (display name only).
- No signup, no servers, no phone verification.

### 2. Adding Friends (invite flow)

- User taps **Generate invite** → gets a link and QR code.
- They share it via SMS, iMessage, in person, or any channel they already use.
- Friend opens the link → sees display name + **safety code** → taps **Accept** (inviter must be online).
- Invite is **single-use** and expires in **15 minutes**.
- Both devices store each other's public keys locally. P2P connects when both are online.

### 3. Sharing Posts

- User creates a post. Stored locally, pushed to friends when peers are online.
- Failed delivery → outbox with retry or natural expiry.

### 4. Messaging

- Store-and-forward on the sender's device until ACK.
- 7-day lifespan. Async delivery when both peers are online.

### 5. Consumption

- Friend's profile and posts fetched on-demand from their device.
- Feed is friends-only, chronological.

### 6. Ephemerality

- Posts and messages vanish after 7 days.
- Profile photos and shared folders stay on the author's device until they remove them.

---

## Technical Architecture Overview

Each user runs **local-first** software on their own device. A small **VPS relay** (`inertia-relay`) provides connectivity only — no accounts, no SQLite, no decrypted content. Friends exchange **signed invite links** that bootstrap both the relay network and the inviter's reachability.

```plaintext
┌─────────────────────┐         ┌─────────────────────┐
│  User A (device)    │         │  User B (device)    │
│  inertia-api        │         │  inertia-api        │
│  inertia-core+P2P   │         │  inertia-core+P2P   │
│  SvelteKit (browser)│         │  SvelteKit (browser)│
└──────────┬──────────┘         └──────────┬──────────┘
           │  E2E encrypted envelopes      │
           │  (Noise + ChaCha20)           │
           └────────────┬──────────────────┘
                        │ circuit relay (TCP)
                        ▼
              ┌─────────────────────┐
              │  VPS (optional)     │
              │  inertia-relay      │
              │  libp2p relay only  │
              │  no user data       │
              └─────────────────────┘
```

**Per device:**

```plaintext
+-----------------------------+
|   SvelteKit Frontend        |
| - Invite / QR UI            |
| - Friends, messages, outbox |
+-----------------------------+
          | HTTP /api (local)
          v
+-----------------------------+
|   inertia-api (127.0.0.1)   |
+-----------------------------+
          |
          v
+-----------------------------+
|   inertia-core (libp2p)     |
| - P2P + relay client        |
| - Encryption & identity     |
| - Invite signing            |
| - SQLite + blobs (local)    |
+-----------------------------+
```

See [inertia-relay README](../crates/inertia-relay/README.md) for relay deployment, [RELAY-CONNECTIVITY.md](./RELAY-CONNECTIVITY.md) for connection architecture and diagrams, and [LIVE-SYNC.md](./LIVE-SYNC.md) for web UI event sync.

---

# Architecture Backbones

## Decisions Log

| Decision | Choice | Implication |
|----------|--------|-------------|
| Friend discovery | **Invite link + QR** | No global directory. Users share invites over channels they already trust. |
| Identity | **Cryptographic keypair** | No phone numbers, no SMS relay, no account database. |
| Connectivity | **libp2p relay circuits + optional VPS relay** | Friend paths are `/p2p-circuit/` via `inertia-relay`; relay is connectivity only. See [RELAY-CONNECTIVITY.md](./RELAY-CONNECTIVITY.md). |
| Post expiration | **7 days** | Default TTL for posts. |
| Message expiration | **7 days** | Same as posts. |
| Invite expiration | **15 minutes** | Links expire quickly; generate a fresh one anytime. |
| Invite usage | **Single-use** | Each nonce can be redeemed once; issuer must be online to accept. |
| Relay hosting | **Community VPS (optional)** | Anyone can run `inertia-relay`. No user data on the host — connectivity only. |
| Public relay list | **Curated directory of relays** | Not a user directory. Helps clients pick bootstrap relays; no accounts or content. |

---

## 1. Identity and Trust

**Backbone rules:**

- **Keypair is identity** — generated on-device, never leaves except as public keys in invites.
- **Display name is cosmetic** — shown in UI, not globally authoritative.
- **Invite links are signed** — recipient verifies signature + safety code before accepting.
- **Friendship is mutual** — accept stores keys locally; optional P2P handshake notifies the other side.
- **No global directory** — zero centralized user storage.

**Invite payload (version 2):**

```
version, display_name, signing_pubkey, encryption_pubkey,
peer_id, multiaddrs[], relay_multiaddr,
created_at, expires_at, nonce, signature
```

- **`multiaddrs`** — how to dial the inviter (circuit addresses via relay preferred).
- **`relay_multiaddr`** — shared VPS relay (`/ip4/HOST/tcp/9000/p2p/RELAY_PEER_ID`), signed by the inviter. On accept, the accepter applies this to Settings so new users can reach the network without a separate relay handoff.

Encoded as base64url in `inertia://invite/<payload>` or `https://app/invite#<payload>`.

**Invite generation** requires the inviter to be **Relay OK** (outbound libp2p session to the configured relay) **and** to hold an inbound **relay reservation** so the invite embeds a dialable `/p2p-circuit/` address. `GET /invite/readiness` reports progress; the Friends UI uses it before **Generate**.

**On accept:** accepter verifies signature + safety code, applies `relay_multiaddr` to local settings (unless `INERTIA_RELAY` env overrides), bootstraps a relay session, redials inviter circuit addresses, then completes P2P `InviteRedemption`.

**Single-use redemption:** the inviter's device stores each issued nonce. When a friend accepts, they send a P2P `InviteRedemption` request; the inviter marks the nonce consumed and rejects any second attempt. Acceptance requires the inviter to be online with P2P running.

---

## 2. P2P Transport

- libp2p with TCP / Noise / Yamux.
- **Relay client** on every node: outbound session to `inertia-relay`, inbound **circuit reservation**, and `/p2p-circuit/` listen addresses for friend reachability.
- **Friend paths are relay-circuit only** — stored contact multiaddrs, invite dial targets, and redials use `/p2p-circuit/` addresses built from configured relays. LAN and direct TCP are not used for friend connectivity (local TCP listen remains for transport to the VPS).
- **Swarm actor** (`p2p/swarm_task.rs`): one task owns the libp2p swarm; `P2pNode` sends commands and reads `watch` state. Bootstrap waits in `engine/relay_dial.rs` are event-driven (no polling loops).
- **DCUtR** (hole punching) is still in the behaviour stack and may upgrade some sessions to direct transport after a circuit is up; the product path for discovery and redial remains relay circuits.
- **VPS relay** (`inertia-relay`): one TCP port, stable relay peer id, no user payloads stored. Must advertise a routable external address in reservation responses.
- Peers connect via circuit multiaddrs in invites; `relay_multiaddr` in invite v2 bootstraps new users onto the network.
- Connection states: `online`, `offline`, `unreachable`. Header shows **API** vs **P2P** (relay health + friend count).

---

## 2b. Community Relays and Public Relay List (planned)

Today every invite embeds one `relay_multiaddr` from the inviter's settings. That works for private circles (family, friends). To grow beyond hand-picked relays without a central **account** server, Inertia can support **public community relays** — optional VPS nodes run by volunteers or small operators, listed in a **public relay list**.

This is **not** global user discovery. It is a directory of **connectivity helpers** (multiaddr + metadata), similar in spirit to public Matrix or email relay lists — not a searchable people graph.

### What a community relay host provides

- Runs **`inertia-relay`** on a VPS (see [inertia-relay README](../crates/inertia-relay/README.md)).
- Stable libp2p peer id, one TCP port, no SQLite, no decrypted payloads.
- Enough bandwidth for **peak** circuit relay traffic; DCUtR may reduce relay load when direct upgrade succeeds, but operators should size for concurrent circuits.

**Rough sizing (indicative, not guarantees):**

| Community on relay | Typical VPS | Indicative cost |
|--------------------|-------------|-----------------|
| ~50–200 users | 1 vCPU, 1–2 GB RAM | ~R$25–80/mo (BR/EU providers) |
| ~500–2,000 users | 2 vCPU, 4 GB RAM | ~R$60–120/mo |
| ~5,000+ users | 4 vCPU, 8 GB RAM or **multiple relays** | ~R$150–300/mo per node |

"Users on relay" ≠ simultaneous connections. Relay load depends on **concurrent circuits** and **blob traffic** when direct dial fails — monitor and shard before one box becomes a hotspot.

### Public relay list

A **public relay list** is a signed or auditable manifest (JSON, static site, or git repo) clients fetch optionally:

```plaintext
relay_id, multiaddr, display_name, region, optional_host_pubkey,
optional_join_fee, optional_pix_key, health_hint, operator_url
```

- Clients may ship a **default list** (project-maintained) and let users add community entries in Settings.
- Invites can still embed a specific relay; the list is for **bootstrap** when you don't know anyone yet or need a fallback.
- Listing is **voluntary** — operators opt in; the project does not operate a user database.

Scaling to large numbers of users means **many relays**, not one mega-host: invite trees cluster around popular relays unless the list spreads load across regions and operators.

### Optional join fee via PIX (Brazil-first)

Community hosts need a simple way to recover VPS costs without ads or a central payment platform storing identities.

**Idea (future invite v3, optional):** embed host funding hints in the invite or relay list entry:

- **`pix_key`** — operator's PIX EVP (email, phone, or random key).
- **`join_fee_brl`** — e.g. `1.00` (R$1 one-time chip-in).
- **`payment_ref`** — nonce or unique amount suffix for reconciliation.

**User journey:**

1. Accepter scans invite QR (or picks a relay from the public list).
2. UI shows: *"R$1 via PIX helps run this relay"* + PIX QR / copia-e-cola.
3. After payment is confirmed, friend acceptance proceeds (P2P `InviteRedemption` as today).

**Why PIX fits:** instant, familiar in Brazil, low friction for person-to-person transfers; invite flow is already QR-native.

**What we must build (later):**

- **Payment verification** — manual confirm, unique-amount reconciliation, or PSP webhook (OpenPix, Mercado Pago, etc.); paying PIX must not be honor-system only if access is gated on payment.
- **Clear trust copy** — user pays **the relay operator**, not "Inertia the company."
- **Geography** — PIX is Brazil-specific; other regions need different optional funding fields or no fee.
- **Regulatory awareness** — charging for relay access may implicate local payment rules; community co-op framing vs commercial service TBD with real-world advice.

**Economics (example):** R$1 × 500 joins = R$500 gross — enough to fund a modest VPS for a long time if traffic stays community-scaled. Ongoing hosting still needs either repeated fees, donations, or operator goodwill; one-time R$1 is a **bootstrap subsidy**, not infinite margin.

### Principles

- **Relay ≠ social server** — paying for relay access buys **connectivity**, not your posts, keys, or feed on someone else's disk.
- **No central wallet** — funds flow host-to-joiner via PIX; Inertia software stays out of custody where possible.
- **Public relays are optional** — private invites with a free family relay remain the core model.
- **Abuse** — open relays need rate limits and monitoring (see [SECURITY-TODO.md](./SECURITY-TODO.md)); paid join is one **social** throttle, not a crypto guarantee by itself.

---

## 3. Content and Ephemerality

| Type | Expiration |
|------|------------|
| Posts (feed) | 7 days |
| Messages | 7 days |
| Invites | 15 minutes, single-use |
| Profile items | No auto-expire (author-hosted) |
| Profile comments | No auto-expire (author-hosted) |
| Shared folders (Files tab) | No auto-expire (author-hosted; pull on demand; peer transfer is DCUtR/direct only - see [ARCHIVE-P2P.md](./ARCHIVE-P2P.md)) |

---

## 4. Local Storage

SQLite on device only:

```
contacts          (display_name, peer_id, pubkeys, last_seen)
outbox            (content_id, recipient_id, status, expires_at)
inbox             (content_id, sender_id, body, media_ref, expires_at)
local_posts       (own feed posts, 7d TTL)
profile_items     (durable gallery; profile_photos kept as legacy mirror)
profile_comments  (comments on profile items; author-hosted)
archive_folders   (shared folders metadata)
archive_entries   (chunked files in shared folders; no inbox fan-out)
archive_uploads   (pending local chunked ingest)
feed_archive      (optional persistent feed history)
app_settings      (e.g. feed_history_enabled)
identity          (signing_pubkey, encryption_pubkey, display_name)
blobs/            (content-addressed media files)
```

---

## 5. Application Layers

- **inertia-core** (Rust): identity, invites, P2P, storage, expiry.
- **inertia-api** (Rust): local HTTP bridge — runs on the user's machine, not in the cloud.
- **SvelteKit** (web/PWA): feed, profile, settings, invites, connections, messages, outbox, Files tab. Live updates: [LIVE-SYNC.md](./LIVE-SYNC.md).
- **Capacitor** (Android Stage B alpha shipped): native WebView + on-device `inertia-api`. iOS and polish remain. See [CAPACITOR.md](./CAPACITOR.md).
- **Tauri** (next): desktop shell that starts the local API and opens the UI in one window, plus a simpler install path than the Windows zip + browser flow.

---

## 6. Security Model

- E2E encryption (X25519 + ChaCha20-Poly1305).
- Signed content envelopes and invite payloads.
- Safety codes for out-of-band verification.
- Threat model v1: protects against corporate harvesting and passive observers; not against screenshots or compromised devices.

---

## Open Questions

1. Profile caching when friend is offline? *(v1: online-only live fetch; roster may still show from local cache)*
2. Web/PWA vs mobile-first for v1?
3. Media size limits for strict P2P? *(~2 MB per image, client-side compression)*
4. User-configurable post TTL? *(optional local feed archive exists)*
5. Public relay list — who curates the default manifest, and how are unhealthy relays delisted?
6. PIX join fee — manual confirm vs PSP webhook for v1 community hosts?

---

## Phased Delivery

| Phase | Scope | Status |
|-------|--------|--------|
| 0 | Vision and backbone alignment | Done |
| 1 | Rust core: identity, storage, expiry | Done |
| 2 | libp2p messaging, outbox | Done |
| 3 | SvelteKit UI + local API | Done |
| 4 | Invite flow, feed, profile, settings, backup | Done |
| 4b | **VPS relay** (`inertia-relay`), relay client, invite v2 with embedded relay | Done |
| 4c | **Event-driven live sync** (SSE), Messages/Connections polish, Profile Posts + Files (archive P2P / DCUtR) | Done |
| 5 | Capacitor Android Stage B (API + UI on device, v0.10) | Done (alpha); iOS + mobile polish remain |
| 6 | **Tauri desktop shell** + easier install path (sidecar or in-process API, app data dir, installers) | **Next** |
| 7 | Thumbnails, orphan blob GC | Planned |
| 8 | **Community relays** - public relay list, optional PIX join fee in invite v3, host health hints | Planned |

P2P blob sync for photo posts ships in Phase 4. Shared-folder peer pulls are DCUtR/direct (see [ARCHIVE-P2P.md](./ARCHIVE-P2P.md)). Phase 7 is thumbnails and orphan blob GC only.
