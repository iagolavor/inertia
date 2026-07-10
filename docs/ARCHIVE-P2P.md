# Archive P2P transfer policy

Shared-folder (**Files** tab) entries are author-hosted blobs. Friends pull them on demand. They are never fan-out to inboxes.

## Local ingest (author device)

1. UI zips a dropped folder (or multi-file pick) with `fflate`, or accepts a single file / `.zip` as-is.
2. Chunked upload to the local API:
   - `POST /archive/folders/:id/uploads` - begin
   - `PUT /archive/uploads/:id/chunks/:index` - raw bytes + `x-chunk-hash` (SHA-256)
   - `GET /archive/uploads/:id` - missing chunks for resume
   - `POST /archive/uploads/:id/complete` - durable `MediaManifest` + `archive_entries` row
3. Chunk size matches core `CHUNK_SIZE` (512 KiB). No product-wide file size cap on this path; disk is the limit. Soft UI warning around ~200 MiB for browser zip CPU/memory.
4. Legacy base64 `POST .../entries` remains for compat but is unused by the Files tab.

## Peer download (friend device)

1. Friend browses folder index from live `ProfileManifest` / archive list when the author is online.
2. Download calls `POST /media/:root_hash/fetch?author_contact_id=…&direct_required=true`.
3. Engine waits ~30s for a **DCUtR / direct** path. If the peer is still only reachable via `/p2p-circuit/`, the fetch **fails closed** with `direct_required`. Archive bulk bytes never ride the relay.
4. Chunk loop skips chunks already on disk (`chunk_exists`) so a failed attempt can resume.
5. UI shows `chunks_done / chunks_total` and transport (`direct` vs error).

## Contrast with video / photos

Video and profile photo fetches may still prefer direct and fall back to relay. Only archive entries use the direct-required policy.

## Security notes

- Serve path remains contact-gated (`blob_is_servable` / `chunk_root_is_servable`).
- Durable refs keep archive blobs out of feed TTL purge. See [SECURITY-TODO.md](./SECURITY-TODO.md).
