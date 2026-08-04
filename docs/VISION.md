# Vision: Ephemeral P2P Social Network

## Purpose

To create a peer-to-peer social media system that:

- Stores all data locally on users' devices.
- Keeps activity private to your circle so it is not harvested for advertising or model training.
- Treats ephemerality as a core design principle: posts and messages age out on their own.
- Offers familiar photo and message sharing within your real social circle, in chronological order.

---

## Core Principles

- **Local-first**: All posts, messages, and profiles live on the user's device. Each person runs their own API and database.
- **Ephemeral by design**: Content auto-expires after a set time. If delivery fails, the post ages out on the sender's device.
- **Direct connections**: Users connect by sharing invite links or QR codes. Each friendship is a secure peer-to-peer link with mutual consent.
- **Transparency**: Failed deliveries are visible to the sender, who can choose to retry or let the post expire.
- **Private by default**: Identity and content stay on-device and among friends; the project does not run a global user database or analytics pipeline.

---

## Identity and Connections

- **Cryptographic keys = identity**: Ed25519/X25519 keypairs generated on-device at install. This is the only real identity.
- **Display name = local label**: A human-readable name shown to friends. Not globally unique.
- **Trusted channels for discovery**: Friends find each other through invites shared over SMS, iMessage, in person, or any channel they already use. Phone numbers are not part of the product identity model.
- **Invite links and QR codes**: The primary way to add friends. An invite contains public keys, optional P2P reachability hints, and a signed expiry.
- **Mutual consent**: Opening an invite shows a preview with a safety code. The recipient must explicitly accept before keys are trusted.
- **Social circle only**: You connect with people you already know through invites you share.

---

## Content Model

- **Personal profile**: A user's own profile and pictures are stored only on their device.
- **Friend profiles**: Friends' profiles and posts are fetched only when the user chooses to view them.
- **Posts**: Shared asynchronously, synced when devices connect. Delivery is not real-time.
- **Messages**: Delivered like familiar private DMs: reliable once both peers are online, and asynchronous when they are not.
- **Ephemeral lifecycle**:
  - Posts and messages expire after **7 days**.
  - Failed-to-send messages remain on the sender's device for manual retry.

---

## Design Goals

- **Local Ownership**: Every user fully owns their profile, posts, and messages.
- **Ephemerality**: Posts and messages are temporary by default. The system forgets naturally.
- **Simplicity**: Delivery failures are handled on the sender's device.
- **Transparency**: Users see what was delivered and what failed.
- **Small-Scale Social Graphs**: Personal circles sized for people you know.
- **Open Source**: Community-driven and auditable. Licensed under [AGPL-3.0-or-later](../LICENSE).

---

## Scope

These boundaries keep the product focused on a trusted circle and local ownership:

- **Invite-based discovery**: Friends join through shared invites. Hashtags, trending, and global user search are outside scope. A future **public relay list** (connectivity nodes only) may help pick a relay - see §2b. A configured relay is still required.
- **Local and ephemeral content**: Feed posts and messages expire. Author-hosted profile photos and optional shared folders may persist on the owner's device only. There is no cloud library of everyone's content.
- **User-run infrastructure**: Accounts, content, and analytics live on each person's device (plus optional self-hosted relays for connectivity). The project does not operate a central social backend.
- **Trusted-channel invites**: Discovery happens through channels you already use, not a phone-number registry inside the app.
- **Chronological friends feed**: The home feed is friends-only and chronological.
- **Calm product surface**: Designed for sharing with people you know, not engagement optimization or advertising.

---

## User Journey

### 1. Onboarding

- User installs the app.
- A cryptographic identity is generated locally (display name only).
- Ready to invite friends once a relay is configured.

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

Each user runs **local-first** software on their own device. A small **VPS relay** (`inertia-relay`) provides connectivity: circuit paths so friends can reach each other. Friends exchange **signed invite links** that bootstrap both the relay network and the inviter's reachability.

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
              │  VPS                │
              │  inertia-relay      │
              │  libp2p relay only  │
              │  circuits only      │
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
| Friend discovery | **Invite link + QR** | Users share invites over channels they already trust. |
| Identity | **Cryptographic keypair** | Created on-device; public keys travel in invites. |
| Connectivity | **libp2p relay circuits via VPS (`inertia-relay`)** | Friend paths are `/p2p-circuit/` only; LAN/direct TCP are not used for friends. Relay is connectivity only. See [RELAY-CONNECTIVITY.md](./RELAY-CONNECTIVITY.md). |
| Post expiration | **7 days** | Default TTL for posts. |
| Message expiration | **7 days** | Same as posts. |
| Invite expiration | **15 minutes** | Links expire quickly; generate a fresh one anytime. |
| Invite usage | **Single-use** | Each nonce can be redeemed once; issuer must be online to accept. |
| Relay hosting | **Self-hosted or community VPS** | Every circle needs a reachable `inertia-relay`. Anyone can run one. Hosts provide connectivity, not social storage. |
| Public relay list | **Curated directory of relays** | Bootstrap helpers for connectivity; accounts and content stay on user devices. |

---

## 1. Identity and Trust

**Backbone rules:**

- **Keypair is identity** - generated on-device; public keys travel in invites.
- **Display name is cosmetic** - shown in UI for friends, not a global unique handle.
- **Invite links are signed** - recipient verifies signature + safety code before accepting.
- **Friendship is mutual** - accept stores keys locally; optional P2P handshake notifies the other side.
- **Invite-based roster** - contacts live on each device after mutual accept.

**Invite payload (version 2):**

```
version, display_name, signing_pubkey, encryption_pubkey,
peer_id, multiaddrs[], relay_multiaddr,
created_at, expires_at, nonce, signature
```

- **`multiaddrs`** - how to dial the inviter (`/p2p-circuit/` addresses via the shared relay).
- **`relay_multiaddr`** - shared VPS relay (`/ip4/HOST/tcp/9000/p2p/RELAY_PEER_ID`), signed by the inviter. On accept, the accepter applies this to Settings so new users join the same relay network without a separate handoff.

Encoded as base64url in `inertia://invite/<payload>` or `https://app/invite#<payload>`.

**Invite generation** requires the inviter to be **Relay OK** (outbound libp2p session to the configured relay) **and** to hold an inbound **relay reservation** so the invite embeds a dialable `/p2p-circuit/` address. `GET /invite/readiness` reports progress; the Friends UI uses it before **Generate**.

**On accept:** accepter verifies signature + safety code, applies `relay_multiaddr` to local settings (unless `INERTIA_RELAY` env overrides), bootstraps a relay session, redials inviter circuit addresses, then completes P2P `InviteRedemption`.

**Single-use redemption:** the inviter's device stores each issued nonce. When a friend accepts, they send a P2P `InviteRedemption` request; the inviter marks the nonce consumed and rejects any second attempt. Acceptance requires the inviter to be online with P2P running.

---

## 2. P2P Transport

- libp2p with TCP / Noise / Yamux.
- **Relay client** on every node: outbound session to `inertia-relay`, inbound **circuit reservation**, and `/p2p-circuit/` listen addresses for friend reachability.
- **Friend paths are relay-circuit only** - stored contact multiaddrs, invite dial targets, and redials use `/p2p-circuit/` addresses built from configured relays. LAN and direct TCP are reserved for other transport needs (local TCP listen remains for transport to the VPS).
- **Swarm actor** (`p2p/swarm_task.rs`): one task owns the libp2p swarm; `P2pNode` sends commands and reads `watch` state. Bootstrap waits in `engine/relay_dial.rs` are event-driven.
- **DCUtR** (hole punching) is still in the behaviour stack and may upgrade some sessions to direct transport after a circuit is up; the product path for discovery and redial remains relay circuits.
- **VPS relay** (`inertia-relay`): one TCP port, stable relay peer id, circuit connectivity only. Must advertise a routable external address in reservation responses.
- Peers connect via circuit multiaddrs in invites; `relay_multiaddr` in invite v2 bootstraps new users onto the network.
- Connection states: `online`, `offline`, `unreachable`. Header shows **API** vs **P2P** (relay health + friend count).

---

## 2b. Community Relays and Public Relay List (planned)

Today every invite embeds one `relay_multiaddr` from the inviter's settings. That works for private circles (family, friends). To grow beyond hand-picked relays while keeping accounts on each device, Inertia can support **public community relays** - VPS nodes run by volunteers or small operators, listed in a **public relay list**.

This list is a directory of **connectivity helpers** (multiaddr + metadata), similar in spirit to public Matrix or email relay lists: operators publish how to reach their relay, and clients can pick one for bootstrap.

### What a community relay host provides

- Runs **`inertia-relay`** on a VPS (see [inertia-relay README](../crates/inertia-relay/README.md)).
- Stable libp2p peer id, one TCP port, and circuit relay for friends.
- Enough bandwidth for **peak** circuit relay traffic; DCUtR may reduce relay load when direct upgrade succeeds, but operators should size for concurrent circuits.

**Rough sizing (indicative, not guarantees):**

| Community on relay | Typical VPS | Indicative cost |
|--------------------|-------------|-----------------|
| ~50–200 users | 1 vCPU, 1–2 GB RAM | ~R$25–80/mo (BR/EU providers) |
| ~500–2,000 users | 2 vCPU, 4 GB RAM | ~R$60–120/mo |
| ~5,000+ users | 4 vCPU, 8 GB RAM or **multiple relays** | ~R$150–300/mo per node |

"Users on relay" ≠ simultaneous connections. Relay load depends on **concurrent circuits** and **blob traffic** when direct dial fails - monitor and shard before one box becomes a hotspot.

### Public relay list

A **public relay list** is a signed or auditable manifest (JSON, static site, or git repo) clients fetch optionally:

```plaintext
relay_id, multiaddr, display_name, region, optional_host_pubkey,
optional_join_fee, optional_pix_key, health_hint, operator_url
```

- Clients may ship a **default list** (project-maintained) and let users add community entries in Settings.
- Invites can still embed a specific relay; the list is for **bootstrap** when you don't know anyone yet or need a fallback.
- Listing is **voluntary** - operators opt in; clients keep their own contacts and content locally.

Scaling to large numbers of users means **many relays**, not one mega-host: invite trees cluster around popular relays unless the list spreads load across regions and operators.

### Optional join fee via PIX (Brazil-first)

Community hosts need a simple way to recover VPS costs while keeping payments between people (for example PIX in Brazil).

**Idea (future invite v3, optional):** embed host funding hints in the invite or relay list entry:

- **`pix_key`** - operator's PIX EVP (email, phone, or random key).
- **`join_fee_brl`** - e.g. `1.00` (R$1 one-time chip-in).
- **`payment_ref`** - nonce or unique amount suffix for reconciliation.

**User journey:**

1. Accepter scans invite QR (or picks a relay from the public list).
2. UI shows: *"R$1 via PIX helps run this relay"* + PIX QR / copia-e-cola.
3. After payment is confirmed, friend acceptance proceeds (P2P `InviteRedemption` as today).

**Why PIX fits:** instant, familiar in Brazil, low friction for person-to-person transfers; invite flow is already QR-native.

**What we must build (later):**

- **Payment verification** - manual confirm, unique-amount reconciliation, or PSP webhook (OpenPix, Mercado Pago, etc.); paying PIX should be verifiable if access is gated on payment.
- **Clear trust copy** - user pays **the relay operator**, and Inertia software stays out of custody where possible.
- **Geography** - PIX is Brazil-specific; other regions need different optional funding fields or free relays.
- **Regulatory awareness** - charging for relay access may implicate local payment rules; community co-op framing vs commercial service TBD with real-world advice.

**Economics (example):** R$1 × 500 joins = R$500 gross - enough to fund a modest VPS for a long time if traffic stays community-scaled. Ongoing hosting still needs either repeated fees, donations, or operator goodwill; one-time R$1 is a **bootstrap subsidy**.

### Principles

- **Relay = connectivity** - paying for relay access buys circuit reachability. Posts, keys, and feeds stay on user devices.
- **Direct funding** - funds flow host-to-joiner via PIX (or regional equivalents); Inertia software stays out of custody where possible.
- **A relay is required** - private/family VPS remains the core model; a future public relay list is only another way to pick one.
- **Abuse** - open relays need rate limits and monitoring (see [SECURITY-TODO.md](./SECURITY-TODO.md)); paid join is one social throttle alongside technical limits.

---

## 3. Content and Ephemerality

| Type | Expiration |
|------|------------|
| Posts (feed) | 7 days |
| Messages | 7 days |
| Invites | 15 minutes, single-use |
| Profile items | Author-hosted (durable until removed) |
| Profile comments | Author-hosted (durable until removed) |
| Shared folders (Files tab) | Author-hosted; pull on demand; peer transfer is DCUtR/direct only - see [ARCHIVE-P2P.md](./ARCHIVE-P2P.md) |

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
- **inertia-api** (Rust): local HTTP bridge that runs on the user's machine.
- **SvelteKit** (web/PWA): feed, profile, settings, invites, connections, messages, outbox, Files tab. Live updates: [LIVE-SYNC.md](./LIVE-SYNC.md).
- **Tauri** (desktop + Android): native WebView shell + on-device / sidecar `inertia-api`. See [TAURI.md](./TAURI.md).
- **iOS**: planned via the same Tauri shell.

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
5. Public relay list - who curates the default manifest, and how are unhealthy relays delisted?
6. PIX join fee - manual confirm vs PSP webhook for v1 community hosts?

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
| 5 | Android on-device install (API + UI on device; now Tauri) | Done |
| 6 | **Tauri desktop + Android** (sidecar / jniLibs API, app data dir, installers + debug APK) | **In progress** ([TAURI.md](./TAURI.md)) |
| 7 | Thumbnails, orphan blob GC | Planned |
| 8 | **Community relays** - public relay list, optional PIX join fee in invite v3, host health hints | Planned |

P2P blob sync for photo posts ships in Phase 4. Shared-folder peer pulls are DCUtR/direct (see [ARCHIVE-P2P.md](./ARCHIVE-P2P.md)). Phase 7 is thumbnails and orphan blob GC only.
