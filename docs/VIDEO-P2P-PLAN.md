# Video P2P — chunked media plan

Local-first video posts: **thumbnail always**, **full video on tap**, **512 KB chunks**, **author as seeder**, **DCUtR-aware transfers**. Multi-source swarming between friends is **planned last** (protocol hooks included).

See [VISION.md](./VISION.md) for product constraints (7d TTL, friends-only, no VPS storage).

---

## Goals

| Goal | MVP | Later |
|------|-----|-------|
| Profile/feed shows video thumb only | ✅ | — |
| Tap thumb → start fetch immediately (no confirm) | ✅ | — |
| Chunk size 512 KB | ✅ | — |
| Author seeds chunks on request | ✅ | — |
| Thumb auto-sync after envelope ACK | ✅ | — |
| Prefer direct path (DCUtR) before bulk transfer | ✅ | tune timeouts |
| Multi-source chunks in friend graph | protocol stub | ✅ Phase C |
| Progressive playback before 100% | — | optional |
| Adaptive quality | — | optional |

---

## Non-goals (this milestone)

- Public torrent / global swarms
- VPS blob storage
- Auto-download full video on post receipt
- Confirmation modal before fetch

---

## Data model

### `MediaManifest` (in encrypted `PostPayload`)

```rust
MediaKind: photo | video

MediaManifest {
  root_hash: String,       // SHA256(canonical manifest body)
  kind: video,
  mime: "video/mp4",
  total_bytes: u64,
  chunk_size: 524288,      // 512 KiB fixed
  chunk_hashes: Vec<String>,
  thumb_hash: String,
  duration_ms: u32,
}
```

### `PostPayload` (extended)

```rust
PostPayload {
  body: String,
  media_ref: Option<String>,     // root_hash (video) or blob hash (photo legacy)
  media_kind: Option<MediaKind>,
  thumb_ref: Option<String>,
  manifest: Option<MediaManifest>, // video only
}
```

**Photo (legacy):** `media_kind = photo`, `media_ref` = blob hash, no manifest.  
**Video:** `media_ref` = `root_hash`, `thumb_ref` + `manifest` required.

### Local storage

```
data/blobs/{thumb_hash}              # small JPEG
data/blobs/{root_hash}               # assembled MP4 when complete
data/blobs/chunks/{root_hash}/{i}    # raw chunk bytes (0-based index)
data/media_manifests                 # SQLite: root_hash → JSON manifest
```

---

## Sync behaviour

```mermaid
sequenceDiagram
  participant Author
  participant Friend
  participant Relay as VPS relay
  Author->>Friend: encrypted post envelope (body + manifest + thumb_ref)
  Friend-->>Author: delivery ACK
  Author->>Friend: BlobPush (thumb only)
  Note over Friend: Grid shows thumb; video not local
  Friend->>Author: user taps thumb
  Note over Friend: wait up to 5s for DCUtR direct
  loop each missing chunk
    Friend->>Author: BlobChunkRequest(root, i)
    Author-->>Friend: BlobChunkData
  end
  Note over Friend: assemble → blobs/{root_hash} → play
```

- **Never** push full video on ACK.
- **Do** push thumb on ACK (same as today for small blobs).
- On connect, pull **missing thumbs only** for video posts (not video chunks).

---

## P2P protocol (`/inertia/1.0.0`)

### MVP (implemented)

| Request | Response |
|---------|----------|
| `BlobRequest { hash }` | `BlobData` / `BlobNotFound` (photos + thumbs) |
| `BlobPush { hash, data }` | `Ok` |
| `BlobChunkRequest { root_hash, chunk_index }` | `BlobChunkData` / `BlobChunkNotFound` |

### Reserved (multi-source — Phase C)

| Request | Purpose |
|---------|---------|
| `BlobHave { root_hash, chunk_bitmap }` | Advertise which chunks a peer holds |
| `BlobChunkSources { root_hash }` | List friend peers with any chunk (future) |

Friends who finished downloading may serve chunks to other friends **within the same post TTL**. Not used in MVP fetch (author-only).

---

## DCUtR and relay cost

Already in stack: `relay::client`, `dcutr::Behaviour`, Identify (see `p2p/behaviour.rs`, `p2p/swarm_task.rs`).

Friend discovery and redial use **relay circuits only** (`filter_friend_multiaddrs`). DCUtR may still upgrade an active session to direct transport after the circuit is up.

### Connection class

On `ConnectionEstablished`, inspect remote multiaddr:

- contains `/p2p-circuit/` → **relay**
- else → **direct**

On `dcutr::Event::Success`, mark peer as **direct available**.

### Before chunk fetch

1. Resolve author `peer_id` from contact.
2. **`wait_for_direct(peer, 5s)`** if only relay connected (DCUtR may still be negotiating).
3. Start parallel chunk requests (sequential OK for MVP v1).
4. Expose `transport: "direct" | "relay" | "unknown"` in fetch status API.

Relay VPS: still **no disk**; bandwidth only when direct path fails.

---

## HTTP API (local)

| Method | Path | Purpose |
|--------|------|---------|
| `POST` | `/api/posts/video` | Create video post (body + thumb + video bytes → chunk + fan-out) |
| `POST` | `/api/media/{root_hash}/fetch` | Start/resume author chunk pull |
| `GET` | `/api/media/{root_hash}/status` | `{ state, chunks_done, chunks_total, transport, error }` |
| `GET` | `/api/blobs/{hash}` | Thumb or assembled video (existing) |

---

## UI

- Grid / feed: thumb + play badge for `media_kind === 'video'`.
- Tap thumb → `fetchMedia(root_hash)` immediately.
- Overlay progress (chunks_done / chunks_total).
- When `state === complete`, show `<video src={blobUrl(root_hash)}>`.

---

## Limits (MVP)

| Limit | Value |
|-------|-------|
| Chunk size | 512 KiB |
| Max video size | 50 MiB (after client transcode) |
| Max thumb | 256 KiB |
| Chunk request timeout | 30 s |
| DCUtR wait | 5 s |

---

## Implementation phases

### Phase A — MVP (current work)

1. `MediaManifest` + `PostPayload` + SQLite manifests table
2. Chunk store + assemble
3. `BlobChunkRequest` / handler / node client
4. Thumb-only push on ACK for video
5. `MediaFetchJob` (author-only, sequential chunks)
6. Peer transport tracking + DCUtR wait
7. API routes + web tap-to-load UI
8. `POST /posts/video` for end-to-end test

### Phase B — Hardening

- Parallel chunk fetch (same peer)
- Resume partial downloads
- Orphan chunk GC on post expiry
- Android native transcode hook

### Phase C — Friend-graph multi-source

- `BlobHave` advertisement after local complete
- Peer selection: author first, then cached friends
- Parallel fetch from multiple peers
- Rate limits per peer

---

## Testing

1. Two devices via relay; author posts video via API.
2. Friend sees thumb in profile grid; no video file yet.
3. Friend taps → progress → playback.
4. Logs show DCUtR success → `transport: direct` when NAT allows.
5. Kill author mid-fetch → resume on reconnect.

---

## Related docs

- [VISION.md](./VISION.md) — Phase 6/7 roadmap
- [inertia-relay README](../crates/inertia-relay/README.md) — relay caps and bandwidth
- [RELAY-CONNECTIVITY.md](./RELAY-CONNECTIVITY.md) — relay circuits, reservations, invite bootstrap
