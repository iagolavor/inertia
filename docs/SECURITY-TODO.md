# Security hardening — deferred TODO

Track security and trust-boundary work here. **Do not prioritize this list until:**

1. **Operational network** — brother E2E (or equivalent) proves invite → P2P → envelope → blob sync on real devices/relay, not just local dev.
2. **Proper data reconstruction on the web** — feed, inbox, messages, and media render reliably from local SQLite + blobs after refresh, reconnect, and restore (backup/restore or equivalent UX).

Until those gates pass, focus stays on delivery and correctness.

---

## P2P trust and spam

- [ ] **Contact-gate inbound envelopes** — reject or quarantine `SendEnvelope` when `author_signing_pubkey` is not in `contacts` (today: signature + decrypt succeed for any peer who knows your `encryption_pubkey`).
- [ ] **Sign `InviteRedemption`** — mutual cryptographic proof on friend accept (inviter currently trusts unsigned redemption + nonce + dial target).
- [ ] **Signed `FriendAccept` response** — wire up and require on the main invite path; retire or harden unused `FriendRequest` handling.
- [ ] **Envelope replay table** — track seen `(content_id, sender_id)` or similar; reject duplicates before inbox insert (expiry alone is not enough).
- [ ] **P2P rate limits** — cap inbound requests per peer (envelopes, blob push/pull) to reduce friend-or-stranger resource exhaustion.

## Crypto hygiene

- [ ] **HKDF shared secret** — derive ChaCha20 key from X25519 output instead of using raw ECDH bytes.
- [ ] **Forward secrecy (stretch)** — per-session or Double Ratchet-style key evolution; document tradeoffs for ephemeral 7d content.

## Local boundary and keys

- [ ] **Private keys out of plaintext SQLite** — OS keychain (mobile/desktop) or encrypted key blob; migration path for existing `data/` installs.
- [ ] **API auth on non-localhost** - required for Capacitor/mobile; token or IPC boundary so other processes cannot drive `127.0.0.1:4783`. **Android on-device install** already binds API to loopback only; auth still needed before any LAN/0.0.0.0 exposure.
- [ ] **Tighten CORS** — replace `Any` origin when API bind policy is defined per platform.

## Blobs and media

- [x] **Blob fetch authorization** - serve profile/archive/feed/inbox hashes only to known contacts (`peer_is_known_contact` + `blob_is_servable` / `chunk_root_is_servable` in P2P handlers). Archive peer downloads additionally require DCUtR/direct (no relay bulk); see [ARCHIVE-P2P.md](./ARCHIVE-P2P.md).
- [ ] **Orphan blob GC** - Phase 7; limit disk growth from abandoned or never-referenced blobs. Must call `Store::durable_blob_refs()` so profile items, shared folders, and avatars are never deleted when feed copies expire.

## Metadata and ops

- [ ] **Document relay metadata threat** — who connects, when, sizes; set expectations in user-facing copy if needed.
- [ ] **Relay resource limits** — monitor reservations/circuits when friend count or user count on shared VPS grows.

---

## Reference

Threat model v1: [VISION.md](./VISION.md) § Security Model — passive observers and harvesting, not compromised devices or nation-states.

Scaling shape: O(friends) fan-out per post; designed for small circles, not influencer-scale graphs.
