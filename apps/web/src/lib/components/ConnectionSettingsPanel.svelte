<script lang="ts">
  import { api } from '$lib/api';
  import { identityState, refreshIdentity } from '$lib/identity.svelte';

  let listenPort = $state(4784);
  let relayMultiaddr = $state('');
  let p2pAnnounce = $state('');
  let shareMultiaddr = $state<string | null>(null);
  let loading = $state(true);
  let saving = $state(false);
  let message = $state('');
  let error = $state('');

  $effect(() => {
    if (identityState.apiOnline && identityState.identity) {
      loadSettings();
    }
  });

  async function loadSettings() {
    loading = true;
    error = '';
    try {
      const settings = await api.getSettings();
      listenPort = settings.p2p_listen_port;
      relayMultiaddr = settings.relay_multiaddr ?? '';
      p2pAnnounce = settings.p2p_announce ?? '';
      const share = await api.p2pShareAddress();
      shareMultiaddr = share.multiaddr;
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to load connection settings';
    } finally {
      loading = false;
    }
  }

  async function saveSettings() {
    saving = true;
    message = '';
    error = '';
    try {
      await api.updateSettings({
        p2p_listen_port: listenPort,
        relay_multiaddr: relayMultiaddr,
        p2p_announce: p2pAnnounce
      });
      message = 'Saved. Restart the API for listen port changes to take effect.';
      await refreshIdentity({ silent: true });
      const share = await api.p2pShareAddress();
      shareMultiaddr = share.multiaddr;
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to save connection settings';
    } finally {
      saving = false;
    }
  }

  async function copyShareAddress() {
    if (!shareMultiaddr) return;
    try {
      await navigator.clipboard.writeText(shareMultiaddr);
      message = 'Multiaddr copied to clipboard.';
    } catch {
      error = 'Could not copy to clipboard.';
    }
  }
</script>

<section class="card settings-section">
  <h2 class="section-title">Connection</h2>
  <p class="section-desc">
    P2P listen port, relay, and addresses advertised in invites. Environment variables
    (`INERTIA_RELAY`, `INERTIA_P2P_LISTEN_PORT`, `INERTIA_P2P_ANNOUNCE`) override saved
    values when set.
  </p>

  {#if loading}
    <p class="muted">Loading…</p>
  {:else}
    <div class="field">
      <label for="listen-port">Listen port</label>
      <input id="listen-port" type="number" min="1" max="65535" bind:value={listenPort} />
    </div>

    <div class="field">
      <label for="relay">Relay multiaddr</label>
      <input
        id="relay"
        type="text"
        placeholder="/ip4/203.0.113.10/tcp/9000/p2p/12D3Koo…"
        bind:value={relayMultiaddr}
      />
    </div>

    <div class="field">
      <label for="announce">Invite announce addresses</label>
      <textarea
        id="announce"
        rows="3"
        placeholder="/ip4/203.0.113.10/tcp/4784, comma-separated"
        bind:value={p2pAnnounce}
      ></textarea>
    </div>

    {#if shareMultiaddr}
      <div class="share-row">
        <code class="mono">{shareMultiaddr}</code>
        <button type="button" class="btn-secondary" onclick={copyShareAddress}>Copy</button>
      </div>
    {:else if identityState.p2pInfo?.peer_id}
      <p class="muted">No shareable multiaddr yet — set announce addresses or wait for P2P.</p>
    {/if}

    <div class="actions">
      <button type="button" class="btn" disabled={saving} onclick={saveSettings}>
        {saving ? 'Saving…' : 'Save connection settings'}
      </button>
    </div>

    {#if message}
      <p class="notice">{message}</p>
    {/if}
    {#if error}
      <p class="error">{error}</p>
    {/if}
  {/if}
</section>

<style>
  .field {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    margin-bottom: 0.85rem;
  }

  .field label {
    font-size: 0.75rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--muted);
  }

  .field input,
  .field textarea {
    width: 100%;
    padding: 0.55rem 0.65rem;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg);
    color: var(--text);
    font: inherit;
    font-size: 0.875rem;
  }

  .share-row {
    display: flex;
    gap: 0.5rem;
    align-items: flex-start;
    margin-bottom: 0.85rem;
  }

  .share-row code {
    flex: 1;
    word-break: break-all;
    font-size: 0.8125rem;
  }

  .actions {
    margin-top: 0.25rem;
  }

  .notice {
    margin: 0.75rem 0 0;
    font-size: 0.8125rem;
    color: var(--success);
  }

  .error {
    margin: 0.75rem 0 0;
    font-size: 0.8125rem;
    color: var(--danger);
  }

  .mono {
    font-family: monospace;
  }

  .muted {
    color: var(--muted);
  }
</style>
