<script lang="ts">
  import { onMount } from 'svelte';
  import QRCode from 'qrcode';
  import { api, type Contact, type InviteReadiness, type InviteResponse } from '$lib/api';
  import { ApiRequestError } from '$lib/api-errors';
  import Avatar from '$lib/components/Avatar.svelte';
  import {
    contactWithLivePresence,
    effectiveConnectionState,
    hasContactRoute,
    presenceIndicator
  } from '$lib/dmThreads';
  import { identityState } from '$lib/identity.svelte';

  let contacts = $state<Contact[]>([]);
  let invite = $state<InviteResponse | null>(null);
  let qrDataUrl = $state('');
  let loading = $state(true);
  let generating = $state(false);
  let removingId = $state<string | null>(null);
  let error = $state('');
  let copied = $state(false);
  let copiedPayload = $state(false);
  let readiness = $state<InviteReadiness | null>(null);
  let searchQuery = $state('');

  const filteredContacts = $derived.by(() => {
    const q = searchQuery.trim().toLowerCase();
    const live = contacts.map((c) =>
      contactWithLivePresence(c, identityState.p2pStatus?.connected_peer_ids)
    );
    if (!q) return live;
    return live.filter((c) => {
      const name = c.display_name.toLowerCase();
      const code = c.signing_pubkey.slice(0, 8).toLowerCase();
      return name.includes(q) || code.includes(q);
    });
  });

  function rosterStatusLabel(contact: Contact): string {
    const state = effectiveConnectionState(contact);
    if (state === 'online') return 'connected';
    if (state === 'reachable') return 'reachable';
    if (!hasContactRoute(contact)) return 'no route yet';
    return 'unreachable';
  }

  onMount(load);

  async function load() {
    loading = true;
    error = '';
    try {
      const [contactList, inviteReady] = await Promise.all([
        api.listContacts(),
        api.inviteReadiness().catch(() => null)
      ]);
      contacts = contactList;
      readiness = inviteReady;
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to load contacts';
    } finally {
      loading = false;
    }
  }

  async function generateInvite() {
    generating = true;
    error = '';
    copied = false;
    try {
      invite = await api.createInvite();
      readiness = await api.inviteReadiness().catch(() => readiness);
      qrDataUrl = await QRCode.toDataURL(invite.link, { margin: 2, width: 256 });
    } catch (e) {
      error = e instanceof ApiRequestError ? e.message : e instanceof Error ? e.message : 'Failed to create invite';
    } finally {
      generating = false;
    }
  }

  async function copyPayload() {
    if (!invite) return;
    await navigator.clipboard.writeText(invite.payload);
    copiedPayload = true;
    setTimeout(() => (copiedPayload = false), 2000);
  }

  async function copyLink() {
    if (!invite) return;
    await navigator.clipboard.writeText(invite.link);
    copied = true;
    setTimeout(() => (copied = false), 2000);
  }

  function shareViaSms() {
    if (!invite) return;
    const body = encodeURIComponent(`Connect with me on Inertia: ${invite.link}`);
    window.open(`sms:?body=${body}`, '_blank');
  }

  async function removeConnection(contact: Contact) {
    if (removingId) return;
    const ok = confirm(`Remove connection with ${contact.display_name}?`);
    if (!ok) return;
    removingId = contact.id;
    error = '';
    try {
      await api.deleteContact(contact.id);
      contacts = contacts.filter((c) => c.id !== contact.id);
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to remove connection';
    } finally {
      removingId = null;
    }
  }
</script>

<div class="page-head">
  <div class="page-intro">
    <h1 class="page-title">Connections</h1>
    <p class="subtitle">
      <a href="/messages">← Back to messages</a> · Manage who you are connected to, or share an invite.
      No global directory.
    </p>
  </div>
</div>

<div class="card action-card">
  <h3>Invite someone</h3>
  <p style="color: var(--muted); font-size: 0.875rem; margin-bottom: 1rem;">
    Each link works <strong>once</strong> and expires in <strong>15 minutes</strong>. Stay online with the app open while your friend accepts.
    Send via SMS, iMessage, or show the QR in person.
    On another phone: tap <strong>Copy for phone</strong>, open Inertia → <strong>Accept invite</strong>, paste, Preview. Do not tap the link in Messages.
  </p>

  {#if error}<p class="error">{error}</p>{/if}

  {#if readiness && !readiness.ready}
    <p style="color: var(--warning); font-size: 0.875rem; margin: 0 0 1rem;">
      {readiness.message}
    </p>
  {/if}

  <button class="btn" onclick={generateInvite} disabled={generating}>
    {generating ? (readiness?.ready ? 'Generating…' : 'Preparing relay…') : 'Generate invite link'}
  </button>

  {#if invite}
    <div style="margin-top: 1.25rem; display: flex; flex-direction: column; gap: 1rem; align-items: flex-start;">
      {#if qrDataUrl}
        <img src={qrDataUrl} alt="Invite QR code" width="256" height="256" style="border-radius: 8px;" />
      {/if}

      <p style="color: var(--muted); font-size: 0.875rem; margin: 0;">
        Safety code: <strong style="color: var(--text); font-family: monospace;">{invite.safety_code}</strong>
        - ask them to confirm this matches before accepting.
      </p>

      <p class="invite-code">{invite.link}</p>

      <div style="display: flex; gap: 0.5rem; flex-wrap: wrap;">
        <button class="btn btn-secondary" onclick={copyLink}>{copied ? 'Copied!' : 'Copy link'}</button>
        <button class="btn btn-secondary" onclick={copyPayload}>{copiedPayload ? 'Copied!' : 'Copy for phone (paste only)'}</button>
        <button class="btn btn-secondary" onclick={shareViaSms}>Share via SMS</button>
      </div>

      <p style="color: var(--warning); font-size: 0.8rem; margin: 0;">
        Single-use · expires in 15 min · you must be online when they accept
      </p>
      <p style="color: var(--muted); font-size: 0.75rem; margin: 0;">
        Expires {new Date(invite.expires_at).toLocaleString()}
      </p>
    </div>
  {/if}
</div>

<div class="card action-card">
  <h3>Accept an invite</h3>
  <p style="color: var(--muted); font-size: 0.875rem; margin-bottom: 1rem;">
    Use <strong>Menu → Accept invite</strong> in the header, or open the accept page below. Paste the
    invite code, Preview, then Accept. On another phone: tap <strong>Copy for phone</strong> above,
    then paste there (do not tap the link in Messages).
  </p>

  <a class="btn btn-secondary" href="/invite">Accept invite</a>
</div>

<section class="roster">
  <h3>Your connections</h3>

  {#if loading}
    <p class="empty">Loading...</p>
  {:else if contacts.length === 0}
    <p class="empty">No connections yet. Generate an invite and send it to someone you trust.</p>
  {:else}
    <label class="search">
      <span class="sr-only">Search connections</span>
      <input
        type="search"
        bind:value={searchQuery}
        placeholder="Search by name or safety code"
        autocomplete="off"
      />
    </label>

    {#if filteredContacts.length === 0}
      <p class="empty">No connections match “{searchQuery.trim()}”.</p>
    {:else}
      {#each filteredContacts as contact (contact.id)}
        {@const state = effectiveConnectionState(contact)}
        {@const statusLabel = rosterStatusLabel(contact)}
        <div class="card contact-card">
          <div class="contact-row">
            <Avatar seed={contact.signing_pubkey} alt={contact.display_name} size={32} />
            <div class="contact-meta">
              <div class="contact-top">
                <strong class="contact-name">{contact.display_name}</strong>
                <span
                  class="status-chip"
                  class:connected={state === 'online'}
                  class:reachable={state === 'reachable'}
                  class:unreachable={state === 'unreachable' || state === 'offline'}
                  title={statusLabel}
                >
                  <span class="status-dot" aria-hidden="true">{presenceIndicator(contact)}</span>
                  {statusLabel}
                </span>
              </div>
              <p class="contact-detail">
                <span>Safety code: {contact.signing_pubkey.slice(0, 8)}</span>
                {#if contact.peer_id}
                  <span class="peer-id">Peer: {contact.peer_id}</span>
                {/if}
              </p>
            </div>
            <button
              type="button"
              class="btn-remove"
              disabled={removingId === contact.id}
              onclick={() => void removeConnection(contact)}
            >
              {removingId === contact.id ? 'Removing…' : 'Remove'}
            </button>
          </div>
        </div>
      {/each}
    {/if}
  {/if}
</section>

<style>
  .page-head {
    margin-bottom: 1.25rem;
  }

  .page-title {
    margin: 0 0 0.25rem;
    font-size: 1.35rem;
    font-weight: 700;
    letter-spacing: -0.02em;
  }

  .subtitle {
    color: var(--muted);
    margin: 0;
    font-size: 0.9rem;
  }

  .action-card {
    padding: 0.75rem 1.25rem 1.1rem;
  }

  .action-card h3 {
    margin: 0 0 0.45rem;
    font-size: 1rem;
  }

  .action-card :global(.btn) {
    padding: 0.35rem 0.75rem;
    font-size: 0.8rem;
    font-weight: 600;
    border-radius: 6px;
    gap: 0.35rem;
  }

  .invite-code {
    color: var(--muted);
    font-size: 0.8rem;
    word-break: break-all;
    margin: 0;
    font-family: monospace;
    user-select: text;
    -webkit-user-select: text;
    pointer-events: none;
  }

  .roster {
    margin-top: 1.5rem;
  }

  .roster h3 {
    margin: 0 0 0.55rem;
    font-size: 0.95rem;
  }

  .search {
    display: block;
    margin: 0 0 0.6rem;
  }

  .search input {
    width: 100%;
    box-sizing: border-box;
    padding: 0.4rem 0.65rem;
    border: 1px solid var(--border);
    border-radius: 7px;
    background: var(--surface);
    color: var(--text);
    font: inherit;
    font-size: 0.82rem;
  }

  .search input::placeholder {
    color: var(--muted);
  }

  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }

  .contact-card {
    padding: 0.55rem 0.7rem;
    margin-bottom: 0.45rem;
    border-radius: var(--radius-md, 8px);
  }

  .contact-row {
    display: flex;
    align-items: center;
    gap: 0.55rem;
  }

  .contact-meta {
    min-width: 0;
    flex: 1;
  }

  .contact-top {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 0.35rem;
  }

  .contact-name {
    font-size: 0.85rem;
    font-weight: 600;
  }

  .status-chip {
    display: inline-flex;
    align-items: center;
    gap: 0.22rem;
    padding: 0.08rem 0.4rem 0.08rem 0.28rem;
    border-radius: 999px;
    border: 1px solid var(--border);
    background: color-mix(in srgb, var(--bg) 55%, var(--surface));
    color: var(--muted);
    font-size: 0.65rem;
    font-weight: 600;
    letter-spacing: 0.01em;
    line-height: 1.2;
    text-transform: none;
  }

  .status-dot {
    font-size: 0.55rem;
    line-height: 1;
  }

  .status-chip.connected {
    color: var(--connection-live);
    border-color: color-mix(in srgb, var(--connection-live) 35%, var(--border));
    background: color-mix(in srgb, var(--connection-live) 10%, var(--surface));
  }

  .status-chip.reachable {
    color: var(--connection-reachable);
    border-color: color-mix(in srgb, var(--connection-reachable) 35%, var(--border));
    background: color-mix(in srgb, var(--connection-reachable) 10%, var(--surface));
  }

  .status-chip.unreachable {
    color: var(--muted);
    border-style: dashed;
    background: transparent;
  }

  .contact-detail {
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
    color: var(--muted);
    font-size: 0.72rem;
    line-height: 1.3;
    margin: 0.15rem 0 0;
  }

  .peer-id {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .btn-remove {
    flex-shrink: 0;
    padding: 0.25rem 0.5rem;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: var(--danger);
    font: inherit;
    font-size: 0.72rem;
    font-weight: 600;
    cursor: pointer;
  }

  .btn-remove:hover:not(:disabled) {
    background: color-mix(in srgb, var(--danger) 12%, var(--surface));
  }

  .btn-remove:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }
</style>
