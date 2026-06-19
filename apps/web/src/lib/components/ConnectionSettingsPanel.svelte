<script lang="ts">
  import { api } from '$lib/api';
  import { identityState, refreshIdentity } from '$lib/identity.svelte';

  let listenPort = $state(4784);
  let relayMultiaddr = $state('');
  let p2pAnnounce = $state('');
  let webOrigin = $state('');
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

  async function refreshShareMultiaddr() {
    const { multiaddr } = await api.p2pShareAddress();
    shareMultiaddr = multiaddr;
  }

  async function loadSettings() {
    loading = true;
    error = '';
    try {
      const [settings, { multiaddr }] = await Promise.all([
        api.getSettings(),
        api.p2pShareAddress()
      ]);
      const { p2p_listen_port, relay_multiaddr, p2p_announce, web_origin } = settings;
      listenPort = p2p_listen_port;
      relayMultiaddr = relay_multiaddr ?? '';
      p2pAnnounce = p2p_announce ?? '';
      webOrigin = web_origin ?? '';
      shareMultiaddr = multiaddr;
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
        p2p_announce: p2pAnnounce,
        web_origin: webOrigin
      });
      message = 'Saved. Restart the API for listen port changes to take effect.';
      await refreshIdentity({ silent: true });
      await refreshShareMultiaddr();
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
  <p class="muted">
    How this device listens and connects to friends. Set relay and announce addresses so
    invites work across networks.
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
        placeholder="/ip4/203.0.113.10/tcp/9000/p2p/12D3KooW…"
        bind:value={relayMultiaddr}
      />
      <p class="field-hint">
        Full multiaddr from the VPS relay logs — include IP, port, and peer id. Do not paste only
        the <code>12D3KooW…</code> id.
      </p>
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

    <div class="field">
      <label for="web-origin">Invite link base URL</label>
      <input
        id="web-origin"
        type="url"
        placeholder="http://192.168.1.10:4173"
        bind:value={webOrigin}
      />
      <p class="field-hint">
        Overrides the browser URL in invite links. Use your LAN IP and web port so friends can open
        the link on their machine. Leave empty to use this browser&apos;s address, or set
        <code>INERTIA_WEB_ORIGIN</code> via env.
      </p>
    </div>

    {#if shareMultiaddr}
      <div class="share-row">
        <code class="mono">{shareMultiaddr}</code>
        <button type="button" class="btn btn-secondary btn-compact" onclick={copyShareAddress}>Copy</button>
      </div>
    {:else if identityState.p2pInfo?.peer_id}
      <p class="muted">No shareable multiaddr yet — set announce addresses or wait for P2P.</p>
    {/if}

    <div class="actions">
      <button type="button" class="btn btn-secondary btn-compact" disabled={saving} onclick={saveSettings}>
        {saving ? 'Saving…' : 'Save'}
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

  .field-hint {
    margin: 0;
    font-size: 0.8125rem;
    color: var(--muted);
    line-height: 1.45;
  }

  .field-hint code {
    font-family: monospace;
    font-size: 0.75rem;
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

  .btn-compact {
    padding: 0.4rem 0.75rem;
    font-size: 0.8125rem;
    font-weight: 500;
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
    font-size: 0.875rem;
    margin: 0 0 0.85rem;
    line-height: 1.45;
  }
</style>
