<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/state';
  import InertiaLogo from '$lib/components/InertiaLogo.svelte';
  import OnlineStatus from '$lib/components/OnlineStatus.svelte';
  import P2pStatus from '$lib/components/P2pStatus.svelte';
  import {
    countUnreadDmMessages,
    ensureDmUnreadBaseline,
    formatUnreadBadge,
    subscribeDmUnread
  } from '$lib/dm-unread';
  import { identityState } from '$lib/identity.svelte';
  import { readCachedMessages } from '$lib/local-cache';
  import {
    refreshInboxSilently,
    seedInboxSnapshot,
    subscribeInboxSync,
    type InboxSnapshot
  } from '$lib/messages-sync';

  const primaryTabs = [
    { href: '/', label: 'Feed', match: (path: string) => path === '/' },
    {
      href: '/messages',
      label: 'Messages',
      match: (path: string) =>
        path.startsWith('/messages') || path.startsWith('/friends/')
    },
    { href: '/profile', label: 'Profile', match: (path: string) => path.startsWith('/profile') },
    {
      href: '/connections',
      label: 'Connections',
      match: (path: string) =>
        path.startsWith('/connections') || path.startsWith('/invite')
    },
    {
      href: '/settings',
      label: 'Settings',
      match: (path: string) => path.startsWith('/settings')
    }
  ] as const;

  const moreLinks = [
    { href: '/invite', label: 'Accept invite' },
    { href: '/outbox', label: 'Outbox' }
  ];

  let moreOpen = $state(false);
  let inboxSnapshot = $state<InboxSnapshot | null>(null);
  let unreadTick = $state(0);

  const messagesUnread = $derived.by(() => {
    unreadTick;
    if (!inboxSnapshot) return 0;
    return countUnreadDmMessages(inboxSnapshot.inbox);
  });
  const messagesBadge = $derived(formatUnreadBadge(messagesUnread));

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

  onMount(() => {
    const unsubInbox = subscribeInboxSync((snapshot) => {
      ensureDmUnreadBaseline(snapshot.inbox);
      inboxSnapshot = snapshot;
    });
    const unsubUnread = subscribeDmUnread(() => {
      unreadTick += 1;
    });

    void readCachedMessages().then((cached) => {
      if (cached) seedInboxSnapshot({ contacts: cached.contacts, inbox: cached.inbox });
      if (identityState.apiOnline) void refreshInboxSilently();
    });

    return () => {
      unsubInbox();
      unsubUnread();
    };
  });
</script>

<svelte:window onkeydown={onKeydown} />

<header class="app-header">
  <div class="header-inner">
    <a href="/" class="brand">
      <InertiaLogo />
      <span class="brand-name">Inertia</span>
    </a>

    <nav class="primary-tabs" aria-label="Principal">
      {#each primaryTabs as tab}
        {@const isMessages = tab.href === '/messages'}
        <a
          href={tab.href}
          class:active={tab.match(page.url.pathname)}
          aria-current={tab.match(page.url.pathname) ? 'page' : undefined}
          aria-label={
            isMessages && messagesUnread > 0
              ? `Messages, ${messagesUnread} unread`
              : undefined
          }
        >
          {tab.label}
          {#if isMessages && messagesBadge}
            <span class="tab-count">{messagesBadge}</span>
          {/if}
        </a>
      {/each}
    </nav>

    <div class="header-end">
      <OnlineStatus
        online={identityState.apiOnline}
        loading={identityState.loading}
        compact
      />
      <P2pStatus
        status={identityState.p2pStatus}
        loading={identityState.loading}
        compact
      />

      <div class="more-menu">
        <button
          type="button"
          class="more-trigger"
          aria-expanded={moreOpen}
          aria-haspopup="true"
          onclick={() => (moreOpen = !moreOpen)}
        >
          Menu
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
    padding-top: var(--safe-top);
    border-bottom: 1px solid var(--border);
    background: var(--bg);
  }

  .header-inner {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    grid-template-areas:
      'brand end'
      'tabs tabs';
    align-items: center;
    gap: 0.55rem 0.75rem;
    /* Same max-width as .container (shared app shell) */
    max-width: 960px;
    margin: 0 auto;
    padding: 0.75rem 1.5rem;
    padding-left: max(1.5rem, var(--safe-left));
    padding-right: max(1.5rem, var(--safe-right));
  }

  .brand {
    grid-area: brand;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex-shrink: 0;
    min-width: 0;
    text-decoration: none;
    border-radius: 6px;
    transition: opacity 0.12s;
  }

  .brand:hover {
    text-decoration: none;
    opacity: 0.82;
  }

  .brand-name {
    font-family: var(--font-display);
    font-size: var(--font-size-xl);
    font-weight: var(--font-weight-bold);
    letter-spacing: -0.035em;
    line-height: 1.05;
    color: var(--text);
  }

  .primary-tabs {
    grid-area: tabs;
    display: inline-flex;
    align-items: center;
    justify-self: start;
    padding: 0.2rem;
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
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.3rem;
    padding: 0.3rem 0.75rem;
    border-radius: 7px;
    font-size: var(--font-size-sm);
    font-weight: var(--font-weight-semibold);
    line-height: 1;
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
  }

  .tab-count {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 1.15rem;
    height: 1.15rem;
    padding: 0 0.28rem;
    border-radius: 999px;
    background: var(--accent);
    color: var(--btn-on-accent, #fff);
    font-size: var(--font-size-xs);
    font-weight: var(--font-weight-bold);
    line-height: 1;
  }

  .header-end {
    grid-area: end;
    display: flex;
    align-items: center;
    gap: 0.45rem;
    flex-shrink: 0;
    justify-self: end;
  }

  .more-menu {
    position: relative;
  }

  .more-trigger {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 0.28rem 0.55rem;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: transparent;
    color: var(--muted);
    font: inherit;
    font-size: var(--font-size-sm);
    font-weight: var(--font-weight-semibold);
    line-height: 1.2;
    flex-shrink: 0;
    cursor: pointer;
    transition:
      border-color 0.15s,
      background 0.15s,
      color 0.12s;
  }

  .more-trigger:hover {
    color: var(--text);
    background: color-mix(in srgb, var(--border) 35%, transparent);
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
    font-size: var(--font-size-md);
    font-weight: var(--font-weight-medium);
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

  @media (max-width: 640px) {
    .header-inner {
      gap: 0.5rem;
      padding-left: max(1rem, var(--safe-left));
      padding-right: max(1rem, var(--safe-right));
    }

    .primary-tabs {
      justify-self: stretch;
      width: 100%;
    }

    .primary-tabs a {
      flex: 1 1 0;
      min-width: 0;
      padding: 0.45rem 0.35rem;
      font-size: var(--font-size-xs);
    }
  }

  @media (max-width: 380px) {
    .brand-name {
      display: none;
    }

    .header-end {
      gap: 0.35rem;
    }
  }
</style>
