<script lang="ts">
  import { setTheme, themeState, type Theme } from '$lib/theme.svelte';

  const options: { value: Theme; label: string }[] = [
    { value: 'light', label: 'Light' },
    { value: 'dark', label: 'Dark' }
  ];
</script>

<div class="theme-toggle" role="group" aria-label="Theme">
  {#each options as option}
    <button
      type="button"
      class:active={themeState.theme === option.value}
      onclick={() => setTheme(option.value)}
    >
      {option.label}
    </button>
  {/each}
</div>

<style>
  .theme-toggle {
    display: inline-flex;
    padding: 0.2rem;
    border-radius: 999px;
    border: 1px solid var(--border);
    background: var(--bg);
    gap: 0.15rem;
  }

  .theme-toggle button {
    border: none;
    background: transparent;
    color: var(--muted);
    font-size: 0.75rem;
    font-weight: 600;
    padding: 0.3rem 0.65rem;
    border-radius: 999px;
    line-height: 1.2;
  }

  .theme-toggle button:hover {
    color: var(--text);
  }

  .theme-toggle button.active {
    background: var(--surface);
    color: var(--text);
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.12);
  }
</style>
