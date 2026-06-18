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
- **Open Source**: Community-driven, auditable, no corporate lock-in.

---

## Non-Goals

- **No Global Discovery**: No hashtags, trending, global feeds, or user search.
- **No Permanent Archives**: Once expired, content is gone.
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

- Posts and messages vanish after 7 days. No archives by default.

---

## Technical Architecture Overview

```plaintext
+-----------------------------+
|         User Device         |
| (Cryptographic identity)    |
+-----------------------------+
          | Local-first
          v
+-----------------------------+
|   Rust Core (libp2p)        |
| - P2P connections           |
| - Encryption & identity     |
| - Invite signing            |
| - Ephemeral storage         |
+-----------------------------+
          | Local API
          v
+-----------------------------+
|   SvelteKit Frontend        |
| - Invite / QR UI            |
| - Friends, messages, outbox |
+-----------------------------+
```

---

# Architecture Backbones

## Decisions Log

| Decision | Choice | Implication |
|----------|--------|-------------|
| Friend discovery | **Invite link + QR** | No global directory. Users share invites over channels they already trust. |
| Identity | **Cryptographic keypair** | No phone numbers, no SMS relay, no account database. |
| Connectivity | **Strict P2P** | Delivery only when both peers are online. Outbox handles failures. |
| Post expiration | **7 days** | Default TTL for posts. |
| Message expiration | **7 days** | Same as posts. |
| Invite expiration | **15 minutes** | Links expire quickly; generate a fresh one anytime. |
| Invite usage | **Single-use** | Each nonce can be redeemed once; issuer must be online to accept. |

---

## 1. Identity and Trust

**Backbone rules:**

- **Keypair is identity** — generated on-device, never leaves except as public keys in invites.
- **Display name is cosmetic** — shown in UI, not globally authoritative.
- **Invite links are signed** — recipient verifies signature + safety code before accepting.
- **Friendship is mutual** — accept stores keys locally; optional P2P handshake notifies the other side.
- **No global directory** — zero centralized user storage.

**Invite payload:**

```
version, display_name, signing_pubkey, encryption_pubkey,
peer_id, multiaddrs[], created_at, expires_at, nonce, signature
```

Encoded as base64url in `inertia://invite/<payload>` or `https://app/invite#<payload>`.

**Single-use redemption:** the inviter's device stores each issued nonce. When a friend accepts, they send a P2P `InviteRedemption` request; the inviter marks the nonce consumed and rejects any second attempt. Acceptance requires the inviter to be online with P2P running.

---

## 2. P2P Transport (strict mode)

- libp2p with TCP/Noise/Yamux.
- No relays, no TURN, no friend-as-relay.
- Peers connect via multiaddrs exchanged in invites.
- Connection states: `online`, `offline`, `unreachable`.

---

## 3. Content and Ephemerality

| Type | Expiration |
|------|------------|
| Posts | 7 days |
| Messages | 7 days |
| Invites | 15 minutes, single-use |
| Profile | No auto-expire |

---

## 4. Local Storage

SQLite on device only:

```
contacts       (display_name, peer_id, pubkeys, last_seen)
outbox         (content_id, recipient_id, status, expires_at)
inbox          (content_id, sender_id, body, media_ref, expires_at)
local_posts    (own posts, 7d TTL)
profile_photos (local photo grid)
feed_archive   (optional persistent feed history)
app_settings   (e.g. feed_history_enabled)
identity       (signing_pubkey, encryption_pubkey, display_name)
blobs/         (content-addressed media files)
```

---

## 5. Application Layers

- **inertia-core** (Rust): identity, invites, P2P, storage, expiry.
- **inertia-api** (Rust): local HTTP bridge — runs on the user's machine, not in the cloud.
- **SvelteKit** (web/PWA): feed, profile, settings, invites, friends, messages, outbox.
- **Capacitor / Tauri** (future): mobile and desktop shells.

---

## 6. Security Model

- E2E encryption (X25519 + ChaCha20-Poly1305).
- Signed content envelopes and invite payloads.
- Safety codes for out-of-band verification.
- Threat model v1: protects against corporate harvesting and passive observers; not against screenshots or compromised devices.

---

## Open Questions

1. Profile caching when friend is offline?
2. Web/PWA vs mobile-first for v1?
3. Media size limits for strict P2P? *(~2 MB per image, client-side compression)*
4. User-configurable post TTL? *(optional local feed archive exists)*

---

## Phased Delivery

| Phase | Scope |
|-------|-------|
| 0 | Vision and backbone alignment |
| 1 | Rust core: identity, storage, expiry |
| 2 | libp2p messaging, outbox |
| 3 | SvelteKit UI + local API |
| 4 | **Invite flow, feed, profile, settings, backup** (current) |
| 5 | Capacitor mobile shell |
| 6 | P2P blob sync, thumbnails, orphan blob GC |
