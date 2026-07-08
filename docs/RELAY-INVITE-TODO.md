# Relay + Invite work - resume notes

Working branch: `fix/relay-only-p2p` (base `development`).
Status: relay-only P2P committed (`efa5818`); the P2P swarm ACTOR REWRITE plus
invite click-and-send are UNCOMMITTED in the working tree.

## TL;DR of the problem we are solving

- "Relay OK" only means an OUTBOUND session to the VPS (fast, ~10ms).
- To be reachable by a friend you also need an INBOUND slot on the VPS
  (a circuit "reservation"). Without `relay reservation active` you can
  generate an invite but nobody can dial you, so accept fails with
  "could not reach the inviter".
- The relay is NOT single-connection. VPS caps are dozens of reservations and
  circuits (see `crates/inertia-relay/src/config.rs`).

## Architecture now: swarm actor (rewritten)

The old `Arc<Mutex<Swarm>>` + polling event loop caused missed reservation
events, 12s timeouts, and a startup deadlock. It was replaced by an actor:

- `crates/inertia-core/src/p2p/swarm_task.rs` - single task exclusively owns
  the swarm; `tokio::select!` over `swarm.next()` and a command channel
  (`Dial`, `EnsureRelayCircuits`, `SendRequest`, `SendResponse`).
- `crates/inertia-core/src/p2p/node.rs` - `P2pNode` is now a thin, Clone-able
  handle: command sender + `watch::Receiver<NetState>` (connected peers,
  reservations, direct peers, listen/external addrs). All method signatures
  preserved; no mutex on the swarm anywhere.
- Waits are event-driven (`watch::Receiver::wait_for` + timeout), no sleep
  polling. Reservation confirms in ~150ms after boot in local testing.
- Inbound requests are handled in spawned tasks and reply through the command
  channel, so slow store access never stalls the swarm.
- Self-healing: relay reconnect or a dead circuit listener automatically
  re-listens (3s backoff) and re-reserves. No engine involvement.
- `crates/inertia-core/src/engine/relay_dial.rs` - simplified to
  dial + wait_for_any_connected + ensure_relay_circuits + wait_for_reservation.
  Retry scaffolding and `RELAY_RESERVATION_PAUSE` removed.

Also fixed on this branch:

- `inertia-relay` now confirms an external address (from identify candidates,
  or `INERTIA_RELAY_PUBLIC_ADDR` env). Without it, reservation responses had
  no addresses and clients rejected them in a listen/close loop. REDEPLOY THE
  RELAY on the VPS after merging.
- `send_message` self-deadlock: it held the engine p2p guard while
  `emit_message_sent_ui` tried to lock p2p again for a status snapshot. The
  send now releases the guard first. This was latent before the rewrite and
  made POST /messages hang forever.

## Verified end-to-end (local, two instances + local relay)

1. Boot: `relay reservation active` ~150ms after `peer connected` to relay.
2. Generate invite: instant (reservation already held), payload contains the
   `/p2p-circuit/` address.
3. Accept invite on second instance: ~0.3s, contact appears on both sides.
4. Messages both directions, outbox `delivered` with acks.

Repeat on real hardware (desktop + phone via VPS relay):

1. Redeploy `inertia-relay` on the VPS (external-address fix). Optionally set
   `INERTIA_RELAY_PUBLIC_ADDR=/ip4/<VPS_IP>/tcp/9000` in the compose env.
2. Desktop `npm run api`; expect `relay reservation active` within ~1s of the
   relay session, exactly one `listening via relay circuit` per relay.
3. Friends -> Add -> Generate (should be near-instant). Use
   "Copy for phone (paste only)".
4. Phone Stage B (`npm run android:stage-b && npm run android:run`), same relay
   in Settings, paste payload -> Preview -> Accept.
5. Watch desktop logs for `invite redeemed friend=<name>`, then exchange a
   message both ways.

## STILL TO DO / open questions

- [ ] Device test: desktop <-> phone accept + message via the VPS relay
      (local two-instance test passed; hardware not yet re-verified).
- [ ] Redeploy `inertia-relay` on the VPS before the device test.
- [ ] Multi-relay: verify accepter can use a relay it did not previously have
      configured (apply_relay_from_invite).
- [ ] Clean up em dashes in OLD Rust user-facing strings (repo rule forbids
      them; new actor code is clean, some `engine/*.rs` strings still have
      them).
- [ ] Consider trimming the web invite timeouts (CREATE 90s / ACCEPT 180s in
      `apps/web/src/lib/api.ts`) now that create is sub-second.

## COMMIT / PR PLAN (per git-and-scope rule)

On `fix/relay-only-p2p`:
1. Already committed: `efa5818` relay-only friend paths.
2. Pending (after device test): swarm actor rewrite + relay external-address
   fix + send_message deadlock fix + invite click-and-send. Suggested message:
   `fix: rewrite p2p node as swarm actor so relay reservations and invites are reliable`

Do NOT push until desktop <-> phone accept is verified. PR targets `development`.
Label: `bugfix`. Do not commit `.idea/`.

## KEY FILES

- `crates/inertia-core/src/p2p/swarm_task.rs` - NEW: swarm actor (commands,
  NetState watch, event handling, circuit self-heal).
- `crates/inertia-core/src/p2p/node.rs` - thin Clone-able handle over the actor.
- `crates/inertia-core/src/engine/relay_dial.rs` - simplified bootstrap waits.
- `crates/inertia-core/src/engine/messaging.rs` - send_message guard fix.
- `crates/inertia-relay/src/main.rs` - external address confirmation.
- `crates/inertia-core/src/engine/invite.rs` - create/accept, readiness.
- `crates/inertia-api/src/routes/invite.rs` - bootstrap outside engine lock,
  `/invite/readiness`.
- `apps/web/src/lib/api.ts` / `friends/add/+page.svelte` / `WelcomeLogin.svelte`
  - invite UX and timeouts.
