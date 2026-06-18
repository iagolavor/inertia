<script lang="ts">
  import FeedHistoryPanel from '$lib/components/FeedHistoryPanel.svelte';
  import ThemeToggle from '$lib/components/ThemeToggle.svelte';
  import { identityState } from '$lib/identity.svelte';
</script>

<h1 class="page-title">Configurações</h1>

<section class="card settings-section">
  <h2 class="section-title">Aparência</h2>
  <p class="section-desc">Tema da interface neste dispositivo.</p>
  <ThemeToggle />
</section>

{#if identityState.loading}
  <p class="empty">A carregar…</p>
{:else if !identityState.apiOnline}
  <section class="card settings-section">
    <h2 class="section-title">Feed e backup</h2>
    <p class="section-desc muted">Liga o API bridge para gerir histórico e backups.</p>
    <pre class="cmd">cargo run -p inertia-api</pre>
  </section>
{:else if identityState.identity}
  <section class="card settings-section">
    <h2 class="section-title">Feed e backup</h2>
    <FeedHistoryPanel />
  </section>

  <section class="card settings-section">
    <h2 class="section-title">Identidade</h2>
    <p class="section-desc">
      Chaves criptográficas deste dispositivo. Usa o safety code para confirmar convites.
    </p>

    <dl class="identity-list">
      <div class="identity-row">
        <dt>Nome</dt>
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
            <span class="muted">P2P a iniciar…</span>
          {/if}
        </dd>
      </div>
    </dl>

    <p class="badge-local">Guardado apenas neste dispositivo</p>
  </section>
{:else}
  <section class="card settings-section">
    <h2 class="section-title">Identidade</h2>
    <p class="section-desc muted">
      Ainda não tens perfil. Cria um no separador Perfil para ver as tuas chaves.
    </p>
    <p style="margin-top: 0.75rem;">
      <a class="btn" href="/profile">Ir para o perfil</a>
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
