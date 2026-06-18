<script lang="ts">
  import { page } from '$app/state';
  import Avatar from '$lib/components/Avatar.svelte';
  import OnlineStatus from '$lib/components/OnlineStatus.svelte';
  import ThemeToggle from '$lib/components/ThemeToggle.svelte';
  import { identityState } from '$lib/identity.svelte';

  interface Props {
    open?: boolean;
    onclose?: () => void;
  }

  let { open = $bindable(false), onclose }: Props = $props();

  const links = [
    { href: '/', label: 'Home' },
    { href: '/profile', label: 'Profile' },
    { href: '/friends', label: 'Friends' },
    { href: '/invite', label: 'Accept invite' },
    { href: '/messages', label: 'Messages' },
    { href: '/outbox', label: 'Outbox' }
  ];

  function close() {
    open = false;
    onclose?.();
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape' && open) close();
  }

  $effect(() => {
    if (typeof document === 'undefined') return;
    document.body.style.overflow = open ? 'hidden' : '';
    return () => {
      document.body.style.overflow = '';
    };
  });
</script>

<svelte:window onkeydown={onKeydown} />

<button
  type="button"
  class="menu-trigger"
  aria-label="Open menu"
  aria-expanded={open}
  onclick={() => (open = true)}
>
  <span class="menu-icon" aria-hidden="true"></span>
</button>

{#if open}
  <button type="button" class="backdrop" aria-label="Close menu" onclick={close}></button>

  <aside class="drawer" role="dialog" aria-modal="true" aria-label="Navigation">
    <header class="drawer-header">
      <span class="drawer-title">Inertia</span>
      <button type="button" class="close-btn" aria-label="Close menu" onclick={close}>×</button>
    </header>

    {#if identityState.identity}
      <a href="/profile" class="drawer-profile" onclick={close}>
        <Avatar
          seed={identityState.identity.signing_pubkey}
          alt={identityState.identity.display_name}
          size={44}
        />
        <span class="drawer-profile-name">{identityState.identity.display_name}</span>
      </a>
    {/if}

    <div class="drawer-status">
      <OnlineStatus online={identityState.apiOnline} loading={identityState.loading} />
    </div>

    <nav class="drawer-nav">
      {#each links as link}
        <a
          href={link.href}
          class:active={page.url.pathname === link.href}
          onclick={close}
        >
          {link.label}
        </a>
      {/each}
    </nav>

    <footer class="drawer-footer">
      <ThemeToggle />
    </footer>
  </aside>
{/if}

<style>
  .menu-trigger {
    position: fixed;
    top: 1rem;
    left: 50%;
    z-index: 40;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 2.5rem;
    height: 2.5rem;
    padding: 0;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--surface);
    color: var(--text);
    cursor: pointer;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.08);
    transform: translateX(-50%);
  }

  .menu-trigger:hover {
    background: color-mix(in srgb, var(--border) 30%, var(--surface));
  }

  .menu-icon,
  .menu-icon::before,
  .menu-icon::after {
    display: block;
    width: 1.1rem;
    height: 2px;
    background: currentColor;
    border-radius: 1px;
    position: relative;
  }

  .menu-icon::before,
  .menu-icon::after {
    content: '';
    position: absolute;
    left: 0;
  }

  .menu-icon::before {
    top: -6px;
  }

  .menu-icon::after {
    top: 6px;
  }

  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 50;
    border: none;
    padding: 0;
    background: rgba(0, 0, 0, 0.45);
    cursor: pointer;
  }

  .drawer {
    position: fixed;
    top: 0;
    left: 0;
    z-index: 51;
    display: flex;
    flex-direction: column;
    width: min(18rem, 85vw);
    height: 100%;
    background: var(--surface);
    border-right: 1px solid var(--border);
    box-shadow: 4px 0 24px rgba(0, 0, 0, 0.12);
    animation: slide-in 0.2s ease-out;
  }

  @keyframes slide-in {
    from {
      transform: translateX(-100%);
    }
    to {
      transform: translateX(0);
    }
  }

  .drawer-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 1rem 1rem 0.75rem;
    border-bottom: 1px solid var(--border);
  }

  .drawer-title {
    font-size: 1.1rem;
    font-weight: 700;
    letter-spacing: -0.02em;
  }

  .close-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 2rem;
    height: 2rem;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: var(--muted);
    font-size: 1.5rem;
    line-height: 1;
    cursor: pointer;
  }

  .close-btn:hover {
    background: var(--bg);
    color: var(--text);
  }

  .drawer-profile {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 1rem;
    text-decoration: none;
    color: inherit;
    border-bottom: 1px solid var(--border);
  }

  .drawer-profile:hover {
    background: var(--bg);
    text-decoration: none;
  }

  .drawer-profile-name {
    font-weight: 600;
    font-size: 0.95rem;
  }

  .drawer-status {
    padding: 0.75rem 1rem;
    border-bottom: 1px solid var(--border);
  }

  .drawer-nav {
    display: flex;
    flex-direction: column;
    padding: 0.5rem 0;
    flex: 1;
    overflow-y: auto;
  }

  .drawer-nav a {
    padding: 0.7rem 1rem;
    color: var(--muted);
    font-weight: 500;
    text-decoration: none;
  }

  .drawer-nav a:hover {
    background: var(--bg);
    color: var(--text);
    text-decoration: none;
  }

  .drawer-nav a.active {
    color: var(--accent);
    background: color-mix(in srgb, var(--accent) 8%, transparent);
  }

  .drawer-footer {
    padding: 1rem;
    border-top: 1px solid var(--border);
  }
</style>
