<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/state';
  import { goto } from '$app/navigation';
  import { api, type Contact, type InvitePreview } from '$lib/api';
  import { ApiRequestError } from '$lib/api-errors';
  import DevSetupActions from '$lib/components/DevSetupActions.svelte';
  import InertiaLogo from '$lib/components/InertiaLogo.svelte';
  import RelayMultiaddrList from '$lib/components/RelayMultiaddrList.svelte';
  import ProfileHeader from '$lib/components/ProfileHeader.svelte';
  import { normalizeInviteInput } from '$lib/invite-input';
  import {
    identityState,
    refreshIdentity,
    setIdentity,
    startP2pInBackground
  } from '$lib/identity.svelte';

  type Path = 'invite' | 'host';

  let path = $state<Path | null>(null);
  let displayName = $state('');
  let creating = $state(false);
  let createError = $state('');

  let inviteInput = $state('');
  let preview = $state<InvitePreview | null>(null);
  let accepted = $state<Contact | null>(null);
  let inviteLoading = $state(false);
  let accepting = $state(false);
  let inviteError = $state('');

  let relayList = $state<string[]>([]);
  let relayAddError = $state('');
  let relaySaving = $state(false);
  let relayMessage = $state('');
  let relayError = $state('');
  let relaySkipped = $state(false);

  let introDone = $state(false);
  let introLeaving = $state(false);

  const INTRO_HOLD_MS = 1200;
  const INTRO_EXIT_MS = 450;

  onMount(() => {
    const reduced = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
    if (reduced) {
      introDone = true;
      return;
    }

    const leaveTimer = window.setTimeout(() => {
      introLeaving = true;
    }, INTRO_HOLD_MS);

    const doneTimer = window.setTimeout(() => {
      introDone = true;
    }, INTRO_HOLD_MS + INTRO_EXIT_MS);

    return () => {
      window.clearTimeout(leaveTimer);
      window.clearTimeout(doneTimer);
    };
  });

  const hasProfile = $derived(!!identityState.identity);
  const relayOk = $derived(identityState.p2pStatus?.relay_connected === true);
  const relayConfigured = $derived(identityState.p2pStatus?.relay_configured === true);
  const relayWarning = $derived(relayConfigured && !relayOk);

  const hostStep = $derived.by(() => {
    if (!hasProfile) return 1;
    if (!relaySkipped && !relayOk) return 2;
    return 3;
  });

  const inviteStep = $derived.by(() => {
    if (accepted) return 3;
    if (!hasProfile) return 1;
    return 2;
  });

  function readInviteFromUrl(): string {
    const params = page.url.searchParams;
    const d = params.get('d');
    const hash = page.url.hash.slice(1);
    if (d) return normalizeInviteInput(decodeURIComponent(d));
    if (hash) return normalizeInviteInput(hash);
    return '';
  }

  $effect(() => {
    page.url.hash;
    page.url.search;
    const fromUrl = readInviteFromUrl();
    if (!fromUrl) return;
    path = 'invite';
    inviteInput = fromUrl;
    if (identityState.identity && identityState.apiOnline) {
      void loadPreview();
    }
  });

  function choosePath(next: Path) {
    path = next;
    createError = '';
    inviteError = '';
    relayError = '';
    relayMessage = '';
    relaySkipped = false;
  }

  function back() {
    path = null;
    preview = null;
    accepted = null;
    inviteError = '';
    relayError = '';
    relayMessage = '';
    relaySkipped = false;
  }

  async function createProfile() {
    if (!displayName.trim()) {
      createError = 'Display name is required';
      return;
    }
    if (identityState.identity) return;

    creating = true;
    createError = '';
    try {
      const identity = await api.initIdentity(displayName.trim());
      await setIdentity(identity);
      void startP2pInBackground();
      if (path === 'invite' && inviteInput.trim()) {
        await loadPreview();
      }
    } catch (e) {
      createError =
        e instanceof Error ? e.message : 'Failed to create profile';
      await refreshIdentity();
    } finally {
      creating = false;
    }
  }

  async function loadPreview() {
    if (!inviteInput.trim()) {
      inviteError = 'Paste an invite link or code first';
      return;
    }
    inviteLoading = true;
    inviteError = '';
    preview = null;
    accepted = null;
    try {
      preview = await api.previewInvite(normalizeInviteInput(inviteInput));
    } catch (e) {
      inviteError =
        e instanceof ApiRequestError
          ? e.message
          : e instanceof Error
            ? e.message
            : 'Invalid or expired invite';
    } finally {
      inviteLoading = false;
    }
  }

  async function acceptInvite() {
    if (!inviteInput.trim()) return;
    accepting = true;
    inviteError = '';
    try {
      accepted = await api.acceptInvite(normalizeInviteInput(inviteInput));
      preview = null;
    } catch (e) {
      inviteError =
        e instanceof ApiRequestError
          ? e.message
          : e instanceof Error
            ? e.message
            : 'Failed to accept invite';
    } finally {
      accepting = false;
    }
  }

  async function saveRelay() {
    relaySaving = true;
    relayMessage = '';
    relayError = '';
    try {
      await api.updateSettings({ relay_multiaddrs: relayList });
      relayMessage = 'Saved. Waiting for relay connection…';
      await refreshIdentity({ silent: true });
      await new Promise((r) => setTimeout(r, 1500));
      await refreshIdentity({ silent: true });
      if (identityState.p2pStatus?.relay_connected) {
        relayMessage = 'Relay connected.';
      }
    } catch (e) {
      relayError = e instanceof Error ? e.message : 'Failed to save relay';
    } finally {
      relaySaving = false;
    }
  }

  function enterApp(href = '/') {
    void goto(href);
  }
</script>

<div class="welcome-login">
  {#if !introDone}
    <div class="intro-splash" class:intro-leaving={introLeaving} aria-hidden="true">
      <div class="intro-brand">
        <InertiaLogo size={44} />
        <span class="intro-title">Inertia</span>
      </div>
    </div>
  {/if}

  <div class="welcome-content" class:visible={introDone}>
    <header class="brand">
      <div class="brand-row">
        <InertiaLogo size={36} />
        <h1 class="brand-title">Inertia</h1>
      </div>
      <p class="brand-tagline">
        Distributed, P2P, local-first social network for the people you trust.
      </p>
    </header>

  {#if !identityState.apiOnline}
    <div class="panel api-panel">
      <h2 class="panel-title">Start the local API</h2>
      <p class="muted panel-lead">
        Inertia runs on your device. Start the API bridge to continue.
      </p>
      <DevSetupActions showHint />
    </div>
  {/if}

  {#if path === null}
    <div class="panel">
      <h2 class="panel-title">How are you joining?</h2>
      <p class="muted panel-lead">Pick one path. You can always invite friends later.</p>

      <div class="path-grid">
        <button type="button" class="path-card" onclick={() => choosePath('invite')}>
          <span class="path-icon" aria-hidden="true">
            <svg viewBox="0 0 24 24">
              <rect
                x="3"
                y="5"
                width="18"
                height="14"
                rx="2"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
              />
              <path
                d="M3 7l9 6 9-6"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
              />
            </svg>
          </span>
          <span class="path-label">I have an invite</span>
          <span class="path-desc muted">
            Someone sent you a link. Create your profile, paste the invite, and connect.
          </span>
        </button>

        <button type="button" class="path-card" onclick={() => choosePath('host')}>
          <span class="path-icon" aria-hidden="true">
            <svg viewBox="0 0 24 24">
              <circle cx="12" cy="12" r="8" fill="none" stroke="currentColor" stroke-width="2" />
              <circle cx="12" cy="12" r="2.5" fill="currentColor" stroke="none" />
            </svg>
          </span>
          <span class="path-label">I'm starting a network</span>
          <span class="path-desc muted">
            Host a relay on your VPS, create your profile, then invite people you trust.
          </span>
        </button>
      </div>

      <ul class="value-pills muted">
        <li>No ads or tracking</li>
        <li>No centralized database</li>
        <li>No algorithmic feeds</li>
      </ul>
    </div>
  {:else if path === 'invite'}
    <div class="panel">
      <div class="panel-head">
        <button type="button" class="back-btn" onclick={back}>← Back</button>
        <span class="step-label muted">
          Invite · {accepted ? 'done' : `step ${inviteStep} of 2`}
        </span>
      </div>

      {#if inviteStep === 1}
        <h2 class="panel-title">Create your profile</h2>
        <p class="muted panel-lead">
          One identity per device. Your keys stay on this machine.
        </p>
        <div class="field">
          <label for="welcome-name">Display name</label>
          <input
            id="welcome-name"
            bind:value={displayName}
            placeholder="Your name"
            disabled={!identityState.apiOnline || creating}
          />
        </div>
        {#if createError}<p class="error">{createError}</p>{/if}
        <button
          class="btn btn-block"
          onclick={createProfile}
          disabled={!identityState.apiOnline || creating}
        >
          {creating ? 'Creating…' : 'Continue'}
        </button>
      {:else if inviteStep === 2 && !accepted}
        <h2 class="panel-title">Paste your invite</h2>
        <p class="muted panel-lead">
          Single-use links expire in 15 minutes. Your friend should stay online while you accept.
        </p>
        <div class="field">
          <label for="welcome-invite">Invite link or code</label>
          <textarea
            id="welcome-invite"
            bind:value={inviteInput}
            rows="4"
            placeholder="Paste invite here…"
            disabled={!identityState.apiOnline}
          ></textarea>
        </div>
        {#if inviteError}<p class="error">{inviteError}</p>{/if}
        <div class="btn-row">
          <button
            class="btn btn-secondary"
            onclick={loadPreview}
            disabled={!identityState.apiOnline || inviteLoading}
          >
            {inviteLoading ? 'Checking…' : 'Preview'}
          </button>
        </div>

        {#if preview}
          <div class="preview-card">
            <ProfileHeader displayName={preview.display_name} seed={preview.signing_pubkey} size={56}>
              <p class="muted preview-copy">
                Confirm this safety code matches what they told you:
              </p>
              <p class="safety-code">{preview.safety_code}</p>
              <p class="muted preview-meta">
                Expires {new Date(preview.expires_at).toLocaleString()}
              </p>
              <p class="muted preview-meta">
                This invite includes their relay. It will be configured on your device when you
                accept.
              </p>
              {#if relayWarning}
                <p class="relay-warn">
                  Relay not connected yet. Wait a moment or try Accept anyway (can take up to a
                  minute).
                </p>
              {/if}
              <button
                class="btn btn-block"
                style="margin-top: 1rem;"
                onclick={acceptInvite}
                disabled={accepting || !identityState.apiOnline}
              >
                {accepting ? 'Connecting via relay (up to 2 min)…' : 'Accept invite'}
              </button>
            </ProfileHeader>
          </div>
        {/if}
      {:else}
        <h2 class="panel-title">You're connected</h2>
        {#if accepted}
          <ProfileHeader displayName={accepted.display_name} seed={accepted.signing_pubkey} size={56}>
            <p class="muted preview-copy">
              {accepted.display_name} is saved on this device. No central server was involved.
            </p>
          </ProfileHeader>
        {/if}
        <div class="btn-row" style="margin-top: 1.25rem;">
          <button class="btn btn-block" onclick={() => enterApp('/messages')}>Open messages</button>
          <button class="btn btn-secondary btn-block" onclick={() => enterApp('/')}>
            Go to feed
          </button>
        </div>
      {/if}
    </div>
  {:else if path === 'host'}
    <div class="panel">
      <div class="panel-head">
        <button type="button" class="back-btn" onclick={back}>← Back</button>
        <span class="step-label muted">Host · step {hostStep} of 3</span>
      </div>

      {#if hostStep === 1}
        <h2 class="panel-title">Create your profile</h2>
        <p class="muted panel-lead">
          You will invite friends after your relay is set up. One profile per device.
        </p>
        <div class="field">
          <label for="welcome-host-name">Display name</label>
          <input
            id="welcome-host-name"
            bind:value={displayName}
            placeholder="Your name"
            disabled={!identityState.apiOnline || creating}
          />
        </div>
        {#if createError}<p class="error">{createError}</p>{/if}
        <button
          class="btn btn-block"
          onclick={createProfile}
          disabled={!identityState.apiOnline || creating}
        >
          {creating ? 'Creating…' : 'Continue'}
        </button>
      {:else if hostStep === 2}
        <h2 class="panel-title">Connect your relay</h2>
        <p class="muted panel-lead">
          Run <code>inertia-relay</code> on a VPS you control (see the relay README in the repo).
          Copy the multiaddr from its startup logs and paste it below. Every invite you send will
          include this relay so friends can reach you.
        </p>
        <div class="field">
          <label for="welcome-relay">Relay multiaddr</label>
          <RelayMultiaddrList
            bind:relays={relayList}
            bind:addError={relayAddError}
            inputId="welcome-relay"
            disabled={!identityState.apiOnline || relaySaving}
          />
        </div>
        {#if relayError}<p class="error">{relayError}</p>{/if}
        {#if relayMessage}<p class="success-msg">{relayMessage}</p>{/if}
        {#if relayOk}
          <p class="success-msg">Relay connected.</p>
        {/if}
        <button
          class="btn btn-block"
          onclick={saveRelay}
          disabled={!identityState.apiOnline || relaySaving || relayList.length === 0}
        >
          {relaySaving ? 'Saving…' : 'Save relay'}
        </button>
        <button
          type="button"
          class="link-btn muted"
          onclick={() => {
            relaySkipped = true;
            relayMessage = '';
            relayError = '';
          }}
        >
          Skip for now (you can add this in Settings later)
        </button>
      {:else}
        <h2 class="panel-title">Ready to invite</h2>
        <p class="muted panel-lead">
          Generate a one-time link from Connections and share it over SMS or in person. Stay online
          while they accept.
        </p>
        {#if relayOk}
          <p class="success-msg">Relay OK. Your invites will include this network.</p>
        {:else if relayConfigured}
          <p class="relay-warn">
            Relay saved but not connected yet. Wait before inviting, or finish setup in Settings.
          </p>
        {:else}
          <p class="muted preview-meta">
            No relay configured yet. Add a VPS relay in Settings and wait for Relay OK before inviting.
          </p>
        {/if}
        <div class="btn-row" style="margin-top: 1.25rem;">
          <button class="btn btn-block" onclick={() => enterApp('/connections')}>
            Invite a friend
          </button>
          <button class="btn btn-secondary btn-block" onclick={() => enterApp('/')}>
            Go to feed
          </button>
        </div>
      {/if}
    </div>
  {/if}
  </div>
</div>

<style>
  .welcome-login {
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 1rem;
    position: relative;
  }

  .intro-splash {
    position: fixed;
    inset: 0;
    z-index: 20;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--bg);
    animation: intro-enter 0.55s ease both;
  }

  .intro-splash.intro-leaving {
    animation: intro-leave 0.45s ease forwards;
  }

  .intro-brand {
    display: flex;
    align-items: center;
    gap: 0.65rem;
    animation: intro-brand-in 0.65s ease 0.08s both;
  }

  .intro-title {
    font-size: 1.85rem;
    font-weight: 700;
    letter-spacing: -0.03em;
    line-height: 1;
  }

  .welcome-content {
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 1rem;
    opacity: 0;
    transform: translateY(10px);
    transition:
      opacity 0.45s ease,
      transform 0.45s ease;
  }

  .welcome-content.visible {
    opacity: 1;
    transform: none;
  }

  @keyframes intro-enter {
    from {
      opacity: 0;
    }
    to {
      opacity: 1;
    }
  }

  @keyframes intro-leave {
    from {
      opacity: 1;
    }
    to {
      opacity: 0;
    }
  }

  @keyframes intro-brand-in {
    from {
      opacity: 0;
      transform: scale(0.94) translateY(6px);
    }
    to {
      opacity: 1;
      transform: scale(1) translateY(0);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .intro-splash,
    .intro-brand,
    .welcome-content {
      animation: none;
      transition: none;
    }
  }

  .brand {
    text-align: center;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 0.25rem;
  }

  .brand-row {
    display: flex;
    align-items: center;
    gap: 0.55rem;
  }

  .brand-title {
    margin: 0;
    font-size: 1.65rem;
    font-weight: 700;
    letter-spacing: -0.03em;
  }

  .brand-tagline {
    margin: 0;
    font-size: 0.92rem;
    line-height: 1.5;
    color: var(--muted);
    max-width: 26rem;
  }

  .panel {
    width: 100%;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    padding: 1.25rem;
  }

  .api-panel {
    border-color: color-mix(in srgb, var(--warning) 35%, var(--border));
  }

  .panel-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
    margin-bottom: 1rem;
  }

  .panel-title {
    margin: 0 0 0.5rem;
    font-size: 1.1rem;
    font-weight: 600;
  }

  .panel-lead {
    margin: 0 0 1rem;
    font-size: 0.875rem;
    line-height: 1.5;
  }

  .muted {
    color: var(--muted);
  }

  .path-grid {
    display: grid;
    gap: 0.65rem;
    margin-bottom: 1rem;
  }

  .path-card {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 0.35rem;
    width: 100%;
    padding: 1rem;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--bg);
    color: var(--text);
    font: inherit;
    text-align: left;
    cursor: pointer;
    transition:
      border-color 0.15s,
      background 0.15s;
  }

  .path-card:hover {
    border-color: color-mix(in srgb, var(--accent) 45%, var(--border));
    background: color-mix(in srgb, var(--accent) 6%, var(--bg));
  }

  .path-icon {
    display: flex;
    line-height: 0;
    color: var(--text);
  }

  .path-icon svg {
    width: 1.25rem;
    height: 1.25rem;
    display: block;
  }

  .path-label {
    font-weight: 600;
    font-size: 0.95rem;
  }

  .path-desc {
    font-size: 0.82rem;
    line-height: 1.45;
  }

  .value-pills {
    list-style: none;
    margin: 0;
    padding: 0.75rem 0 0;
    border-top: 1px solid var(--border);
    display: flex;
    flex-wrap: wrap;
    gap: 0.4rem;
    font-size: 0.75rem;
  }

  .value-pills li {
    padding: 0.2rem 0.55rem;
    border: 1px solid var(--border);
    border-radius: 999px;
    background: color-mix(in srgb, var(--border) 12%, transparent);
  }

  .back-btn {
    padding: 0;
    border: none;
    background: none;
    color: var(--accent);
    font: inherit;
    font-size: 0.85rem;
    font-weight: 500;
    cursor: pointer;
  }

  .back-btn:hover {
    text-decoration: underline;
  }

  .step-label {
    font-size: 0.78rem;
  }

  .btn-block {
    width: 100%;
    margin-top: 0.25rem;
  }

  .btn-row {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .preview-card {
    margin-top: 1rem;
    padding-top: 1rem;
    border-top: 1px solid var(--border);
  }

  .preview-copy {
    margin: 0.5rem 0 0;
    font-size: 0.85rem;
  }

  .preview-meta {
    margin: 0.5rem 0 0;
    font-size: 0.8rem;
    line-height: 1.45;
  }

  .safety-code {
    margin: 0.65rem 0 0;
    font-family: ui-monospace, monospace;
    font-size: 1.35rem;
    letter-spacing: 0.12em;
  }

  .relay-warn {
    margin: 0.75rem 0 0;
    font-size: 0.8rem;
    line-height: 1.45;
    color: var(--warning);
  }

  .success-msg {
    margin: 0.75rem 0 0;
    font-size: 0.85rem;
    color: var(--success);
  }

  .link-btn {
    display: block;
    width: 100%;
    margin-top: 0.75rem;
    padding: 0.35rem 0;
    border: none;
    background: none;
    font: inherit;
    font-size: 0.8rem;
    text-align: center;
    cursor: pointer;
    text-decoration: underline;
  }

  .panel-lead code {
    font-size: 0.85em;
    padding: 0.1em 0.35em;
    border-radius: 4px;
    background: color-mix(in srgb, var(--border) 35%, transparent);
  }
</style>
