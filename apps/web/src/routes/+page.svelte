<script lang="ts">
  import { identityState } from '$lib/identity.svelte';
</script>

<h1>Inertia</h1>
<p class="subtitle">Ephemeral P2P social — zero tracking, zero ads, your circle only.</p>

{#if identityState.loading}
  <p class="empty">Loading...</p>
{:else if !identityState.apiOnline}
  <div class="card">
    <h2>API offline</h2>
    <p>Start the Rust API bridge before using the app:</p>
    <pre class="cmd">cargo run -p inertia-api</pre>
  </div>
{:else if identityState.identity}
  <div class="card">
    <h2>Welcome back, {identityState.identity.display_name}</h2>
    <p class="muted">
      <a href="/profile">View your profile</a> ·
      <a href="/friends">Invite a friend</a> ·
      <a href="/messages">Messages</a>
    </p>
  </div>
{:else}
  <div class="card">
    <h2>Get started</h2>
    <p class="muted">Create a local profile to connect with people you trust.</p>
    <p style="margin-top: 1rem;">
      <a class="btn" href="/profile">Create your profile</a>
    </p>
  </div>
{/if}

<div class="card">
  <h3>How it works</h3>
  <ul class="muted list">
    <li>Invite links expire in 15 minutes and work only once</li>
    <li>Messages expire after 7 days; posts after 48 hours</li>
    <li>Delivery is direct P2P when both of you are online</li>
    <li>No ads, no algorithms, no doomscrolling</li>
  </ul>
</div>

<style>
  .muted {
    color: var(--muted);
  }

  .list {
    padding-left: 1.25rem;
    margin: 0;
  }

  .cmd {
    background: var(--bg);
    padding: 1rem;
    border-radius: 8px;
    overflow-x: auto;
  }
</style>
