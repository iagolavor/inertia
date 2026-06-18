<script lang="ts">
  import { page } from '$app/state';
  import OnlineStatus from '$lib/components/OnlineStatus.svelte';
  import { identityState } from '$lib/identity.svelte';

  const primaryTabs = [
    { href: '/', label: 'Feed', match: (path: string) => path === '/' },
    { href: '/profile', label: 'Perfil', match: (path: string) => path.startsWith('/profile') },
    {
      href: '/settings',
      label: 'Config',
      match: (path: string) => path.startsWith('/settings')
    }
  ] as const;

  const moreLinks = [
    { href: '/friends', label: 'Amigos' },
    { href: '/messages', label: 'Mensagens' },
    { href: '/invite', label: 'Aceitar convite' },
    { href: '/outbox', label: 'Outbox' }
  ];

  let moreOpen = $state(false);

  function closeMore() {
    moreOpen = false;
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape' && moreOpen) closeMore();
  }

  $effect(() => {
    if (typeof document === 'undefined') return;
    if (!moreOpen) return;

    const onPointerDown = (event: PointerEvent) => {
      const target = event.target as HTMLElement | null;
      if (!target?.closest('.more-menu')) closeMore();
    };

    document.addEventListener('pointerdown', onPointerDown);
    return () => document.removeEventListener('pointerdown', onPointerDown);
  });

  $effect(() => {
    page.url.pathname;
    closeMore();
  });
</script>

<svelte:window onkeydown={onKeydown} />

<header class="app-header">
  <div class="header-inner">
    <div class="header-start">
      <a href="/" class="brand">Inertia</a>

      <nav class="primary-tabs" aria-label="Principal">
        {#each primaryTabs as tab}
          <a
            href={tab.href}
            class:active={tab.match(page.url.pathname)}
            aria-current={tab.match(page.url.pathname) ? 'page' : undefined}
          >
            {tab.label}
          </a>
        {/each}
      </nav>
    </div>

    <div class="header-end">
      <OnlineStatus
        online={identityState.apiOnline}
        loading={identityState.loading}
        compact
      />

      <div class="more-menu">
        <button
          type="button"
          class="more-trigger"
          aria-label="Mais opções"
          aria-expanded={moreOpen}
          aria-haspopup="true"
          onclick={() => (moreOpen = !moreOpen)}
        >
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <circle cx="5" cy="12" r="1.75" fill="currentColor" />
            <circle cx="12" cy="12" r="1.75" fill="currentColor" />
            <circle cx="19" cy="12" r="1.75" fill="currentColor" />
          </svg>
        </button>

        {#if moreOpen}
          <div class="more-panel" role="menu">
            {#each moreLinks as link}
              <a
                href={link.href}
                role="menuitem"
                class:active={page.url.pathname === link.href}
                onclick={closeMore}
              >
                {link.label}
              </a>
            {/each}
          </div>
        {/if}
      </div>
    </div>
  </div>
</header>

<style>
  .app-header {
    position: sticky;
    top: 0;
    z-index: 30;
    border-bottom: 1px solid var(--border);
    background: color-mix(in srgb, var(--bg) 88%, transparent);
    backdrop-filter: blur(10px);
  }

  .header-inner {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
    max-width: 720px;
    margin: 0 auto;
    padding: 0.75rem 1.5rem;
  }

  .header-start {
    display: flex;
    align-items: center;
    gap: 0.65rem;
    min-width: 0;
    flex: 1;
  }

  .brand {
    flex-shrink: 0;
    font-size: 1.15rem;
    font-weight: 700;
    letter-spacing: -0.02em;
    color: var(--text);
    text-decoration: none;
  }

  .brand:hover {
    text-decoration: none;
    color: var(--accent);
  }

  .primary-tabs {
    display: inline-flex;
    align-items: center;
    padding: 0.15rem;
    border-radius: 9px;
    border: 1px solid var(--border);
    background: var(--surface);
    gap: 0.1rem;
    max-width: 100%;
    overflow-x: auto;
    scrollbar-width: none;
  }

  .primary-tabs::-webkit-scrollbar {
    display: none;
  }

  .primary-tabs a {
    padding: 0.35rem 0.8rem;
    border-radius: 7px;
    font-size: 0.8125rem;
    font-weight: 600;
    color: var(--muted);
    text-decoration: none;
    white-space: nowrap;
    transition:
      color 0.12s,
      background 0.12s;
  }

  .primary-tabs a:hover {
    color: var(--text);
    text-decoration: none;
  }

  .primary-tabs a.active {
    background: var(--bg);
    color: var(--text);
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.1);
  }

  .header-end {
    display: flex;
    align-items: center;
    gap: 0.45rem;
    flex-shrink: 0;
  }

  .more-menu {
    position: relative;
  }

  .more-trigger {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 2rem;
    height: 2rem;
    padding: 0;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--surface);
    color: var(--muted);
    cursor: pointer;
  }

  .more-trigger:hover {
    color: var(--text);
    background: color-mix(in srgb, var(--border) 25%, var(--surface));
  }

  .more-trigger svg {
    width: 1.15rem;
    height: 1.15rem;
  }

  .more-panel {
    position: absolute;
    top: calc(100% + 0.35rem);
    right: 0;
    min-width: 10.5rem;
    padding: 0.35rem;
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--surface);
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.14);
  }

  .more-panel a {
    display: block;
    padding: 0.55rem 0.7rem;
    border-radius: 7px;
    font-size: 0.875rem;
    font-weight: 500;
    color: var(--muted);
    text-decoration: none;
  }

  .more-panel a:hover {
    background: var(--bg);
    color: var(--text);
    text-decoration: none;
  }

  .more-panel a.active {
    color: var(--accent);
    background: color-mix(in srgb, var(--accent) 8%, transparent);
  }

  @media (max-width: 480px) {
    .header-inner {
      padding-left: 1rem;
      padding-right: 1rem;
    }

    .primary-tabs a {
      padding: 0.35rem 0.65rem;
    }
  }
</style>
