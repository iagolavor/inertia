<script lang="ts">
  import FeedHistoryPanel from '$lib/components/FeedHistoryPanel.svelte';
  import ThemeToggle from '$lib/components/ThemeToggle.svelte';
  import { identityState } from '$lib/identity.svelte';
</script>

<h1 class="page-title">Settings</h1>

<section class="card settings-section">
  <h2 class="section-title">Appearance</h2>
  <p class="section-desc">Theme for this device.</p>
  <ThemeToggle />
</section>

{#if identityState.loading}
  <p class="empty">Loading…</p>
{:else if !identityState.apiOnline}
  <section class="card settings-section">
    <h2 class="section-title">Feed & backup</h2>
    <p class="section-desc muted">Start the API bridge to manage feed history and backups.</p>
    <pre class="cmd">cargo run -p inertia-api</pre>
  </section>
{:else if identityState.identity}
  <section class="card settings-section">
    <h2 class="section-title">Feed & backup</h2>
    <FeedHistoryPanel />
  </section>

  <section class="card settings-section">
    <h2 class="section-title">Identity</h2>
    <p class="section-desc">
      Cryptographic keys for this device. Use the safety code to verify invites.
    </p>

    <dl class="identity-list">
      <div class="identity-row">
        <dt>Name</dt>
        <dd>{identityState.identity.display_name}</dd>
      </div>
      <div class="identity-row">
        <dt>Signing key</dt>
        <dd class="mono">{identityState.identity.signing_pubkey}</dd>
      </div>
      <div class="identity-row">
        <dt>Encryption key</dt>
        <dd class="mono">{identityState.identity.encryption_pubkey}</dd>
      </div>
      <div class="identity-row">
        <dt>Safety code</dt>
        <dd class="mono">{identityState.identity.signing_pubkey.slice(0, 8)}</dd>
      </div>
      <div class="identity-row">
        <dt>Peer ID</dt>
        <dd class="mono">
          {#if identityState.p2pInfo?.peer_id}
            {identityState.p2pInfo.peer_id}
          {:else}
            <span class="muted">Starting P2P…</span>
          {/if}
        </dd>
      </div>
    </dl>

    <p class="badge-local">Stored on this device only</p>
  </section>
{:else}
  <section class="card settings-section">
    <h2 class="section-title">Identity</h2>
    <p class="section-desc muted">
      No profile yet. Create one on the Profile tab to see your keys.
    </p>
    <p style="margin-top: 0.75rem;">
      <a class="btn" href="/profile">Go to profile</a>
    </p>
  </section>
{/if}

<style>
  .page-title {
    margin: 0 0 1rem;
    font-size: 1.35rem;
    font-weight: 700;
    letter-spacing: -0.02em;
  }

  .settings-section {
    margin-bottom: 1rem;
  }

  .section-title {
    margin: 0 0 0.35rem;
    font-size: 1rem;
    font-weight: 600;
  }

  .section-desc {
    margin: 0 0 0.85rem;
    font-size: 0.875rem;
    color: var(--muted);
    line-height: 1.45;
  }

  .identity-list {
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .identity-row {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
  }

  .identity-row dt {
    font-size: 0.75rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--muted);
  }

  .identity-row dd {
    margin: 0;
    font-size: 0.8125rem;
    word-break: break-all;
  }

  .mono {
    font-family: monospace;
  }

  .badge-local {
    display: inline-block;
    margin: 1rem 0 0;
    padding: 0.2rem 0.55rem;
    border-radius: 999px;
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--success);
    background: var(--badge-success-bg);
  }

  .cmd {
    background: var(--bg);
    padding: 1rem;
    border-radius: 8px;
    overflow-x: auto;
    margin: 0;
    font-size: 0.85rem;
  }

  .muted {
    color: var(--muted);
  }
</style>
